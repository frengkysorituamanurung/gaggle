use crate::credentials::{CredentialError, CredentialStore, SecretString, redact_sensitive_text};
use async_trait::async_trait;
use futures::{Stream, StreamExt};
use serde::{Deserialize, Serialize};
use std::{
    collections::BTreeMap,
    net::IpAddr,
    pin::Pin,
    sync::Arc,
    time::{Duration, SystemTime, UNIX_EPOCH},
};
use thiserror::Error;

/// Maximum number of bytes retained for a single buffered SSE frame or NDJSON line.
pub const MAX_PROVIDER_STREAM_LINE_BYTES: usize = 1024 * 1024;
/// Maximum number of bytes accumulated for a normalized streamed response (text plus tool input).
pub const MAX_PROVIDER_STREAM_BODY_BYTES: usize = 8 * 1024 * 1024;
/// Anthropic requires an explicit `anthropic-version` header on every request.
pub const ANTHROPIC_API_VERSION: &str = "2023-06-01";
/// Fallback `max_tokens` when the completion request does not specify an output budget.
pub const ANTHROPIC_DEFAULT_MAX_TOKENS: u32 = 4096;

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

    #[error("OAuth error for provider '{provider}': {message}")]
    OAuth { provider: String, message: String },

    #[error(transparent)]
    Credential(#[from] CredentialError),
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Default)]
pub struct ProviderMessage {
    pub role: String, // "system" | "user" | "assistant" | "tool"
    pub content: String,
    /// Tool call id associated with a `tool` result message (Anthropic `tool_use_id`,
    /// OpenAI `tool_call_id`). `None` for non-tool messages.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub tool_call_id: Option<String>,
    /// Tool calls emitted by an assistant message. Empty for non-assistant messages.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub tool_calls: Vec<ProposedToolCall>,
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

/// Normalized event emitted by native provider streaming implementations.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum ModelStreamEvent {
    TextDelta(String),
    ToolCall(ProposedToolCall),
    Usage(TokenUsage),
    Done(ModelCompletionResponse),
}

/// Boxed, `Send` stream of normalized provider events. Adapters that do not implement native
/// streaming return `None` from [`ModelProviderAdapter::stream`] and callers fall back to
/// [`ModelProviderAdapter::complete`].
pub type ModelCompletionStream =
    Pin<Box<dyn Stream<Item = Result<ModelStreamEvent, ProviderError>> + Send>>;

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

    if !parsed.username().is_empty() || parsed.password().is_some() {
        return Err(ProviderError::InvalidBaseUrl {
            url: raw_url.to_string(),
            reason: "Credentials must not be embedded in the URL".to_string(),
        });
    }

    let host = parsed
        .host_str()
        .ok_or_else(|| ProviderError::InvalidBaseUrl {
            url: raw_url.to_string(),
            reason: "Missing host".to_string(),
        })?;
    let normalized_host = host
        .trim_start_matches('[')
        .trim_end_matches(']')
        .to_ascii_lowercase();

    // Block cloud metadata and link-local destinations. Loopback and private addresses remain
    // allowed because local model servers such as Ollama are a supported configuration.
    if matches!(
        normalized_host.as_str(),
        "metadata.google.internal" | "metadata.azure.internal"
    ) {
        return Err(ProviderError::InvalidBaseUrl {
            url: raw_url.to_string(),
            reason: "Cloud metadata endpoints are not allowed".to_string(),
        });
    }
    if let Ok(ip) = normalized_host.parse::<IpAddr>() {
        let blocked = match ip {
            IpAddr::V4(v4) => v4.is_link_local() || v4.is_multicast() || v4.is_unspecified(),
            IpAddr::V6(v6) => {
                v6.is_unicast_link_local() || v6.is_multicast() || v6.is_unspecified()
            }
        };
        if blocked {
            return Err(ProviderError::InvalidBaseUrl {
                url: raw_url.to_string(),
                reason: "Link-local, multicast, and unspecified addresses are not allowed"
                    .to_string(),
            });
        }
    }

    Ok(trimmed.to_string())
}

/// Maps a non-success provider HTTP response onto a structured [`ProviderError`].
///
/// Response bodies are redacted against the known secret and truncated to keep diagnostics
/// actionable without leaking credentials or unbounded upstream payloads.
pub fn map_provider_http_error(
    provider: &str,
    status: u16,
    body: &str,
    secret: Option<&SecretString>,
    retry_after_ms: Option<u64>,
) -> ProviderError {
    let redacted = redact_sensitive_text(body.trim(), secret);
    let safe_body: String = redacted.chars().take(2048).collect();

    match status {
        401 | 403 => ProviderError::AuthenticationFailed {
            provider: provider.to_string(),
            message: if safe_body.is_empty() {
                format!("HTTP {status}")
            } else {
                safe_body
            },
        },
        429 => ProviderError::RateLimited {
            provider: provider.to_string(),
            retry_after_ms,
        },
        _ => ProviderError::Transport {
            provider: provider.to_string(),
            message: if safe_body.is_empty() {
                format!("HTTP {status}")
            } else {
                format!("HTTP {status}: {safe_body}")
            },
        },
    }
}

/// Parses a `Retry-After` header expressed in whole seconds into milliseconds.
pub fn retry_after_ms(headers: &reqwest::header::HeaderMap) -> Option<u64> {
    let value = headers.get(reqwest::header::RETRY_AFTER)?.to_str().ok()?;
    value
        .trim()
        .parse::<u64>()
        .ok()
        .map(|seconds| seconds.saturating_mul(1000))
}

/// Returns the current time as Unix epoch milliseconds.
pub fn now_unix_ms() -> i64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|elapsed| elapsed.as_millis() as i64)
        .unwrap_or(0)
}

#[async_trait]
pub trait ModelProviderAdapter: Send + Sync {
    fn provider_name(&self) -> &str;
    async fn complete(
        &self,
        req: &ModelCompletionRequest,
    ) -> Result<ModelCompletionResponse, ProviderError>;

    /// Optional native streaming. The default `None` means the runtime falls back to
    /// [`ModelProviderAdapter::complete`].
    fn stream(&self, _req: ModelCompletionRequest) -> Option<ModelCompletionStream> {
        None
    }
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
#[derive(Clone)]
pub struct OpenAiCompatibleAdapter {
    provider_name: String,
    base_url: String,
    secret_ref: Option<String>,
    auth_header: Option<String>,
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
        Self::new_with_auth_header(
            provider_name,
            base_url,
            secret_ref,
            credential_store,
            timeout_ms,
            None,
        )
    }

    pub fn new_with_auth_header(
        provider_name: impl Into<String>,
        base_url: &str,
        secret_ref: Option<String>,
        credential_store: Arc<dyn CredentialStore>,
        timeout_ms: u64,
        auth_header: Option<String>,
    ) -> Result<Self, ProviderError> {
        let provider_name = provider_name.into();
        let validated_url = validate_provider_base_url(base_url)?;
        let client = reqwest::Client::builder()
            .timeout(Duration::from_millis(timeout_ms))
            .build()
            .map_err(|e| ProviderError::Transport {
                provider: provider_name.clone(),
                message: e.to_string(),
            })?;

        Ok(Self {
            provider_name,
            base_url: validated_url,
            secret_ref,
            auth_header,
            credential_store,
            client,
            timeout_ms,
        })
    }

    fn completion_endpoint(&self) -> Result<String, ProviderError> {
        let mut url =
            reqwest::Url::parse(&self.base_url).map_err(|error| ProviderError::InvalidBaseUrl {
                url: self.base_url.clone(),
                reason: error.to_string(),
            })?;
        let mut segments = url
            .path_segments_mut()
            .map_err(|_| ProviderError::InvalidBaseUrl {
                url: self.base_url.clone(),
                reason: "Base URL cannot accept a chat-completions path".to_string(),
            })?;
        segments.pop_if_empty().push("chat").push("completions");
        drop(segments);
        Ok(url.to_string())
    }

    fn authorize(
        &self,
        request: reqwest::RequestBuilder,
        secret: Option<&SecretString>,
    ) -> reqwest::RequestBuilder {
        let Some(secret) = secret else {
            return request;
        };
        match self.auth_header.as_deref() {
            Some(header) => request.header(header, secret.expose_secret()),
            None => request.bearer_auth(secret.expose_secret()),
        }
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

        let endpoint = self.completion_endpoint()?;
        let payload = adapter_support::build_openai_chat_payload(req, false);

        let builder = self.authorize(self.client.post(&endpoint).json(&payload), secret.as_ref());

        let response = builder.send().await.map_err(|e| {
            adapter_support::map_reqwest_send_error(
                &self.provider_name,
                self.timeout_ms,
                &e,
                secret.as_ref(),
            )
        })?;

        let status = response.status();
        let retry_after = retry_after_ms(response.headers());
        let body_text = response.text().await.unwrap_or_default();

        if !status.is_success() {
            return Err(map_provider_http_error(
                &self.provider_name,
                status.as_u16(),
                &body_text,
                secret.as_ref(),
                retry_after,
            ));
        }

        Self::parse_chat_completion_json(&self.provider_name, &body_text)
    }

    fn stream(&self, req: ModelCompletionRequest) -> Option<ModelCompletionStream> {
        let adapter = self.clone();

        Some(Box::pin(async_stream::try_stream! {
            let secret: Option<SecretString> = match &adapter.secret_ref {
                Some(ref_key) => Some(adapter.credential_store.get_secret(ref_key)?),
                None => None,
            };

            let endpoint = adapter.completion_endpoint()?;
            let payload = adapter_support::build_openai_chat_payload(&req, true);

            let builder = adapter.authorize(adapter.client.post(&endpoint).json(&payload), secret.as_ref());

            let response = builder.send().await.map_err(|e| {
                adapter_support::map_reqwest_send_error(
                    &adapter.provider_name,
                    adapter.timeout_ms,
                    &e,
                    secret.as_ref(),
                )
            })?;

            let response = adapter_support::ensure_success_response(
                &adapter.provider_name,
                response,
                secret.as_ref(),
            )
            .await?;

            let provider = adapter.provider_name.clone();
            let mut byte_stream = response.bytes_stream();
            let mut lines = adapter_support::LineAccumulator::new(&provider);
            let mut frames = adapter_support::SseFrameAccumulator::new(&provider);
            let mut state = adapter_support::OpenAiSseState::new();
            let mut finished = false;

            'stream: while let Some(chunk) = byte_stream.next().await {
                let chunk = chunk.map_err(|e| ProviderError::Transport {
                    provider: provider.clone(),
                    message: redact_sensitive_text(&e.to_string(), secret.as_ref()),
                })?;

                for line in lines.push(&chunk)? {
                    let Some(frame) = frames.push_line(&line)? else {
                        continue;
                    };
                    match state.process_data(&provider, &frame.data)? {
                        adapter_support::OpenAiChunkOutcome::Events(events) => {
                            for event in events {
                                yield event;
                            }
                        }
                        adapter_support::OpenAiChunkOutcome::Done(response) => {
                            yield ModelStreamEvent::Done(response);
                            finished = true;
                            break 'stream;
                        }
                    }
                }
            }

            if !finished {
                let mut trailing_frames = Vec::new();
                let trailing_frame = match lines.finish()? {
                    Some(line) => frames.push_line(&line)?,
                    None => None,
                };
                if let Some(frame) = trailing_frame {
                    trailing_frames.push(frame);
                }
                if let Some(frame) = frames.finish() {
                    trailing_frames.push(frame);
                }

                for frame in trailing_frames {
                    match state.process_data(&provider, &frame.data)? {
                        adapter_support::OpenAiChunkOutcome::Events(events) => {
                            for event in events {
                                yield event;
                            }
                        }
                        adapter_support::OpenAiChunkOutcome::Done(response) => {
                            yield ModelStreamEvent::Done(response);
                            finished = true;
                            break;
                        }
                    }
                }
            }

            if !finished {
                yield ModelStreamEvent::Done(state.snapshot(&provider)?);
            }
        }))
    }
}

/// Shared streaming and request-building internals used by the provider adapters in this crate.
///
/// These items are public so that sibling modules (for example the OAuth-backed ChatGPT adapter)
/// can reuse the same normalization logic. They are implementation details and not part of the
/// stable adapter surface.
#[doc(hidden)]
pub mod adapter_support {
    use super::*;

    /// Formats normalized tool definitions using the OpenAI `function` schema.
    fn format_openai_tools(tools: &[ToolDefinition]) -> Vec<serde_json::Value> {
        tools
            .iter()
            .map(|tool| {
                serde_json::json!({
                    "type": "function",
                    "function": {
                        "name": tool.name,
                        "description": tool.description,
                        "parameters": tool.input_schema,
                    }
                })
            })
            .collect()
    }

    /// Builds an OpenAI-compatible chat completion payload.
    pub fn build_openai_chat_payload(
        req: &ModelCompletionRequest,
        stream: bool,
    ) -> serde_json::Value {
        let mut payload = serde_json::json!({
            "model": req.model_key,
            "messages": req.messages,
        });

        if stream {
            payload["stream"] = serde_json::json!(true);
        }

        if let Some(max_tokens) = req.max_output_tokens {
            payload["max_tokens"] = serde_json::json!(max_tokens);
        }

        if !req.tools.is_empty() {
            payload["tools"] = serde_json::json!(format_openai_tools(&req.tools));
        }

        payload
    }

    /// Maps a `reqwest` send failure, distinguishing timeouts and redacting the secret.
    pub fn map_reqwest_send_error(
        provider: &str,
        timeout_ms: u64,
        error: &reqwest::Error,
        secret: Option<&SecretString>,
    ) -> ProviderError {
        if error.is_timeout() {
            ProviderError::Timeout { timeout_ms }
        } else {
            ProviderError::Transport {
                provider: provider.to_string(),
                message: redact_sensitive_text(&error.to_string(), secret),
            }
        }
    }

    /// Consumes a response, converting non-success statuses into structured errors.
    pub async fn ensure_success_response(
        provider: &str,
        response: reqwest::Response,
        secret: Option<&SecretString>,
    ) -> Result<reqwest::Response, ProviderError> {
        let status = response.status();
        if status.is_success() {
            return Ok(response);
        }
        let retry = retry_after_ms(response.headers());
        let body = response.text().await.unwrap_or_default();
        Err(map_provider_http_error(
            provider,
            status.as_u16(),
            &body,
            secret,
            retry,
        ))
    }

    /// Converts an invalid-UTF-8 byte chunk into a structured malformed-response error.
    pub fn invalid_utf8_error(provider: &str, error: std::str::Utf8Error) -> ProviderError {
        ProviderError::MalformedResponse {
            provider: provider.to_string(),
            reason: format!("Stream contained invalid UTF-8: {error}"),
        }
    }

    /// Incremental newline-delimited line reader with a hard cap on buffered partial lines.
    pub struct LineAccumulator {
        provider: String,
        buffer: Vec<u8>,
    }

    impl LineAccumulator {
        pub fn new(provider: &str) -> Self {
            Self {
                provider: provider.to_string(),
                buffer: Vec::new(),
            }
        }

        /// Appends a byte chunk and returns every complete line it produced (without terminators).
        pub fn push(&mut self, chunk: &[u8]) -> Result<Vec<String>, ProviderError> {
            self.buffer.extend_from_slice(chunk);

            let mut lines = Vec::new();
            let mut start = 0usize;
            for index in 0..self.buffer.len() {
                if self.buffer[index] != b'\n' {
                    continue;
                }
                let mut end = index;
                if end > start && self.buffer[end - 1] == b'\r' {
                    end -= 1;
                }
                let line = std::str::from_utf8(&self.buffer[start..end])
                    .map_err(|e| invalid_utf8_error(&self.provider, e))?;
                lines.push(line.to_string());
                start = index + 1;
            }

            if start > 0 {
                self.buffer.drain(..start);
            }

            if self.buffer.len() > MAX_PROVIDER_STREAM_LINE_BYTES {
                return Err(ProviderError::MalformedResponse {
                    provider: self.provider.clone(),
                    reason: format!(
                        "Stream line exceeded the {} byte buffer limit",
                        MAX_PROVIDER_STREAM_LINE_BYTES
                    ),
                });
            }

            Ok(lines)
        }

        /// Returns the remaining partial line, if any, when the stream ends.
        pub fn finish(&mut self) -> Result<Option<String>, ProviderError> {
            if self.buffer.is_empty() {
                return Ok(None);
            }
            let mut bytes = std::mem::take(&mut self.buffer);
            if bytes.last() == Some(&b'\r') {
                bytes.pop();
            }
            if bytes.is_empty() {
                return Ok(None);
            }
            let line =
                std::str::from_utf8(&bytes).map_err(|e| invalid_utf8_error(&self.provider, e))?;
            Ok(Some(line.to_string()))
        }
    }

    /// A single parsed server-sent event frame.
    #[derive(Debug, Clone, PartialEq, Eq)]
    pub struct SseFrame {
        pub event: Option<String>,
        pub data: String,
    }

    /// Incremental SSE frame accumulator handling `event:`/`data:` fields, CRLF, comments,
    /// and frames split across network chunks.
    pub struct SseFrameAccumulator {
        provider: String,
        event: Option<String>,
        data: String,
    }

    impl SseFrameAccumulator {
        pub fn new(provider: &str) -> Self {
            Self {
                provider: provider.to_string(),
                event: None,
                data: String::new(),
            }
        }

        /// Feeds one line. Returns a frame when a blank line terminates it.
        pub fn push_line(&mut self, line: &str) -> Result<Option<SseFrame>, ProviderError> {
            if line.is_empty() {
                let event = self.event.take();
                if self.data.is_empty() {
                    return Ok(None);
                }
                return Ok(Some(SseFrame {
                    event,
                    data: std::mem::take(&mut self.data),
                }));
            }

            if let Some(rest) = line.strip_prefix("event:") {
                self.event = Some(rest.trim().to_string());
            } else if let Some(rest) = line.strip_prefix("data:") {
                let value = rest.strip_prefix(' ').unwrap_or(rest);
                let additional = value.len() + usize::from(!self.data.is_empty());
                if self.data.len().saturating_add(additional) > MAX_PROVIDER_STREAM_LINE_BYTES {
                    return Err(ProviderError::MalformedResponse {
                        provider: self.provider.clone(),
                        reason: format!(
                            "SSE frame exceeded the {} byte buffer limit",
                            MAX_PROVIDER_STREAM_LINE_BYTES
                        ),
                    });
                }
                if !self.data.is_empty() {
                    self.data.push('\n');
                }
                self.data.push_str(value);
            }

            Ok(None)
        }

        /// Emits a trailing frame when a provider omits the terminating blank line.
        pub fn finish(&mut self) -> Option<SseFrame> {
            if self.data.is_empty() {
                self.event = None;
                return None;
            }
            Some(SseFrame {
                event: self.event.take(),
                data: std::mem::take(&mut self.data),
            })
        }
    }

    /// Outcome of feeding one OpenAI-compatible SSE payload.
    pub enum OpenAiChunkOutcome {
        Events(Vec<ModelStreamEvent>),
        Done(ModelCompletionResponse),
    }

    #[derive(Default)]
    struct OpenAiToolAccum {
        id: String,
        name: String,
        arguments_json: String,
    }

    /// Normalizes OpenAI-compatible chat completion streaming chunks.
    #[derive(Default)]
    pub struct OpenAiSseState {
        text: String,
        usage: TokenUsage,
        tools: BTreeMap<u64, OpenAiToolAccum>,
        accumulated_bytes: usize,
    }

    impl OpenAiSseState {
        pub fn new() -> Self {
            Self::default()
        }

        fn reserve_bytes(&mut self, provider: &str, bytes: usize) -> Result<(), ProviderError> {
            self.accumulated_bytes = self.accumulated_bytes.saturating_add(bytes);
            if self.accumulated_bytes > MAX_PROVIDER_STREAM_BODY_BYTES {
                return Err(ProviderError::MalformedResponse {
                    provider: provider.to_string(),
                    reason: format!(
                        "Streamed response exceeded the {} byte accumulation limit",
                        MAX_PROVIDER_STREAM_BODY_BYTES
                    ),
                });
            }
            Ok(())
        }

        /// Normalizes one `data:` payload, including the terminal `[DONE]` marker.
        pub fn process_data(
            &mut self,
            provider: &str,
            data: &str,
        ) -> Result<OpenAiChunkOutcome, ProviderError> {
            let trimmed = data.trim();
            if trimmed.is_empty() {
                return Ok(OpenAiChunkOutcome::Events(Vec::new()));
            }
            if trimmed == "[DONE]" {
                return Ok(OpenAiChunkOutcome::Done(self.snapshot(provider)?));
            }

            let value: serde_json::Value =
                serde_json::from_str(trimmed).map_err(|e| ProviderError::MalformedResponse {
                    provider: provider.to_string(),
                    reason: format!("Invalid JSON stream chunk: {e}"),
                })?;

            if let Some(error) = value.get("error") {
                let message = error
                    .get("message")
                    .and_then(|v| v.as_str())
                    .unwrap_or("provider reported a streaming error");
                return Err(ProviderError::MalformedResponse {
                    provider: provider.to_string(),
                    reason: format!("Stream error: {message}"),
                });
            }

            let mut events = Vec::new();

            if let Some(usage) = value.get("usage") {
                self.usage.input_tokens = usage
                    .get("prompt_tokens")
                    .and_then(|v| v.as_i64())
                    .unwrap_or(self.usage.input_tokens);
                self.usage.output_tokens = usage
                    .get("completion_tokens")
                    .and_then(|v| v.as_i64())
                    .unwrap_or(self.usage.output_tokens);
                self.usage.cached_tokens = usage
                    .get("prompt_tokens_details")
                    .and_then(|details| details.get("cached_tokens"))
                    .and_then(|v| v.as_i64())
                    .unwrap_or(self.usage.cached_tokens);
                events.push(ModelStreamEvent::Usage(self.usage.clone()));
            }

            if let Some(delta) = value
                .get("choices")
                .and_then(|choices| choices.as_array())
                .and_then(|choices| choices.first())
                .and_then(|choice| choice.get("delta"))
            {
                if let Some(content) = delta.get("content").and_then(|v| v.as_str())
                    && !content.is_empty()
                {
                    self.reserve_bytes(provider, content.len())?;
                    self.text.push_str(content);
                    events.push(ModelStreamEvent::TextDelta(content.to_string()));
                }

                if let Some(calls) = delta.get("tool_calls").and_then(|v| v.as_array()) {
                    for call in calls {
                        let index = call.get("index").and_then(|v| v.as_u64()).unwrap_or(0);
                        let id = call.get("id").and_then(|v| v.as_str());
                        let mut name = "";
                        let mut fragment = String::new();

                        if let Some(function) = call.get("function") {
                            if let Some(value) = function.get("name").and_then(|v| v.as_str()) {
                                name = value;
                            }
                            if let Some(arguments) = function.get("arguments") {
                                fragment = match arguments {
                                    serde_json::Value::String(text) => text.clone(),
                                    serde_json::Value::Null => String::new(),
                                    other => other.to_string(),
                                };
                            }
                        }

                        let added = name.len().saturating_add(fragment.len());
                        if added > 0 {
                            self.reserve_bytes(provider, added)?;
                        }

                        let entry = self.tools.entry(index).or_default();
                        if let Some(id) = id
                            && !id.is_empty()
                            && entry.id.is_empty()
                        {
                            entry.id = id.to_string();
                        }
                        if !name.is_empty() {
                            entry.name.push_str(name);
                        }
                        if !fragment.is_empty() {
                            entry.arguments_json.push_str(&fragment);
                        }
                    }
                }
            }

            Ok(OpenAiChunkOutcome::Events(events))
        }

        /// Produces the accumulated completion used for the terminal `Done` event.
        pub fn snapshot(&self, provider: &str) -> Result<ModelCompletionResponse, ProviderError> {
            let mut tool_calls = Vec::with_capacity(self.tools.len());
            for (index, entry) in &self.tools {
                if entry.name.is_empty() {
                    return Err(ProviderError::MalformedResponse {
                        provider: provider.to_string(),
                        reason: "Streamed tool call is missing a function name".to_string(),
                    });
                }
                let arguments_json = if entry.arguments_json.trim().is_empty() {
                    "{}".to_string()
                } else {
                    entry.arguments_json.clone()
                };
                serde_json::from_str::<serde_json::Value>(&arguments_json).map_err(|e| {
                    ProviderError::MalformedResponse {
                        provider: provider.to_string(),
                        reason: format!("Streamed tool call arguments are not valid JSON: {e}"),
                    }
                })?;
                tool_calls.push(ProposedToolCall {
                    id: if entry.id.is_empty() {
                        format!("call_{index}")
                    } else {
                        entry.id.clone()
                    },
                    tool_name: entry.name.clone(),
                    arguments_json,
                });
            }

            Ok(ModelCompletionResponse {
                text: self.text.clone(),
                tool_calls,
                usage: self.usage.clone(),
            })
        }
    }
}

/// Anthropic Messages API adapter (`POST {base_url}/messages`).
pub struct AnthropicAdapter {
    provider_name: String,
    base_url: String,
    secret_ref: Option<String>,
    credential_store: Arc<dyn CredentialStore>,
    client: reqwest::Client,
    timeout_ms: u64,
}

impl AnthropicAdapter {
    pub fn new(
        provider_name: impl Into<String>,
        base_url: &str,
        secret_ref: Option<String>,
        credential_store: Arc<dyn CredentialStore>,
        timeout_ms: u64,
    ) -> Result<Self, ProviderError> {
        let provider_name = provider_name.into();
        let validated_url = validate_provider_base_url(base_url)?;
        let client = reqwest::Client::builder()
            .timeout(Duration::from_millis(timeout_ms))
            .build()
            .map_err(|e| ProviderError::Transport {
                provider: provider_name.clone(),
                message: e.to_string(),
            })?;

        Ok(Self {
            provider_name,
            base_url: validated_url,
            secret_ref,
            credential_store,
            client,
            timeout_ms,
        })
    }

    fn require_secret(&self) -> Result<SecretString, ProviderError> {
        match &self.secret_ref {
            Some(secret_ref) => Ok(self.credential_store.get_secret(secret_ref)?),
            None => Err(ProviderError::Credential(CredentialError::NotFound(
                "anthropic_api_key".to_string(),
            ))),
        }
    }

    /// Parses a non-streaming Anthropic Messages response into a normalized completion.
    pub fn parse_message_json(
        provider_name: &str,
        body: &str,
    ) -> Result<ModelCompletionResponse, ProviderError> {
        let value: serde_json::Value =
            serde_json::from_str(body).map_err(|e| ProviderError::MalformedResponse {
                provider: provider_name.to_string(),
                reason: format!("Invalid JSON: {e}"),
            })?;

        let blocks = value
            .get("content")
            .and_then(|content| content.as_array())
            .ok_or_else(|| ProviderError::MalformedResponse {
                provider: provider_name.to_string(),
                reason: "Missing 'content' array".to_string(),
            })?;

        let mut text = String::new();
        let mut tool_calls = Vec::new();

        for block in blocks {
            match block.get("type").and_then(|v| v.as_str()) {
                Some("text") => {
                    if let Some(chunk) = block.get("text").and_then(|v| v.as_str()) {
                        text.push_str(chunk);
                    }
                }
                Some("tool_use") => {
                    let id = block
                        .get("id")
                        .and_then(|v| v.as_str())
                        .unwrap_or("tool_0")
                        .to_string();
                    let tool_name = block
                        .get("name")
                        .and_then(|v| v.as_str())
                        .ok_or_else(|| ProviderError::MalformedResponse {
                            provider: provider_name.to_string(),
                            reason: "Anthropic tool_use block is missing 'name'".to_string(),
                        })?
                        .to_string();
                    let arguments_json = block
                        .get("input")
                        .map(|input| {
                            serde_json::to_string(input).unwrap_or_else(|_| "{}".to_string())
                        })
                        .unwrap_or_else(|| "{}".to_string());
                    tool_calls.push(ProposedToolCall {
                        id,
                        tool_name,
                        arguments_json,
                    });
                }
                _ => {}
            }
        }

        let usage = value
            .get("usage")
            .map(|usage| TokenUsage {
                input_tokens: usage
                    .get("input_tokens")
                    .and_then(|v| v.as_i64())
                    .unwrap_or(0),
                output_tokens: usage
                    .get("output_tokens")
                    .and_then(|v| v.as_i64())
                    .unwrap_or(0),
                cached_tokens: usage
                    .get("cache_read_input_tokens")
                    .and_then(|v| v.as_i64())
                    .unwrap_or(0),
            })
            .unwrap_or_default();

        Ok(ModelCompletionResponse {
            text,
            tool_calls,
            usage,
        })
    }
}

fn build_anthropic_payload(req: &ModelCompletionRequest, stream: bool) -> serde_json::Value {
    let mut system_parts = Vec::new();
    let mut messages = Vec::new();

    // Anthropic pairs tool results with assistant `tool_use` blocks. A run of consecutive
    // `tool` messages is collapsed into a single `user` message whose content is a list of
    // `tool_result` blocks; assistant messages carrying `tool_calls` are emitted with
    // `tool_use` blocks so the pairing survives a round trip.
    let mut pending_tool_results: Vec<serde_json::Value> = Vec::new();

    let flush_tool_results = |messages: &mut Vec<serde_json::Value>,
                              pending: &mut Vec<serde_json::Value>| {
        if !pending.is_empty() {
            messages.push(serde_json::json!({
                "role": "user",
                "content": std::mem::take(pending),
            }));
        }
    };

    for message in &req.messages {
        match message.role.as_str() {
            "system" => system_parts.push(message.content.clone()),
            "tool" => {
                let tool_use_id = message
                    .tool_call_id
                    .clone()
                    .unwrap_or_else(|| "tool_0".to_string());
                pending_tool_results.push(serde_json::json!({
                    "type": "tool_result",
                    "tool_use_id": tool_use_id,
                    "content": message.content,
                }));
            }
            "assistant" if !message.tool_calls.is_empty() => {
                flush_tool_results(&mut messages, &mut pending_tool_results);
                let mut blocks: Vec<serde_json::Value> = Vec::new();
                if !message.content.trim().is_empty() {
                    blocks.push(serde_json::json!({
                        "type": "text",
                        "text": message.content,
                    }));
                }
                for call in &message.tool_calls {
                    let input = serde_json::from_str::<serde_json::Value>(&call.arguments_json)
                        .unwrap_or_else(|_| serde_json::json!({}));
                    blocks.push(serde_json::json!({
                        "type": "tool_use",
                        "id": call.id,
                        "name": call.tool_name,
                        "input": input,
                    }));
                }
                if blocks.is_empty() {
                    blocks.push(serde_json::json!({
                        "type": "text",
                        "text": "",
                    }));
                }
                messages.push(serde_json::json!({
                    "role": "assistant",
                    "content": blocks,
                }));
            }
            role => {
                flush_tool_results(&mut messages, &mut pending_tool_results);
                messages.push(serde_json::json!({
                    "role": role,
                    "content": message.content,
                }));
            }
        }
    }
    flush_tool_results(&mut messages, &mut pending_tool_results);

    let mut payload = serde_json::json!({
        "model": req.model_key,
        "max_tokens": req.max_output_tokens.unwrap_or(ANTHROPIC_DEFAULT_MAX_TOKENS),
        "messages": messages,
        "stream": stream,
    });

    if !system_parts.is_empty() {
        payload["system"] = serde_json::json!(system_parts.join("\n\n"));
    }

    if !req.tools.is_empty() {
        let tools: Vec<serde_json::Value> = req
            .tools
            .iter()
            .map(|tool| {
                serde_json::json!({
                    "name": tool.name,
                    "description": tool.description,
                    "input_schema": tool.input_schema,
                })
            })
            .collect();
        payload["tools"] = serde_json::json!(tools);
    }

    payload
}

#[derive(Default)]
struct PendingAnthropicTool {
    id: String,
    name: String,
    arguments_json: String,
}

#[derive(Default)]
struct AnthropicStreamState {
    text: String,
    tool_calls: Vec<ProposedToolCall>,
    pending_tools: BTreeMap<u64, PendingAnthropicTool>,
    usage: TokenUsage,
    accumulated_bytes: usize,
}

impl AnthropicStreamState {
    fn reserve_bytes(&mut self, provider: &str, bytes: usize) -> Result<(), ProviderError> {
        self.accumulated_bytes = self.accumulated_bytes.saturating_add(bytes);
        if self.accumulated_bytes > MAX_PROVIDER_STREAM_BODY_BYTES {
            return Err(ProviderError::MalformedResponse {
                provider: provider.to_string(),
                reason: format!(
                    "Streamed response exceeded the {} byte accumulation limit",
                    MAX_PROVIDER_STREAM_BODY_BYTES
                ),
            });
        }
        Ok(())
    }

    fn malformed(provider: &str, reason: impl Into<String>) -> ProviderError {
        ProviderError::MalformedResponse {
            provider: provider.to_string(),
            reason: reason.into(),
        }
    }

    /// Handles one SSE frame and returns the normalized events it produces.
    fn process_frame(
        &mut self,
        provider: &str,
        frame: &adapter_support::SseFrame,
    ) -> Result<Vec<ModelStreamEvent>, ProviderError> {
        let value: serde_json::Value = serde_json::from_str(&frame.data)
            .map_err(|e| Self::malformed(provider, format!("Invalid JSON stream event: {e}")))?;

        let event_type = value
            .get("type")
            .and_then(|v| v.as_str())
            .or(frame.event.as_deref())
            .unwrap_or("");

        let mut events = Vec::new();

        match event_type {
            "message_start" => {
                if let Some(usage) = value
                    .get("message")
                    .and_then(|message| message.get("usage"))
                {
                    self.usage.input_tokens = usage
                        .get("input_tokens")
                        .and_then(|v| v.as_i64())
                        .unwrap_or(self.usage.input_tokens);
                    self.usage.output_tokens = usage
                        .get("output_tokens")
                        .and_then(|v| v.as_i64())
                        .unwrap_or(self.usage.output_tokens);
                    self.usage.cached_tokens = usage
                        .get("cache_read_input_tokens")
                        .and_then(|v| v.as_i64())
                        .unwrap_or(self.usage.cached_tokens);
                }
            }
            "content_block_start" => {
                let index = value.get("index").and_then(|v| v.as_u64()).unwrap_or(0);
                let block = value.get("content_block").ok_or_else(|| {
                    Self::malformed(provider, "content_block_start is missing 'content_block'")
                })?;
                match block.get("type").and_then(|v| v.as_str()) {
                    Some("tool_use") => {
                        let id = block
                            .get("id")
                            .and_then(|v| v.as_str())
                            .unwrap_or("tool_0")
                            .to_string();
                        let name = block
                            .get("name")
                            .and_then(|v| v.as_str())
                            .ok_or_else(|| {
                                Self::malformed(
                                    provider,
                                    "Anthropic streamed tool_use block is missing 'name'",
                                )
                            })?
                            .to_string();
                        let initial_arguments = block
                            .get("input")
                            .filter(|input| {
                                input
                                    .as_object()
                                    .map(|map| !map.is_empty())
                                    .unwrap_or(false)
                            })
                            .and_then(|input| serde_json::to_string(input).ok())
                            .unwrap_or_default();
                        self.pending_tools.insert(
                            index,
                            PendingAnthropicTool {
                                id,
                                name,
                                arguments_json: initial_arguments,
                            },
                        );
                    }
                    Some("text") => {
                        if let Some(text) = block.get("text").and_then(|v| v.as_str())
                            && !text.is_empty()
                        {
                            self.reserve_bytes(provider, text.len())?;
                            self.text.push_str(text);
                            events.push(ModelStreamEvent::TextDelta(text.to_string()));
                        }
                    }
                    _ => {}
                }
            }
            "content_block_delta" => {
                let index = value.get("index").and_then(|v| v.as_u64()).unwrap_or(0);
                let delta = value.get("delta").ok_or_else(|| {
                    Self::malformed(provider, "content_block_delta is missing 'delta'")
                })?;

                match delta.get("type").and_then(|v| v.as_str()) {
                    Some("text_delta") => {
                        if let Some(text) = delta.get("text").and_then(|v| v.as_str())
                            && !text.is_empty()
                        {
                            self.reserve_bytes(provider, text.len())?;
                            self.text.push_str(text);
                            events.push(ModelStreamEvent::TextDelta(text.to_string()));
                        }
                    }
                    Some("input_json_delta") => {
                        if let Some(partial) = delta.get("partial_json").and_then(|v| v.as_str()) {
                            self.reserve_bytes(provider, partial.len())?;
                            let pending = self.pending_tools.get_mut(&index).ok_or_else(|| {
                                Self::malformed(
                                    provider,
                                    "Received tool input before content_block_start",
                                )
                            })?;
                            pending.arguments_json.push_str(partial);
                        }
                    }
                    _ => {}
                }
            }
            "content_block_stop" => {
                let index = value.get("index").and_then(|v| v.as_u64()).unwrap_or(0);
                if let Some(pending) = self.pending_tools.remove(&index) {
                    let arguments_json = if pending.arguments_json.trim().is_empty() {
                        "{}".to_string()
                    } else {
                        pending.arguments_json
                    };
                    serde_json::from_str::<serde_json::Value>(&arguments_json).map_err(|e| {
                        Self::malformed(
                            provider,
                            format!("Streamed tool call arguments are not valid JSON: {e}"),
                        )
                    })?;
                    let call = ProposedToolCall {
                        id: pending.id,
                        tool_name: pending.name,
                        arguments_json,
                    };
                    self.tool_calls.push(call.clone());
                    events.push(ModelStreamEvent::ToolCall(call));
                }
            }
            "message_delta" => {
                if let Some(usage) = value.get("usage") {
                    self.usage.input_tokens = usage
                        .get("input_tokens")
                        .and_then(|v| v.as_i64())
                        .unwrap_or(self.usage.input_tokens);
                    self.usage.output_tokens = usage
                        .get("output_tokens")
                        .and_then(|v| v.as_i64())
                        .unwrap_or(self.usage.output_tokens);
                    self.usage.cached_tokens = usage
                        .get("cache_read_input_tokens")
                        .and_then(|v| v.as_i64())
                        .unwrap_or(self.usage.cached_tokens);
                    events.push(ModelStreamEvent::Usage(self.usage.clone()));
                }
            }
            "message_stop" => {
                events.push(ModelStreamEvent::Done(ModelCompletionResponse {
                    text: self.text.clone(),
                    tool_calls: self.tool_calls.clone(),
                    usage: self.usage.clone(),
                }));
            }
            "error" => {
                let message = value
                    .get("error")
                    .and_then(|error| error.get("message"))
                    .and_then(|v| v.as_str())
                    .unwrap_or("provider reported a streaming error");
                return Err(Self::malformed(
                    provider,
                    format!("Stream error: {message}"),
                ));
            }
            _ => {}
        }

        Ok(events)
    }
}

#[async_trait]
impl ModelProviderAdapter for AnthropicAdapter {
    fn provider_name(&self) -> &str {
        &self.provider_name
    }

    async fn complete(
        &self,
        req: &ModelCompletionRequest,
    ) -> Result<ModelCompletionResponse, ProviderError> {
        let secret = self.require_secret()?;
        let endpoint = format!("{}/messages", self.base_url);
        let payload = build_anthropic_payload(req, false);

        let response = self
            .client
            .post(&endpoint)
            .header("x-api-key", secret.expose_secret())
            .header("anthropic-version", ANTHROPIC_API_VERSION)
            .header("content-type", "application/json")
            .json(&payload)
            .send()
            .await
            .map_err(|e| {
                adapter_support::map_reqwest_send_error(
                    &self.provider_name,
                    self.timeout_ms,
                    &e,
                    Some(&secret),
                )
            })?;

        let status = response.status();
        let retry_after = retry_after_ms(response.headers());
        let body = response.text().await.unwrap_or_default();

        if !status.is_success() {
            return Err(map_provider_http_error(
                &self.provider_name,
                status.as_u16(),
                &body,
                Some(&secret),
                retry_after,
            ));
        }

        Self::parse_message_json(&self.provider_name, &body)
    }

    fn stream(&self, req: ModelCompletionRequest) -> Option<ModelCompletionStream> {
        let client = self.client.clone();
        let provider = self.provider_name.clone();
        let base_url = self.base_url.clone();
        let secret_ref = self.secret_ref.clone();
        let store = self.credential_store.clone();
        let timeout_ms = self.timeout_ms;

        Some(Box::pin(async_stream::try_stream! {
            let secret = match &secret_ref {
                Some(secret_ref) => Some(store.get_secret(secret_ref)?),
                None => None,
            };
            let secret = secret.ok_or_else(|| {
                ProviderError::Credential(CredentialError::NotFound(
                    "anthropic_api_key".to_string(),
                ))
            })?;

            let endpoint = format!("{base_url}/messages");
            let payload = build_anthropic_payload(&req, true);
            let response = client
                .post(&endpoint)
                .header("x-api-key", secret.expose_secret())
                .header("anthropic-version", ANTHROPIC_API_VERSION)
                .header("content-type", "application/json")
                .json(&payload)
                .send()
                .await
                .map_err(|e| {
                    adapter_support::map_reqwest_send_error(
                        &provider,
                        timeout_ms,
                        &e,
                        Some(&secret),
                    )
                })?;

            let response =
                adapter_support::ensure_success_response(&provider, response, Some(&secret))
                    .await?;

            let mut byte_stream = response.bytes_stream();
            let mut lines = adapter_support::LineAccumulator::new(&provider);
            let mut frames = adapter_support::SseFrameAccumulator::new(&provider);
            let mut state = AnthropicStreamState::default();

            while let Some(chunk) = byte_stream.next().await {
                let chunk = chunk.map_err(|e| ProviderError::Transport {
                    provider: provider.clone(),
                    message: redact_sensitive_text(&e.to_string(), Some(&secret)),
                })?;

                for line in lines.push(&chunk)? {
                    if let Some(frame) = frames.push_line(&line)? {
                        for event in state.process_frame(&provider, &frame)? {
                            yield event;
                        }
                    }
                }
            }

            let trailing_frame = match lines.finish()? {
                Some(line) => frames.push_line(&line)?,
                None => None,
            };
            if let Some(frame) = trailing_frame {
                for event in state.process_frame(&provider, &frame)? {
                    yield event;
                }
            }
            if let Some(frame) = frames.finish() {
                for event in state.process_frame(&provider, &frame)? {
                    yield event;
                }
            }
        }))
    }
}

/// Ollama native chat adapter (`POST {base_url}/api/chat`), with optional NDJSON streaming.
/// Google Gemini API adapter (`POST {base_url}/models/{model}:generateContent`).
pub struct GoogleGeminiAdapter {
    provider_name: String,
    base_url: String,
    secret_ref: Option<String>,
    credential_store: Arc<dyn CredentialStore>,
    client: reqwest::Client,
    timeout_ms: u64,
}

impl GoogleGeminiAdapter {
    pub fn new(
        provider_name: impl Into<String>,
        base_url: &str,
        secret_ref: Option<String>,
        credential_store: Arc<dyn CredentialStore>,
        timeout_ms: u64,
    ) -> Result<Self, ProviderError> {
        let provider_name = provider_name.into();
        let validated_url = validate_provider_base_url(base_url)?;
        let client = reqwest::Client::builder()
            .timeout(Duration::from_millis(timeout_ms))
            .build()
            .map_err(|e| ProviderError::Transport {
                provider: provider_name.clone(),
                message: e.to_string(),
            })?;
        Ok(Self {
            provider_name,
            base_url: validated_url,
            secret_ref,
            credential_store,
            client,
            timeout_ms,
        })
    }
}

#[async_trait]
impl ModelProviderAdapter for GoogleGeminiAdapter {
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

        let endpoint = format!(
            "{}/v1beta/models/{}:generateContent",
            self.base_url.trim_end_matches('/'),
            req.model_key
        );
        let payload = serde_json::json!({
            "contents": req.messages.iter().map(|m| serde_json::json!({
                "role": if m.role == "assistant" { "model" } else { "user" },
                "parts": [{"text": m.content}]
            })).collect::<Vec<_>>(),
            "generationConfig": {
                "maxOutputTokens": req.max_output_tokens.unwrap_or(1024)
            }
        });

        let mut builder = self.client.post(&endpoint).json(&payload);
        if let Some(s) = &secret {
            builder = builder.header("x-goog-api-key", s.expose_secret());
        }

        let response = builder.send().await.map_err(|e| {
            adapter_support::map_reqwest_send_error(
                &self.provider_name,
                self.timeout_ms,
                &e,
                secret.as_ref(),
            )
        })?;

        let status = response.status();
        let body_text = response.text().await.unwrap_or_default();
        if !status.is_success() {
            return Err(map_provider_http_error(
                &self.provider_name,
                status.as_u16(),
                &body_text,
                secret.as_ref(),
                None,
            ));
        }

        let val: serde_json::Value =
            serde_json::from_str(&body_text).map_err(|e| ProviderError::MalformedResponse {
                provider: self.provider_name.clone(),
                reason: format!("Invalid JSON: {e}"),
            })?;

        let text = val
            .get("candidates")
            .and_then(|c| c.as_array())
            .and_then(|arr| arr.first())
            .and_then(|c| c.get("content"))
            .and_then(|c| c.get("parts"))
            .and_then(|p| p.as_array())
            .and_then(|parts| parts.first())
            .and_then(|p| p.get("text"))
            .and_then(|t| t.as_str())
            .unwrap_or("")
            .to_string();

        Ok(ModelCompletionResponse {
            text,
            tool_calls: Vec::new(),
            usage: TokenUsage::default(),
        })
    }

    fn stream(&self, _req: ModelCompletionRequest) -> Option<ModelCompletionStream> {
        None
    }
}

pub struct OllamaAdapter {
    provider_name: String,
    base_url: String,
    client: reqwest::Client,
    timeout_ms: u64,
}

impl OllamaAdapter {
    pub fn new(
        provider_name: impl Into<String>,
        base_url: &str,
        timeout_ms: u64,
    ) -> Result<Self, ProviderError> {
        let provider_name = provider_name.into();
        let validated_url = validate_provider_base_url(base_url)?;
        let client = reqwest::Client::builder()
            .timeout(Duration::from_millis(timeout_ms))
            .build()
            .map_err(|e| ProviderError::Transport {
                provider: provider_name.clone(),
                message: e.to_string(),
            })?;

        Ok(Self {
            provider_name,
            base_url: validated_url,
            client,
            timeout_ms,
        })
    }

    /// Parses a non-streaming Ollama chat response into a normalized completion.
    pub fn parse_chat_json(
        provider_name: &str,
        body: &str,
    ) -> Result<ModelCompletionResponse, ProviderError> {
        let value: serde_json::Value =
            serde_json::from_str(body).map_err(|e| ProviderError::MalformedResponse {
                provider: provider_name.to_string(),
                reason: format!("Invalid JSON: {e}"),
            })?;

        if let Some(error) = value.get("error").and_then(|v| v.as_str()) {
            return Err(ProviderError::MalformedResponse {
                provider: provider_name.to_string(),
                reason: format!("Provider error: {error}"),
            });
        }

        let message = value
            .get("message")
            .ok_or_else(|| ProviderError::MalformedResponse {
                provider: provider_name.to_string(),
                reason: "Missing 'message' object".to_string(),
            })?;

        let text = message
            .get("content")
            .and_then(|v| v.as_str())
            .unwrap_or("")
            .to_string();

        let tool_calls = parse_ollama_tool_calls(provider_name, message)?;
        let usage = parse_ollama_usage(&value);

        Ok(ModelCompletionResponse {
            text,
            tool_calls,
            usage,
        })
    }
}

fn build_ollama_payload(req: &ModelCompletionRequest, stream: bool) -> serde_json::Value {
    let mut payload = serde_json::json!({
        "model": req.model_key,
        "messages": req.messages,
        "stream": stream,
    });

    if let Some(max_tokens) = req.max_output_tokens {
        payload["options"] = serde_json::json!({ "num_predict": max_tokens });
    }

    if !req.tools.is_empty() {
        let tools: Vec<serde_json::Value> = req
            .tools
            .iter()
            .map(|tool| {
                serde_json::json!({
                    "type": "function",
                    "function": {
                        "name": tool.name,
                        "description": tool.description,
                        "parameters": tool.input_schema,
                    }
                })
            })
            .collect();
        payload["tools"] = serde_json::json!(tools);
    }

    payload
}

fn parse_ollama_tool_calls(
    provider_name: &str,
    message: &serde_json::Value,
) -> Result<Vec<ProposedToolCall>, ProviderError> {
    let Some(raw_calls) = message.get("tool_calls").and_then(|v| v.as_array()) else {
        return Ok(Vec::new());
    };

    let mut tool_calls = Vec::with_capacity(raw_calls.len());
    for (index, call) in raw_calls.iter().enumerate() {
        let function = call
            .get("function")
            .ok_or_else(|| ProviderError::MalformedResponse {
                provider: provider_name.to_string(),
                reason: "Ollama tool call is missing 'function'".to_string(),
            })?;
        let tool_name = function
            .get("name")
            .and_then(|v| v.as_str())
            .ok_or_else(|| ProviderError::MalformedResponse {
                provider: provider_name.to_string(),
                reason: "Ollama tool call is missing 'function.name'".to_string(),
            })?
            .to_string();
        let arguments_json = match function.get("arguments") {
            Some(serde_json::Value::String(text)) => text.clone(),
            Some(other) => serde_json::to_string(other).unwrap_or_else(|_| "{}".to_string()),
            None => "{}".to_string(),
        };
        let id = call
            .get("id")
            .and_then(|v| v.as_str())
            .map(str::to_string)
            .unwrap_or_else(|| format!("call_{index}"));

        tool_calls.push(ProposedToolCall {
            id,
            tool_name,
            arguments_json,
        });
    }

    Ok(tool_calls)
}

fn parse_ollama_usage(value: &serde_json::Value) -> TokenUsage {
    TokenUsage {
        input_tokens: value
            .get("prompt_eval_count")
            .and_then(|v| v.as_i64())
            .unwrap_or(0),
        output_tokens: value
            .get("eval_count")
            .and_then(|v| v.as_i64())
            .unwrap_or(0),
        cached_tokens: 0,
    }
}

#[derive(Default)]
struct OllamaStreamState {
    text: String,
    tool_calls: Vec<ProposedToolCall>,
    usage: TokenUsage,
    accumulated_bytes: usize,
}

impl OllamaStreamState {
    fn process_line(
        &mut self,
        provider: &str,
        line: &str,
    ) -> Result<Vec<ModelStreamEvent>, ProviderError> {
        let trimmed = line.trim();
        if trimmed.is_empty() {
            return Ok(Vec::new());
        }

        let value: serde_json::Value =
            serde_json::from_str(trimmed).map_err(|e| ProviderError::MalformedResponse {
                provider: provider.to_string(),
                reason: format!("Invalid NDJSON stream chunk: {e}"),
            })?;

        if let Some(error) = value.get("error").and_then(|v| v.as_str()) {
            return Err(ProviderError::MalformedResponse {
                provider: provider.to_string(),
                reason: format!("Provider error: {error}"),
            });
        }

        let mut events = Vec::new();

        if let Some(message) = value.get("message") {
            if let Some(content) = message.get("content").and_then(|v| v.as_str())
                && !content.is_empty()
            {
                self.accumulated_bytes = self.accumulated_bytes.saturating_add(content.len());
                if self.accumulated_bytes > MAX_PROVIDER_STREAM_BODY_BYTES {
                    return Err(ProviderError::MalformedResponse {
                        provider: provider.to_string(),
                        reason: format!(
                            "Streamed response exceeded the {} byte accumulation limit",
                            MAX_PROVIDER_STREAM_BODY_BYTES
                        ),
                    });
                }
                self.text.push_str(content);
                events.push(ModelStreamEvent::TextDelta(content.to_string()));
            }

            let calls = parse_ollama_tool_calls(provider, message)?;
            for call in calls {
                self.tool_calls.push(call.clone());
                events.push(ModelStreamEvent::ToolCall(call));
            }
        }

        self.usage = parse_ollama_usage(&value);
        self.usage.cached_tokens = 0;

        if value.get("done").and_then(|v| v.as_bool()).unwrap_or(false) {
            events.push(ModelStreamEvent::Usage(self.usage.clone()));
            events.push(ModelStreamEvent::Done(ModelCompletionResponse {
                text: self.text.clone(),
                tool_calls: self.tool_calls.clone(),
                usage: self.usage.clone(),
            }));
        }

        Ok(events)
    }
}

#[async_trait]
impl ModelProviderAdapter for OllamaAdapter {
    fn provider_name(&self) -> &str {
        &self.provider_name
    }

    async fn complete(
        &self,
        req: &ModelCompletionRequest,
    ) -> Result<ModelCompletionResponse, ProviderError> {
        let endpoint = format!("{}/api/chat", self.base_url);
        let payload = build_ollama_payload(req, false);

        let response = self
            .client
            .post(&endpoint)
            .json(&payload)
            .send()
            .await
            .map_err(|e| {
                adapter_support::map_reqwest_send_error(
                    &self.provider_name,
                    self.timeout_ms,
                    &e,
                    None,
                )
            })?;

        let status = response.status();
        let retry_after = retry_after_ms(response.headers());
        let body = response.text().await.unwrap_or_default();

        if !status.is_success() {
            return Err(map_provider_http_error(
                &self.provider_name,
                status.as_u16(),
                &body,
                None,
                retry_after,
            ));
        }

        Self::parse_chat_json(&self.provider_name, &body)
    }

    fn stream(&self, req: ModelCompletionRequest) -> Option<ModelCompletionStream> {
        let client = self.client.clone();
        let provider = self.provider_name.clone();
        let base_url = self.base_url.clone();
        let timeout_ms = self.timeout_ms;

        Some(Box::pin(async_stream::try_stream! {
            let endpoint = format!("{base_url}/api/chat");
            let payload = build_ollama_payload(&req, true);
            let response = client
                .post(&endpoint)
                .json(&payload)
                .send()
                .await
                .map_err(|e| {
                    adapter_support::map_reqwest_send_error(
                        &provider,
                        timeout_ms,
                        &e,
                        None,
                    )
                })?;

            let response =
                adapter_support::ensure_success_response(&provider, response, None).await?;

            let mut byte_stream = response.bytes_stream();
            let mut lines = adapter_support::LineAccumulator::new(&provider);
            let mut state = OllamaStreamState::default();

            while let Some(chunk) = byte_stream.next().await {
                let chunk = chunk.map_err(|e| ProviderError::Transport {
                    provider: provider.clone(),
                    message: e.to_string(),
                })?;

                for line in lines.push(&chunk)? {
                    for event in state.process_line(&provider, &line)? {
                        yield event;
                    }
                }
            }

            if let Some(line) = lines.finish()? {
                for event in state.process_line(&provider, &line)? {
                    yield event;
                }
            }
        }))
    }
}
