//! Integration tests for Issue #22: WAL-safe SQLite snapshots.

use anyhow::Result;
use gaggle_db::{
    backup::{snapshot_database, verify_database_file},
    connect_db,
    repositories::{chats::ChatRepository, projects::ProjectRepository},
};

#[tokio::test]
async fn snapshot_includes_uncheckpointed_wal_content_and_verifies() -> Result<()> {
    let temp = tempfile::tempdir()?;
    let db_path = temp.path().join("gaggle.db");
    let snapshot_path = temp.path().join("snapshot.db");

    let pool = connect_db(&db_path).await?;

    let projects = ProjectRepository::new(pool.clone());
    let chats = ChatRepository::new(pool.clone());
    let project = projects
        .create("snapshot-project", "/tmp/gaggle-issue22-snapshot", "git")
        .await?;
    let chat = chats
        .create(&project.id, "snapshot chat", "user", "local", "ask")
        .await?;
    chats
        .append_message(&chat.id, None, "user", "durable message")
        .await?;

    let wal_path = std::path::PathBuf::from(format!("{}-wal", db_path.display()));
    assert!(
        wal_path.exists() && std::fs::metadata(&wal_path)?.len() > 0,
        "test expects uncheckpointed WAL content before snapshotting"
    );

    snapshot_database(&db_path, &snapshot_path).await?;
    assert!(snapshot_path.is_file());
    assert!(!temp.path().join("snapshot.db-wal").exists());

    let verification = verify_database_file(&snapshot_path).await?;
    assert_eq!(verification.integrity, "ok");
    assert_eq!(verification.projects, 1);
    assert_eq!(verification.chats, 1);
    assert_eq!(verification.messages, 1);
    assert!(verification.applied_migrations > 0);

    // The live pool still works after the snapshot.
    drop(pool);

    Ok(())
}

#[tokio::test]
async fn snapshot_refuses_to_overwrite_and_requires_source() -> Result<()> {
    let temp = tempfile::tempdir()?;
    let db_path = temp.path().join("gaggle.db");
    let snapshot_path = temp.path().join("snapshot.db");

    assert!(snapshot_database(&db_path, &snapshot_path).await.is_err());

    let pool = connect_db(&db_path).await?;
    drop(pool);

    std::fs::write(&snapshot_path, b"existing")?;
    assert!(snapshot_database(&db_path, &snapshot_path).await.is_err());

    Ok(())
}

#[tokio::test]
async fn verify_rejects_corrupt_or_non_gaggle_database() -> Result<()> {
    let temp = tempfile::tempdir()?;
    let corrupt = temp.path().join("corrupt.db");
    std::fs::write(&corrupt, b"not a sqlite database")?;

    let result = verify_database_file(&corrupt).await;
    assert!(result.is_err(), "corrupt database must fail verification");

    let missing = temp.path().join("missing.db");
    assert!(verify_database_file(&missing).await.is_err());

    // A valid SQLite database without Gaggle migrations is not a valid backup.
    let empty = temp.path().join("empty.db");
    let pool = sqlx::SqlitePool::connect(&format!("sqlite://{}?mode=rwc", empty.display())).await?;
    sqlx::query("CREATE TABLE unrelated (id TEXT PRIMARY KEY)")
        .execute(&pool)
        .await?;
    pool.close().await;

    let result = verify_database_file(&empty).await;
    assert!(result.is_err(), "database without migrations must fail");

    Ok(())
}
