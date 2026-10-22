use crate::{
    connection::{new_id, now_ms},
    models::{Approval, Run, RunCheckpoint, RunEvent, ToolCall},
};
use anyhow::Result;
use sqlx::SqlitePool;

pub struct RunRepository {
    pool: SqlitePool,
}

impl RunRepository {
    pub fn new(pool: SqlitePool) -> Self {
        Self { pool }
    }

    pub async fn create(
        &self,
        chat_id: &str,
        trigger_message_id: Option<&str>,
        model_id: Option<&str>,
    ) -> Result<Run> {
        let id = new_id();
        let now = now_ms();

        sqlx::query(
            r#"
            INSERT INTO runs (id, chat_id, trigger_message_id, model_id, status, started_at, last_heartbeat_at)
            VALUES (?, ?, ?, ?, 'running', ?, ?)
            "#,
        )
        .bind(&id)
        .bind(chat_id)
        .bind(trigger_message_id)
        .bind(model_id)
        .bind(now)
        .bind(now)
        .execute(&self.pool)
        .await?;

        let run = sqlx::query_as::<_, Run>("SELECT * FROM runs WHERE id = ?")
            .bind(&id)
            .fetch_one(&self.pool)
            .await?;
        Ok(run)
    }

    pub async fn emit_event(
        &self,
        run_id: &str,
        kind: &str,
        payload_json: &str,
    ) -> Result<RunEvent> {
        let id = new_id();
        let now = now_ms();

        let mut tx = self.pool.begin().await?;

        let next_seq: i64 =
            sqlx::query_scalar("SELECT COALESCE(MAX(seq), 0) + 1 FROM run_events WHERE run_id = ?")
                .bind(run_id)
                .fetch_one(&mut *tx)
                .await?;

        sqlx::query(
            r#"
            INSERT INTO run_events (id, run_id, seq, kind, payload, created_at)
            VALUES (?, ?, ?, ?, ?, ?)
            "#,
        )
        .bind(&id)
        .bind(run_id)
        .bind(next_seq)
        .bind(kind)
        .bind(payload_json)
        .bind(now)
        .execute(&mut *tx)
        .await?;

        tx.commit().await?;

        let ev = sqlx::query_as::<_, RunEvent>("SELECT * FROM run_events WHERE id = ?")
            .bind(&id)
            .fetch_one(&self.pool)
            .await?;
        Ok(ev)
    }

    pub async fn save_checkpoint(
        &self,
        run_id: &str,
        step_index: i64,
        state_json: &str,
    ) -> Result<RunCheckpoint> {
        let id = new_id();
        let now = now_ms();

        sqlx::query(
            r#"
            INSERT INTO run_checkpoints (id, run_id, step_index, state, created_at)
            VALUES (?, ?, ?, ?, ?)
            "#,
        )
        .bind(&id)
        .bind(run_id)
        .bind(step_index)
        .bind(state_json)
        .bind(now)
        .execute(&self.pool)
        .await?;

        let cp = sqlx::query_as::<_, RunCheckpoint>("SELECT * FROM run_checkpoints WHERE id = ?")
            .bind(&id)
            .fetch_one(&self.pool)
            .await?;
        Ok(cp)
    }

    /// Finds runs with `running` or `waiting_approval` status for server restart recovery.
    pub async fn list_recoverable_runs(&self) -> Result<Vec<Run>> {
        let runs = sqlx::query_as::<_, Run>(
            "SELECT * FROM runs WHERE status IN ('running', 'waiting_approval')",
        )
        .fetch_all(&self.pool)
        .await?;
        Ok(runs)
    }

    pub async fn increment_resume_count(&self, run_id: &str) -> Result<()> {
        let now = now_ms();
        sqlx::query(
            "UPDATE runs SET resume_count = resume_count + 1, last_heartbeat_at = ? WHERE id = ?",
        )
        .bind(now)
        .bind(run_id)
        .execute(&self.pool)
        .await?;
        Ok(())
    }

    /// Safely transitions an interrupted run (`running` or `waiting_approval`) to `paused`,
    /// increments `resume_count`, expires any stale `pending` approvals for that run, and
    /// marks `running` or `awaiting_approval` tool calls as `error` with an interrupted reason
    /// so non-idempotent side effects are never re-executed automatically on crash recovery.
    pub async fn recover_interrupted_run(&self, run_id: &str) -> Result<Run> {
        let now = now_ms();
        let mut tx = self.pool.begin().await?;

        sqlx::query(
            r#"
            UPDATE approvals
            SET status = 'expired', decided_at = ?
            WHERE status = 'pending'
              AND tool_call_id IN (SELECT id FROM tool_calls WHERE run_id = ?)
            "#,
        )
        .bind(now)
        .bind(run_id)
        .execute(&mut *tx)
        .await?;

        sqlx::query(
            r#"
            UPDATE tool_calls
            SET status = 'error',
                result = '{"error":"interrupted_by_restart","recoverable":true}'
            WHERE run_id = ?
              AND status IN ('running', 'awaiting_approval')
            "#,
        )
        .bind(run_id)
        .execute(&mut *tx)
        .await?;

        sqlx::query(
            r#"
            UPDATE runs
            SET status = 'paused',
                resume_count = resume_count + 1,
                last_heartbeat_at = ?
            WHERE id = ?
            "#,
        )
        .bind(now)
        .bind(run_id)
        .execute(&mut *tx)
        .await?;

        let event_id = new_id();
        let next_seq: i64 =
            sqlx::query_scalar("SELECT COALESCE(MAX(seq), 0) + 1 FROM run_events WHERE run_id = ?")
                .bind(run_id)
                .fetch_one(&mut *tx)
                .await?;

        let payload = serde_json::json!({
            "new_status": "paused",
            "reason": "server_restart_recovery"
        })
        .to_string();

        sqlx::query(
            r#"
            INSERT INTO run_events (id, run_id, seq, kind, payload, created_at)
            VALUES (?, ?, ?, 'run.recovered', ?, ?)
            "#,
        )
        .bind(&event_id)
        .bind(run_id)
        .bind(next_seq)
        .bind(&payload)
        .bind(now)
        .execute(&mut *tx)
        .await?;

        tx.commit().await?;

        let updated = sqlx::query_as::<_, Run>("SELECT * FROM runs WHERE id = ?")
            .bind(run_id)
            .fetch_one(&self.pool)
            .await?;
        Ok(updated)
    }

    pub async fn record_tool_call(
        &self,
        run_id: &str,
        step_index: i64,
        source: &str,
        tool_name: &str,
        args_json: &str,
        status: &str,
    ) -> Result<ToolCall> {
        let id = new_id();
        let now = now_ms();

        sqlx::query(
            r#"
            INSERT INTO tool_calls (id, run_id, step_index, source, tool_name, args, status, started_at)
            VALUES (?, ?, ?, ?, ?, ?, ?, ?)
            "#,
        )
        .bind(&id)
        .bind(run_id)
        .bind(step_index)
        .bind(source)
        .bind(tool_name)
        .bind(args_json)
        .bind(status)
        .bind(now)
        .execute(&self.pool)
        .await?;

        let tc = sqlx::query_as::<_, ToolCall>("SELECT * FROM tool_calls WHERE id = ?")
            .bind(&id)
            .fetch_one(&self.pool)
            .await?;
        Ok(tc)
    }

    pub async fn create_approval(
        &self,
        tool_call_id: &str,
        kind: &str,
        summary: &str,
        command: Option<&str>,
        cwd: Option<&str>,
    ) -> Result<Approval> {
        let id = new_id();
        let now = now_ms();

        sqlx::query(
            r#"
            INSERT INTO approvals (id, tool_call_id, kind, summary, command, cwd, status, created_at)
            VALUES (?, ?, ?, ?, ?, ?, 'pending', ?)
            "#,
        )
        .bind(&id)
        .bind(tool_call_id)
        .bind(kind)
        .bind(summary)
        .bind(command)
        .bind(cwd)
        .bind(now)
        .execute(&self.pool)
        .await?;

        let approval = sqlx::query_as::<_, Approval>("SELECT * FROM approvals WHERE id = ?")
            .bind(&id)
            .fetch_one(&self.pool)
            .await?;
        Ok(approval)
    }
}
