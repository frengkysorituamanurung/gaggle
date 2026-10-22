use anyhow::Result;
use gaggle_db::models::{Extension, ExtensionTool};
use sqlx::SqlitePool;

/// Goose recommends keeping active tools below ~25 so LLM tool selection stays accurate.
pub const RECOMMENDED_MAX_ACTIVE_TOOLS: usize = 25;

pub struct ExtensionRegistry {
    pool: SqlitePool,
}

impl ExtensionRegistry {
    pub fn new(pool: SqlitePool) -> Self {
        Self { pool }
    }

    pub async fn list_enabled_extensions(&self) -> Result<Vec<Extension>> {
        let exts = sqlx::query_as::<_, Extension>(
            "SELECT * FROM extensions WHERE enabled = 1 ORDER BY name ASC",
        )
        .fetch_all(&self.pool)
        .await?;
        Ok(exts)
    }

    pub async fn list_active_tools_for_chat(&self, chat_id: &str) -> Result<Vec<ExtensionTool>> {
        let tools = sqlx::query_as::<_, ExtensionTool>(
            r#"
            SELECT et.* FROM extension_tools et
            INNER JOIN chat_extensions ce ON ce.extension_id = et.extension_id
            WHERE ce.chat_id = ? AND ce.enabled = 1 AND et.enabled = 1
            ORDER BY et.tool_name ASC
            "#,
        )
        .bind(chat_id)
        .fetch_all(&self.pool)
        .await?;
        Ok(tools)
    }

    pub fn exceeds_recommended_tool_limit(active_tool_count: usize) -> bool {
        active_tool_count > RECOMMENDED_MAX_ACTIVE_TOOLS
    }
}
