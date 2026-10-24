use crate::{
    connection::{new_id, now_ms},
    models::{Extension, ExtensionPermission, ExtensionTool},
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

    // ---------------------------------------------------------------------
    // Extension lifecycle (MCP / issue #12)
    // ---------------------------------------------------------------------

    #[allow(clippy::too_many_arguments)]
    pub async fn create_extension(
        &self,
        name: &str,
        description: Option<&str>,
        kind: &str,
        version: &str,
        source: &str,
        command: Option<&str>,
        args: Option<&str>,
        url: Option<&str>,
        env_secret_refs: Option<&str>,
        timeout_secs: i64,
        manifest: Option<&str>,
        bundled: bool,
        enabled: bool,
    ) -> Result<Extension> {
        let id = new_id();
        let now = now_ms();

        sqlx::query(
            r#"
            INSERT INTO extensions (
                id, name, description, kind, version, source, command, args, url,
                env_secret_refs, timeout_secs, manifest, bundled, enabled, installed_at
            ) VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)
            "#,
        )
        .bind(&id)
        .bind(name)
        .bind(description)
        .bind(kind)
        .bind(version)
        .bind(source)
        .bind(command)
        .bind(args)
        .bind(url)
        .bind(env_secret_refs)
        .bind(timeout_secs)
        .bind(manifest)
        .bind(if bundled { 1 } else { 0 })
        .bind(if enabled { 1 } else { 0 })
        .bind(now)
        .execute(&self.pool)
        .await?;

        self.get_extension_by_id(&id)
            .await?
            .ok_or_else(|| anyhow::anyhow!("Extension {id} not found after insert"))
    }

    pub async fn delete_extension(&self, id: &str) -> Result<()> {
        sqlx::query("DELETE FROM extensions WHERE id = ?")
            .bind(id)
            .execute(&self.pool)
            .await?;
        Ok(())
    }

    // ---------------------------------------------------------------------
    // Capability grants (extension_permissions)
    // ---------------------------------------------------------------------

    pub async fn list_permissions(&self, extension_id: &str) -> Result<Vec<ExtensionPermission>> {
        let perms = sqlx::query_as::<_, ExtensionPermission>(
            "SELECT * FROM extension_permissions WHERE extension_id = ? ORDER BY permission ASC",
        )
        .bind(extension_id)
        .fetch_all(&self.pool)
        .await?;
        Ok(perms)
    }

    pub async fn grant_permission(
        &self,
        extension_id: &str,
        permission: &str,
        scope: Option<&str>,
    ) -> Result<ExtensionPermission> {
        let id = new_id();
        let now = now_ms();

        // Upsert by (extension_id, permission, scope) best-effort.
        let existing: Option<String> = sqlx::query_scalar(
            "SELECT id FROM extension_permissions WHERE extension_id = ? AND permission = ? AND (scope IS ? OR scope = ?)",
        )
        .bind(extension_id)
        .bind(permission)
        .bind(scope)
        .bind(scope.unwrap_or(""))
        .fetch_optional(&self.pool)
        .await
        .unwrap_or(None);

        let row_id = if let Some(existing_id) = existing {
            sqlx::query(
                "UPDATE extension_permissions SET granted = 1, granted_at = ?, scope = ? WHERE id = ?",
            )
            .bind(now)
            .bind(scope)
            .bind(&existing_id)
            .execute(&self.pool)
            .await?;
            existing_id
        } else {
            sqlx::query(
                r#"
                INSERT INTO extension_permissions (id, extension_id, permission, scope, granted, granted_at)
                VALUES (?, ?, ?, ?, 1, ?)
                "#,
            )
            .bind(&id)
            .bind(extension_id)
            .bind(permission)
            .bind(scope)
            .bind(now)
            .execute(&self.pool)
            .await?;
            id
        };

        let perm = sqlx::query_as::<_, ExtensionPermission>(
            "SELECT * FROM extension_permissions WHERE id = ?",
        )
        .bind(&row_id)
        .fetch_one(&self.pool)
        .await?;
        Ok(perm)
    }

    pub async fn revoke_permission(&self, extension_id: &str, permission: &str) -> Result<()> {
        sqlx::query(
            "UPDATE extension_permissions SET granted = 0, granted_at = NULL WHERE extension_id = ? AND permission = ?",
        )
        .bind(extension_id)
        .bind(permission)
        .execute(&self.pool)
        .await?;
        Ok(())
    }

    pub async fn has_granted_permission(
        &self,
        extension_id: &str,
        permission: &str,
    ) -> Result<bool> {
        let count: i64 = sqlx::query_scalar(
            "SELECT COUNT(*) FROM extension_permissions WHERE extension_id = ? AND permission = ? AND granted = 1",
        )
        .bind(extension_id)
        .bind(permission)
        .fetch_one(&self.pool)
        .await?;
        Ok(count > 0)
    }

    pub async fn any_granted_permission(&self, extension_id: &str) -> Result<bool> {
        let count: i64 = sqlx::query_scalar(
            "SELECT COUNT(*) FROM extension_permissions WHERE extension_id = ? AND granted = 1",
        )
        .bind(extension_id)
        .fetch_one(&self.pool)
        .await?;
        Ok(count > 0)
    }

    // ---------------------------------------------------------------------
    // chat_extensions
    // ---------------------------------------------------------------------

    pub async fn enable_extension_for_chat(&self, chat_id: &str, extension_id: &str) -> Result<()> {
        let now = now_ms();
        sqlx::query(
            r#"
            INSERT INTO chat_extensions (chat_id, extension_id, enabled, enabled_at)
            VALUES (?, ?, 1, ?)
            ON CONFLICT(chat_id, extension_id) DO UPDATE SET enabled = 1
            "#,
        )
        .bind(chat_id)
        .bind(extension_id)
        .bind(now)
        .execute(&self.pool)
        .await?;
        Ok(())
    }

    pub async fn disable_extension_for_chat(
        &self,
        chat_id: &str,
        extension_id: &str,
    ) -> Result<()> {
        sqlx::query(
            "UPDATE chat_extensions SET enabled = 0 WHERE chat_id = ? AND extension_id = ?",
        )
        .bind(chat_id)
        .bind(extension_id)
        .execute(&self.pool)
        .await?;
        Ok(())
    }

    pub async fn list_chat_extensions(&self, chat_id: &str) -> Result<Vec<String>> {
        let ids: Vec<String> = sqlx::query_scalar(
            "SELECT extension_id FROM chat_extensions WHERE chat_id = ? AND enabled = 1",
        )
        .bind(chat_id)
        .fetch_all(&self.pool)
        .await?;
        Ok(ids)
    }

    // ---------------------------------------------------------------------
    // audit logs
    // ---------------------------------------------------------------------

    pub async fn record_audit_log(
        &self,
        actor_kind: &str,
        actor_id: Option<&str>,
        action: &str,
        entity_type: &str,
        entity_id: Option<&str>,
        metadata: Option<&str>,
    ) -> Result<()> {
        let id = new_id();
        let now = now_ms();
        sqlx::query(
            r#"
            INSERT INTO audit_logs (id, actor_kind, actor_id, action, entity_type, entity_id, metadata, created_at)
            VALUES (?, ?, ?, ?, ?, ?, ?, ?)
            "#,
        )
        .bind(&id)
        .bind(actor_kind)
        .bind(actor_id)
        .bind(action)
        .bind(entity_type)
        .bind(entity_id)
        .bind(metadata)
        .bind(now)
        .execute(&self.pool)
        .await?;
        Ok(())
    }
}
