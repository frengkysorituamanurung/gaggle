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
        self.create_snapshot_with_summary(chat_id, None).await
    }

    /// Create a snapshot with an optional summary.
    pub async fn create_snapshot_with_summary(
        &self,
        chat_id: &str,
        summary: Option<String>,
    ) -> Result<ChatSnapshot, sqlx::Error> {
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
            INSERT INTO chat_snapshots (id, chat_id, message_count, summary, created_at)
            VALUES (?, ?, ?, ?, ?)
            "#,
        )
        .bind(&id)
        .bind(chat_id)
        .bind(message_count)
        .bind(&summary)
        .bind(now)
        .execute(&self.pool)
        .await?;

        Ok(ChatSnapshot {
            id,
            chat_id: chat_id.to_string(),
            message_count,
            summary,
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

    /// Get a single snapshot by id.
    pub async fn get_snapshot(
        &self,
        snapshot_id: &str,
    ) -> Result<Option<ChatSnapshot>, sqlx::Error> {
        sqlx::query_as::<_, ChatSnapshot>("SELECT * FROM chat_snapshots WHERE id = ?")
            .bind(snapshot_id)
            .fetch_optional(&self.pool)
            .await
    }

    /// Delete a snapshot.
    pub async fn delete_snapshot(&self, snapshot_id: &str) -> Result<(), sqlx::Error> {
        sqlx::query("DELETE FROM chat_snapshots WHERE id = ?")
            .bind(snapshot_id)
            .execute(&self.pool)
            .await?;
        Ok(())
    }

    /// Restore a chat to the state captured by a snapshot by truncating
    /// messages beyond the snapshot's message count.
    pub async fn restore_snapshot(
        &self,
        chat_id: &str,
        snapshot_id: &str,
    ) -> Result<ChatSnapshot, sqlx::Error> {
        let snapshot = self
            .get_snapshot(snapshot_id)
            .await?
            .ok_or_else(|| sqlx::Error::RowNotFound)?;
        if snapshot.chat_id != chat_id {
            return Err(sqlx::Error::RowNotFound);
        }

        let mut tx = self.pool.begin().await?;
        // Delete message parts for messages beyond the snapshot count.
        sqlx::query(
            r#"
            DELETE FROM message_parts
            WHERE message_id IN (
                SELECT id FROM messages
                WHERE chat_id = ? AND seq > ?
            )
            "#,
        )
        .bind(chat_id)
        .bind(snapshot.message_count)
        .execute(&mut *tx)
        .await?;
        sqlx::query("DELETE FROM messages WHERE chat_id = ? AND seq > ?")
            .bind(chat_id)
            .bind(snapshot.message_count)
            .execute(&mut *tx)
            .await?;
        // Drop summaries that cover beyond the restored point.
        sqlx::query("DELETE FROM chat_summaries WHERE chat_id = ? AND covers_until_seq > ?")
            .bind(chat_id)
            .bind(snapshot.message_count)
            .execute(&mut *tx)
            .await?;
        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.as_millis() as i64)
            .unwrap_or(0);
        sqlx::query("UPDATE chats SET updated_at = ? WHERE id = ?")
            .bind(now)
            .bind(chat_id)
            .execute(&mut *tx)
            .await?;
        tx.commit().await?;

        Ok(snapshot)
    }

    /// Undo: restore to the most recent snapshot whose message count is
    /// strictly less than the current message count.
    pub async fn undo(&self, chat_id: &str) -> Result<Option<ChatSnapshot>, sqlx::Error> {
        let current_count: i64 =
            sqlx::query_scalar("SELECT COUNT(*) FROM messages WHERE chat_id = ?")
                .bind(chat_id)
                .fetch_one(&self.pool)
                .await?;

        let snapshots = self.list_snapshots(chat_id).await?;
        // list is newest first; find the newest snapshot with smaller count.
        let target = snapshots
            .into_iter()
            .filter(|s| s.message_count < current_count)
            .max_by_key(|s| s.created_at);
        match target {
            Some(snap) => {
                let id = snap.id.clone();
                self.restore_snapshot(chat_id, &id).await.map(Some)
            }
            None => Ok(None),
        }
    }

    /// Redo: restore to the nearest snapshot whose message count is
    /// strictly greater than the current message count.
    ///
    /// Note: after a destructive undo, original messages are removed, so
    /// redo can only restore to a snapshot whose content still exists.
    /// When no such snapshot exists, returns `Ok(None)`.
    pub async fn redo(&self, chat_id: &str) -> Result<Option<ChatSnapshot>, sqlx::Error> {
        let current_count: i64 =
            sqlx::query_scalar("SELECT COUNT(*) FROM messages WHERE chat_id = ?")
                .bind(chat_id)
                .fetch_one(&self.pool)
                .await?;

        let snapshots = self.list_snapshots(chat_id).await?;
        let target = snapshots
            .into_iter()
            .filter(|s| s.message_count > current_count)
            .min_by_key(|s| s.message_count);
        match target {
            Some(snap) => {
                let id = snap.id.clone();
                self.restore_snapshot(chat_id, &id).await.map(Some)
            }
            None => Ok(None),
        }
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
