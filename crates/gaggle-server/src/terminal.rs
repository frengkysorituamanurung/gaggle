use crate::AppState;
use axum::{
    Router,
    extract::{
        Path, Query, State, WebSocketUpgrade,
        ws::{Message, WebSocket},
    },
    http::StatusCode,
    response::IntoResponse,
    routing::get,
};
use futures::{SinkExt, StreamExt};
use gaggle_core::WorkspacePathPolicy;
use gaggle_db::{
    models::TerminalSession,
    repositories::{projects::ProjectRepository, terminal_sessions::TerminalSessionRepository},
};
use gaggle_tools::{TerminalPty, TerminalPtyConfig, TerminalPtyError};
use serde::Deserialize;
use sqlx::SqlitePool;
use std::collections::HashMap;
use std::sync::Arc;
use std::sync::atomic::{AtomicI64, Ordering};
use tokio::sync::RwLock;

pub const DEFAULT_WALL_CLOCK_TIMEOUT_MS: u64 = 60 * 60 * 1000; // 1 hour
pub const DEFAULT_IDLE_TIMEOUT_MS: u64 = 30 * 60 * 1000; // 30 min

#[derive(Clone)]
pub struct TerminalManager {
    pool: SqlitePool,
    sessions: Arc<RwLock<HashMap<String, Arc<TerminalSessionHandle>>>>,
}

pub struct TerminalSessionHandle {
    pub id: String,
    pub project_id: String,
    pub chat_id: Option<String>,
    pub worktree_id: Option<String>,
    pub cwd: String,
    pub shell: String,
    pub cols: Arc<AtomicI64>,
    pub rows: Arc<AtomicI64>,
    pub pty: Arc<TerminalPty>,
    pub last_activity_ms: Arc<AtomicI64>,
    pub started_at: i64,
    pub wall_clock_timeout_ms: u64,
    pub idle_timeout_ms: u64,
}

impl TerminalManager {
    pub fn new(pool: SqlitePool) -> Self {
        Self {
            pool,
            sessions: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    #[allow(clippy::too_many_arguments)]
    pub async fn create_session(
        &self,
        project_id: String,
        chat_id: Option<String>,
        worktree_id: Option<String>,
        cwd: Option<String>,
        shell: Option<String>,
        cols: i64,
        rows: i64,
        idle_timeout_ms: u64,
        wall_clock_timeout_ms: u64,
    ) -> Result<TerminalSession, (StatusCode, String)> {
        // Validate project.
        let proj_repo = ProjectRepository::new(self.pool.clone());
        let project = proj_repo
            .get_by_id(&project_id)
            .await
            .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?
            .ok_or_else(|| {
                (
                    StatusCode::NOT_FOUND,
                    format!("Project {} not found", project_id),
                )
            })?;

        let policy = WorkspacePathPolicy::new(&project.root_path)
            .map_err(|e| (StatusCode::BAD_REQUEST, e.to_string()))?;

        let cwd_path = match cwd {
            Some(rel) if !rel.trim().is_empty() => policy
                .resolve_existing_for_read(rel.trim())
                .map_err(|e| (StatusCode::BAD_REQUEST, e.to_string()))?,
            _ => policy.canonical_root().to_path_buf(),
        };
        if !cwd_path.is_dir() {
            return Err((
                StatusCode::BAD_REQUEST,
                format!(
                    "Working directory '{}' is not a directory",
                    cwd_path.display()
                ),
            ));
        }

        let shell = shell
            .filter(|s| !s.trim().is_empty())
            .unwrap_or_else(|| std::env::var("SHELL").unwrap_or_else(|_| "/bin/sh".to_string()));

        let c = if cols <= 0 { 80 } else { cols as u16 };
        let r = if rows <= 0 { 24 } else { rows as u16 };

        let config = TerminalPtyConfig {
            shell: shell.clone(),
            cwd: cwd_path.clone(),
            cols: c,
            rows: r,
        };

        let pty = tokio::task::spawn_blocking(move || TerminalPty::spawn(config))
            .await
            .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?
            .map_err(|e: TerminalPtyError| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;

        let pid = pty.process_id().map(|p| p as i64);
        let cwd_str = cwd_path.to_string_lossy().to_string();
        let now = gaggle_db::connection::now_ms();

        let repo = TerminalSessionRepository::new(self.pool.clone());
        let session = repo
            .create(
                &project_id,
                chat_id.as_deref(),
                worktree_id.as_deref(),
                &cwd_str,
                &shell,
                cols.max(1),
                rows.max(1),
                pid,
            )
            .await
            .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;

        let handle = Arc::new(TerminalSessionHandle {
            id: session.id.clone(),
            project_id: project_id.clone(),
            chat_id: chat_id.clone(),
            worktree_id: worktree_id.clone(),
            cwd: cwd_str.clone(),
            shell: shell.clone(),
            cols: Arc::new(AtomicI64::new(cols.max(1))),
            rows: Arc::new(AtomicI64::new(rows.max(1))),
            pty: Arc::new(pty),
            last_activity_ms: Arc::new(AtomicI64::new(now)),
            started_at: now,
            wall_clock_timeout_ms: wall_clock_timeout_ms.max(100),
            idle_timeout_ms: idle_timeout_ms.max(100),
        });

        self.sessions
            .write()
            .await
            .insert(session.id.clone(), handle.clone());

        // Output activity pump.
        let last_activity = handle.last_activity_ms.clone();
        let mut rx = handle.pty.subscribe();
        tokio::spawn(async move {
            while rx.recv().await.is_ok() {
                last_activity.store(gaggle_db::connection::now_ms(), Ordering::SeqCst);
            }
        });

        // Lifecycle monitor.
        let pool = self.pool.clone();
        let sessions = self.sessions.clone();
        let sid = session.id.clone();
        let handle_clone = handle.clone();
        tokio::spawn(async move {
            loop {
                tokio::time::sleep(std::time::Duration::from_secs(1)).await;
                if handle_clone.pty.has_exited() {
                    let code = handle_clone.pty.exit_code().map(|c| c as i64);
                    let ended = gaggle_db::connection::now_ms();
                    let repo = TerminalSessionRepository::new(pool.clone());
                    let _ = repo.mark_exited(&sid, code, ended).await;
                    // keep handle for scrollback; mark exited in memory not needed.
                    break;
                }
                let now = gaggle_db::connection::now_ms();
                let elapsed = (now - handle_clone.started_at) as u64;
                let idle = (now - handle_clone.last_activity_ms.load(Ordering::SeqCst)) as u64;
                if elapsed >= handle_clone.wall_clock_timeout_ms
                    || idle >= handle_clone.idle_timeout_ms
                {
                    handle_clone.pty.kill();
                    // wait a bit for exit
                    tokio::time::sleep(std::time::Duration::from_millis(500)).await;
                    let code = handle_clone.pty.exit_code().map(|c| c as i64);
                    let ended = gaggle_db::connection::now_ms();
                    let repo = TerminalSessionRepository::new(pool.clone());
                    let _ = repo.mark_exited(&sid, code, ended).await;
                    break;
                }
            }
            // Remove from map after a grace period? Keep for scrollback access; but cleanup memory.
            // We'll leave handle in map; status in DB reflects exited.
            let _ = sessions;
        });

        Ok(session)
    }

    pub async fn get_handle(&self, id: &str) -> Option<Arc<TerminalSessionHandle>> {
        self.sessions.read().await.get(id).cloned()
    }

    pub async fn get_session(
        &self,
        id: &str,
    ) -> Result<Option<TerminalSession>, (StatusCode, String)> {
        let repo = TerminalSessionRepository::new(self.pool.clone());
        repo.get_by_id(id)
            .await
            .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))
    }

    pub async fn list_by_project(
        &self,
        project_id: &str,
    ) -> Result<Vec<TerminalSession>, (StatusCode, String)> {
        let repo = TerminalSessionRepository::new(self.pool.clone());
        repo.list_by_project(project_id)
            .await
            .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))
    }

    pub async fn resize(
        &self,
        id: &str,
        cols: i64,
        rows: i64,
    ) -> Result<TerminalSession, (StatusCode, String)> {
        let handle = self.get_handle(id).await.ok_or_else(|| {
            (
                StatusCode::NOT_FOUND,
                "Terminal session not found".to_string(),
            )
        })?;
        let c = cols.max(1) as u16;
        let r = rows.max(1) as u16;
        handle
            .pty
            .resize(c, r)
            .map_err(|e| (StatusCode::BAD_REQUEST, e.to_string()))?;
        handle.cols.store(cols.max(1), Ordering::SeqCst);
        handle.rows.store(rows.max(1), Ordering::SeqCst);
        let repo = TerminalSessionRepository::new(self.pool.clone());
        repo.update_dimensions(id, cols.max(1), rows.max(1))
            .await
            .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;
        repo.get_by_id(id)
            .await
            .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?
            .ok_or_else(|| {
                (
                    StatusCode::NOT_FOUND,
                    "Terminal session not found".to_string(),
                )
            })
    }

    pub async fn write_input(&self, id: &str, data: &[u8]) -> Result<(), (StatusCode, String)> {
        let handle = self.get_handle(id).await.ok_or_else(|| {
            (
                StatusCode::NOT_FOUND,
                "Terminal session not found".to_string(),
            )
        })?;
        handle
            .last_activity_ms
            .store(gaggle_db::connection::now_ms(), Ordering::SeqCst);
        handle
            .pty
            .write(data)
            .map_err(|e| (StatusCode::BAD_REQUEST, e.to_string()))?;
        Ok(())
    }

    pub async fn close(&self, id: &str) -> Result<(), (StatusCode, String)> {
        if let Some(handle) = self.get_handle(id).await {
            handle.pty.kill();
        }
        // Update DB if still running.
        let repo = TerminalSessionRepository::new(self.pool.clone());
        if let Ok(Some(s)) = repo.get_by_id(id).await
            && s.status == "running"
        {
            let _ = repo
                .mark_exited(id, None, gaggle_db::connection::now_ms())
                .await;
        }
        // Remove from memory.
        self.sessions.write().await.remove(id);
        Ok(())
    }

    pub async fn scrollback(&self, id: &str) -> Result<Vec<u8>, (StatusCode, String)> {
        let handle = self.get_handle(id).await.ok_or_else(|| {
            (
                StatusCode::NOT_FOUND,
                "Terminal session not found".to_string(),
            )
        })?;
        Ok(handle.pty.scrollback())
    }

    pub async fn shutdown_all(&self) {
        let handles: Vec<Arc<TerminalSessionHandle>> =
            self.sessions.read().await.values().cloned().collect();
        for h in handles {
            h.pty.kill();
        }
        // Mark DB running sessions exited.
        let repo = TerminalSessionRepository::new(self.pool.clone());
        if let Ok(running) = repo.list_running().await {
            for s in running {
                let _ = repo
                    .mark_exited(&s.id, None, gaggle_db::connection::now_ms())
                    .await;
            }
        }
        self.sessions.write().await.clear();
    }
}

// Request/response types
#[derive(Debug, Deserialize)]
pub struct CreateTerminalSessionRequest {
    pub project_id: String,
    pub chat_id: Option<String>,
    pub worktree_id: Option<String>,
    pub cwd: Option<String>,
    pub shell: Option<String>,
    pub cols: Option<i64>,
    pub rows: Option<i64>,
    pub idle_timeout_ms: Option<u64>,
    pub wall_clock_timeout_ms: Option<u64>,
    pub timeout_ms: Option<u64>,
}

#[derive(Debug, Deserialize)]
pub struct ResizeTerminalRequest {
    pub cols: i64,
    pub rows: i64,
}

#[derive(Debug, Deserialize)]
pub struct TerminalInputRequest {
    pub data: String,
}

// Axum handlers
pub async fn list_terminal_sessions_handler(
    State(state): State<AppState>,
    Query(q): Query<HashMap<String, String>>,
) -> Result<axum::Json<Vec<TerminalSession>>, (StatusCode, String)> {
    let project_id = q.get("project_id").cloned().unwrap_or_default();
    if project_id.is_empty() {
        return Err((
            StatusCode::BAD_REQUEST,
            "project_id is required".to_string(),
        ));
    }
    let sessions = state.terminal_manager.list_by_project(&project_id).await?;
    Ok(axum::Json(sessions))
}

pub async fn create_terminal_session_handler(
    State(state): State<AppState>,
    axum::Json(req): axum::Json<CreateTerminalSessionRequest>,
) -> Result<(StatusCode, axum::Json<TerminalSession>), (StatusCode, String)> {
    let session = state
        .terminal_manager
        .create_session(
            req.project_id,
            req.chat_id,
            req.worktree_id,
            req.cwd,
            req.shell,
            req.cols.unwrap_or(80),
            req.rows.unwrap_or(24),
            req.idle_timeout_ms
                .or(req.timeout_ms)
                .unwrap_or(DEFAULT_IDLE_TIMEOUT_MS),
            req.wall_clock_timeout_ms
                .or(req.timeout_ms)
                .unwrap_or(DEFAULT_WALL_CLOCK_TIMEOUT_MS),
        )
        .await?;
    Ok((StatusCode::CREATED, axum::Json(session)))
}

pub async fn get_terminal_session_handler(
    State(state): State<AppState>,
    Path(id): Path<String>,
) -> Result<axum::Json<TerminalSession>, (StatusCode, String)> {
    let session = state
        .terminal_manager
        .get_session(&id)
        .await?
        .ok_or_else(|| {
            (
                StatusCode::NOT_FOUND,
                "Terminal session not found".to_string(),
            )
        })?;
    Ok(axum::Json(session))
}

pub async fn resize_terminal_session_handler(
    State(state): State<AppState>,
    Path(id): Path<String>,
    axum::Json(req): axum::Json<ResizeTerminalRequest>,
) -> Result<axum::Json<TerminalSession>, (StatusCode, String)> {
    let session = state
        .terminal_manager
        .resize(&id, req.cols, req.rows)
        .await?;
    Ok(axum::Json(session))
}

pub async fn input_terminal_session_handler(
    State(state): State<AppState>,
    Path(id): Path<String>,
    axum::Json(req): axum::Json<TerminalInputRequest>,
) -> Result<StatusCode, (StatusCode, String)> {
    state
        .terminal_manager
        .write_input(&id, req.data.as_bytes())
        .await?;
    Ok(StatusCode::NO_CONTENT)
}

pub async fn close_terminal_session_handler(
    State(state): State<AppState>,
    Path(id): Path<String>,
) -> Result<StatusCode, (StatusCode, String)> {
    state.terminal_manager.close(&id).await?;
    Ok(StatusCode::NO_CONTENT)
}

pub async fn scrollback_terminal_session_handler(
    State(state): State<AppState>,
    Path(id): Path<String>,
) -> Result<axum::Json<String>, (StatusCode, String)> {
    let bytes = state.terminal_manager.scrollback(&id).await?;
    let text = String::from_utf8_lossy(&bytes).to_string();
    Ok(axum::Json(text))
}

pub async fn terminal_session_ws_handler(
    State(state): State<AppState>,
    Path(id): Path<String>,
    ws: WebSocketUpgrade,
) -> impl IntoResponse {
    ws.on_upgrade(move |socket| handle_terminal_socket(state, id, socket))
}

async fn handle_terminal_socket(state: AppState, id: String, socket: WebSocket) {
    let handle = match state.terminal_manager.get_handle(&id).await {
        Some(h) => h,
        None => {
            // Try to see if session exists in DB but no handle (exited); close.
            return;
        }
    };

    let (mut sender, mut receiver) = socket.split();
    let mut output_rx = handle.pty.subscribe();

    // Task: forward PTY output to WebSocket.
    let send_task = tokio::spawn(async move {
        while let Ok(chunk) = output_rx.recv().await {
            if sender.send(Message::Binary(chunk.into())).await.is_err() {
                break;
            }
        }
    });

    // Receive from WebSocket.
    while let Some(Ok(msg)) = receiver.next().await {
        match msg {
            Message::Text(text) => {
                // Try parse JSON control.
                if let Ok(v) = serde_json::from_str::<serde_json::Value>(&text) {
                    match v.get("type").and_then(|t| t.as_str()) {
                        Some("resize") => {
                            let cols = v.get("cols").and_then(|x| x.as_i64()).unwrap_or(0);
                            let rows = v.get("rows").and_then(|x| x.as_i64()).unwrap_or(0);
                            let _ = state.terminal_manager.resize(&id, cols, rows).await;
                        }
                        Some("input") => {
                            if let Some(data) = v.get("data").and_then(|x| x.as_str()) {
                                let _ = state
                                    .terminal_manager
                                    .write_input(&id, data.as_bytes())
                                    .await;
                            }
                        }
                        Some("ping") => {}
                        _ => {
                            // Treat raw text as input.
                            let _ = state
                                .terminal_manager
                                .write_input(&id, text.as_bytes())
                                .await;
                        }
                    }
                } else {
                    let _ = state
                        .terminal_manager
                        .write_input(&id, text.as_bytes())
                        .await;
                }
            }
            Message::Binary(bin) => {
                let _ = state.terminal_manager.write_input(&id, &bin).await;
            }
            Message::Close(_) => break,
            _ => {}
        }
    }

    send_task.abort();
    // Do not kill session on disconnect; keep running.
}

pub fn terminal_routes() -> Router<AppState> {
    Router::new()
        .route(
            "/api/terminal-sessions",
            get(list_terminal_sessions_handler).post(create_terminal_session_handler),
        )
        .route(
            "/api/terminal-sessions/{id}",
            get(get_terminal_session_handler).delete(close_terminal_session_handler),
        )
        .route(
            "/api/terminal-sessions/{id}/resize",
            axum::routing::post(resize_terminal_session_handler)
                .put(resize_terminal_session_handler),
        )
        .route(
            "/api/terminal-sessions/{id}/input",
            axum::routing::post(input_terminal_session_handler),
        )
        .route(
            "/api/terminal-sessions/{id}/scrollback",
            get(scrollback_terminal_session_handler),
        )
        .route(
            "/api/terminal-sessions/{id}/ws",
            get(terminal_session_ws_handler),
        )
        .route(
            "/api/terminal-sessions/{id}/stream",
            get(terminal_session_ws_handler),
        )
        .route(
            "/api/terminal-sessions/{id}/websocket",
            get(terminal_session_ws_handler),
        )
        .route(
            "/api/terminal-sessions/{id}/close",
            axum::routing::post(close_terminal_session_handler),
        )
}
