//! Minimal ACP (Agent Client Protocol) client over HTTP (JSON-RPC 2.0).
//!
//! Pure Rust implementation using `reqwest`. This client builds JSON-RPC
//! requests and POSTs them to an ACP-compatible HTTP endpoint.

use crate::acp::AcpError;
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Arc;

/// Re-exported alias for callers that prefer a client-specific error name.
pub type AcpClientError = AcpError;

/// A JSON-RPC 2.0 request envelope used by [`AcpClient`].
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct JsonRpcRequest {
    pub jsonrpc: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub id: Option<u64>,
    pub method: String,
    pub params: Value,
}

/// A JSON-RPC 2.0 error object.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct JsonRpcError {
    pub code: i64,
    pub message: String,
}

/// A JSON-RPC 2.0 response envelope.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct JsonRpcResponse {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub id: Option<u64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub result: Option<Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<JsonRpcError>,
}

/// HTTP JSON-RPC client for an ACP server.
#[derive(Clone)]
pub struct AcpClient {
    url: String,
    client: reqwest::Client,
    next_id: Arc<AtomicU64>,
}

impl AcpClient {
    /// Create a new client pointing at `url`.
    pub fn new(url: impl Into<String>) -> Self {
        Self {
            url: url.into(),
            client: reqwest::Client::new(),
            next_id: Arc::new(AtomicU64::new(1)),
        }
    }

    /// Create a client with a pre-configured reqwest client.
    pub fn with_client(url: impl Into<String>, client: reqwest::Client) -> Self {
        Self {
            url: url.into(),
            client,
            next_id: Arc::new(AtomicU64::new(1)),
        }
    }

    /// Base URL of the ACP endpoint.
    pub fn url(&self) -> &str {
        &self.url
    }

    fn next_request_id(&self) -> u64 {
        self.next_id.fetch_add(1, Ordering::Relaxed)
    }

    /// Generic request builder.
    pub fn build_request(&self, method: impl Into<String>, params: Value) -> JsonRpcRequest {
        JsonRpcRequest {
            jsonrpc: "2.0".to_string(),
            id: Some(self.next_request_id()),
            method: method.into(),
            params,
        }
    }

    /// Build a JSON-RPC notification (no `id`).
    pub fn build_notification(&self, method: impl Into<String>, params: Value) -> JsonRpcRequest {
        JsonRpcRequest {
            jsonrpc: "2.0".to_string(),
            id: None,
            method: method.into(),
            params,
        }
    }

    pub fn build_initialize_request(&self) -> JsonRpcRequest {
        self.build_request(
            "initialize",
            json!({
                "protocolVersion": 1,
                "clientCapabilities": {},
                "clientInfo": {
                    "name": "gaggle",
                    "version": env!("CARGO_PKG_VERSION")
                }
            }),
        )
    }

    pub fn build_new_session_request(&self, cwd: Option<&str>) -> JsonRpcRequest {
        let mut params = json!({});
        if let Some(cwd) = cwd {
            params["cwd"] = json!(cwd);
        }
        self.build_request("session/new", params)
    }

    pub fn build_prompt_request(&self, session_id: &str, text: &str) -> JsonRpcRequest {
        self.build_request(
            "session/prompt",
            json!({
                "sessionId": session_id,
                "prompt": [{"type": "text", "text": text}]
            }),
        )
    }

    pub fn build_cancel_request(&self, session_id: &str) -> JsonRpcRequest {
        self.build_notification(
            "session/cancel",
            json!({"sessionId": session_id}),
        )
    }

    pub fn build_list_sessions_request(&self) -> JsonRpcRequest {
        self.build_request("session/list", json!({}))
    }

    // Convenience aliases for request building.
    pub fn initialize_request(&self) -> JsonRpcRequest {
        self.build_initialize_request()
    }
    pub fn new_session_request(&self, cwd: Option<&str>) -> JsonRpcRequest {
        self.build_new_session_request(cwd)
    }
    pub fn prompt_request(&self, session_id: &str, text: &str) -> JsonRpcRequest {
        self.build_prompt_request(session_id, text)
    }
    pub fn cancel_request(&self, session_id: &str) -> JsonRpcRequest {
        self.build_cancel_request(session_id)
    }
    pub fn list_sessions_request(&self) -> JsonRpcRequest {
        self.build_list_sessions_request()
    }

    async fn rpc(&self, req: JsonRpcRequest) -> Result<Option<Value>, AcpError> {
        let response = self
            .client
            .post(&self.url)
            .json(&req)
            .send()
            .await
            .map_err(|e| {
                if e.is_timeout() {
                    AcpError::Timeout
                } else {
                    AcpError::Io(e.to_string())
                }
            })?;

        if !response.status().is_success() {
            return Err(AcpError::Protocol(format!(
                "HTTP status {}",
                response.status()
            )));
        }

        let bytes = response.bytes().await.map_err(|e| AcpError::Io(e.to_string()))?;
        if bytes.is_empty() {
            return Ok(None);
        }

        let parsed: JsonRpcResponse =
            serde_json::from_slice(&bytes).map_err(|e| AcpError::Protocol(e.to_string()))?;
        if let Some(err) = parsed.error {
            return Err(AcpError::Protocol(err.message));
        }
        Ok(parsed.result)
    }

    pub async fn initialize(&self) -> Result<Value, AcpError> {
        let req = self.build_initialize_request();
        Ok(self.rpc(req).await?.unwrap_or_else(|| json!({})))
    }

    pub async fn new_session(&self, cwd: Option<String>) -> Result<Value, AcpError> {
        let req = self.build_new_session_request(cwd.as_deref());
        Ok(self.rpc(req).await?.unwrap_or_else(|| json!({})))
    }

    pub async fn prompt(&self, session_id: &str, text: &str) -> Result<Value, AcpError> {
        let req = self.build_prompt_request(session_id, text);
        Ok(self.rpc(req).await?.unwrap_or_else(|| json!({})))
    }

    pub async fn cancel(&self, session_id: &str) -> Result<(), AcpError> {
        let req = self.build_cancel_request(session_id);
        let _ = self.rpc(req).await?;
        Ok(())
    }

    pub async fn list_sessions(&self) -> Result<Value, AcpError> {
        let req = self.build_list_sessions_request();
        Ok(self.rpc(req).await?.unwrap_or_else(|| json!({})))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn initialize_request_serializes() {
        let client = AcpClient::new("http://example/acp");
        let req = client.build_initialize_request();
        let v = serde_json::to_value(&req).unwrap();
        assert_eq!(v["jsonrpc"], "2.0");
        assert_eq!(v["method"], "initialize");
        assert!(v["id"].is_number());
        assert!(v["params"]["protocolVersion"] == 1);
    }

    #[test]
    fn new_session_request_serializes() {
        let client = AcpClient::new("http://example/acp");
        let req = client.build_new_session_request(Some("/tmp/proj"));
        let v = serde_json::to_value(&req).unwrap();
        assert_eq!(v["method"], "session/new");
        assert_eq!(v["params"]["cwd"], "/tmp/proj");
    }

    #[test]
    fn prompt_request_serializes() {
        let client = AcpClient::new("http://example/acp");
        let req = client.build_prompt_request("sess-1", "hello");
        let v = serde_json::to_value(&req).unwrap();
        assert_eq!(v["method"], "session/prompt");
        assert_eq!(v["params"]["sessionId"], "sess-1");
        assert_eq!(v["params"]["prompt"][0]["text"], "hello");
    }

    #[test]
    fn cancel_request_is_notification() {
        let client = AcpClient::new("http://example/acp");
        let req = client.build_cancel_request("sess-1");
        let v = serde_json::to_value(&req).unwrap();
        assert_eq!(v["method"], "session/cancel");
        assert!(v.get("id").is_none());
        assert_eq!(v["params"]["sessionId"], "sess-1");
    }

    #[test]
    fn list_sessions_request_serializes() {
        let client = AcpClient::new("http://example/acp");
        let req = client.build_list_sessions_request();
        let v = serde_json::to_value(&req).unwrap();
        assert_eq!(v["method"], "session/list");
        assert!(v["id"].is_number());
    }

    #[test]
    fn aliases_work() {
        let client = AcpClient::new("http://example/acp");
        assert_eq!(client.initialize_request().method, "initialize");
        assert_eq!(client.new_session_request(None).method, "session/new");
        assert_eq!(client.prompt_request("a", "b").method, "session/prompt");
        assert_eq!(client.cancel_request("a").method, "session/cancel");
        assert_eq!(client.list_sessions_request().method, "session/list");
    }
}