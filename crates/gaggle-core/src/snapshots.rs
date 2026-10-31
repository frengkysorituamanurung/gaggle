//! Session snapshots for undo/redo support.
//!
//! Snapshots capture the state of a chat at a specific point in time,
//! allowing users to revert to a previous state.

use serde::{Deserialize, Serialize};

/// A snapshot of a chat's state.
#[derive(Debug, Clone, Serialize, Deserialize, sqlx::FromRow)]
pub struct ChatSnapshot {
    pub id: String,
    pub chat_id: String,
    pub message_count: i64,
    pub summary: Option<String>,
    pub created_at: i64,
}

/// Snapshot manager.
pub struct SnapshotManager {
    pool: sqlx::SqlitePool,
}

impl SnapshotManager {
    pub fn new(pool: sqlx::SqlitePool) -> Self {
        Self { pool }
    }

    /// Create a snapshot of the current chat state.
    pub async fn create_snapshot(&self, chat_id: &str) -> Result<ChatSnapshot, sqlx::Error> {
        let id = uuid::Uuid::new_v4().to_string();
        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.as_millis() as i64)
            .unwrap_or(0);

        let message_count: i64 =
            sqlx::query_scalar("SELECT COUNT(*) FROM messages WHERE chat_id = ?")
                .bind(chat_id)
                .fetch_one(&self.pool)
                .await?;

        sqlx::query(
            r#"
            INSERT INTO chat_snapshots (id, chat_id, message_count, created_at)
            VALUES (?, ?, ?, ?)
            "#,
        )
        .bind(&id)
        .bind(chat_id)
        .bind(message_count)
        .bind(now)
        .execute(&self.pool)
        .await?;

        Ok(ChatSnapshot {
            id,
            chat_id: chat_id.to_string(),
            message_count,
            summary: None,
            created_at: now,
        })
    }

    /// List snapshots for a chat.
    pub async fn list_snapshots(&self, chat_id: &str) -> Result<Vec<ChatSnapshot>, sqlx::Error> {
        sqlx::query_as::<_, ChatSnapshot>(
            "SELECT * FROM chat_snapshots WHERE chat_id = ? ORDER BY created_at DESC",
        )
        .bind(chat_id)
        .fetch_all(&self.pool)
        .await
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn snapshot_serializes() {
        let snap = ChatSnapshot {
            id: "1".to_string(),
            chat_id: "c1".to_string(),
            message_count: 5,
            summary: None,
            created_at: 1000,
        };
        let json = serde_json::to_string(&snap).unwrap();
        assert!(json.contains("c1"));
    }
}
