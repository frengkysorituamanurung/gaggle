//! Minimal Agent Client Protocol (ACP) transport abstraction.
//!
//! This module provides a small, additive ACP surface used by Gaggle for
//! exchanging request/response messages over a transport. It intentionally
//! avoids any network or process concerns; concrete transports can be added
//! later.

use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use std::sync::{Arc, Mutex};

/// An ACP request message.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct AcpMessage {
    pub id: String,
    pub method: String,
    pub params: serde_json::Value,
    pub seq: u64,
}

/// An ACP response message.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AcpResponse {
    pub id: String,
    pub result: Option<serde_json::Value>,
    pub error: Option<String>,
}

/// Errors produced by an [`AcpTransport`].
#[derive(Debug, Clone, thiserror::Error)]
pub enum AcpError {
    #[error("I/O error: {0}")]
    Io(String),
    #[error("protocol error: {0}")]
    Protocol(String),
    #[error("request timed out")]
    Timeout,
}

/// Async transport capable of sending an [`AcpMessage`] and receiving an
/// [`AcpResponse`].
#[async_trait]
pub trait AcpTransport: Send + Sync {
    async fn send(&self, msg: AcpMessage) -> Result<AcpResponse, AcpError>;
    fn name(&self) -> &str;
}

/// In-memory transport that records the last sent message and returns an echo
/// response.
#[derive(Clone)]
pub struct InMemoryAcpTransport {
    pub last_message: Arc<Mutex<Option<AcpMessage>>>,
}

impl InMemoryAcpTransport {
    pub fn new() -> Self {
        Self {
            last_message: Arc::new(Mutex::new(None)),
        }
    }

    /// Returns a clone of the last message recorded by this transport, if any.
    pub fn last_message_snapshot(&self) -> Option<AcpMessage> {
        self.last_message
            .lock()
            .ok()
            .and_then(|guard| guard.clone())
    }
}

impl Default for InMemoryAcpTransport {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl AcpTransport for InMemoryAcpTransport {
    async fn send(&self, msg: AcpMessage) -> Result<AcpResponse, AcpError> {
        {
            let mut guard = self
                .last_message
                .lock()
                .map_err(|_| AcpError::Io("lock poisoned".to_string()))?;
            *guard = Some(msg.clone());
        }
        Ok(AcpResponse {
            id: msg.id.clone(),
            result: Some(msg.params.clone()),
            error: None,
        })
    }

    fn name(&self) -> &str {
        "in-memory"
    }
}

/// Minimal HTTP transport that POSTs an [`AcpMessage`] as JSON and parses an
/// [`AcpResponse`] from the response body.
#[derive(Clone)]
pub struct HttpAcpTransport {
    pub url: String,
    pub client: reqwest::Client,
}

impl HttpAcpTransport {
    pub fn new(url: impl Into<String>) -> Self {
        Self {
            url: url.into(),
            client: reqwest::Client::new(),
        }
    }
}

#[async_trait]
impl AcpTransport for HttpAcpTransport {
    async fn send(&self, msg: AcpMessage) -> Result<AcpResponse, AcpError> {
        let response = self
            .client
            .post(&self.url)
            .json(&msg)
            .send()
            .await
            .map_err(|e| AcpError::Io(e.to_string()))?;
        if !response.status().is_success() {
            return Err(AcpError::Protocol(format!(
                "HTTP status {}",
                response.status()
            )));
        }
        response
            .json::<AcpResponse>()
            .await
            .map_err(|e| AcpError::Protocol(e.to_string()))
    }

    fn name(&self) -> &str {
        "http"
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn round_trip_echo() {
        let transport = InMemoryAcpTransport::new();
        let msg = AcpMessage {
            id: "msg-1".to_string(),
            method: "echo".to_string(),
            params: serde_json::json!({"hello": "world"}),
            seq: 7,
        };
        let response = transport.send(msg.clone()).await.expect("send ok");
        assert_eq!(response.id, msg.id);
        assert_eq!(response.result, Some(msg.params.clone()));
        assert!(response.error.is_none());

        let stored = transport.last_message_snapshot().expect("stored");
        assert_eq!(stored, msg);
    }

    #[test]
    fn name_is_stable() {
        let transport = InMemoryAcpTransport::new();
        assert_eq!(transport.name(), "in-memory");
        assert!(!transport.name().is_empty());
    }
}
