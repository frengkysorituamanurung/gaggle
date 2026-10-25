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
use gaggle_core::CompactionEngine;
use gaggle_core::{
    ActionableError, ActiveRunRegistry, AgentLoopOutcome, AgentRuntime, AnthropicAdapter,
    BuildIdentity, ChatGptAdapter, ChatGptOAuthClient, ChatGptOAuthConfig, CorrelationIds,
    CredentialStore, DEFAULT_MAX_AGENT_STEPS, DataPreservation, DiagnosticReport,
    DiagnosticsExport, FailureCategory, InMemoryCredentialStore, MockProviderAdapter,
    ModelCompletionResponse, ModelProviderAdapter, OllamaAdapter, OpenAiCompatibleAdapter,
    RunEngine, SecretString, TokenUsage, WorkspacePathPolicy, collect_diagnostics,
    export_diagnostics, validate_provider_base_url,
};
use gaggle_db::{
    connection::{new_id, now_ms},
    models::{
        AppSetting, Approval, Chat, ChatSummary, CommandRule, Extension, ExtensionPermission,
        ExtensionTool, Memory, Message, MessagePart, Model, Project, Provider, Recipe, Run,
        Schedule, Worktree,
    },
    repositories::{
        chats::{ChatExportBundle, ChatRepository},
        extensions::ExtensionRepository,
        memories::MemoryRepository,
        permissions::PermissionRepository,
        projects::ProjectRepository,
        providers::ProviderRepository,
        recipes::RecipeRepository,
        runs::RunRepository,
        settings::SettingsRepository,
        worktrees::WorktreeRepository,
    },
};
use gaggle_mcp::{ExtensionRegistry, InstallExtensionRequest, RECOMMENDED_MAX_ACTIVE_TOOLS};
use gaggle_recipes::{
    compute_next_run, render_recipe_template, resolve_recipe_parameters,
    validate_recipe_parameters, validate_timezone,
};
use gaggle_tools::{
    BuiltinToolExecutor, DEFAULT_MAX_DIFF_BYTES, DEFAULT_MAX_LIST_ENTRIES, FsEntry, GitDiffOutput,
    GitWorkspaceStatus, fs_list, git_diff, git_status, git_worktree_create, git_worktree_discard,
    git_worktree_merge, register_builtin_extensions,
};
use serde::{Deserialize, Serialize};
use sqlx::SqlitePool;
use std::{
    collections::{HashMap, HashSet},
    convert::Infallible,
    net::SocketAddr,
    path::PathBuf,
    sync::{Arc, RwLock},
};
use tower_http::cors::CorsLayer;
use tracing::info;

mod terminal;
use terminal::{TerminalManager, terminal_routes};

/// Pending ChatGPT OAuth authorization keyed by the OAuth `state` value. The PKCE verifier is
/// held only in memory for the short authorization window and is never written to SQLite.
#[derive(Clone)]
pub struct PendingOAuthSession {
    pub provider_id: String,
    pub secret_ref: String,
    pub code_verifier: String,
    pub config: ChatGptOAuthConfig,
    pub created_at_ms: i64,
}

#[derive(Clone)]
pub struct AppState {
    pub pool: SqlitePool,
    pub db_path: PathBuf,
    pub credential_store: Arc<dyn CredentialStore>,
    pub active_runs: ActiveRunRegistry,
    pub oauth_sessions: Arc<RwLock<HashMap<String, PendingOAuthSession>>>,
    pub terminal_manager: Arc<TerminalManager>,
}

#[derive(Debug, Deserialize, Serialize)]
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

#[derive(Debug, Deserialize, Serialize)]
pub struct CreateChatRequest {
    pub project_id: String,
    pub title: String,
    pub mode: Option<String>,
    pub permission_mode: Option<String>,
    pub chat_type: Option<String>,
    pub parent_chat_id: Option<String>,
    pub parent_tool_call_id: Option<String>,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct CreateRecipeRequest {
    pub name: String,
    pub title: String,
    pub description: Option<String>,
    pub source_kind: Option<String>,
    pub source_ref: Option<String>,
    pub instructions: Option<String>,
    pub prompt: Option<String>,
    pub definition: Option<String>,
    pub default_model_id: Option<String>,
    pub parameters: Option<Vec<CreateRecipeParameterRequest>>,
    pub extensions: Option<Vec<String>>,
    pub sub_recipes: Option<Vec<CreateRecipeSubRecipeRequest>>,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct CreateRecipeParameterRequest {
    pub name: String,
    pub param_type: Option<String>,
    pub description: Option<String>,
    pub required: Option<i64>,
    pub default_value: Option<String>,
    pub idx: Option<i64>,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct CreateRecipeSubRecipeRequest {
    pub child_recipe_id: String,
    pub name: String,
    pub values_json: Option<String>,
    pub sequential_when_repeated: Option<i64>,
    pub idx: Option<i64>,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct UpdateRecipeRequest {
    pub name: Option<String>,
    pub title: Option<String>,
    pub description: Option<String>,
    pub source_kind: Option<String>,
    pub source_ref: Option<String>,
    pub instructions: Option<String>,
    pub prompt: Option<String>,
    pub definition: Option<String>,
    pub default_model_id: Option<String>,
    pub parameters: Option<Vec<CreateRecipeParameterRequest>>,
    pub extensions: Option<Vec<String>>,
    pub sub_recipes: Option<Vec<CreateRecipeSubRecipeRequest>>,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct CreateScheduleRequest {
    pub recipe_id: String,
    pub project_id: Option<String>,
    pub cron_expr: String,
    pub timezone: Option<String>,
    pub param_values: Option<serde_json::Value>,
    pub enabled: Option<i64>,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct ExecuteRecipeRequest {
    pub project_id: String,
    pub chat_type: Option<String>,
    pub mode: Option<String>,
    pub permission_mode: Option<String>,
    pub param_values: Option<serde_json::Value>,
    pub parent_chat_id: Option<String>,
    pub parent_tool_call_id: Option<String>,
    pub title: Option<String>,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct ProjectPathQuery {
    pub path: Option<String>,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct DecideApprovalRequest {
    pub approved: bool,
    pub decided_by_device: Option<String>,
    pub project_id: Option<String>,
    pub remember_pattern: Option<String>,
    pub remember_match_kind: Option<String>,
}

#[derive(Debug, Deserialize, Serialize)]
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

#[derive(Debug, Deserialize, Serialize)]
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

#[derive(Debug, Deserialize, Serialize)]
pub struct SubmitChatMessageRequest {
    pub prompt: String,
    pub model_key: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct CompactChatRequest {
    pub trigger: Option<String>,
}

#[derive(Debug, Serialize)]
pub struct ChatMessagesResponse {
    pub messages: Vec<Message>,
    pub message_parts: Vec<MessagePart>,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct RunEventsQuery {
    pub after_seq: Option<i64>,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct UpdateChatRequest {
    pub title: Option<String>,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct UpdateProjectRequest {
    pub default_mode: Option<String>,
    pub default_access: Option<String>,
    pub default_model_id: Option<String>,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct CreateCommandRuleRequest {
    pub pattern: String,
    pub match_kind: Option<String>,
    pub action: String,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct SetToolPermissionRequest {
    pub extension_tool_id: String,
    pub level: String,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct UpdateExtensionRequest {
    pub enabled: bool,
}

#[derive(Debug, Deserialize)]
pub struct CreateExtensionRequest {
    pub name: String,
    pub description: Option<String>,
    pub kind: String,
    pub version: Option<String>,
    pub source: Option<String>,
    pub command: Option<String>,
    pub args: Option<Vec<String>>,
    pub url: Option<String>,
    pub env_secret_refs: Option<serde_json::Value>,
    pub timeout_secs: Option<i64>,
    pub manifest: Option<serde_json::Value>,
    pub bundled: Option<bool>,
    pub enabled: Option<bool>,
}

#[derive(Debug, Deserialize)]
pub struct GrantExtensionPermissionRequest {
    pub permission: String,
    pub scope: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
pub struct ExtensionToolSummary {
    pub tool_name: String,
    pub is_read_only: bool,
}

#[derive(Debug, Clone, Serialize)]
pub struct ExtensionInventoryItem {
    #[serde(flatten)]
    pub extension: Extension,
    pub tool_count: i64,
    pub tools: Vec<ExtensionToolSummary>,
}

#[derive(Debug, Clone, Serialize)]
pub struct ExtensionsInventoryResponse {
    pub recommended_max_active_tools: usize,
    pub active_tool_count: i64,
    pub exceeds_recommended_tool_limit: bool,
    pub extensions: Vec<ExtensionInventoryItem>,
}

#[derive(Debug, Clone, Serialize)]
pub struct ToolPermissionStatus {
    pub extension_tool_id: String,
    pub extension_name: String,
    pub tool_name: String,
    pub is_read_only: bool,
    /// `None` means no explicit override: the permission engine's default
    /// (`ask_before` unless the tool is annotated read-only) applies.
    pub level: Option<String>,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct WorktreeActionRequest {
    pub confirm: bool,
    pub decided_by_device: Option<String>,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct UpsertSettingRequest {
    pub key: String,
    pub value: serde_json::Value,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct MemoryQuery {
    pub project_id: Option<String>,
    pub query: Option<String>,
    pub limit: Option<i64>,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct CreateMemoryRequest {
    pub project_id: String,
    pub kind: String,
    pub title: String,
    pub content: String,
    pub tags: Option<Vec<String>>,
    pub pinned: Option<bool>,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct UpdateMemoryRequest {
    pub kind: Option<String>,
    pub title: Option<String>,
    pub content: Option<String>,
    pub pinned: Option<bool>,
    pub tags: Option<Vec<String>>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MemoryResponse {
    #[serde(flatten)]
    pub memory: Memory,
    pub tags: Vec<String>,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct StartChatGptOAuthRequest {
    pub client_id: Option<String>,
    pub auth_url: Option<String>,
    pub token_url: Option<String>,
    pub redirect_uri: Option<String>,
    pub scopes: Option<Vec<String>>,
}

#[derive(Debug, Serialize)]
pub struct StartChatGptOAuthResponse {
    pub authorization_url: String,
    pub state: String,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct ExchangeChatGptOAuthRequest {
    pub state: String,
    pub code: String,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct ChatGptOAuthCallbackQuery {
    pub state: String,
    pub code: String,
}

#[derive(Debug, Serialize)]
pub struct ChatGptOAuthStatusResponse {
    pub configured: bool,
    pub provider_id: String,
}

pub fn build_router(state: AppState) -> Router {
    Router::new()
        .route("/", get(index_handler))
        .route("/api/health", get(health_handler))
        .route("/api/diagnostics", get(diagnostics_handler))
        .route("/api/diagnostics/export", get(diagnostics_export_handler))
        .route("/api/settings", get(list_settings).put(upsert_setting))
        .route("/api/projects", get(list_projects).post(create_project))
        .route("/api/projects/{id}", patch(update_project))
        .route("/api/projects/{id}/chats", get(list_project_chats))
        .route("/api/projects/{id}/files", get(list_project_files))
        .route("/api/projects/{id}/git-status", get(project_git_status))
        .route("/api/projects/{id}/diff", get(project_git_diff))
        .route("/api/projects/{id}/worktrees", get(list_project_worktrees))
        .route(
            "/api/projects/{id}/command-rules",
            get(list_project_command_rules).post(create_project_command_rule),
        )
        .route(
            "/api/projects/{id}/tool-permissions",
            get(list_project_tool_permissions).put(set_project_tool_permission),
        )
        .route("/api/providers", get(list_providers).post(create_provider))
        .route(
            "/api/providers/{id}/models",
            get(list_provider_models).post(create_provider_model),
        )
        .route(
            "/api/providers/{id}/oauth/chatgpt/start",
            post(start_chatgpt_oauth),
        )
        .route(
            "/api/providers/{id}/oauth/chatgpt/exchange",
            post(exchange_chatgpt_oauth),
        )
        .route("/api/oauth/chatgpt/callback", get(chatgpt_oauth_callback))
        .route("/api/memories", get(list_memories).post(create_memory))
        .route("/api/memories/export", get(export_memories))
        .route(
            "/api/memories/{id}",
            get(get_memory)
                .patch(update_memory)
                .put(update_memory)
                .delete(delete_memory),
        )
        .route(
            "/api/extensions",
            get(list_extensions).post(create_extension),
        )
        .route("/api/extensions/inventory", get(extensions_inventory))
        .route(
            "/api/extensions/{id}",
            patch(update_extension).delete(delete_extension),
        )
        .route(
            "/api/extensions/{id}/discover",
            post(discover_extension_tools),
        )
        .route(
            "/api/extensions/{id}/permissions",
            get(list_extension_permissions).post(grant_extension_permission),
        )
        .route("/api/recipes", get(list_recipes).post(create_recipe))
        .route(
            "/api/recipes/{id}",
            get(get_recipe)
                .put(update_recipe)
                .patch(update_recipe)
                .delete(delete_recipe),
        )
        .route("/api/recipes/{id}/execute", post(execute_recipe))
        .route("/api/schedules", get(list_schedules).post(create_schedule))
        .route(
            "/api/schedules/{id}",
            get(get_schedule)
                .patch(update_schedule)
                .delete(delete_schedule),
        )
        .route("/api/chats", post(create_chat))
        .route("/api/chats/{id}", get(get_chat).patch(update_chat))
        .route(
            "/api/chats/{id}/messages",
            get(get_chat_messages).post(submit_chat_message),
        )
        .route("/api/chats/{id}/compact", post(compact_chat))
        .route("/api/chats/{id}/export", get(export_chat))
        .route("/api/worktrees/{id}/git-status", get(worktree_git_status))
        .route("/api/worktrees/{id}/diff", get(worktree_git_diff))
        .route("/api/worktrees/{id}/merge", post(merge_worktree))
        .route("/api/worktrees/{id}/discard", post(discard_worktree))
        .route("/api/runs/{id}/cancel", post(cancel_run))
        .route("/api/runs/{id}/events", get(stream_run_events))
        .route("/api/approvals/pending", get(list_pending_approvals))
        .route("/api/approvals/{id}/decide", post(decide_approval))
        .merge(terminal_routes())
        .layer(CorsLayer::permissive())
        .with_state(state)
}

pub async fn initialize_server_state(pool: SqlitePool) -> Result<AppState> {
    register_builtin_extensions(&pool).await?;
    let build = BuildIdentity::current();
    let engine = RunEngine::new(pool.clone());
    let recovered = engine.recover_unfinished_runs().await?;
    if !recovered.is_empty() {
        info!(
            count = recovered.len(),
            version = %build.version,
            target = %build.target,
            "Recovered durable runs on startup"
        );
    }
    info!(
        version = %build.version,
        commit = %build.commit,
        profile = %build.profile,
        target = %build.target,
        "Gaggle core initialized"
    );
    let db_path = pool.connect_options().get_filename().to_path_buf();
    let credential_store: Arc<dyn CredentialStore> = Arc::new(InMemoryCredentialStore::new());
    let active_runs = ActiveRunRegistry::new();
    let terminal_manager = Arc::new(TerminalManager::new(pool.clone()));
    let state = AppState {
        pool,
        db_path,
        credential_store,
        active_runs,
        oauth_sessions: Arc::new(RwLock::new(HashMap::new())),
        terminal_manager,
    };
    let _ = missed_run_catchup(&state).await;
    Ok(state)
}

pub async fn serve(pool: SqlitePool, addr: SocketAddr) -> Result<()> {
    let state = initialize_server_state(pool).await?;
    let browser_pool = state.pool.clone();
    let tm = state.terminal_manager.clone();
    let _ = missed_run_catchup(&state).await;
    let _scheduler = spawn_recipe_scheduler(state.clone());
    let app = build_router(state);
    let listener = tokio::net::TcpListener::bind(addr).await?;
    info!(%addr, "Gaggle server listening");
    let res = axum::serve(listener, app).await;
    tm.shutdown_all().await;
    let _ = gaggle_tools::close_all_browsers_with_pool(&browser_pool).await;
    res?;
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

/// Explicit user-initiated export. Produces a redacted bundle with recent failure summaries and
/// never includes prompts, source files, tool arguments, or raw tool output.
async fn diagnostics_export_handler(
    State(state): State<AppState>,
) -> Result<Json<DiagnosticsExport>, (StatusCode, String)> {
    let export = export_diagnostics(&state.pool, &state.db_path)
        .await
        .map_err(|e| {
            let failure = ActionableError::new(
                FailureCategory::Database,
                "diagnostics.export",
                e.to_string(),
            )
            .with_data_preservation(DataPreservation::Preserved);
            (StatusCode::INTERNAL_SERVER_ERROR, failure.to_string())
        })?;
    tracing::info!(
        target: "gaggle.diagnostics",
        failure_count = export.recent_failures.len(),
        telemetry_enabled = export.privacy.telemetry_enabled,
        "Redacted diagnostics export generated on explicit request"
    );
    Ok(Json(export))
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

async fn update_project(
    State(state): State<AppState>,
    Path(id): Path<String>,
    Json(req): Json<UpdateProjectRequest>,
) -> Result<Json<Project>, (StatusCode, String)> {
    if let Some(mode) = &req.default_mode
        && !matches!(mode.as_str(), "local" | "worktree" | "readonly")
    {
        return Err((
            StatusCode::BAD_REQUEST,
            "default_mode must be one of: local, worktree, readonly".to_string(),
        ));
    }
    if let Some(access) = &req.default_access
        && !matches!(access.as_str(), "ask" | "full")
    {
        return Err((
            StatusCode::BAD_REQUEST,
            "default_access must be one of: ask, full".to_string(),
        ));
    }

    let repo = ProjectRepository::new(state.pool);
    let updated = repo
        .update_defaults(
            &id,
            req.default_mode.as_deref(),
            req.default_access.as_deref(),
            req.default_model_id.as_deref(),
        )
        .await
        .map_err(|e| (StatusCode::BAD_REQUEST, e.to_string()))?;
    Ok(Json(updated))
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

async fn get_chat(
    State(state): State<AppState>,
    Path(chat_id): Path<String>,
) -> Result<Json<Chat>, (StatusCode, String)> {
    let repo = ChatRepository::new(state.pool);
    let chat = repo
        .get_by_id(&chat_id)
        .await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?
        .ok_or_else(|| (StatusCode::NOT_FOUND, format!("Chat '{chat_id}' not found")))?;
    Ok(Json(chat))
}

async fn get_chat_messages(
    State(state): State<AppState>,
    Path(chat_id): Path<String>,
) -> Result<Json<ChatMessagesResponse>, (StatusCode, String)> {
    let repo = ChatRepository::new(state.pool);
    let rows = repo
        .list_messages_with_parts(&chat_id, 500)
        .await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;
    let mut messages = Vec::with_capacity(rows.len());
    let mut message_parts = Vec::new();
    for (msg, parts) in rows {
        messages.push(msg);
        message_parts.extend(parts);
    }
    Ok(Json(ChatMessagesResponse {
        messages,
        message_parts,
    }))
}

async fn list_recipes(
    State(state): State<AppState>,
) -> Result<Json<Vec<Recipe>>, (StatusCode, String)> {
    let repo = RecipeRepository::new(state.pool);
    let recipes = repo
        .list_all()
        .await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;
    Ok(Json(recipes))
}

async fn get_recipe(
    State(state): State<AppState>,
    Path(id): Path<String>,
) -> Result<Json<Recipe>, (StatusCode, String)> {
    let repo = RecipeRepository::new(state.pool);
    let recipe = repo
        .get_by_id(&id)
        .await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?
        .ok_or_else(|| (StatusCode::NOT_FOUND, format!("Recipe {id} not found")))?;
    Ok(Json(recipe))
}

async fn create_recipe(
    State(state): State<AppState>,
    Json(req): Json<CreateRecipeRequest>,
) -> Result<(StatusCode, Json<Recipe>), (StatusCode, String)> {
    let source_kind = req
        .source_kind
        .clone()
        .unwrap_or_else(|| "inline".to_string());
    let definition = req.definition.clone().unwrap_or_else(|| {
        serde_json::json!({
            "name": req.name,
            "title": req.title,
            "description": req.description,
            "instructions": req.instructions,
            "prompt": req.prompt,
            "parameters": req.parameters,
            "extensions": req.extensions,
            "sub_recipes": req.sub_recipes,
        })
        .to_string()
    });

    let repo = RecipeRepository::new(state.pool.clone());
    let recipe = repo
        .create_recipe_full(
            &req.name,
            &req.title,
            req.description.as_deref(),
            &source_kind,
            req.source_ref.as_deref(),
            req.instructions.as_deref(),
            req.prompt.as_deref(),
            &definition,
            req.default_model_id.as_deref(),
        )
        .await
        .map_err(|e| (StatusCode::BAD_REQUEST, e.to_string()))?;

    // Parameters
    if let Some(params) = &req.parameters {
        for (idx, p) in params.iter().enumerate() {
            let required = p.required.unwrap_or(1);
            let default_value = p.default_value.clone();
            if required == 1 && default_value.is_some() {
                return Err((
                    StatusCode::BAD_REQUEST,
                    format!("Parameter '{}' required but has default", p.name),
                ));
            }
            if required == 0 && default_value.is_none() {
                return Err((
                    StatusCode::BAD_REQUEST,
                    format!("Parameter '{}' optional but missing default", p.name),
                ));
            }
            repo.create_parameter(
                &recipe.id,
                &p.name,
                p.param_type.as_deref().unwrap_or("string"),
                p.description.as_deref(),
                required,
                default_value.as_deref(),
                p.idx.unwrap_or(idx as i64),
            )
            .await
            .map_err(|e| (StatusCode::BAD_REQUEST, e.to_string()))?;
        }
    }

    // Extensions: resolve by id or name.
    if let Some(exts) = &req.extensions {
        let ext_repo = ExtensionRepository::new(state.pool.clone());
        let all = ext_repo
            .list_all_extensions()
            .await
            .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;
        for ext_ref in exts {
            let found = all.iter().find(|e| e.id == *ext_ref || e.name == *ext_ref);
            if let Some(ext) = found {
                repo.add_extension(&recipe.id, &ext.id)
                    .await
                    .map_err(|e| (StatusCode::BAD_REQUEST, e.to_string()))?;
            } else {
                return Err((
                    StatusCode::BAD_REQUEST,
                    format!("Extension '{ext_ref}' not found"),
                ));
            }
        }
    }

    // Sub-recipes
    if let Some(subs) = &req.sub_recipes {
        for (idx, s) in subs.iter().enumerate() {
            repo.create_sub_recipe(
                &recipe.id,
                &s.child_recipe_id,
                &s.name,
                s.values_json.as_deref(),
                s.sequential_when_repeated.unwrap_or(0),
                s.idx.unwrap_or(idx as i64),
            )
            .await
            .map_err(|e| (StatusCode::BAD_REQUEST, e.to_string()))?;
        }
    }

    Ok((StatusCode::CREATED, Json(recipe)))
}

async fn update_recipe(
    State(state): State<AppState>,
    Path(id): Path<String>,
    Json(req): Json<UpdateRecipeRequest>,
) -> Result<Json<Recipe>, (StatusCode, String)> {
    if let Some(source_kind) = &req.source_kind
        && !matches!(source_kind.as_str(), "inline" | "file" | "github" | "url")
    {
        return Err((
            StatusCode::BAD_REQUEST,
            "source_kind must be one of: inline, file, github, url".to_string(),
        ));
    }

    let repo = RecipeRepository::new(state.pool.clone());
    repo.get_by_id(&id)
        .await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?
        .ok_or_else(|| (StatusCode::NOT_FOUND, format!("Recipe {id} not found")))?;

    let _updated = repo
        .update_recipe(
            &id,
            req.name.as_deref(),
            req.title.as_deref(),
            req.description.as_deref(),
            req.source_kind.as_deref(),
            req.source_ref.as_deref(),
            req.instructions.as_deref(),
            req.prompt.as_deref(),
            req.definition.as_deref(),
            req.default_model_id.as_deref(),
        )
        .await
        .map_err(|e| (StatusCode::BAD_REQUEST, e.to_string()))?;

    // Replace parameters if provided.
    if let Some(params) = &req.parameters {
        repo.delete_parameters(&id)
            .await
            .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;
        for (idx, p) in params.iter().enumerate() {
            let required = p.required.unwrap_or(1);
            let default_value = p.default_value.clone();
            if required == 1 && default_value.is_some() {
                return Err((
                    StatusCode::BAD_REQUEST,
                    format!("Parameter '{}' required but has default", p.name),
                ));
            }
            if required == 0 && default_value.is_none() {
                return Err((
                    StatusCode::BAD_REQUEST,
                    format!("Parameter '{}' optional but missing default", p.name),
                ));
            }
            repo.create_parameter(
                &id,
                &p.name,
                p.param_type.as_deref().unwrap_or("string"),
                p.description.as_deref(),
                required,
                default_value.as_deref(),
                p.idx.unwrap_or(idx as i64),
            )
            .await
            .map_err(|e| (StatusCode::BAD_REQUEST, e.to_string()))?;
        }
    }

    // Replace extensions if provided.
    if let Some(exts) = &req.extensions {
        let existing = repo
            .list_recipe_extensions(&id)
            .await
            .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;
        for re in existing {
            repo.remove_extension(&id, &re.extension_id)
                .await
                .map_err(|e| (StatusCode::BAD_REQUEST, e.to_string()))?;
        }
        let ext_repo = ExtensionRepository::new(state.pool.clone());
        let all = ext_repo
            .list_all_extensions()
            .await
            .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;
        for ext_ref in exts {
            let found = all.iter().find(|e| e.id == *ext_ref || e.name == *ext_ref);
            if let Some(ext) = found {
                repo.add_extension(&id, &ext.id)
                    .await
                    .map_err(|e| (StatusCode::BAD_REQUEST, e.to_string()))?;
            } else {
                return Err((
                    StatusCode::BAD_REQUEST,
                    format!("Extension '{ext_ref}' not found"),
                ));
            }
        }
    }

    // Replace sub-recipes if provided.
    if let Some(subs) = &req.sub_recipes {
        repo.delete_sub_recipes(&id)
            .await
            .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;
        for (idx, s) in subs.iter().enumerate() {
            repo.create_sub_recipe(
                &id,
                &s.child_recipe_id,
                &s.name,
                s.values_json.as_deref(),
                s.sequential_when_repeated.unwrap_or(0),
                s.idx.unwrap_or(idx as i64),
            )
            .await
            .map_err(|e| (StatusCode::BAD_REQUEST, e.to_string()))?;
        }
    }

    let recipe = repo
        .get_by_id(&id)
        .await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?
        .ok_or_else(|| (StatusCode::NOT_FOUND, format!("Recipe {id} not found")))?;
    Ok(Json(recipe))
}

async fn delete_recipe(
    State(state): State<AppState>,
    Path(id): Path<String>,
) -> Result<StatusCode, (StatusCode, String)> {
    let repo = RecipeRepository::new(state.pool);
    let deleted = repo
        .delete_recipe(&id)
        .await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;
    if !deleted {
        return Err((StatusCode::NOT_FOUND, format!("Recipe {id} not found")));
    }
    Ok(StatusCode::NO_CONTENT)
}

async fn execute_recipe(
    State(state): State<AppState>,
    Path(id): Path<String>,
    Json(req): Json<ExecuteRecipeRequest>,
) -> Result<(StatusCode, Json<Chat>), (StatusCode, String)> {
    let recipe_repo = RecipeRepository::new(state.pool.clone());
    let recipe = recipe_repo
        .get_by_id(&id)
        .await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?
        .ok_or_else(|| (StatusCode::NOT_FOUND, format!("Recipe {id} not found")))?;

    // Recursion/cycle guard for recipe sub-recipes.
    if recipe_graph_has_cycle(&recipe_repo, &recipe.id).await? {
        return Err((
            StatusCode::BAD_REQUEST,
            "Recipe recursion or cycle detected".to_string(),
        ));
    }

    // Load params specs
    let specs = recipe_repo
        .list_parameters(&recipe.id)
        .await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;

    // Convert DB specs to gaggle_db model (already) and provided values
    let provided: HashMap<String, String> = req
        .param_values
        .as_ref()
        .and_then(|v| v.as_object())
        .map(|obj| {
            obj.iter()
                .map(|(k, v)| {
                    let s = match v {
                        serde_json::Value::String(s) => s.clone(),
                        other => other.to_string(),
                    };
                    (k.clone(), s)
                })
                .collect()
        })
        .unwrap_or_default();

    // Validate specs
    validate_recipe_parameters(&specs).map_err(|e| (StatusCode::BAD_REQUEST, e.to_string()))?;

    let resolved = resolve_recipe_parameters(&specs, &provided)
        .map_err(|e| (StatusCode::BAD_REQUEST, e.to_string()))?;

    // Render instructions/prompt
    let (rendered_instructions, rendered_prompt) = {
        let env_values = resolved.clone();
        // also include raw provided? enough
        let instr = recipe
            .instructions
            .as_deref()
            .map(|t| render_recipe_template(t, &env_values))
            .transpose()
            .map_err(|e| (StatusCode::BAD_REQUEST, e.to_string()))?;
        let prmpt = recipe
            .prompt
            .as_deref()
            .map(|t| render_recipe_template(t, &env_values))
            .transpose()
            .map_err(|e| (StatusCode::BAD_REQUEST, e.to_string()))?;
        (instr, prmpt)
    };

    let chat_type = req.chat_type.clone().unwrap_or_else(|| "user".to_string());
    let mode = req.mode.clone().unwrap_or_else(|| "local".to_string());
    let permission_mode = req
        .permission_mode
        .clone()
        .unwrap_or_else(|| "ask".to_string());
    let title = req.title.clone().unwrap_or_else(|| recipe.title.clone());

    let recipe_values_json = serde_json::to_string(&resolved).unwrap_or_else(|_| "{}".to_string());

    let chat_repo = ChatRepository::new(state.pool.clone());
    let chat = chat_repo
        .create_recipe_chat(
            &req.project_id,
            &title,
            &chat_type,
            &mode,
            &permission_mode,
            Some(&recipe.id),
            Some(&recipe_values_json),
            None,
            req.parent_chat_id.as_deref(),
            req.parent_tool_call_id.as_deref(),
        )
        .await
        .map_err(|e| (StatusCode::BAD_REQUEST, e.to_string()))?;

    // Activate required extensions in chat_extensions
    let ext_repo = ExtensionRepository::new(state.pool.clone());
    let ext_ids = recipe_repo
        .list_extension_ids(&recipe.id)
        .await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;
    for ext_id in ext_ids {
        ext_repo
            .enable_extension_for_chat(&chat.id, &ext_id)
            .await
            .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;
    }

    // Optionally store rendered instructions as system prompt override? Set system_prompt_override maybe not needed.
    // We could append rendered prompt as a message? Keep minimal.

    let _ = (rendered_instructions, rendered_prompt);

    // Execute (create child chats for) any sub-recipes attached to this recipe.
    // Child chats are bound to the main chat as subagents.
    let sub_recipes = recipe_repo
        .list_sub_recipes(&recipe.id)
        .await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;

    let ext_repo = ExtensionRepository::new(state.pool.clone());
    for sub in sub_recipes {
        let child_recipe = recipe_repo
            .get_by_id(&sub.child_recipe_id)
            .await
            .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?
            .ok_or_else(|| {
                (
                    StatusCode::NOT_FOUND,
                    format!("Recipe {} not found", sub.child_recipe_id),
                )
            })?;

        let title = if sub.name.trim().is_empty() {
            child_recipe.title.clone()
        } else {
            sub.name.clone()
        };

        let child_chat = chat_repo
            .create_recipe_chat(
                &req.project_id,
                &title,
                "subagent",
                &mode,
                &permission_mode,
                Some(&child_recipe.id),
                sub.values_json.as_deref(),
                None,
                Some(&chat.id),
                req.parent_tool_call_id.as_deref(),
            )
            .await
            .map_err(|e| (StatusCode::BAD_REQUEST, e.to_string()))?;

        // Activate required extensions for the child recipe chat.
        let child_ext_ids = recipe_repo
            .list_extension_ids(&child_recipe.id)
            .await
            .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;
        for ext_id in child_ext_ids {
            ext_repo
                .enable_extension_for_chat(&child_chat.id, &ext_id)
                .await
                .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;
        }

        // `sequential_when_repeated` is enforced by creating these child chats
        // sequentially (awaiting each creation) rather than in parallel.
        let _ = sub.sequential_when_repeated;
    }

    Ok((StatusCode::CREATED, Json(chat)))
}

async fn list_schedules(
    State(state): State<AppState>,
    Query(query): Query<ScheduleQuery>,
) -> Result<Json<Vec<Schedule>>, (StatusCode, String)> {
    let repo = RecipeRepository::new(state.pool);
    let schedules = repo
        .list_schedules(query.project_id.as_deref())
        .await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;
    Ok(Json(schedules))
}

#[derive(Debug, Deserialize, Serialize)]
pub struct ScheduleQuery {
    pub project_id: Option<String>,
}

async fn get_schedule(
    State(state): State<AppState>,
    Path(id): Path<String>,
) -> Result<Json<Schedule>, (StatusCode, String)> {
    let repo = RecipeRepository::new(state.pool);
    let sched = repo
        .get_schedule(&id)
        .await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?
        .ok_or_else(|| (StatusCode::NOT_FOUND, format!("Schedule {id} not found")))?;
    Ok(Json(sched))
}

async fn create_schedule(
    State(state): State<AppState>,
    Json(req): Json<CreateScheduleRequest>,
) -> Result<(StatusCode, Json<Schedule>), (StatusCode, String)> {
    let timezone = req.timezone.clone().unwrap_or_else(|| "UTC".to_string());
    let param_values_str = req.param_values.as_ref().map(|v| v.to_string());

    // compute next_run_at
    let now = now_ms();
    let next_run = compute_next_run(&req.cron_expr, &timezone, now)
        .map_err(|e| (StatusCode::BAD_REQUEST, e.to_string()))?;

    let enabled = req.enabled.unwrap_or(1);
    let repo = RecipeRepository::new(state.pool.clone());
    // validate recipe exists
    repo.get_by_id(&req.recipe_id)
        .await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?
        .ok_or_else(|| {
            (
                StatusCode::NOT_FOUND,
                format!("Recipe {} not found", req.recipe_id),
            )
        })?;

    let sched = repo
        .create_schedule_with_options(
            &req.recipe_id,
            req.project_id.as_deref(),
            &req.cron_expr,
            &timezone,
            param_values_str.as_deref(),
            enabled,
            Some(next_run),
        )
        .await
        .map_err(|e| (StatusCode::BAD_REQUEST, e.to_string()))?;

    Ok((StatusCode::CREATED, Json(sched)))
}

async fn update_schedule(
    State(state): State<AppState>,
    Path(id): Path<String>,
    Json(req): Json<UpdateScheduleRequest>,
) -> Result<Json<Schedule>, (StatusCode, String)> {
    let repo = RecipeRepository::new(state.pool.clone());

    let existing = repo
        .get_schedule(&id)
        .await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?
        .ok_or_else(|| (StatusCode::NOT_FOUND, format!("Schedule {id} not found")))?;

    if let Some(enabled) = req.enabled {
        repo.set_schedule_enabled(&id, enabled != 0)
            .await
            .map_err(|e| (StatusCode::BAD_REQUEST, e.to_string()))?;
    }

    // Persist param_values when provided (including null to clear).
    if req.param_values.is_some() {
        let param_values_str = req.param_values.as_ref().and_then(|v| {
            if v.is_null() {
                None
            } else {
                Some(v.to_string())
            }
        });
        sqlx::query("UPDATE schedules SET param_values = ? WHERE id = ?")
            .bind(param_values_str)
            .bind(&id)
            .execute(&state.pool)
            .await
            .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;
    }

    // Update cron/timezone when either is provided, validating timezone.
    if req.cron_expr.is_some() || req.timezone.is_some() {
        let cron_expr = req.cron_expr.as_deref().unwrap_or(&existing.cron_expr);
        let timezone = req.timezone.as_deref().unwrap_or(&existing.timezone);

        validate_timezone(timezone).map_err(|e| (StatusCode::BAD_REQUEST, e.to_string()))?;

        let now = now_ms();
        let next = compute_next_run(cron_expr, timezone, now)
            .map_err(|e| (StatusCode::BAD_REQUEST, e.to_string()))?;

        sqlx::query(
            "UPDATE schedules SET cron_expr = ?, timezone = ?, next_run_at = ? WHERE id = ?",
        )
        .bind(cron_expr)
        .bind(timezone)
        .bind(next)
        .bind(&id)
        .execute(&state.pool)
        .await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;
    }

    let sched = repo
        .get_schedule(&id)
        .await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?
        .ok_or_else(|| (StatusCode::NOT_FOUND, format!("Schedule {id} not found")))?;
    Ok(Json(sched))
}

#[derive(Debug, Deserialize, Serialize)]
pub struct UpdateScheduleRequest {
    pub enabled: Option<i64>,
    pub cron_expr: Option<String>,
    pub timezone: Option<String>,
    pub param_values: Option<serde_json::Value>,
}

async fn delete_schedule(
    State(state): State<AppState>,
    Path(id): Path<String>,
) -> Result<StatusCode, (StatusCode, String)> {
    let repo = RecipeRepository::new(state.pool);
    let deleted = repo
        .delete_schedule(&id)
        .await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;
    if !deleted {
        return Err((StatusCode::NOT_FOUND, format!("Schedule {id} not found")));
    }
    Ok(StatusCode::NO_CONTENT)
}

async fn create_chat(
    State(state): State<AppState>,
    Json(req): Json<CreateChatRequest>,
) -> Result<(StatusCode, Json<Chat>), (StatusCode, String)> {
    let chat_repo = ChatRepository::new(state.pool.clone());
    let mode = req.mode.as_deref().unwrap_or("local");
    let permission_mode = req.permission_mode.as_deref().unwrap_or("ask");
    let chat_type = req.chat_type.as_deref().unwrap_or("user");
    if !matches!(
        chat_type,
        "user" | "subagent" | "scheduled" | "hidden" | "terminal"
    ) {
        return Err((
            StatusCode::BAD_REQUEST,
            "chat_type must be one of: user, subagent, scheduled, hidden, terminal".to_string(),
        ));
    }
    let chat = chat_repo
        .create_child_chat(
            &req.project_id,
            &req.title,
            chat_type,
            mode,
            permission_mode,
            req.parent_chat_id.as_deref(),
            req.parent_tool_call_id.as_deref(),
        )
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

async fn list_project_command_rules(
    State(state): State<AppState>,
    Path(project_id): Path<String>,
) -> Result<Json<Vec<CommandRule>>, (StatusCode, String)> {
    let repo = PermissionRepository::new(state.pool);
    let rules = repo
        .list_command_rules(Some(&project_id))
        .await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;
    Ok(Json(rules))
}

async fn create_project_command_rule(
    State(state): State<AppState>,
    Path(project_id): Path<String>,
    Json(req): Json<CreateCommandRuleRequest>,
) -> Result<(StatusCode, Json<CommandRule>), (StatusCode, String)> {
    let pattern = req.pattern.trim();
    if pattern.is_empty() {
        return Err((
            StatusCode::BAD_REQUEST,
            "Command rule pattern must not be empty".to_string(),
        ));
    }
    let match_kind = req.match_kind.as_deref().unwrap_or("exact");
    if !matches!(match_kind, "exact" | "prefix" | "regex") {
        return Err((
            StatusCode::BAD_REQUEST,
            "match_kind must be one of: exact, prefix, regex".to_string(),
        ));
    }
    if !matches!(req.action.as_str(), "allow" | "deny" | "ask") {
        return Err((
            StatusCode::BAD_REQUEST,
            "action must be one of: allow, deny, ask".to_string(),
        ));
    }

    let repo = PermissionRepository::new(state.pool);
    let rule = repo
        .add_command_rule(Some(&project_id), pattern, match_kind, &req.action)
        .await
        .map_err(|e| (StatusCode::BAD_REQUEST, e.to_string()))?;
    Ok((StatusCode::CREATED, Json(rule)))
}

async fn list_project_tool_permissions(
    State(state): State<AppState>,
    Path(project_id): Path<String>,
) -> Result<Json<Vec<ToolPermissionStatus>>, (StatusCode, String)> {
    let ext_repo = ExtensionRepository::new(state.pool.clone());
    let perm_repo = PermissionRepository::new(state.pool);

    let extensions = ext_repo
        .list_enabled_extensions()
        .await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;

    let mut statuses = Vec::new();
    for extension in extensions {
        let tools = ext_repo
            .list_tools_for_extension(&extension.id)
            .await
            .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;
        for tool in tools.into_iter().filter(|tool| tool.enabled == 1) {
            let effective = perm_repo
                .get_tool_permission(Some(&project_id), &tool.id)
                .await
                .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;
            statuses.push(ToolPermissionStatus {
                extension_tool_id: tool.id,
                extension_name: extension.name.clone(),
                tool_name: tool.tool_name,
                is_read_only: tool.is_read_only == 1,
                level: effective.map(|perm| perm.level),
            });
        }
    }

    Ok(Json(statuses))
}

async fn set_project_tool_permission(
    State(state): State<AppState>,
    Path(project_id): Path<String>,
    Json(req): Json<SetToolPermissionRequest>,
) -> Result<Json<ToolPermissionStatus>, (StatusCode, String)> {
    if !matches!(
        req.level.as_str(),
        "always_allow" | "ask_before" | "never_allow"
    ) {
        return Err((
            StatusCode::BAD_REQUEST,
            "level must be one of: always_allow, ask_before, never_allow".to_string(),
        ));
    }

    let ext_repo = ExtensionRepository::new(state.pool.clone());
    let perm_repo = PermissionRepository::new(state.pool.clone());
    let tool = sqlx::query_as::<_, ExtensionTool>("SELECT * FROM extension_tools WHERE id = ?")
        .bind(&req.extension_tool_id)
        .fetch_optional(&state.pool)
        .await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?
        .ok_or_else(|| {
            (
                StatusCode::NOT_FOUND,
                format!("Extension tool {} not found", req.extension_tool_id),
            )
        })?;

    let permission = perm_repo
        .upsert_tool_permission(Some(&project_id), &tool.id, &req.level)
        .await
        .map_err(|e| (StatusCode::BAD_REQUEST, e.to_string()))?;

    let extension_name = ext_repo
        .get_extension_by_id(&tool.extension_id)
        .await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?
        .map(|extension| extension.name)
        .unwrap_or_else(|| "unknown".to_string());

    Ok(Json(ToolPermissionStatus {
        extension_tool_id: tool.id,
        extension_name,
        tool_name: tool.tool_name,
        is_read_only: tool.is_read_only == 1,
        level: Some(permission.level),
    }))
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

async fn extensions_inventory(
    State(state): State<AppState>,
) -> Result<Json<ExtensionsInventoryResponse>, (StatusCode, String)> {
    let repo = ExtensionRepository::new(state.pool.clone());

    let extensions = repo
        .list_all_extensions()
        .await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;
    let active_tool_count = repo
        .count_active_tools()
        .await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;

    let mut items = Vec::with_capacity(extensions.len());
    for extension in extensions {
        let tools = repo
            .list_tools_for_extension(&extension.id)
            .await
            .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;
        items.push(ExtensionInventoryItem {
            tool_count: tools.len() as i64,
            tools: tools
                .into_iter()
                .map(|tool| ExtensionToolSummary {
                    tool_name: tool.tool_name,
                    is_read_only: tool.is_read_only == 1,
                })
                .collect(),
            extension,
        });
    }

    Ok(Json(ExtensionsInventoryResponse {
        recommended_max_active_tools: RECOMMENDED_MAX_ACTIVE_TOOLS,
        active_tool_count,
        exceeds_recommended_tool_limit: ExtensionRegistry::exceeds_recommended_tool_limit(
            active_tool_count as usize,
        ),
        extensions: items,
    }))
}

async fn update_extension(
    State(state): State<AppState>,
    Path(id): Path<String>,
    Json(req): Json<UpdateExtensionRequest>,
) -> Result<Json<Extension>, (StatusCode, String)> {
    let repo = ExtensionRepository::new(state.pool);
    let updated = repo
        .set_enabled(&id, req.enabled)
        .await
        .map_err(|e| (StatusCode::BAD_REQUEST, e.to_string()))?;
    Ok(Json(updated))
}

async fn create_extension(
    State(state): State<AppState>,
    Json(req): Json<CreateExtensionRequest>,
) -> Result<Json<Extension>, (StatusCode, String)> {
    let registry = ExtensionRegistry::new(state.pool.clone());
    let install_req = InstallExtensionRequest {
        name: req.name,
        description: req.description,
        kind: req.kind,
        version: req.version.unwrap_or_else(|| "0.1.0".to_string()),
        source: req.source.unwrap_or_else(|| "local".to_string()),
        command: req.command,
        args: req.args.unwrap_or_default(),
        url: req.url,
        env_secret_refs: req.env_secret_refs,
        timeout_secs: req.timeout_secs.unwrap_or(120),
        manifest: req.manifest,
        bundled: req.bundled.unwrap_or(false),
        enabled: req.enabled.unwrap_or(true),
    };
    let ext = registry
        .install_extension(install_req)
        .await
        .map_err(|e| (StatusCode::BAD_REQUEST, e.to_string()))?;
    Ok(Json(ext))
}

async fn delete_extension(
    State(state): State<AppState>,
    Path(id): Path<String>,
) -> Result<StatusCode, (StatusCode, String)> {
    let registry = ExtensionRegistry::new(state.pool);
    registry
        .uninstall_extension(&id)
        .await
        .map_err(|e| (StatusCode::BAD_REQUEST, e.to_string()))?;
    Ok(StatusCode::NO_CONTENT)
}

async fn discover_extension_tools(
    State(state): State<AppState>,
    Path(id): Path<String>,
) -> Result<Json<Vec<ExtensionTool>>, (StatusCode, String)> {
    let registry = ExtensionRegistry::new(state.pool);
    let tools = registry
        .discover_tools(&id)
        .await
        .map_err(|e| (StatusCode::BAD_REQUEST, e.to_string()))?;
    Ok(Json(tools))
}

async fn list_extension_permissions(
    State(state): State<AppState>,
    Path(id): Path<String>,
) -> Result<Json<Vec<ExtensionPermission>>, (StatusCode, String)> {
    let registry = ExtensionRegistry::new(state.pool);
    let perms = registry
        .list_permissions(&id)
        .await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;
    Ok(Json(perms))
}

async fn grant_extension_permission(
    State(state): State<AppState>,
    Path(id): Path<String>,
    Json(req): Json<GrantExtensionPermissionRequest>,
) -> Result<Json<ExtensionPermission>, (StatusCode, String)> {
    let registry = ExtensionRegistry::new(state.pool);
    let perm = registry
        .grant_permission(&id, &req.permission, req.scope.as_deref())
        .await
        .map_err(|e| (StatusCode::BAD_REQUEST, e.to_string()))?;
    Ok(Json(perm))
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
        Some(url) if !url.trim().is_empty() => Some(
            validate_provider_base_url(url)
                .map_err(|e| (StatusCode::BAD_REQUEST, e.to_string()))?,
        ),
        _ => None,
    };

    // A provider credential is stored exclusively in the CredentialStore. SQLite only
    // ever receives the generated `secret_ref` identifier, never the raw key.
    let secret_ref = req.secret_ref.clone().or_else(|| {
        if req.api_key.is_some() || req.auth_kind == "oauth" {
            Some(format!("provider:{}", new_id()))
        } else {
            None
        }
    });

    if let Some(raw_key) = &req.api_key {
        let secret_ref = secret_ref.as_deref().ok_or_else(|| {
            (
                StatusCode::BAD_REQUEST,
                "A secret_ref is required when an api_key is supplied".to_string(),
            )
        })?;
        let secret =
            SecretString::new(raw_key).map_err(|e| (StatusCode::BAD_REQUEST, e.to_string()))?;
        state
            .credential_store
            .set_secret(secret_ref, secret)
            .map_err(|e| (StatusCode::BAD_REQUEST, e.to_string()))?;
    }

    let repo = ProviderRepository::new(state.pool);
    let provider = repo
        .create_provider(
            &req.kind,
            &req.name,
            validated_url.as_deref(),
            &req.auth_kind,
            secret_ref.as_deref(),
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

const DEFAULT_PROVIDER_TIMEOUT_MS: u64 = 120_000;

fn default_base_url(kind: &str) -> Option<&'static str> {
    match kind {
        "openai" | "chatgpt" => Some("https://api.openai.com/v1"),
        "anthropic" => Some("https://api.anthropic.com/v1"),
        "openrouter" => Some("https://openrouter.ai/api/v1"),
        "deepseek" => Some("https://api.deepseek.com/v1"),
        "ollama" => Some("http://127.0.0.1:11434"),
        _ => None,
    }
}

fn chatgpt_oauth_config(redirect_uri: Option<String>) -> Option<ChatGptOAuthConfig> {
    let client_id = std::env::var("GAGGLE_CHATGPT_CLIENT_ID").ok()?;
    let auth_url = std::env::var("GAGGLE_CHATGPT_AUTH_URL")
        .unwrap_or_else(|_| "https://auth.openai.com/oauth/authorize".to_string());
    let token_url = std::env::var("GAGGLE_CHATGPT_TOKEN_URL")
        .unwrap_or_else(|_| "https://auth.openai.com/oauth/token".to_string());
    let redirect_uri = redirect_uri
        .or_else(|| std::env::var("GAGGLE_CHATGPT_REDIRECT_URI").ok())
        .unwrap_or_else(|| "http://127.0.0.1:4321/api/oauth/chatgpt/callback".to_string());
    Some(ChatGptOAuthConfig {
        client_id,
        auth_url,
        token_url,
        redirect_uri,
        scopes: vec![
            "openid".to_string(),
            "profile".to_string(),
            "email".to_string(),
            "offline_access".to_string(),
        ],
    })
}

fn build_provider_adapter(
    state: &AppState,
    provider: &Provider,
) -> Result<Box<dyn ModelProviderAdapter>, (StatusCode, String)> {
    let base_url = provider
        .base_url
        .clone()
        .or_else(|| default_base_url(&provider.kind).map(String::from))
        .ok_or_else(|| {
            (
                StatusCode::BAD_REQUEST,
                format!(
                    "Provider '{}' of kind '{}' requires a base_url",
                    provider.name, provider.kind
                ),
            )
        })?;

    match provider.kind.as_str() {
        "anthropic" => AnthropicAdapter::new(
            provider.name.clone(),
            &base_url,
            provider.secret_ref.clone(),
            state.credential_store.clone(),
            DEFAULT_PROVIDER_TIMEOUT_MS,
        )
        .map(|adapter| Box::new(adapter) as Box<dyn ModelProviderAdapter>),
        "ollama" => OllamaAdapter::new(
            provider.name.clone(),
            &base_url,
            DEFAULT_PROVIDER_TIMEOUT_MS,
        )
        .map(|adapter| Box::new(adapter) as Box<dyn ModelProviderAdapter>),
        "chatgpt" => {
            let oauth = chatgpt_oauth_config(None).and_then(|config| {
                ChatGptOAuthClient::new(
                    config,
                    state.credential_store.clone(),
                    DEFAULT_PROVIDER_TIMEOUT_MS,
                )
                .ok()
            });
            ChatGptAdapter::new(
                provider.name.clone(),
                &base_url,
                provider.secret_ref.clone(),
                state.credential_store.clone(),
                oauth,
                DEFAULT_PROVIDER_TIMEOUT_MS,
            )
            .map(|adapter| Box::new(adapter) as Box<dyn ModelProviderAdapter>)
        }
        _ => OpenAiCompatibleAdapter::new(
            provider.name.clone(),
            &base_url,
            provider.secret_ref.clone(),
            state.credential_store.clone(),
            DEFAULT_PROVIDER_TIMEOUT_MS,
        )
        .map(|adapter| Box::new(adapter) as Box<dyn ModelProviderAdapter>),
    }
    .map_err(|e| (StatusCode::BAD_REQUEST, e.to_string()))
}

async fn resolve_chat_provider(
    state: &AppState,
    chat: &Chat,
    requested_model_key: &str,
    user_prompt: &str,
) -> Result<(Box<dyn ModelProviderAdapter>, String), (StatusCode, String)> {
    let repo = ProviderRepository::new(state.pool.clone());

    let mut resolved: Option<(Model, Provider)> = None;
    if requested_model_key != "local-mock" {
        resolved = repo
            .find_model_by_key(requested_model_key)
            .await
            .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;
        if resolved.is_none() {
            return Err((
                StatusCode::BAD_REQUEST,
                format!("Unknown model key '{}'", requested_model_key),
            ));
        }
    } else if let Some(model_id) = chat.model_id.as_deref()
        && let Some(model) = repo
            .get_model_by_id(model_id)
            .await
            .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?
    {
        resolved = repo
            .get_provider_by_id(&model.provider_id)
            .await
            .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?
            .map(|provider| (model, provider));
    }

    match resolved {
        Some((model, provider)) => {
            let adapter = build_provider_adapter(state, &provider)?;
            Ok((adapter, model.model_key))
        }
        None => {
            // Offline built-in fallback when no provider/model is configured.
            let provider = MockProviderAdapter::with_response(
                "local-mock",
                ModelCompletionResponse {
                    text: format!("Processed request: {user_prompt}"),
                    tool_calls: vec![],
                    usage: TokenUsage {
                        input_tokens: 12,
                        output_tokens: 8,
                        cached_tokens: 0,
                    },
                },
            );
            Ok((Box::new(provider), "local-mock".to_string()))
        }
    }
}

async fn list_memories(
    State(state): State<AppState>,
    Query(query): Query<MemoryQuery>,
) -> Result<Json<Vec<MemoryResponse>>, (StatusCode, String)> {
    let limit = query.limit.unwrap_or(20).clamp(1, 100);
    let repo = MemoryRepository::new(state.pool);
    let memories = match query
        .query
        .as_deref()
        .map(str::trim)
        .filter(|value| !value.is_empty())
    {
        Some(search) => repo
            .search_scoped(query.project_id.as_deref(), search, limit)
            .await
            .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?,
        None => {
            let project_id = query.project_id.as_deref().ok_or_else(|| {
                (
                    StatusCode::BAD_REQUEST,
                    "project_id or a non-empty query is required".to_string(),
                )
            })?;
            repo.list_by_project(project_id, limit)
                .await
                .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?
        }
    };

    let mut response = Vec::with_capacity(memories.len());
    for memory in memories {
        let tags = repo
            .list_tags(&memory.id)
            .await
            .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;
        response.push(MemoryResponse { memory, tags });
    }
    Ok(Json(response))
}

async fn create_memory(
    State(state): State<AppState>,
    Json(req): Json<CreateMemoryRequest>,
) -> Result<(StatusCode, Json<MemoryResponse>), (StatusCode, String)> {
    ProjectRepository::new(state.pool.clone())
        .get_by_id(&req.project_id)
        .await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?
        .ok_or_else(|| {
            (
                StatusCode::NOT_FOUND,
                format!("Project '{}' not found", req.project_id),
            )
        })?;

    let repo = MemoryRepository::new(state.pool);
    let tags = req.tags.unwrap_or_default();
    let memory = repo
        .create_with_options(
            Some(&req.project_id),
            None,
            None,
            &req.kind,
            &req.title,
            &req.content,
            &tags,
            req.pinned.unwrap_or(false),
        )
        .await
        .map_err(|e| (StatusCode::BAD_REQUEST, e.to_string()))?;
    let stored_tags = repo
        .list_tags(&memory.id)
        .await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;
    Ok((
        StatusCode::CREATED,
        Json(MemoryResponse {
            memory,
            tags: stored_tags,
        }),
    ))
}

async fn get_memory(
    State(state): State<AppState>,
    Path(id): Path<String>,
) -> Result<Json<MemoryResponse>, (StatusCode, String)> {
    let repo = MemoryRepository::new(state.pool);
    let memory = repo
        .get_by_id(&id)
        .await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?
        .ok_or_else(|| (StatusCode::NOT_FOUND, format!("Memory '{id}' not found")))?;
    let tags = repo
        .list_tags(&id)
        .await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;
    Ok(Json(MemoryResponse { memory, tags }))
}

async fn update_memory(
    State(state): State<AppState>,
    Path(id): Path<String>,
    Json(req): Json<UpdateMemoryRequest>,
) -> Result<Json<MemoryResponse>, (StatusCode, String)> {
    let repo = MemoryRepository::new(state.pool);
    let memory = repo
        .update(
            &id,
            req.kind.as_deref(),
            req.title.as_deref(),
            req.content.as_deref(),
            req.pinned,
        )
        .await
        .map_err(|e| (StatusCode::BAD_REQUEST, e.to_string()))?;
    if let Some(tags) = req.tags {
        repo.set_tags(&id, &tags)
            .await
            .map_err(|e| (StatusCode::BAD_REQUEST, e.to_string()))?;
    }
    let stored_tags = repo
        .list_tags(&id)
        .await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;
    Ok(Json(MemoryResponse {
        memory,
        tags: stored_tags,
    }))
}

async fn delete_memory(
    State(state): State<AppState>,
    Path(id): Path<String>,
) -> Result<StatusCode, (StatusCode, String)> {
    let repo = MemoryRepository::new(state.pool);
    let deleted = repo
        .soft_delete(&id)
        .await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;
    if !deleted {
        return Err((StatusCode::NOT_FOUND, format!("Memory '{id}' not found")));
    }
    Ok(StatusCode::NO_CONTENT)
}

async fn export_memories(
    State(state): State<AppState>,
    Query(query): Query<MemoryQuery>,
) -> Result<Json<serde_json::Value>, (StatusCode, String)> {
    let project_id = query.project_id.ok_or_else(|| {
        (
            StatusCode::BAD_REQUEST,
            "project_id is required".to_string(),
        )
    })?;
    let repo = MemoryRepository::new(state.pool);
    let markdown = repo
        .export_markdown(&project_id)
        .await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;
    Ok(Json(serde_json::json!({
        "project_id": project_id,
        "markdown": markdown
    })))
}

fn remove_expired_oauth_sessions(sessions: &mut HashMap<String, PendingOAuthSession>) {
    let cutoff = now_ms().saturating_sub(10 * 60 * 1000);
    sessions.retain(|_, session| session.created_at_ms >= cutoff);
}

async fn start_chatgpt_oauth(
    State(state): State<AppState>,
    Path(provider_id): Path<String>,
    Json(req): Json<StartChatGptOAuthRequest>,
) -> Result<Json<StartChatGptOAuthResponse>, (StatusCode, String)> {
    let repo = ProviderRepository::new(state.pool.clone());
    let mut provider = repo
        .get_provider_by_id(&provider_id)
        .await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?
        .ok_or_else(|| {
            (
                StatusCode::NOT_FOUND,
                format!("Provider '{provider_id}' not found"),
            )
        })?;
    if provider.kind != "chatgpt" {
        return Err((
            StatusCode::BAD_REQUEST,
            "ChatGPT OAuth is only available for providers of kind 'chatgpt'".to_string(),
        ));
    }

    if provider.secret_ref.is_none() {
        provider = repo
            .set_secret_ref(&provider.id, &format!("provider:{}", new_id()))
            .await
            .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;
    }
    let secret_ref = provider.secret_ref.clone().ok_or_else(|| {
        (
            StatusCode::INTERNAL_SERVER_ERROR,
            "Provider is missing a secret reference".to_string(),
        )
    })?;

    let env_config = chatgpt_oauth_config(req.redirect_uri.clone());
    let config = ChatGptOAuthConfig {
        client_id: req
            .client_id
            .or_else(|| env_config.as_ref().map(|config| config.client_id.clone()))
            .filter(|value| !value.trim().is_empty())
            .ok_or_else(|| {
                (
                    StatusCode::BAD_REQUEST,
                    "client_id is required (or set GAGGLE_CHATGPT_CLIENT_ID)".to_string(),
                )
            })?,
        auth_url: req
            .auth_url
            .or_else(|| env_config.as_ref().map(|config| config.auth_url.clone()))
            .unwrap_or_else(|| "https://auth.openai.com/oauth/authorize".to_string()),
        token_url: req
            .token_url
            .or_else(|| env_config.as_ref().map(|config| config.token_url.clone()))
            .unwrap_or_else(|| "https://auth.openai.com/oauth/token".to_string()),
        redirect_uri: req
            .redirect_uri
            .or_else(|| {
                env_config
                    .as_ref()
                    .map(|config| config.redirect_uri.clone())
            })
            .unwrap_or_else(|| "http://127.0.0.1:4321/api/oauth/chatgpt/callback".to_string()),
        scopes: req
            .scopes
            .or_else(|| env_config.as_ref().map(|config| config.scopes.clone()))
            .unwrap_or_else(|| {
                vec![
                    "openid".to_string(),
                    "profile".to_string(),
                    "email".to_string(),
                    "offline_access".to_string(),
                ]
            }),
    };

    let client = ChatGptOAuthClient::new(
        config.clone(),
        state.credential_store.clone(),
        DEFAULT_PROVIDER_TIMEOUT_MS,
    )
    .map_err(|e| (StatusCode::BAD_REQUEST, e.to_string()))?;
    let authorization = client
        .authorization_url()
        .map_err(|e| (StatusCode::BAD_REQUEST, e.to_string()))?;

    let mut sessions = state.oauth_sessions.write().map_err(|_| {
        (
            StatusCode::INTERNAL_SERVER_ERROR,
            "OAuth session lock poisoned".to_string(),
        )
    })?;
    remove_expired_oauth_sessions(&mut sessions);
    sessions.insert(
        authorization.state.clone(),
        PendingOAuthSession {
            provider_id: provider.id.clone(),
            secret_ref,
            code_verifier: authorization.code_verifier,
            config,
            created_at_ms: now_ms(),
        },
    );

    Ok(Json(StartChatGptOAuthResponse {
        authorization_url: authorization.url,
        state: authorization.state,
    }))
}

async fn exchange_chatgpt_oauth(
    State(state): State<AppState>,
    Path(provider_id): Path<String>,
    Json(req): Json<ExchangeChatGptOAuthRequest>,
) -> Result<Json<ChatGptOAuthStatusResponse>, (StatusCode, String)> {
    let session = {
        let mut sessions = state.oauth_sessions.write().map_err(|_| {
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                "OAuth session lock poisoned".to_string(),
            )
        })?;
        sessions.remove(&req.state)
    }
    .ok_or_else(|| {
        (
            StatusCode::BAD_REQUEST,
            "Unknown or expired OAuth state".to_string(),
        )
    })?;

    if session.provider_id != provider_id {
        return Err((
            StatusCode::BAD_REQUEST,
            "OAuth state does not match this provider".to_string(),
        ));
    }
    if session.created_at_ms < now_ms().saturating_sub(10 * 60 * 1000) {
        return Err((
            StatusCode::BAD_REQUEST,
            "OAuth session expired; start sign-in again".to_string(),
        ));
    }

    let client = ChatGptOAuthClient::new(
        session.config,
        state.credential_store.clone(),
        DEFAULT_PROVIDER_TIMEOUT_MS,
    )
    .map_err(|e| (StatusCode::BAD_REQUEST, e.to_string()))?;
    client
        .exchange_code(&session.secret_ref, &req.code, &session.code_verifier)
        .await
        .map_err(|e| (StatusCode::BAD_REQUEST, e.to_string()))?;

    Ok(Json(ChatGptOAuthStatusResponse {
        configured: true,
        provider_id,
    }))
}

async fn chatgpt_oauth_callback(
    State(state): State<AppState>,
    Query(query): Query<ChatGptOAuthCallbackQuery>,
) -> Result<Html<String>, (StatusCode, String)> {
    let session = {
        let mut sessions = state.oauth_sessions.write().map_err(|_| {
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                "OAuth session lock poisoned".to_string(),
            )
        })?;
        sessions.remove(&query.state)
    }
    .ok_or_else(|| {
        (
            StatusCode::BAD_REQUEST,
            "Unknown or expired OAuth state".to_string(),
        )
    })?;

    let client = ChatGptOAuthClient::new(
        session.config,
        state.credential_store.clone(),
        DEFAULT_PROVIDER_TIMEOUT_MS,
    )
    .map_err(|e| (StatusCode::BAD_REQUEST, e.to_string()))?;
    client
        .exchange_code(&session.secret_ref, &query.code, &session.code_verifier)
        .await
        .map_err(|e| (StatusCode::BAD_REQUEST, e.to_string()))?;

    Ok(Html(
        "<!doctype html><html><body style=\"font-family:system-ui;padding:2rem;\">\
         <h1>ChatGPT sign-in complete</h1>\
         <p>You can close this window and return to Gaggle.</p>\
         <script>window.setTimeout(function(){window.close();},1200);</script>\
         </body></html>"
            .to_string(),
    ))
}

async fn submit_chat_message(
    State(state): State<AppState>,
    Path(chat_id): Path<String>,
    Json(req): Json<SubmitChatMessageRequest>,
) -> Result<Json<AgentLoopOutcome>, (StatusCode, String)> {
    let runtime = AgentRuntime::new_with_active_runs(
        state.pool.clone(),
        DEFAULT_MAX_AGENT_STEPS,
        state.active_runs.clone(),
    );
    let executor = BuiltinToolExecutor::new(state.pool.clone());
    let requested_model_key = req.model_key.as_deref().unwrap_or("local-mock").trim();
    if requested_model_key.len() > 200 {
        return Err((
            StatusCode::BAD_REQUEST,
            "model_key must be at most 200 characters".to_string(),
        ));
    }
    let requested_model_key = if requested_model_key.is_empty() {
        "local-mock"
    } else {
        requested_model_key
    };

    let chat_repo = ChatRepository::new(state.pool.clone());
    let chat = chat_repo
        .get_by_id(&chat_id)
        .await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?
        .ok_or_else(|| {
            let failure = ActionableError::new(
                FailureCategory::Configuration,
                "chat.load",
                format!("Chat '{chat_id}' not found"),
            )
            .with_data_preservation(DataPreservation::Preserved)
            .with_retry_guidance("Select or create an existing chat, then retry the message.");
            (StatusCode::NOT_FOUND, failure.to_string())
        })?;

    let (provider, model_key) =
        resolve_chat_provider(&state, &chat, requested_model_key, &req.prompt)
            .await
            .map_err(|(status, message)| {
                let failure = ActionableError::new(
                    FailureCategory::Configuration,
                    "provider.resolve",
                    message,
                )
                .with_data_preservation(DataPreservation::Preserved)
                .with_retry_guidance(
                    "Open Settings → Providers & Models, verify the selected model, then retry.",
                );
                (status, failure.to_string())
            })?;

    tracing::info!(
        target: "gaggle.server",
        chat_id = %chat_id,
        model_key = %model_key,
        prompt_chars = req.prompt.chars().count(),
        "Chat turn submitted"
    );

    let cancel_token = gaggle_core::CancellationToken::new();
    let outcome = runtime
        .run_turn(
            &chat_id,
            &req.prompt,
            &model_key,
            provider.as_ref(),
            &executor,
            &cancel_token,
        )
        .await
        .map_err(|e| {
            let failure =
                ActionableError::new(FailureCategory::Internal, "agent.turn", e.to_string())
                    .with_data_preservation(DataPreservation::Unknown)
                    .with_correlation(&CorrelationIds::for_chat(&chat_id));
            tracing::error!(
                target: "gaggle.server",
                chat_id = %chat_id,
                failure_category = failure.category.as_str(),
                operation = %failure.operation,
                data_preservation = failure.data_preservation.as_str(),
                error = %failure.message,
                "Chat turn failed before producing a durable outcome"
            );
            (StatusCode::BAD_REQUEST, failure.to_string())
        })?;

    let correlation = match &outcome {
        AgentLoopOutcome::Completed { run, .. }
        | AgentLoopOutcome::WaitingApproval { run, .. }
        | AgentLoopOutcome::Cancelled { run }
        | AgentLoopOutcome::Failed { run, .. } => {
            CorrelationIds::for_chat(&chat_id).with_run(&run.id)
        }
    };
    tracing::info!(
        target: "gaggle.server",
        chat_id = %correlation.chat_id_field(),
        run_id = %correlation.run_id_field(),
        outcome = outcome_kind(&outcome),
        "Chat turn finished"
    );

    Ok(Json(outcome))
}

fn outcome_kind(outcome: &AgentLoopOutcome) -> &'static str {
    match outcome {
        AgentLoopOutcome::Completed { .. } => "completed",
        AgentLoopOutcome::WaitingApproval { .. } => "waiting_approval",
        AgentLoopOutcome::Cancelled { .. } => "cancelled",
        AgentLoopOutcome::Failed { .. } => "failed",
    }
}

async fn compact_chat(
    State(state): State<AppState>,
    Path(chat_id): Path<String>,
    req: Result<Option<Json<CompactChatRequest>>, axum::extract::rejection::JsonRejection>,
) -> Result<Json<ChatSummary>, (StatusCode, String)> {
    let chat_repo = ChatRepository::new(state.pool.clone());
    let chat = chat_repo
        .get_by_id(&chat_id)
        .await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?
        .ok_or_else(|| (StatusCode::NOT_FOUND, format!("Chat '{chat_id}' not found")))?;

    let trigger = match req {
        Ok(Some(Json(r))) => r.trigger.clone().unwrap_or_else(|| "manual".to_string()),
        _ => "manual".to_string(),
    };
    if !matches!(trigger.as_str(), "auto" | "manual") {
        return Err((
            StatusCode::BAD_REQUEST,
            "trigger must be 'auto' or 'manual'".to_string(),
        ));
    }

    let requested_model_key = "local-mock";
    let (provider, _model_key) =
        resolve_chat_provider(&state, &chat, requested_model_key, "compact").await?;

    // Resolve context window.
    let context_window: i64 = if let Some(model_id) = chat.model_id.as_deref() {
        let repo = ProviderRepository::new(state.pool.clone());
        if let Some(model) = repo
            .get_model_by_id(model_id)
            .await
            .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?
        {
            if model.context_window > 0 {
                model.context_window
            } else {
                8192
            }
        } else {
            8192
        }
    } else {
        8192
    };

    let result = CompactionEngine::compact_chat(
        state.pool.clone(),
        &chat_id,
        context_window,
        provider.as_ref(),
        &trigger,
        true,
    )
    .await
    .map_err(|e| (StatusCode::BAD_REQUEST, e.to_string()))?;

    match result {
        Some(_cr) => {
            // Fetch the stored summary row.
            let summary = chat_repo
                .get_latest_summary(&chat_id)
                .await
                .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?
                .ok_or_else(|| {
                    (
                        StatusCode::INTERNAL_SERVER_ERROR,
                        "Compaction did not store a summary".to_string(),
                    )
                })?;
            Ok(Json(summary))
        }
        None => Err((StatusCode::BAD_REQUEST, "Nothing to compact".to_string())),
    }
}

async fn cancel_run(
    State(state): State<AppState>,
    Path(run_id): Path<String>,
) -> Result<Json<Run>, (StatusCode, String)> {
    tracing::info!(
        target: "gaggle.server",
        run_id = %run_id,
        "Cancellation requested for durable run"
    );

    let repo = RunRepository::new(state.pool.clone());
    // Signal the active run (if any) so the agent loop stops promptly.
    let was_active = state.active_runs.cancel(&run_id);

    if was_active {
        // Let the agent loop own the terminal event/status update to avoid duplicates.
        for _ in 0..40 {
            tokio::time::sleep(std::time::Duration::from_millis(50)).await;
            if let Ok(Some(run)) = repo.get_run_by_id(&run_id).await
                && matches!(run.status.as_str(), "cancelled" | "completed" | "failed")
            {
                return Ok(Json(run));
            }
        }
        if let Ok(Some(run)) = repo.get_run_by_id(&run_id).await {
            return Ok(Json(run));
        }
    }

    // Fallback for runs without an active token: record cancellation directly.
    let _ = repo
        .emit_event(
            &run_id,
            "run.cancelled",
            &serde_json::json!({ "reason": "user_cancelled_via_api" }).to_string(),
        )
        .await;
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

    // Validate the run exists.
    let repo = RunRepository::new(state.pool.clone());
    if repo
        .get_run_by_id(&run_id)
        .await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?
        .is_none()
    {
        return Err((StatusCode::NOT_FOUND, format!("Run '{run_id}' not found")));
    }

    let (tx, rx) = tokio::sync::mpsc::channel::<Result<Event, Infallible>>(64);
    let pool = state.pool.clone();
    let run_id_task = run_id.clone();
    tokio::spawn(async move {
        let mut after_seq = after_seq;
        loop {
            let repo = RunRepository::new(pool.clone());
            if let Ok(events) = repo.list_run_events(&run_id_task, after_seq).await {
                let mut terminal = false;
                for ev in events {
                    after_seq = ev.seq;
                    let kind = ev.kind.clone();
                    if matches!(
                        kind.as_str(),
                        "run.completed" | "run.cancelled" | "run.failed"
                    ) {
                        terminal = true;
                    }
                    let event = Event::default()
                        .id(ev.seq.to_string())
                        .event(ev.kind)
                        .data(ev.payload);
                    if tx.send(Ok(event)).await.is_err() {
                        return;
                    }
                }
                if terminal {
                    return;
                }
            }
            tokio::time::sleep(std::time::Duration::from_millis(250)).await;
        }
    });

    let stream = tokio_stream::wrappers::ReceiverStream::new(rx);
    Ok(Sse::new(stream).keep_alive(axum::response::sse::KeepAlive::default()))
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

// ============================================================================
// Recipe scheduler helpers
// ============================================================================

pub async fn missed_run_catchup(state: &AppState) -> Result<(), anyhow::Error> {
    let repo = RecipeRepository::new(state.pool.clone());
    let schedules = repo.list_schedules(None).await?;
    let now = now_ms();
    for sched in schedules {
        if sched.enabled != 1 {
            continue;
        }
        if let Some(next) = sched.next_run_at {
            if next <= now {
                // compute next future occurrence
                if let Ok(future) = compute_next_run(&sched.cron_expr, &sched.timezone, now) {
                    repo.update_schedule_next_run(&sched.id, Some(future))
                        .await?;
                }
            }
        } else {
            // no next_run_at; compute
            if let Ok(future) = compute_next_run(&sched.cron_expr, &sched.timezone, now) {
                repo.update_schedule_next_run(&sched.id, Some(future))
                    .await?;
            }
        }
    }
    Ok(())
}

pub async fn execute_scheduled_recipe(
    state: &AppState,
    schedule: &Schedule,
) -> Result<Chat, anyhow::Error> {
    let recipe_repo = RecipeRepository::new(state.pool.clone());
    let recipe = recipe_repo
        .get_by_id(&schedule.recipe_id)
        .await?
        .ok_or_else(|| anyhow::anyhow!("Recipe not found"))?;

    let specs = recipe_repo.list_parameters(&recipe.id).await?;
    validate_recipe_parameters(&specs).map_err(|e| anyhow::anyhow!(e.to_string()))?;

    let provided: HashMap<String, String> = schedule
        .param_values
        .as_ref()
        .and_then(|s| serde_json::from_str::<serde_json::Value>(s).ok())
        .and_then(|v| v.as_object().cloned())
        .map(|obj| {
            obj.into_iter()
                .map(|(k, v)| {
                    let s = match v {
                        serde_json::Value::String(s) => s,
                        other => other.to_string(),
                    };
                    (k, s)
                })
                .collect()
        })
        .unwrap_or_default();

    let resolved =
        resolve_recipe_parameters(&specs, &provided).map_err(|e| anyhow::anyhow!(e.to_string()))?;

    let chat_type = "scheduled".to_string();
    let mode = "local".to_string();
    let permission_mode = "ask".to_string();
    let title = recipe.title.clone();
    let recipe_values_json = serde_json::to_string(&resolved).unwrap_or_else(|_| "{}".to_string());

    let project_id = schedule
        .project_id
        .clone()
        .ok_or_else(|| anyhow::anyhow!("Schedule has no project_id"))?;

    let chat_repo = ChatRepository::new(state.pool.clone());
    let chat = chat_repo
        .create_recipe_chat(
            &project_id,
            &title,
            &chat_type,
            &mode,
            &permission_mode,
            Some(&recipe.id),
            Some(&recipe_values_json),
            Some(&schedule.id),
            None,
            None,
        )
        .await?;

    let ext_repo = ExtensionRepository::new(state.pool.clone());
    let ext_ids = recipe_repo.list_extension_ids(&recipe.id).await?;
    for ext_id in ext_ids {
        ext_repo
            .enable_extension_for_chat(&chat.id, &ext_id)
            .await?;
    }

    Ok(chat)
}

pub async fn process_due_schedules(state: &AppState) -> Result<(), anyhow::Error> {
    let repo = RecipeRepository::new(state.pool.clone());
    let now = now_ms();
    let due = repo.list_due_schedules(now).await?;
    for sched in due {
        let result = execute_scheduled_recipe(state, &sched).await;
        let next = compute_next_run(&sched.cron_expr, &sched.timezone, now).ok();
        match result {
            Ok(_) => {
                repo.update_schedule_status(&sched.id, Some(now), Some("ok"), next)
                    .await?;
            }
            Err(_) => {
                repo.update_schedule_status(&sched.id, Some(now), Some("error"), next)
                    .await?;
            }
        }
    }
    Ok(())
}

// Aliases
pub async fn run_scheduler_tick(state: &AppState) -> Result<(), anyhow::Error> {
    process_due_schedules(state).await
}

pub async fn check_due_schedules(state: &AppState) -> Result<(), anyhow::Error> {
    process_due_schedules(state).await
}

pub fn spawn_recipe_scheduler(state: AppState) -> tokio::task::JoinHandle<()> {
    tokio::spawn(async move {
        // catch up missed runs on startup
        let _ = missed_run_catchup(&state).await;
        let mut interval = tokio::time::interval(std::time::Duration::from_secs(60));
        loop {
            interval.tick().await;
            let _ = process_due_schedules(&state).await;
        }
    })
}

async fn recipe_graph_has_cycle(
    repo: &RecipeRepository,
    root: &str,
) -> Result<bool, (StatusCode, String)> {
    // Load the reachable sub-recipe graph iteratively.
    let mut adjacency: HashMap<String, Vec<String>> = HashMap::new();
    let mut to_visit = vec![root.to_string()];
    let mut visited = HashSet::new();
    while let Some(node) = to_visit.pop() {
        if visited.contains(&node) {
            continue;
        }
        visited.insert(node.clone());
        let subs = repo
            .list_sub_recipes(&node)
            .await
            .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;
        let children: Vec<String> = subs.into_iter().map(|s| s.child_recipe_id).collect();
        for c in &children {
            if !visited.contains(c) {
                to_visit.push(c.clone());
            }
        }
        adjacency.insert(node, children);
    }

    // Synchronous cycle detection.
    let mut state_visited = HashSet::new();
    let mut stack = HashSet::new();
    fn dfs(
        adjacency: &HashMap<String, Vec<String>>,
        node: &str,
        visited: &mut HashSet<String>,
        stack: &mut HashSet<String>,
    ) -> bool {
        if stack.contains(node) {
            return true;
        }
        if visited.contains(node) {
            return false;
        }
        stack.insert(node.to_string());
        if let Some(children) = adjacency.get(node) {
            for child in children {
                if dfs(adjacency, child, visited, stack) {
                    return true;
                }
            }
        }
        stack.remove(node);
        visited.insert(node.to_string());
        false
    }

    for node in adjacency.keys() {
        if dfs(&adjacency, node, &mut state_visited, &mut stack) {
            return Ok(true);
        }
    }
    Ok(false)
}
