//! End-to-end CLI tests for Issue #22: chat export/import, memory export,
//! checksummed backup, and safe restore.
//!
//! These tests exercise the real `gaggle` binary via `CARGO_BIN_EXE_gaggle` and
//! operate entirely inside temporary directories.

use anyhow::{Context, Result, bail};
use flate2::{Compression, read::GzDecoder, write::GzEncoder};
use gaggle_db::{
    connect_db,
    repositories::{
        chats::ChatRepository, memories::MemoryRepository, projects::ProjectRepository,
    },
};
use std::{
    fs::{self, File},
    io::{BufReader, Read, Write},
    path::{Path, PathBuf},
    process::{Command, Output},
};
use tar::{Archive, Builder, EntryType, Header};
use tempfile::TempDir;

const BIN: &str = env!("CARGO_BIN_EXE_gaggle");

fn run(db: &Path, args: &[&str]) -> Output {
    Command::new(BIN)
        .arg("--db")
        .arg(db)
        .args(args)
        .env("RUST_LOG", "error")
        .output()
        .expect("failed to execute gaggle binary")
}

fn stderr(output: &Output) -> String {
    String::from_utf8_lossy(&output.stderr).to_string()
}

fn stdout(output: &Output) -> String {
    String::from_utf8_lossy(&output.stdout).to_string()
}

fn assert_success(output: &Output) {
    assert!(
        output.status.success(),
        "command failed with status {:?}: {}",
        output.status.code(),
        stderr(output)
    );
}

fn assert_failure(output: &Output, needle: &str) {
    assert!(
        !output.status.success(),
        "command unexpectedly succeeded: {}",
        stdout(output)
    );
    let message = stderr(output);
    assert!(
        message.contains(needle),
        "expected stderr to contain {needle:?}, got: {message}"
    );
}

struct Seeded {
    project_id: String,
    chat_id: String,
}

async fn seed_project_chat(db: &Path, root: &Path, title: &str) -> Result<Seeded> {
    fs::create_dir_all(root)?;
    let pool = connect_db(db).await?;
    let projects = ProjectRepository::new(pool.clone());
    let chats = ChatRepository::new(pool.clone());

    let project = projects
        .create(title, &root.to_string_lossy(), "git")
        .await?;
    let chat = chats
        .create(
            &project.id,
            &format!("{title} chat"),
            "user",
            "local",
            "ask",
        )
        .await?;
    chats
        .append_message(&chat.id, None, "user", "Hello portable world")
        .await?;
    chats
        .append_message(&chat.id, None, "assistant", "Hello back")
        .await?;

    pool.close().await;
    Ok(Seeded {
        project_id: project.id,
        chat_id: chat.id,
    })
}

async fn chat_count(db: &Path) -> Result<i64> {
    let pool = connect_db(db).await?;
    let count = sqlx::query_scalar("SELECT COUNT(*) FROM chats")
        .fetch_one(&pool)
        .await?;
    pool.close().await;
    Ok(count)
}

fn read_archive(path: &Path) -> Result<Vec<(String, Vec<u8>)>> {
    let file = File::open(path)?;
    let mut archive = Archive::new(GzDecoder::new(BufReader::new(file)));
    let mut entries = Vec::new();
    for entry in archive.entries()? {
        let mut entry = entry?;
        let name = entry.path()?.to_string_lossy().to_string();
        let mut data = Vec::new();
        entry.read_to_end(&mut data)?;
        entries.push((name, data));
    }
    Ok(entries)
}

fn write_archive(path: &Path, entries: &[(String, Vec<u8>)]) -> Result<()> {
    let file = File::create(path)?;
    let encoder = GzEncoder::new(file, Compression::default());
    let mut builder = Builder::new(encoder);
    for (name, data) in entries {
        let mut header = Header::new_gnu();
        header.set_entry_type(EntryType::Regular);
        header.set_size(data.len() as u64);
        header.set_mode(0o644);
        header.set_mtime(0);
        // `set_path` rejects absolute and `..` paths, so the test writes the
        // raw name field directly to simulate a malicious archive.
        header.set_path("placeholder")?;
        let bytes = name.as_bytes();
        if bytes.len() > 100 {
            bail!("test archive entry name too long: {name}");
        }
        header.as_mut_bytes()[0..100].fill(0);
        header.as_mut_bytes()[..bytes.len()].copy_from_slice(bytes);
        header.set_cksum();
        builder.append(&header, data.as_slice())?;
    }
    let encoder = builder.into_inner()?;
    let mut file = encoder.finish()?;
    file.flush()?;
    Ok(())
}

fn manifest_from(entries: &[(String, Vec<u8>)]) -> Result<serde_json::Value> {
    let manifest = entries
        .iter()
        .find(|(name, _)| name == "manifest.json")
        .map(|(_, data)| data)
        .context("archive has no manifest.json")?;
    Ok(serde_json::from_slice(manifest)?)
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn cli_chat_export_and_import_round_trip() -> Result<()> {
    let temp = TempDir::new()?;
    let db = temp.path().join("gaggle.db");
    let source = seed_project_chat(&db, &temp.path().join("source"), "source").await?;
    let target = seed_project_chat(&db, &temp.path().join("target"), "target").await?;
    let bundle_path = temp.path().join("chat.json");

    let export = run(
        &db,
        &[
            "export",
            "chat",
            &source.chat_id,
            "--output",
            bundle_path.to_str().expect("utf-8 path"),
        ],
    );
    assert_success(&export);

    let bundle: serde_json::Value = serde_json::from_slice(&fs::read(&bundle_path)?)?;
    assert_eq!(bundle["format"], "gaggle.chat-bundle");
    assert_eq!(bundle["version"], 1);
    assert_eq!(bundle["chat"]["id"], source.chat_id);
    assert_eq!(bundle["messages"].as_array().expect("messages").len(), 2);
    assert_eq!(bundle["message_parts"].as_array().expect("parts").len(), 2);
    assert!(bundle.get("chat").is_some());
    assert!(bundle.get("messages").is_some());
    assert!(bundle.get("message_parts").is_some());

    let import = run(
        &db,
        &[
            "import",
            "chat",
            bundle_path.to_str().expect("utf-8 path"),
            "--project",
            &target.project_id,
        ],
    );
    assert_success(&import);
    let summary: serde_json::Value = serde_json::from_slice(&import.stdout)?;
    let imported_id = summary["chat"]["id"].as_str().context("imported chat id")?;
    assert_ne!(imported_id, source.chat_id);
    assert_eq!(summary["chat"]["project_id"], target.project_id);
    assert_eq!(summary["message_count"], 2);
    assert_eq!(summary["part_count"], 2);

    let pool = connect_db(&db).await?;
    let chats = ChatRepository::new(pool.clone());
    let exported = chats
        .export_chat(imported_id)
        .await?
        .context("imported chat should exist")?;
    assert_eq!(
        exported.message_parts[0].content.as_deref(),
        Some("Hello portable world")
    );
    assert_eq!(
        exported.message_parts[1].content.as_deref(),
        Some("Hello back")
    );
    pool.close().await;

    let bad_bundle = temp.path().join("bad.json");
    fs::write(&bad_bundle, "{ not json")?;
    let invalid = run(
        &db,
        &[
            "import",
            "chat",
            bad_bundle.to_str().expect("utf-8 path"),
            "--project",
            &target.project_id,
        ],
    );
    assert_failure(&invalid, "not valid JSON");

    let unknown_project = run(
        &db,
        &[
            "import",
            "chat",
            bundle_path.to_str().expect("utf-8 path"),
            "--project",
            "01900000-0000-7000-8000-000000000000",
        ],
    );
    assert_failure(&unknown_project, "not found");

    Ok(())
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn cli_memory_export_writes_pal_directory() -> Result<()> {
    let temp = TempDir::new()?;
    let db = temp.path().join("gaggle.db");
    let root = temp.path().join("workspace");
    let seeded = seed_project_chat(&db, &root, "memory").await?;

    let pool = connect_db(&db).await?;
    let memories = MemoryRepository::new(pool.clone());
    memories
        .create_with_options(
            Some(&seeded.project_id),
            None,
            None,
            "decision",
            "Use WAL for durability",
            "api_key=sk-live-do-not-share",
            &["sqlite".to_string()],
            true,
        )
        .await?;
    memories
        .create(None, "fact", "Global preference", "Prefer small commits")
        .await?;
    pool.close().await;

    let export = run(&db, &["export", "memory", "--project", &seeded.project_id]);
    assert_success(&export);
    let report: serde_json::Value = serde_json::from_slice(&export.stdout)?;
    assert_eq!(
        report["output_dir"].as_str(),
        Some(
            fs::canonicalize(&root)?
                .join(".pal/memory")
                .to_string_lossy()
                .as_ref()
        )
    );
    assert_eq!(report["files"].as_array().expect("files").len(), 2);

    let memory_dir = root.join(".pal/memory");
    let mut names = Vec::new();
    for entry in fs::read_dir(&memory_dir)? {
        let entry = entry?;
        names.push(entry.file_name().to_string_lossy().to_string());
    }
    names.sort();
    assert_eq!(names.len(), 2);
    for name in &names {
        assert!(name.ends_with(".md"));
        assert!(name.starts_with("decision-") || name.starts_with("fact-"));
        assert!(!name.contains('/'));
        assert!(!name.contains(".."));
    }

    let exported_text = {
        let mut combined = String::new();
        for entry in fs::read_dir(&memory_dir)? {
            combined.push_str(&fs::read_to_string(entry?.path())?);
            combined.push('\n');
        }
        combined
    };
    assert!(exported_text.contains("Use WAL for durability"));
    assert!(exported_text.contains("Prefer small commits"));
    assert!(!exported_text.contains("sk-live-do-not-share"));

    let outside = temp.path().join("outside-memory");
    let escape = run(
        &db,
        &[
            "export",
            "memory",
            "--project",
            &seeded.project_id,
            "--output",
            outside.to_str().expect("utf-8 path"),
        ],
    );
    assert_failure(&escape, "outside project workspace");
    assert!(!outside.exists());

    Ok(())
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn cli_backup_and_restore_requires_confirmation() -> Result<()> {
    let temp = TempDir::new()?;
    let db = temp.path().join("gaggle.db");
    let attachments = temp.path().join("attachments");
    fs::create_dir_all(attachments.join("nested"))?;
    fs::write(attachments.join("nested/note.txt"), b"attachment body")?;

    let seed = seed_project_chat(&db, &temp.path().join("workspace"), "backup").await?;
    // Keep the pool open so the backup reads uncheckpointed WAL content.
    let live_pool = connect_db(&db).await?;

    let backup_path = temp.path().join("backup.tar.gz");
    let backup = run(
        &db,
        &[
            "backup",
            "--output",
            backup_path.to_str().expect("utf-8 path"),
        ],
    );
    assert_success(&backup);
    let report: serde_json::Value = serde_json::from_slice(&backup.stdout)?;
    assert_eq!(report["archive_sha256"].as_str().expect("sha").len(), 64);
    assert_eq!(report["manifest"]["attachments_included"], true);

    let entries = read_archive(&backup_path)?;
    let names = entries
        .iter()
        .map(|(name, _)| name.as_str())
        .collect::<Vec<_>>();
    assert!(names.contains(&"manifest.json"));
    assert!(names.contains(&"gaggle.db"));
    assert!(names.contains(&"attachments/nested/note.txt"));

    let manifest = manifest_from(&entries)?;
    let files = manifest["files"].as_array().expect("manifest files");
    let db_entry = files
        .iter()
        .find(|file| file["path"] == "gaggle.db")
        .context("manifest must list gaggle.db")?;
    let db_bytes = entries
        .iter()
        .find(|(name, _)| name == "gaggle.db")
        .map(|(_, data)| data)
        .context("archive must contain gaggle.db")?;
    assert_eq!(db_entry["size_bytes"].as_u64(), Some(db_bytes.len() as u64));
    assert_eq!(db_entry["sha256"].as_str().expect("sha").len(), 64);

    // Write a second chat after the backup so restore has something to remove.
    let chats = ChatRepository::new(live_pool.clone());
    let projects = ProjectRepository::new(live_pool.clone());
    let project = projects
        .get_by_id(&seed.project_id)
        .await?
        .context("project")?;
    chats
        .create(&project.id, "created after backup", "user", "local", "ask")
        .await?;
    assert_eq!(chat_count(&db).await?, 2);

    live_pool.close().await;

    let refused = run(&db, &["restore", backup_path.to_str().expect("utf-8 path")]);
    assert_failure(&refused, "--force");
    assert_eq!(chat_count(&db).await?, 2);

    let dry_run = run(
        &db,
        &[
            "restore",
            backup_path.to_str().expect("utf-8 path"),
            "--dry-run",
        ],
    );
    assert_success(&dry_run);
    let dry_report: serde_json::Value = serde_json::from_slice(&dry_run.stdout)?;
    assert_eq!(dry_report["dry_run"], true);
    assert_eq!(dry_report["replaced_existing"], true);
    assert_eq!(chat_count(&db).await?, 2);

    let restore = run(
        &db,
        &[
            "restore",
            backup_path.to_str().expect("utf-8 path"),
            "--force",
        ],
    );
    assert_success(&restore);
    let restore_report: serde_json::Value = serde_json::from_slice(&restore.stdout)?;
    assert_eq!(restore_report["dry_run"], false);
    assert_eq!(restore_report["replaced_existing"], true);
    assert_eq!(restore_report["attachments_restored"], 1);
    let previous = restore_report["previous_data_dir"]
        .as_str()
        .context("previous data dir")?;
    let previous = PathBuf::from(previous);
    assert!(previous.join("gaggle.db").is_file());
    assert!(previous.join("attachments/nested/note.txt").is_file());

    assert_eq!(chat_count(&db).await?, 1);
    assert_eq!(
        fs::read(attachments.join("nested/note.txt"))?,
        b"attachment body"
    );

    // A second backup must not clobber the existing archive without --force.
    let overwrite = run(
        &db,
        &[
            "backup",
            "--output",
            backup_path.to_str().expect("utf-8 path"),
        ],
    );
    assert_failure(&overwrite, "--force");

    let overwrite_force = run(
        &db,
        &[
            "backup",
            "--output",
            backup_path.to_str().expect("utf-8 path"),
            "--force",
        ],
    );
    assert_success(&overwrite_force);

    Ok(())
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn cli_restore_rejects_checksum_mismatch_without_touching_data() -> Result<()> {
    let temp = TempDir::new()?;
    let db = temp.path().join("gaggle.db");
    seed_project_chat(&db, &temp.path().join("workspace"), "checksum").await?;

    let good_archive = temp.path().join("good.tar.gz");
    assert_success(&run(
        &db,
        &[
            "backup",
            "--output",
            good_archive.to_str().expect("utf-8 path"),
        ],
    ));

    let mut entries = read_archive(&good_archive)?;
    let db_entry = entries
        .iter_mut()
        .find(|(name, _)| name == "gaggle.db")
        .context("gaggle.db entry")?;
    assert!(!db_entry.1.is_empty());
    db_entry.1[0] ^= 0xFF;

    let tampered = temp.path().join("tampered.tar.gz");
    write_archive(&tampered, &entries)?;

    let restore = run(
        &db,
        &["restore", tampered.to_str().expect("utf-8 path"), "--force"],
    );
    assert_failure(&restore, "Checksum verification failed");
    assert_eq!(chat_count(&db).await?, 1);

    Ok(())
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn cli_restore_rejects_path_traversal_entries() -> Result<()> {
    let temp = TempDir::new()?;
    let db = temp.path().join("gaggle.db");
    seed_project_chat(&db, &temp.path().join("workspace"), "traversal").await?;

    let good_archive = temp.path().join("good.tar.gz");
    assert_success(&run(
        &db,
        &[
            "backup",
            "--output",
            good_archive.to_str().expect("utf-8 path"),
        ],
    ));

    let mut entries = read_archive(&good_archive)?;
    entries.push(("../escape.txt".to_string(), b"escaped".to_vec()));
    let malicious = temp.path().join("malicious.tar.gz");
    write_archive(&malicious, &entries)?;

    let restore = run(
        &db,
        &[
            "restore",
            malicious.to_str().expect("utf-8 path"),
            "--force",
        ],
    );
    assert_failure(&restore, "unsafe path component");
    assert!(!temp.path().join("escape.txt").exists());
    assert_eq!(chat_count(&db).await?, 1);

    Ok(())
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn cli_restore_rejects_absolute_and_undeclared_entries() -> Result<()> {
    let temp = TempDir::new()?;
    let db = temp.path().join("gaggle.db");
    seed_project_chat(&db, &temp.path().join("workspace"), "layout").await?;

    let good_archive = temp.path().join("good.tar.gz");
    assert_success(&run(
        &db,
        &[
            "backup",
            "--output",
            good_archive.to_str().expect("utf-8 path"),
        ],
    ));

    let entries = read_archive(&good_archive)?;

    let mut absolute = entries.clone();
    absolute.push(("/tmp/gaggle-escape.txt".to_string(), b"escaped".to_vec()));
    let absolute_path = temp.path().join("absolute.tar.gz");
    write_archive(&absolute_path, &absolute)?;
    let restore = run(
        &db,
        &[
            "restore",
            absolute_path.to_str().expect("utf-8 path"),
            "--force",
        ],
    );
    assert_failure(&restore, "absolute path");

    let mut undeclared = entries;
    undeclared.push(("extra.txt".to_string(), b"not declared".to_vec()));
    let undeclared_path = temp.path().join("undeclared.tar.gz");
    write_archive(&undeclared_path, &undeclared)?;
    let restore = run(
        &db,
        &[
            "restore",
            undeclared_path.to_str().expect("utf-8 path"),
            "--force",
        ],
    );
    assert_failure(&restore, "outside the allowed backup layout");

    assert_eq!(chat_count(&db).await?, 1);

    Ok(())
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn cli_backup_preserves_long_attachment_names() -> Result<()> {
    let temp = TempDir::new()?;
    let db = temp.path().join("gaggle.db");
    let attachments = temp.path().join("attachments");
    fs::create_dir_all(&attachments)?;
    let long_name = format!("{}.bin", "attachment-".repeat(12));
    assert!(long_name.len() > 100);
    fs::write(attachments.join(&long_name), b"long name body")?;

    seed_project_chat(&db, &temp.path().join("workspace"), "long-name").await?;

    let backup_path = temp.path().join("backup.tar.gz");
    assert_success(&run(
        &db,
        &[
            "backup",
            "--output",
            backup_path.to_str().expect("utf-8 path"),
        ],
    ));

    fs::remove_dir_all(&attachments)?;
    assert_success(&run(
        &db,
        &[
            "restore",
            backup_path.to_str().expect("utf-8 path"),
            "--force",
        ],
    ));

    assert_eq!(fs::read(attachments.join(&long_name))?, b"long name body");

    Ok(())
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn cli_export_chat_missing_id_fails_cleanly() -> Result<()> {
    let temp = TempDir::new()?;
    let db = temp.path().join("gaggle.db");
    seed_project_chat(&db, &temp.path().join("workspace"), "missing").await?;

    let output = temp.path().join("missing.json");
    let export = run(
        &db,
        &[
            "export",
            "chat",
            "01900000-0000-7000-8000-000000000000",
            "--output",
            output.to_str().expect("utf-8 path"),
        ],
    );
    assert_failure(&export, "not found");
    assert!(!output.exists());

    Ok(())
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn cli_backup_requires_existing_database() -> Result<()> {
    let temp = TempDir::new()?;
    let db = temp.path().join("missing/gaggle.db");
    let output = temp.path().join("backup.tar.gz");

    let backup = run(
        &db,
        &["backup", "--output", output.to_str().expect("utf-8 path")],
    );
    if backup.status.success() {
        // Opening a database for backup must not create an empty database.
        bail!("backup unexpectedly created data for a missing database");
    }
    assert!(!output.exists());

    Ok(())
}
