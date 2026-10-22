use anyhow::Result;
use async_trait::async_trait;
use gaggle_core::{
    AgentLoopOutcome, AgentRuntime, CancellationToken, ModelCompletionRequest,
    ModelCompletionResponse, ModelProviderAdapter, ProposedToolCall, ProviderError, TokenUsage,
};
use gaggle_db::{
    connect_in_memory,
    connection::new_id,
    repositories::{chats::ChatRepository, projects::ProjectRepository, runs::RunRepository},
};
use gaggle_tools::{BuiltinToolExecutor, register_builtin_extensions};
use std::sync::atomic::{AtomicUsize, Ordering};

/// Multi-step mock provider that proposes a tool call on step 1 and returns a final answer on step 2.
struct MultiStepProvider {
    call_count: AtomicUsize,
    first_tool_call: ProposedToolCall,
}

#[async_trait]
impl ModelProviderAdapter for MultiStepProvider {
    fn provider_name(&self) -> &str {
        "multi-step-mock"
    }

    async fn complete(
        &self,
        _req: &ModelCompletionRequest,
    ) -> Result<ModelCompletionResponse, ProviderError> {
        let step = self.call_count.fetch_add(1, Ordering::SeqCst);
        if step == 0 {
            Ok(ModelCompletionResponse {
                text: "Reading file first...".to_string(),
                tool_calls: vec![self.first_tool_call.clone()],
                usage: TokenUsage {
                    input_tokens: 20,
                    output_tokens: 10,
                    cached_tokens: 0,
                },
            })
        } else {
            Ok(ModelCompletionResponse {
                text: "Finished reading the file and completed the task.".to_string(),
                tool_calls: vec![],
                usage: TokenUsage {
                    input_tokens: 35,
                    output_tokens: 15,
                    cached_tokens: 0,
                },
            })
        }
    }
}

/// Provider that always proposes a read-only tool call on every turn to test max-step budget exhaustion.
struct InfiniteToolLoopProvider {
    tool_call: ProposedToolCall,
}

#[async_trait]
impl ModelProviderAdapter for InfiniteToolLoopProvider {
    fn provider_name(&self) -> &str {
        "infinite-loop-mock"
    }

    async fn complete(
        &self,
        _req: &ModelCompletionRequest,
    ) -> Result<ModelCompletionResponse, ProviderError> {
        Ok(ModelCompletionResponse {
            text: String::new(),
            tool_calls: vec![self.tool_call.clone()],
            usage: TokenUsage::default(),
        })
    }
}

#[tokio::test]
async fn test_agent_loop_read_only_tool_execution_and_sse_event_replay() -> Result<()> {
    let temp_root = std::env::temp_dir().join(format!("gaggle_m5_loop_{}", new_id()));
    std::fs::create_dir_all(&temp_root)?;
    std::fs::write(temp_root.join("hello.txt"), "Hello from workspace!\n")?;

    let pool = connect_in_memory().await?;
    register_builtin_extensions(&pool).await?;

    let proj_repo = ProjectRepository::new(pool.clone());
    let chat_repo = ChatRepository::new(pool.clone());
    let run_repo = RunRepository::new(pool.clone());

    let proj = proj_repo
        .create("m5-proj", &temp_root.to_string_lossy(), "git")
        .await?;
    let chat = chat_repo
        .create(&proj.id, "Agent Loop Chat", "user", "local", "ask")
        .await?;

    let provider = MultiStepProvider {
        call_count: AtomicUsize::new(0),
        first_tool_call: ProposedToolCall {
            id: "call_1".to_string(),
            tool_name: "fs.read".to_string(),
            arguments_json: r#"{"path":"hello.txt"}"#.to_string(),
        },
    };

    let runtime = AgentRuntime::new(pool.clone(), 5);
    let executor = BuiltinToolExecutor::new(pool.clone());
    let cancel = CancellationToken::new();

    // `fs.read` is annotated `is_read_only = 1`, so even in `ask` mode it executes automatically on step 1,
    // feeds the result back to the provider, and completes on step 2.
    let outcome = runtime
        .run_turn(
            &chat.id,
            "Read hello.txt",
            "mock-model",
            &provider,
            &executor,
            &cancel,
        )
        .await?;

    let run = match outcome {
        AgentLoopOutcome::Completed {
            run,
            final_text,
            steps_executed,
        } => {
            assert_eq!(steps_executed, 2);
            assert_eq!(
                final_text,
                "Finished reading the file and completed the task."
            );
            assert_eq!(run.status, "completed");
            run
        }
        other => panic!("Expected Completed outcome, got {other:?}"),
    };

    // Verify ordered run_events and SSE replay cursor (`after_seq`)
    let all_events = run_repo.list_run_events(&run.id, 0).await?;
    assert!(all_events.len() >= 5);
    assert_eq!(all_events[0].seq, 1);
    assert_eq!(all_events[0].kind, "run.started");
    assert_eq!(all_events.last().unwrap().kind, "run.completed");

    let replayed_tail = run_repo.list_run_events(&run.id, 2).await?;
    assert_eq!(replayed_tail[0].seq, 3);

    // Verify accumulated token usage on chat
    let updated_chat = chat_repo.get_by_id(&chat.id).await?.unwrap();
    assert_eq!(updated_chat.total_input_tokens, 55);
    assert_eq!(updated_chat.total_output_tokens, 25);

    let _ = std::fs::remove_dir_all(&temp_root);
    Ok(())
}

#[tokio::test]
async fn test_agent_loop_pauses_for_approval_on_mutating_tool() -> Result<()> {
    let temp_root = std::env::temp_dir().join(format!("gaggle_m5_appr_{}", new_id()));
    std::fs::create_dir_all(&temp_root)?;

    let pool = connect_in_memory().await?;
    register_builtin_extensions(&pool).await?;

    let proj_repo = ProjectRepository::new(pool.clone());
    let chat_repo = ChatRepository::new(pool.clone());

    let proj = proj_repo
        .create("m5-appr", &temp_root.to_string_lossy(), "git")
        .await?;
    let chat = chat_repo
        .create(&proj.id, "Approval Loop Chat", "user", "local", "ask")
        .await?;

    let provider = MultiStepProvider {
        call_count: AtomicUsize::new(0),
        first_tool_call: ProposedToolCall {
            id: "call_shell_1".to_string(),
            tool_name: "shell.exec".to_string(),
            arguments_json: r#"{"command":"echo test"}"#.to_string(),
        },
    };

    let runtime = AgentRuntime::new(pool.clone(), 5);
    let executor = BuiltinToolExecutor::new(pool.clone());
    let cancel = CancellationToken::new();

    // `shell.exec` is non-read-only, so in `ask` mode the loop must pause in `WaitingApproval`
    let outcome = runtime
        .run_turn(
            &chat.id,
            "Run echo",
            "mock-model",
            &provider,
            &executor,
            &cancel,
        )
        .await?;

    match outcome {
        AgentLoopOutcome::WaitingApproval {
            run,
            pending_approval_id,
            tool_call_id,
        } => {
            assert_eq!(run.status, "waiting_approval");
            assert!(!pending_approval_id.is_empty());
            assert!(!tool_call_id.is_empty());
        }
        other => panic!("Expected WaitingApproval, got {other:?}"),
    }

    let _ = std::fs::remove_dir_all(&temp_root);
    Ok(())
}

#[tokio::test]
async fn test_agent_loop_cancellation_and_step_budget_exhaustion() -> Result<()> {
    let temp_root = std::env::temp_dir().join(format!("gaggle_m5_budget_{}", new_id()));
    std::fs::create_dir_all(&temp_root)?;
    std::fs::write(temp_root.join("a.txt"), "content\n")?;

    let pool = connect_in_memory().await?;
    register_builtin_extensions(&pool).await?;

    let proj_repo = ProjectRepository::new(pool.clone());
    let chat_repo = ChatRepository::new(pool.clone());

    let proj = proj_repo
        .create("m5-budget", &temp_root.to_string_lossy(), "git")
        .await?;
    let chat = chat_repo
        .create(&proj.id, "Budget Chat", "user", "local", "ask")
        .await?;

    let infinite_provider = InfiniteToolLoopProvider {
        tool_call: ProposedToolCall {
            id: "call_inf".to_string(),
            tool_name: "fs.read".to_string(),
            arguments_json: r#"{"path":"a.txt"}"#.to_string(),
        },
    };

    let executor = BuiltinToolExecutor::new(pool.clone());

    // 1. Pre-cancelled token stops immediately with `Cancelled` outcome
    let cancel_now = CancellationToken::new();
    cancel_now.cancel();
    let runtime = AgentRuntime::new(pool.clone(), 3);
    let cancelled_outcome = runtime
        .run_turn(
            &chat.id,
            "Cancel immediately",
            "mock-model",
            &infinite_provider,
            &executor,
            &cancel_now,
        )
        .await?;
    assert!(matches!(
        cancelled_outcome,
        AgentLoopOutcome::Cancelled { .. }
    ));

    // 2. Infinite tool loop stops after max_steps = 2 with `Failed` outcome
    let runtime_bounded = AgentRuntime::new(pool.clone(), 2);
    let active_token = CancellationToken::new();
    let budget_outcome = runtime_bounded
        .run_turn(
            &chat.id,
            "Trigger max steps",
            "mock-model",
            &infinite_provider,
            &executor,
            &active_token,
        )
        .await?;
    match budget_outcome {
        AgentLoopOutcome::Failed { run, error } => {
            assert_eq!(run.status, "failed");
            assert!(error.contains("Maximum agent step budget (2) exhausted"));
        }
        other => panic!("Expected Failed due to budget exhaustion, got {other:?}"),
    }

    let _ = std::fs::remove_dir_all(&temp_root);
    Ok(())
}
