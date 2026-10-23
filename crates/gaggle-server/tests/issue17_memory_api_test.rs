use anyhow::Result;
use axum::{
    body::{Body, to_bytes},
    http::{Request, StatusCode},
};
use gaggle_db::{
    connect_in_memory,
    repositories::{
        memories::MemoryRepository, projects::ProjectRepository, providers::ProviderRepository,
    },
};
use gaggle_server::{MemoryResponse, build_router, initialize_server_state};
use serde_json::json;
use tower::ServiceExt;

async fn json_request(
    app: &axum::Router,
    method: &str,
    uri: &str,
    payload: Option<serde_json::Value>,
) -> Result<(StatusCode, serde_json::Value)> {
    let mut builder = Request::builder().method(method).uri(uri);
    let body = match payload {
        Some(value) => {
            builder = builder.header("content-type", "application/json");
            Body::from(value.to_string())
        }
        None => Body::empty(),
    };
    let response = app.clone().oneshot(builder.body(body)?).await?;
    let status = response.status();
    let bytes = to_bytes(response.into_body(), 1024 * 1024).await?;
    let value = if bytes.is_empty() {
        serde_json::Value::Null
    } else {
        serde_json::from_slice(&bytes).unwrap_or_else(|_| {
            serde_json::Value::String(String::from_utf8_lossy(&bytes).to_string())
        })
    };
    Ok((status, value))
}

#[tokio::test]
async fn test_memory_crud_search_pin_soft_delete_and_export_api() -> Result<()> {
    let pool = connect_in_memory().await?;
    let state = initialize_server_state(pool.clone()).await?;
    let app = build_router(state);

    let project = ProjectRepository::new(pool.clone())
        .create("Memory API", "/tmp/gaggle-memory-api", "none")
        .await?;

    // Create a project memory with tags.
    let (status, body) = json_request(
        &app,
        "POST",
        "/api/memories",
        Some(json!({
            "project_id": project.id,
            "kind": "decision",
            "title": "Database choice",
            "content": "Use SQLite WAL for local persistence.",
            "tags": ["architecture", "sqlite"],
            "pinned": true
        })),
    )
    .await?;
    assert_eq!(status, StatusCode::CREATED);
    let created: MemoryResponse = serde_json::from_value(body)?;
    assert_eq!(created.memory.title, "Database choice");
    assert_eq!(created.memory.pinned, 1);
    assert_eq!(created.tags, vec!["architecture", "sqlite"]);

    // List project memories.
    let (status, body) = json_request(
        &app,
        "GET",
        &format!("/api/memories?project_id={}", project.id),
        None,
    )
    .await?;
    assert_eq!(status, StatusCode::OK);
    let listed: Vec<MemoryResponse> = serde_json::from_value(body)?;
    assert_eq!(listed.len(), 1);
    assert_eq!(listed[0].memory.id, created.memory.id);

    // FTS search finds the memory and a malformed query does not error.
    let (status, body) = json_request(
        &app,
        "GET",
        &format!("/api/memories?project_id={}&query=sqlite", project.id),
        None,
    )
    .await?;
    assert_eq!(status, StatusCode::OK);
    let found: Vec<MemoryResponse> = serde_json::from_value(body)?;
    assert_eq!(found.len(), 1);

    let (status, _) = json_request(
        &app,
        "GET",
        &format!(
            "/api/memories?project_id={}&query=%22unbalanced",
            project.id
        ),
        None,
    )
    .await?;
    assert_eq!(status, StatusCode::OK);

    // Update content and tags through PUT.
    let (status, body) = json_request(
        &app,
        "PUT",
        &format!("/api/memories/{}", created.memory.id),
        Some(json!({
            "content": "Use SQLite with WAL and foreign keys enabled.",
            "tags": ["sqlite"]
        })),
    )
    .await?;
    assert_eq!(status, StatusCode::OK);
    let updated: MemoryResponse = serde_json::from_value(body)?;
    assert_eq!(updated.tags, vec!["sqlite"]);
    assert!(updated.memory.content.contains("foreign keys"));

    // GET by id returns the same updated memory.
    let (status, body) = json_request(
        &app,
        "GET",
        &format!("/api/memories/{}", created.memory.id),
        None,
    )
    .await?;
    assert_eq!(status, StatusCode::OK);
    let fetched: MemoryResponse = serde_json::from_value(body)?;
    assert_eq!(fetched.memory.content, updated.memory.content);

    // Export returns Markdown that includes the memory.
    let (status, body) = json_request(
        &app,
        "GET",
        &format!("/api/memories/export?project_id={}", project.id),
        None,
    )
    .await?;
    assert_eq!(status, StatusCode::OK);
    assert!(
        body["markdown"]
            .as_str()
            .unwrap()
            .contains("Database choice")
    );

    // Soft delete excludes the memory from subsequent reads.
    let (status, _) = json_request(
        &app,
        "DELETE",
        &format!("/api/memories/{}", created.memory.id),
        None,
    )
    .await?;
    assert_eq!(status, StatusCode::NO_CONTENT);

    let repo = MemoryRepository::new(pool);
    assert!(repo.get_by_id(&created.memory.id).await?.is_none());
    let remaining = repo.list_by_project(&project.id, 50).await?;
    assert!(remaining.is_empty());

    // Second delete reports not found.
    let (status, _) = json_request(
        &app,
        "DELETE",
        &format!("/api/memories/{}", created.memory.id),
        None,
    )
    .await?;
    assert_eq!(status, StatusCode::NOT_FOUND);

    Ok(())
}

#[tokio::test]
async fn test_memory_api_rejects_invalid_project_and_kind() -> Result<()> {
    let pool = connect_in_memory().await?;
    let state = initialize_server_state(pool).await?;
    let app = build_router(state);

    let (status, _) = json_request(
        &app,
        "POST",
        "/api/memories",
        Some(json!({
            "project_id": "missing",
            "kind": "fact",
            "title": "Title",
            "content": "Content"
        })),
    )
    .await?;
    assert_eq!(status, StatusCode::NOT_FOUND);

    let (status, _) = json_request(&app, "GET", "/api/memories", None).await?;
    assert_eq!(status, StatusCode::BAD_REQUEST);

    Ok(())
}

#[tokio::test]
async fn test_chatgpt_oauth_start_returns_pkce_authorization_url() -> Result<()> {
    let pool = connect_in_memory().await?;
    let state = initialize_server_state(pool.clone()).await?;
    let app = build_router(state);

    let provider = ProviderRepository::new(pool)
        .create_provider("chatgpt", "ChatGPT", None, "oauth", None)
        .await?;

    let (status, body) = json_request(
        &app,
        "POST",
        &format!("/api/providers/{}/oauth/chatgpt/start", provider.id),
        Some(json!({
            "client_id": "test-client-id",
            "auth_url": "https://example.com/oauth/authorize",
            "token_url": "https://example.com/oauth/token",
            "redirect_uri": "http://127.0.0.1:4321/api/oauth/chatgpt/callback",
            "scopes": ["openid", "offline_access"]
        })),
    )
    .await?;
    assert_eq!(status, StatusCode::OK);
    let url = body["authorization_url"].as_str().unwrap();
    assert!(url.starts_with("https://example.com/oauth/authorize?"));
    assert!(url.contains("code_challenge_method=S256"));
    assert!(url.contains("client_id=test-client-id"));
    assert!(url.contains("state="));
    assert!(!body["state"].as_str().unwrap().is_empty());

    // Unknown state is rejected without making a network call.
    let (status, _) = json_request(
        &app,
        "POST",
        &format!("/api/providers/{}/oauth/chatgpt/exchange", provider.id),
        Some(json!({ "state": "missing", "code": "abc" })),
    )
    .await?;
    assert_eq!(status, StatusCode::BAD_REQUEST);

    Ok(())
}
