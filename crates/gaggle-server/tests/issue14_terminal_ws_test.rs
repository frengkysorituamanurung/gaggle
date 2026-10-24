use anyhow::Result;
use axum::http::StatusCode;
use futures::{SinkExt, StreamExt};
use gaggle_db::{connect_in_memory, connection::new_id};
use gaggle_server::{build_router, initialize_server_state};
use serde_json::json;
use tokio::net::TcpListener;
use tokio_tungstenite::connect_async;

#[tokio::test]
async fn test_terminal_ws() -> Result<()> {
    let temp_root = std::env::temp_dir().join(format!("gaggle_termws_{}", new_id()));
    std::fs::create_dir_all(&temp_root)?;

    let pool = connect_in_memory().await?;
    let state = initialize_server_state(pool).await?;
    let app = build_router(state);

    let listener = TcpListener::bind("127.0.0.1:0").await?;
    let addr = listener.local_addr()?;
    let server = tokio::spawn(async move {
        axum::serve(listener, app).await.unwrap();
    });

    // create project
    let client = reqwest::Client::new();
    let proj_resp = client
        .post(format!("http://{}/api/projects", addr))
        .json(&json!({"name":"p","root_path":temp_root.to_string_lossy(),"vcs_kind":"none"}))
        .send()
        .await?;
    assert_eq!(proj_resp.status(), StatusCode::CREATED);
    let proj: serde_json::Value = proj_resp.json().await?;
    let project_id = proj["id"].as_str().unwrap();

    // create terminal
    let term_resp = client
        .post(format!("http://{}/api/terminal-sessions", addr))
        .json(&json!({"project_id":project_id,"cols":80,"rows":24}))
        .send()
        .await?;
    assert_eq!(term_resp.status(), StatusCode::CREATED);
    let term: serde_json::Value = term_resp.json().await?;
    let sid = term["id"].as_str().unwrap();

    // connect ws
    let url = format!("ws://{}/api/terminal-sessions/{}/ws", addr, sid);
    let (mut ws, _) = connect_async(url).await?;
    // send input
    ws.send(tokio_tungstenite::tungstenite::Message::Text(
        json!({"type":"input","data":"echo WS_HELLO\n"})
            .to_string()
            .into(),
    ))
    .await?;

    // collect output
    let mut collected = String::new();
    let deadline = tokio::time::Instant::now() + std::time::Duration::from_secs(5);
    while tokio::time::Instant::now() < deadline {
        if let Ok(Some(Ok(msg))) =
            tokio::time::timeout(std::time::Duration::from_millis(200), ws.next()).await
        {
            if let tokio_tungstenite::tungstenite::Message::Binary(b) = msg {
                collected.push_str(&String::from_utf8_lossy(&b));
                if collected.contains("WS_HELLO") {
                    break;
                }
            } else if let tokio_tungstenite::tungstenite::Message::Text(t) = msg {
                collected.push_str(&t);
                if collected.contains("WS_HELLO") {
                    break;
                }
            }
        }
    }
    assert!(collected.contains("WS_HELLO"), "output: {}", collected);

    // resize via ws
    ws.send(tokio_tungstenite::tungstenite::Message::Text(
        json!({"type":"resize","cols":100,"rows":30})
            .to_string()
            .into(),
    ))
    .await?;
    tokio::time::sleep(std::time::Duration::from_millis(100)).await;

    // close
    let _ = client
        .delete(format!("http://{}/api/terminal-sessions/{}", addr, sid))
        .send()
        .await;
    let _ = ws.close(None).await;
    server.abort();
    std::fs::remove_dir_all(&temp_root).ok();
    Ok(())
}
