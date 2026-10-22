use anyhow::Result;
use axum::{
    Json, Router,
    extract::{Path, Query, State},
    http::StatusCode,
    response::Html,
    routing::{get, post},
};
use gaggle_core::{DiagnosticReport, RunEngine, WorkspacePathPolicy, collect_diagnostics};
use gaggle_db::{
    default_db_path,
    models::{Chat, Project},
    repositories::{
        chats::{ChatExportBundle, ChatRepository},
        projects::ProjectRepository,
    },
};
use gaggle_tools::{
    DEFAULT_MAX_DIFF_BYTES, DEFAULT_MAX_LIST_ENTRIES, FsEntry, GitDiffOutput, GitWorkspaceStatus,
    fs_list, git_diff, git_status, register_builtin_extensions,
};
use serde::Deserialize;
use sqlx::SqlitePool;
use std::{net::SocketAddr, path::PathBuf};
use tower_http::cors::CorsLayer;
use tracing::info;

#[derive(Clone)]
pub struct AppState {
    pub pool: SqlitePool,
    pub db_path: PathBuf,
}

#[derive(Debug, Deserialize)]
pub struct CreateProjectRequest {
    pub name: String,
    pub root_path: String,
    pub vcs_kind: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct CreateChatRequest {
    pub project_id: String,
    pub title: String,
    pub mode: Option<String>,
    pub permission_mode: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct ProjectPathQuery {
    pub path: Option<String>,
}

pub fn build_router(state: AppState) -> Router {
    Router::new()
        .route("/", get(index_handler))
        .route("/api/health", get(health_handler))
        .route("/api/diagnostics", get(diagnostics_handler))
        .route("/api/projects", get(list_projects).post(create_project))
        .route("/api/projects/{id}/files", get(list_project_files))
        .route("/api/projects/{id}/git-status", get(project_git_status))
        .route("/api/projects/{id}/diff", get(project_git_diff))
        .route("/api/chats", post(create_chat))
        .route("/api/chats/{id}/export", get(export_chat))
        .layer(CorsLayer::permissive())
        .with_state(state)
}

pub async fn initialize_server_state(pool: SqlitePool) -> Result<AppState> {
    register_builtin_extensions(&pool).await?;
    let engine = RunEngine::new(pool.clone());
    let recovered = engine.recover_unfinished_runs().await?;
    if !recovered.is_empty() {
        info!(count = recovered.len(), "Recovered durable runs on startup");
    }
    let db_path = default_db_path().unwrap_or_else(|_| PathBuf::from(":memory:"));
    Ok(AppState { pool, db_path })
}

pub async fn serve(pool: SqlitePool, addr: SocketAddr) -> Result<()> {
    let state = initialize_server_state(pool).await?;
    let app = build_router(state);
    let listener = tokio::net::TcpListener::bind(addr).await?;
    info!(%addr, "Gaggle server listening");
    axum::serve(listener, app).await?;
    Ok(())
}

async fn index_handler() -> Html<&'static str> {
    Html(gaggle_ui::render_index_html())
}

async fn health_handler() -> Json<serde_json::Value> {
    Json(serde_json::json!({ "status": "ok", "service": "gaggle" }))
}

async fn diagnostics_handler(
    State(state): State<AppState>,
) -> Result<Json<DiagnosticReport>, (StatusCode, String)> {
    let report = collect_diagnostics(&state.pool, &state.db_path)
        .await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;
    Ok(Json(report))
}

async fn list_projects(
    State(state): State<AppState>,
) -> Result<Json<Vec<Project>>, (StatusCode, String)> {
    let repo = ProjectRepository::new(state.pool);
    let projects = repo
        .list_active()
        .await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;
    Ok(Json(projects))
}

async fn create_project(
    State(state): State<AppState>,
    Json(req): Json<CreateProjectRequest>,
) -> Result<(StatusCode, Json<Project>), (StatusCode, String)> {
    let policy = WorkspacePathPolicy::new(&req.root_path)
        .map_err(|e| (StatusCode::BAD_REQUEST, e.to_string()))?;
    let canonical_root = policy.canonical_root().to_string_lossy().to_string();

    let repo = ProjectRepository::new(state.pool);
    let vcs_kind = req.vcs_kind.as_deref().unwrap_or("git");
    let project = repo
        .create(&req.name, &canonical_root, vcs_kind)
        .await
        .map_err(|e| (StatusCode::BAD_REQUEST, e.to_string()))?;
    Ok((StatusCode::CREATED, Json(project)))
}

async fn create_chat(
    State(state): State<AppState>,
    Json(req): Json<CreateChatRequest>,
) -> Result<(StatusCode, Json<Chat>), (StatusCode, String)> {
    let repo = ChatRepository::new(state.pool);
    let mode = req.mode.as_deref().unwrap_or("local");
    let permission_mode = req.permission_mode.as_deref().unwrap_or("ask");
    let chat = repo
        .create(&req.project_id, &req.title, "user", mode, permission_mode)
        .await
        .map_err(|e| (StatusCode::BAD_REQUEST, e.to_string()))?;
    Ok((StatusCode::CREATED, Json(chat)))
}

async fn export_chat(
    State(state): State<AppState>,
    Path(id): Path<String>,
) -> Result<Json<ChatExportBundle>, (StatusCode, String)> {
    let repo = ChatRepository::new(state.pool);
    let bundle = repo
        .export_chat(&id)
        .await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?
        .ok_or_else(|| (StatusCode::NOT_FOUND, format!("Chat {id} not found")))?;
    Ok(Json(bundle))
}

async fn resolve_project_policy(
    pool: SqlitePool,
    project_id: &str,
) -> Result<WorkspacePathPolicy, (StatusCode, String)> {
    let repo = ProjectRepository::new(pool);
    let project = repo
        .get_by_id(project_id)
        .await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?
        .ok_or_else(|| {
            (
                StatusCode::NOT_FOUND,
                format!("Project {project_id} not found"),
            )
        })?;

    WorkspacePathPolicy::new(&project.root_path)
        .map_err(|e| (StatusCode::BAD_REQUEST, e.to_string()))
}

async fn list_project_files(
    State(state): State<AppState>,
    Path(id): Path<String>,
    Query(query): Query<ProjectPathQuery>,
) -> Result<Json<Vec<FsEntry>>, (StatusCode, String)> {
    let policy = resolve_project_policy(state.pool, &id).await?;
    let entries = tokio::task::spawn_blocking(move || {
        fs_list(&policy, query.path.as_deref(), DEFAULT_MAX_LIST_ENTRIES)
    })
    .await
    .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?
    .map_err(|e| (StatusCode::BAD_REQUEST, e.to_string()))?;

    Ok(Json(entries))
}

async fn project_git_status(
    State(state): State<AppState>,
    Path(id): Path<String>,
) -> Result<Json<GitWorkspaceStatus>, (StatusCode, String)> {
    let policy = resolve_project_policy(state.pool, &id).await?;
    let status = tokio::task::spawn_blocking(move || git_status(&policy))
        .await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?
        .map_err(|e| (StatusCode::BAD_REQUEST, e.to_string()))?;

    Ok(Json(status))
}

async fn project_git_diff(
    State(state): State<AppState>,
    Path(id): Path<String>,
    Query(query): Query<ProjectPathQuery>,
) -> Result<Json<GitDiffOutput>, (StatusCode, String)> {
    let policy = resolve_project_policy(state.pool, &id).await?;
    let diff = tokio::task::spawn_blocking(move || {
        git_diff(&policy, query.path.as_deref(), DEFAULT_MAX_DIFF_BYTES)
    })
    .await
    .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?
    .map_err(|e| (StatusCode::BAD_REQUEST, e.to_string()))?;

    Ok(Json(diff))
}
