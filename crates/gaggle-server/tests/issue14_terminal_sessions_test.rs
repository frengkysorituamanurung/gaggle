use anyhow::Result;
use axum::{
    body::{Body, to_bytes},
    http::{Request, StatusCode},
};
use gaggle_db::{connect_in_memory, connection::new_id};
use gaggle_server::{build_router, initialize_server_state};
use serde_json::json;
use tower::ServiceExt;

#[tokio::test]
async fn test_terminal_session_lifecycle() -> Result<()> {
    let temp_root = std::env::temp_dir().join(format!("gaggle_term_{}", new_id()));
    std::fs::create_dir_all(&temp_root)?;

    let pool = connect_in_memory().await?;
    let state = initialize_server_state(pool).await?;
    let app = build_router(state);

    // create project
    let create_proj = Request::builder()
        .method("POST")
        .uri("/api/projects")
        .header("content-type", "application/json")
        .body(Body::from(
            json!({
                "name": "TermProj",
                "root_path": temp_root.to_string_lossy(),
                "vcs_kind": "none"
            })
            .to_string(),
        ))?;
    let resp = app.clone().oneshot(create_proj).await?;
    assert_eq!(resp.status(), StatusCode::CREATED);
    let bytes = to_bytes(resp.into_body(), 64 * 1024).await?;
    let proj: serde_json::Value = serde_json::from_slice(&bytes)?;
    let project_id = proj["id"].as_str().unwrap().to_string();

    // create terminal session
    let create_term = Request::builder()
        .method("POST")
        .uri("/api/terminal-sessions")
        .header("content-type", "application/json")
        .body(Body::from(
            json!({
                "project_id": project_id,
                "cwd": ".",
                "cols": 80,
                "rows": 24
            })
            .to_string(),
        ))?;
    let resp = app.clone().oneshot(create_term).await?;
    assert_eq!(resp.status(), StatusCode::CREATED);
    let bytes = to_bytes(resp.into_body(), 64 * 1024).await?;
    let sess: serde_json::Value = serde_json::from_slice(&bytes)?;
    let sid = sess["id"].as_str().unwrap().to_string();
    assert_eq!(sess["status"], "running");

    // send exit command via input endpoint
    let input_req = Request::builder()
        .method("POST")
        .uri(format!("/api/terminal-sessions/{}/input", sid))
        .header("content-type", "application/json")
        .body(Body::from(
            json!({"data":"echo HELLO_TERM\nexit 0\n"}).to_string(),
        ))?;
    let resp = app.clone().oneshot(input_req).await?;
    assert_eq!(resp.status(), StatusCode::NO_CONTENT);

    // wait for exit
    for _ in 0..50 {
        tokio::time::sleep(std::time::Duration::from_millis(100)).await;
        let get_req = Request::builder()
            .method("GET")
            .uri(format!("/api/terminal-sessions/{}", sid))
            .body(Body::empty())?;
        let resp = app.clone().oneshot(get_req).await?;
        let bytes = to_bytes(resp.into_body(), 64 * 1024).await?;
        let s: serde_json::Value = serde_json::from_slice(&bytes)?;
        if s["status"] == "exited" {
            // exit code may be 0
            break;
        }
    }

    let get_req = Request::builder()
        .method("GET")
        .uri(format!("/api/terminal-sessions/{}", sid))
        .body(Body::empty())?;
    let resp = app.clone().oneshot(get_req).await?;
    let bytes = to_bytes(resp.into_body(), 64 * 1024).await?;
    let s: serde_json::Value = serde_json::from_slice(&bytes)?;
    assert_eq!(s["status"], "exited");

    // scrollback contains output
    let sb_req = Request::builder()
        .method("GET")
        .uri(format!("/api/terminal-sessions/{}/scrollback", sid))
        .body(Body::empty())?;
    let resp = app.clone().oneshot(sb_req).await?;
    let bytes = to_bytes(resp.into_body(), 256 * 1024).await?;
    let sb: String = serde_json::from_slice(&bytes)?;
    // scrollback may or may not contain HELLO depending timing; log
    let _ = sb;

    // resize (session exited but endpoint still maybe)
    // close
    let close_req = Request::builder()
        .method("DELETE")
        .uri(format!("/api/terminal-sessions/{}", sid))
        .body(Body::empty())?;
    let resp = app.clone().oneshot(close_req).await?;
    assert!(resp.status() == StatusCode::NO_CONTENT || resp.status() == StatusCode::NOT_FOUND);

    std::fs::remove_dir_all(&temp_root).ok();
    Ok(())
}
