use anyhow::{Context, Result};
use directories::ProjectDirs;
use sqlx::{
    SqlitePool,
    sqlite::{SqliteConnectOptions, SqliteJournalMode, SqlitePoolOptions, SqliteSynchronous},
};
use std::{
    path::{Path, PathBuf},
    str::FromStr,
    time::Duration,
};
use uuid::Uuid;

/// Returns the global data directory for Gaggle:
/// - macOS: `~/Library/Application Support/gaggle/`
/// - Linux: `~/.local/share/gaggle/`
pub fn default_data_dir() -> Result<PathBuf> {
    let proj_dirs = ProjectDirs::from("", "", "gaggle")
        .context("Failed to determine OS application data directory for gaggle")?;
    let data_dir = proj_dirs.data_dir().to_path_buf();
    std::fs::create_dir_all(&data_dir).with_context(|| {
        format!(
            "Failed to create gaggle data directory at {}",
            data_dir.display()
        )
    })?;
    Ok(data_dir)
}

/// Returns the default SQLite path (`<data_dir>/gaggle.db`).
pub fn default_db_path() -> Result<PathBuf> {
    Ok(default_data_dir()?.join("gaggle.db"))
}

/// Generates a time-ordered UUIDv7 string for primary keys.
pub fn new_id() -> String {
    Uuid::now_v7().to_string()
}

/// Returns the current Unix epoch timestamp in milliseconds.
pub fn now_ms() -> i64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .expect("System clock before UNIX_EPOCH")
        .as_millis() as i64
}

/// Opens the SQLite database at `db_path`, applies all required PRAGMAs:
/// - `journal_mode = WAL`
/// - `foreign_keys = ON`
/// - `synchronous = NORMAL`
/// - `busy_timeout = 5000`
/// - `auto_vacuum = INCREMENTAL`
///
/// and runs `sqlx::migrate!("./migrations")`.
pub async fn connect_db(db_path: &Path) -> Result<SqlitePool> {
    if let Some(parent) = db_path.parent() {
        std::fs::create_dir_all(parent)?;
    }

    let db_url = format!("sqlite://{}?mode=rwc", db_path.display());
    let options = SqliteConnectOptions::from_str(&db_url)?
        .create_if_missing(true)
        .journal_mode(SqliteJournalMode::Wal)
        .foreign_keys(true)
        .synchronous(SqliteSynchronous::Normal)
        .busy_timeout(Duration::from_millis(5000))
        .pragma("auto_vacuum", "INCREMENTAL");

    let pool = SqlitePoolOptions::new()
        .max_connections(10)
        .connect_with(options)
        .await
        .with_context(|| {
            format!(
                "Failed to connect to SQLite database at {}",
                db_path.display()
            )
        })?;

    sqlx::migrate!("./migrations")
        .run(&pool)
        .await
        .context("Failed to run Gaggle database migrations")?;

    Ok(pool)
}

/// Opens an in-memory SQLite pool with migrations applied (ideal for tests).
pub async fn connect_in_memory() -> Result<SqlitePool> {
    let options = SqliteConnectOptions::from_str("sqlite::memory:")?
        .journal_mode(SqliteJournalMode::Wal)
        .foreign_keys(true)
        .synchronous(SqliteSynchronous::Normal)
        .busy_timeout(Duration::from_millis(5000))
        .pragma("auto_vacuum", "INCREMENTAL");

    let pool = SqlitePoolOptions::new()
        .max_connections(1)
        .connect_with(options)
        .await?;

    sqlx::migrate!("./migrations").run(&pool).await?;
    Ok(pool)
}
