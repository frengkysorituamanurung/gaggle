use anyhow::Result;
use async_trait::async_trait;
use gaggle_core::{
    AgentLoopOutcome, AgentRuntime, CancellationToken, ModelCompletionRequest,
    ModelCompletionResponse, ModelProviderAdapter, ProposedToolCall, ProviderError, TokenUsage,
    ToolExecutor,
};
use gaggle_db::{
    connect_in_memory,
    connection::new_id,
    models::ProjectSettings,
    repositories::{
        chats::ChatRepository, memories::MemoryRepository, projects::ProjectRepository,
        runs::RunRepository,
    },
};
use gaggle_tools::{BuiltinToolExecutor, register_builtin_extensions};
use std::sync::{
    Arc, Mutex,
    atomic::{AtomicUsize, Ordering},
};

/// Captures each provider request so the test can assert what the runtime assembled.
struct ContextCapturingProvider {
    requests: Arc<Mutex<Vec<ModelCompletionRequest>>>,
    call_count: AtomicUsize,
    tool_call: ProposedToolCall,
}

#[async_trait]
impl ModelProviderAdapter for ContextCapturingProvider {
    fn provider_name(&self) -> &str {
        "context-capture-mock"
    }

    async fn complete(
        &self,
        req: &ModelCompletionRequest,
    ) -> Result<ModelCompletionResponse, ProviderError> {
        self.requests.lock().unwrap().push(req.clone());
        let step = self.call_count.fetch_add(1, Ordering::SeqCst);
        if step == 0 {
            Ok(ModelCompletionResponse {
                text: "Saving that decision.".to_string(),
                tool_calls: vec![self.tool_call.clone()],
                usage: TokenUsage {
                    input_tokens: 100,
                    output_tokens: 20,
                    cached_tokens: 0,
                },
            })
        } else {
            Ok(ModelCompletionResponse {
                text: "Decision saved.".to_string(),
                tool_calls: vec![],
                usage: TokenUsage {
                    input_tokens: 150,
                    output_tokens: 10,
                    cached_tokens: 0,
                },
            })
        }
    }
}

#[tokio::test]
async fn test_agent_runtime_assembles_context_and_executes_memory_tools() -> Result<()> {
    let temp_root = std::env::temp_dir().join(format!("gaggle_ctx_{}", new_id()));
    std::fs::create_dir_all(temp_root.join("src"))?;
    std::fs::write(
        temp_root.join("AGENTS.md"),
        "Always run cargo fmt before reporting completion.\nAuthorization: Bearer super-secret-token-value\n",
    )?;
    std::fs::write(
        temp_root.join("src/AGENTS.md"),
        "Prefer explicit error types in src.\n",
    )?;

    let pool = connect_in_memory().await?;
    register_builtin_extensions(&pool).await?;

    let project_repo = ProjectRepository::new(pool.clone());
    let project = project_repo
        .create("Context Project", &temp_root.to_string_lossy(), "git")
        .await?;
    project_repo
        .upsert_settings(&ProjectSettings {
            project_id: project.id.clone(),
            instructions: Some("Project policy: keep changes small and tested.".to_string()),
            ignore_globs: None,
            env: None,
            shell: None,
            setup_command: None,
        })
        .await?;

    let chat_repo = ChatRepository::new(pool.clone());
    let chat = chat_repo
        .create(&project.id, "Context Chat", "user", "local", "full")
        .await?;
    // Prior history must be included in the assembled context.
    chat_repo
        .append_message(&chat.id, None, "user", "Earlier question about storage")
        .await?;
    chat_repo
        .append_message(&chat.id, None, "assistant", "Earlier answer about SQLite")
        .await?;

    let requests = Arc::new(Mutex::new(Vec::new()));
    let provider = ContextCapturingProvider {
        requests: requests.clone(),
        call_count: AtomicUsize::new(0),
        tool_call: ProposedToolCall {
            id: "call_mem_1".to_string(),
            tool_name: "memory.write".to_string(),
            arguments_json: serde_json::json!({
                "kind": "decision",
                "title": "Database choice",
                "content": "Use SQLite WAL. API_KEY=sk-live-super-secret-123456",
                "tags": ["architecture", "sqlite"],
                "pinned": true
            })
            .to_string(),
        },
    };

    let runtime = AgentRuntime::new(pool.clone(), 4);
    let executor = BuiltinToolExecutor::new(pool.clone());
    let cancel = CancellationToken::new();
    let outcome = runtime
        .run_turn(
            &chat.id,
            "Remember our database decision",
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
            assert_eq!(final_text, "Decision saved.");
            run
        }
        other => panic!("Expected completion, got {other:?}"),
    };

    {
        let captured = requests.lock().unwrap();
        assert_eq!(captured.len(), 2);

        let first = &captured[0];
        assert_eq!(first.messages[0].role, "system");
        let system = &first.messages[0].content;
        assert!(system.contains("Project policy: keep changes small and tested."));
        assert!(system.contains("Always run cargo fmt"));
        assert!(system.contains("Prefer explicit error types"));
        assert!(!system.contains("super-secret-token-value"));
        assert!(system.contains("[REDACTED]"));

        assert!(first.messages.iter().any(|message| {
            message.role == "user" && message.content.contains("Earlier question about storage")
        }));
        assert!(first.messages.iter().any(|message| {
            message.role == "assistant" && message.content.contains("Earlier answer about SQLite")
        }));
        let last = first.messages.last().unwrap();
        assert_eq!(last.role, "user");
        assert_eq!(last.content, "Remember our database decision");
        assert_eq!(
            first
                .messages
                .iter()
                .filter(|message| message.content == "Remember our database decision")
                .count(),
            1
        );

        let tool_names: Vec<&str> = first.tools.iter().map(|tool| tool.name.as_str()).collect();
        assert!(tool_names.contains(&"memory.write"));
        assert!(tool_names.contains(&"memory.read"));

        // The second request includes the tool result with the credential redacted.
        let second = &captured[1];
        let tool_message = second
            .messages
            .iter()
            .find(|message| message.role == "tool")
            .expect("tool result should be fed back to the provider");
        assert!(tool_message.content.contains("Database choice"));
        assert!(!tool_message.content.contains("sk-live-super-secret-123456"));
    }

    // Secrets are not persisted into memory rows.
    let memory_repo = MemoryRepository::new(pool.clone());
    let memories = memory_repo.list_by_project(&project.id, 20).await?;
    assert_eq!(memories.len(), 1);
    assert_eq!(memories[0].title, "Database choice");
    assert!(!memories[0].content.contains("sk-live-super-secret-123456"));
    assert!(memories[0].content.contains("[REDACTED]"));

    // The memory.read tool returns the stored memory via FTS5.
    let read_result = executor
        .execute_tool(
            &run.id,
            &chat,
            &temp_root.to_string_lossy(),
            "memory.read",
            r#"{"query":"sqlite","limit":5}"#,
        )
        .await
        .expect("memory.read should succeed");
    assert!(read_result.contains("Database choice"));
    assert!(read_result.contains("architecture"));

    // Run events include the context summary but no message contents or secrets.
    let events = RunRepository::new(pool.clone())
        .list_run_events(&run.id, 0)
        .await?;
    let context_event = events
        .iter()
        .find(|event| event.kind == "context.assembled")
        .expect("context.assembled event");
    assert!(context_event.payload.contains("estimated_tokens"));
    assert!(!context_event.payload.contains("super-secret"));

    let _ = std::fs::remove_dir_all(&temp_root);
    Ok(())
}
