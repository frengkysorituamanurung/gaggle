//! WAL-safe SQLite snapshots and offline database verification.
//!
//! Backups use SQLite's `VACUUM INTO`, which produces a consistent copy of a
//! live database (including any content still held in the WAL) without copying
//! `-wal`/`-shm` sidecar files or requiring application write locks to be held
//! across network calls.

use anyhow::{Context, Result, bail};
use serde::{Deserialize, Serialize};
use sqlx::{
    Connection,
    sqlite::{SqliteConnectOptions, SqliteConnection},
};
use std::{
    path::{Path, PathBuf},
    time::Duration,
};

/// How long a snapshot/verification connection waits on a busy database.
const BUSY_TIMEOUT: Duration = Duration::from_secs(10);

/// Summary produced by [`verify_database_file`].
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct DatabaseVerification {
    /// Result of `PRAGMA integrity_check`; always `"ok"` when verification succeeds.
    pub integrity: String,
    /// Number of successfully applied `sqlx` migrations.
    pub applied_migrations: i64,
    pub projects: i64,
    pub chats: i64,
    pub messages: i64,
    pub memories: i64,
}

/// Creates a consistent snapshot of the SQLite database at `db_path` in `dest`.
///
/// The destination must not exist yet; callers should stage snapshots in a
/// temporary directory and move them into place only after verification.
pub async fn snapshot_database(db_path: &Path, dest: &Path) -> Result<()> {
    if !db_path.is_file() {
        bail!(
            "SQLite database not found at {}; nothing to back up",
            db_path.display()
        );
    }
    if dest.exists() {
        bail!(
            "Refusing to overwrite existing snapshot target {}",
            dest.display()
        );
    }
    if let Some(parent) = dest.parent() {
        std::fs::create_dir_all(parent)
            .with_context(|| format!("Failed to create snapshot directory {}", parent.display()))?;
    }

    // `VACUUM INTO` does not accept bound parameters, so the target path is
    // embedded as an escaped SQL string literal. Only internally generated
    // staging paths are passed here, and non-UTF-8 paths are rejected instead
    // of being lossily converted into a different path.
    let dest_str = dest.to_str().ok_or_else(|| {
        anyhow::anyhow!(
            "Snapshot destination path is not valid UTF-8: {}",
            dest.display()
        )
    })?;
    let escaped = dest_str.replace('\'', "''");

    let options = SqliteConnectOptions::new()
        .filename(db_path)
        .create_if_missing(false)
        .busy_timeout(BUSY_TIMEOUT);

    let mut conn = SqliteConnection::connect_with(&options)
        .await
        .with_context(|| format!("Failed to open database at {}", db_path.display()))?;

    let snapshot_result = sqlx::query(sqlx::AssertSqlSafe(format!("VACUUM INTO '{escaped}'")))
        .execute(&mut conn)
        .await;

    // Close the dedicated connection before returning so the caller never
    // holds a SQLite lock while archiving or performing network work.
    let close_result = conn.close().await;

    snapshot_result.with_context(|| {
        format!(
            "Failed to create WAL-safe SQLite snapshot for {}",
            db_path.display()
        )
    })?;
    close_result.with_context(|| {
        format!(
            "Failed to close snapshot connection for {}",
            db_path.display()
        )
    })?;

    if !dest.is_file() {
        bail!("SQLite snapshot was not created at {}", dest.display());
    }

    Ok(())
}

/// Opens a database file read-only and validates its integrity and table counts.
///
/// This is used to prove that a staged snapshot is a usable Gaggle database
/// before any existing data is replaced.
pub async fn verify_database_file(db_path: &Path) -> Result<DatabaseVerification> {
    if !db_path.is_file() {
        bail!("Database file not found at {}", db_path.display());
    }

    let options = SqliteConnectOptions::new()
        .filename(db_path)
        .create_if_missing(false)
        .read_only(true)
        .busy_timeout(BUSY_TIMEOUT);

    let mut conn = SqliteConnection::connect_with(&options)
        .await
        .with_context(|| format!("Failed to open database snapshot {}", db_path.display()))?;

    let result = verify_open_database(&mut conn).await;
    let close_result = conn.close().await;
    let verification = result?;
    close_result.with_context(|| {
        format!(
            "Failed to close verification connection for {}",
            db_path.display()
        )
    })?;

    Ok(verification)
}

async fn verify_open_database(conn: &mut SqliteConnection) -> Result<DatabaseVerification> {
    let integrity: String = sqlx::query_scalar("PRAGMA integrity_check")
        .fetch_one(&mut *conn)
        .await
        .context("Failed to run PRAGMA integrity_check on database snapshot")?;

    if integrity != "ok" {
        bail!("Database snapshot failed integrity_check: {integrity}");
    }

    let migrations_table: Option<String> = sqlx::query_scalar(
        "SELECT name FROM sqlite_master WHERE type = 'table' AND name = '_sqlx_migrations'",
    )
    .fetch_optional(&mut *conn)
    .await?;

    let applied_migrations = match migrations_table {
        Some(_) => {
            sqlx::query_scalar("SELECT COUNT(*) FROM _sqlx_migrations WHERE success = 1")
                .fetch_one(&mut *conn)
                .await?
        }
        None => 0,
    };

    if applied_migrations == 0 {
        bail!(
            "Database snapshot contains no applied Gaggle migrations; refusing to treat it as a valid backup"
        );
    }

    let projects = count_rows(conn, "projects").await?;
    let chats = count_rows(conn, "chats").await?;
    let messages = count_rows(conn, "messages").await?;
    let memories = count_rows(conn, "memories").await?;

    Ok(DatabaseVerification {
        integrity,
        applied_migrations,
        projects,
        chats,
        messages,
        memories,
    })
}

async fn count_rows(conn: &mut SqliteConnection, table: &str) -> Result<i64> {
    // `table` is a compile-time constant supplied by this module, never user input.
    let sql = format!("SELECT COUNT(*) FROM {table}");
    let count = sqlx::query_scalar(sqlx::AssertSqlSafe(sql))
        .fetch_one(&mut *conn)
        .await
        .with_context(|| format!("Failed to count rows in {table}"))?;
    Ok(count)
}

/// Returns the conventional attachments directory next to a database file.
pub fn attachments_dir(db_path: &Path) -> Option<PathBuf> {
    db_path.parent().map(|parent| parent.join("attachments"))
}
