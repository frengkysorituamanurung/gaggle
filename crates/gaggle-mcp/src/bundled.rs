use anyhow::Result;
use gaggle_db::repositories::extensions::ExtensionRepository;
use crate::{ExtensionRegistry, InstallExtensionRequest};
use serde_json::Value;
use sqlx::SqlitePool;
use std::collections::HashSet;

/// A bundled MCP server definition shipped with Gaggle.
///
/// The list is intentionally empty in Phase 4; concrete entries can be added
/// here without touching the install path.
#[derive(Debug, Clone)]
pub struct BundledMcpServer {
    pub name: &'static str,
    pub description: Option<&'static str>,
    pub kind: &'static str,
    pub version: &'static str,
    pub source: &'static str,
    pub command: Option<&'static str>,
    pub args: &'static [&'static str],
    pub url: Option<&'static str>,
    pub env_secret_refs: Option<&'static str>,
    pub timeout_secs: i64,
    pub manifest: Option<&'static str>,
    pub bundled: bool,
    pub enabled: bool,
}

/// Empty bootstrap manifest for bundled MCP servers.
pub static BUNDLED_MCP_SERVERS: &[BundledMcpServer] = &[];

/// Installs every bundled MCP server into the database.
///
/// Installation is idempotent by server name: servers whose name already
/// exists are skipped.
pub async fn install_bundled(pool: SqlitePool) -> Result<()> {
    let registry = ExtensionRegistry::new(pool.clone());
    let repo = ExtensionRepository::new(pool.clone());
    let existing: HashSet<String> = repo
        .list_all_extensions()
        .await
        .unwrap_or_default()
        .into_iter()
        .map(|e| e.name)
        .collect();

    for server in BUNDLED_MCP_SERVERS {
        if existing.contains(server.name) {
            continue;
        }

        let args: Vec<String> = server.args.iter().map(|s| s.to_string()).collect();
        let env_secret_refs = server
            .env_secret_refs
            .and_then(|s| serde_json::from_str::<Value>(s).ok());
        let manifest = server
            .manifest
            .and_then(|s| serde_json::from_str::<Value>(s).ok());

        let req = InstallExtensionRequest {
            name: server.name.to_string(),
            description: server.description.map(|s| s.to_string()),
            kind: server.kind.to_string(),
            version: server.version.to_string(),
            source: server.source.to_string(),
            command: server.command.map(|s| s.to_string()),
            args,
            url: server.url.map(|s| s.to_string()),
            env_secret_refs,
            timeout_secs: server.timeout_secs,
            manifest,
            bundled: server.bundled,
            enabled: server.enabled,
        };

        let _ = registry.install_extension(req).await?;
    }

    Ok(())
}