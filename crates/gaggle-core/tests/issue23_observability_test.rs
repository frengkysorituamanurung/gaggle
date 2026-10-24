use anyhow::Result;
use async_trait::async_trait;
use gaggle_core::{
    ActionableError, AgentLoopOutcome, AgentRuntime, CancellationToken, CapabilityState,
    FailureCategory, HealthStatus, ModelCompletionRequest, ModelCompletionResponse,
    ModelProviderAdapter, ProposedToolCall, ProviderError, TokenUsage, ToolExecutor,
    collect_diagnostics, export_diagnostics,
};
use gaggle_db::{
    connect_in_memory,
    connection::{new_id, now_ms},
    models::Chat,
    repositories::{chats::ChatRepository, projects::ProjectRepository, runs::RunRepository},
};

/// Registers a minimal built-in extension tool so the permission engine can resolve it.
async fn register_test_tool(
    pool: &sqlx::SqlitePool,
    tool_name: &str,
    is_read_only: bool,
) -> Result<()> {
    let extension_id = new_id();
    let now = now_ms();
    sqlx::query(
        "INSERT INTO extensions (id, name, kind, version, source, bundled, enabled, installed_at) \
         VALUES (?, ?, 'builtin', '0.1.0', 'bundled', 1, 1, ?)",
    )
    .bind(&extension_id)
    .bind(format!("test.{tool_name}"))
    .bind(now)
    .execute(pool)
    .await?;
    sqlx::query(
        "INSERT INTO extension_tools (id, extension_id, tool_name, is_read_only, enabled) \
         VALUES (?, ?, ?, ?, 1)",
    )
    .bind(new_id())
    .bind(&extension_id)
    .bind(tool_name)
    .bind(if is_read_only { 1_i64 } else { 0_i64 })
    .execute(pool)
    .await?;
    Ok(())
}

/// Provider that proposes one tool call, then completes with a final answer.
struct ToolCallProvider {
    tool_call: ProposedToolCall,
    calls: std::sync::atomic::AtomicUsize,
}

#[async_trait]
impl ModelProviderAdapter for ToolCallProvider {
    fn provider_name(&self) -> &str {
        "issue23-tool-call"
    }

    async fn complete(
        &self,
        _req: &ModelCompletionRequest,
    ) -> Result<ModelCompletionResponse, ProviderError> {
        let step = self.calls.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
        if step == 0 {
            Ok(ModelCompletionResponse {
                text: String::new(),
                tool_calls: vec![self.tool_call.clone()],
                usage: TokenUsage {
                    input_tokens: 10,
                    output_tokens: 5,
                    cached_tokens: 0,
                },
            })
        } else {
            Ok(ModelCompletionResponse {
                text: "Done".to_string(),
                tool_calls: Vec::new(),
                usage: TokenUsage {
                    input_tokens: 20,
                    output_tokens: 5,
                    cached_tokens: 0,
                },
            })
        }
    }
}

struct TimeoutProvider;

#[async_trait]
impl ModelProviderAdapter for TimeoutProvider {
    fn provider_name(&self) -> &str {
        "issue23-timeout"
    }

    async fn complete(
        &self,
        _req: &ModelCompletionRequest,
    ) -> Result<ModelCompletionResponse, ProviderError> {
        Err(ProviderError::Timeout { timeout_ms: 250 })
    }
}

/// Executor that echoes a sensitive payload so the test can prove it never reaches logs.
struct SecretEchoExecutor;

#[async_trait]
impl ToolExecutor for SecretEchoExecutor {
    async fn execute_tool(
        &self,
        _run_id: &str,
        _chat: &Chat,
        _project_root: &str,
        tool_name: &str,
        args_json: &str,
    ) -> Result<String, String> {
        Ok(serde_json::json!({
            "tool": tool_name,
            "args": args_json,
            "output": "sk-live-TOOL-OUTPUT-1234567890"
        })
        .to_string())
    }
}

#[tokio::test]
async fn test_diagnostics_report_includes_build_health_and_capabilities() -> Result<()> {
    let pool = connect_in_memory().await?;
    let report = collect_diagnostics(&pool, std::path::Path::new(":memory:")).await?;

    assert_eq!(report.service, "gaggle");
    assert_eq!(report.build.service, "gaggle");
    assert!(!report.build.version.is_empty());
    assert!(!report.build.target.is_empty());
    assert!(report.db_healthy);
    assert_eq!(report.core_health.status, HealthStatus::Ok);
    assert!(
        report
            .core_health
            .checks
            .iter()
            .any(|check| check.name == "migrations_applied" && check.status == HealthStatus::Ok)
    );
    assert!(
        report
            .capabilities
            .iter()
            .any(|capability| capability.id == "local_mock_model" && capability.available)
    );

    let remote = report
        .capabilities
        .iter()
        .find(|capability| capability.id == "remote_models")
        .expect("remote model capability is reported");
    assert_eq!(remote.state, CapabilityState::Unavailable);
    assert!(!remote.available);
    assert!(remote.reason.contains("No provider"));
    assert!(
        remote.action.is_some(),
        "unavailable capabilities need an action"
    );

    let telemetry = report
        .capabilities
        .iter()
        .find(|capability| capability.id == "automatic_telemetry")
        .expect("telemetry posture is reported");
    assert_eq!(telemetry.state, CapabilityState::Unavailable);
    assert!(telemetry.reason.contains("does not send telemetry"));

    Ok(())
}

#[tokio::test]
async fn test_diagnostics_export_redacts_secrets_and_excludes_prompts() -> Result<()> {
    let pool = connect_in_memory().await?;
    let temp_root = std::env::temp_dir().join(format!("gaggle_issue23_export_{}", new_id()));
    std::fs::create_dir_all(&temp_root)?;

    let project = ProjectRepository::new(pool.clone())
        .create("Export", &temp_root.to_string_lossy(), "none")
        .await?;
    let chat = ChatRepository::new(pool.clone())
        .create(&project.id, "Diagnostics Chat", "user", "local", "ask")
        .await?;

    let secret = "sk-live-EXPORT-SECRET-1234567890";
    let prompt = "PROMPT-SHOULD-NOT-BE-EXPORTED";
    ChatRepository::new(pool.clone())
        .append_message(&chat.id, None, "user", prompt)
        .await?;

    let run_repo = RunRepository::new(pool.clone());
    let run = run_repo.create(&chat.id, None, None).await?;
    run_repo
        .update_run_status(
            &run.id,
            "failed",
            Some(&format!(
                "category=provider_timeout operation=agent.turn data_preservation=preserved retryable=true: \
                 provider timed out with {secret}. Next step: retry."
            )),
        )
        .await?;

    let export = export_diagnostics(&pool, std::path::Path::new(":memory:")).await?;
    assert_eq!(export.format_version, 1);
    assert!(!export.privacy.telemetry_enabled);
    assert!(!export.privacy.contains_prompts);
    assert!(!export.privacy.contains_source_files);
    assert!(!export.privacy.contains_raw_tool_output);
    assert!(!export.privacy.contains_secrets);
    assert!(export.privacy.generated_by_user_action);

    let failure = export
        .recent_failures
        .iter()
        .find(|failure| failure.run_id == run.id)
        .expect("failed run is included in the export");
    assert_eq!(
        failure.failure_category.as_deref(),
        Some("provider_timeout")
    );
    assert!(!failure.message.contains(secret));
    assert!(failure.message.contains("[REDACTED]"));

    let rendered = serde_json::to_string(&export)?;
    assert!(
        !rendered.contains(secret),
        "export must not contain secrets"
    );
    assert!(
        !rendered.contains(prompt),
        "export must not contain prompts or source content"
    );
    assert!(rendered.contains("\"build\""));
    assert!(rendered.contains("\"core_health\""));
    assert!(rendered.contains("\"capabilities\""));
    assert!(rendered.contains("no telemetry") || rendered.contains("does not send telemetry"));

    let _ = std::fs::remove_dir_all(&temp_root);
    Ok(())
}

#[tokio::test]
async fn test_provider_failure_is_actionable_and_persisted() -> Result<()> {
    let pool = connect_in_memory().await?;
    let temp_root = std::env::temp_dir().join(format!("gaggle_issue23_fail_{}", new_id()));
    std::fs::create_dir_all(&temp_root)?;

    let project = ProjectRepository::new(pool.clone())
        .create("Failure", &temp_root.to_string_lossy(), "none")
        .await?;
    let chat = ChatRepository::new(pool.clone())
        .create(&project.id, "Failure Chat", "user", "local", "ask")
        .await?;

    let runtime = AgentRuntime::new(pool.clone(), 3);
    let outcome = runtime
        .run_turn(
            &chat.id,
            "Trigger a provider timeout",
            "timeout-model",
            &TimeoutProvider,
            &SecretEchoExecutor,
            &CancellationToken::new(),
        )
        .await?;

    let (run, error) = match outcome {
        AgentLoopOutcome::Failed { run, error } => (run, error),
        other => panic!("Expected Failed, got {other:?}"),
    };
    assert_eq!(run.status, "failed");
    assert!(error.contains("category=provider_timeout"));
    assert!(error.contains("operation=agent.turn"));
    assert!(error.contains("data_preservation=preserved"));
    assert!(error.contains("retryable=true"));
    assert!(error.contains("Next step:"));

    let events = RunRepository::new(pool.clone())
        .list_run_events(&run.id, 0)
        .await?;
    let failed_event = events
        .iter()
        .find(|event| event.kind == "run.failed")
        .expect("run.failed event");
    let payload: serde_json::Value = serde_json::from_str(&failed_event.payload)?;
    assert_eq!(payload["failure_category"], "provider_timeout");
    assert_eq!(payload["operation"], "agent.turn");
    assert_eq!(payload["data_preservation"], "preserved");
    assert_eq!(payload["retryable"], true);
    assert!(
        payload["retry_guidance"]
            .as_str()
            .unwrap_or_default()
            .contains("provider")
    );

    // The same failure can be reconstructed from a category and redacted message.
    let reconstructed = ActionableError::new(
        FailureCategory::ProviderTimeout,
        "agent.turn",
        "Provider request timed out",
    );
    assert!(
        reconstructed
            .to_string()
            .contains("category=provider_timeout")
    );

    let _ = std::fs::remove_dir_all(&temp_root);
    Ok(())
}

#[tokio::test]
async fn test_runtime_logs_correlation_ids_and_never_logs_tool_output() -> Result<()> {
    let pool = connect_in_memory().await?;
    let temp_root = std::env::temp_dir().join(format!("gaggle_issue23_corr_{}", new_id()));
    std::fs::create_dir_all(&temp_root)?;

    let project = ProjectRepository::new(pool.clone())
        .create("Correlation", &temp_root.to_string_lossy(), "none")
        .await?;
    let chat = ChatRepository::new(pool.clone())
        .create(&project.id, "Correlation Chat", "user", "local", "ask")
        .await?;
    register_test_tool(&pool, "fs.read", true).await?;

    let secret_args = r#"{"path":"a.txt","content":"sk-live-ARG-SECRET-1234567890","command":"cat /etc/passwd","password":"hunter2"}"#;
    let provider = ToolCallProvider {
        tool_call: ProposedToolCall {
            id: "call_correlation".to_string(),
            tool_name: "fs.read".to_string(),
            arguments_json: secret_args.to_string(),
        },
        calls: std::sync::atomic::AtomicUsize::new(0),
    };

    let logs = std::sync::Arc::new(std::sync::Mutex::new(Vec::<u8>::new()));
    let writer = SharedWriter(logs.clone());
    let subscriber = tracing_subscriber::fmt()
        .with_max_level(tracing::Level::TRACE)
        .with_ansi(false)
        .with_writer(writer)
        .finish();
    let dispatch = tracing::Dispatch::new(subscriber);
    let _guard = tracing::dispatcher::set_default(&dispatch);

    let runtime = AgentRuntime::new(pool.clone(), 3);
    let outcome = runtime
        .run_turn(
            &chat.id,
            "PROMPT-TEXT-MUST-NOT-BE-LOGGED",
            "tool-model",
            &provider,
            &SecretEchoExecutor,
            &CancellationToken::new(),
        )
        .await?;

    let run = match outcome {
        AgentLoopOutcome::Completed { run, .. } => run,
        other => panic!("Expected Completed, got {other:?}"),
    };

    let captured = String::from_utf8_lossy(&logs.lock().expect("log buffer lock")).to_string();
    assert!(
        captured.contains(&format!("chat_id={}", chat.id)),
        "chat_id must be propagated to logs:\n{captured}"
    );
    assert!(
        captured.contains(&format!("run_id={}", run.id)),
        "run_id must be propagated to logs:\n{captured}"
    );

    let tool_call_id: String = sqlx::query_scalar("SELECT id FROM tool_calls WHERE run_id = ?")
        .bind(&run.id)
        .fetch_one(&pool)
        .await?;
    assert!(
        captured.contains(&format!("tool_call_id={tool_call_id}")),
        "tool_call_id must be propagated to logs:\n{captured}"
    );
    assert!(captured.contains("args_fingerprint"));

    assert!(!captured.contains("sk-live-ARG-SECRET-1234567890"));
    assert!(!captured.contains("sk-live-TOOL-OUTPUT-1234567890"));
    assert!(!captured.contains("hunter2"));
    assert!(!captured.contains("cat /etc/passwd"));
    assert!(!captured.contains("PROMPT-TEXT-MUST-NOT-BE-LOGGED"));

    let _ = std::fs::remove_dir_all(&temp_root);
    Ok(())
}

#[derive(Clone)]
struct SharedWriter(std::sync::Arc<std::sync::Mutex<Vec<u8>>>);

impl std::io::Write for SharedWriter {
    fn write(&mut self, buf: &[u8]) -> std::io::Result<usize> {
        self.0
            .lock()
            .map_err(|_| std::io::Error::other("log buffer lock poisoned"))?
            .extend_from_slice(buf);
        Ok(buf.len())
    }

    fn flush(&mut self) -> std::io::Result<()> {
        Ok(())
    }
}

impl<'a> tracing_subscriber::fmt::MakeWriter<'a> for SharedWriter {
    type Writer = Self;

    fn make_writer(&'a self) -> Self::Writer {
        self.clone()
    }
}
