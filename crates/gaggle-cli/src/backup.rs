//! Checksummed, WAL-safe backup and atomic restore of Gaggle local data.
//!
//! A backup archive (`gaggle.backup` format version 1) contains:
//! - `gaggle.db`: a consistent `VACUUM INTO` snapshot of the SQLite database.
//! - `attachments/...`: every regular file below the attachments directory,
//!   when that directory exists.
//! - `manifest.json`: format metadata, a database integrity/count summary, and
//!   a SHA-256 checksum plus byte size for every other archived file.
//!
//! Restore extracts into a staging directory on the same filesystem as the
//! target, verifies every checksum and the SQLite integrity of the snapshot,
//! and only then replaces existing data after moving it aside.

use anyhow::{Context, Result, bail};
use flate2::{Compression, read::GzDecoder, write::GzEncoder};
use gaggle_db::{
    DatabaseVerification,
    backup::{attachments_dir, snapshot_database, verify_database_file},
    connection::now_ms,
};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::{
    collections::{HashMap, HashSet},
    fs::{self, File, OpenOptions},
    io::{BufReader, Read, Write},
    path::{Component, Path, PathBuf},
};
use tar::{Archive, Builder, EntryType, Header};

/// Format identifier written into `manifest.json`.
pub const BACKUP_FORMAT: &str = "gaggle.backup";
/// Version of the backup archive layout.
pub const BACKUP_VERSION: u32 = 1;
/// Archive entry holding the backup manifest.
pub const MANIFEST_ENTRY: &str = "manifest.json";
/// Archive entry holding the SQLite snapshot.
pub const DATABASE_ENTRY: &str = "gaggle.db";
/// Archive entry prefix for attachment files.
pub const ATTACHMENTS_PREFIX: &str = "attachments/";
/// Directory name (next to the database) holding attachment files.
const ATTACHMENTS_DIR_NAME: &str = "attachments";

const MAX_ARCHIVE_ENTRIES: usize = 200_000;
const MAX_ARCHIVE_ENTRY_BYTES: u64 = 8 * 1024 * 1024 * 1024;
const MAX_ARCHIVE_TOTAL_BYTES: u64 = 16 * 1024 * 1024 * 1024;

/// One file recorded in the backup manifest.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct BackupFile {
    /// Archive-relative path using `/` separators.
    pub path: String,
    pub sha256: String,
    pub size_bytes: u64,
}

/// Machine-readable manifest stored at `manifest.json` inside the archive.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BackupManifest {
    pub format: String,
    pub version: u32,
    pub created_at: i64,
    pub app_version: String,
    pub database: DatabaseVerification,
    pub attachments_included: bool,
    pub files: Vec<BackupFile>,
}

impl BackupManifest {
    /// Validates manifest metadata and the set of declared file entries.
    ///
    /// `manifest.json` itself is intentionally not self-checksummed; every
    /// other archived file is verified against this manifest during restore.
    pub fn validate(&self) -> Result<()> {
        if self.format != BACKUP_FORMAT {
            bail!(
                "Unsupported backup format {:?}; expected {BACKUP_FORMAT:?}",
                self.format
            );
        }
        if self.version != BACKUP_VERSION {
            bail!(
                "Unsupported backup version {}; this build supports version {BACKUP_VERSION}",
                self.version
            );
        }
        if self.created_at < 0 {
            bail!("Backup manifest created_at must be a non-negative Unix timestamp");
        }
        if self.files.len() > MAX_ARCHIVE_ENTRIES {
            bail!(
                "Backup manifest lists {} files; the maximum is {MAX_ARCHIVE_ENTRIES}",
                self.files.len()
            );
        }
        if self.database.integrity != "ok" {
            bail!(
                "Backup manifest reports an unhealthy database integrity result: {}",
                self.database.integrity
            );
        }

        let mut seen = HashSet::with_capacity(self.files.len());
        for file in &self.files {
            validate_archive_path(Path::new(&file.path))?;
            if file.path == MANIFEST_ENTRY {
                bail!("Backup manifest must not list {MANIFEST_ENTRY} as a checksummed file");
            }
            if !seen.insert(file.path.as_str()) {
                bail!("Backup manifest lists duplicate path {}", file.path);
            }
            if file.size_bytes > MAX_ARCHIVE_ENTRY_BYTES {
                bail!(
                    "Backup entry {} is {} bytes; the maximum is {} bytes",
                    file.path,
                    file.size_bytes,
                    MAX_ARCHIVE_ENTRY_BYTES
                );
            }
            validate_sha256(&file.path, &file.sha256)?;
        }
        if !seen.contains(DATABASE_ENTRY) {
            bail!("Backup manifest does not list the required {DATABASE_ENTRY} database entry");
        }
        Ok(())
    }

    fn attachments(&self) -> impl Iterator<Item = &BackupFile> {
        self.files
            .iter()
            .filter(|file| file.path.starts_with(ATTACHMENTS_PREFIX))
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BackupReport {
    pub archive: PathBuf,
    pub archive_bytes: u64,
    pub archive_sha256: String,
    pub manifest: BackupManifest,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RestoreReport {
    pub archive: PathBuf,
    pub archive_sha256: String,
    pub database_path: PathBuf,
    pub database: DatabaseVerification,
    pub attachments_restored: usize,
    pub replaced_existing: bool,
    pub dry_run: bool,
    /// Directory holding the previous database/attachments after a successful
    /// restore. Retained so an operator can roll back or delete it manually.
    pub previous_data_dir: Option<PathBuf>,
}

struct AttachmentFile {
    archive_path: String,
    source: PathBuf,
}

/// Creates a checksummed `.tar.gz` backup of the database and attachments.
///
/// The SQLite snapshot is produced with `VACUUM INTO`, so uncheckpointed WAL
/// content is included and no `-wal`/`-shm` files are copied. The function is
/// async only for the database snapshot; archiving is local filesystem work.
pub async fn create_backup(db_path: &Path, output: &Path, force: bool) -> Result<BackupReport> {
    let data_dir = data_dir_for(db_path)?;
    if !db_path.is_file() {
        bail!(
            "SQLite database not found at {}; nothing to back up",
            db_path.display()
        );
    }
    if output.as_os_str().is_empty() {
        bail!("Backup output path must not be empty");
    }
    if output.is_dir() {
        bail!("Backup output path is a directory: {}", output.display());
    }
    if output.exists() && !force {
        bail!(
            "Backup output {} already exists; pass --force to overwrite it",
            output.display()
        );
    }

    fs::create_dir_all(data_dir)
        .with_context(|| format!("Failed to create data directory {}", data_dir.display()))?;

    let staging = tempfile::Builder::new()
        .prefix(".gaggle-backup-")
        .tempdir_in(data_dir)
        .with_context(|| {
            format!(
                "Failed to create backup staging directory in {}",
                data_dir.display()
            )
        })?;

    let snapshot_path = staging.path().join(DATABASE_ENTRY);
    snapshot_database(db_path, &snapshot_path).await?;
    let database = verify_database_file(&snapshot_path).await?;

    let attachments_root =
        attachments_dir(db_path).unwrap_or_else(|| data_dir.join(ATTACHMENTS_DIR_NAME));
    if output.starts_with(&attachments_root) {
        bail!(
            "Backup output must not be inside the attachments directory {}; choose a path outside the archived data",
            attachments_root.display()
        );
    }
    let attachments_included = match fs::symlink_metadata(&attachments_root) {
        Ok(metadata) if metadata.file_type().is_symlink() => {
            bail!(
                "Attachments path {} is a symlink; refusing to follow it during backup",
                attachments_root.display()
            );
        }
        Ok(metadata) if metadata.is_dir() => true,
        Ok(_) => {
            bail!(
                "Attachments path exists but is not a directory: {}",
                attachments_root.display()
            );
        }
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => false,
        Err(error) => {
            return Err(error).with_context(|| {
                format!(
                    "Failed to inspect attachments directory {}",
                    attachments_root.display()
                )
            });
        }
    };
    let attachments = collect_attachment_files(&attachments_root)?;

    let created_at = now_ms();
    let mut files = Vec::with_capacity(attachments.len() + 1);
    let (db_sha, db_size) = sha256_file(&snapshot_path)?;
    files.push(BackupFile {
        path: DATABASE_ENTRY.to_string(),
        sha256: db_sha,
        size_bytes: db_size,
    });
    for attachment in &attachments {
        let (sha256, size_bytes) = sha256_file(&attachment.source)?;
        files.push(BackupFile {
            path: attachment.archive_path.clone(),
            sha256,
            size_bytes,
        });
    }

    let manifest = BackupManifest {
        format: BACKUP_FORMAT.to_string(),
        version: BACKUP_VERSION,
        created_at,
        app_version: env!("CARGO_PKG_VERSION").to_string(),
        database,
        attachments_included,
        files,
    };
    manifest.validate()?;

    let manifest_path = staging.path().join(MANIFEST_ENTRY);
    let manifest_json =
        serde_json::to_vec_pretty(&manifest).context("Failed to serialize backup manifest")?;
    fs::write(&manifest_path, &manifest_json).with_context(|| {
        format!(
            "Failed to write backup manifest to {}",
            manifest_path.display()
        )
    })?;
    File::open(&manifest_path)?
        .sync_all()
        .context("Failed to flush backup manifest")?;

    let output_parent = output
        .parent()
        .filter(|parent| !parent.as_os_str().is_empty())
        .unwrap_or_else(|| Path::new("."));
    fs::create_dir_all(output_parent).with_context(|| {
        format!(
            "Failed to create backup output directory {}",
            output_parent.display()
        )
    })?;

    let temp = tempfile::NamedTempFile::new_in(output_parent).with_context(|| {
        format!(
            "Failed to create temporary backup file in {}",
            output_parent.display()
        )
    })?;
    {
        let encoder = GzEncoder::new(temp.as_file().try_clone()?, Compression::default());
        let mut builder = Builder::new(encoder);
        let mtime = u64::try_from(created_at / 1000).unwrap_or(0);

        append_file(&mut builder, DATABASE_ENTRY, &snapshot_path, mtime)?;
        for attachment in &attachments {
            append_file(
                &mut builder,
                &attachment.archive_path,
                &attachment.source,
                mtime,
            )?;
        }
        append_file(&mut builder, MANIFEST_ENTRY, &manifest_path, mtime)?;

        let encoder = builder
            .into_inner()
            .context("Failed to finish backup archive")?;
        let file = encoder
            .finish()
            .context("Failed to finish gzip-compressed backup archive")?;
        file.sync_all().context("Failed to flush backup archive")?;
    }

    temp.persist(output).map_err(|error| {
        anyhow::anyhow!(
            "Failed to write backup archive to {}: {error}",
            output.display()
        )
    })?;

    let (archive_sha256, archive_bytes) = sha256_file(output)?;
    Ok(BackupReport {
        archive: output.to_path_buf(),
        archive_bytes,
        archive_sha256,
        manifest,
    })
}

/// Verifies and restores a backup archive.
///
/// Existing `gaggle.db`, `gaggle.db-wal`, `gaggle.db-shm`, or `attachments/`
/// data is only replaced when `force` is `true`; otherwise the call fails with
/// an actionable error. `dry_run` performs full verification without touching
/// existing data.
pub async fn restore_backup(
    archive_path: &Path,
    db_path: &Path,
    force: bool,
    dry_run: bool,
) -> Result<RestoreReport> {
    if !archive_path.is_file() {
        bail!("Backup archive not found at {}", archive_path.display());
    }
    let data_dir = data_dir_for(db_path)?;
    fs::create_dir_all(data_dir)
        .with_context(|| format!("Failed to create data directory {}", data_dir.display()))?;

    let (archive_sha256, _) = sha256_file(archive_path)?;

    let staging = tempfile::Builder::new()
        .prefix(".gaggle-restore-")
        .tempdir_in(data_dir)
        .with_context(|| {
            format!(
                "Failed to create restore staging directory in {}",
                data_dir.display()
            )
        })?;

    let manifest = extract_and_verify_archive(archive_path, staging.path())?;

    let snapshot_path = staging.path().join(DATABASE_ENTRY);
    let database = verify_database_file(&snapshot_path)
        .await
        .context("Staged database snapshot failed verification")?;
    if database != manifest.database {
        bail!(
            "Backup manifest database summary does not match the staged snapshot; the archive may have been modified"
        );
    }

    let attachments_restored = manifest.attachments().count();
    let staged_attachments = staging.path().join(ATTACHMENTS_DIR_NAME);
    if attachments_restored > 0 && !staged_attachments.is_dir() {
        bail!("Backup manifest lists attachments but the archive does not contain them");
    }

    let existing = existing_restore_targets(db_path);
    if !existing.is_empty() && !force && !dry_run {
        let list = existing
            .iter()
            .map(|path| path.display().to_string())
            .collect::<Vec<_>>()
            .join(", ");
        bail!(
            "Refusing to replace existing Gaggle data without confirmation. Existing targets: {list}. Re-run with --force after verifying the archive, or use --dry-run to validate it first."
        );
    }

    if dry_run {
        return Ok(RestoreReport {
            archive: archive_path.to_path_buf(),
            archive_sha256,
            database_path: db_path.to_path_buf(),
            database,
            attachments_restored,
            replaced_existing: !existing.is_empty(),
            dry_run: true,
            previous_data_dir: None,
        });
    }

    let previous_data_dir = apply_restore(staging.path(), db_path)?;

    Ok(RestoreReport {
        archive: archive_path.to_path_buf(),
        archive_sha256,
        database_path: db_path.to_path_buf(),
        database,
        attachments_restored,
        replaced_existing: previous_data_dir.is_some(),
        dry_run: false,
        previous_data_dir,
    })
}

/// Extracts an archive into `staging`, rejecting unsafe paths and verifying
/// every declared SHA-256 checksum.
fn extract_and_verify_archive(archive_path: &Path, staging: &Path) -> Result<BackupManifest> {
    let file = File::open(archive_path)
        .with_context(|| format!("Failed to open backup archive {}", archive_path.display()))?;
    let decoder = GzDecoder::new(BufReader::new(file));
    let mut archive = Archive::new(decoder);

    let mut seen: HashSet<String> = HashSet::new();
    let mut computed: HashMap<String, BackupFile> = HashMap::new();
    let mut total_bytes = 0u64;
    let mut entry_count = 0usize;

    for entry in archive
        .entries()
        .context("Failed to read backup archive entries")?
    {
        let mut entry = entry.context("Failed to read backup archive entry")?;
        entry_count += 1;
        if entry_count > MAX_ARCHIVE_ENTRIES {
            bail!("Backup archive contains more than {MAX_ARCHIVE_ENTRIES} entries");
        }
        if !entry.header().entry_type().is_file() {
            bail!(
                "Backup archive contains a non-regular entry type {:?}; only regular files are allowed",
                entry.header().entry_type()
            );
        }

        let raw_path = entry
            .path()
            .context("Backup archive contains an invalid entry path")?
            .into_owned();
        let archive_path = validate_archive_path(&raw_path)?;
        if !seen.insert(archive_path.clone()) {
            bail!("Backup archive contains duplicate entry {archive_path}");
        }

        let declared_size = entry
            .header()
            .size()
            .context("Backup archive entry has an invalid size")?;
        if declared_size > MAX_ARCHIVE_ENTRY_BYTES {
            bail!(
                "Backup archive entry {archive_path} is {declared_size} bytes; the maximum is {MAX_ARCHIVE_ENTRY_BYTES} bytes"
            );
        }
        total_bytes = total_bytes.saturating_add(declared_size);
        if total_bytes > MAX_ARCHIVE_TOTAL_BYTES {
            bail!(
                "Backup archive expands beyond the {} byte limit",
                MAX_ARCHIVE_TOTAL_BYTES
            );
        }

        let destination = staging.join(&archive_path);
        if let Some(parent) = destination.parent() {
            fs::create_dir_all(parent).with_context(|| {
                format!("Failed to create staging directory {}", parent.display())
            })?;
        }
        let mut output = OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(&destination)
            .with_context(|| format!("Failed to stage archive entry {archive_path}"))?;

        let mut hasher = Sha256::new();
        let mut size_bytes = 0u64;
        let mut buffer = [0u8; 64 * 1024];
        loop {
            let read = entry
                .read(&mut buffer)
                .context("Failed to read backup archive entry data")?;
            if read == 0 {
                break;
            }
            size_bytes = size_bytes.saturating_add(read as u64);
            if size_bytes > MAX_ARCHIVE_ENTRY_BYTES {
                bail!("Backup archive entry {archive_path} exceeds the per-file size limit");
            }
            hasher.update(&buffer[..read]);
            output
                .write_all(&buffer[..read])
                .with_context(|| format!("Failed to stage archive entry {archive_path}"))?;
        }
        output
            .sync_all()
            .with_context(|| format!("Failed to flush archive entry {archive_path}"))?;

        if size_bytes != declared_size {
            bail!(
                "Backup archive entry {archive_path} is truncated: expected {declared_size} bytes, read {size_bytes}"
            );
        }

        computed.insert(
            archive_path.clone(),
            BackupFile {
                path: archive_path,
                sha256: hex(&hasher.finalize()),
                size_bytes,
            },
        );
    }

    if !seen.contains(MANIFEST_ENTRY) {
        bail!("Backup archive is missing {MANIFEST_ENTRY}");
    }
    if !seen.contains(DATABASE_ENTRY) {
        bail!("Backup archive is missing {DATABASE_ENTRY}");
    }

    let manifest_bytes = fs::read(staging.join(MANIFEST_ENTRY))
        .context("Failed to read the extracted backup manifest")?;
    let manifest: BackupManifest =
        serde_json::from_slice(&manifest_bytes).context("Backup manifest is not valid JSON")?;
    manifest.validate()?;

    for file in &manifest.files {
        let Some(actual) = computed.get(&file.path) else {
            bail!(
                "Backup manifest lists {} but the archive does not contain it",
                file.path
            );
        };
        if actual.size_bytes != file.size_bytes {
            bail!(
                "Checksum verification failed for {}: expected {} bytes, archive contains {} bytes",
                file.path,
                file.size_bytes,
                actual.size_bytes
            );
        }
        if !actual.sha256.eq_ignore_ascii_case(&file.sha256) {
            bail!(
                "Checksum verification failed for {}: expected {}, computed {}",
                file.path,
                file.sha256,
                actual.sha256
            );
        }
    }

    let expected_entries = manifest
        .files
        .iter()
        .map(|file| file.path.as_str())
        .chain(std::iter::once(MANIFEST_ENTRY))
        .collect::<HashSet<_>>();
    if let Some(unexpected) = computed
        .keys()
        .find(|path| !expected_entries.contains(path.as_str()))
    {
        bail!("Backup archive contains an undeclared entry {unexpected}");
    }

    Ok(manifest)
}

fn apply_restore(staging: &Path, db_path: &Path) -> Result<Option<PathBuf>> {
    let data_dir = data_dir_for(db_path)?;
    let targets = existing_restore_targets(db_path);
    let previous_dir = unique_previous_dir(data_dir);
    let mut moved: Vec<(PathBuf, PathBuf)> = Vec::new();

    if !targets.is_empty() {
        fs::create_dir_all(&previous_dir).with_context(|| {
            format!(
                "Failed to create previous-data directory {}",
                previous_dir.display()
            )
        })?;

        for target in &targets {
            let file_name = target
                .file_name()
                .ok_or_else(|| anyhow::anyhow!("Invalid restore target {}", target.display()))?;
            let destination = previous_dir.join(file_name);
            if let Err(error) = fs::rename(target, &destination) {
                for (original, backup) in moved.iter().rev() {
                    let _ = fs::rename(backup, original);
                }
                let _ = fs::remove_dir_all(&previous_dir);
                return Err(error).with_context(|| {
                    format!(
                        "Failed to move existing data {} aside before restore",
                        target.display()
                    )
                });
            }
            moved.push((target.clone(), destination));
        }
    }

    match install_staged(staging, db_path) {
        Ok(()) => {
            if targets.is_empty() {
                let _ = fs::remove_dir_all(&previous_dir);
                Ok(None)
            } else {
                Ok(Some(previous_dir))
            }
        }
        Err(error) => {
            remove_restored_targets(db_path);
            for (original, backup) in moved.iter().rev() {
                let _ = fs::rename(backup, original);
            }
            let _ = fs::remove_dir_all(&previous_dir);
            Err(error)
        }
    }
}

fn install_staged(staging: &Path, db_path: &Path) -> Result<()> {
    let staged_db = staging.join(DATABASE_ENTRY);
    fs::rename(&staged_db, db_path).with_context(|| {
        format!(
            "Failed to install restored database at {}",
            db_path.display()
        )
    })?;

    let staged_attachments = staging.join(ATTACHMENTS_DIR_NAME);
    if staged_attachments.is_dir() {
        let target = attachments_dir(db_path)
            .ok_or_else(|| anyhow::anyhow!("Cannot determine attachments directory"))?;
        fs::rename(&staged_attachments, &target).with_context(|| {
            format!(
                "Failed to install restored attachments at {}",
                target.display()
            )
        })?;
    }

    if let Some(parent) = db_path.parent() {
        let _ = File::open(parent).and_then(|dir| dir.sync_all());
    }
    Ok(())
}

fn remove_restored_targets(db_path: &Path) {
    for path in existing_restore_targets(db_path) {
        if path.is_dir() {
            let _ = fs::remove_dir_all(&path);
        } else {
            let _ = fs::remove_file(&path);
        }
    }
}

fn existing_restore_targets(db_path: &Path) -> Vec<PathBuf> {
    let mut targets = Vec::new();
    for candidate in [
        db_path.to_path_buf(),
        sidecar_path(db_path, "-wal"),
        sidecar_path(db_path, "-shm"),
    ] {
        if candidate.exists() {
            targets.push(candidate);
        }
    }
    if let Some(attachments) = attachments_dir(db_path)
        && attachments.exists()
    {
        targets.push(attachments);
    }
    targets
}

fn sidecar_path(db_path: &Path, suffix: &str) -> PathBuf {
    let mut name = db_path.as_os_str().to_os_string();
    name.push(suffix);
    PathBuf::from(name)
}

/// Picks a non-existing rollback directory so two restores in the same
/// millisecond cannot collide.
fn unique_previous_dir(data_dir: &Path) -> PathBuf {
    let base = format!("gaggle-restore-previous-{}", now_ms());
    let mut candidate = data_dir.join(&base);
    let mut suffix = 1u32;
    while candidate.exists() {
        candidate = data_dir.join(format!("{base}-{suffix}"));
        suffix += 1;
    }
    candidate
}

fn data_dir_for(db_path: &Path) -> Result<&Path> {
    Ok(db_path
        .parent()
        .filter(|parent| !parent.as_os_str().is_empty())
        .unwrap_or_else(|| Path::new(".")))
}

fn collect_attachment_files(root: &Path) -> Result<Vec<AttachmentFile>> {
    if !root.exists() {
        return Ok(Vec::new());
    }
    if !root.is_dir() {
        bail!(
            "Attachments path exists but is not a directory: {}",
            root.display()
        );
    }

    let mut files = Vec::new();
    collect_attachment_files_inner(root, root, &mut files)?;
    files.sort_by(|left, right| left.archive_path.cmp(&right.archive_path));
    Ok(files)
}

fn collect_attachment_files_inner(
    root: &Path,
    directory: &Path,
    files: &mut Vec<AttachmentFile>,
) -> Result<()> {
    for entry in fs::read_dir(directory).with_context(|| {
        format!(
            "Failed to read attachments directory {}",
            directory.display()
        )
    })? {
        let entry = entry.context("Failed to read attachments directory entry")?;
        let path = entry.path();
        let file_type = entry
            .file_type()
            .with_context(|| format!("Failed to inspect {}", path.display()))?;

        if file_type.is_symlink() {
            tracing::warn!(
                path = %path.display(),
                "Skipping symlink while creating backup; symlinked attachments are not archived"
            );
            continue;
        }
        if file_type.is_dir() {
            collect_attachment_files_inner(root, &path, files)?;
            continue;
        }
        if !file_type.is_file() {
            tracing::warn!(
                path = %path.display(),
                "Skipping special file while creating backup"
            );
            continue;
        }

        let relative = path.strip_prefix(root).with_context(|| {
            format!(
                "Attachment {} is outside the attachments directory",
                path.display()
            )
        })?;
        let archive_path = format!("{ATTACHMENTS_PREFIX}{}", relative_archive_path(relative)?);
        files.push(AttachmentFile {
            archive_path,
            source: path,
        });
    }
    Ok(())
}

fn relative_archive_path(relative: &Path) -> Result<String> {
    let mut parts = Vec::new();
    for component in relative.components() {
        let Component::Normal(part) = component else {
            bail!(
                "Attachment path contains an unsupported component: {}",
                relative.display()
            );
        };
        let part = part.to_str().ok_or_else(|| {
            anyhow::anyhow!(
                "Attachment path is not valid UTF-8 and cannot be backed up: {}",
                relative.display()
            )
        })?;
        if part.contains(['\\', '\0']) {
            bail!("Attachment path contains unsupported characters: {part:?}");
        }
        parts.push(part.to_string());
    }
    if parts.is_empty() {
        bail!("Attachment path is empty");
    }
    Ok(parts.join("/"))
}

/// Validates an archive entry path and returns its canonical `/`-separated form.
fn validate_archive_path(path: &Path) -> Result<String> {
    if path.is_absolute() {
        bail!(
            "Backup archive contains an absolute path, which is not allowed: {}",
            path.display()
        );
    }
    let mut parts = Vec::new();
    for component in path.components() {
        let Component::Normal(part) = component else {
            bail!(
                "Backup archive contains an unsafe path component in {}; parent, current, root, and prefix components are rejected",
                path.display()
            );
        };
        let part = part.to_str().ok_or_else(|| {
            anyhow::anyhow!(
                "Backup archive entry path is not valid UTF-8: {}",
                path.display()
            )
        })?;
        if part.contains(['\\', '\0']) {
            bail!(
                "Backup archive entry path contains unsupported characters: {}",
                path.display()
            );
        }
        parts.push(part);
    }
    if parts.is_empty() {
        bail!("Backup archive contains an empty entry path");
    }

    let joined = parts.join("/");
    let allowed = joined == MANIFEST_ENTRY
        || joined == DATABASE_ENTRY
        || joined.starts_with(ATTACHMENTS_PREFIX);
    if !allowed {
        bail!(
            "Backup archive entry {joined} is outside the allowed backup layout ({DATABASE_ENTRY}, {ATTACHMENTS_PREFIX}*, {MANIFEST_ENTRY})"
        );
    }
    if joined == ATTACHMENTS_PREFIX.trim_end_matches('/') {
        bail!("Backup archive contains an attachments directory entry rather than a file");
    }
    Ok(joined)
}

fn validate_sha256(path: &str, value: &str) -> Result<()> {
    if value.len() != 64 || !value.chars().all(|ch| ch.is_ascii_hexdigit()) {
        bail!("Backup manifest contains an invalid SHA-256 checksum for {path}");
    }
    Ok(())
}

fn append_file<W: Write>(
    builder: &mut Builder<W>,
    archive_path: &str,
    source: &Path,
    mtime: u64,
) -> Result<()> {
    let metadata = fs::metadata(source)
        .with_context(|| format!("Failed to inspect archive source {}", source.display()))?;
    let mut file = File::open(source)
        .with_context(|| format!("Failed to open archive source {}", source.display()))?;
    let mut header = Header::new_gnu();
    header.set_entry_type(EntryType::Regular);
    header.set_size(metadata.len());
    header.set_mode(0o644);
    header.set_mtime(mtime);
    header.set_cksum();
    builder
        .append_data(&mut header, archive_path, &mut file)
        .with_context(|| format!("Failed to archive {archive_path}"))?;
    Ok(())
}

fn sha256_file(path: &Path) -> Result<(String, u64)> {
    let file = File::open(path)
        .with_context(|| format!("Failed to open file for checksum: {}", path.display()))?;
    let mut reader = BufReader::new(file);
    let mut hasher = Sha256::new();
    let mut size_bytes = 0u64;
    let mut buffer = [0u8; 64 * 1024];
    loop {
        let read = reader
            .read(&mut buffer)
            .with_context(|| format!("Failed to read file for checksum: {}", path.display()))?;
        if read == 0 {
            break;
        }
        size_bytes = size_bytes.saturating_add(read as u64);
        hasher.update(&buffer[..read]);
    }
    Ok((hex(&hasher.finalize()), size_bytes))
}

fn hex(bytes: &[u8]) -> String {
    let mut out = String::with_capacity(bytes.len() * 2);
    for byte in bytes {
        use std::fmt::Write as _;
        let _ = write!(out, "{byte:02x}");
    }
    out
}
