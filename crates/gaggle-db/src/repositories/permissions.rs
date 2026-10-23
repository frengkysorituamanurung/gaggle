use crate::{
    connection::{new_id, now_ms},
    models::{CommandRule, ToolPermission, ToolRiskCache},
};
use anyhow::Result;
use sqlx::SqlitePool;

pub struct PermissionRepository {
    pool: SqlitePool,
}

impl PermissionRepository {
    pub fn new(pool: SqlitePool) -> Self {
        Self { pool }
    }

    pub async fn add_command_rule(
        &self,
        project_id: Option<&str>,
        pattern: &str,
        match_kind: &str,
        action: &str,
    ) -> Result<CommandRule> {
        let id = new_id();
        let now = now_ms();

        sqlx::query(
            r#"
            INSERT INTO command_rules (id, project_id, pattern, match_kind, action, created_at)
            VALUES (?, ?, ?, ?, ?, ?)
            "#,
        )
        .bind(&id)
        .bind(project_id)
        .bind(pattern)
        .bind(match_kind)
        .bind(action)
        .bind(now)
        .execute(&self.pool)
        .await?;

        let rule = sqlx::query_as::<_, CommandRule>("SELECT * FROM command_rules WHERE id = ?")
            .bind(&id)
            .fetch_one(&self.pool)
            .await?;
        Ok(rule)
    }

    pub async fn list_command_rules(&self, project_id: Option<&str>) -> Result<Vec<CommandRule>> {
        let rules = sqlx::query_as::<_, CommandRule>(
            r#"
            SELECT * FROM command_rules
            WHERE project_id IS NULL OR project_id = ?
            ORDER BY created_at DESC
            "#,
        )
        .bind(project_id)
        .fetch_all(&self.pool)
        .await?;
        Ok(rules)
    }

    pub async fn set_tool_permission(
        &self,
        project_id: Option<&str>,
        extension_tool_id: &str,
        level: &str,
    ) -> Result<ToolPermission> {
        let id = new_id();
        let now = now_ms();

        sqlx::query(
            r#"
            INSERT INTO tool_permissions (id, project_id, extension_tool_id, level, updated_at)
            VALUES (?, ?, ?, ?, ?)
            "#,
        )
        .bind(&id)
        .bind(project_id)
        .bind(extension_tool_id)
        .bind(level)
        .bind(now)
        .execute(&self.pool)
        .await?;

        let perm =
            sqlx::query_as::<_, ToolPermission>("SELECT * FROM tool_permissions WHERE id = ?")
                .bind(&id)
                .fetch_one(&self.pool)
                .await?;
        Ok(perm)
    }

    pub async fn get_tool_permission(
        &self,
        project_id: Option<&str>,
        extension_tool_id: &str,
    ) -> Result<Option<ToolPermission>> {
        if let Some(pid) = project_id
            && let Some(project_perm) = sqlx::query_as::<_, ToolPermission>(
                "SELECT * FROM tool_permissions WHERE project_id = ? AND extension_tool_id = ?",
            )
            .bind(pid)
            .bind(extension_tool_id)
            .fetch_optional(&self.pool)
            .await?
        {
            return Ok(Some(project_perm));
        }

        let global_perm = sqlx::query_as::<_, ToolPermission>(
            "SELECT * FROM tool_permissions WHERE project_id IS NULL AND extension_tool_id = ?",
        )
        .bind(extension_tool_id)
        .fetch_optional(&self.pool)
        .await?;
        Ok(global_perm)
    }

    /// Lists the effective permission rows that apply to a project: project-scoped
    /// rows plus global defaults.
    pub async fn list_tool_permissions(
        &self,
        project_id: Option<&str>,
    ) -> Result<Vec<ToolPermission>> {
        let permissions = sqlx::query_as::<_, ToolPermission>(
            r#"
            SELECT * FROM tool_permissions
            WHERE project_id IS NULL OR project_id = ?
            ORDER BY extension_tool_id ASC
            "#,
        )
        .bind(project_id)
        .fetch_all(&self.pool)
        .await?;
        Ok(permissions)
    }

    /// Creates or replaces the permission level for one tool, scoped to the project
    /// when a project id is supplied.
    pub async fn upsert_tool_permission(
        &self,
        project_id: Option<&str>,
        extension_tool_id: &str,
        level: &str,
    ) -> Result<ToolPermission> {
        let now = now_ms();

        let existing_id: Option<String> = if let Some(pid) = project_id {
            sqlx::query_scalar(
                "SELECT id FROM tool_permissions WHERE project_id = ? AND extension_tool_id = ?",
            )
            .bind(pid)
            .bind(extension_tool_id)
            .fetch_optional(&self.pool)
            .await?
        } else {
            sqlx::query_scalar(
                "SELECT id FROM tool_permissions WHERE project_id IS NULL AND extension_tool_id = ?",
            )
            .bind(extension_tool_id)
            .fetch_optional(&self.pool)
            .await?
        };

        let id = if let Some(existing_id) = existing_id {
            sqlx::query("UPDATE tool_permissions SET level = ?, updated_at = ? WHERE id = ?")
                .bind(level)
                .bind(now)
                .bind(&existing_id)
                .execute(&self.pool)
                .await?;
            existing_id
        } else {
            let id = new_id();
            sqlx::query(
                r#"
                INSERT INTO tool_permissions (id, project_id, extension_tool_id, level, updated_at)
                VALUES (?, ?, ?, ?, ?)
                "#,
            )
            .bind(&id)
            .bind(project_id)
            .bind(extension_tool_id)
            .bind(level)
            .bind(now)
            .execute(&self.pool)
            .await?;
            id
        };

        let permission =
            sqlx::query_as::<_, ToolPermission>("SELECT * FROM tool_permissions WHERE id = ?")
                .bind(&id)
                .fetch_one(&self.pool)
                .await?;
        Ok(permission)
    }

    pub async fn get_risk_cache(
        &self,
        tool_name: &str,
        args_fingerprint: &str,
    ) -> Result<Option<ToolRiskCache>> {
        let cached = sqlx::query_as::<_, ToolRiskCache>(
            "SELECT * FROM tool_risk_cache WHERE tool_name = ? AND args_fingerprint = ?",
        )
        .bind(tool_name)
        .bind(args_fingerprint)
        .fetch_optional(&self.pool)
        .await?;
        Ok(cached)
    }

    pub async fn upsert_risk_cache(
        &self,
        tool_name: &str,
        args_fingerprint: &str,
        is_read_only: bool,
        source: &str,
        model_id: Option<&str>,
    ) -> Result<ToolRiskCache> {
        let id = new_id();
        let now = now_ms();

        sqlx::query(
            r#"
            INSERT INTO tool_risk_cache (id, tool_name, args_fingerprint, is_read_only, source, model_id, created_at)
            VALUES (?, ?, ?, ?, ?, ?, ?)
            ON CONFLICT(tool_name, args_fingerprint) DO UPDATE SET
                is_read_only = excluded.is_read_only,
                source = excluded.source,
                model_id = excluded.model_id
            "#,
        )
        .bind(&id)
        .bind(tool_name)
        .bind(args_fingerprint)
        .bind(if is_read_only { 1 } else { 0 })
        .bind(source)
        .bind(model_id)
        .bind(now)
        .execute(&self.pool)
        .await?;

        self.get_risk_cache(tool_name, args_fingerprint)
            .await?
            .ok_or_else(|| anyhow::anyhow!("Risk cache missing after upsert"))
    }
}
