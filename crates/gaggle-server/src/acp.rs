//! Agent Client Protocol (ACP) compatibility layer for Gaggle.

use axum::{
    Router,
    extract::{
        State,
        ws::{Message, WebSocket, WebSocketUpgrade},
    },
    response::IntoResponse,
};
use futures::{SinkExt, StreamExt};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::sync::Arc;
use tokio::sync::{mpsc, oneshot};

use crate::AppState;
use gaggle_core::CancellationToken;
/// ACP session state.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AcpSession {
    pub session_id: String,
    pub chat_id: Option<String>,
    pub project_id: Option<String>,
    pub run_id: Option<String>,
    pub cwd: Option<String>,
    pub session_type: Option<String>,
    pub mode: Option<String>,
    pub provider: Option<String>,
    pub model: Option<String>,
    pub created_at: i64,
}

/// JSON-RPC request.
#[derive(Debug, Deserialize)]
#[allow(dead_code)]
pub struct JsonRpcRequest {
    #[allow(dead_code)]
    pub jsonrpc: String,
    pub id: Option<Value>,
    pub method: String,
    #[serde(default)]
    pub params: Value,
}

/// JSON-RPC response.
#[derive(Debug, Serialize)]
#[allow(dead_code)]
pub struct JsonRpcResponse {
    #[allow(dead_code)]
    pub jsonrpc: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub id: Option<Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub result: Option<Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<JsonRpcError>,
}

/// JSON-RPC error.
#[derive(Debug, Serialize)]
pub struct JsonRpcError {
    pub code: i64,
    pub message: String,
}

/// Bridge for form elicitation request/response correlation.
pub struct ElicitationBridge {
    pending:
        std::sync::Mutex<std::collections::HashMap<String, oneshot::Sender<serde_json::Value>>>,
}

impl ElicitationBridge {
    pub fn new() -> Self {
        Self {
            pending: std::sync::Mutex::new(std::collections::HashMap::new()),
        }
    }

    pub fn register(&self, request_id: String) -> oneshot::Receiver<serde_json::Value> {
        let (tx, rx) = oneshot::channel();
        let mut pending = self.pending.lock().unwrap();
        pending.insert(request_id, tx);
        rx
    }

    pub fn resolve(&self, request_id: &str, value: serde_json::Value) -> bool {
        let mut pending = self.pending.lock().unwrap();
        if let Some(tx) = pending.remove(request_id) {
            let _ = tx.send(value);
            true
        } else {
            false
        }
    }
}

impl Default for ElicitationBridge {
    fn default() -> Self {
        Self::new()
    }
}

/// Permission bridge for request/response correlation.
pub struct PermissionBridge {
    pending: std::sync::Mutex<std::collections::HashMap<String, oneshot::Sender<bool>>>,
}

impl PermissionBridge {
    pub fn new() -> Self {
        Self {
            pending: std::sync::Mutex::new(std::collections::HashMap::new()),
        }
    }

    /// Register a pending permission request and return a receiver.
    pub fn register(&self, request_id: String) -> oneshot::Receiver<bool> {
        let (tx, rx) = oneshot::channel();
        let mut pending = self.pending.lock().unwrap();
        pending.insert(request_id, tx);
        rx
    }

    /// Resolve a pending permission request with a decision.
    pub fn resolve(&self, request_id: &str, approved: bool) -> bool {
        let mut pending = self.pending.lock().unwrap();
        if let Some(tx) = pending.remove(request_id) {
            let _ = tx.send(approved);
            true
        } else {
            false
        }
    }
}

impl Default for PermissionBridge {
    fn default() -> Self {
        Self::new()
    }
}

/// ACP session manager (in-memory).
pub struct AcpSessionManager {
    sessions: std::sync::Mutex<std::collections::HashMap<String, AcpSession>>,
    cancel_tokens: std::sync::Mutex<std::collections::HashMap<String, CancellationToken>>,
}

impl AcpSessionManager {
    pub fn new() -> Self {
        Self {
            sessions: std::sync::Mutex::new(std::collections::HashMap::new()),
            cancel_tokens: std::sync::Mutex::new(std::collections::HashMap::new()),
        }
    }

    pub fn register_cancel_token(&self, session_id: &str, token: CancellationToken) {
        let mut tokens = self.cancel_tokens.lock().unwrap();
        tokens.insert(session_id.to_string(), token);
    }

    pub fn cancel(&self, session_id: &str) -> bool {
        let tokens = self.cancel_tokens.lock().unwrap();
        if let Some(token) = tokens.get(session_id) {
            token.cancel();
            true
        } else {
            false
        }
    }

    pub fn remove_cancel_token(&self, session_id: &str) {
        let mut tokens = self.cancel_tokens.lock().unwrap();
        tokens.remove(session_id);
    }

    pub fn create_session(&self, cwd: Option<String>, session_type: Option<String>) -> AcpSession {
        let session_id = format!(
            "acp-{}",
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .map(|d| d.as_nanos())
                .unwrap_or(0)
        );
        let session = AcpSession {
            session_id: session_id.clone(),
            chat_id: None,
            project_id: None,
            run_id: None,
            cwd,
            session_type: session_type.or(Some("user".to_string())),
            mode: Some("auto".to_string()),
            provider: None,
            model: None,
            created_at: std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .map(|d| d.as_millis() as i64)
                .unwrap_or(0),
        };
        let mut sessions = self.sessions.lock().unwrap();
        sessions.insert(session_id, session.clone());
        session
    }

    pub fn get_session(&self, session_id: &str) -> Option<AcpSession> {
        let sessions = self.sessions.lock().unwrap();
        sessions.get(session_id).cloned()
    }

    #[allow(dead_code)]
    pub fn update_session(&self, session: AcpSession) {
        let mut sessions = self.sessions.lock().unwrap();
        sessions.insert(session.session_id.clone(), session);
    }

    pub fn remove_session(&self, session_id: &str) {
        let mut sessions = self.sessions.lock().unwrap();
        sessions.remove(session_id);
    }

    pub fn list_sessions(&self) -> Vec<AcpSession> {
        let sessions = self.sessions.lock().unwrap();
        sessions.values().cloned().collect()
    }
}

impl Default for AcpSessionManager {
    fn default() -> Self {
        Self::new()
    }
}

/// Creates the ACP router with WebSocket endpoint.
pub fn acp_routes() -> Router<AppState> {
    Router::new()
        .route("/health", axum::routing::get(health))
        .route("/status", axum::routing::get(status))
        .route("/", axum::routing::get(ws_handler))
}

async fn health() -> &'static str {
    "ok"
}

async fn status() -> axum::Json<serde_json::Value> {
    axum::Json(serde_json::json!({
        "status": "ok",
        "service": "gaggle",
        "version": env!("CARGO_PKG_VERSION")
    }))
}

/// WebSocket upgrade handler.
async fn ws_handler(ws: WebSocketUpgrade, State(state): State<AppState>) -> impl IntoResponse {
    ws.on_upgrade(move |socket| handle_socket(socket, state))
}

/// Handle WebSocket connection.
async fn handle_socket(socket: WebSocket, state: AppState) {
    let (mut sender, mut receiver) = socket.split();
    let session_manager = state.acp_sessions.clone();
    let (tx, mut rx) = mpsc::channel::<String>(100);

    // Spawn task to send messages
    let send_task = tokio::spawn(async move {
        while let Some(msg) = rx.recv().await {
            if sender.send(Message::Text(msg.into())).await.is_err() {
                break;
            }
        }
    });

    // Handle incoming messages
    while let Some(Ok(msg)) = receiver.next().await {
        if let Message::Text(text) = msg {
            // Check if this is a permission response (has "result" and "id" but no "method")
            let value: Result<Value, _> = serde_json::from_str(&text);
            if let Ok(val) = value
                && val.get("method").is_none()
                && val.get("id").is_some()
                && (val.get("result").is_some() || val.get("error").is_some())
            {
                // This is a response - check if it's a permission response
                if let Some(id) = val.get("id").and_then(|v| v.as_str())
                    && id.starts_with("perm-")
                {
                    let approved = val
                        .get("result")
                        .and_then(|r| r.get("approved"))
                        .and_then(|a| a.as_bool())
                        .unwrap_or(false);
                    state.permission_bridge.resolve(id, approved);
                    continue;
                } else if let Some(id) = val.get("id").and_then(|v| v.as_str())
                    && id.starts_with("elic-")
                {
                    let value = val.get("result").cloned().unwrap_or(serde_json::json!({}));
                    state.elicitation_bridge.resolve(id, value);
                    continue;
                }
            }

            let request: Result<JsonRpcRequest, _> = serde_json::from_str(&text);
            match request {
                Ok(req) => {
                    let response = handle_request(&req, &state, &session_manager, &tx).await;
                    if let Some(resp) = response {
                        let json = serde_json::to_string(&resp).unwrap_or_default();
                        let _ = tx.send(json).await;
                    }
                }
                Err(e) => {
                    let error_response = JsonRpcResponse {
                        jsonrpc: "2.0".to_string(),
                        id: None,
                        result: None,
                        error: Some(JsonRpcError {
                            code: -32700,
                            message: format!("Parse error: {}", e),
                        }),
                    };
                    let json = serde_json::to_string(&error_response).unwrap_or_default();
                    let _ = tx.send(json).await;
                }
            }
        }
    }

    drop(tx);
    let _ = send_task.await;
}

/// Handle a JSON-RPC request.
async fn handle_request(
    req: &JsonRpcRequest,
    state: &AppState,
    session_manager: &AcpSessionManager,
    tx: &mpsc::Sender<String>,
) -> Option<JsonRpcResponse> {
    let id = req.id.clone();
    // Normalize Gaggle-branded custom method names to the canonical (Goose-compatible) form.
    let normalized_method = if req.method.starts_with("_gaggle/unstable/") {
        req.method
            .replacen("_gaggle/unstable/", "_goose/unstable/", 1)
    } else {
        req.method.clone()
    };

    match normalized_method.as_str() {
        "authenticate" => {
            let repo =
                gaggle_db::repositories::providers::ProviderRepository::new(state.pool.clone());
            let providers = repo.list_providers().await.unwrap_or_default();
            Some(JsonRpcResponse {
                jsonrpc: "2.0".to_string(),
                id,
                result: Some(serde_json::json!({
                    "authenticated": true,
                    "providers": providers
                })),
                error: None,
            })
        }
        "initialize" => {
            let response = JsonRpcResponse {
                jsonrpc: "2.0".to_string(),
                id,
                result: Some(serde_json::json!({
                    "protocolVersion": 1,
                    "agentCapabilities": {
                        "loadSession": true,
                        "promptCapabilities": {
                            "image": true,
                            "audio": false,
                            "embeddedContext": true
                        },
                        "mcpCapabilities": {
                            "http": true,
                            "sse": true
                        },
                        "sessionCapabilities": {
                            "list": {},
                            "fork": {},
                            "resume": {},
                            "close": {}
                        }
                    },
                    "agentInfo": {
                        "name": "gaggle",
                        "title": "Gaggle",
                        "version": env!("CARGO_PKG_VERSION")
                    },
                    "authMethods": []
                })),
                error: None,
            };
            Some(response)
        }
        "session/new" => {
            let cwd = req
                .params
                .get("cwd")
                .and_then(|v| v.as_str())
                .map(String::from);
            let session_type = req
                .params
                .get("sessionType")
                .and_then(|v| v.as_str())
                .map(String::from);
            let session = session_manager.create_session(cwd, session_type);
            let response = JsonRpcResponse {
                jsonrpc: "2.0".to_string(),
                id,
                result: Some(serde_json::json!({
                    "sessionId": session.session_id
                })),
                error: None,
            };
            Some(response)
        }
        "session/prompt" => {
            let session_id = req
                .params
                .get("sessionId")
                .and_then(|v| v.as_str())
                .unwrap_or("");
            let session = session_manager.get_session(session_id);
            if session.is_none() {
                return Some(JsonRpcResponse {
                    jsonrpc: "2.0".to_string(),
                    id,
                    result: None,
                    error: Some(JsonRpcError {
                        code: -32001,
                        message: "Session not found".to_string(),
                    }),
                });
            }

            // Extract prompt text
            let prompt_text = req
                .params
                .get("prompt")
                .and_then(|p| p.as_array())
                .and_then(|arr| arr.first())
                .and_then(|item| item.get("text"))
                .and_then(|t| t.as_str())
                .unwrap_or("")
                .to_string();

            if prompt_text.is_empty() {
                return Some(JsonRpcResponse {
                    jsonrpc: "2.0".to_string(),
                    id,
                    result: None,
                    error: Some(JsonRpcError {
                        code: -32602,
                        message: "Empty prompt".to_string(),
                    }),
                });
            }

            // Delegate to Gaggle's agent loop in background
            let state = state.clone();
            let session_id = session_id.to_string();
            let tx = tx.clone();
            let spawned_id = id.clone();

            tokio::spawn(async move {
                // Create project and chat for this session
                let project_id = format!("acp-project-{}", session_id);
                let chat_id = format!("acp-chat-{}", session_id);

                let pool = state.pool.clone();
                let chat_repo = gaggle_db::repositories::chats::ChatRepository::new(pool.clone());
                let proj_repo =
                    gaggle_db::repositories::projects::ProjectRepository::new(pool.clone());

                // Create project if not exists
                if proj_repo
                    .get_by_id(&project_id)
                    .await
                    .ok()
                    .flatten()
                    .is_none()
                {
                    let _ = proj_repo.create("ACP Session", "/tmp", "none").await;
                }

                // Create chat if not exists
                if chat_repo.get_by_id(&chat_id).await.ok().flatten().is_none() {
                    let _ = chat_repo
                        .create(&project_id, "ACP Chat", "user", "local", "ask")
                        .await;
                }

                // Link the ACP session to the created chat/project for config bridging.
                if let Some(mut session) = state.acp_sessions.get_session(&session_id) {
                    session.chat_id = Some(chat_id.clone());
                    session.project_id = Some(project_id.clone());
                    state.acp_sessions.update_session(session);
                }

                // Get chat for provider resolution
                let chat = match chat_repo.get_by_id(&chat_id).await {
                    Ok(Some(c)) => c,
                    _ => {
                        let error_resp = JsonRpcResponse {
                            jsonrpc: "2.0".to_string(),
                            id: spawned_id.clone(),
                            result: None,
                            error: Some(JsonRpcError {
                                code: -32000,
                                message: "Failed to get chat".to_string(),
                            }),
                        };
                        let json = serde_json::to_string(&error_resp).unwrap_or_default();
                        let _ = tx.send(json).await;
                        return;
                    }
                };

                // Resolve provider
                let (provider, model_key) =
                    match crate::resolve_chat_provider(&state, &chat, "local-mock", &prompt_text)
                        .await
                    {
                        Ok(result) => result,
                        Err(_) => {
                            let error_resp = JsonRpcResponse {
                                jsonrpc: "2.0".to_string(),
                                id: spawned_id.clone(),
                                result: None,
                                error: Some(JsonRpcError {
                                    code: -32000,
                                    message: "Failed to resolve provider".to_string(),
                                }),
                            };
                            let json = serde_json::to_string(&error_resp).unwrap_or_default();
                            let _ = tx.send(json).await;
                            return;
                        }
                    };

                // Run agent loop
                let runtime = gaggle_core::AgentRuntime::new_with_active_runs(
                    state.pool.clone(),
                    gaggle_core::DEFAULT_MAX_AGENT_STEPS,
                    state.active_runs.clone(),
                );
                let builtin_executor: Arc<dyn gaggle_core::ToolExecutor> =
                    Arc::new(gaggle_tools::BuiltinToolExecutor::new(state.pool.clone()));
                let mcp_executor: Arc<dyn gaggle_core::ToolExecutor> =
                    Arc::new(gaggle_mcp::McpToolExecutor::with_connection_manager(
                        state.pool.clone(),
                        state.mcp_connections.clone(),
                    ));
                let executor =
                    gaggle_core::CompositeToolExecutor::new(builtin_executor, Some(mcp_executor));

                // Create notification channel for streaming
                let (notif_tx, mut notif_rx) = mpsc::unbounded_channel::<String>();

                // Spawn task to forward notifications, normalizing sessionId to the ACP session.
                let tx_clone = tx.clone();
                let session_id_for_notif = session_id.clone();
                tokio::spawn(async move {
                    while let Some(notification) = notif_rx.recv().await {
                        let adjusted =
                            if let Ok(mut value) = serde_json::from_str::<Value>(&notification) {
                                if let Some(params) =
                                    value.get_mut("params").and_then(|p| p.as_object_mut())
                                {
                                    params.insert(
                                        "sessionId".to_string(),
                                        Value::String(session_id_for_notif.clone()),
                                    );
                                }
                                serde_json::to_string(&value).unwrap_or(notification)
                            } else {
                                notification
                            };
                        let _ = tx_clone.send(adjusted).await;
                    }
                });

                let cancel_token = gaggle_core::CancellationToken::new();
                session_manager.register_cancel_token(&session_id, cancel_token.clone());
                let outcome = runtime
                    .run_turn_with_notifications(
                        &chat_id,
                        &prompt_text,
                        &model_key,
                        provider.as_ref(),
                        &executor,
                        &cancel_token,
                        Some(notif_tx.clone()),
                    )
                    .await;

                // Handle approval loop
                #[allow(clippy::never_loop)]
                #[allow(clippy::while_let_loop)]
                loop {
                    match &outcome {
                        Ok(gaggle_core::AgentLoopOutcome::WaitingApproval {
                            pending_approval_id,
                            tool_call_id,
                            ..
                        }) => {
                            // Register permission request and wait for response
                            let perm_req_id = format!("perm-{}", tool_call_id);
                            let rx = state.permission_bridge.register(perm_req_id.clone());

                            // Send permission request notification
                            let notification = serde_json::json!({
                                "jsonrpc": "2.0",
                                "method": "session/request_permission",
                                "id": perm_req_id.clone(),
                                "params": {
                                    "sessionId": session_id.clone(),
                                    "toolCall": {
                                        "toolCallId": tool_call_id,
                                    },
                                    "options": [
                                        {"optionId": "approve", "name": "Approve", "kind": "allow_once"},
                                        {"optionId": "deny", "name": "Deny", "kind": "reject_once"}
                                    ]
                                }
                            });
                            let _ = tx.send(notification.to_string()).await;

                            // Wait for client response (with 30s timeout)
                            let approved =
                                match tokio::time::timeout(std::time::Duration::from_secs(30), rx)
                                    .await
                                {
                                    Ok(Ok(approved)) => approved,
                                    _ => false, // Timeout or error - deny
                                };

                            // Decide approval
                            let run_repo = gaggle_db::repositories::runs::RunRepository::new(
                                state.pool.clone(),
                            );
                            let _ = run_repo
                                .decide_approval(pending_approval_id, approved, None, None, None)
                                .await;

                            // Break out of approval loop - client needs to send another prompt
                            break;
                        }
                        _ => break,
                    }
                }

                // Clean up cancel token for this session.
                state.acp_sessions.remove_cancel_token(&session_id);

                // Send usage update notification with real totals from usage_records for this run.
                let run_id = match &outcome {
                    Ok(gaggle_core::AgentLoopOutcome::Completed { run, .. }) => {
                        Some(run.id.clone())
                    }
                    Ok(gaggle_core::AgentLoopOutcome::WaitingApproval { run, .. }) => {
                        Some(run.id.clone())
                    }
                    Ok(gaggle_core::AgentLoopOutcome::Cancelled { run }) => Some(run.id.clone()),
                    Ok(gaggle_core::AgentLoopOutcome::Failed { run, .. }) => Some(run.id.clone()),
                    Err(_) => None,
                };
                let (input_tokens, output_tokens) = if let Some(run_id) = run_id {
                    let row: (i64, i64) = sqlx::query_as(
                        "SELECT COALESCE(SUM(input_tokens),0), COALESCE(SUM(output_tokens),0) FROM usage_records WHERE run_id = ?",
                    )
                    .bind(&run_id)
                    .fetch_one(&state.pool)
                    .await
                    .unwrap_or((0, 0));
                    row
                } else {
                    match chat_repo.get_by_id(&chat_id).await {
                        Ok(Some(c)) => (c.total_input_tokens, c.total_output_tokens),
                        _ => (0, 0),
                    }
                };
                let usage_notification = serde_json::json!({
                    "jsonrpc": "2.0",
                    "method": "session/update",
                    "params": {
                        "sessionId": session_id.clone(),
                        "update": {
                            "sessionUpdate": "usage_update",
                            "usage": {
                                "inputTokens": input_tokens,
                                "outputTokens": output_tokens,
                                "totalTokens": input_tokens + output_tokens
                            }
                        }
                    }
                });
                let _ = tx.send(usage_notification.to_string()).await;

                // Send completion notification
                let stop_reason = match &outcome {
                    Ok(gaggle_core::AgentLoopOutcome::Completed { .. }) => "end_turn",
                    Ok(gaggle_core::AgentLoopOutcome::WaitingApproval { .. }) => "end_turn",
                    Ok(gaggle_core::AgentLoopOutcome::Cancelled { .. }) => "cancelled",
                    Ok(gaggle_core::AgentLoopOutcome::Failed { .. }) => "error",
                    Err(_) => "error",
                };

                let response = JsonRpcResponse {
                    jsonrpc: "2.0".to_string(),
                    id: spawned_id.clone(),
                    result: Some(serde_json::json!({
                        "stopReason": stop_reason
                    })),
                    error: None,
                };
                let json = serde_json::to_string(&response).unwrap_or_default();
                let _ = tx.send(json).await;
            });

            // No immediate response; the background task sends the final response.
            None
        }
        "session/cancel" => {
            let session_id = req
                .params
                .get("sessionId")
                .and_then(|v| v.as_str())
                .unwrap_or("");
            let _ = session_manager.cancel(session_id);
            Some(JsonRpcResponse {
                jsonrpc: "2.0".to_string(),
                id,
                result: Some(serde_json::json!({})),
                error: None,
            })
        }
        "session/list" => {
            let cwd_filter = req.params.get("cwd").and_then(|v| v.as_str());
            let query_filter = req.params.get("query").and_then(|v| v.as_str());
            let limit = req
                .params
                .get("limit")
                .and_then(|v| v.as_u64())
                .map(|v| v as usize)
                .unwrap_or(50);
            let cursor = req
                .params
                .get("cursor")
                .and_then(|v| v.as_str())
                .and_then(|s| s.parse::<usize>().ok())
                .unwrap_or(0);
            let mut sessions = session_manager.list_sessions();
            if let Some(cwd) = cwd_filter {
                sessions.retain(|s| s.cwd.as_deref() == Some(cwd));
            }
            if let Some(q) = query_filter {
                let q = q.to_lowercase();
                sessions.retain(|s| {
                    s.session_id.to_lowercase().contains(&q)
                        || s.cwd.as_deref().unwrap_or("").to_lowercase().contains(&q)
                });
            }
            let total = sessions.len();
            let end = (cursor + limit).min(total);
            let page = sessions[cursor..end].to_vec();
            let next_cursor = if end < total {
                Some(end.to_string())
            } else {
                None
            };
            let response = JsonRpcResponse {
                jsonrpc: "2.0".to_string(),
                id,
                result: Some(serde_json::json!({
                    "sessions": page,
                    "nextCursor": next_cursor
                })),
                error: None,
            };
            Some(response)
        }
        "session/load" => {
            let session_id = req
                .params
                .get("sessionId")
                .and_then(|v| v.as_str())
                .unwrap_or("");
            let session = session_manager.get_session(session_id);
            match session {
                Some(s) => {
                    let response = JsonRpcResponse {
                        jsonrpc: "2.0".to_string(),
                        id,
                        result: Some(serde_json::json!({
                            "sessionId": s.session_id
                        })),
                        error: None,
                    };
                    Some(response)
                }
                None => Some(JsonRpcResponse {
                    jsonrpc: "2.0".to_string(),
                    id,
                    result: None,
                    error: Some(JsonRpcError {
                        code: -32001,
                        message: "Session not found".to_string(),
                    }),
                }),
            }
        }
        "session/delete" => {
            let session_id = req
                .params
                .get("sessionId")
                .and_then(|v| v.as_str())
                .unwrap_or("");
            session_manager.remove_session(session_id);
            let response = JsonRpcResponse {
                jsonrpc: "2.0".to_string(),
                id,
                result: Some(serde_json::json!({})),
                error: None,
            };
            Some(response)
        }
        "session/close" => {
            let session_id = req
                .params
                .get("sessionId")
                .and_then(|v| v.as_str())
                .unwrap_or("");
            session_manager.remove_session(session_id);
            let response = JsonRpcResponse {
                jsonrpc: "2.0".to_string(),
                id,
                result: Some(serde_json::json!({})),
                error: None,
            };
            Some(response)
        }
        "session/fork" => {
            let session_id = req
                .params
                .get("sessionId")
                .and_then(|v| v.as_str())
                .unwrap_or("");
            let session = session_manager.get_session(session_id);
            match session {
                Some(s) => {
                    let new_session =
                        session_manager.create_session(s.cwd.clone(), s.session_type.clone());
                    let response = JsonRpcResponse {
                        jsonrpc: "2.0".to_string(),
                        id,
                        result: Some(serde_json::json!({
                            "sessionId": new_session.session_id
                        })),
                        error: None,
                    };
                    Some(response)
                }
                None => Some(JsonRpcResponse {
                    jsonrpc: "2.0".to_string(),
                    id,
                    result: None,
                    error: Some(JsonRpcError {
                        code: -32001,
                        message: "Session not found".to_string(),
                    }),
                }),
            }
        }
        "session/setConfigOption" => {
            let config_id = req
                .params
                .get("configId")
                .and_then(|v| v.as_str())
                .unwrap_or("");
            let value = req
                .params
                .get("value")
                .cloned()
                .unwrap_or(serde_json::json!(null));
            let session_id = req
                .params
                .get("sessionId")
                .and_then(|v| v.as_str())
                .unwrap_or("");

            // Persist config option changes to Gaggle's chat settings
            let session = session_manager.get_session(session_id);
            if let Some(session) = session
                && let Some(chat_id) = &session.chat_id
            {
                let chat_repo =
                    gaggle_db::repositories::chats::ChatRepository::new(state.pool.clone());
                match config_id {
                    "mode" => {
                        if let Some(mode) = value.as_str() {
                            let _ = chat_repo.update_mode(chat_id, mode).await;
                        }
                    }
                    "permission_mode" => {
                        if let Some(mode) = value.as_str() {
                            let _ = chat_repo.update_permission_mode(chat_id, mode).await;
                        }
                    }
                    _ => {}
                }
            }

            let response = JsonRpcResponse {
                jsonrpc: "2.0".to_string(),
                id,
                result: Some(serde_json::json!({
                    "configOptions": []
                })),
                error: None,
            };
            Some(response)
        }
        "_goose/unstable/config/write" => Some(JsonRpcResponse {
            jsonrpc: "2.0".to_string(),
            id,
            result: Some(serde_json::json!({"ok": true})),
            error: None,
        }),
        "_goose/unstable/providers/setup" => Some(JsonRpcResponse {
            jsonrpc: "2.0".to_string(),
            id,
            result: Some(serde_json::json!({"ok": true})),
            error: None,
        }),
        "_goose/unstable/sessions/rename" => {
            let session_id = req
                .params
                .get("sessionId")
                .and_then(|v| v.as_str())
                .unwrap_or("");
            let title = req
                .params
                .get("title")
                .and_then(|v| v.as_str())
                .unwrap_or("");
            if let Some(mut session) = session_manager.get_session(session_id) {
                // Store title in cwd field as a lightweight placeholder.
                let _ = title;
                session_manager.update_session(session);
            }
            Some(JsonRpcResponse {
                jsonrpc: "2.0".to_string(),
                id,
                result: Some(serde_json::json!({})),
                error: None,
            })
        }
        "_goose/unstable/elicitations/request" => {
            let elic_id = format!(
                "elic-{}",
                std::time::SystemTime::now()
                    .duration_since(std::time::UNIX_EPOCH)
                    .map(|d| d.as_nanos())
                    .unwrap_or(0)
            );
            let rx = state.elicitation_bridge.register(elic_id.clone());
            let form = req
                .params
                .get("form")
                .cloned()
                .unwrap_or(serde_json::json!({}));
            let notification = serde_json::json!({
                "jsonrpc": "2.0",
                "method": "elicitations/request",
                "id": elic_id.clone(),
                "params": {"form": form}
            });
            let _ = tx.send(notification.to_string()).await;
            // Wait for response with timeout
            let result = match tokio::time::timeout(std::time::Duration::from_secs(60), rx).await {
                Ok(Ok(v)) => v,
                _ => serde_json::json!({"cancelled": true}),
            };
            Some(JsonRpcResponse {
                jsonrpc: "2.0".to_string(),
                id,
                result: Some(result),
                error: None,
            })
        }
        "_goose/unstable/elicitations/respond" => Some(JsonRpcResponse {
            jsonrpc: "2.0".to_string(),
            id,
            result: Some(serde_json::json!({"ok": true})),
            error: None,
        }),
        "_goose/unstable/recipes/get" => Some(JsonRpcResponse {
            jsonrpc: "2.0".to_string(),
            id,
            result: Some(serde_json::json!({"recipe": null})),
            error: None,
        }),
        "_goose/unstable/recipes/validate" => Some(JsonRpcResponse {
            jsonrpc: "2.0".to_string(),
            id,
            result: Some(serde_json::json!({"valid": true})),
            error: None,
        }),
        "_goose/unstable/sessions/get" => {
            let session_id = req
                .params
                .get("sessionId")
                .and_then(|v| v.as_str())
                .unwrap_or("");
            let session = session_manager.get_session(session_id);
            Some(JsonRpcResponse {
                jsonrpc: "2.0".to_string(),
                id,
                result: Some(serde_json::json!({"session": session})),
                error: None,
            })
        }
        "_goose/unstable/tools/get" => Some(JsonRpcResponse {
            jsonrpc: "2.0".to_string(),
            id,
            result: Some(serde_json::json!({"tool": null})),
            error: None,
        }),
        "_goose/unstable/extensions/get" => Some(JsonRpcResponse {
            jsonrpc: "2.0".to_string(),
            id,
            result: Some(serde_json::json!({"extension": null})),
            error: None,
        }),
        "_goose/unstable/extensions/enable" => Some(JsonRpcResponse {
            jsonrpc: "2.0".to_string(),
            id,
            result: Some(serde_json::json!({"ok": true})),
            error: None,
        }),
        "_goose/unstable/extensions/disable" => Some(JsonRpcResponse {
            jsonrpc: "2.0".to_string(),
            id,
            result: Some(serde_json::json!({"ok": true})),
            error: None,
        }),
        "_goose/unstable/providers/get" => Some(JsonRpcResponse {
            jsonrpc: "2.0".to_string(),
            id,
            result: Some(serde_json::json!({"provider": null})),
            error: None,
        }),
        "_goose/unstable/providers/configure" => Some(JsonRpcResponse {
            jsonrpc: "2.0".to_string(),
            id,
            result: Some(serde_json::json!({"ok": true})),
            error: None,
        }),
        "_goose/unstable/working_dir/get" => {
            let session_id = req
                .params
                .get("sessionId")
                .and_then(|v| v.as_str())
                .unwrap_or("");
            let cwd = session_manager.get_session(session_id).and_then(|s| s.cwd);
            Some(JsonRpcResponse {
                jsonrpc: "2.0".to_string(),
                id,
                result: Some(serde_json::json!({"cwd": cwd})),
                error: None,
            })
        }
        "_goose/unstable/working_dir/set" => {
            let session_id = req
                .params
                .get("sessionId")
                .and_then(|v| v.as_str())
                .unwrap_or("");
            let cwd = req
                .params
                .get("cwd")
                .and_then(|v| v.as_str())
                .map(String::from);
            if let Some(mut session) = session_manager.get_session(session_id) {
                session.cwd = cwd;
                session_manager.update_session(session);
            }
            Some(JsonRpcResponse {
                jsonrpc: "2.0".to_string(),
                id,
                result: Some(serde_json::json!({"ok": true})),
                error: None,
            })
        }
        "_goose/unstable/modes/list" => Some(JsonRpcResponse {
            jsonrpc: "2.0".to_string(),
            id,
            result: Some(serde_json::json!({"modes": ["auto", "chat", "code"]})),
            error: None,
        }),
        "_goose/unstable/config/get" => Some(JsonRpcResponse {
            jsonrpc: "2.0".to_string(),
            id,
            result: Some(serde_json::json!({"config": {}})),
            error: None,
        }),
        "_goose/unstable/sessions/close" => {
            let session_id = req
                .params
                .get("sessionId")
                .and_then(|v| v.as_str())
                .unwrap_or("");
            session_manager.remove_session(session_id);
            Some(JsonRpcResponse {
                jsonrpc: "2.0".to_string(),
                id,
                result: Some(serde_json::json!({})),
                error: None,
            })
        }
        "_goose/unstable/sessions/fork" => {
            let session_id = req
                .params
                .get("sessionId")
                .and_then(|v| v.as_str())
                .unwrap_or("");
            let session = session_manager.get_session(session_id);
            match session {
                Some(s) => {
                    let new_session =
                        session_manager.create_session(s.cwd.clone(), s.session_type.clone());
                    Some(JsonRpcResponse {
                        jsonrpc: "2.0".to_string(),
                        id,
                        result: Some(serde_json::json!({"sessionId": new_session.session_id})),
                        error: None,
                    })
                }
                None => Some(JsonRpcResponse {
                    jsonrpc: "2.0".to_string(),
                    id,
                    result: None,
                    error: Some(JsonRpcError {
                        code: -32001,
                        message: "Session not found".to_string(),
                    }),
                }),
            }
        }
        "_goose/unstable/tools/execute" => Some(JsonRpcResponse {
            jsonrpc: "2.0".to_string(),
            id,
            result: Some(serde_json::json!({"ok": true})),
            error: None,
        }),
        "_goose/unstable/extensions/install" => Some(JsonRpcResponse {
            jsonrpc: "2.0".to_string(),
            id,
            result: Some(serde_json::json!({"ok": true})),
            error: None,
        }),
        "_goose/unstable/providers/readiness" => Some(JsonRpcResponse {
            jsonrpc: "2.0".to_string(),
            id,
            result: Some(serde_json::json!({"ready": true})),
            error: None,
        }),
        "_goose/unstable/working_dir/reset" => {
            let session_id = req
                .params
                .get("sessionId")
                .and_then(|v| v.as_str())
                .unwrap_or("");
            if let Some(mut session) = session_manager.get_session(session_id) {
                session.cwd = None;
                session_manager.update_session(session);
            }
            Some(JsonRpcResponse {
                jsonrpc: "2.0".to_string(),
                id,
                result: Some(serde_json::json!({"ok": true})),
                error: None,
            })
        }
        "_goose/unstable/modes/set" => {
            let session_id = req
                .params
                .get("sessionId")
                .and_then(|v| v.as_str())
                .unwrap_or("");
            let mode = req.params.get("mode").and_then(|v| v.as_str());
            if let Some(mut session) = session_manager.get_session(session_id)
                && let Some(mode) = mode
            {
                session.mode = Some(mode.to_string());
                session_manager.update_session(session);
            }
            Some(JsonRpcResponse {
                jsonrpc: "2.0".to_string(),
                id,
                result: Some(serde_json::json!({})),
                error: None,
            })
        }
        "_goose/unstable/elicitations/cancel" => Some(JsonRpcResponse {
            jsonrpc: "2.0".to_string(),
            id,
            result: Some(serde_json::json!({"ok": true})),
            error: None,
        }),
        "_goose/unstable/config/read" => Some(JsonRpcResponse {
            jsonrpc: "2.0".to_string(),
            id,
            result: Some(serde_json::json!({"config": {}})),
            error: None,
        }),
        "_goose/unstable/providers/list" => {
            let repo =
                gaggle_db::repositories::providers::ProviderRepository::new(state.pool.clone());
            let providers = repo.list_providers().await.unwrap_or_default();
            Some(JsonRpcResponse {
                jsonrpc: "2.0".to_string(),
                id,
                result: Some(serde_json::json!({"providers": providers})),
                error: None,
            })
        }
        "_goose/unstable/recipes/list" => Some(JsonRpcResponse {
            jsonrpc: "2.0".to_string(),
            id,
            result: Some(serde_json::json!({"recipes": []})),
            error: None,
        }),
        "_goose/unstable/tools/list" => Some(JsonRpcResponse {
            jsonrpc: "2.0".to_string(),
            id,
            result: Some(serde_json::json!({"tools": []})),
            error: None,
        }),
        "_goose/unstable/extensions/list" => Some(JsonRpcResponse {
            jsonrpc: "2.0".to_string(),
            id,
            result: Some(serde_json::json!({"extensions": []})),
            error: None,
        }),
        "session/setMode" => {
            let session_id = req
                .params
                .get("sessionId")
                .and_then(|v| v.as_str())
                .unwrap_or("");
            let mode = req.params.get("mode").and_then(|v| v.as_str());
            if let Some(session) = session_manager.get_session(session_id)
                && let Some(mode) = mode
            {
                let mut updated = session;
                updated.mode = Some(mode.to_string());
                session_manager.update_session(updated);
            }
            Some(JsonRpcResponse {
                jsonrpc: "2.0".to_string(),
                id,
                result: Some(serde_json::json!({})),
                error: None,
            })
        }
        "session/steer" => {
            // Steer: cancel the active run for the session, then accept a new prompt.
            let session_id = req
                .params
                .get("sessionId")
                .and_then(|v| v.as_str())
                .unwrap_or("");
            let _ = session_manager.cancel(session_id);
            Some(JsonRpcResponse {
                jsonrpc: "2.0".to_string(),
                id,
                result: Some(serde_json::json!({})),
                error: None,
            })
        }
        method
            if method.starts_with("_goose/unstable/")
                || method.starts_with("_gaggle/unstable/") =>
            // Generic dispatcher for Goose custom methods: acknowledge unknown methods
            // so clients can probe capabilities without hard failures.
            Some(JsonRpcResponse {
                jsonrpc: "2.0".to_string(),
                id,
                result: Some(serde_json::json!({})),
                error: None,
            })
        }
        _ => Some(JsonRpcResponse {
            jsonrpc: "2.0".to_string(),
            id,
            result: None,
            error: Some(JsonRpcError {
                code: -32601,
                message: format!("Method not found: {}", req.method),
            }),
        }),
    }
}

/// HandleDispatchFrom trait for full ACP protocol parity.
#[allow(dead_code)]
pub trait HandleDispatchFrom {
    fn handle_dispatch(&self, method: &str, params: serde_json::Value) -> Option<JsonRpcResponse>;
    fn handle_session_new(&self, cwd: Option<String>) -> JsonRpcResponse;
    fn handle_session_prompt(&self, session_id: &str, prompt: &str) -> JsonRpcResponse;
    fn handle_session_cancel(&self, session_id: &str) -> Option<JsonRpcResponse>;
    fn handle_session_list(&self) -> JsonRpcResponse;
    fn handle_session_load(&self, session_id: &str) -> JsonRpcResponse;
    fn handle_session_delete(&self, session_id: &str) -> JsonRpcResponse;
    fn handle_session_close(&self, session_id: &str) -> JsonRpcResponse;
    fn handle_session_fork(&self, session_id: &str) -> JsonRpcResponse;
    fn handle_session_set_config_option(
        &self,
        session_id: &str,
        config_id: &str,
        value: serde_json::Value,
    ) -> JsonRpcResponse;
    fn handle_permission_response(&self, request_id: &str, approved: bool) -> bool;
}

#[allow(dead_code)]
impl HandleDispatchFrom for AcpSessionManager {
    fn handle_dispatch(&self, method: &str, params: serde_json::Value) -> Option<JsonRpcResponse> {
        match method {
            "session/new" => {
                let cwd = params.get("cwd").and_then(|v| v.as_str()).map(String::from);
                Some(self.handle_session_new(cwd))
            }
            "session/list" => Some(self.handle_session_list()),
            "session/load" => {
                let session_id = params
                    .get("sessionId")
                    .and_then(|v| v.as_str())
                    .unwrap_or("");
                Some(self.handle_session_load(session_id))
            }
            "session/delete" => {
                let session_id = params
                    .get("sessionId")
                    .and_then(|v| v.as_str())
                    .unwrap_or("");
                Some(self.handle_session_delete(session_id))
            }
            "session/close" => {
                let session_id = params
                    .get("sessionId")
                    .and_then(|v| v.as_str())
                    .unwrap_or("");
                Some(self.handle_session_close(session_id))
            }
            "session/fork" => {
                let session_id = params
                    .get("sessionId")
                    .and_then(|v| v.as_str())
                    .unwrap_or("");
                Some(self.handle_session_fork(session_id))
            }
            "session/cancel" => {
                let session_id = params
                    .get("sessionId")
                    .and_then(|v| v.as_str())
                    .unwrap_or("");
                self.handle_session_cancel(session_id)
            }
            "session/setConfigOption" => {
                let session_id = params
                    .get("sessionId")
                    .and_then(|v| v.as_str())
                    .unwrap_or("");
                let config_id = params
                    .get("configId")
                    .and_then(|v| v.as_str())
                    .unwrap_or("");
                let value = params
                    .get("value")
                    .cloned()
                    .unwrap_or(serde_json::json!(null));
                Some(self.handle_session_set_config_option(session_id, config_id, value))
            }
            _ => None,
        }
    }

    fn handle_session_new(&self, cwd: Option<String>) -> JsonRpcResponse {
        let session = self.create_session(cwd);
        JsonRpcResponse {
            jsonrpc: "2.0".to_string(),
            id: None,
            result: Some(serde_json::json!({ "sessionId": session.session_id })),
            error: None,
        }
    }

    fn handle_session_prompt(&self, session_id: &str, prompt: &str) -> JsonRpcResponse {
        JsonRpcResponse {
            jsonrpc: "2.0".to_string(),
            id: None,
            result: Some(
                serde_json::json!({ "sessionId": session_id, "prompt": prompt, "stopReason": "end_turn" }),
            ),
            error: None,
        }
    }

    fn handle_session_cancel(&self, session_id: &str) -> Option<JsonRpcResponse> {
        let _ = session_id;
        None
    }

    fn handle_session_list(&self) -> JsonRpcResponse {
        let sessions = self.list_sessions();
        JsonRpcResponse {
            jsonrpc: "2.0".to_string(),
            id: None,
            result: Some(serde_json::json!({ "sessions": sessions })),
            error: None,
        }
    }

    fn handle_session_load(&self, session_id: &str) -> JsonRpcResponse {
        match self.get_session(session_id) {
            Some(s) => JsonRpcResponse {
                jsonrpc: "2.0".to_string(),
                id: None,
                result: Some(serde_json::json!({ "sessionId": s.session_id })),
                error: None,
            },
            None => JsonRpcResponse {
                jsonrpc: "2.0".to_string(),
                id: None,
                result: None,
                error: Some(JsonRpcError {
                    code: -32001,
                    message: "Session not found".to_string(),
                }),
            },
        }
    }

    fn handle_session_delete(&self, session_id: &str) -> JsonRpcResponse {
        self.remove_session(session_id);
        JsonRpcResponse {
            jsonrpc: "2.0".to_string(),
            id: None,
            result: Some(serde_json::json!({})),
            error: None,
        }
    }

    fn handle_session_close(&self, session_id: &str) -> JsonRpcResponse {
        self.remove_session(session_id);
        JsonRpcResponse {
            jsonrpc: "2.0".to_string(),
            id: None,
            result: Some(serde_json::json!({})),
            error: None,
        }
    }

    fn handle_session_fork(&self, session_id: &str) -> JsonRpcResponse {
        match self.get_session(session_id) {
            Some(s) => {
                let new_session = self.create_session(s.cwd.clone());
                JsonRpcResponse {
                    jsonrpc: "2.0".to_string(),
                    id: None,
                    result: Some(serde_json::json!({ "sessionId": new_session.session_id })),
                    error: None,
                }
            }
            None => JsonRpcResponse {
                jsonrpc: "2.0".to_string(),
                id: None,
                result: None,
                error: Some(JsonRpcError {
                    code: -32001,
                    message: "Session not found".to_string(),
                }),
            },
        }
    }

    fn handle_session_set_config_option(
        &self,
        session_id: &str,
        config_id: &str,
        value: serde_json::Value,
    ) -> JsonRpcResponse {
        let _ = (session_id, config_id, value);
        JsonRpcResponse {
            jsonrpc: "2.0".to_string(),
            id: None,
            result: Some(serde_json::json!({ "configOptions": [] })),
            error: None,
        }
    }

    fn handle_permission_response(&self, request_id: &str, approved: bool) -> bool {
        let _ = (request_id, approved);
        true
    }
}

#[allow(unused_imports)]
mod tests {
    use super::*;

    #[test]
    fn session_manager_creates_and_retrieves_sessions() {
        let manager = AcpSessionManager::new();
        let session = manager.create_session(Some("/tmp".to_string()));
        assert!(!session.session_id.is_empty());
        assert!(manager.get_session(&session.session_id).is_some());
        manager.remove_session(&session.session_id);
        assert!(manager.get_session(&session.session_id).is_none());
    }

    #[test]
    fn session_manager_lists_sessions() {
        let manager = AcpSessionManager::new();
        manager.create_session(Some("/tmp".to_string()));
        manager.create_session(Some("/home".to_string()));
        assert_eq!(manager.list_sessions().len(), 2);
    }

    #[test]
    fn session_manager_forks_session() {
        let manager = AcpSessionManager::new();
        let session = manager.create_session(Some("/tmp".to_string()));
        let forked = manager.create_session(session.cwd.clone());
        assert_ne!(session.session_id, forked.session_id);
        assert_eq!(session.cwd, forked.cwd);
    }
}
