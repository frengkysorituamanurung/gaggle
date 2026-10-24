use anyhow::Result;
use axum::{
    body::{Body, to_bytes},
    http::{Request, StatusCode},
};
use gaggle_db::{
    connect_in_memory,
    repositories::{chats::ChatRepository, projects::ProjectRepository},
};
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
async fn test_compact_endpoint() -> Result<()> {
    let pool = connect_in_memory().await?;
    let state = initialize_server_state(pool.clone()).await?;
    let app = build_router(state);
    let project = ProjectRepository::new(pool.clone())
        .create("p", "/tmp/x", "none")
        .await?;
    let chat = ChatRepository::new(pool.clone())
        .create(&project.id, "c", "user", "local", "ask")
        .await?;
    for i in 0..3 {
        ChatRepository::new(pool.clone())
            .append_message(&chat.id, None, "user", &format!("msg {}", i))
            .await?;
    }
    let (status, body) = json_request(
        &app,
        "POST",
        &format!("/api/chats/{}/compact", chat.id),
        Some(json!({"trigger":"manual"})),
    )
    .await?;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(body["trigger"], "manual");
    assert!(body["covers_until_seq"].as_i64().unwrap() >= 3);
    Ok(())
}

#[tokio::test]
async fn test_compact_endpoint_no_body() -> Result<()> {
    let pool = connect_in_memory().await?;
    let state = initialize_server_state(pool.clone()).await?;
    let app = build_router(state);
    let project = ProjectRepository::new(pool.clone())
        .create("p2", "/tmp/x2", "none")
        .await?;
    let chat = ChatRepository::new(pool.clone())
        .create(&project.id, "c2", "user", "local", "ask")
        .await?;
    for i in 0..3 {
        ChatRepository::new(pool.clone())
            .append_message(&chat.id, None, "user", &format!("msg {}", i))
            .await?;
    }
    let (status, body) = json_request(
        &app,
        "POST",
        &format!("/api/chats/{}/compact", chat.id),
        None,
    )
    .await?;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(body["trigger"], "manual");
    Ok(())
}
