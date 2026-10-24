use crate::{
    connection::{new_id, now_ms},
    models::{Chat, Message, MessagePart},
};
use anyhow::Result;
use serde::{Deserialize, Serialize};
use sqlx::SqlitePool;
use std::collections::HashMap;

/// Hard upper bound for [`ChatRepository::list_messages_with_parts`] to keep context
/// assembly reads bounded even if a caller passes an unbounded limit.
const MAX_HISTORY_MESSAGES_PER_QUERY: i64 = 500;

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

    /// Creates a child chat (used by subagents/sub-recipes) with parent linkage.
    #[allow(clippy::too_many_arguments)]
    pub async fn create_child_chat(
        &self,
        project_id: &str,
        title: &str,
        chat_type: &str,
        mode: &str,
        permission_mode: &str,
        parent_chat_id: Option<&str>,
        parent_tool_call_id: Option<&str>,
    ) -> Result<Chat> {
        let id = new_id();
        let now = now_ms();

        sqlx::query(
            r#"
            INSERT INTO chats (
                id, project_id, chat_type, title, mode, permission_mode,
                parent_chat_id, parent_tool_call_id, created_at, updated_at
            ) VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?)
            "#,
        )
        .bind(&id)
        .bind(project_id)
        .bind(chat_type)
        .bind(title)
        .bind(mode)
        .bind(permission_mode)
        .bind(parent_chat_id)
        .bind(parent_tool_call_id)
        .bind(now)
        .bind(now)
        .execute(&self.pool)
        .await?;

        self.get_by_id(&id)
            .await?
            .ok_or_else(|| anyhow::anyhow!("Chat not found after insert"))
    }

    /// Convenience for creating a `chat_type = 'subagent'` child chat.
    pub async fn create_subagent_chat(
        &self,
        project_id: &str,
        title: &str,
        mode: &str,
        permission_mode: &str,
        parent_chat_id: &str,
        parent_tool_call_id: Option<&str>,
    ) -> Result<Chat> {
        self.create_child_chat(
            project_id,
            title,
            "subagent",
            mode,
            permission_mode,
            Some(parent_chat_id),
            parent_tool_call_id,
        )
        .await
    }

    /// Creates a chat carrying recipe/schedule context.
    #[allow(clippy::too_many_arguments)]
    #[allow(clippy::too_many_arguments)]
    pub async fn create_recipe_chat(
        &self,
        project_id: &str,
        title: &str,
        chat_type: &str,
        mode: &str,
        permission_mode: &str,
        recipe_id: Option<&str>,
        recipe_values: Option<&str>,
        schedule_id: Option<&str>,
        parent_chat_id: Option<&str>,
        parent_tool_call_id: Option<&str>,
    ) -> Result<Chat> {
        let id = new_id();
        let now = now_ms();

        sqlx::query(
            r#"
            INSERT INTO chats (
                id, project_id, chat_type, title, mode, permission_mode,
                recipe_id, recipe_values, schedule_id,
                parent_chat_id, parent_tool_call_id, created_at, updated_at
            ) VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)
            "#,
        )
        .bind(&id)
        .bind(project_id)
        .bind(chat_type)
        .bind(title)
        .bind(mode)
        .bind(permission_mode)
        .bind(recipe_id)
        .bind(recipe_values)
        .bind(schedule_id)
        .bind(parent_chat_id)
        .bind(parent_tool_call_id)
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

    pub async fn update_title(&self, chat_id: &str, title: &str) -> Result<Chat> {
        let now = now_ms();
        sqlx::query("UPDATE chats SET title = ?, title_user_set = 1, updated_at = ? WHERE id = ?")
            .bind(title)
            .bind(now)
            .bind(chat_id)
            .execute(&self.pool)
            .await?;

        self.get_by_id(chat_id)
            .await?
            .ok_or_else(|| anyhow::anyhow!("Chat not found after title update"))
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

    /// Loads the most recent `limit` messages for a chat (ascending `seq` order) together
    /// with their ordered parts.
    ///
    /// Two queries are used (messages, then parts for the fetched message ids) to avoid an
    /// N+1 pattern. Messages without any parts are preserved with an empty `Vec`.
    pub async fn list_messages_with_parts(
        &self,
        chat_id: &str,
        limit: i64,
    ) -> Result<Vec<(Message, Vec<MessagePart>)>> {
        let capped_limit = limit.clamp(1, MAX_HISTORY_MESSAGES_PER_QUERY);

        let mut messages = sqlx::query_as::<_, Message>(
            "SELECT * FROM messages WHERE chat_id = ? ORDER BY seq DESC LIMIT ?",
        )
        .bind(chat_id)
        .bind(capped_limit)
        .fetch_all(&self.pool)
        .await?;

        // Query newest-first so the LIMIT keeps the most recent rows, then restore
        // chronological order for callers and model context.
        messages.reverse();

        if messages.is_empty() {
            return Ok(Vec::new());
        }

        let placeholders = std::iter::repeat_n("?", messages.len())
            .collect::<Vec<_>>()
            .join(", ");
        let parts_sql = format!(
            "SELECT * FROM message_parts WHERE message_id IN ({placeholders}) ORDER BY message_id ASC, idx ASC"
        );

        let mut query = sqlx::query_as::<_, MessagePart>(sqlx::AssertSqlSafe(parts_sql));
        for message in &messages {
            query = query.bind(&message.id);
        }
        let parts = query.fetch_all(&self.pool).await?;

        let mut parts_by_message: HashMap<String, Vec<MessagePart>> = HashMap::new();
        for part in parts {
            parts_by_message
                .entry(part.message_id.clone())
                .or_default()
                .push(part);
        }

        Ok(messages
            .into_iter()
            .map(|message| {
                let message_parts = parts_by_message.remove(&message.id).unwrap_or_default();
                (message, message_parts)
            })
            .collect())
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
