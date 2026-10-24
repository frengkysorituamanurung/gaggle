use anyhow::Result;
use axum::{
    body::{Body, to_bytes},
    http::{Request, StatusCode},
};
use gaggle_db::{
    connect_in_memory,
    connection::new_id,
    repositories::{chats::ChatRepository, projects::ProjectRepository, runs::RunRepository},
};
use gaggle_server::{build_router, initialize_server_state};
use serde_json::Value;
use tower::ServiceExt;

async fn response_json(response: axum::response::Response) -> Result<Value> {
    let bytes = to_bytes(response.into_body(), 256 * 1024).await?;
    Ok(serde_json::from_slice(&bytes)?)
}

#[tokio::test]
async fn test_diagnostics_and_export_api_are_redacted_and_actionable() -> Result<()> {
    let temp_root = std::env::temp_dir().join(format!("gaggle_issue23_api_{}", new_id()));
    std::fs::create_dir_all(&temp_root)?;

    let pool = connect_in_memory().await?;
    let state = initialize_server_state(pool.clone()).await?;
    let app = build_router(state);

    let project = ProjectRepository::new(pool.clone())
        .create("Issue 23", &temp_root.to_string_lossy(), "none")
        .await?;
    let chat = ChatRepository::new(pool.clone())
        .create(&project.id, "Diagnostics Chat", "user", "local", "ask")
        .await?;

    let secret = "sk-live-API-EXPORT-SECRET-1234567890";
    let prompt = "API-PROMPT-MUST-NOT-LEAK";
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
                "category=rate_limited operation=agent.turn data_preservation=preserved \
                 retryable=true: provider rejected {secret}. Next step: wait and retry."
            )),
        )
        .await?;

    // --- Live diagnostics report -------------------------------------------------
    let diagnostics_res = app
        .clone()
        .oneshot(
            Request::builder()
                .method("GET")
                .uri("/api/diagnostics")
                .body(Body::empty())?,
        )
        .await?;
    assert_eq!(diagnostics_res.status(), StatusCode::OK);
    let diagnostics = response_json(diagnostics_res).await?;

    assert_eq!(diagnostics["service"], "gaggle");
    assert_eq!(diagnostics["build"]["service"], "gaggle");
    assert!(diagnostics["build"]["version"].as_str().is_some());
    assert!(diagnostics["build"]["commit"].as_str().is_some());
    assert_eq!(diagnostics["core_health"]["status"], "ok");
    assert!(
        diagnostics["core_health"]["checks"]
            .as_array()
            .is_some_and(|checks| !checks.is_empty())
    );

    let capabilities = diagnostics["capabilities"]
        .as_array()
        .expect("capabilities array");
    assert!(capabilities.iter().any(|capability| {
        capability["id"] == "local_mock_model" && capability["available"] == true
    }));
    let telemetry = capabilities
        .iter()
        .find(|capability| capability["id"] == "automatic_telemetry")
        .expect("telemetry capability");
    assert_eq!(telemetry["state"], "unavailable");
    assert!(
        telemetry["reason"]
            .as_str()
            .unwrap_or_default()
            .contains("does not send telemetry")
    );
    let remote = capabilities
        .iter()
        .find(|capability| capability["id"] == "remote_models")
        .expect("remote model capability");
    assert_eq!(remote["available"], false);
    assert!(
        remote["reason"]
            .as_str()
            .is_some_and(|reason| !reason.is_empty())
    );
    assert!(
        remote["action"]
            .as_str()
            .is_some_and(|action| !action.is_empty())
    );

    let diagnostics_body = diagnostics.to_string();
    assert!(!diagnostics_body.contains(secret));
    assert!(!diagnostics_body.contains(prompt));

    // --- Explicit redacted export ------------------------------------------------
    let export_res = app
        .oneshot(
            Request::builder()
                .method("GET")
                .uri("/api/diagnostics/export")
                .body(Body::empty())?,
        )
        .await?;
    assert_eq!(export_res.status(), StatusCode::OK);
    let export = response_json(export_res).await?;

    assert_eq!(export["format_version"], 1);
    assert_eq!(export["privacy"]["telemetry_enabled"], false);
    assert_eq!(export["privacy"]["contains_prompts"], false);
    assert_eq!(export["privacy"]["contains_source_files"], false);
    assert_eq!(export["privacy"]["contains_raw_tool_output"], false);
    assert_eq!(export["privacy"]["contains_secrets"], false);
    assert_eq!(export["privacy"]["generated_by_user_action"], true);
    assert!(export["report"]["build"]["version"].as_str().is_some());

    let failures = export["recent_failures"]
        .as_array()
        .expect("recent failures array");
    let failure = failures
        .iter()
        .find(|failure| failure["run_id"] == run.id)
        .expect("failed run appears in export");
    assert_eq!(failure["failure_category"], "rate_limited");
    assert!(
        failure["message"]
            .as_str()
            .unwrap_or_default()
            .contains("[REDACTED]")
    );

    let export_body = export.to_string();
    assert!(!export_body.contains(secret), "export leaked a secret");
    assert!(!export_body.contains(prompt), "export leaked a prompt");
    assert!(
        export_body.contains("no telemetry") || export_body.contains("does not send telemetry")
    );

    let _ = std::fs::remove_dir_all(&temp_root);
    Ok(())
}
