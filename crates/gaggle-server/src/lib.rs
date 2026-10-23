use anyhow::Result;
use axum::{
    Json, Router,
    extract::{Path, Query, State},
    http::{HeaderMap, StatusCode},
    response::{
        Html,
        sse::{Event, Sse},
    },
    routing::{get, patch, post},
};
use futures::stream::Stream;
use gaggle_core::{
    ActiveRunRegistry, AgentLoopOutcome, AgentRuntime, CredentialStore, DEFAULT_MAX_AGENT_STEPS,
    DiagnosticReport, InMemoryCredentialStore, MockProviderAdapter, ModelCompletionResponse,
    RunEngine, SecretString, TokenUsage, WorkspacePathPolicy, collect_diagnostics,
    validate_provider_base_url,
};
use gaggle_db::{
    default_db_path,
    models::{AppSetting, Approval, Chat, Extension, Model, Project, Provider, Run, Worktree},
    repositories::{
        chats::{ChatExportBundle, ChatRepository},
        extensions::ExtensionRepository,
        projects::ProjectRepository,
        providers::ProviderRepository,
        runs::RunRepository,
        settings::SettingsRepository,
        worktrees::WorktreeRepository,
    },
};
use gaggle_tools::{
    BuiltinToolExecutor, DEFAULT_MAX_DIFF_BYTES, DEFAULT_MAX_LIST_ENTRIES, FsEntry, GitDiffOutput,
    GitWorkspaceStatus, fs_list, git_diff, git_status, git_worktree_create, git_worktree_discard,
    git_worktree_merge, register_builtin_extensions,
};
use serde::{Deserialize, Serialize};
use sqlx::SqlitePool;
use std::{convert::Infallible, net::SocketAddr, path::PathBuf, sync::Arc};
use tower_http::cors::CorsLayer;
use tracing::info;

#[derive(Clone)]
pub struct AppState {
    pub pool: SqlitePool,
    pub db_path: PathBuf,
    pub credential_store: Arc<dyn CredentialStore>,
    pub active_runs: ActiveRunRegistry,
}

#[derive(Debug, Deserialize)]
pub struct CreateProjectRequest {
    pub name: String,
    pub root_path: String,
    pub vcs_kind: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProjectStatusResponse {
    #[serde(flatten)]
    pub project: Project,
    pub path_exists: bool,
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

#[derive(Debug, Deserialize)]
pub struct DecideApprovalRequest {
    pub approved: bool,
    pub decided_by_device: Option<String>,
    pub project_id: Option<String>,
    pub remember_pattern: Option<String>,
    pub remember_match_kind: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct CreateProviderRequest {
    pub kind: String,
    pub name: String,
    pub base_url: Option<String>,
    pub auth_kind: String,
    pub secret_ref: Option<String>,
    pub api_key: Option<String>,
}

#[derive(Debug, Serialize)]
pub struct ProviderStatusResponse {
    pub provider: Provider,
    pub has_configured_secret: bool,
}

#[derive(Debug, Deserialize)]
pub struct CreateModelRequest {
    pub model_key: String,
    pub display_name: String,
    pub context_window: i64,
    pub supports_tools: Option<bool>,
    pub supports_vision: Option<bool>,
    pub supports_reasoning: Option<bool>,
    pub price_in_per_mtok: Option<f64>,
    pub price_out_per_mtok: Option<f64>,
}

#[derive(Debug, Deserialize)]
pub struct SubmitChatMessageRequest {
    pub prompt: String,
    pub model_key: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct RunEventsQuery {
    pub after_seq: Option<i64>,
}

#[derive(Debug, Deserialize)]
pub struct UpdateChatRequest {
    pub title: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct WorktreeActionRequest {
    pub confirm: bool,
    pub decided_by_device: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct UpsertSettingRequest {
    pub key: String,
    pub value: serde_json::Value,
}

pub fn build_router(state: AppState) -> Router {
    Router::new()
        .route("/", get(index_handler))
        .route("/api/health", get(health_handler))
        .route("/api/diagnostics", get(diagnostics_handler))
        .route("/api/settings", get(list_settings).put(upsert_setting))
        .route("/api/projects", get(list_projects).post(create_project))
        .route("/api/projects/{id}/chats", get(list_project_chats))
        .route("/api/projects/{id}/files", get(list_project_files))
        .route("/api/projects/{id}/git-status", get(project_git_status))
        .route("/api/projects/{id}/diff", get(project_git_diff))
        .route("/api/projects/{id}/worktrees", get(list_project_worktrees))
        .route("/api/providers", get(list_providers).post(create_provider))
        .route(
            "/api/providers/{id}/models",
            get(list_provider_models).post(create_provider_model),
        )
        .route("/api/extensions", get(list_extensions))
        .route("/api/chats", post(create_chat))
        .route("/api/chats/{id}/messages", post(submit_chat_message))
        .route("/api/chats/{id}", patch(update_chat))
        .route("/api/chats/{id}/export", get(export_chat))
        .route("/api/worktrees/{id}/git-status", get(worktree_git_status))
        .route("/api/worktrees/{id}/diff", get(worktree_git_diff))
        .route("/api/worktrees/{id}/merge", post(merge_worktree))
        .route("/api/worktrees/{id}/discard", post(discard_worktree))
        .route("/api/runs/{id}/cancel", post(cancel_run))
        .route("/api/runs/{id}/events", get(stream_run_events))
        .route("/api/approvals/pending", get(list_pending_approvals))
        .route("/api/approvals/{id}/decide", post(decide_approval))
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
    let credential_store: Arc<dyn CredentialStore> = Arc::new(InMemoryCredentialStore::new());
    let active_runs = ActiveRunRegistry::new();
    Ok(AppState {
        pool,
        db_path,
        credential_store,
        active_runs,
    })
}

pub async fn serve(pool: SqlitePool, addr: SocketAddr) -> Result<()> {
    let state = initialize_server_state(pool).await?;
    let app = build_router(state);
    let listener = tokio::net::TcpListener::bind(addr).await?;
    info!(%addr, "Gaggle server listening");
    axum::serve(listener, app).await?;
    Ok(())
}

async fn index_handler() -> Html<String> {
    Html(gaggle_ui::render_index_html())
}

async fn health_handler() -> Json<serde_json::Value> {
    Json(serde_json::json!({ "status": "ok", "service": "gaggle" }))
}

async fn list_settings(
    State(state): State<AppState>,
) -> Result<Json<Vec<AppSetting>>, (StatusCode, String)> {
    let repo = SettingsRepository::new(state.pool);
    let settings = repo
        .list_all()
        .await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;
    Ok(Json(settings))
}

async fn upsert_setting(
    State(state): State<AppState>,
    Json(req): Json<UpsertSettingRequest>,
) -> Result<Json<AppSetting>, (StatusCode, String)> {
    if req.key.trim().is_empty() {
        return Err((
            StatusCode::BAD_REQUEST,
            "Setting key must not be empty".to_string(),
        ));
    }
    let value_json =
        serde_json::to_string(&req.value).map_err(|e| (StatusCode::BAD_REQUEST, e.to_string()))?;
    let repo = SettingsRepository::new(state.pool);
    let setting = repo
        .upsert(req.key.trim(), &value_json)
        .await
        .map_err(|e| (StatusCode::BAD_REQUEST, e.to_string()))?;
    Ok(Json(setting))
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
) -> Result<Json<Vec<ProjectStatusResponse>>, (StatusCode, String)> {
    let repo = ProjectRepository::new(state.pool);
    let projects = repo
        .list_active()
        .await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;

    let enriched = projects
        .into_iter()
        .map(|project| {
            let path_exists = std::path::Path::new(&project.root_path).is_dir();
            ProjectStatusResponse {
                project,
                path_exists,
            }
        })
        .collect();

    Ok(Json(enriched))
}

async fn create_project(
    State(state): State<AppState>,
    Json(req): Json<CreateProjectRequest>,
) -> Result<(StatusCode, Json<ProjectStatusResponse>), (StatusCode, String)> {
    let policy = WorkspacePathPolicy::new(&req.root_path)
        .map_err(|e| (StatusCode::BAD_REQUEST, e.to_string()))?;
    let canonical_root = policy.canonical_root().to_string_lossy().to_string();

    let repo = ProjectRepository::new(state.pool);
    let vcs_kind = req.vcs_kind.as_deref().unwrap_or("git");
    let project = repo
        .create(&req.name, &canonical_root, vcs_kind)
        .await
        .map_err(|e| (StatusCode::BAD_REQUEST, e.to_string()))?;
    Ok((
        StatusCode::CREATED,
        Json(ProjectStatusResponse {
            project,
            path_exists: true,
        }),
    ))
}

async fn list_project_chats(
    State(state): State<AppState>,
    Path(project_id): Path<String>,
) -> Result<Json<Vec<Chat>>, (StatusCode, String)> {
    let repo = ChatRepository::new(state.pool);
    let chats = repo
        .list_by_project(&project_id)
        .await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;
    Ok(Json(chats))
}

async fn create_chat(
    State(state): State<AppState>,
    Json(req): Json<CreateChatRequest>,
) -> Result<(StatusCode, Json<Chat>), (StatusCode, String)> {
    let chat_repo = ChatRepository::new(state.pool.clone());
    let mode = req.mode.as_deref().unwrap_or("local");
    let permission_mode = req.permission_mode.as_deref().unwrap_or("ask");
    let chat = chat_repo
        .create(&req.project_id, &req.title, "user", mode, permission_mode)
        .await
        .map_err(|e| (StatusCode::BAD_REQUEST, e.to_string()))?;

    if mode == "worktree" {
        let proj_repo = ProjectRepository::new(state.pool.clone());
        let project = proj_repo
            .get_by_id(&req.project_id)
            .await
            .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?
            .ok_or_else(|| {
                (
                    StatusCode::NOT_FOUND,
                    format!("Project {} not found", req.project_id),
                )
            })?;

        let policy = WorkspacePathPolicy::new(&project.root_path)
            .map_err(|e| (StatusCode::BAD_REQUEST, e.to_string()))?;
        let vcs_kind = project.vcs_kind.clone();
        let chat_id_clone = chat.id.clone();

        let created_wt = tokio::task::spawn_blocking(move || {
            git_worktree_create(&policy, &vcs_kind, &chat_id_clone)
        })
        .await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?
        .map_err(|e| (StatusCode::BAD_REQUEST, e.to_string()))?;

        let wt_repo = WorktreeRepository::new(state.pool);
        wt_repo
            .create_and_attach_to_chat(
                &project.id,
                &chat.id,
                &created_wt.path,
                &created_wt.branch,
                &created_wt.base_ref,
                &created_wt.base_commit,
            )
            .await
            .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;

        let updated_chat = chat_repo
            .get_by_id(&chat.id)
            .await
            .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?
            .ok_or_else(|| (StatusCode::NOT_FOUND, format!("Chat {} not found", chat.id)))?;

        return Ok((StatusCode::CREATED, Json(updated_chat)));
    }

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

async fn update_chat(
    State(state): State<AppState>,
    Path(id): Path<String>,
    Json(req): Json<UpdateChatRequest>,
) -> Result<Json<Chat>, (StatusCode, String)> {
    let repo = ChatRepository::new(state.pool);
    let updated = if let Some(title) = req.title {
        repo.update_title(&id, &title)
            .await
            .map_err(|e| (StatusCode::BAD_REQUEST, e.to_string()))?
    } else {
        repo.get_by_id(&id)
            .await
            .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?
            .ok_or_else(|| (StatusCode::NOT_FOUND, format!("Chat {id} not found")))?
    };
    Ok(Json(updated))
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

async fn list_pending_approvals(
    State(state): State<AppState>,
) -> Result<Json<Vec<Approval>>, (StatusCode, String)> {
    let repo = RunRepository::new(state.pool);
    let list = repo
        .list_pending_approvals()
        .await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;
    Ok(Json(list))
}

async fn decide_approval(
    State(state): State<AppState>,
    Path(id): Path<String>,
    Json(req): Json<DecideApprovalRequest>,
) -> Result<Json<Approval>, (StatusCode, String)> {
    let repo = RunRepository::new(state.pool);
    let remember_pair = req.remember_pattern.as_deref().map(|pat| {
        let kind = req.remember_match_kind.as_deref().unwrap_or("exact");
        (pat, kind)
    });

    let updated = repo
        .decide_approval(
            &id,
            req.approved,
            req.decided_by_device.as_deref(),
            req.project_id.as_deref(),
            remember_pair,
        )
        .await
        .map_err(|e| (StatusCode::BAD_REQUEST, e.to_string()))?;

    Ok(Json(updated))
}

async fn list_extensions(
    State(state): State<AppState>,
) -> Result<Json<Vec<Extension>>, (StatusCode, String)> {
    let repo = ExtensionRepository::new(state.pool);
    let exts = repo
        .list_enabled_extensions()
        .await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;
    Ok(Json(exts))
}

async fn list_providers(
    State(state): State<AppState>,
) -> Result<Json<Vec<ProviderStatusResponse>>, (StatusCode, String)> {
    let repo = ProviderRepository::new(state.pool);
    let providers = repo
        .list_providers()
        .await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;

    let statuses = providers
        .into_iter()
        .map(|p| {
            let has_configured_secret = p
                .secret_ref
                .as_deref()
                .is_some_and(|key| state.credential_store.has_secret(key));
            ProviderStatusResponse {
                provider: p,
                has_configured_secret,
            }
        })
        .collect();

    Ok(Json(statuses))
}

async fn create_provider(
    State(state): State<AppState>,
    Json(req): Json<CreateProviderRequest>,
) -> Result<(StatusCode, Json<ProviderStatusResponse>), (StatusCode, String)> {
    let validated_url = match &req.base_url {
        Some(url) => Some(
            validate_provider_base_url(url)
                .map_err(|e| (StatusCode::BAD_REQUEST, e.to_string()))?,
        ),
        None => None,
    };

    if let (Some(secret_ref), Some(raw_key)) = (&req.secret_ref, &req.api_key) {
        let secret =
            SecretString::new(raw_key).map_err(|e| (StatusCode::BAD_REQUEST, e.to_string()))?;
        state
            .credential_store
            .set_secret(secret_ref, secret)
            .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;
    }

    let repo = ProviderRepository::new(state.pool);
    let provider = repo
        .create_provider(
            &req.kind,
            &req.name,
            validated_url.as_deref(),
            &req.auth_kind,
            req.secret_ref.as_deref(),
        )
        .await
        .map_err(|e| (StatusCode::BAD_REQUEST, e.to_string()))?;

    let has_configured_secret = provider
        .secret_ref
        .as_deref()
        .is_some_and(|key| state.credential_store.has_secret(key));

    Ok((
        StatusCode::CREATED,
        Json(ProviderStatusResponse {
            provider,
            has_configured_secret,
        }),
    ))
}

async fn list_provider_models(
    State(state): State<AppState>,
    Path(id): Path<String>,
) -> Result<Json<Vec<Model>>, (StatusCode, String)> {
    let repo = ProviderRepository::new(state.pool);
    let models = repo
        .list_models_by_provider(&id)
        .await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;
    Ok(Json(models))
}

async fn create_provider_model(
    State(state): State<AppState>,
    Path(id): Path<String>,
    Json(req): Json<CreateModelRequest>,
) -> Result<(StatusCode, Json<Model>), (StatusCode, String)> {
    let repo = ProviderRepository::new(state.pool);
    let model = repo
        .create_model(
            &id,
            &req.model_key,
            &req.display_name,
            req.context_window,
            req.supports_tools.unwrap_or(true),
            req.supports_vision.unwrap_or(false),
            req.supports_reasoning.unwrap_or(false),
            req.price_in_per_mtok,
            req.price_out_per_mtok,
        )
        .await
        .map_err(|e| (StatusCode::BAD_REQUEST, e.to_string()))?;
    Ok((StatusCode::CREATED, Json(model)))
}

async fn submit_chat_message(
    State(state): State<AppState>,
    Path(chat_id): Path<String>,
    Json(req): Json<SubmitChatMessageRequest>,
) -> Result<Json<AgentLoopOutcome>, (StatusCode, String)> {
    let runtime = AgentRuntime::new(state.pool.clone(), DEFAULT_MAX_AGENT_STEPS);
    let executor = BuiltinToolExecutor::new(state.pool.clone());
    let model_key = req.model_key.as_deref().unwrap_or("local-mock");

    // Default local fallback provider when no external provider is bound to the chat
    let provider = MockProviderAdapter::with_response(
        "local-mock",
        ModelCompletionResponse {
            text: format!("Processed request: {}", req.prompt),
            tool_calls: vec![],
            usage: TokenUsage {
                input_tokens: 12,
                output_tokens: 8,
                cached_tokens: 0,
            },
        },
    );

    let cancel_token = gaggle_core::CancellationToken::new();
    let outcome = runtime
        .run_turn(
            &chat_id,
            &req.prompt,
            model_key,
            &provider,
            &executor,
            &cancel_token,
        )
        .await
        .map_err(|e| (StatusCode::BAD_REQUEST, e.to_string()))?;

    Ok(Json(outcome))
}

async fn cancel_run(
    State(state): State<AppState>,
    Path(run_id): Path<String>,
) -> Result<Json<Run>, (StatusCode, String)> {
    state.active_runs.cancel(&run_id);
    let repo = RunRepository::new(state.pool);
    repo.emit_event(
        &run_id,
        "run.cancelled",
        &serde_json::json!({ "reason": "user_cancelled_via_api" }).to_string(),
    )
    .await
    .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;

    let updated = repo
        .update_run_status(&run_id, "cancelled", Some("Cancelled by user"))
        .await
        .map_err(|e| (StatusCode::BAD_REQUEST, e.to_string()))?;
    Ok(Json(updated))
}

async fn stream_run_events(
    State(state): State<AppState>,
    Path(run_id): Path<String>,
    headers: HeaderMap,
    Query(query): Query<RunEventsQuery>,
) -> Result<Sse<impl Stream<Item = Result<Event, Infallible>>>, (StatusCode, String)> {
    let last_event_id_header = headers
        .get("Last-Event-ID")
        .and_then(|v| v.to_str().ok())
        .and_then(|s| s.parse::<i64>().ok());

    let after_seq = query.after_seq.or(last_event_id_header).unwrap_or(0);

    let repo = RunRepository::new(state.pool);
    let events = repo
        .list_run_events(&run_id, after_seq)
        .await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;

    let sse_items: Vec<Result<Event, Infallible>> = events
        .into_iter()
        .map(|ev| {
            Ok(Event::default()
                .id(ev.seq.to_string())
                .event(ev.kind)
                .data(ev.payload))
        })
        .collect();

    Ok(Sse::new(tokio_stream::iter(sse_items)))
}

async fn list_project_worktrees(
    State(state): State<AppState>,
    Path(project_id): Path<String>,
) -> Result<Json<Vec<Worktree>>, (StatusCode, String)> {
    let repo = WorktreeRepository::new(state.pool);
    let list = repo
        .list_by_project(&project_id)
        .await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;
    Ok(Json(list))
}

async fn resolve_worktree_policy(
    pool: SqlitePool,
    worktree_id: &str,
) -> Result<WorkspacePathPolicy, (StatusCode, String)> {
    let repo = WorktreeRepository::new(pool);
    let wt = repo
        .get_by_id(worktree_id)
        .await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?
        .ok_or_else(|| {
            (
                StatusCode::NOT_FOUND,
                format!("Worktree {worktree_id} not found"),
            )
        })?;

    WorkspacePathPolicy::new(&wt.path).map_err(|e| (StatusCode::BAD_REQUEST, e.to_string()))
}

async fn worktree_git_status(
    State(state): State<AppState>,
    Path(id): Path<String>,
) -> Result<Json<GitWorkspaceStatus>, (StatusCode, String)> {
    let policy = resolve_worktree_policy(state.pool, &id).await?;
    let status = tokio::task::spawn_blocking(move || git_status(&policy))
        .await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?
        .map_err(|e| (StatusCode::BAD_REQUEST, e.to_string()))?;
    Ok(Json(status))
}

async fn worktree_git_diff(
    State(state): State<AppState>,
    Path(id): Path<String>,
    Query(query): Query<ProjectPathQuery>,
) -> Result<Json<GitDiffOutput>, (StatusCode, String)> {
    let policy = resolve_worktree_policy(state.pool, &id).await?;
    let diff = tokio::task::spawn_blocking(move || {
        git_diff(&policy, query.path.as_deref(), DEFAULT_MAX_DIFF_BYTES)
    })
    .await
    .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?
    .map_err(|e| (StatusCode::BAD_REQUEST, e.to_string()))?;
    Ok(Json(diff))
}

async fn merge_worktree(
    State(state): State<AppState>,
    Path(id): Path<String>,
    Json(req): Json<WorktreeActionRequest>,
) -> Result<Json<Worktree>, (StatusCode, String)> {
    let wt_repo = WorktreeRepository::new(state.pool.clone());
    let wt = wt_repo
        .get_by_id(&id)
        .await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?
        .ok_or_else(|| (StatusCode::NOT_FOUND, format!("Worktree {id} not found")))?;

    let policy = resolve_project_policy(state.pool, &wt.project_id).await?;
    let wt_path = wt.path.clone();
    let wt_branch = wt.branch.clone();
    let confirm = req.confirm;

    tokio::task::spawn_blocking(move || git_worktree_merge(&policy, &wt_path, &wt_branch, confirm))
        .await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?
        .map_err(|e| (StatusCode::BAD_REQUEST, e.to_string()))?;

    let updated = wt_repo
        .transition_status(&id, "merged", req.decided_by_device.as_deref())
        .await
        .map_err(|e| (StatusCode::BAD_REQUEST, e.to_string()))?;

    Ok(Json(updated))
}

async fn discard_worktree(
    State(state): State<AppState>,
    Path(id): Path<String>,
    Json(req): Json<WorktreeActionRequest>,
) -> Result<Json<Worktree>, (StatusCode, String)> {
    let wt_repo = WorktreeRepository::new(state.pool.clone());
    let wt = wt_repo
        .get_by_id(&id)
        .await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?
        .ok_or_else(|| (StatusCode::NOT_FOUND, format!("Worktree {id} not found")))?;

    let policy = resolve_project_policy(state.pool, &wt.project_id).await?;
    let wt_path = wt.path.clone();
    let wt_branch = wt.branch.clone();
    let confirm = req.confirm;

    tokio::task::spawn_blocking(move || {
        git_worktree_discard(&policy, &wt_path, &wt_branch, confirm)
    })
    .await
    .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?
    .map_err(|e| (StatusCode::BAD_REQUEST, e.to_string()))?;

    let updated = wt_repo
        .transition_status(&id, "discarded", req.decided_by_device.as_deref())
        .await
        .map_err(|e| (StatusCode::BAD_REQUEST, e.to_string()))?;

    Ok(Json(updated))
}
