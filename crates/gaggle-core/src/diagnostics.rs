use anyhow::Result;
use serde::{Deserialize, Serialize};
use sqlx::SqlitePool;
use std::path::Path;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DiagnosticReport {
    pub service: String,
    pub version: String,
    pub os: String,
    pub arch: String,
    pub db_path: String,
    pub db_healthy: bool,
    pub journal_mode: String,
    pub foreign_keys_enabled: bool,
    pub applied_migrations: i64,
    pub active_projects_count: i64,
    pub enabled_extensions_count: i64,
    pub pending_approvals_count: i64,
}

/// Builds a redacted, non-sensitive diagnostic report inspecting runtime environment
/// and SQLite health.
pub async fn collect_diagnostics(pool: &SqlitePool, db_path: &Path) -> Result<DiagnosticReport> {
    let journal_mode: String = sqlx::query_scalar("PRAGMA journal_mode")
        .fetch_one(pool)
        .await
        .unwrap_or_else(|_| "unknown".to_string());

    let fk_flag: i64 = sqlx::query_scalar("PRAGMA foreign_keys")
        .fetch_one(pool)
        .await
        .unwrap_or(0);

    let applied_migrations: i64 =
        sqlx::query_scalar("SELECT COUNT(*) FROM _sqlx_migrations WHERE success = 1")
            .fetch_one(pool)
            .await
            .unwrap_or(0);

    let active_projects_count: i64 =
        sqlx::query_scalar("SELECT COUNT(*) FROM projects WHERE archived_at IS NULL")
            .fetch_one(pool)
            .await
            .unwrap_or(0);

    let enabled_extensions_count: i64 =
        sqlx::query_scalar("SELECT COUNT(*) FROM extensions WHERE enabled = 1")
            .fetch_one(pool)
            .await
            .unwrap_or(0);

    let pending_approvals_count: i64 =
        sqlx::query_scalar("SELECT COUNT(*) FROM approvals WHERE status = 'pending'")
            .fetch_one(pool)
            .await
            .unwrap_or(0);

    Ok(DiagnosticReport {
        service: "gaggle".to_string(),
        version: env!("CARGO_PKG_VERSION").to_string(),
        os: std::env::consts::OS.to_string(),
        arch: std::env::consts::ARCH.to_string(),
        db_path: db_path.display().to_string(),
        db_healthy: fk_flag == 1 && applied_migrations > 0,
        journal_mode,
        foreign_keys_enabled: fk_flag == 1,
        applied_migrations,
        active_projects_count,
        enabled_extensions_count,
        pending_approvals_count,
    })
}
