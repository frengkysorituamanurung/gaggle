use crate::{
    ActionableError, CompactionEngine, ContextAssembler, CorrelationIds, DataPreservation,
    FailureCategory, InferenceOperationError, PermissionDecision, PermissionEngine,
    ToolCallContext,
    context_assembler::DEFAULT_CONTEXT_WINDOW,
    hooks::{HookContext, HookDecision, HookEvent, HookManager},
    observability::{redact_log_text, summarize_tool_arguments},
    operations::InferenceOperation,
    provider::{ModelCompletionRequest, ModelProviderAdapter, ProposedToolCall, ProviderMessage},
};
use anyhow::{Context, Result, bail};
use async_trait::async_trait;
use gaggle_db::{
    models::{Chat, Project, Run},
    repositories::{
        chats::ChatRepository, extensions::ExtensionRepository, projects::ProjectRepository,
        runs::RunRepository,
    },
};
use serde::{Deserialize, Serialize};
use sqlx::SqlitePool;
use std::{
    collections::{HashMap, HashSet},
    sync::{
        Arc, RwLock,
        atomic::{AtomicBool, Ordering},
    },
    time::Instant,
};
use tokio::sync::Notify;
use tracing::Instrument;

/// Default maximum agent loop iterations per run to prevent runaway tool-call loops.
pub const DEFAULT_MAX_AGENT_STEPS: usize = 8;

/// Maximum tool response size (in characters) before the result is spilled to a
/// temporary file and replaced with a reference message (mirrors Goose
/// `large_response_handler`).
pub const MAX_TOOL_RESPONSE_CHARS: usize = 200_000;

/// Thread-safe cooperative cancellation token for an active run.
#[derive(Clone, Default)]
pub struct CancellationToken {
    cancelled: Arc<AtomicBool>,
    notify: Arc<Notify>,
}

impl CancellationToken {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn cancel(&self) {
        self.cancelled.store(true, Ordering::SeqCst);
        self.notify.notify_waiters();
        self.notify.notify_one();
    }

    pub fn is_cancelled(&self) -> bool {
        self.cancelled.load(Ordering::SeqCst)
    }

    pub async fn cancelled(&self) {
        if !self.is_cancelled() {
            self.notify.notified().await;
        }
    }
}

/// Guard that unregisters a run's cancellation token when dropped.
struct RunRegistrationGuard {
    registry: ActiveRunRegistry,
    run_id: String,
}

impl Drop for RunRegistrationGuard {
    fn drop(&mut self) {
        self.registry.unregister(&self.run_id);
    }
}

/// Registry of active run cancellation tokens so HTTP/CLI endpoints can cancel an in-flight run.
#[derive(Clone, Default)]
pub struct ActiveRunRegistry {
    tokens: Arc<RwLock<HashMap<String, CancellationToken>>>,
    steering: Arc<RwLock<HashMap<String, Vec<String>>>>,
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
        if let Ok(mut guard) = self.steering.write() {
            guard.remove(run_id);
        }
    }

    /// Queue a mid-run steering message for `run_id`.
    pub fn steer(&self, run_id: &str, message: String) {
        if let Ok(mut guard) = self.steering.write() {
            guard.entry(run_id.to_string()).or_default().push(message);
        }
    }

    /// Drain all queued steering messages for `run_id`.
    pub fn drain_steering(&self, run_id: &str) -> Vec<String> {
        if let Ok(mut guard) = self.steering.write() {
            guard.remove(run_id).unwrap_or_default()
        } else {
            Vec::new()
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

/// Composite executor that routes tool calls to a built-in executor or an MCP
/// executor based on the tool name. Built-in tools take precedence for the known
/// `gaggle.*` namespaces; unknown names are forwarded to the MCP executor.
pub struct CompositeToolExecutor {
    builtin: Arc<dyn ToolExecutor>,
    mcp: Option<Arc<dyn ToolExecutor>>,
    builtin_prefixes: HashSet<String>,
}

impl CompositeToolExecutor {
    pub fn new(builtin: Arc<dyn ToolExecutor>, mcp: Option<Arc<dyn ToolExecutor>>) -> Self {
        let builtin_prefixes = [
            "fs.",
            "git.",
            "shell.",
            "memory.",
            "web.",
            "browser.",
            "terminal.",
        ]
        .iter()
        .map(|s| s.to_string())
        .collect();
        Self {
            builtin,
            mcp,
            builtin_prefixes,
        }
    }

    fn is_builtin(&self, tool_name: &str) -> bool {
        self.builtin_prefixes
            .iter()
            .any(|prefix| tool_name.starts_with(prefix))
    }
}

#[async_trait]
impl ToolExecutor for CompositeToolExecutor {
    async fn execute_tool(
        &self,
        run_id: &str,
        chat: &Chat,
        project_root: &str,
        tool_name: &str,
        args_json: &str,
    ) -> Result<String, String> {
        if self.is_builtin(tool_name) {
            self.builtin
                .execute_tool(run_id, chat, project_root, tool_name, args_json)
                .await
        } else if let Some(mcp) = &self.mcp {
            mcp.execute_tool(run_id, chat, project_root, tool_name, args_json)
                .await
        } else {
            Err(format!("No executor available for tool '{tool_name}'"))
        }
    }
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
    pub active_runs: ActiveRunRegistry,
}

impl AgentRuntime {
    pub fn new(pool: SqlitePool, max_steps: usize) -> Self {
        Self {
            pool,
            max_steps,
            active_runs: ActiveRunRegistry::new(),
        }
    }

    pub fn new_with_active_runs(
        pool: SqlitePool,
        max_steps: usize,
        active_runs: ActiveRunRegistry,
    ) -> Self {
        Self {
            pool,
            max_steps,
            active_runs,
        }
    }

    /// Attach/replace the [`ActiveRunRegistry`] used for cancellation.
    pub fn with_active_runs(mut self, registry: ActiveRunRegistry) -> Self {
        self.active_runs = registry;
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
        self.run_turn_with_precreated(
            chat_id,
            user_prompt,
            model_key,
            provider,
            tool_executor,
            cancel_token,
            None,
            None,
        )
        .await
    }

    #[allow(clippy::too_many_arguments)]
    pub async fn run_turn_with_precreated(
        &self,
        chat_id: &str,
        user_prompt: &str,
        model_key: &str,
        provider: &dyn ModelProviderAdapter,
        tool_executor: &dyn ToolExecutor,
        cancel_token: &CancellationToken,
        precreated_run_id: Option<String>,
        precreated_user_msg_id: Option<String>,
    ) -> Result<AgentLoopOutcome> {
        let span = CorrelationIds::for_chat(chat_id).span("agent.run");
        self.run_turn_inner(
            chat_id,
            user_prompt,
            model_key,
            provider,
            tool_executor,
            cancel_token,
            precreated_run_id,
            precreated_user_msg_id,
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
        precreated_run_id: Option<String>,
        precreated_user_msg_id: Option<String>,
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

        // 2. Persist user message and create durable run (or reuse precreated)
        let user_msg_id: String = if let Some(uid) = precreated_user_msg_id.clone() {
            uid
        } else {
            let m = chat_repo
                .append_message(chat_id, None, "user", user_prompt)
                .await?;
            m.id
        };
        let run = if let Some(rid) = precreated_run_id.clone() {
            run_repo
                .get_run_by_id(&rid)
                .await?
                .ok_or_else(|| anyhow::anyhow!("precreated run not found"))?
        } else {
            run_repo
                .create(chat_id, Some(&user_msg_id), chat.model_id.as_deref())
                .await?
        };
        self.active_runs
            .register_token(&run.id, cancel_token.clone());
        let _run_registration = RunRegistrationGuard {
            registry: self.active_runs.clone(),
            run_id: run.id.clone(),
        };
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
                    "trigger_message_id": user_msg_id,
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

            // Apply any mid-run steering messages queued by the user.
            for steering_message in self.active_runs.drain_steering(&run.id) {
                if !steering_message.trim().is_empty() {
                    conversation.push(ProviderMessage {
                        role: "user".to_string(),
                        content: steering_message,
                        ..Default::default()
                    });
                }
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
            let inference = InferenceOperation::run(provider, req, cancel_token, {
                let run_repo = run_repo.clone();
                let run_id = run.id.clone();
                move |delta| {
                    let run_repo = run_repo.clone();
                    let run_id = run_id.clone();
                    async move {
                        run_repo
                            .emit_event(
                                &run_id,
                                "message.delta",
                                &serde_json::json!({
                                    "step": step_idx,
                                    "text": delta
                                })
                                .to_string(),
                            )
                            .await?;
                        Ok(())
                    }
                }
            })
            .await;

            let (completion, used_native_stream) = match inference {
                Ok(output) => (output.completion, output.used_native_stream),
                Err(InferenceOperationError::Cancelled) => {
                    return self.finish_cancelled(&run_repo, &run.id).await;
                }
                Err(InferenceOperationError::Persistence(err)) => return Err(err),
                Err(InferenceOperationError::Provider(err)) => {
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

            // Record the assistant turn that requested these tool calls so providers can pair
            // tool results with their originating tool_use blocks (required by Anthropic).
            conversation.push(ProviderMessage {
                role: "assistant".to_string(),
                content: completion.text.clone(),
                tool_calls: completion.tool_calls.clone(),
                ..Default::default()
            });

            let tool_outcome = ToolCallingOperation {
                runtime: self,
                run_repo: &run_repo,
                ext_repo: &ext_repo,
                permission_engine: &perm_engine,
            }
            .run(
                &run,
                step_idx,
                &chat,
                &project,
                &completion.tool_calls,
                tool_executor,
                cancel_token,
                &mut conversation,
                &correlation,
            )
            .await?;
            match tool_outcome {
                ToolCallingOutcome::Continue => {}
                ToolCallingOutcome::Cancelled => {
                    return self.finish_cancelled(&run_repo, &run.id).await;
                }
                ToolCallingOutcome::WaitingApproval {
                    run,
                    pending_approval_id,
                    tool_call_id,
                } => {
                    return Ok(AgentLoopOutcome::WaitingApproval {
                        run,
                        pending_approval_id,
                        tool_call_id,
                    });
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
                    "tool_name": proposed.tool_name,
                    "label": proposed.tool_name,
                    "approval_kind": crate::permission_engine::PermissionEngine::infer_approval_kind(&proposed.tool_name),
                    "args_summary": summarize_tool_arguments(&proposed.tool_name, &proposed.arguments_json)
                })
                .to_string(),
            )
            .await?;

        let hook_manager = HookManager::new();
        let pre_hook_payload = serde_json::from_str::<serde_json::Value>(&proposed.arguments_json)
            .unwrap_or_else(|_| serde_json::json!({}));
        let pre_hook = hook_manager.emit(&HookContext {
            event: HookEvent::PreToolUse,
            run_id: Some(run_id.to_string()),
            chat_id: Some(chat.id.clone()),
            tool_name: Some(proposed.tool_name.clone()),
            tool_call_id: Some(tc.id.clone()),
            payload: pre_hook_payload,
        });
        if pre_hook.decision == HookDecision::Deny {
            let msg = pre_hook
                .message
                .unwrap_or_else(|| "Blocked by hook".to_string());
            let err_json = serde_json::json!({ "error": msg }).to_string();
            run_repo
                .update_tool_call_result(&tc.id, "error", Some(&err_json), 0)
                .await?;
            run_repo
                .emit_event(
                    run_id,
                    "tool.failed",
                    &serde_json::json!({
                        "tool_call_id": tc.id,
                        "tool_name": proposed.tool_name,
                        "error": msg
                    })
                    .to_string(),
                )
                .await?;
            return Ok(err_json);
        }

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
                let result_bytes = result_json.len();
                // Large-response spill: persist oversized tool output to a temp file
                // and replace the in-context result with a reference message.
                let stored_result = if result_bytes > MAX_TOOL_RESPONSE_CHARS {
                    let spill_path =
                        std::env::temp_dir().join(format!("gaggle-tool-result-{}.json", tc.id));
                    match std::fs::write(&spill_path, &result_json) {
                        Ok(()) => serde_json::json!({
                            "spilled": true,
                            "path": spill_path.display().to_string(),
                            "original_bytes": result_bytes,
                            "message": "Tool result was too large and was saved to the referenced file."
                        })
                        .to_string(),
                        Err(err) => serde_json::json!({
                            "spilled": false,
                            "error": format!("failed to spill large tool result: {err}"),
                            "truncated_preview": "Tool result too large to include inline."
                        })
                        .to_string(),
                    }
                } else {
                    result_json
                };
                tracing::info!(
                    target: "gaggle.agent",
                    run_id = %run_id,
                    tool_call_id = %tc.id,
                    tool_name = %proposed.tool_name,
                    step = step_index,
                    duration_ms,
                    result_bytes,
                    status = "ok",
                    "Tool call completed"
                );
                run_repo
                    .update_tool_call_result(&tc.id, "ok", Some(&stored_result), duration_ms)
                    .await?;
                run_repo
                    .emit_event(
                        run_id,
                        "tool.completed",
                        &serde_json::json!({
                            "tool_call_id": tc.id,
                            "tool_name": proposed.tool_name,
                            "label": proposed.tool_name,
                            "approval_kind": crate::permission_engine::PermissionEngine::infer_approval_kind(&proposed.tool_name),
                            "duration_ms": duration_ms,
                            "result_bytes": result_bytes,
                            "spilled": stored_result.contains("\"spilled\":true") || stored_result.contains("\"spilled\": true")
                        })
                        .to_string(),
                    )
                    .await?;
                Ok(stored_result)
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

enum ToolCallingOutcome {
    Continue,
    WaitingApproval {
        run: Run,
        pending_approval_id: String,
        tool_call_id: String,
    },
    Cancelled,
}

struct ToolCallingOperation<'a> {
    runtime: &'a AgentRuntime,
    run_repo: &'a RunRepository,
    ext_repo: &'a ExtensionRepository,
    permission_engine: &'a PermissionEngine,
}

impl ToolCallingOperation<'_> {
    #[allow(clippy::too_many_arguments)]
    async fn run(
        &self,
        run: &Run,
        step_idx: usize,
        chat: &Chat,
        project: &Project,
        proposed_calls: &[ProposedToolCall],
        tool_executor: &dyn ToolExecutor,
        cancel_token: &CancellationToken,
        conversation: &mut Vec<ProviderMessage>,
        correlation: &CorrelationIds,
    ) -> Result<ToolCallingOutcome> {
        for proposed in proposed_calls {
            if cancel_token.is_cancelled() {
                return Ok(ToolCallingOutcome::Cancelled);
            }

            let Some(ext_tool) = self
                .ext_repo
                .find_enabled_tool_by_name(&proposed.tool_name)
                .await?
            else {
                let failure = ActionableError::new(
                    FailureCategory::Tool,
                    "agent.tool_lookup",
                    format!("Unknown or disabled tool '{}'", proposed.tool_name),
                )
                .with_data_preservation(DataPreservation::Preserved)
                .with_correlation(correlation);
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
                AgentRuntime::extract_shell_command(&proposed.tool_name, &proposed.arguments_json);
            let ctx = ToolCallContext {
                project_id: Some(project.id.clone()),
                permission_mode: chat.permission_mode.clone(),
                extension_tool: ext_tool,
                args_json: proposed.arguments_json.clone(),
                shell_command: shell_cmd.clone(),
            };

            match self.permission_engine.evaluate(&ctx).await? {
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
                    let tool_call = self
                        .run_repo
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
                    self.run_repo
                        .update_tool_call_result(&tool_call.id, "denied", Some(&deny_json), 0)
                        .await?;
                    self.run_repo
                        .emit_event(
                            &run.id,
                            "tool.denied",
                            &serde_json::json!({
                                "tool_call_id": tool_call.id,
                                "tool_name": proposed.tool_name,
                                "reason": reason
                            })
                            .to_string(),
                        )
                        .await?;
                    conversation.push(ProviderMessage {
                        role: "tool".to_string(),
                        content: deny_json,
                        tool_call_id: Some(proposed.id.clone()),
                        ..Default::default()
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
                    let tool_call = self
                        .run_repo
                        .record_tool_call(
                            &run.id,
                            step_idx as i64,
                            "native",
                            &proposed.tool_name,
                            &proposed.arguments_json,
                            "awaiting_approval",
                        )
                        .await?;
                    let approval = self
                        .run_repo
                        .create_approval(
                            &tool_call.id,
                            &kind,
                            &summary,
                            shell_cmd.as_deref(),
                            Some(&project.root_path),
                        )
                        .await?;
                    self.run_repo
                        .save_checkpoint(
                            &run.id,
                            step_idx as i64,
                            &serde_json::json!({
                                "waiting_approval_id": approval.id,
                                "tool_call_id": tool_call.id
                            })
                            .to_string(),
                        )
                        .await?;
                    self.run_repo
                        .emit_event(
                            &run.id,
                            "approval.requested",
                            &serde_json::json!({
                                "approval_id": approval.id,
                                "tool_call_id": tool_call.id,
                                "tool_name": proposed.tool_name,
                                "summary": summary,
                                "command": shell_cmd,
                                "cwd": project.root_path
                            })
                            .to_string(),
                        )
                        .await?;
                    let waiting_run = self
                        .run_repo
                        .update_run_status(&run.id, "waiting_approval", None)
                        .await?;
                    return Ok(ToolCallingOutcome::WaitingApproval {
                        run: waiting_run,
                        pending_approval_id: approval.id,
                        tool_call_id: tool_call.id,
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
                        .runtime
                        .execute_allowed_tool(
                            self.run_repo,
                            &run.id,
                            step_idx as i64,
                            chat,
                            &project.root_path,
                            proposed,
                            tool_executor,
                        )
                        .await?;
                    conversation.push(ProviderMessage {
                        role: "tool".to_string(),
                        content: tool_out,
                        tool_call_id: Some(proposed.id.clone()),
                        ..Default::default()
                    });
                }
            }
        }

        Ok(ToolCallingOutcome::Continue)
    }
}
