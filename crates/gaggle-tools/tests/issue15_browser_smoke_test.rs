use anyhow::Result;
use gaggle_core::ToolExecutor;
use gaggle_db::{
    connect_in_memory,
    connection::new_id,
    repositories::{chats::ChatRepository, projects::ProjectRepository, runs::RunRepository},
};
use gaggle_tools::{BuiltinToolExecutor, browser_tools, register_builtin_extensions};
use std::net::SocketAddr;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::TcpListener;

async fn spawn_test_server() -> Result<(SocketAddr, tokio::task::JoinHandle<()>)> {
    let listener = TcpListener::bind("127.0.0.1:0").await?;
    let addr = listener.local_addr()?;
    let handle = tokio::spawn(async move {
        loop {
            if let Ok((mut socket, _)) = listener.accept().await {
                let mut buf = [0u8; 1024];
                let _ = socket.read(&mut buf).await;
                let body = r#"<!doctype html><html><head><title>Test Page</title></head><body>
<h1>Hello Browser</h1>
<input id="name" />
<button id="btn">Click me</button>
<a href="/next">Next</a>
</body></html>"#;
                let resp = format!(
                    "HTTP/1.1 200 OK\r\nContent-Type: text/html\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{}",
                    body.len(),
                    body
                );
                let _ = socket.write_all(resp.as_bytes()).await;
            }
        }
    });
    Ok((addr, handle))
}

#[tokio::test]
async fn test_browser_navigate_click_type_scroll_screenshot_extract() -> Result<()> {
    if !browser_tools::chromium_available() {
        eprintln!("SKIP: Chromium/Chrome not available");
        return Ok(());
    }

    let (addr, _server) = spawn_test_server().await?;
    let url = format!("http://{}/", addr);

    let pool = connect_in_memory().await?;
    register_builtin_extensions(&pool).await?;

    let proj_repo = ProjectRepository::new(pool.clone());
    let chat_repo = ChatRepository::new(pool.clone());
    let run_repo = RunRepository::new(pool.clone());

    let temp_root = std::env::temp_dir().join(format!("gaggle_browser_{}", new_id()));
    std::fs::create_dir_all(&temp_root)?;
    let project = proj_repo
        .create("browser-proj", &temp_root.to_string_lossy(), "git")
        .await?;
    let chat = chat_repo
        .create(&project.id, "Browser Chat", "user", "local", "full")
        .await?;
    let run = run_repo.create(&chat.id, None, None).await?;

    let executor = BuiltinToolExecutor::new(pool.clone());

    // navigate
    let nav_args = serde_json::json!({"url": url}).to_string();
    let nav_res = executor
        .execute_tool(
            &run.id,
            &chat,
            &project.root_path,
            "browser.navigate",
            &nav_args,
        )
        .await
        .map_err(anyhow::Error::msg)?;
    let nav_json: serde_json::Value = serde_json::from_str(&nav_res)?;
    let session_id = nav_json["session_id"].as_str().unwrap().to_string();
    assert!(
        nav_json["url"]
            .as_str()
            .unwrap()
            .contains(&addr.to_string())
    );

    // type
    let type_args = serde_json::json!({"selector": "#name", "text": "hello"}).to_string();
    let type_res = executor
        .execute_tool(
            &run.id,
            &chat,
            &project.root_path,
            "browser.type",
            &type_args,
        )
        .await
        .map_err(anyhow::Error::msg)?;
    assert!(type_res.contains("type"));

    // click (button may not navigate; should not error)
    let click_args = serde_json::json!({"selector": "#btn"}).to_string();
    let click_res = executor
        .execute_tool(
            &run.id,
            &chat,
            &project.root_path,
            "browser.click",
            &click_args,
        )
        .await
        .map_err(anyhow::Error::msg)?;
    assert!(click_res.contains("click"));

    // scroll
    let scroll_args = serde_json::json!({"amount": 100}).to_string();
    let scroll_res = executor
        .execute_tool(
            &run.id,
            &chat,
            &project.root_path,
            "browser.scroll",
            &scroll_args,
        )
        .await
        .map_err(anyhow::Error::msg)?;
    assert!(scroll_res.contains("scroll"));

    // screenshot
    let shot_res = executor
        .execute_tool(
            &run.id,
            &chat,
            &project.root_path,
            "browser.screenshot",
            "{}",
        )
        .await
        .map_err(anyhow::Error::msg)?;
    let shot_json: serde_json::Value = serde_json::from_str(&shot_res)?;
    let att_id = shot_json["attachment_id"].as_str().unwrap();
    // verify attachment row
    let att_repo = gaggle_db::repositories::attachments::AttachmentRepository::new(pool.clone());
    let att = att_repo.get_by_id(att_id).await?.expect("attachment");
    assert!(std::path::Path::new(&att.storage_path).exists());
    assert!(!att.sha256.is_empty());

    // extract
    let extract_args = serde_json::json!({"format": "text"}).to_string();
    let extract_res = executor
        .execute_tool(
            &run.id,
            &chat,
            &project.root_path,
            "browser.extract",
            &extract_args,
        )
        .await
        .map_err(anyhow::Error::msg)?;
    let extract_json: serde_json::Value = serde_json::from_str(&extract_res)?;
    assert!(
        extract_json["content"]
            .as_str()
            .unwrap()
            .contains("Hello Browser")
    );

    // verify browser_actions seq
    let browser_repo = gaggle_db::repositories::browsers::BrowserRepository::new(pool.clone());
    let sessions = browser_repo.list_sessions_by_chat(&chat.id).await?;
    assert_eq!(sessions.len(), 1);
    let actions = browser_repo.list_actions(&session_id).await?;
    let seqs: Vec<i64> = actions.iter().map(|a| a.seq).collect();
    // should be 1..=6
    assert_eq!(actions.len(), 6);
    for (i, s) in seqs.iter().enumerate() {
        assert_eq!(*s, (i as i64) + 1);
    }

    let _ = std::fs::remove_dir_all(&temp_root);
    Ok(())
}
