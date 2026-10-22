use crate::{
    connection::{new_id, now_ms},
    models::{Extension, ExtensionTool},
};
use anyhow::Result;
use sqlx::SqlitePool;

pub struct ExtensionRepository {
    pool: SqlitePool,
}

impl ExtensionRepository {
    pub fn new(pool: SqlitePool) -> Self {
        Self { pool }
    }

    pub async fn upsert_builtin(
        &self,
        name: &str,
        description: &str,
        version: &str,
    ) -> Result<Extension> {
        let id = new_id();
        let now = now_ms();

        sqlx::query(
            r#"
            INSERT INTO extensions (id, name, description, kind, version, source, bundled, enabled, installed_at)
            VALUES (?, ?, ?, 'builtin', ?, 'bundled', 1, 1, ?)
            ON CONFLICT(name) DO UPDATE SET
                description = excluded.description,
                version = excluded.version
            "#,
        )
        .bind(&id)
        .bind(name)
        .bind(description)
        .bind(version)
        .bind(now)
        .execute(&self.pool)
        .await?;

        let ext = sqlx::query_as::<_, Extension>("SELECT * FROM extensions WHERE name = ?")
            .bind(name)
            .fetch_one(&self.pool)
            .await?;
        Ok(ext)
    }

    pub async fn upsert_tool(
        &self,
        extension_id: &str,
        tool_name: &str,
        description: &str,
        input_schema_json: &str,
        is_read_only: bool,
    ) -> Result<ExtensionTool> {
        let id = new_id();

        sqlx::query(
            r#"
            INSERT INTO extension_tools (id, extension_id, tool_name, description, input_schema, is_read_only, enabled)
            VALUES (?, ?, ?, ?, ?, ?, 1)
            ON CONFLICT(extension_id, tool_name) DO UPDATE SET
                description = excluded.description,
                input_schema = excluded.input_schema,
                is_read_only = excluded.is_read_only
            "#,
        )
        .bind(&id)
        .bind(extension_id)
        .bind(tool_name)
        .bind(description)
        .bind(input_schema_json)
        .bind(if is_read_only { 1 } else { 0 })
        .execute(&self.pool)
        .await?;

        let tool = sqlx::query_as::<_, ExtensionTool>(
            "SELECT * FROM extension_tools WHERE extension_id = ? AND tool_name = ?",
        )
        .bind(extension_id)
        .bind(tool_name)
        .fetch_one(&self.pool)
        .await?;
        Ok(tool)
    }

    pub async fn find_enabled_tool_by_name(
        &self,
        tool_name: &str,
    ) -> Result<Option<ExtensionTool>> {
        let tool = sqlx::query_as::<_, ExtensionTool>(
            "SELECT * FROM extension_tools WHERE tool_name = ? AND enabled = 1 LIMIT 1",
        )
        .bind(tool_name)
        .fetch_optional(&self.pool)
        .await?;
        Ok(tool)
    }
}
