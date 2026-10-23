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
        // A tool is executable only when both the extension and the tool are enabled.
        let tool = sqlx::query_as::<_, ExtensionTool>(
            r#"
            SELECT et.* FROM extension_tools et
            INNER JOIN extensions e ON e.id = et.extension_id
            WHERE et.tool_name = ? AND et.enabled = 1 AND e.enabled = 1
            LIMIT 1
            "#,
        )
        .bind(tool_name)
        .fetch_optional(&self.pool)
        .await?;
        Ok(tool)
    }

    pub async fn list_enabled_extensions(&self) -> Result<Vec<Extension>> {
        let exts = sqlx::query_as::<_, Extension>(
            "SELECT * FROM extensions WHERE enabled = 1 ORDER BY name ASC",
        )
        .fetch_all(&self.pool)
        .await?;
        Ok(exts)
    }

    pub async fn list_all_extensions(&self) -> Result<Vec<Extension>> {
        let exts = sqlx::query_as::<_, Extension>("SELECT * FROM extensions ORDER BY name ASC")
            .fetch_all(&self.pool)
            .await?;
        Ok(exts)
    }

    pub async fn get_extension_by_id(&self, id: &str) -> Result<Option<Extension>> {
        let ext = sqlx::query_as::<_, Extension>("SELECT * FROM extensions WHERE id = ?")
            .bind(id)
            .fetch_optional(&self.pool)
            .await?;
        Ok(ext)
    }

    pub async fn list_tools_for_extension(&self, extension_id: &str) -> Result<Vec<ExtensionTool>> {
        let tools = sqlx::query_as::<_, ExtensionTool>(
            "SELECT * FROM extension_tools WHERE extension_id = ? ORDER BY tool_name ASC",
        )
        .bind(extension_id)
        .fetch_all(&self.pool)
        .await?;
        Ok(tools)
    }

    /// Counts every enabled tool that belongs to an enabled extension. This is the
    /// workspace-wide active tool budget surfaced by the Settings UI.
    pub async fn count_active_tools(&self) -> Result<i64> {
        let count: i64 = sqlx::query_scalar(
            r#"
            SELECT COUNT(*) FROM extension_tools et
            INNER JOIN extensions e ON e.id = et.extension_id
            WHERE e.enabled = 1 AND et.enabled = 1
            "#,
        )
        .fetch_one(&self.pool)
        .await?;
        Ok(count)
    }

    /// Lists every enabled tool that belongs to an enabled extension, ordered by tool name.
    ///
    /// The result is intentionally uncapped: callers (context assembly, tool registries)
    /// apply their own active-tool budget.
    pub async fn list_enabled_tools(&self) -> Result<Vec<ExtensionTool>> {
        let tools = sqlx::query_as::<_, ExtensionTool>(
            r#"
            SELECT et.* FROM extension_tools et
            INNER JOIN extensions e ON e.id = et.extension_id
            WHERE e.enabled = 1 AND et.enabled = 1
            ORDER BY et.tool_name ASC, et.id ASC
            "#,
        )
        .fetch_all(&self.pool)
        .await?;
        Ok(tools)
    }

    pub async fn set_enabled(&self, id: &str, enabled: bool) -> Result<Extension> {
        sqlx::query("UPDATE extensions SET enabled = ? WHERE id = ?")
            .bind(if enabled { 1 } else { 0 })
            .bind(id)
            .execute(&self.pool)
            .await?;

        self.get_extension_by_id(id)
            .await?
            .ok_or_else(|| anyhow::anyhow!("Extension {id} not found after update"))
    }
}
