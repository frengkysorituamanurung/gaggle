use anyhow::Result;
use chromiumoxide::browser::BrowserConfig;
use chromiumoxide::page::ScreenshotParams;
use chromiumoxide::{Browser, Page};
use futures::StreamExt;
use gaggle_db::{
    connection::new_id,
    repositories::{attachments::AttachmentRepository, browsers::BrowserRepository},
};
use serde_json::json;
use sha2::{Digest, Sha256};
use sqlx::SqlitePool;
use std::collections::HashMap;
use std::env;
use std::path::PathBuf;
use std::sync::OnceLock;
use tokio::sync::Mutex as TokioMutex;

pub const DEFAULT_BROWSER_TIMEOUT_MS: u64 = 30_000;

#[derive(Debug, thiserror::Error)]
pub enum BrowserToolError {
    #[error("Chromium/Chrome binary not found; install Chromium or set CHROME_BIN")]
    ChromiumUnavailable,
    #[error("invalid url: {0}")]
    InvalidUrl(String),
    #[error("blocked url scheme: {0}")]
    BlockedScheme(String),
    #[error("no active browser session; call browser.navigate first")]
    NoActiveSession,
    #[error("browser launch failed: {0}")]
    LaunchFailed(String),
    #[error("browser action failed: {0}")]
    ActionFailed(String),
    #[error("database error: {0}")]
    Database(String),
}

#[derive(Clone)]
struct ActiveBrowser {
    pool: SqlitePool,
    session_id: String,
    browser: std::sync::Arc<TokioMutex<Browser>>,
    page: Page,
    profile_dir: PathBuf,
    handler_task: std::sync::Arc<tokio::task::JoinHandle<()>>,
}

static REGISTRY: OnceLock<TokioMutex<HashMap<String, ActiveBrowser>>> = OnceLock::new();

fn registry() -> &'static TokioMutex<HashMap<String, ActiveBrowser>> {
    REGISTRY.get_or_init(|| TokioMutex::new(HashMap::new()))
}

fn chat_pools() -> &'static TokioMutex<HashMap<String, SqlitePool>> {
    static CHAT_POOLS: OnceLock<TokioMutex<HashMap<String, SqlitePool>>> = OnceLock::new();
    CHAT_POOLS.get_or_init(|| TokioMutex::new(HashMap::new()))
}

fn session_pools() -> &'static TokioMutex<HashMap<String, SqlitePool>> {
    static SESSION_POOLS: OnceLock<TokioMutex<HashMap<String, SqlitePool>>> = OnceLock::new();
    SESSION_POOLS.get_or_init(|| TokioMutex::new(HashMap::new()))
}

pub fn chromium_executable() -> Option<PathBuf> {
    for var in [
        "CHROME_BIN",
        "CHROMIUM_BIN",
        "CHROMIUM_PATH",
        "GOOGLE_CHROME_BIN",
    ] {
        if let Some(val) = env::var_os(var) {
            let p = PathBuf::from(val);
            if p.is_file() {
                return Some(p);
            }
        }
    }

    let names = [
        "google-chrome",
        "google-chrome-stable",
        "chromium",
        "chromium-browser",
        "chrome",
    ];
    if let Some(path_var) = env::var_os("PATH") {
        for dir in env::split_paths(&path_var) {
            for name in &names {
                let candidate = dir.join(name);
                if candidate.is_file() {
                    return Some(candidate);
                }
            }
        }
    }

    let known = [
        "/Applications/Google Chrome.app/Contents/MacOS/Google Chrome",
        "/Applications/Chromium.app/Contents/MacOS/Chromium",
        "/usr/bin/google-chrome",
        "/usr/bin/google-chrome-stable",
        "/usr/bin/chromium",
        "/usr/bin/chromium-browser",
        "/usr/local/bin/chrome",
        "/usr/local/bin/chromium",
    ];
    for candidate in known {
        let p = PathBuf::from(candidate);
        if p.is_file() {
            return Some(p);
        }
    }
    None
}

pub fn chromium_available() -> bool {
    chromium_executable().is_some()
}

pub fn validate_url(url: &str) -> Result<(), BrowserToolError> {
    let parsed = url::Url::parse(url).map_err(|e| BrowserToolError::InvalidUrl(e.to_string()))?;
    match parsed.scheme() {
        "http" | "https" => {}
        other => return Err(BrowserToolError::BlockedScheme(other.to_string())),
    }
    let host = parsed
        .host_str()
        .ok_or_else(|| BrowserToolError::InvalidUrl("missing host".to_string()))?;
    if let Ok(ip) = host.parse::<std::net::IpAddr>() {
        if ip.is_unspecified() || ip.is_multicast() {
            return Err(BrowserToolError::BlockedScheme(
                "unspecified/multicast".to_string(),
            ));
        }
        // Block common private ranges and link-local.
        match ip {
            std::net::IpAddr::V4(v4) => {
                let octets = v4.octets();
                if octets[0] == 10
                    || (octets[0] == 172 && (16..=31).contains(&octets[1]))
                    || (octets[0] == 192 && octets[1] == 168)
                    || (octets[0] == 169 && octets[1] == 254)
                {
                    return Err(BrowserToolError::BlockedScheme(
                        "private/link-local".to_string(),
                    ));
                }
            }
            std::net::IpAddr::V6(v6) => {
                if v6.is_unicast_link_local() {
                    return Err(BrowserToolError::BlockedScheme("link-local".to_string()));
                }
            }
        }
    }
    // Block cloud metadata hostnames.
    if host.eq_ignore_ascii_case("metadata.google.internal")
        || host.eq_ignore_ascii_case("metadata.azure.internal")
    {
        return Err(BrowserToolError::BlockedScheme("metadata".to_string()));
    }
    Ok(())
}

async fn ensure_session(
    pool: &SqlitePool,
    chat_id: &str,
    run_id: Option<&str>,
) -> Result<ActiveBrowser, BrowserToolError> {
    {
        let guard = registry().lock().await;
        if let Some(existing) = guard.get(chat_id) {
            return Ok(existing.clone());
        }
    }

    let exe = chromium_executable().ok_or(BrowserToolError::ChromiumUnavailable)?;
    let session_id = new_id();
    let profile_dir = env::temp_dir().join(format!("gaggle-browser-{}", session_id));
    std::fs::create_dir_all(&profile_dir)
        .map_err(|e| BrowserToolError::LaunchFailed(e.to_string()))?;

    let config = BrowserConfig::builder()
        .chrome_executable(&exe)
        .user_data_dir(&profile_dir)
        .build()
        .map_err(|e| BrowserToolError::LaunchFailed(e.to_string()))?;

    let (browser, mut handler) = Browser::launch(config)
        .await
        .map_err(|e| BrowserToolError::LaunchFailed(e.to_string()))?;

    let browser = std::sync::Arc::new(TokioMutex::new(browser));
    let handler_task = std::sync::Arc::new(tokio::spawn(async move {
        while handler.next().await.is_some() {}
    }));

    let page = {
        let mut b = browser.lock().await;
        match b.new_page("about:blank").await {
            Ok(p) => p,
            Err(e) => {
                let _ = b.close().await;
                handler_task.abort();
                remove_profile_dir(&profile_dir).await;
                return Err(BrowserToolError::LaunchFailed(e.to_string()));
            }
        }
    };

    let repo = BrowserRepository::new(pool.clone());
    let created = match repo
        .create_session_with_id(
            &session_id,
            chat_id,
            run_id,
            &profile_dir.to_string_lossy(),
            true,
        )
        .await
    {
        Ok(s) => s,
        Err(e) => {
            let mut b = browser.lock().await;
            let _ = b.close().await;
            handler_task.abort();
            remove_profile_dir(&profile_dir).await;
            return Err(BrowserToolError::Database(e.to_string()));
        }
    };

    let active = ActiveBrowser {
        pool: pool.clone(),
        session_id: created.id,
        browser,
        page,
        profile_dir,
        handler_task,
    };

    {
        let mut cp = chat_pools().lock().await;
        cp.insert(chat_id.to_string(), pool.clone());
    }
    {
        let mut sp = session_pools().lock().await;
        sp.insert(active.session_id.clone(), pool.clone());
    }

    let mut guard = registry().lock().await;
    guard.insert(chat_id.to_string(), active.clone());
    Ok(active)
}

async fn get_active_session(chat_id: &str) -> Result<ActiveBrowser, BrowserToolError> {
    let guard = registry().lock().await;
    guard
        .get(chat_id)
        .cloned()
        .ok_or(BrowserToolError::NoActiveSession)
}

fn sha256_hex(bytes: &[u8]) -> String {
    let mut hasher = Sha256::new();
    hasher.update(bytes);
    let digest = hasher.finalize();
    let mut hex = String::with_capacity(digest.len() * 2);
    for byte in digest.as_slice() {
        use std::fmt::Write as _;
        let _ = write!(&mut hex, "{byte:02x}");
    }
    hex
}

fn attachments_root() -> PathBuf {
    env::temp_dir().join("gaggle-attachments")
}

#[allow(clippy::too_many_arguments)]
async fn record_action(
    pool: &SqlitePool,
    active: &ActiveBrowser,
    tool_call_id: Option<&str>,
    action: &str,
    url: Option<&str>,
    selector: Option<&str>,
    value: Option<&str>,
    result: Option<&str>,
    screenshot_attachment_id: Option<&str>,
) -> Result<i64, BrowserToolError> {
    let repo = BrowserRepository::new(pool.clone());
    let ba = repo
        .record_action(
            &active.session_id,
            tool_call_id,
            action,
            url,
            selector,
            value,
            result,
            screenshot_attachment_id,
        )
        .await
        .map_err(|e| BrowserToolError::Database(e.to_string()))?;
    Ok(ba.seq)
}

pub async fn browser_navigate(
    pool: &SqlitePool,
    chat_id: &str,
    run_id: Option<&str>,
    url: &str,
) -> Result<String, BrowserToolError> {
    validate_url(url)?;
    let active = ensure_session(pool, chat_id, run_id).await?;
    let page = &active.page;
    page.goto(url)
        .await
        .map_err(|e| BrowserToolError::ActionFailed(e.to_string()))?;
    let final_url = page
        .url()
        .await
        .ok()
        .flatten()
        .unwrap_or_else(|| url.to_string());
    let title = page.get_title().await.ok().flatten();

    let result = json!({"url": final_url, "title": title});
    let seq = record_action(
        pool,
        &active,
        None,
        "navigate",
        Some(&final_url),
        None,
        None,
        Some(&result.to_string()),
        None,
    )
    .await?;

    Ok(json!({
        "session_id": active.session_id,
        "action": "navigate",
        "seq": seq,
        "url": final_url,
        "title": title,
    })
    .to_string())
}

pub async fn browser_click(
    pool: &SqlitePool,
    chat_id: &str,
    run_id: Option<&str>,
    selector: &str,
) -> Result<String, BrowserToolError> {
    let _ = run_id;
    let active = get_active_session(chat_id).await?;
    let page = &active.page;
    let element = page
        .find_element(selector)
        .await
        .map_err(|e| BrowserToolError::ActionFailed(e.to_string()))?;
    element
        .click()
        .await
        .map_err(|e| BrowserToolError::ActionFailed(e.to_string()))?;
    let _ = tokio::time::timeout(
        std::time::Duration::from_secs(5),
        page.wait_for_navigation(),
    )
    .await;
    let final_url = page.url().await.ok().flatten();

    let result = json!({"selector": selector, "url": final_url});
    let seq = record_action(
        pool,
        &active,
        None,
        "click",
        final_url.as_deref(),
        Some(selector),
        None,
        Some(&result.to_string()),
        None,
    )
    .await?;

    Ok(json!({
        "session_id": active.session_id,
        "action": "click",
        "seq": seq,
        "selector": selector,
        "url": final_url,
    })
    .to_string())
}

pub async fn browser_type(
    pool: &SqlitePool,
    chat_id: &str,
    run_id: Option<&str>,
    selector: &str,
    text: &str,
) -> Result<String, BrowserToolError> {
    let _ = run_id;
    let active = get_active_session(chat_id).await?;
    let page = &active.page;
    let element = page
        .find_element(selector)
        .await
        .map_err(|e| BrowserToolError::ActionFailed(e.to_string()))?;
    let _ = element.click().await;
    element
        .type_str(text)
        .await
        .map_err(|e| BrowserToolError::ActionFailed(e.to_string()))?;

    let result = json!({"selector": selector, "typed": text.len()});
    let seq = record_action(
        pool,
        &active,
        None,
        "type",
        page.url().await.ok().flatten().as_deref(),
        Some(selector),
        Some(text),
        Some(&result.to_string()),
        None,
    )
    .await?;

    Ok(json!({
        "session_id": active.session_id,
        "action": "type",
        "seq": seq,
        "selector": selector,
    })
    .to_string())
}

pub async fn browser_scroll(
    pool: &SqlitePool,
    chat_id: &str,
    run_id: Option<&str>,
    selector: Option<&str>,
    amount: Option<i64>,
    x: Option<i64>,
    y: Option<i64>,
) -> Result<String, BrowserToolError> {
    let _ = run_id;
    let active = get_active_session(chat_id).await?;
    let page = &active.page;

    if let Some(sel) = selector {
        let element = page
            .find_element(sel)
            .await
            .map_err(|e| BrowserToolError::ActionFailed(e.to_string()))?;
        element
            .scroll_into_view()
            .await
            .map_err(|e| BrowserToolError::ActionFailed(e.to_string()))?;
    } else {
        let dx = x.unwrap_or(0);
        let dy = amount.or(y).unwrap_or(500);
        let expr = format!("window.scrollBy({}, {})", dx, dy);
        page.evaluate(expr)
            .await
            .map_err(|e| BrowserToolError::ActionFailed(e.to_string()))?;
    }

    let pos = page
        .evaluate("({x: window.scrollX, y: window.scrollY})")
        .await
        .ok()
        .and_then(|r| r.into_value::<serde_json::Value>().ok())
        .unwrap_or_else(|| json!({"x": 0, "y": 0}));

    let result = json!({"scroll": pos});
    let seq = record_action(
        pool,
        &active,
        None,
        "scroll",
        page.url().await.ok().flatten().as_deref(),
        selector,
        amount.map(|v| v.to_string()).as_deref(),
        Some(&result.to_string()),
        None,
    )
    .await?;

    Ok(json!({
        "session_id": active.session_id,
        "action": "scroll",
        "seq": seq,
        "position": pos,
    })
    .to_string())
}

pub async fn browser_screenshot(
    pool: &SqlitePool,
    chat_id: &str,
    run_id: Option<&str>,
) -> Result<String, BrowserToolError> {
    let _ = run_id;
    let active = get_active_session(chat_id).await?;
    let page = &active.page;
    let bytes = page
        .screenshot(ScreenshotParams::default())
        .await
        .map_err(|e| BrowserToolError::ActionFailed(e.to_string()))?;

    let root = attachments_root();
    std::fs::create_dir_all(&root).map_err(|e| BrowserToolError::ActionFailed(e.to_string()))?;
    let file_name = format!("screenshot-{}.png", new_id());
    let storage_path = root.join(&file_name);
    std::fs::write(&storage_path, &bytes)
        .map_err(|e| BrowserToolError::ActionFailed(e.to_string()))?;
    let sha = sha256_hex(&bytes);

    let attachment = AttachmentRepository::new(pool.clone())
        .create(
            chat_id,
            &file_name,
            "image/png",
            bytes.len() as i64,
            &sha,
            &storage_path.to_string_lossy(),
        )
        .await
        .map_err(|e| BrowserToolError::Database(e.to_string()))?;

    let result = json!({"attachment_id": attachment.id, "size_bytes": bytes.len()});
    let seq = record_action(
        pool,
        &active,
        None,
        "screenshot",
        page.url().await.ok().flatten().as_deref(),
        None,
        None,
        Some(&result.to_string()),
        Some(&attachment.id),
    )
    .await?;

    Ok(json!({
        "session_id": active.session_id,
        "action": "screenshot",
        "seq": seq,
        "attachment_id": attachment.id,
        "storage_path": attachment.storage_path,
        "sha256": attachment.sha256,
    })
    .to_string())
}

pub async fn browser_extract(
    pool: &SqlitePool,
    chat_id: &str,
    run_id: Option<&str>,
    format: &str,
) -> Result<String, BrowserToolError> {
    let _ = run_id;
    let active = get_active_session(chat_id).await?;
    let page = &active.page;

    let content = if format == "html" {
        page.content()
            .await
            .map_err(|e| BrowserToolError::ActionFailed(e.to_string()))?
    } else {
        page.evaluate(
            "document.body ? document.body.innerText : document.documentElement.innerText",
        )
        .await
        .ok()
        .and_then(|r| r.into_value::<String>().ok())
        .unwrap_or_default()
    };
    let title = page.get_title().await.ok().flatten();
    let url = page.url().await.ok().flatten();

    let result = json!({
        "url": url,
        "title": title,
        "format": format,
        "content": content,
    });
    let seq = record_action(
        pool,
        &active,
        None,
        "extract",
        url.as_deref(),
        None,
        Some(format),
        Some(&result.to_string()),
        None,
    )
    .await?;

    Ok(json!({
        "session_id": active.session_id,
        "action": "extract",
        "seq": seq,
        "url": url,
        "title": title,
        "content": content,
    })
    .to_string())
}

async fn close_active_browser(active: ActiveBrowser) {
    {
        let mut b = active.browser.lock().await;
        let close_result = tokio::time::timeout(std::time::Duration::from_secs(5), b.close()).await;
        if !matches!(close_result, Ok(Ok(_))) {
            let _ = b.kill().await;
        }
        // `Browser::close` returns once Chromium acknowledges the close request, but
        // the OS process may still be shutting down and holding files in the profile
        // directory. Wait for the process to exit (with a fallback kill) before
        // removing the directory.
        let wait_result = tokio::time::timeout(std::time::Duration::from_secs(10), b.wait()).await;
        if wait_result.is_err() {
            let _ = b.kill().await;
            let _ = b.wait().await;
        }
    }
    active.handler_task.abort();
    remove_profile_dir(&active.profile_dir).await;
    let repo = BrowserRepository::new(active.pool.clone());
    let _ = repo.close_session(&active.session_id).await;
}

async fn remove_profile_dir(path: &std::path::Path) {
    for _ in 0..40 {
        if !path.exists() {
            return;
        }
        if std::fs::remove_dir_all(path).is_ok() {
            return;
        }
        tokio::time::sleep(std::time::Duration::from_millis(50)).await;
    }
    let _ = std::fs::remove_dir_all(path);
}

pub async fn close_browser(chat_id: &str) -> Result<(), BrowserToolError> {
    // Close the live Chromium instance for this chat, if any.
    let active = {
        let mut guard = registry().lock().await;
        guard.remove(chat_id)
    };
    if let Some(active) = active {
        close_active_browser(active).await;
    }

    // Mark every DB session for this chat as closed and remove profile dirs.
    let pool = {
        let guard = chat_pools().lock().await;
        guard.get(chat_id).cloned()
    };
    if let Some(pool) = pool {
        let repo = BrowserRepository::new(pool);
        let sessions = repo
            .list_sessions_by_chat(chat_id)
            .await
            .map_err(|e| BrowserToolError::Database(e.to_string()))?;
        for session in sessions {
            if session.status != "closed" {
                let _ = repo.close_session(&session.id).await;
            }
            remove_profile_dir(std::path::Path::new(&session.profile_dir)).await;
        }
    }
    Ok(())
}

pub async fn close_browser_with_pool(
    pool: &SqlitePool,
    chat_id: &str,
) -> Result<(), BrowserToolError> {
    {
        let mut guard = chat_pools().lock().await;
        guard.insert(chat_id.to_string(), pool.clone());
    }
    close_browser(chat_id).await
}

pub async fn close_browser_session(session_id: &str) -> Result<(), BrowserToolError> {
    // Close the live Chromium instance for this session, if any.
    let active = {
        let mut guard = registry().lock().await;
        let key = guard
            .iter()
            .find(|(_, a)| a.session_id == session_id)
            .map(|(k, _)| k.clone());
        if let Some(key) = key {
            guard.remove(&key)
        } else {
            None
        }
    };
    if let Some(active) = active {
        close_active_browser(active).await;
    }

    // Mark the DB session as closed and remove its profile dir.
    let pool = {
        let guard = session_pools().lock().await;
        guard.get(session_id).cloned()
    };
    if let Some(pool) = pool {
        let repo = BrowserRepository::new(pool);
        let _ = repo.close_session(session_id).await;
        if let Ok(Some(session)) = repo.get_session(session_id).await {
            remove_profile_dir(std::path::Path::new(&session.profile_dir)).await;
        }
    }
    Ok(())
}

pub async fn close_browser_session_with_pool(
    pool: &SqlitePool,
    session_id: &str,
) -> Result<(), BrowserToolError> {
    {
        let mut guard = session_pools().lock().await;
        guard.insert(session_id.to_string(), pool.clone());
    }
    close_browser_session(session_id).await
}

pub async fn close_all_browsers() -> Result<(), BrowserToolError> {
    // Close every live Chromium instance.
    let actives: Vec<ActiveBrowser> = {
        let mut guard = registry().lock().await;
        guard.drain().map(|(_, v)| v).collect()
    };
    for active in actives {
        close_active_browser(active).await;
    }

    // Mark any remaining DB rows as closed and remove profile dirs.
    let pools: Vec<SqlitePool> = {
        let guard = session_pools().lock().await;
        guard.values().cloned().collect()
    };
    for pool in pools {
        let repo = BrowserRepository::new(pool);
        let sessions = match repo.list_running_sessions().await {
            Ok(s) => s,
            Err(_) => continue,
        };
        for session in sessions {
            if session.status != "closed" {
                let _ = repo.close_session(&session.id).await;
            }
            remove_profile_dir(std::path::Path::new(&session.profile_dir)).await;
        }
    }
    Ok(())
}

pub async fn close_all_browsers_with_pool(pool: &SqlitePool) -> Result<(), BrowserToolError> {
    // Mark all running rows closed using provided pool.

    // Mark all running rows closed using provided pool.
    let repo = BrowserRepository::new(pool.clone());
    let sessions = repo.list_running_sessions().await.unwrap_or_default();
    // Close active instances first.
    let actives: Vec<ActiveBrowser> = {
        let mut guard = registry().lock().await;
        guard.drain().map(|(_, v)| v).collect()
    };
    for active in actives {
        close_active_browser(active).await;
    }
    for session in sessions {
        if session.status != "closed" {
            let _ = repo.close_session(&session.id).await;
        }
        remove_profile_dir(std::path::Path::new(&session.profile_dir)).await;
    }
    Ok(())
}
