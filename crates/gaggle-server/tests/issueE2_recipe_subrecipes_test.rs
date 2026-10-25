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
async fn test_execute_recipe_creates_subagent_child_chats() -> Result<()> {
    let pool = connect_in_memory().await?;
    let state = initialize_server_state(pool.clone()).await?;
    let app = build_router(state);
    let project = ProjectRepository::new(pool.clone())
        .create("p", "/tmp/x", "none")
        .await?;

    // child recipe
    let (status, child_recipe) = json_request(
        &app,
        "POST",
        "/api/recipes",
        Some(json!({
            "name": "child-recipe",
            "title": "Child Recipe",
        })),
    )
    .await?;
    assert_eq!(status, StatusCode::CREATED);
    let child_recipe_id = child_recipe["id"].as_str().unwrap().to_string();

    // parent recipe with sub-recipe
    let (status, parent_recipe) = json_request(
        &app,
        "POST",
        "/api/recipes",
        Some(json!({
            "name": "parent-recipe",
            "title": "Parent Recipe",
            "sub_recipes": [
                {
                    "child_recipe_id": child_recipe_id,
                    "name": "child_sub",
                    "values_json": "{\"k\":\"v\"}",
                    "sequential_when_repeated": 1,
                    "idx": 0
                }
            ]
        })),
    )
    .await?;
    assert_eq!(status, StatusCode::CREATED);
    let parent_recipe_id = parent_recipe["id"].as_str().unwrap().to_string();

    // execute
    let (status, chat) = json_request(
        &app,
        "POST",
        &format!("/api/recipes/{}/execute", parent_recipe_id),
        Some(json!({
            "project_id": project.id,
            "parent_tool_call_id": "tc123"
        })),
    )
    .await?;
    assert_eq!(status, StatusCode::CREATED);
    let chat_id = chat["id"].as_str().unwrap().to_string();

    // list project chats
    let (status, chats) = json_request(
        &app,
        "GET",
        &format!("/api/projects/{}/chats", project.id),
        None,
    )
    .await?;
    assert_eq!(status, StatusCode::OK);
    let children: Vec<&serde_json::Value> = chats
        .as_array()
        .unwrap()
        .iter()
        .filter(|c| c["chat_type"] == "subagent" && c["parent_chat_id"] == chat_id)
        .collect();
    assert_eq!(children.len(), 1, "expected one child subagent chat");
    assert_eq!(children[0]["parent_tool_call_id"], "tc123");
    assert_eq!(children[0]["recipe_id"], child_recipe_id);
    Ok(())
}

#[tokio::test]
async fn test_execute_recipe_cycle_returns_bad_request() -> Result<()> {
    let pool = connect_in_memory().await?;
    let state = initialize_server_state(pool.clone()).await?;
    let app = build_router(state);
    let project = ProjectRepository::new(pool.clone())
        .create("p2", "/tmp/x2", "none")
        .await?;

    // create recipe
    let (status, recipe) = json_request(
        &app,
        "POST",
        "/api/recipes",
        Some(json!({
            "name": "self-recipe",
            "title": "Self Recipe",
        })),
    )
    .await?;
    assert_eq!(status, StatusCode::CREATED);
    let recipe_id = recipe["id"].as_str().unwrap().to_string();

    // make it reference itself as sub-recipe
    let (status, _) = json_request(
        &app,
        "PATCH",
        &format!("/api/recipes/{}", recipe_id),
        Some(json!({
            "sub_recipes": [
                {"child_recipe_id": recipe_id, "name": "self_sub", "idx": 0}
            ]
        })),
    )
    .await?;
    assert_eq!(status, StatusCode::OK);

    let (status, _) = json_request(
        &app,
        "POST",
        &format!("/api/recipes/{}/execute", recipe_id),
        Some(json!({
            "project_id": project.id
        })),
    )
    .await?;
    assert_eq!(status, StatusCode::BAD_REQUEST);
    Ok(())
}
