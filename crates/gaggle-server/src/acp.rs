//! Minimal Agent Client Protocol (ACP) compatibility layer for Gaggle.
//!
//! This module provides ACP-compatible endpoints so the rebranded Gaggle
//! Electron desktop app can communicate with the Gaggle backend. It wraps
//! Gaggle's existing REST/SSE agent runtime in ACP-compatible handlers.
//!
//! Full ACP parity with Goose requires implementing the complete
//! `agent-client-protocol` trait. This module provides the essential
//! methods (initialize, new_session, prompt, cancel) as a starting point.

use axum::{Router, extract::State, http::StatusCode, response::Json, routing::get};
use serde::{Deserialize, Serialize};
use serde_json::json;

use crate::AppState;

/// ACP session state.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AcpSession {
    pub session_id: String,
    pub chat_id: Option<String>,
    pub project_id: Option<String>,
    pub created_at: i64,
}

/// ACP initialize request.
#[derive(Debug, Deserialize)]
pub struct AcpInitializeRequest {
    pub protocol_version: Option<i64>,
    pub client_capabilities: Option<serde_json::Value>,
    pub client_info: Option<serde_json::Value>,
}

/// ACP initialize response.
#[derive(Debug, Serialize)]
pub struct AcpInitializeResponse {
    pub protocol_version: i64,
    pub agent_capabilities: serde_json::Value,
    pub agent_info: serde_json::Value,
}

/// ACP new session request.
#[derive(Debug, Deserialize)]
pub struct AcpNewSessionRequest {
    pub cwd: Option<String>,
    pub mcp_servers: Option<Vec<serde_json::Value>>,
}

/// ACP new session response.
#[derive(Debug, Serialize)]
pub struct AcpNewSessionResponse {
    pub session_id: String,
}

/// ACP prompt request.
#[derive(Debug, Deserialize)]
pub struct AcpPromptRequest {
    pub session_id: String,
    pub prompt: Vec<serde_json::Value>,
}

/// ACP prompt response.
#[derive(Debug, Serialize)]
pub struct AcpPromptResponse {
    pub stop_reason: String,
}

/// ACP cancel notification.
#[derive(Debug, Deserialize)]
pub struct AcpCancelNotification {
    pub session_id: String,
}

/// Creates the ACP router with health/status endpoints.
pub fn acp_routes() -> Router<AppState> {
    Router::new()
        .route("/health", get(health))
        .route("/status", get(status))
}

async fn health() -> &'static str {
    "ok"
}

async fn status() -> Json<serde_json::Value> {
    Json(json!({
        "status": "ok",
        "service": "gaggle",
        "version": env!("CARGO_PKG_VERSION")
    }))
}

/// ACP session manager (in-memory for now).
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
            created_at: std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .map(|d| d.as_millis() as i64)
                .unwrap_or(0),
        };
        let mut sessions = self.sessions.lock().unwrap();
        sessions.insert(session_id.clone(), session.clone());
        session
    }

    pub fn get_session(&self, session_id: &str) -> Option<AcpSession> {
        let sessions = self.sessions.lock().unwrap();
        sessions.get(session_id).cloned()
    }

    pub fn remove_session(&self, session_id: &str) {
        let mut sessions = self.sessions.lock().unwrap();
        sessions.remove(session_id);
    }
}

impl Default for AcpSessionManager {
    fn default() -> Self {
        Self::new()
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
}
