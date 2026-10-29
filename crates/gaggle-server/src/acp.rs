//! Minimal Agent Client Protocol (ACP) compatibility layer for Gaggle.
//!
//! This module provides ACP-compatible endpoints so the rebranded Gaggle
//! Electron desktop app can communicate with the Gaggle backend. It wraps
//! Gaggle's existing REST/SSE agent runtime in ACP-compatible handlers.

use axum::{
    Router,
    extract::{
        State,
        ws::{Message, WebSocket, WebSocketUpgrade},
    },
    response::IntoResponse,
    routing::get,
};
use futures::{SinkExt, StreamExt};
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};
use std::sync::Arc;
use tokio::sync::mpsc;
use uuid::Uuid;

use crate::AppState;

/// ACP session state.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AcpSession {
    pub session_id: String,
    pub chat_id: Option<String>,
    pub project_id: Option<String>,
    pub cwd: Option<String>,
    pub created_at: i64,
}

/// JSON-RPC request.
#[derive(Debug, Deserialize)]
pub struct JsonRpcRequest {
    pub jsonrpc: String,
    pub id: Option<Value>,
    pub method: String,
    #[serde(default)]
    pub params: Value,
}

/// JSON-RPC response.
#[derive(Debug, Serialize)]
pub struct JsonRpcResponse {
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

/// ACP session manager (in-memory).
pub struct AcpSessionManager {
    sessions: std::sync::Mutex<std::collections::HashMap<String, AcpSession>>,
}

impl AcpSessionManager {
    pub fn new() -> Self {
        Self {
            sessions: std::sync::Mutex::new(std::collections::HashMap::new()),
        }
    }

    pub fn create_session(&self, cwd: Option<String>) -> AcpSession {
        let session_id = Uuid::new_v4().to_string();
        let session = AcpSession {
            session_id: session_id.clone(),
            chat_id: None,
            project_id: None,
            cwd,
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
    axum::Json(json!({
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
    let session_manager = Arc::new(AcpSessionManager::new());
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
            let request: Result<JsonRpcRequest, _> = serde_json::from_str(&text);
            match request {
                Ok(req) => {
                    let response = handle_request(&req, &state, &session_manager).await;
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
) -> Option<JsonRpcResponse> {
    let id = req.id.clone();

    match req.method.as_str() {
        "initialize" => {
            let response = JsonRpcResponse {
                jsonrpc: "2.0".to_string(),
                id,
                result: Some(json!({
                    "protocolVersion": 1,
                    "agentCapabilities": {
                        "loadSession": true,
                        "promptCapabilities": {
                            "image": false,
                            "audio": false,
                            "embeddedContext": false
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
                    }
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
            let session = session_manager.create_session(cwd);
            let response = JsonRpcResponse {
                jsonrpc: "2.0".to_string(),
                id,
                result: Some(json!({
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
            // For now, return a simple response
            // Full implementation would delegate to Gaggle's agent loop
            let response = JsonRpcResponse {
                jsonrpc: "2.0".to_string(),
                id,
                result: Some(json!({
                    "stopReason": "end_turn"
                })),
                error: None,
            };
            Some(response)
        }
        "session/cancel" => {
            // Cancel is a notification, no response needed
            // Full implementation would cancel the run
            None
        }
        "session/list" => {
            let sessions = session_manager.list_sessions();
            let response = JsonRpcResponse {
                jsonrpc: "2.0".to_string(),
                id,
                result: Some(json!({
                    "sessions": sessions
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
                        result: Some(json!({
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
                result: Some(json!({})),
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
                result: Some(json!({})),
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
                    let new_session = session_manager.create_session(s.cwd.clone());
                    let response = JsonRpcResponse {
                        jsonrpc: "2.0".to_string(),
                        id,
                        result: Some(json!({
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
            // Acknowledge config option changes
            let response = JsonRpcResponse {
                jsonrpc: "2.0".to_string(),
                id,
                result: Some(json!({
                    "configOptions": []
                })),
                error: None,
            };
            Some(response)
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

#[cfg(test)]
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
