use crate::{
    connection::{new_id, now_ms},
    models::Attachment,
};
use anyhow::Result;
use sqlx::SqlitePool;

pub struct AttachmentRepository {
    pool: SqlitePool,
}

impl AttachmentRepository {
    pub fn new(pool: SqlitePool) -> Self {
        Self { pool }
    }

    pub async fn create(
        &self,
        chat_id: &str,
        file_name: &str,
        mime: &str,
        size_bytes: i64,
        sha256: &str,
        storage_path: &str,
    ) -> Result<Attachment> {
        let id = new_id();
        let now = now_ms();

        sqlx::query(
            r#"
            INSERT INTO attachments (id, chat_id, file_name, mime, size_bytes, sha256, storage_path, created_at)
            VALUES (?, ?, ?, ?, ?, ?, ?, ?)
            "#,
        )
        .bind(&id)
        .bind(chat_id)
        .bind(file_name)
        .bind(mime)
        .bind(size_bytes)
        .bind(sha256)
        .bind(storage_path)
        .bind(now)
        .execute(&self.pool)
        .await?;

        let attachment = sqlx::query_as::<_, Attachment>("SELECT * FROM attachments WHERE id = ?")
            .bind(&id)
            .fetch_one(&self.pool)
            .await?;
        Ok(attachment)
    }

    pub async fn get_by_id(&self, id: &str) -> Result<Option<Attachment>> {
        let attachment = sqlx::query_as::<_, Attachment>("SELECT * FROM attachments WHERE id = ?")
            .bind(id)
            .fetch_optional(&self.pool)
            .await?;
        Ok(attachment)
    }

    pub async fn list_by_chat(&self, chat_id: &str) -> Result<Vec<Attachment>> {
        let attachments = sqlx::query_as::<_, Attachment>(
            "SELECT * FROM attachments WHERE chat_id = ? ORDER BY created_at ASC",
        )
        .bind(chat_id)
        .fetch_all(&self.pool)
        .await?;
        Ok(attachments)
    }
}
