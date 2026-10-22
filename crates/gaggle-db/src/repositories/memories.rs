use crate::{
    connection::{new_id, now_ms},
    models::Memory,
};
use anyhow::Result;
use sqlx::SqlitePool;

pub struct MemoryRepository {
    pool: SqlitePool,
}

impl MemoryRepository {
    pub fn new(pool: SqlitePool) -> Self {
        Self { pool }
    }

    pub async fn create(
        &self,
        project_id: Option<&str>,
        kind: &str,
        title: &str,
        content: &str,
    ) -> Result<Memory> {
        let id = new_id();
        let now = now_ms();

        sqlx::query(
            r#"
            INSERT INTO memories (id, project_id, kind, title, content, pinned, confidence, created_at, updated_at)
            VALUES (?, ?, ?, ?, ?, 0, 1.0, ?, ?)
            "#,
        )
        .bind(&id)
        .bind(project_id)
        .bind(kind)
        .bind(title)
        .bind(content)
        .bind(now)
        .bind(now)
        .execute(&self.pool)
        .await?;

        let mem = sqlx::query_as::<_, Memory>("SELECT * FROM memories WHERE id = ?")
            .bind(&id)
            .fetch_one(&self.pool)
            .await?;
        Ok(mem)
    }

    /// Full-text search across memories via the `memories_fts` FTS5 virtual table.
    pub async fn search_fts(&self, query: &str, limit: i64) -> Result<Vec<Memory>> {
        let rows = sqlx::query_as::<_, Memory>(
            r#"
            SELECT m.* FROM memories m
            INNER JOIN memories_fts fts ON m.rowid = fts.rowid
            WHERE memories_fts MATCH ? AND m.deleted_at IS NULL
            ORDER BY rank
            LIMIT ?
            "#,
        )
        .bind(query)
        .bind(limit)
        .fetch_all(&self.pool)
        .await?;
        Ok(rows)
    }
}
