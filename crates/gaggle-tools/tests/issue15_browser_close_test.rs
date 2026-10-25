use anyhow::Result;
use gaggle_core::ToolExecutor;
use gaggle_db::{
    connect_in_memory,
    connection::new_id,
    repositories::{
        browsers::BrowserRepository, chats::ChatRepository, projects::ProjectRepository,
        runs::RunRepository,
    },
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
                let body = "<html><head><title>T</title></head><body>hi</body></html>";
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

async fn setup() -> Result<(sqlx::SqlitePool, String, String)> {
    let (addr, _server) = spawn_test_server().await?;
    let url = format!("http://{}/", addr);

    let pool = connect_in_memory().await?;
    register_builtin_extensions(&pool).await?;

    let proj_repo = ProjectRepository::new(pool.clone());
    let chat_repo = ChatRepository::new(pool.clone());
    let run_repo = RunRepository::new(pool.clone());

    let temp_root = std::env::temp_dir().join(format!("gaggle_browser_close_{}", new_id()));
    std::fs::create_dir_all(&temp_root)?;
    let project = proj_repo
        .create("browser-close-proj", &temp_root.to_string_lossy(), "git")
        .await?;
    let chat = chat_repo
        .create(&project.id, "Browser Close Chat", "user", "local", "full")
        .await?;
    let run = run_repo.create(&chat.id, None, None).await?;

    let executor = BuiltinToolExecutor::new(pool.clone());
    let nav_args = serde_json::json!({"url": url}).to_string();
    let res = match executor
        .execute_tool(
            &run.id,
            &chat,
            &project.root_path,
            "browser.navigate",
            &nav_args,
        )
        .await
    {
        Ok(r) => r,
        Err(e) => {
            let msg = e.to_string();
            if msg.contains("browser launch failed")
                || msg.contains("Timeout while resolving websocket")
                || msg.contains("Chromium/Chrome not available")
            {
                // Return empty marker by panicking? Instead skip via Ok early not possible; use sentinel.
                eprintln!("SKIP: Chromium not usable: {}", msg);
                return Err(anyhow::anyhow!("skip"));
            }
            return Err(anyhow::anyhow!(e));
        }
    };
    let nav_json: serde_json::Value = serde_json::from_str(&res)?;
    let session_id = nav_json["session_id"].as_str().unwrap().to_string();
    let _ = session_id;
    // store temp_root? ignore
    let _ = temp_root;
    Ok((pool, chat.id, session_id))
}

#[tokio::test]
async fn test_close_browser_marks_db_and_removes_profile() -> Result<()> {
    if !browser_tools::chromium_available() {
        eprintln!("SKIP: Chromium not available");
        return Ok(());
    }
    let (pool, chat_id, session_id) = match setup().await {
        Ok(v) => v,
        Err(e) if e.to_string() == "skip" => return Ok(()),
        Err(e) => return Err(e),
    };

    let repo = BrowserRepository::new(pool.clone());
    let sessions = repo.list_sessions_by_chat(&chat_id).await?;
    assert_eq!(sessions.len(), 1);
    let profile_dir = std::path::PathBuf::from(&sessions[0].profile_dir);
    assert!(profile_dir.exists());

    browser_tools::close_browser(&chat_id).await.unwrap();

    let s = repo.get_session(&session_id).await?.unwrap();
    assert_eq!(s.status, "closed");
    assert!(s.ended_at.is_some());
    assert!(!profile_dir.exists());
    Ok(())
}

#[tokio::test]
async fn test_close_browser_session_with_pool() -> Result<()> {
    if !browser_tools::chromium_available() {
        eprintln!("SKIP: Chromium not available");
        return Ok(());
    }
    let (pool, _chat_id, session_id) = match setup().await {
        Ok(v) => v,
        Err(e) if e.to_string() == "skip" => return Ok(()),
        Err(e) => return Err(e),
    };

    let repo = BrowserRepository::new(pool.clone());
    let session = repo.get_session(&session_id).await?.unwrap();
    let profile_dir = std::path::PathBuf::from(&session.profile_dir);
    assert!(profile_dir.exists());

    browser_tools::close_browser_session_with_pool(&pool, &session_id)
        .await
        .unwrap();

    let s = repo.get_session(&session_id).await?.unwrap();
    assert_eq!(s.status, "closed");
    assert!(s.ended_at.is_some());
    assert!(!profile_dir.exists());
    Ok(())
}

#[tokio::test]
async fn test_close_all_browsers() -> Result<()> {
    if !browser_tools::chromium_available() {
        eprintln!("SKIP: Chromium not available");
        return Ok(());
    }
    let (pool, _chat_id, session_id) = match setup().await {
        Ok(v) => v,
        Err(e) if e.to_string() == "skip" => return Ok(()),
        Err(e) => return Err(e),
    };

    let repo = BrowserRepository::new(pool.clone());
    browser_tools::close_all_browsers_with_pool(&pool)
        .await
        .unwrap();

    let s = repo.get_session(&session_id).await?.unwrap();
    assert_eq!(s.status, "closed");
    assert!(s.ended_at.is_some());
    Ok(())
}
