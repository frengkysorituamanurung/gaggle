//! Validated chat bundle export and import used by `gaggle export/import chat`.

use anyhow::{Context, Result, bail};
use gaggle_db::repositories::chats::{
    CHAT_BUNDLE_FORMAT, CHAT_BUNDLE_VERSION, ChatBundle, ChatImportResult, ChatRepository,
};
use serde::{Deserialize, Serialize};
use sqlx::SqlitePool;
use std::{
    fs,
    io::{Read, Write},
    path::{Path, PathBuf},
};

/// Upper bound on a chat bundle read from disk. The limit exists to keep a
/// malformed or hostile file from exhausting memory; exported bundles above
/// this size must be split before they can be re-imported.
pub const MAX_CHAT_BUNDLE_BYTES: u64 = 64 * 1024 * 1024;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChatExportSummary {
    pub format: String,
    pub version: u32,
    pub chat_id: String,
    pub output: PathBuf,
    pub messages: usize,
    pub message_parts: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChatImportSummary {
    #[serde(flatten)]
    pub result: ChatImportResult,
    pub project_id: String,
    pub source: PathBuf,
}

/// Exports `chat_id` as a portable, versioned JSON bundle written atomically.
pub async fn export_chat(
    pool: &SqlitePool,
    chat_id: &str,
    output: &Path,
) -> Result<ChatExportSummary> {
    let repo = ChatRepository::new(pool.clone());
    let export = repo
        .export_chat(chat_id)
        .await?
        .ok_or_else(|| anyhow::anyhow!("Chat {chat_id} not found"))?;

    let bundle = ChatBundle::from_export(export);
    write_chat_bundle(output, &bundle)?;

    Ok(ChatExportSummary {
        format: CHAT_BUNDLE_FORMAT.to_string(),
        version: CHAT_BUNDLE_VERSION,
        chat_id: bundle.chat.id,
        output: output.to_path_buf(),
        messages: bundle.messages.len(),
        message_parts: bundle.message_parts.len(),
    })
}

/// Reads and validates a chat bundle, then imports it as a new chat.
pub async fn import_chat(
    pool: &SqlitePool,
    project_id: &str,
    source: &Path,
) -> Result<ChatImportSummary> {
    let bundle = read_chat_bundle(source)?;
    let repo = ChatRepository::new(pool.clone());
    let result = repo.import_chat(project_id, &bundle).await?;

    Ok(ChatImportSummary {
        result,
        project_id: project_id.to_string(),
        source: source.to_path_buf(),
    })
}

/// Reads a chat bundle with a hard size limit and validates its structure.
pub fn read_chat_bundle(path: &Path) -> Result<ChatBundle> {
    let metadata = fs::metadata(path)
        .with_context(|| format!("Failed to read chat bundle metadata at {}", path.display()))?;
    if !metadata.is_file() {
        bail!("Chat bundle path is not a regular file: {}", path.display());
    }
    if metadata.len() > MAX_CHAT_BUNDLE_BYTES {
        bail!(
            "Chat bundle {} is {} bytes; the maximum supported size is {} bytes",
            path.display(),
            metadata.len(),
            MAX_CHAT_BUNDLE_BYTES
        );
    }

    let mut bytes = Vec::with_capacity(metadata.len() as usize);
    fs::File::open(path)
        .with_context(|| format!("Failed to open chat bundle {}", path.display()))?
        .take(MAX_CHAT_BUNDLE_BYTES + 1)
        .read_to_end(&mut bytes)
        .with_context(|| format!("Failed to read chat bundle {}", path.display()))?;
    if bytes.len() as u64 > MAX_CHAT_BUNDLE_BYTES {
        bail!(
            "Chat bundle {} exceeds the {} byte limit",
            path.display(),
            MAX_CHAT_BUNDLE_BYTES
        );
    }

    let bundle: ChatBundle = serde_json::from_slice(&bytes)
        .with_context(|| format!("Chat bundle {} is not valid JSON", path.display()))?;
    bundle.validate()?;
    Ok(bundle)
}

/// Writes a chat bundle atomically so an interrupted export never leaves a
/// truncated file at the destination path.
pub fn write_chat_bundle(path: &Path, bundle: &ChatBundle) -> Result<()> {
    if path.as_os_str().is_empty() {
        bail!("Chat bundle output path must not be empty");
    }
    if path.is_dir() {
        bail!("Chat bundle output path is a directory: {}", path.display());
    }

    let bytes = serde_json::to_vec_pretty(bundle).context("Failed to serialize chat bundle")?;

    let parent = path
        .parent()
        .filter(|parent| !parent.as_os_str().is_empty())
        .unwrap_or_else(|| Path::new("."));
    fs::create_dir_all(parent)
        .with_context(|| format!("Failed to create output directory {}", parent.display()))?;

    let mut temp = tempfile::NamedTempFile::new_in(parent)
        .with_context(|| format!("Failed to create temporary file in {}", parent.display()))?;
    temp.write_all(&bytes)
        .context("Failed to write chat bundle")?;
    temp.as_file()
        .sync_all()
        .context("Failed to flush chat bundle to disk")?;
    temp.persist(path).map_err(|error| {
        anyhow::anyhow!("Failed to write chat bundle to {}: {error}", path.display())
    })?;
    Ok(())
}
