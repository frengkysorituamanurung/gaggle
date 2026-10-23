use anyhow::Result;
use async_trait::async_trait;
use gaggle_core::{
    AgentLoopOutcome, AgentRuntime, CancellationToken, ModelCompletionRequest,
    ModelCompletionResponse, ModelCompletionStream, ModelProviderAdapter, ModelStreamEvent,
    ProviderError, TokenUsage, ToolExecutor,
};
use gaggle_db::{
    connect_in_memory,
    models::Chat,
    repositories::{chats::ChatRepository, projects::ProjectRepository, runs::RunRepository},
};

struct StreamingProvider;

#[async_trait]
impl ModelProviderAdapter for StreamingProvider {
    fn provider_name(&self) -> &str {
        "streaming-mock"
    }

    async fn complete(
        &self,
        _req: &ModelCompletionRequest,
    ) -> Result<ModelCompletionResponse, ProviderError> {
        Err(ProviderError::Transport {
            provider: "streaming-mock".to_string(),
            message: "complete() should not be used when stream() is available".to_string(),
        })
    }

    fn stream(&self, _req: ModelCompletionRequest) -> Option<ModelCompletionStream> {
        Some(Box::pin(futures::stream::iter(vec![
            Ok(ModelStreamEvent::TextDelta("Hello ".to_string())),
            Ok(ModelStreamEvent::TextDelta("world".to_string())),
            Ok(ModelStreamEvent::Usage(TokenUsage {
                input_tokens: 21,
                output_tokens: 7,
                cached_tokens: 2,
            })),
            Ok(ModelStreamEvent::Done(ModelCompletionResponse {
                text: "Hello world".to_string(),
                tool_calls: vec![],
                usage: TokenUsage {
                    input_tokens: 21,
                    output_tokens: 7,
                    cached_tokens: 2,
                },
            })),
        ])))
    }
}

struct NoopToolExecutor;

#[async_trait]
impl ToolExecutor for NoopToolExecutor {
    async fn execute_tool(
        &self,
        _run_id: &str,
        _chat: &Chat,
        _project_root: &str,
        tool_name: &str,
        _args_json: &str,
    ) -> Result<String, String> {
        Err(format!("Tool '{tool_name}' is not available in this test"))
    }
}

#[tokio::test]
async fn test_agent_runtime_consumes_native_provider_stream() -> Result<()> {
    let temp_root =
        std::env::temp_dir().join(format!("gaggle_stream_{}", gaggle_db::connection::new_id()));
    std::fs::create_dir_all(&temp_root)?;

    let pool = connect_in_memory().await?;
    let project = ProjectRepository::new(pool.clone())
        .create("Streaming", &temp_root.to_string_lossy(), "none")
        .await?;
    let chat = ChatRepository::new(pool.clone())
        .create(&project.id, "Streaming Chat", "user", "local", "ask")
        .await?;

    let runtime = AgentRuntime::new(pool.clone(), 3);
    let outcome = runtime
        .run_turn(
            &chat.id,
            "Say hello",
            "streaming-model",
            &StreamingProvider,
            &NoopToolExecutor,
            &CancellationToken::new(),
        )
        .await?;

    let run = match outcome {
        AgentLoopOutcome::Completed {
            run,
            final_text,
            steps_executed,
        } => {
            assert_eq!(steps_executed, 1);
            assert_eq!(final_text, "Hello world");
            assert_eq!(run.status, "completed");
            run
        }
        other => panic!("Expected Completed, got {other:?}"),
    };

    let events = RunRepository::new(pool.clone())
        .list_run_events(&run.id, 0)
        .await?;
    let deltas: Vec<&str> = events
        .iter()
        .filter(|event| event.kind == "message.delta")
        .map(|event| event.payload.as_str())
        .collect();
    assert_eq!(deltas.len(), 2, "one delta per streamed chunk");
    assert!(deltas[0].contains("Hello "));
    assert!(deltas[1].contains("world"));
    assert!(events.iter().any(|event| event.kind == "context.assembled"));
    assert!(events.iter().any(|event| event.kind == "run.completed"));

    let updated_chat = ChatRepository::new(pool)
        .get_by_id(&chat.id)
        .await?
        .unwrap();
    assert_eq!(updated_chat.total_input_tokens, 21);
    assert_eq!(updated_chat.total_output_tokens, 7);

    let _ = std::fs::remove_dir_all(&temp_root);
    Ok(())
}
