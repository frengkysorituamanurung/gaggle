use crate::{
    connection::{new_id, now_ms},
    models::{Chat, Message, MessagePart},
};
use anyhow::Result;
use serde::{Deserialize, Serialize};
use sqlx::SqlitePool;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChatExportBundle {
    pub chat: Chat,
    pub messages: Vec<Message>,
    pub message_parts: Vec<MessagePart>,
}

pub struct ChatRepository {
    pool: SqlitePool,
}

impl ChatRepository {
    pub fn new(pool: SqlitePool) -> Self {
        Self { pool }
    }

    pub async fn create(
        &self,
        project_id: &str,
        title: &str,
        chat_type: &str,
        mode: &str,
        permission_mode: &str,
    ) -> Result<Chat> {
        let id = new_id();
        let now = now_ms();

        sqlx::query(
            r#"
            INSERT INTO chats (
                id, project_id, chat_type, title, mode, permission_mode, created_at, updated_at
            ) VALUES (?, ?, ?, ?, ?, ?, ?, ?)
            "#,
        )
        .bind(&id)
        .bind(project_id)
        .bind(chat_type)
        .bind(title)
        .bind(mode)
        .bind(permission_mode)
        .bind(now)
        .bind(now)
        .execute(&self.pool)
        .await?;

        self.get_by_id(&id)
            .await?
            .ok_or_else(|| anyhow::anyhow!("Chat not found after insert"))
    }

    pub async fn get_by_id(&self, id: &str) -> Result<Option<Chat>> {
        let chat = sqlx::query_as::<_, Chat>("SELECT * FROM chats WHERE id = ?")
            .bind(id)
            .fetch_optional(&self.pool)
            .await?;
        Ok(chat)
    }

    pub async fn list_by_project(&self, project_id: &str) -> Result<Vec<Chat>> {
        let chats = sqlx::query_as::<_, Chat>(
            "SELECT * FROM chats WHERE project_id = ? AND archived_at IS NULL ORDER BY updated_at DESC",
        )
        .bind(project_id)
        .fetch_all(&self.pool)
        .await?;
        Ok(chats)
    }

    pub async fn append_message(
        &self,
        chat_id: &str,
        run_id: Option<&str>,
        role: &str,
        text_content: &str,
    ) -> Result<Message> {
        let id = new_id();
        let part_id = new_id();
        let now = now_ms();

        let next_seq: i64 =
            sqlx::query_scalar("SELECT COALESCE(MAX(seq), 0) + 1 FROM messages WHERE chat_id = ?")
                .bind(chat_id)
                .fetch_one(&self.pool)
                .await?;

        let mut tx = self.pool.begin().await?;

        sqlx::query(
            r#"
            INSERT INTO messages (id, chat_id, run_id, seq, role, created_at)
            VALUES (?, ?, ?, ?, ?, ?)
            "#,
        )
        .bind(&id)
        .bind(chat_id)
        .bind(run_id)
        .bind(next_seq)
        .bind(role)
        .bind(now)
        .execute(&mut *tx)
        .await?;

        sqlx::query(
            r#"
            INSERT INTO message_parts (id, message_id, idx, kind, content)
            VALUES (?, ?, 0, 'text', ?)
            "#,
        )
        .bind(&part_id)
        .bind(&id)
        .bind(text_content)
        .execute(&mut *tx)
        .await?;

        sqlx::query("UPDATE chats SET updated_at = ? WHERE id = ?")
            .bind(now)
            .bind(chat_id)
            .execute(&mut *tx)
            .await?;

        tx.commit().await?;

        let msg = sqlx::query_as::<_, Message>("SELECT * FROM messages WHERE id = ?")
            .bind(&id)
            .fetch_one(&self.pool)
            .await?;
        Ok(msg)
    }

    pub async fn export_chat(&self, chat_id: &str) -> Result<Option<ChatExportBundle>> {
        let Some(chat) = self.get_by_id(chat_id).await? else {
            return Ok(None);
        };

        let messages = sqlx::query_as::<_, Message>(
            "SELECT * FROM messages WHERE chat_id = ? ORDER BY seq ASC",
        )
        .bind(chat_id)
        .fetch_all(&self.pool)
        .await?;

        let message_parts = sqlx::query_as::<_, MessagePart>(
            r#"
            SELECT mp.* FROM message_parts mp
            INNER JOIN messages m ON m.id = mp.message_id
            WHERE m.chat_id = ?
            ORDER BY m.seq ASC, mp.idx ASC
            "#,
        )
        .bind(chat_id)
        .fetch_all(&self.pool)
        .await?;

        Ok(Some(ChatExportBundle {
            chat,
            messages,
            message_parts,
        }))
    }
}
