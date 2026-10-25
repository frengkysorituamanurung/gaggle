use anyhow::Result;
use axum::{
    body::{Body, to_bytes},
    http::{Request, StatusCode},
};
use gaggle_db::{connect_in_memory, repositories::projects::ProjectRepository};
use gaggle_server::{build_router, initialize_server_state};
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
async fn test_create_chat_and_submit_message_persists_history() -> Result<()> {
    let pool = connect_in_memory().await?;
    let state = initialize_server_state(pool.clone()).await?;
    let app = build_router(state);
    let project = ProjectRepository::new(pool.clone())
        .create("p", "/tmp/x", "none")
        .await?;

    let (status, chat) = json_request(
        &app,
        "POST",
        "/api/chats",
        Some(json!({
            "project_id": project.id,
            "title": "Chat 1",
            "mode": "local",
            "permission_mode": "ask"
        })),
    )
    .await?;
    assert_eq!(status, StatusCode::CREATED);
    let chat_id = chat["id"].as_str().unwrap().to_string();

    let (status, outcome) = json_request(
        &app,
        "POST",
        &format!("/api/chats/{}/messages", chat_id),
        Some(json!({"prompt": "hello", "model_key": "local-mock"})),
    )
    .await?;
    assert_eq!(status, StatusCode::OK);
    assert!(outcome["run"].is_object());

    // Retrieval by id
    let (status, fetched) =
        json_request(&app, "GET", &format!("/api/chats/{}", chat_id), None).await?;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(fetched["id"], chat_id);

    // History restoration
    let (status, history) = json_request(
        &app,
        "GET",
        &format!("/api/chats/{}/messages", chat_id),
        None,
    )
    .await?;
    assert_eq!(status, StatusCode::OK);
    let messages = history["messages"].as_array().unwrap();
    assert!(messages.len() >= 2, "expected user and assistant messages");
    Ok(())
}

#[tokio::test]
async fn test_submit_message_unknown_chat_returns_not_found() -> Result<()> {
    let pool = connect_in_memory().await?;
    let state = initialize_server_state(pool.clone()).await?;
    let app = build_router(state);
    let (status, _) = json_request(
        &app,
        "POST",
        "/api/chats/does-not-exist/messages",
        Some(json!({"prompt": "hi", "model_key": "local-mock"})),
    )
    .await?;
    assert_eq!(status, StatusCode::NOT_FOUND);
    Ok(())
}
