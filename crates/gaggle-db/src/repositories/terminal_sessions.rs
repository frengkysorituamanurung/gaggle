use crate::{
    connection::{new_id, now_ms},
    models::TerminalSession,
};
use anyhow::{Context, Result};
use sqlx::SqlitePool;

pub struct TerminalSessionRepository {
    pool: SqlitePool,
}

impl TerminalSessionRepository {
    pub fn new(pool: SqlitePool) -> Self {
        Self { pool }
    }

    #[allow(clippy::too_many_arguments)]
    pub async fn create(
        &self,
        project_id: &str,
        chat_id: Option<&str>,
        worktree_id: Option<&str>,
        cwd: &str,
        shell: &str,
        cols: i64,
        rows: i64,
        pid: Option<i64>,
    ) -> Result<TerminalSession> {
        let id = new_id();
        let now = now_ms();
        sqlx::query(
            r#"
            INSERT INTO terminal_sessions (
                id, project_id, chat_id, worktree_id, cwd, shell, cols, rows, pid, status, started_at
            ) VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, 'running', ?)
            "#,
        )
        .bind(&id)
        .bind(project_id)
        .bind(chat_id)
        .bind(worktree_id)
        .bind(cwd)
        .bind(shell)
        .bind(cols)
        .bind(rows)
        .bind(pid)
        .bind(now)
        .execute(&self.pool)
        .await?;

        self.get_by_id(&id)
            .await?
            .context("Terminal session not found after creation")
    }

    pub async fn get_by_id(&self, id: &str) -> Result<Option<TerminalSession>> {
        let session =
            sqlx::query_as::<_, TerminalSession>("SELECT * FROM terminal_sessions WHERE id = ?")
                .bind(id)
                .fetch_optional(&self.pool)
                .await?;
        Ok(session)
    }

    pub async fn list_by_project(&self, project_id: &str) -> Result<Vec<TerminalSession>> {
        let list = sqlx::query_as::<_, TerminalSession>(
            "SELECT * FROM terminal_sessions WHERE project_id = ? ORDER BY started_at DESC",
        )
        .bind(project_id)
        .fetch_all(&self.pool)
        .await?;
        Ok(list)
    }

    pub async fn list_by_chat(&self, chat_id: &str) -> Result<Vec<TerminalSession>> {
        let list = sqlx::query_as::<_, TerminalSession>(
            "SELECT * FROM terminal_sessions WHERE chat_id = ? ORDER BY started_at DESC",
        )
        .bind(chat_id)
        .fetch_all(&self.pool)
        .await?;
        Ok(list)
    }

    pub async fn update_dimensions(&self, id: &str, cols: i64, rows: i64) -> Result<()> {
        sqlx::query("UPDATE terminal_sessions SET cols = ?, rows = ? WHERE id = ?")
            .bind(cols)
            .bind(rows)
            .bind(id)
            .execute(&self.pool)
            .await?;
        Ok(())
    }

    pub async fn mark_exited(&self, id: &str, exit_code: Option<i64>, ended_at: i64) -> Result<()> {
        sqlx::query(
            "UPDATE terminal_sessions SET status = 'exited', exit_code = ?, ended_at = ? WHERE id = ?",
        )
        .bind(exit_code)
        .bind(ended_at)
        .bind(id)
        .execute(&self.pool)
        .await?;
        Ok(())
    }

    pub async fn list_running(&self) -> Result<Vec<TerminalSession>> {
        let list = sqlx::query_as::<_, TerminalSession>(
            "SELECT * FROM terminal_sessions WHERE status = 'running'",
        )
        .fetch_all(&self.pool)
        .await?;
        Ok(list)
    }
}
