use crate::{
    ActionableError, CompactionEngine, ContextAssembler, CorrelationIds, DataPreservation,
    FailureCategory, PermissionDecision, PermissionEngine, ToolCallContext,
    context_assembler::DEFAULT_CONTEXT_WINDOW,
    observability::{redact_log_text, summarize_tool_arguments},
    provider::{
        ModelCompletionRequest, ModelCompletionResponse, ModelProviderAdapter, ModelStreamEvent,
        ProposedToolCall, ProviderMessage, TokenUsage,
    },
};
use anyhow::{Context, Result, bail};
use async_trait::async_trait;
use futures::StreamExt;
use gaggle_db::{
    models::{Chat, Run},
    repositories::{
        chats::ChatRepository, extensions::ExtensionRepository, projects::ProjectRepository,
        runs::RunRepository,
    },
};
use serde::{Deserialize, Serialize};
use sqlx::SqlitePool;
use std::{
    collections::HashMap,
    sync::{
        Arc, RwLock,
        atomic::{AtomicBool, Ordering},
    },
    time::Instant,
};
use tracing::Instrument;

/// Default maximum agent loop iterations per run to prevent runaway tool-call loops.
pub const DEFAULT_MAX_AGENT_STEPS: usize = 8;

/// Thread-safe cooperative cancellation token for an active run.
#[derive(Clone, Default)]
pub struct CancellationToken {
    cancelled: Arc<AtomicBool>,
}

impl CancellationToken {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn cancel(&self) {
        self.cancelled.store(true, Ordering::SeqCst);
    }

    pub fn is_cancelled(&self) -> bool {
        self.cancelled.load(Ordering::SeqCst)
    }
}

/// Registry of active run cancellation tokens so HTTP/CLI endpoints can cancel an in-flight run.
#[derive(Clone, Default)]
pub struct ActiveRunRegistry {
    tokens: Arc<RwLock<HashMap<String, CancellationToken>>>,
}

impl ActiveRunRegistry {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn register(&self, run_id: &str) -> CancellationToken {
        let token = CancellationToken::new();
        if let Ok(mut guard) = self.tokens.write() {
            guard.insert(run_id.to_string(), token.clone());
        }
        token
    }

    /// Register an existing cancellation token under `run_id` so external callers can cancel it.
    pub fn register_token(&self, run_id: &str, token: CancellationToken) {
        if let Ok(mut guard) = self.tokens.write() {
            guard.insert(run_id.to_string(), token);
        }
    }

    pub fn cancel(&self, run_id: &str) -> bool {
        if let Ok(guard) = self.tokens.read()
            && let Some(token) = guard.get(run_id)
        {
            token.cancel();
            return true;
        }
        false
    }

    pub fn unregister(&self, run_id: &str) {
        if let Ok(mut guard) = self.tokens.write() {
            guard.remove(run_id);
        }
    }
}

/// Abstract executor for approved tools so `gaggle-core` stays decoupled from `gaggle-tools`
/// implementation details while still orchestrating the full runtime loop.
#[async_trait]
pub trait ToolExecutor: Send + Sync {
    async fn execute_tool(
        &self,
        run_id: &str,
        chat: &Chat,
        project_root: &str,
        tool_name: &str,
        args_json: &str,
    ) -> Result<String, String>;
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "outcome", rename_all = "snake_case")]
pub enum AgentLoopOutcome {
    Completed {
        run: Run,
        final_text: String,
        steps_executed: usize,
    },
    WaitingApproval {
        run: Run,
        pending_approval_id: String,
        tool_call_id: String,
    },
    Cancelled {
        run: Run,
    },
    Failed {
        run: Run,
        error: String,
    },
}

/// Executes Gaggle's bounded 12-step agent loop with permission enforcement,
/// checkpointing, ordered `run_events` persistence, and cooperative cancellation.
pub struct AgentRuntime {
    pool: SqlitePool,
    max_steps: usize,
    active_runs: Option<ActiveRunRegistry>,
}

impl AgentRuntime {
    pub fn new(pool: SqlitePool, max_steps: usize) -> Self {
        Self {
            pool,
            max_steps,
            active_runs: None,
        }
    }

    /// Attach an [`ActiveRunRegistry`] so cancellation tokens are registered under the
    /// durable run id and can be reached by `/api/runs/{id}/cancel`.
    pub fn with_active_runs(mut self, registry: ActiveRunRegistry) -> Self {
        self.active_runs = Some(registry);
        self
    }

    pub async fn run_turn(
        &self,
        chat_id: &str,
        user_prompt: &str,
        model_key: &str,
        provider: &dyn ModelProviderAdapter,
        tool_executor: &dyn ToolExecutor,
        cancel_token: &CancellationToken,
    ) -> Result<AgentLoopOutcome> {
        let span = CorrelationIds::for_chat(chat_id).span("agent.run");
        self.run_turn_inner(
            chat_id,
            user_prompt,
            model_key,
            provider,
            tool_executor,
            cancel_token,
            self.active_runs.clone(),
        )
        .instrument(span)
        .await
    }

    #[allow(clippy::too_many_arguments)]
    #[allow(clippy::too_many_arguments)]
    async fn run_turn_inner(
        &self,
        chat_id: &str,
        user_prompt: &str,
        model_key: &str,
        provider: &dyn ModelProviderAdapter,
        tool_executor: &dyn ToolExecutor,
        cancel_token: &CancellationToken,
        active_runs: Option<ActiveRunRegistry>,
    ) -> Result<AgentLoopOutcome> {
        let chat_repo = ChatRepository::new(self.pool.clone());
        let proj_repo = ProjectRepository::new(self.pool.clone());
        let run_repo = RunRepository::new(self.pool.clone());
        let ext_repo = ExtensionRepository::new(self.pool.clone());
        let perm_engine = PermissionEngine::new(self.pool.clone());

        let chat = chat_repo
            .get_by_id(chat_id)
            .await?
            .with_context(|| format!("Chat '{chat_id}' not found"))?;
        let project = proj_repo
            .get_by_id(&chat.project_id)
            .await?
            .with_context(|| format!("Project '{}' not found", chat.project_id))?;

        // 0. Auto-compaction: summarize older history when token ratio exceeds threshold.
        let resolved_context_window: i64 = if let Some(model_id) = chat.model_id.as_deref() {
            sqlx::query_scalar::<_, i64>("SELECT context_window FROM models WHERE id = ?")
                .bind(model_id)
                .fetch_optional(&self.pool)
                .await?
                .filter(|w| *w > 0)
                .unwrap_or(DEFAULT_CONTEXT_WINDOW)
        } else {
            DEFAULT_CONTEXT_WINDOW
        };
        let _ = CompactionEngine::maybe_compact_chat(
            self.pool.clone(),
            chat_id,
            resolved_context_window,
            provider,
            "auto",
        )
        .await;

        // 1. Assemble bounded context before persisting the current user message. The assembler
        // loads project instructions, AGENTS.md, recent history, relevant memories, and enabled
        // tool definitions while excluding secrets.
        let assembled = ContextAssembler::new(self.pool.clone())
            .assemble(chat_id, user_prompt)
            .await?;

        // 2. Persist user message and create durable run
        let user_msg = chat_repo
            .append_message(chat_id, None, "user", user_prompt)
            .await?;
        let run = run_repo
            .create(chat_id, Some(&user_msg.id), chat.model_id.as_deref())
            .await?;
        if let Some(registry) = active_runs.as_ref() {
            registry.register_token(&run.id, cancel_token.clone());
        }
        let correlation = CorrelationIds::for_chat(chat_id).with_run(&run.id);
        let run_repo = run_repo.with_correlation(correlation.clone());
        let perm_engine = perm_engine.with_correlation(correlation.clone());

        tracing::info!(
            target: "gaggle.agent",
            run_id = %run.id,
            model_key = %model_key,
            max_steps = self.max_steps,
            prompt_chars = user_prompt.chars().count(),
            "Agent run started"
        );

        run_repo
            .emit_event(
                &run.id,
                "run.started",
                &serde_json::json!({
                    "chat_id": chat_id,
                    "trigger_message_id": user_msg.id,
                    "model_key": model_key
                })
                .to_string(),
            )
            .await?;

        run_repo
            .emit_event(
                &run.id,
                "context.assembled",
                &serde_json::json!({
                    "estimated_tokens": assembled.estimated_tokens,
                    "budget_tokens": assembled.budget_tokens,
                    "truncated": assembled.truncated,
                    "agents_md": assembled.agents_md_paths,
                    "tool_count": assembled.tools.len()
                })
                .to_string(),
            )
            .await?;

        tracing::info!(
            target: "gaggle.agent",
            run_id = %run.id,
            estimated_tokens = assembled.estimated_tokens,
            budget_tokens = assembled.budget_tokens,
            truncated = assembled.truncated,
            tool_count = assembled.tools.len(),
            "Agent context assembled"
        );

        let mut conversation = assembled.messages;
        let tool_definitions = assembled.tools;

        for step_idx in 1..=self.max_steps {
            if cancel_token.is_cancelled() {
                return self.finish_cancelled(&run_repo, &run.id).await;
            }

            let step_start = Instant::now();
            tracing::debug!(
                target: "gaggle.agent",
                run_id = %run.id,
                step = step_idx,
                message_count = conversation.len(),
                tool_count = tool_definitions.len(),
                "Agent step started"
            );
            let req = ModelCompletionRequest {
                model_key: model_key.to_string(),
                messages: conversation.clone(),
                tools: tool_definitions.clone(),
                max_output_tokens: Some(1024),
            };

            // Prefer native provider streaming when available; otherwise fall back to a single
            // blocking completion. Both paths normalize to `ModelCompletionResponse`.
            let mut used_native_stream = false;
            let completion_result: Result<ModelCompletionResponse, crate::ProviderError> =
                if let Some(mut stream) = provider.stream(req.clone()) {
                    used_native_stream = true;
                    let mut streamed_text = String::new();
                    let mut streamed_calls = Vec::new();
                    let mut streamed_usage = TokenUsage::default();
                    let mut done_response = None;
                    let mut stream_error = None;

                    while let Some(item) = stream.next().await {
                        if cancel_token.is_cancelled() {
                            return self.finish_cancelled(&run_repo, &run.id).await;
                        }
                        match item {
                            Ok(ModelStreamEvent::TextDelta(delta)) => {
                                if !delta.is_empty() {
                                    streamed_text.push_str(&delta);
                                    run_repo
                                        .emit_event(
                                            &run.id,
                                            "message.delta",
                                            &serde_json::json!({
                                                "step": step_idx,
                                                "text": delta
                                            })
                                            .to_string(),
                                        )
                                        .await?;
                                }
                            }
                            Ok(ModelStreamEvent::ToolCall(call)) => streamed_calls.push(call),
                            Ok(ModelStreamEvent::Usage(usage)) => streamed_usage = usage,
                            Ok(ModelStreamEvent::Done(response)) => done_response = Some(response),
                            Err(err) => {
                                stream_error = Some(err);
                                break;
                            }
                        }
                    }

                    match (stream_error, done_response) {
                        (Some(err), _) => Err(err),
                        (None, Some(response)) => {
                            if response.text.is_empty() && !streamed_text.is_empty() {
                                Ok(ModelCompletionResponse {
                                    text: streamed_text,
                                    ..response
                                })
                            } else if response.tool_calls.is_empty() && !streamed_calls.is_empty() {
                                Ok(ModelCompletionResponse {
                                    tool_calls: streamed_calls,
                                    ..response
                                })
                            } else if response.usage == TokenUsage::default()
                                && streamed_usage != TokenUsage::default()
                            {
                                Ok(ModelCompletionResponse {
                                    usage: streamed_usage,
                                    ..response
                                })
                            } else {
                                Ok(response)
                            }
                        }
                        (None, None) => Ok(ModelCompletionResponse {
                            text: streamed_text,
                            tool_calls: streamed_calls,
                            usage: streamed_usage,
                        }),
                    }
                } else {
                    provider.complete(&req).await
                };

            let completion = match completion_result {
                Ok(resp) => resp,
                Err(err) => {
                    let failure =
                        ActionableError::from_provider_error("agent.turn", &err, &correlation);
                    tracing::error!(
                        target: "gaggle.agent",
                        run_id = %run.id,
                        step = step_idx,
                        failure_category = failure.category.as_str(),
                        operation = %failure.operation,
                        data_preservation = failure.data_preservation.as_str(),
                        retryable = failure.retryable,
                        error = %failure.message,
                        "Agent run failed while waiting for the model provider"
                    );
                    return self
                        .finish_failed(&run_repo, &run.id, Some(step_idx), failure)
                        .await;
                }
            };

            let latency_ms = step_start.elapsed().as_millis() as i64;
            tracing::info!(
                target: "gaggle.agent",
                run_id = %run.id,
                step = step_idx,
                latency_ms,
                input_tokens = completion.usage.input_tokens,
                output_tokens = completion.usage.output_tokens,
                cached_tokens = completion.usage.cached_tokens,
                tool_call_count = completion.tool_calls.len(),
                native_stream = used_native_stream,
                "Model completion received"
            );
            run_repo
                .record_usage(
                    &run.id,
                    chat.model_id.as_deref(),
                    completion.usage.input_tokens,
                    completion.usage.output_tokens,
                    completion.usage.cached_tokens,
                    latency_ms,
                )
                .await?;

            if !used_native_stream && !completion.text.is_empty() {
                run_repo
                    .emit_event(
                        &run.id,
                        "message.delta",
                        &serde_json::json!({
                            "step": step_idx,
                            "text": completion.text
                        })
                        .to_string(),
                    )
                    .await?;
            }

            // If no tool calls are requested, persist assistant message and complete the run
            if completion.tool_calls.is_empty() {
                chat_repo
                    .append_message(chat_id, Some(&run.id), "assistant", &completion.text)
                    .await?;
                run_repo
                    .save_checkpoint(
                        &run.id,
                        step_idx as i64,
                        &serde_json::json!({ "completed": true, "step": step_idx }).to_string(),
                    )
                    .await?;
                run_repo
                    .emit_event(
                        &run.id,
                        "run.completed",
                        &serde_json::json!({ "steps_executed": step_idx }).to_string(),
                    )
                    .await?;
                let completed_run = run_repo
                    .update_run_status(&run.id, "completed", None)
                    .await?;
                return Ok(AgentLoopOutcome::Completed {
                    run: completed_run,
                    final_text: completion.text,
                    steps_executed: step_idx,
                });
            }

            // Process proposed tool calls through the 7-step PermissionEngine
            for proposed in &completion.tool_calls {
                if cancel_token.is_cancelled() {
                    return self.finish_cancelled(&run_repo, &run.id).await;
                }

                let Some(ext_tool) = ext_repo
                    .find_enabled_tool_by_name(&proposed.tool_name)
                    .await?
                else {
                    let failure = ActionableError::new(
                        FailureCategory::Tool,
                        "agent.tool_lookup",
                        format!("Unknown or disabled tool '{}'", proposed.tool_name),
                    )
                    .with_data_preservation(DataPreservation::Preserved)
                    .with_correlation(&correlation);
                    tracing::error!(
                        target: "gaggle.agent",
                        run_id = %run.id,
                        step = step_idx,
                        tool_name = %proposed.tool_name,
                        failure_category = failure.category.as_str(),
                        "Proposed tool is unknown or disabled"
                    );
                    bail!("{failure}");
                };

                let shell_cmd =
                    Self::extract_shell_command(&proposed.tool_name, &proposed.arguments_json);
                let ctx = ToolCallContext {
                    project_id: Some(project.id.clone()),
                    permission_mode: chat.permission_mode.clone(),
                    extension_tool: ext_tool,
                    args_json: proposed.arguments_json.clone(),
                    shell_command: shell_cmd.clone(),
                };

                let decision = perm_engine.evaluate(&ctx).await?;
                match decision {
                    PermissionDecision::Deny { reason } => {
                        tracing::warn!(
                            target: "gaggle.agent",
                            run_id = %run.id,
                            step = step_idx,
                            tool_name = %proposed.tool_name,
                            decision = "deny",
                            reason = %redact_log_text(&reason),
                            "Tool call denied by policy"
                        );
                        let tc = run_repo
                            .record_tool_call(
                                &run.id,
                                step_idx as i64,
                                "native",
                                &proposed.tool_name,
                                &proposed.arguments_json,
                                "denied",
                            )
                            .await?;
                        let deny_json = serde_json::json!({ "denied": reason }).to_string();
                        run_repo
                            .update_tool_call_result(&tc.id, "denied", Some(&deny_json), 0)
                            .await?;
                        run_repo
                            .emit_event(
                                &run.id,
                                "tool.denied",
                                &serde_json::json!({
                                    "tool_call_id": tc.id,
                                    "tool_name": proposed.tool_name,
                                    "reason": reason
                                })
                                .to_string(),
                            )
                            .await?;
                        conversation.push(ProviderMessage {
                            role: "tool".to_string(),
                            content: deny_json,
                        });
                    }
                    PermissionDecision::NeedsApproval { kind, summary } => {
                        tracing::info!(
                            target: "gaggle.agent",
                            run_id = %run.id,
                            step = step_idx,
                            tool_name = %proposed.tool_name,
                            decision = "needs_approval",
                            approval_kind = %kind,
                            "Tool call is waiting for user approval"
                        );
                        let tc = run_repo
                            .record_tool_call(
                                &run.id,
                                step_idx as i64,
                                "native",
                                &proposed.tool_name,
                                &proposed.arguments_json,
                                "awaiting_approval",
                            )
                            .await?;
                        let approval = run_repo
                            .create_approval(
                                &tc.id,
                                &kind,
                                &summary,
                                shell_cmd.as_deref(),
                                Some(&project.root_path),
                            )
                            .await?;
                        run_repo
                            .save_checkpoint(
                                &run.id,
                                step_idx as i64,
                                &serde_json::json!({
                                    "waiting_approval_id": approval.id,
                                    "tool_call_id": tc.id
                                })
                                .to_string(),
                            )
                            .await?;
                        run_repo
                            .emit_event(
                                &run.id,
                                "approval.requested",
                                &serde_json::json!({
                                    "approval_id": approval.id,
                                    "tool_call_id": tc.id,
                                    "tool_name": proposed.tool_name,
                                    "summary": summary,
                                    "command": shell_cmd,
                                    "cwd": project.root_path
                                })
                                .to_string(),
                            )
                            .await?;
                        let waiting_run = run_repo
                            .update_run_status(&run.id, "waiting_approval", None)
                            .await?;
                        return Ok(AgentLoopOutcome::WaitingApproval {
                            run: waiting_run,
                            pending_approval_id: approval.id,
                            tool_call_id: tc.id,
                        });
                    }
                    PermissionDecision::Allow => {
                        tracing::debug!(
                            target: "gaggle.agent",
                            run_id = %run.id,
                            step = step_idx,
                            tool_name = %proposed.tool_name,
                            decision = "allow",
                            "Tool call allowed by policy"
                        );
                        let tool_out = self
                            .execute_allowed_tool(
                                &run_repo,
                                &run.id,
                                step_idx as i64,
                                &chat,
                                &project.root_path,
                                proposed,
                                tool_executor,
                            )
                            .await?;
                        conversation.push(ProviderMessage {
                            role: "tool".to_string(),
                            content: tool_out,
                        });
                    }
                }
            }

            run_repo
                .save_checkpoint(
                    &run.id,
                    step_idx as i64,
                    &serde_json::json!({ "step": step_idx }).to_string(),
                )
                .await?;
        }

        // Budget exhausted
        let failure = ActionableError::new(
            FailureCategory::BudgetExhausted,
            "agent.turn",
            format!("Maximum agent step budget ({}) exhausted", self.max_steps),
        )
        .with_data_preservation(DataPreservation::Preserved)
        .with_correlation(&correlation);
        tracing::warn!(
            target: "gaggle.agent",
            run_id = %run.id,
            max_steps = self.max_steps,
            failure_category = failure.category.as_str(),
            "Agent step budget exhausted"
        );
        self.finish_failed(&run_repo, &run.id, None, failure).await
    }

    /// Marks the run failed, appends a structured `run.failed` event, and returns the outcome.
    async fn finish_failed(
        &self,
        run_repo: &RunRepository,
        run_id: &str,
        step: Option<usize>,
        failure: ActionableError,
    ) -> Result<AgentLoopOutcome> {
        let display_error = failure.to_string();
        let payload = serde_json::json!({
            "step": step,
            "error": display_error,
            "failure_category": failure.category.as_str(),
            "operation": failure.operation,
            "data_preservation": failure.data_preservation.as_str(),
            "retryable": failure.retryable,
            "retry_guidance": failure.retry_guidance,
        })
        .to_string();

        run_repo.emit_event(run_id, "run.failed", &payload).await?;
        let failed_run = run_repo
            .update_run_status(run_id, "failed", Some(&display_error))
            .await?;
        Ok(AgentLoopOutcome::Failed {
            run: failed_run,
            error: display_error,
        })
    }

    #[allow(clippy::too_many_arguments)]
    async fn execute_allowed_tool(
        &self,
        run_repo: &RunRepository,
        run_id: &str,
        step_index: i64,
        chat: &Chat,
        project_root: &str,
        proposed: &ProposedToolCall,
        tool_executor: &dyn ToolExecutor,
    ) -> Result<String> {
        let tc = run_repo
            .record_tool_call(
                run_id,
                step_index,
                "native",
                &proposed.tool_name,
                &proposed.arguments_json,
                "running",
            )
            .await?;
        tracing::debug!(
            target: "gaggle.agent",
            run_id = %run_id,
            tool_call_id = %tc.id,
            tool_name = %proposed.tool_name,
            step = step_index,
            args = %summarize_tool_arguments(&proposed.tool_name, &proposed.arguments_json),
            "Tool call started"
        );
        run_repo
            .emit_event(
                run_id,
                "tool.started",
                &serde_json::json!({
                    "tool_call_id": tc.id,
                    "tool_name": proposed.tool_name
                })
                .to_string(),
            )
            .await?;

        let started = Instant::now();
        let tool_span = CorrelationIds::for_chat(&chat.id)
            .with_run(run_id)
            .with_tool_call(&tc.id)
            .span("agent.tool_call");
        let exec_res = tool_executor
            .execute_tool(
                run_id,
                chat,
                project_root,
                &proposed.tool_name,
                &proposed.arguments_json,
            )
            .instrument(tool_span)
            .await;
        let duration_ms = started.elapsed().as_millis() as i64;

        match exec_res {
            Ok(result_json) => {
                tracing::info!(
                    target: "gaggle.agent",
                    run_id = %run_id,
                    tool_call_id = %tc.id,
                    tool_name = %proposed.tool_name,
                    step = step_index,
                    duration_ms,
                    result_bytes = result_json.len(),
                    status = "ok",
                    "Tool call completed"
                );
                run_repo
                    .update_tool_call_result(&tc.id, "ok", Some(&result_json), duration_ms)
                    .await?;
                run_repo
                    .emit_event(
                        run_id,
                        "tool.completed",
                        &serde_json::json!({
                            "tool_call_id": tc.id,
                            "tool_name": proposed.tool_name,
                            "duration_ms": duration_ms
                        })
                        .to_string(),
                    )
                    .await?;
                Ok(result_json)
            }
            Err(err_str) => {
                tracing::warn!(
                    target: "gaggle.agent",
                    run_id = %run_id,
                    tool_call_id = %tc.id,
                    tool_name = %proposed.tool_name,
                    step = step_index,
                    duration_ms,
                    failure_category = FailureCategory::Tool.as_str(),
                    error = %redact_log_text(&err_str),
                    "Tool call failed"
                );
                let err_json = serde_json::json!({ "error": err_str }).to_string();
                run_repo
                    .update_tool_call_result(&tc.id, "error", Some(&err_json), duration_ms)
                    .await?;
                run_repo
                    .emit_event(
                        run_id,
                        "tool.failed",
                        &serde_json::json!({
                            "tool_call_id": tc.id,
                            "tool_name": proposed.tool_name,
                            "error": err_str
                        })
                        .to_string(),
                    )
                    .await?;
                Ok(err_json)
            }
        }
    }

    async fn finish_cancelled(
        &self,
        run_repo: &RunRepository,
        run_id: &str,
    ) -> Result<AgentLoopOutcome> {
        tracing::info!(
            target: "gaggle.agent",
            run_id = %run_id,
            failure_category = FailureCategory::Cancelled.as_str(),
            data_preservation = DataPreservation::Preserved.as_str(),
            "Agent run cancelled by user"
        );
        run_repo
            .emit_event(
                run_id,
                "run.cancelled",
                &serde_json::json!({ "reason": "user_cancelled" }).to_string(),
            )
            .await?;
        let cancelled_run = run_repo
            .update_run_status(run_id, "cancelled", Some("Cancelled by user"))
            .await?;
        Ok(AgentLoopOutcome::Cancelled { run: cancelled_run })
    }

    fn extract_shell_command(tool_name: &str, args_json: &str) -> Option<String> {
        if tool_name == "shell.exec"
            && let Ok(val) = serde_json::from_str::<serde_json::Value>(args_json)
        {
            return val
                .get("command")
                .and_then(|v| v.as_str())
                .map(|s| s.to_string());
        }
        None
    }
}
