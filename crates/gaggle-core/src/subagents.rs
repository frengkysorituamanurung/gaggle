use crate::agent_loop::{AgentLoopOutcome, AgentRuntime, CancellationToken};
use anyhow::{Context, Result, bail};
use gaggle_db::{
    models::{Chat, Run},
    repositories::{chats::ChatRepository, projects::ProjectRepository},
};
use sqlx::SqlitePool;
use std::sync::Arc;
use tokio::sync::{Mutex, Semaphore};

/// Default bounded concurrency for subagent execution.
pub const DEFAULT_MAX_PARALLEL_SUBAGENTS: usize = 4;

/// Default recursion depth limit (no nested subagents beyond depth 1).
pub const DEFAULT_SUBAGENT_DEPTH: usize = 1;

/// Default agent step budget for a subagent run.
pub const DEFAULT_SUBAGENT_MAX_STEPS: usize = 8;

#[derive(Debug, Clone)]
pub struct SubagentConfig {
    pub max_parallel_subagents: usize,
    pub max_depth: usize,
    pub max_steps: usize,
}

impl Default for SubagentConfig {
    fn default() -> Self {
        Self {
            max_parallel_subagents: DEFAULT_MAX_PARALLEL_SUBAGENTS,
            max_depth: DEFAULT_SUBAGENT_DEPTH,
            max_steps: DEFAULT_SUBAGENT_MAX_STEPS,
        }
    }
}

#[derive(Debug, Clone, Default)]
pub struct SubagentSpawnRequest {
    pub parent_chat_id: String,
    pub parent_tool_call_id: Option<String>,
    pub project_id: Option<String>,
    pub title: String,
    pub prompt: String,
    pub permission_mode: Option<String>,
    pub mode: Option<String>,
    pub model_key: Option<String>,
    pub sequential: bool,
    pub sequential_when_repeated: bool,
    pub sequential_key: Option<String>,
}

#[derive(Debug, Clone)]
pub struct SubagentOutcome {
    pub child_chat: Chat,
    pub run: Option<Run>,
    pub final_text: String,
    pub status: String,
    pub summary: String,
}

impl SubagentOutcome {
    /// Subagent output is untrusted tool data; callers must treat `summary` as such.
    pub fn untrusted_summary(&self) -> String {
        format!(
            "<SUBAGENT_RESULT untrusted=\"true\">{}</SUBAGENT_RESULT>",
            self.summary
        )
    }
}

/// Manages bounded, isolated subagent chats with inherited permissions.
pub struct SubagentManager {
    pool: SqlitePool,
    config: SubagentConfig,
    semaphore: Arc<Semaphore>,
    sequential_lock: Arc<Mutex<()>>,
    max_steps: usize,
}

impl SubagentManager {
    pub fn new(pool: SqlitePool, max_parallel_subagents: usize) -> Self {
        Self::with_config(
            pool,
            SubagentConfig {
                max_parallel_subagents: max_parallel_subagents.max(1),
                ..SubagentConfig::default()
            },
        )
    }

    pub fn with_config(pool: SqlitePool, config: SubagentConfig) -> Self {
        let max_parallel = config.max_parallel_subagents.max(1);
        Self {
            pool: pool.clone(),
            config: SubagentConfig {
                max_parallel_subagents: max_parallel,
                ..config
            },
            semaphore: Arc::new(Semaphore::new(max_parallel)),
            sequential_lock: Arc::new(Mutex::new(())),
            max_steps: config.max_steps,
        }
    }

    pub fn max_parallel_subagents(&self) -> usize {
        self.config.max_parallel_subagents
    }

    pub fn max_depth(&self) -> usize {
        self.config.max_depth
    }

    /// Computes the nesting depth of a chat by walking `parent_chat_id`.
    pub async fn chat_depth(&self, chat_id: &str) -> Result<usize> {
        let chat_repo = ChatRepository::new(self.pool.clone());
        let mut depth = 0usize;
        let mut current = chat_repo.get_by_id(chat_id).await?;
        while let Some(chat) = current {
            if let Some(parent) = chat.parent_chat_id {
                depth += 1;
                current = chat_repo.get_by_id(&parent).await?;
            } else {
                break;
            }
        }
        Ok(depth)
    }

    /// Spawns a subagent child chat without running it.
    pub async fn spawn_child_chat(
        &self,
        parent_chat_id: &str,
        request: &SubagentSpawnRequest,
    ) -> Result<Chat> {
        let parent_depth = self.chat_depth(parent_chat_id).await?;
        if parent_depth >= self.config.max_depth {
            bail!(
                "subagent depth limit exceeded: parent depth {} >= limit {}",
                parent_depth,
                self.config.max_depth
            );
        }

        let chat_repo = ChatRepository::new(self.pool.clone());
        let proj_repo = ProjectRepository::new(self.pool.clone());
        let parent_chat = chat_repo
            .get_by_id(parent_chat_id)
            .await?
            .with_context(|| format!("Parent chat '{parent_chat_id}' not found"))?;

        let project_id = request
            .project_id
            .clone()
            .unwrap_or_else(|| parent_chat.project_id.clone());

        // Validate project exists.
        proj_repo
            .get_by_id(&project_id)
            .await?
            .with_context(|| format!("Project '{project_id}' not found"))?;

        let permission_mode = request
            .permission_mode
            .clone()
            .unwrap_or_else(|| parent_chat.permission_mode.clone());

        let mode = request
            .mode
            .clone()
            .unwrap_or_else(|| parent_chat.mode.clone());

        let title = if request.title.trim().is_empty() {
            format!("Subagent of {}", parent_chat.title)
        } else {
            request.title.clone()
        };

        chat_repo
            .create_subagent_chat(
                &project_id,
                &title,
                &mode,
                &permission_mode,
                parent_chat_id,
                request.parent_tool_call_id.as_deref(),
            )
            .await
    }

    /// Runs an already-created subagent child chat to completion, returning a summary.
    #[allow(clippy::too_many_arguments)]
    pub async fn run_child_chat(
        &self,
        child_chat_id: &str,
        prompt: &str,
        model_key: &str,
        provider: &dyn crate::ModelProviderAdapter,
        tool_executor: &dyn crate::ToolExecutor,
        cancel_token: &CancellationToken,
    ) -> Result<SubagentOutcome> {
        let chat_repo = ChatRepository::new(self.pool.clone());
        let child_chat = chat_repo
            .get_by_id(child_chat_id)
            .await?
            .with_context(|| format!("Child chat '{child_chat_id}' not found"))?;

        let runtime = AgentRuntime::new(self.pool.clone(), self.max_steps);
        let outcome = runtime
            .run_turn(
                child_chat_id,
                prompt,
                model_key,
                provider,
                tool_executor,
                cancel_token,
            )
            .await?;

        let (status, run, final_text) = match outcome {
            AgentLoopOutcome::Completed {
                run, final_text, ..
            } => ("completed".to_string(), Some(run), final_text),
            AgentLoopOutcome::Cancelled { run } => {
                ("cancelled".to_string(), Some(run), String::new())
            }
            AgentLoopOutcome::Failed { run, error } => ("failed".to_string(), Some(run), error),
            AgentLoopOutcome::WaitingApproval {
                run,
                pending_approval_id,
                ..
            } => (
                "waiting_approval".to_string(),
                Some(run),
                format!("awaiting approval {pending_approval_id}"),
            ),
        };

        let summary = final_text.clone();
        Ok(SubagentOutcome {
            child_chat,
            run,
            final_text,
            status,
            summary,
        })
    }

    /// Spawns (creates) and runs a subagent, bounding concurrency.
    #[allow(clippy::too_many_arguments)]
    pub async fn spawn_and_run(
        &self,
        parent_chat_id: &str,
        provider: &dyn crate::ModelProviderAdapter,
        tool_executor: &dyn crate::ToolExecutor,
        cancel_token: &CancellationToken,
        request: SubagentSpawnRequest,
    ) -> Result<SubagentOutcome> {
        // Depth check before acquiring locks.
        let parent_depth = self.chat_depth(parent_chat_id).await?;
        if parent_depth >= self.config.max_depth {
            bail!(
                "subagent depth limit exceeded: parent depth {} >= limit {}",
                parent_depth,
                self.config.max_depth
            );
        }

        let sequential = request.sequential || request.sequential_when_repeated;

        // Acquire sequential lock first (if needed) to serialize repeated calls.
        let _seq_guard = if sequential {
            Some(self.sequential_lock.lock().await)
        } else {
            None
        };

        // Acquire bounded concurrency permit.
        let _permit = self
            .semaphore
            .acquire()
            .await
            .map_err(|e| anyhow::anyhow!("semaphore closed: {e}"))?;

        let child_chat = self.spawn_child_chat(parent_chat_id, &request).await?;

        let model_key = request
            .model_key
            .clone()
            .unwrap_or_else(|| "local-mock".to_string());

        let outcome = self
            .run_child_chat(
                &child_chat.id,
                &request.prompt,
                &model_key,
                provider,
                tool_executor,
                cancel_token,
            )
            .await?;

        // Propagate cancellation: if child cancelled and token shared, parent already cancelled.
        if outcome.status == "cancelled" {
            cancel_token.cancel();
        }

        Ok(outcome)
    }

    // Aliases for compatibility with various call sites.
    #[allow(clippy::too_many_arguments)]
    pub async fn spawn(
        &self,
        parent_chat_id: &str,
        provider: &dyn crate::ModelProviderAdapter,
        tool_executor: &dyn crate::ToolExecutor,
        cancel_token: &CancellationToken,
        request: SubagentSpawnRequest,
    ) -> Result<SubagentOutcome> {
        self.spawn_and_run(
            parent_chat_id,
            provider,
            tool_executor,
            cancel_token,
            request,
        )
        .await
    }

    #[allow(clippy::too_many_arguments)]
    pub async fn spawn_subagent(
        &self,
        parent_chat_id: &str,
        provider: &dyn crate::ModelProviderAdapter,
        tool_executor: &dyn crate::ToolExecutor,
        cancel_token: &CancellationToken,
        request: SubagentSpawnRequest,
    ) -> Result<SubagentOutcome> {
        self.spawn_and_run(
            parent_chat_id,
            provider,
            tool_executor,
            cancel_token,
            request,
        )
        .await
    }

    #[allow(clippy::too_many_arguments)]
    pub async fn delegate(
        &self,
        parent_chat_id: &str,
        provider: &dyn crate::ModelProviderAdapter,
        tool_executor: &dyn crate::ToolExecutor,
        cancel_token: &CancellationToken,
        request: SubagentSpawnRequest,
    ) -> Result<SubagentOutcome> {
        self.spawn_and_run(
            parent_chat_id,
            provider,
            tool_executor,
            cancel_token,
            request,
        )
        .await
    }

    #[allow(clippy::too_many_arguments)]
    pub async fn spawn_and_wait(
        &self,
        parent_chat_id: &str,
        provider: &dyn crate::ModelProviderAdapter,
        tool_executor: &dyn crate::ToolExecutor,
        cancel_token: &CancellationToken,
        request: SubagentSpawnRequest,
    ) -> Result<SubagentOutcome> {
        self.spawn_and_run(
            parent_chat_id,
            provider,
            tool_executor,
            cancel_token,
            request,
        )
        .await
    }
}
