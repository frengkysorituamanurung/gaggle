use crate::{
    PermissionDecision, PermissionEngine, ToolCallContext,
    provider::{
        ModelCompletionRequest, ModelProviderAdapter, ProposedToolCall, ProviderMessage,
        ToolDefinition,
    },
};
use anyhow::{Context, Result, bail};
use async_trait::async_trait;
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
}

impl AgentRuntime {
    pub fn new(pool: SqlitePool, max_steps: usize) -> Self {
        Self { pool, max_steps }
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

        // 1. Persist user message and create durable run
        let user_msg = chat_repo
            .append_message(chat_id, None, "user", user_prompt)
            .await?;
        let run = run_repo
            .create(chat_id, Some(&user_msg.id), chat.model_id.as_deref())
            .await?;

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

        let mut conversation = vec![ProviderMessage {
            role: "user".to_string(),
            content: user_prompt.to_string(),
        }];

        for step_idx in 1..=self.max_steps {
            if cancel_token.is_cancelled() {
                return self.finish_cancelled(&run_repo, &run.id).await;
            }

            let step_start = Instant::now();
            let req = ModelCompletionRequest {
                model_key: model_key.to_string(),
                messages: conversation.clone(),
                tools: Vec::<ToolDefinition>::new(),
                max_output_tokens: Some(1024),
            };

            let completion = match provider.complete(&req).await {
                Ok(resp) => resp,
                Err(err) => {
                    let err_msg = err.to_string();
                    run_repo
                        .emit_event(
                            &run.id,
                            "run.failed",
                            &serde_json::json!({ "step": step_idx, "error": err_msg }).to_string(),
                        )
                        .await?;
                    let failed_run = run_repo
                        .update_run_status(&run.id, "failed", Some(&err_msg))
                        .await?;
                    return Ok(AgentLoopOutcome::Failed {
                        run: failed_run,
                        error: err_msg,
                    });
                }
            };

            let latency_ms = step_start.elapsed().as_millis() as i64;
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

            if !completion.text.is_empty() {
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
                    bail!("Unknown or disabled tool '{}'", proposed.tool_name);
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
                                    "summary": summary
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
        let budget_err = format!("Maximum agent step budget ({}) exhausted", self.max_steps);
        run_repo
            .emit_event(
                &run.id,
                "run.failed",
                &serde_json::json!({ "error": budget_err }).to_string(),
            )
            .await?;
        let failed_run = run_repo
            .update_run_status(&run.id, "failed", Some(&budget_err))
            .await?;
        Ok(AgentLoopOutcome::Failed {
            run: failed_run,
            error: budget_err,
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
        let exec_res = tool_executor
            .execute_tool(
                run_id,
                chat,
                project_root,
                &proposed.tool_name,
                &proposed.arguments_json,
            )
            .await;
        let duration_ms = started.elapsed().as_millis() as i64;

        match exec_res {
            Ok(result_json) => {
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
