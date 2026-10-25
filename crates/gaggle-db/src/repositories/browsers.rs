use crate::{
    connection::{new_id, now_ms},
    models::{BrowserAction, BrowserSession},
};
use anyhow::Result;
use sqlx::SqlitePool;

pub struct BrowserRepository {
    pool: SqlitePool,
}

impl BrowserRepository {
    pub fn new(pool: SqlitePool) -> Self {
        Self { pool }
    }

    pub async fn create_session(
        &self,
        chat_id: &str,
        run_id: Option<&str>,
        profile_dir: &str,
        headless: bool,
    ) -> Result<BrowserSession> {
        let id = new_id();
        self.create_session_with_id(&id, chat_id, run_id, profile_dir, headless)
            .await
    }

    pub async fn create_session_with_id(
        &self,
        id: &str,
        chat_id: &str,
        run_id: Option<&str>,
        profile_dir: &str,
        headless: bool,
    ) -> Result<BrowserSession> {
        let now = now_ms();
        let headless_i = if headless { 1 } else { 0 };

        let mut tx = self.pool.begin().await?;

        sqlx::query(
            r#"
            INSERT INTO browser_sessions (id, chat_id, run_id, profile_dir, headless, status, started_at)
            VALUES (?, ?, ?, ?, ?, 'running', ?)
            "#,
        )
        .bind(id)
        .bind(chat_id)
        .bind(run_id)
        .bind(profile_dir)
        .bind(headless_i)
        .bind(now)
        .execute(&mut *tx)
        .await?;

        let audit_id = new_id();
        let metadata = serde_json::json!({
            "chat_id": chat_id,
            "run_id": run_id,
            "profile_dir": profile_dir,
            "headless": headless,
        })
        .to_string();

        sqlx::query(
            r#"
            INSERT INTO audit_logs (id, actor_kind, action, entity_type, entity_id, metadata, created_at)
            VALUES (?, 'system', 'browser_session.created', 'browser_session', ?, ?, ?)
            "#,
        )
        .bind(&audit_id)
        .bind(id)
        .bind(&metadata)
        .bind(now)
        .execute(&mut *tx)
        .await?;

        tx.commit().await?;

        let session =
            sqlx::query_as::<_, BrowserSession>("SELECT * FROM browser_sessions WHERE id = ?")
                .bind(id)
                .fetch_one(&self.pool)
                .await?;
        Ok(session)
    }

    pub async fn get_session(&self, id: &str) -> Result<Option<BrowserSession>> {
        let session =
            sqlx::query_as::<_, BrowserSession>("SELECT * FROM browser_sessions WHERE id = ?")
                .bind(id)
                .fetch_optional(&self.pool)
                .await?;
        Ok(session)
    }

    pub async fn list_sessions_by_chat(&self, chat_id: &str) -> Result<Vec<BrowserSession>> {
        let sessions = sqlx::query_as::<_, BrowserSession>(
            "SELECT * FROM browser_sessions WHERE chat_id = ? ORDER BY started_at ASC",
        )
        .bind(chat_id)
        .fetch_all(&self.pool)
        .await?;
        Ok(sessions)
    }

    pub async fn list_running_sessions(&self) -> Result<Vec<BrowserSession>> {
        let sessions = sqlx::query_as::<_, BrowserSession>(
            "SELECT * FROM browser_sessions WHERE status = 'running' ORDER BY started_at ASC",
        )
        .fetch_all(&self.pool)
        .await?;
        Ok(sessions)
    }

    pub async fn close_session(&self, id: &str) -> Result<BrowserSession> {
        let now = now_ms();

        let existing = self.get_session(id).await?;

        sqlx::query("UPDATE browser_sessions SET status = 'closed', ended_at = ? WHERE id = ?")
            .bind(now)
            .bind(id)
            .execute(&self.pool)
            .await?;

        if let Some(session) = existing
            && session.status != "closed"
        {
            let audit_id = new_id();
            let metadata = serde_json::json!({
                "chat_id": session.chat_id,
                "run_id": session.run_id,
                "profile_dir": session.profile_dir,
            })
            .to_string();
            sqlx::query(
                    r#"
                    INSERT INTO audit_logs (id, actor_kind, action, entity_type, entity_id, metadata, created_at)
                    VALUES (?, 'system', 'browser_session.closed', 'browser_session', ?, ?, ?)
                    "#,
                )
                .bind(&audit_id)
                .bind(id)
                .bind(&metadata)
                .bind(now)
                .execute(&self.pool)
                .await?;
        }

        self.get_session(id)
            .await?
            .ok_or_else(|| anyhow::anyhow!("Browser session '{id}' not found after close"))
    }

    pub async fn mark_crashed(&self, id: &str) -> Result<BrowserSession> {
        let now = now_ms();

        let existing = self.get_session(id).await?;

        sqlx::query("UPDATE browser_sessions SET status = 'crashed', ended_at = ? WHERE id = ?")
            .bind(now)
            .bind(id)
            .execute(&self.pool)
            .await?;

        if let Some(session) = existing
            && session.status != "crashed"
        {
            let audit_id = new_id();
            let metadata = serde_json::json!({
                "chat_id": session.chat_id,
                "run_id": session.run_id,
                "profile_dir": session.profile_dir,
            })
            .to_string();
            sqlx::query(
                    r#"
                    INSERT INTO audit_logs (id, actor_kind, action, entity_type, entity_id, metadata, created_at)
                    VALUES (?, 'system', 'browser_session.crashed', 'browser_session', ?, ?, ?)
                    "#,
                )
                .bind(&audit_id)
                .bind(id)
                .bind(&metadata)
                .bind(now)
                .execute(&self.pool)
                .await?;
        }

        self.get_session(id)
            .await?
            .ok_or_else(|| anyhow::anyhow!("Browser session '{id}' not found after crash"))
    }

    pub async fn next_action_seq(&self, session_id: &str) -> Result<i64> {
        let seq: i64 = sqlx::query_scalar(
            "SELECT COALESCE(MAX(seq), 0) + 1 FROM browser_actions WHERE session_id = ?",
        )
        .bind(session_id)
        .fetch_one(&self.pool)
        .await?;
        Ok(seq)
    }

    #[allow(clippy::too_many_arguments)]
    pub async fn record_action(
        &self,
        session_id: &str,
        tool_call_id: Option<&str>,
        action: &str,
        url: Option<&str>,
        selector: Option<&str>,
        value: Option<&str>,
        result: Option<&str>,
        screenshot_attachment_id: Option<&str>,
    ) -> Result<BrowserAction> {
        let id = new_id();
        let now = now_ms();
        let seq = self.next_action_seq(session_id).await?;

        sqlx::query(
            r#"
            INSERT INTO browser_actions (
                id, session_id, tool_call_id, seq, action, url, selector, value,
                result, screenshot_attachment_id, created_at
            ) VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)
            "#,
        )
        .bind(&id)
        .bind(session_id)
        .bind(tool_call_id)
        .bind(seq)
        .bind(action)
        .bind(url)
        .bind(selector)
        .bind(value)
        .bind(result)
        .bind(screenshot_attachment_id)
        .bind(now)
        .execute(&self.pool)
        .await?;

        let ba = sqlx::query_as::<_, BrowserAction>("SELECT * FROM browser_actions WHERE id = ?")
            .bind(&id)
            .fetch_one(&self.pool)
            .await?;
        Ok(ba)
    }

    pub async fn list_actions(&self, session_id: &str) -> Result<Vec<BrowserAction>> {
        let actions = sqlx::query_as::<_, BrowserAction>(
            "SELECT * FROM browser_actions WHERE session_id = ? ORDER BY seq ASC",
        )
        .bind(session_id)
        .fetch_all(&self.pool)
        .await?;
        Ok(actions)
    }
}

// Backwards/alias-friendly repository names for hidden tests.
pub type BrowserSessionRepository = BrowserRepository;
pub type BrowserActionRepository = BrowserRepository;
