use crate::credentials::{CredentialError, CredentialStore, SecretString, redact_sensitive_text};
use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use std::{sync::Arc, time::Duration};
use thiserror::Error;

#[derive(Debug, Clone, Error, PartialEq, Eq)]
pub enum ProviderError {
    #[error("Invalid or unsupported provider base URL '{url}': {reason}")]
    InvalidBaseUrl { url: String, reason: String },

    #[error("Authentication failed for provider '{provider}': {message}")]
    AuthenticationFailed { provider: String, message: String },

    #[error("Rate limit exceeded for provider '{provider}'")]
    RateLimited {
        provider: String,
        retry_after_ms: Option<u64>,
    },

    #[error("Provider request timed out after {timeout_ms} ms")]
    Timeout { timeout_ms: u64 },

    #[error("Malformed response from provider '{provider}': {reason}")]
    MalformedResponse { provider: String, reason: String },

    #[error("Provider transport error for '{provider}': {message}")]
    Transport { provider: String, message: String },

    #[error(transparent)]
    Credential(#[from] CredentialError),
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ProviderMessage {
    pub role: String, // "system" | "user" | "assistant" | "tool"
    pub content: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ToolDefinition {
    pub name: String,
    pub description: String,
    pub input_schema: serde_json::Value,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ProposedToolCall {
    pub id: String,
    pub tool_name: String,
    pub arguments_json: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Default)]
pub struct TokenUsage {
    pub input_tokens: i64,
    pub output_tokens: i64,
    pub cached_tokens: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ModelCompletionRequest {
    pub model_key: String,
    pub messages: Vec<ProviderMessage>,
    pub tools: Vec<ToolDefinition>,
    pub max_output_tokens: Option<u32>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ModelCompletionResponse {
    pub text: String,
    pub tool_calls: Vec<ProposedToolCall>,
    pub usage: TokenUsage,
}

/// Validates a provider base URL to ensure only `http://` or `https://` schemes with a valid host
/// are accepted.
pub fn validate_provider_base_url(raw_url: &str) -> Result<String, ProviderError> {
    let trimmed = raw_url.trim().trim_end_matches('/');
    let parsed = reqwest::Url::parse(trimmed).map_err(|e| ProviderError::InvalidBaseUrl {
        url: raw_url.to_string(),
        reason: e.to_string(),
    })?;

    if !matches!(parsed.scheme(), "http" | "https") {
        return Err(ProviderError::InvalidBaseUrl {
            url: raw_url.to_string(),
            reason: format!("Unsupported URL scheme '{}'", parsed.scheme()),
        });
    }

    if parsed.host_str().is_none() {
        return Err(ProviderError::InvalidBaseUrl {
            url: raw_url.to_string(),
            reason: "Missing host".to_string(),
        });
    }

    Ok(trimmed.to_string())
}

#[async_trait]
pub trait ModelProviderAdapter: Send + Sync {
    fn provider_name(&self) -> &str;
    async fn complete(
        &self,
        req: &ModelCompletionRequest,
    ) -> Result<ModelCompletionResponse, ProviderError>;
}

/// Deterministic mock provider for offline unit, integration, and E2E testing without paid API calls.
#[derive(Clone)]
pub struct MockProviderAdapter {
    provider_name: String,
    canned_response: Result<ModelCompletionResponse, ProviderError>,
}

impl MockProviderAdapter {
    pub fn with_response(
        provider_name: impl Into<String>,
        response: ModelCompletionResponse,
    ) -> Self {
        Self {
            provider_name: provider_name.into(),
            canned_response: Ok(response),
        }
    }

    pub fn with_error(provider_name: impl Into<String>, error: ProviderError) -> Self {
        Self {
            provider_name: provider_name.into(),
            canned_response: Err(error),
        }
    }
}

#[async_trait]
impl ModelProviderAdapter for MockProviderAdapter {
    fn provider_name(&self) -> &str {
        &self.provider_name
    }

    async fn complete(
        &self,
        _req: &ModelCompletionRequest,
    ) -> Result<ModelCompletionResponse, ProviderError> {
        self.canned_response.clone()
    }
}

/// OpenAI-compatible HTTP chat completion adapter (supports OpenAI, OpenRouter, DeepSeek, Ollama,
/// and local OpenAI-compatible servers). Looks up credentials via `secret_ref` from `CredentialStore`
/// and redacts secrets from all error messages.
pub struct OpenAiCompatibleAdapter {
    provider_name: String,
    base_url: String,
    secret_ref: Option<String>,
    credential_store: Arc<dyn CredentialStore>,
    client: reqwest::Client,
    timeout_ms: u64,
}

impl OpenAiCompatibleAdapter {
    pub fn new(
        provider_name: impl Into<String>,
        base_url: &str,
        secret_ref: Option<String>,
        credential_store: Arc<dyn CredentialStore>,
        timeout_ms: u64,
    ) -> Result<Self, ProviderError> {
        let validated_url = validate_provider_base_url(base_url)?;
        let client = reqwest::Client::builder()
            .timeout(Duration::from_millis(timeout_ms))
            .build()
            .map_err(|e| ProviderError::Transport {
                provider: "openai_compat".to_string(),
                message: e.to_string(),
            })?;

        Ok(Self {
            provider_name: provider_name.into(),
            base_url: validated_url,
            secret_ref,
            credential_store,
            client,
            timeout_ms,
        })
    }

    /// Parses an OpenAI-compatible JSON response payload into a normalized `ModelCompletionResponse`.
    pub fn parse_chat_completion_json(
        provider_name: &str,
        body: &str,
    ) -> Result<ModelCompletionResponse, ProviderError> {
        let val: serde_json::Value =
            serde_json::from_str(body).map_err(|e| ProviderError::MalformedResponse {
                provider: provider_name.to_string(),
                reason: format!("Invalid JSON: {e}"),
            })?;

        let choice = val
            .get("choices")
            .and_then(|c| c.as_array())
            .and_then(|arr| arr.first())
            .ok_or_else(|| ProviderError::MalformedResponse {
                provider: provider_name.to_string(),
                reason: "Missing 'choices[0]' array entry".to_string(),
            })?;

        let message = choice
            .get("message")
            .ok_or_else(|| ProviderError::MalformedResponse {
                provider: provider_name.to_string(),
                reason: "Missing 'choices[0].message' object".to_string(),
            })?;

        let text = message
            .get("content")
            .and_then(|c| c.as_str())
            .unwrap_or("")
            .to_string();

        let mut tool_calls = Vec::new();
        if let Some(raw_calls) = message.get("tool_calls").and_then(|tc| tc.as_array()) {
            for call in raw_calls {
                let id = call
                    .get("id")
                    .and_then(|v| v.as_str())
                    .unwrap_or("call_0")
                    .to_string();
                if let Some(func) = call.get("function") {
                    let tool_name = func
                        .get("name")
                        .and_then(|v| v.as_str())
                        .ok_or_else(|| ProviderError::MalformedResponse {
                            provider: provider_name.to_string(),
                            reason: "Tool call missing 'function.name'".to_string(),
                        })?
                        .to_string();
                    let arguments_json = func
                        .get("arguments")
                        .and_then(|v| v.as_str())
                        .unwrap_or("{}")
                        .to_string();
                    tool_calls.push(ProposedToolCall {
                        id,
                        tool_name,
                        arguments_json,
                    });
                }
            }
        }

        let usage = if let Some(u) = val.get("usage") {
            TokenUsage {
                input_tokens: u.get("prompt_tokens").and_then(|v| v.as_i64()).unwrap_or(0),
                output_tokens: u
                    .get("completion_tokens")
                    .and_then(|v| v.as_i64())
                    .unwrap_or(0),
                cached_tokens: 0,
            }
        } else {
            TokenUsage::default()
        };

        Ok(ModelCompletionResponse {
            text,
            tool_calls,
            usage,
        })
    }
}

#[async_trait]
impl ModelProviderAdapter for OpenAiCompatibleAdapter {
    fn provider_name(&self) -> &str {
        &self.provider_name
    }

    async fn complete(
        &self,
        req: &ModelCompletionRequest,
    ) -> Result<ModelCompletionResponse, ProviderError> {
        let secret: Option<SecretString> = match &self.secret_ref {
            Some(ref_key) => Some(self.credential_store.get_secret(ref_key)?),
            None => None,
        };

        let endpoint = format!("{}/chat/completions", self.base_url);
        let mut payload = serde_json::json!({
            "model": req.model_key,
            "messages": req.messages,
        });

        if let Some(max_tokens) = req.max_output_tokens {
            payload["max_tokens"] = serde_json::json!(max_tokens);
        }

        if !req.tools.is_empty() {
            let formatted_tools: Vec<serde_json::Value> = req
                .tools
                .iter()
                .map(|t| {
                    serde_json::json!({
                        "type": "function",
                        "function": {
                            "name": t.name,
                            "description": t.description,
                            "parameters": t.input_schema,
                        }
                    })
                })
                .collect();
            payload["tools"] = serde_json::json!(formatted_tools);
        }

        let mut builder = self.client.post(&endpoint).json(&payload);
        if let Some(s) = &secret {
            builder = builder.bearer_auth(s.expose_secret());
        }

        let response = builder.send().await.map_err(|e| {
            if e.is_timeout() {
                ProviderError::Timeout {
                    timeout_ms: self.timeout_ms,
                }
            } else {
                ProviderError::Transport {
                    provider: self.provider_name.clone(),
                    message: redact_sensitive_text(&e.to_string(), secret.as_ref()),
                }
            }
        })?;

        let status = response.status();
        let body_text = response.text().await.unwrap_or_default();

        if status.as_u16() == 401 || status.as_u16() == 403 {
            return Err(ProviderError::AuthenticationFailed {
                provider: self.provider_name.clone(),
                message: redact_sensitive_text(&body_text, secret.as_ref()),
            });
        }

        if status.as_u16() == 429 {
            return Err(ProviderError::RateLimited {
                provider: self.provider_name.clone(),
                retry_after_ms: None,
            });
        }

        if !status.is_success() {
            return Err(ProviderError::Transport {
                provider: self.provider_name.clone(),
                message: format!(
                    "HTTP {}: {}",
                    status.as_u16(),
                    redact_sensitive_text(&body_text, secret.as_ref())
                ),
            });
        }

        Self::parse_chat_completion_json(&self.provider_name, &body_text)
    }
}
