use crate::{
    connection::{new_id, now_ms},
    correlation::CorrelationIds,
    models::{Approval, AuditLog, FileChange, Run, RunCheckpoint, RunEvent, ToolCall},
};
use anyhow::{Context, Result, bail};
use sqlx::SqlitePool;

pub struct RunRepository {
    pool: SqlitePool,
    correlation: CorrelationIds,
}

impl RunRepository {
    pub fn new(pool: SqlitePool) -> Self {
        Self {
            pool,
            correlation: CorrelationIds::default(),
        }
    }

    /// Attaches chat/run/tool correlation identifiers so repository log entries carry the same
    /// fields as the runtime that issued the call.
    pub fn with_correlation(mut self, correlation: CorrelationIds) -> Self {
        self.correlation = correlation;
        self
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
        tracing::debug!(
            target: "gaggle.db.runs",
            chat_id = %chat_id,
            run_id = %run.id,
            tool_call_id = %self.correlation.tool_call_id_field(),
            model_id = %model_id.unwrap_or("-"),
            "Durable run row created"
        );
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
        // Event payloads can contain model deltas and tool results; only metadata is logged.
        tracing::debug!(
            target: "gaggle.db.runs",
            chat_id = %self.correlation.chat_id_field(),
            run_id = %run_id,
            tool_call_id = %self.correlation.tool_call_id_field(),
            event_kind = %kind,
            event_seq = ev.seq,
            "Durable run event appended"
        );
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
        tracing::info!(
            target: "gaggle.db.runs",
            chat_id = %updated.chat_id,
            run_id = %updated.id,
            tool_call_id = %self.correlation.tool_call_id_field(),
            new_status = %updated.status,
            resume_count = updated.resume_count,
            "Interrupted run recovered without replaying side effects"
        );
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
        tracing::debug!(
            target: "gaggle.db.runs",
            chat_id = %self.correlation.chat_id_field(),
            run_id = %run_id,
            tool_call_id = %tc.id,
            tool_name = %tc.tool_name,
            step_index,
            args_bytes = args_json.len(),
            status = %status,
            "Tool call recorded"
        );
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
        tracing::info!(
            target: "gaggle.db.runs",
            chat_id = %self.correlation.chat_id_field(),
            run_id = %self.correlation.run_id_field(),
            tool_call_id = %tool_call_id,
            approval_id = %approval.id,
            approval_kind = %approval.kind,
            has_command = command.is_some(),
            has_working_directory = cwd.is_some(),
            "Approval requested for tool call"
        );
        Ok(approval)
    }

    pub async fn list_pending_approvals(&self) -> Result<Vec<Approval>> {
        let approvals = sqlx::query_as::<_, Approval>(
            "SELECT * FROM approvals WHERE status = 'pending' ORDER BY created_at ASC",
        )
        .fetch_all(&self.pool)
        .await?;
        Ok(approvals)
    }

    pub async fn get_approval_by_id(&self, id: &str) -> Result<Option<Approval>> {
        let approval = sqlx::query_as::<_, Approval>("SELECT * FROM approvals WHERE id = ?")
            .bind(id)
            .fetch_optional(&self.pool)
            .await?;
        Ok(approval)
    }

    pub async fn get_tool_call_by_id(&self, id: &str) -> Result<Option<ToolCall>> {
        let tc = sqlx::query_as::<_, ToolCall>("SELECT * FROM tool_calls WHERE id = ?")
            .bind(id)
            .fetch_optional(&self.pool)
            .await?;
        Ok(tc)
    }

    pub async fn get_run_by_id(&self, id: &str) -> Result<Option<Run>> {
        let run = sqlx::query_as::<_, Run>("SELECT * FROM runs WHERE id = ?")
            .bind(id)
            .fetch_optional(&self.pool)
            .await?;
        Ok(run)
    }

    pub async fn update_run_status(
        &self,
        run_id: &str,
        status: &str,
        error: Option<&str>,
    ) -> Result<Run> {
        let now = now_ms();
        let finished_at = if matches!(status, "completed" | "failed" | "cancelled") {
            Some(now)
        } else {
            None
        };

        sqlx::query(
            r#"
            UPDATE runs
            SET status = ?,
                error = ?,
                last_heartbeat_at = ?,
                finished_at = COALESCE(?, finished_at)
            WHERE id = ?
            "#,
        )
        .bind(status)
        .bind(error)
        .bind(now)
        .bind(finished_at)
        .bind(run_id)
        .execute(&self.pool)
        .await?;

        self.get_run_by_id(run_id)
            .await?
            .with_context(|| format!("Run '{run_id}' not found after update"))
            .inspect(|run| {
                tracing::info!(
                    target: "gaggle.db.runs",
                    chat_id = %run.chat_id,
                    run_id = %run.id,
                    tool_call_id = %self.correlation.tool_call_id_field(),
                    status = %run.status,
                    has_error = run.error.is_some(),
                    "Durable run status updated"
                );
            })
    }

    pub async fn update_tool_call_result(
        &self,
        tool_call_id: &str,
        status: &str,
        result_json: Option<&str>,
        duration_ms: i64,
    ) -> Result<ToolCall> {
        sqlx::query(
            r#"
            UPDATE tool_calls
            SET status = ?,
                result = ?,
                duration_ms = ?
            WHERE id = ?
            "#,
        )
        .bind(status)
        .bind(result_json)
        .bind(duration_ms)
        .bind(tool_call_id)
        .execute(&self.pool)
        .await?;

        self.get_tool_call_by_id(tool_call_id)
            .await?
            .with_context(|| format!("ToolCall '{tool_call_id}' not found after update"))
            .inspect(|tool_call| {
                tracing::debug!(
                    target: "gaggle.db.runs",
                    chat_id = %self.correlation.chat_id_field(),
                    run_id = %tool_call.run_id,
                    tool_call_id = %tool_call.id,
                    tool_name = %tool_call.tool_name,
                    status = %status,
                    duration_ms,
                    "Tool call result recorded"
                );
            })
    }

    pub async fn list_run_events(&self, run_id: &str, after_seq: i64) -> Result<Vec<RunEvent>> {
        let events = sqlx::query_as::<_, RunEvent>(
            "SELECT * FROM run_events WHERE run_id = ? AND seq > ? ORDER BY seq ASC",
        )
        .bind(run_id)
        .bind(after_seq)
        .fetch_all(&self.pool)
        .await?;
        Ok(events)
    }

    pub async fn record_usage(
        &self,
        run_id: &str,
        model_id: Option<&str>,
        input_tokens: i64,
        output_tokens: i64,
        cached_tokens: i64,
        latency_ms: i64,
    ) -> Result<()> {
        let id = new_id();
        let now = now_ms();

        let mut tx = self.pool.begin().await?;

        sqlx::query(
            r#"
            INSERT INTO usage_records (
                id, run_id, model_id, input_tokens, output_tokens, cached_tokens, cost_usd, latency_ms, created_at
            ) VALUES (?, ?, ?, ?, ?, ?, 0.0, ?, ?)
            "#,
        )
        .bind(&id)
        .bind(run_id)
        .bind(model_id)
        .bind(input_tokens)
        .bind(output_tokens)
        .bind(cached_tokens)
        .bind(latency_ms)
        .bind(now)
        .execute(&mut *tx)
        .await?;

        sqlx::query(
            r#"
            UPDATE chats
            SET total_input_tokens = total_input_tokens + ?,
                total_output_tokens = total_output_tokens + ?,
                updated_at = ?
            WHERE id = (SELECT chat_id FROM runs WHERE id = ?)
            "#,
        )
        .bind(input_tokens)
        .bind(output_tokens)
        .bind(now)
        .bind(run_id)
        .execute(&mut *tx)
        .await?;

        tx.commit().await?;
        Ok(())
    }

    /// Atomically records a user decision (`approved` or `denied`) on a `pending` approval,
    /// optionally creates a `command_rules` entry when `remember_pattern` is provided,
    /// updates `tool_calls.status` (`denied` if rejected), and writes an `audit_logs` record.
    pub async fn decide_approval(
        &self,
        approval_id: &str,
        approved: bool,
        decided_by_device: Option<&str>,
        project_id: Option<&str>,
        remember_pattern: Option<(&str, &str)>, // (pattern, match_kind)
    ) -> Result<Approval> {
        let now = now_ms();
        let mut tx = self.pool.begin().await?;

        let existing = sqlx::query_as::<_, Approval>("SELECT * FROM approvals WHERE id = ?")
            .bind(approval_id)
            .fetch_optional(&mut *tx)
            .await?
            .with_context(|| format!("Approval '{approval_id}' not found"))?;

        if existing.status != "pending" {
            bail!(
                "Approval '{}' is already in '{}' status and cannot be decided again",
                approval_id,
                existing.status
            );
        }

        let new_status = if approved { "approved" } else { "denied" };
        let rule_action = if approved { "allow" } else { "deny" };

        let mut remember_rule_id: Option<String> = None;
        if let Some((pattern, match_kind)) = remember_pattern {
            let rule_id = new_id();
            sqlx::query(
                r#"
                INSERT INTO command_rules (id, project_id, pattern, match_kind, action, created_at)
                VALUES (?, ?, ?, ?, ?, ?)
                "#,
            )
            .bind(&rule_id)
            .bind(project_id)
            .bind(pattern)
            .bind(match_kind)
            .bind(rule_action)
            .bind(now)
            .execute(&mut *tx)
            .await?;
            remember_rule_id = Some(rule_id);
        }

        sqlx::query(
            r#"
            UPDATE approvals
            SET status = ?,
                decided_by_device = ?,
                remember_rule_id = ?,
                decided_at = ?
            WHERE id = ?
            "#,
        )
        .bind(new_status)
        .bind(decided_by_device)
        .bind(&remember_rule_id)
        .bind(now)
        .bind(approval_id)
        .execute(&mut *tx)
        .await?;

        if !approved {
            sqlx::query(
                r#"
                UPDATE tool_calls
                SET status = 'denied',
                    result = '{"error":"approval_denied_by_user"}'
                WHERE id = ?
                "#,
            )
            .bind(&existing.tool_call_id)
            .execute(&mut *tx)
            .await?;
        }

        let audit_id = new_id();
        let metadata = serde_json::json!({
            "tool_call_id": existing.tool_call_id,
            "kind": existing.kind,
            "decision": new_status,
            "remember_rule_id": remember_rule_id,
        })
        .to_string();

        sqlx::query(
            r#"
            INSERT INTO audit_logs (id, actor_kind, actor_id, action, entity_type, entity_id, metadata, created_at)
            VALUES (?, 'user', ?, ?, 'approval', ?, ?, ?)
            "#,
        )
        .bind(&audit_id)
        .bind(decided_by_device)
        .bind(format!("approval.{new_status}"))
        .bind(approval_id)
        .bind(&metadata)
        .bind(now)
        .execute(&mut *tx)
        .await?;

        tx.commit().await?;

        let updated = sqlx::query_as::<_, Approval>("SELECT * FROM approvals WHERE id = ?")
            .bind(approval_id)
            .fetch_one(&self.pool)
            .await?;
        tracing::info!(
            target: "gaggle.db.runs",
            chat_id = %self.correlation.chat_id_field(),
            run_id = %self.correlation.run_id_field(),
            tool_call_id = %updated.tool_call_id,
            approval_id = %updated.id,
            status = %updated.status,
            remembered_rule = remember_rule_id.is_some(),
            "Approval decision recorded"
        );
        Ok(updated)
    }

    #[allow(clippy::too_many_arguments)]
    pub async fn record_file_change(
        &self,
        run_id: &str,
        chat_id: &str,
        path: &str,
        change_type: &str,
        additions: i64,
        deletions: i64,
        patch: &str,
        before_sha: Option<&str>,
        after_sha: &str,
    ) -> Result<FileChange> {
        let id = new_id();
        let now = now_ms();

        sqlx::query(
            r#"
            INSERT INTO file_changes (
                id, run_id, chat_id, path, change_type, additions, deletions,
                patch, before_sha, after_sha, status, created_at
            ) VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, 'pending', ?)
            "#,
        )
        .bind(&id)
        .bind(run_id)
        .bind(chat_id)
        .bind(path)
        .bind(change_type)
        .bind(additions)
        .bind(deletions)
        .bind(patch)
        .bind(before_sha)
        .bind(after_sha)
        .bind(now)
        .execute(&self.pool)
        .await?;

        let fc = sqlx::query_as::<_, FileChange>("SELECT * FROM file_changes WHERE id = ?")
            .bind(&id)
            .fetch_one(&self.pool)
            .await?;
        Ok(fc)
    }

    pub async fn list_audit_logs(&self, limit: i64) -> Result<Vec<AuditLog>> {
        let logs = sqlx::query_as::<_, AuditLog>(
            "SELECT * FROM audit_logs ORDER BY created_at DESC LIMIT ?",
        )
        .bind(limit)
        .fetch_all(&self.pool)
        .await?;
        Ok(logs)
    }
}
