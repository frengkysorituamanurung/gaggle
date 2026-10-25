//! Integration tests for GitHub issue #18: provider adapters and native streaming.
//!
//! The suite uses a tiny local HTTP server built directly on `tokio::net::TcpListener`; no
//! external mock crate and no paid provider calls are involved.

pub use gaggle_core::{credentials, provider};

#[path = "../src/provider_oauth.rs"]
mod provider_oauth;

use futures::StreamExt;
use gaggle_core::credentials::{CredentialStore, InMemoryCredentialStore, SecretString};
use gaggle_core::provider::{
    AnthropicAdapter, ModelCompletionRequest, ModelProviderAdapter, ModelStreamEvent,
    OllamaAdapter, ProviderError, ProviderMessage, ToolDefinition, map_provider_http_error,
    validate_provider_base_url,
};
use provider_oauth::{ChatGptAdapter, ChatGptOAuthClient, ChatGptOAuthConfig, OAuthTokenSet};
use std::{
    collections::HashMap,
    sync::Arc,
    time::{Duration, SystemTime, UNIX_EPOCH},
};
use tokio::{
    io::{AsyncReadExt, AsyncWriteExt},
    net::{TcpListener, TcpStream},
    sync::mpsc,
};

const ANTHROPIC_KEY: &str = "sk-ant-test-secret-key";

// ---------------------------------------------------------------------------
// Local mock HTTP server
// ---------------------------------------------------------------------------

struct MockResponse {
    status: u16,
    reason: &'static str,
    content_type: &'static str,
    chunks: Vec<String>,
    stream: bool,
    extra_headers: Vec<(&'static str, String)>,
}

impl MockResponse {
    fn json(body: impl Into<String>) -> Self {
        Self {
            status: 200,
            reason: "OK",
            content_type: "application/json",
            chunks: vec![body.into()],
            stream: false,
            extra_headers: Vec::new(),
        }
    }

    fn sse(chunks: Vec<String>) -> Self {
        Self {
            status: 200,
            reason: "OK",
            content_type: "text/event-stream",
            chunks,
            stream: true,
            extra_headers: Vec::new(),
        }
    }

    fn ndjson(chunks: Vec<String>) -> Self {
        Self {
            status: 200,
            reason: "OK",
            content_type: "application/x-ndjson",
            chunks,
            stream: true,
            extra_headers: Vec::new(),
        }
    }

    fn error(status: u16, reason: &'static str, body: impl Into<String>) -> Self {
        Self {
            status,
            reason,
            content_type: "application/json",
            chunks: vec![body.into()],
            stream: false,
            extra_headers: Vec::new(),
        }
    }

    fn with_header(mut self, name: &'static str, value: impl Into<String>) -> Self {
        self.extra_headers.push((name, value.into()));
        self
    }
}

struct MockServer {
    base_url: String,
    requests: mpsc::UnboundedReceiver<Vec<u8>>,
}

impl MockServer {
    async fn spawn(responses: Vec<MockResponse>) -> Self {
        let listener = TcpListener::bind(("127.0.0.1", 0))
            .await
            .expect("bind mock server");
        let address = listener.local_addr().expect("mock server address");
        let (sender, requests) = mpsc::unbounded_channel();

        tokio::spawn(async move {
            for response in responses {
                let accept = tokio::time::timeout(Duration::from_secs(10), listener.accept()).await;
                let Ok(Ok((mut socket, _))) = accept else {
                    return;
                };

                let request = read_request(&mut socket).await;
                let _ = sender.send(request);
                write_response(&mut socket, &response).await;
                let _ = socket.shutdown().await;
            }
        });

        Self {
            base_url: format!("http://{address}"),
            requests,
        }
    }

    async fn next_request(&mut self) -> String {
        let bytes = tokio::time::timeout(Duration::from_secs(10), self.requests.recv())
            .await
            .expect("mock server request timed out")
            .expect("mock server closed");
        String::from_utf8_lossy(&bytes).to_string()
    }
}

async fn read_request(socket: &mut TcpStream) -> Vec<u8> {
    let mut buffer = Vec::new();
    let mut scratch = [0u8; 4096];
    let mut header_end = None;
    let mut content_length = 0usize;

    loop {
        let read = tokio::time::timeout(Duration::from_secs(5), socket.read(&mut scratch)).await;
        let Ok(Ok(read)) = read else {
            break;
        };
        if read == 0 {
            break;
        }
        buffer.extend_from_slice(&scratch[..read]);

        if header_end.is_none()
            && let Some(position) = find_bytes(&buffer, b"\r\n\r\n")
        {
            header_end = Some(position + 4);
            let headers = String::from_utf8_lossy(&buffer[..position]);
            for line in headers.lines() {
                if let Some((name, value)) = line.split_once(':')
                    && name.eq_ignore_ascii_case("content-length")
                {
                    content_length = value.trim().parse().unwrap_or(0);
                }
            }
        }

        if let Some(end) = header_end
            && buffer.len() >= end + content_length
        {
            break;
        }
    }

    buffer
}

async fn write_response(socket: &mut TcpStream, response: &MockResponse) {
    let mut head = format!("HTTP/1.1 {} {}\r\n", response.status, response.reason);
    head.push_str(&format!("Content-Type: {}\r\n", response.content_type));
    for (name, value) in &response.extra_headers {
        head.push_str(&format!("{name}: {value}\r\n"));
    }
    head.push_str("Connection: close\r\n");
    if !response.stream {
        let length: usize = response.chunks.iter().map(String::len).sum();
        head.push_str(&format!("Content-Length: {length}\r\n"));
    }
    head.push_str("\r\n");

    let _ = socket.write_all(head.as_bytes()).await;
    for (index, chunk) in response.chunks.iter().enumerate() {
        let _ = socket.write_all(chunk.as_bytes()).await;
        let _ = socket.flush().await;
        if index + 1 < response.chunks.len() {
            tokio::time::sleep(Duration::from_millis(3)).await;
        }
    }
}

fn find_bytes(haystack: &[u8], needle: &[u8]) -> Option<usize> {
    haystack
        .windows(needle.len())
        .position(|window| window == needle)
}

fn request_body(request: &str) -> &str {
    request
        .split_once("\r\n\r\n")
        .map(|(_, body)| body)
        .unwrap_or("")
}

fn parse_form(body: &str) -> HashMap<String, String> {
    body.split('&')
        .filter(|pair| !pair.is_empty())
        .map(|pair| {
            let (name, value) = pair.split_once('=').unwrap_or((pair, ""));
            (percent_decode(name), percent_decode(value))
        })
        .collect()
}

fn percent_decode(input: &str) -> String {
    let bytes = input.as_bytes();
    let mut output = Vec::with_capacity(bytes.len());
    let mut index = 0;
    while index < bytes.len() {
        match bytes[index] {
            b'+' => {
                output.push(b' ');
                index += 1;
            }
            b'%' if index + 2 < bytes.len() => {
                let encoded = std::str::from_utf8(&bytes[index + 1..index + 3])
                    .ok()
                    .and_then(|hex| u8::from_str_radix(hex, 16).ok());
                if let Some(byte) = encoded {
                    output.push(byte);
                    index += 3;
                } else {
                    output.push(bytes[index]);
                    index += 1;
                }
            }
            byte => {
                output.push(byte);
                index += 1;
            }
        }
    }
    String::from_utf8_lossy(&output).to_string()
}

fn split_into_chunks(input: &str, chunk_len: usize) -> Vec<String> {
    let bytes = input.as_bytes();
    bytes
        .chunks(chunk_len)
        .map(|chunk| String::from_utf8_lossy(chunk).to_string())
        .collect()
}

fn completion_request() -> ModelCompletionRequest {
    ModelCompletionRequest {
        model_key: "gpt-test".to_string(),
        messages: vec![
            ProviderMessage {
                role: "system".to_string(),
                content: "You are helpful".to_string(),
                ..Default::default()
            },
            ProviderMessage {
                role: "user".to_string(),
                content: "Read Cargo.toml".to_string(),
                ..Default::default()
            },
            ProviderMessage {
                role: "tool".to_string(),
                content: "{\"ok\":true}".to_string(),
                ..Default::default()
            },
        ],
        tools: vec![ToolDefinition {
            name: "fs.read".to_string(),
            description: "Read a file".to_string(),
            input_schema: serde_json::json!({
                "type": "object",
                "properties": {"path": {"type": "string"}},
                "required": ["path"],
            }),
        }],
        max_output_tokens: Some(512),
    }
}

fn now_ms() -> i64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("clock after epoch")
        .as_millis() as i64
}

fn oauth_tokens(
    access_token: &str,
    refresh_token: Option<&str>,
    expires_at_ms: Option<i64>,
) -> OAuthTokenSet {
    OAuthTokenSet {
        access_token: access_token.to_string(),
        refresh_token: refresh_token.map(str::to_string),
        expires_at_ms,
        token_type: "Bearer".to_string(),
        scope: Some("openid profile".to_string()),
    }
}

fn chatgpt_config(token_url: &str) -> ChatGptOAuthConfig {
    ChatGptOAuthConfig {
        client_id: "client-abc".to_string(),
        auth_url: "https://auth.example.test/oauth/authorize".to_string(),
        token_url: token_url.to_string(),
        redirect_uri: "http://127.0.0.1:1455/auth/callback".to_string(),
        scopes: vec!["openid".to_string(), "profile".to_string()],
    }
}

// ---------------------------------------------------------------------------
// Anthropic
// ---------------------------------------------------------------------------

#[tokio::test]
async fn anthropic_complete_normalizes_text_tools_and_usage() {
    let body = r#"{
        "id": "msg_1",
        "type": "message",
        "role": "assistant",
        "content": [
            {"type": "text", "text": "Hello "},
            {"type": "text", "text": "world"},
            {"type": "tool_use", "id": "toolu_1", "name": "fs.read", "input": {"path": "Cargo.toml"}}
        ],
        "usage": {"input_tokens": 42, "output_tokens": 7, "cache_read_input_tokens": 3}
    }"#;
    let mut server = MockServer::spawn(vec![MockResponse::json(body)]).await;

    let store = Arc::new(InMemoryCredentialStore::new());
    store
        .set_secret("anthropic_main", SecretString::new(ANTHROPIC_KEY).unwrap())
        .unwrap();
    let adapter = AnthropicAdapter::new(
        "anthropic",
        &format!("{}/v1", server.base_url),
        Some("anthropic_main".to_string()),
        store,
        5000,
    )
    .unwrap();

    let response = adapter
        .complete(&completion_request())
        .await
        .expect("anthropic completion");

    assert_eq!(response.text, "Hello world");
    assert_eq!(response.tool_calls.len(), 1);
    assert_eq!(response.tool_calls[0].id, "toolu_1");
    assert_eq!(response.tool_calls[0].tool_name, "fs.read");
    assert_eq!(
        response.tool_calls[0].arguments_json,
        r#"{"path":"Cargo.toml"}"#
    );
    assert_eq!(response.usage.input_tokens, 42);
    assert_eq!(response.usage.output_tokens, 7);
    assert_eq!(response.usage.cached_tokens, 3);

    let request = server.next_request().await;
    let lower = request.to_lowercase();
    assert!(request.starts_with("POST /v1/messages "));
    assert!(lower.contains("x-api-key: sk-ant-test-secret-key"));
    assert!(lower.contains("anthropic-version: 2023-06-01"));
    assert!(lower.contains("content-type: application/json"));

    let payload: serde_json::Value =
        serde_json::from_str(request_body(&request)).expect("anthropic request JSON");
    assert_eq!(payload["system"], "You are helpful");
    assert_eq!(payload["max_tokens"], 512);
    assert_eq!(payload["stream"], false);
    assert_eq!(payload["messages"][0]["role"], "user");
    let tool_message = &payload["messages"][1];
    assert_eq!(tool_message["role"], "user");
    let tool_content = &tool_message["content"];
    assert!(
        tool_content.is_array(),
        "tool result must be a content block array"
    );
    assert_eq!(tool_content[0]["type"], "tool_result");
    assert_eq!(tool_content[0]["content"], "{\"ok\":true}");
    assert_eq!(payload["tools"][0]["name"], "fs.read");
    assert!(payload["tools"][0].get("input_schema").is_some());
    assert!(payload["tools"][0].get("parameters").is_none());
}

#[tokio::test]
async fn anthropic_stream_emits_text_tool_usage_and_done_with_partial_chunks() {
    let sse = concat!(
        "event: message_start\n",
        "data: {\"type\":\"message_start\",\"message\":{\"usage\":{\"input_tokens\":11,\"cache_read_input_tokens\":2}}}\n\n",
        "event: content_block_start\n",
        "data: {\"type\":\"content_block_start\",\"index\":0,\"content_block\":{\"type\":\"text\",\"text\":\"\"}}\n\n",
        "event: content_block_delta\n",
        "data: {\"type\":\"content_block_delta\",\"index\":0,\"delta\":{\"type\":\"text_delta\",\"text\":\"Hi\"}}\n\n",
        "event: content_block_start\n",
        "data: {\"type\":\"content_block_start\",\"index\":1,\"content_block\":{\"type\":\"tool_use\",\"id\":\"toolu_9\",\"name\":\"fs.read\",\"input\":{}}}\n\n",
        "event: content_block_delta\n",
        "data: {\"type\":\"content_block_delta\",\"index\":1,\"delta\":{\"type\":\"input_json_delta\",\"partial_json\":\"{\\\"path\\\":\"}}\n\n",
        "event: content_block_delta\n",
        "data: {\"type\":\"content_block_delta\",\"index\":1,\"delta\":{\"type\":\"input_json_delta\",\"partial_json\":\"\\\"Cargo.toml\\\"}\"}}\n\n",
        "event: content_block_stop\n",
        "data: {\"type\":\"content_block_stop\",\"index\":1}\n\n",
        "event: message_delta\n",
        "data: {\"type\":\"message_delta\",\"usage\":{\"output_tokens\":5}}\n\n",
        "event: message_stop\n",
        "data: {\"type\":\"message_stop\"}\n\n",
    );
    let chunks = split_into_chunks(sse, 37);
    assert!(chunks.len() > 2, "expected the SSE body to be split");
    let mut server = MockServer::spawn(vec![MockResponse::sse(chunks)]).await;

    let store = Arc::new(InMemoryCredentialStore::new());
    store
        .set_secret("anthropic_main", SecretString::new(ANTHROPIC_KEY).unwrap())
        .unwrap();
    let adapter = AnthropicAdapter::new(
        "anthropic",
        &format!("{}/v1", server.base_url),
        Some("anthropic_main".to_string()),
        store,
        5000,
    )
    .unwrap();

    let mut stream = adapter
        .stream(completion_request())
        .expect("anthropic native stream");
    let mut events = Vec::new();
    while let Some(item) = stream.next().await {
        events.push(item.expect("stream event"));
    }

    let text: String = events
        .iter()
        .filter_map(|event| match event {
            ModelStreamEvent::TextDelta(delta) => Some(delta.as_str()),
            _ => None,
        })
        .collect();
    assert_eq!(text, "Hi");

    let tool_events: Vec<_> = events
        .iter()
        .filter_map(|event| match event {
            ModelStreamEvent::ToolCall(call) => Some(call),
            _ => None,
        })
        .collect();
    assert_eq!(tool_events.len(), 1);
    assert_eq!(tool_events[0].id, "toolu_9");
    assert_eq!(tool_events[0].tool_name, "fs.read");
    assert_eq!(tool_events[0].arguments_json, r#"{"path":"Cargo.toml"}"#);

    let usage = events
        .iter()
        .find_map(|event| match event {
            ModelStreamEvent::Usage(usage) => Some(usage),
            _ => None,
        })
        .expect("usage event");
    assert_eq!(usage.input_tokens, 11);
    assert_eq!(usage.output_tokens, 5);
    assert_eq!(usage.cached_tokens, 2);

    let done = events
        .iter()
        .find_map(|event| match event {
            ModelStreamEvent::Done(response) => Some(response),
            _ => None,
        })
        .expect("done event");
    assert_eq!(done.text, "Hi");
    assert_eq!(done.tool_calls.len(), 1);
    assert_eq!(
        done.tool_calls[0].arguments_json,
        r#"{"path":"Cargo.toml"}"#
    );
    assert_eq!(done.usage.input_tokens, 11);
    assert_eq!(done.usage.output_tokens, 5);

    let request = server.next_request().await;
    let payload: serde_json::Value = serde_json::from_str(request_body(&request)).unwrap();
    assert_eq!(payload["stream"], true);
}

#[tokio::test]
async fn anthropic_missing_api_key_fails_without_network_and_stream_errors() {
    let store = Arc::new(InMemoryCredentialStore::new());
    let adapter = AnthropicAdapter::new(
        "anthropic",
        "https://api.anthropic.test/v1",
        None,
        store,
        1000,
    )
    .unwrap();

    let error = adapter.complete(&completion_request()).await.unwrap_err();
    assert!(matches!(
        error,
        ProviderError::Credential(gaggle_core::credentials::CredentialError::NotFound(_))
    ));

    let mut stream = adapter.stream(completion_request()).unwrap();
    let first = stream.next().await.expect("first stream item");
    assert!(matches!(
        first,
        Err(ProviderError::Credential(
            gaggle_core::credentials::CredentialError::NotFound(_)
        ))
    ));
}

#[tokio::test]
async fn anthropic_stream_malformed_json_yields_error_item() {
    let server = MockServer::spawn(vec![MockResponse::sse(vec![
        "event: content_block_delta\ndata: {not-json}\n\n".to_string(),
    ])])
    .await;

    let store = Arc::new(InMemoryCredentialStore::new());
    store
        .set_secret("anthropic_main", SecretString::new(ANTHROPIC_KEY).unwrap())
        .unwrap();
    let adapter = AnthropicAdapter::new(
        "anthropic",
        &format!("{}/v1", server.base_url),
        Some("anthropic_main".to_string()),
        store,
        5000,
    )
    .unwrap();

    let mut stream = adapter.stream(completion_request()).unwrap();
    let item = stream.next().await.expect("one error item");
    assert!(
        matches!(item, Err(ProviderError::MalformedResponse { .. })),
        "expected a MalformedResponse item, got {item:?}"
    );
}

#[test]
fn stream_line_buffer_is_bounded() {
    use gaggle_core::provider::MAX_PROVIDER_STREAM_LINE_BYTES;
    use gaggle_core::provider::adapter_support::LineAccumulator;

    let mut lines = LineAccumulator::new("buffered");
    let oversized = vec![b'a'; MAX_PROVIDER_STREAM_LINE_BYTES + 1];
    let error = lines.push(&oversized).unwrap_err();
    assert!(matches!(error, ProviderError::MalformedResponse { .. }));
}

// ---------------------------------------------------------------------------
// Ollama
// ---------------------------------------------------------------------------

#[tokio::test]
async fn ollama_complete_normalizes_text_tools_and_usage() {
    let body = r#"{
        "model": "llama3",
        "message": {
            "role": "assistant",
            "content": "Hello from Ollama",
            "tool_calls": [
                {"function": {"name": "fs.list", "arguments": {"path": "."}}},
                {"function": {"name": "fs.read", "arguments": "{\"path\":\"README.md\"}"}}
            ]
        },
        "prompt_eval_count": 12,
        "eval_count": 8,
        "done": true
    }"#;
    let mut server = MockServer::spawn(vec![MockResponse::json(body)]).await;

    let adapter = OllamaAdapter::new("ollama", &server.base_url, 5000).unwrap();
    let response = adapter
        .complete(&completion_request())
        .await
        .expect("ollama completion");

    assert_eq!(response.text, "Hello from Ollama");
    assert_eq!(response.tool_calls.len(), 2);
    assert_eq!(response.tool_calls[0].tool_name, "fs.list");
    assert_eq!(response.tool_calls[0].arguments_json, r#"{"path":"."}"#);
    assert_eq!(response.tool_calls[0].id, "call_0");
    assert_eq!(response.tool_calls[1].tool_name, "fs.read");
    assert_eq!(
        response.tool_calls[1].arguments_json,
        r#"{"path":"README.md"}"#
    );
    assert_eq!(response.usage.input_tokens, 12);
    assert_eq!(response.usage.output_tokens, 8);

    let request = server.next_request().await;
    assert!(request.starts_with("POST /api/chat "));
    assert!(!request.to_lowercase().contains("authorization:"));
    let payload: serde_json::Value = serde_json::from_str(request_body(&request)).unwrap();
    assert_eq!(payload["stream"], false);
    assert_eq!(payload["messages"][0]["role"], "system");
    assert!(payload.get("tools").is_some());
}

#[tokio::test]
async fn ollama_ndjson_stream_emits_text_tool_usage_and_done() {
    let body = concat!(
        "{\"message\":{\"role\":\"assistant\",\"content\":\"Hel\"},\"done\":false}\n",
        "{\"message\":{\"role\":\"assistant\",\"content\":\"lo\"},\"done\":false}\n",
        "{\"message\":{\"role\":\"assistant\",\"content\":\"\",\"tool_calls\":[{\"function\":{\"name\":\"fs.read\",\"arguments\":\"{\\\"path\\\":\\\"x\\\"}\"}}]},\"done\":false}\n",
        "{\"message\":{\"role\":\"assistant\",\"content\":\"\"},\"done\":true,\"prompt_eval_count\":4,\"eval_count\":2}\n",
    );
    let chunks = split_into_chunks(body, 17);
    let mut server = MockServer::spawn(vec![MockResponse::ndjson(chunks)]).await;

    let adapter = OllamaAdapter::new("ollama", &server.base_url, 5000).unwrap();
    let mut stream = adapter
        .stream(completion_request())
        .expect("ollama native stream");
    let mut events = Vec::new();
    while let Some(item) = stream.next().await {
        events.push(item.expect("stream event"));
    }

    let text: String = events
        .iter()
        .filter_map(|event| match event {
            ModelStreamEvent::TextDelta(delta) => Some(delta.as_str()),
            _ => None,
        })
        .collect();
    assert_eq!(text, "Hello");

    let tool_events: Vec<_> = events
        .iter()
        .filter_map(|event| match event {
            ModelStreamEvent::ToolCall(call) => Some(call),
            _ => None,
        })
        .collect();
    assert_eq!(tool_events.len(), 1);
    assert_eq!(tool_events[0].tool_name, "fs.read");
    assert_eq!(tool_events[0].arguments_json, r#"{"path":"x"}"#);
    assert_eq!(tool_events[0].id, "call_0");

    let usage = events
        .iter()
        .find_map(|event| match event {
            ModelStreamEvent::Usage(usage) => Some(usage),
            _ => None,
        })
        .expect("usage event");
    assert_eq!(usage.input_tokens, 4);
    assert_eq!(usage.output_tokens, 2);

    let done = events
        .iter()
        .find_map(|event| match event {
            ModelStreamEvent::Done(response) => Some(response),
            _ => None,
        })
        .expect("done event");
    assert_eq!(done.text, "Hello");
    assert_eq!(done.tool_calls.len(), 1);
    assert_eq!(done.usage.input_tokens, 4);
    assert_eq!(done.usage.output_tokens, 2);

    let request = server.next_request().await;
    let payload: serde_json::Value = serde_json::from_str(request_body(&request)).unwrap();
    assert_eq!(payload["stream"], true);
}

// ---------------------------------------------------------------------------
// OAuth (PKCE) client
// ---------------------------------------------------------------------------

#[tokio::test]
async fn oauth_authorization_url_contains_pkce_state_and_scopes() {
    let store = Arc::new(InMemoryCredentialStore::new());
    let client = ChatGptOAuthClient::new(
        chatgpt_config("https://auth.example.test/oauth/token"),
        store,
        5000,
    )
    .unwrap();

    let authorization = client.authorization_url().unwrap();
    let url = reqwest::Url::parse(&authorization.url).unwrap();
    let pairs: HashMap<String, String> = url.query_pairs().into_owned().collect();

    assert_eq!(pairs.get("response_type").map(String::as_str), Some("code"));
    assert_eq!(
        pairs.get("client_id").map(String::as_str),
        Some("client-abc")
    );
    assert_eq!(
        pairs.get("redirect_uri").map(String::as_str),
        Some("http://127.0.0.1:1455/auth/callback")
    );
    assert_eq!(
        pairs.get("scope").map(String::as_str),
        Some("openid profile")
    );
    assert_eq!(
        pairs.get("code_challenge_method").map(String::as_str),
        Some("S256")
    );
    assert_eq!(pairs.get("state"), Some(&authorization.state));
    assert_eq!(authorization.state.len(), 43);
    assert_eq!(authorization.code_verifier.len(), 43);
    assert_eq!(pairs.get("code_challenge").map(String::len), Some(43));
    assert!(
        authorization
            .code_verifier
            .chars()
            .all(|c| c.is_ascii_alphanumeric() || c == '-' || c == '_'),
        "PKCE verifier must be URL-safe"
    );
    assert_ne!(authorization.state, authorization.code_verifier);
}

#[tokio::test]
async fn oauth_exchange_and_refresh_store_tokens_and_preserve_refresh_token() {
    let exchange_body = serde_json::json!({
        "access_token": "access-1",
        "refresh_token": "refresh-1",
        "expires_in": 3600,
        "token_type": "Bearer",
        "scope": "openid profile",
    })
    .to_string();
    let refresh_body = serde_json::json!({
        "access_token": "access-2",
        "expires_in": 1800,
        "token_type": "Bearer",
    })
    .to_string();

    let mut server = MockServer::spawn(vec![
        MockResponse::json(exchange_body),
        MockResponse::json(refresh_body),
    ])
    .await;

    let store = Arc::new(InMemoryCredentialStore::new());
    let client = ChatGptOAuthClient::new(
        chatgpt_config(&format!("{}/oauth/token", server.base_url)),
        store.clone(),
        5000,
    )
    .unwrap();

    let tokens = client
        .exchange_code("chatgpt_primary", "auth-code-1", "verifier-1")
        .await
        .expect("token exchange");
    assert_eq!(tokens.access_token, "access-1");
    assert_eq!(tokens.refresh_token.as_deref(), Some("refresh-1"));
    assert_eq!(tokens.token_type, "Bearer");
    assert!(tokens.expires_at_ms.unwrap() > now_ms());
    assert_eq!(client.load_tokens("chatgpt_primary").unwrap(), tokens);

    let request = server.next_request().await;
    assert!(request.starts_with("POST /oauth/token "));
    let lower = request.to_lowercase();
    assert!(lower.contains("content-type: application/x-www-form-urlencoded"));
    let form = parse_form(request_body(&request));
    assert_eq!(
        form.get("grant_type").map(String::as_str),
        Some("authorization_code")
    );
    assert_eq!(
        form.get("client_id").map(String::as_str),
        Some("client-abc")
    );
    assert_eq!(form.get("code").map(String::as_str), Some("auth-code-1"));
    assert_eq!(
        form.get("redirect_uri").map(String::as_str),
        Some("http://127.0.0.1:1455/auth/callback")
    );
    assert_eq!(
        form.get("code_verifier").map(String::as_str),
        Some("verifier-1")
    );

    let refreshed = client.refresh("chatgpt_primary").await.expect("refresh");
    assert_eq!(refreshed.access_token, "access-2");
    assert_eq!(
        refreshed.refresh_token.as_deref(),
        Some("refresh-1"),
        "refresh response omitted refresh_token, so the previous token must be preserved"
    );
    assert!(refreshed.expires_at_ms.unwrap() > now_ms());
    assert_eq!(client.load_tokens("chatgpt_primary").unwrap(), refreshed);

    let request = server.next_request().await;
    let form = parse_form(request_body(&request));
    assert_eq!(
        form.get("grant_type").map(String::as_str),
        Some("refresh_token")
    );
    assert_eq!(
        form.get("refresh_token").map(String::as_str),
        Some("refresh-1")
    );
    assert_eq!(
        form.get("client_id").map(String::as_str),
        Some("client-abc")
    );
}

#[test]
fn oauth_rejects_invalid_token_url() {
    let store = Arc::new(InMemoryCredentialStore::new());
    let mut config = chatgpt_config("file:///etc/gaggle/tokens");
    config.redirect_uri = "http://127.0.0.1:1455/auth/callback".to_string();

    let error = match ChatGptOAuthClient::new(config, store, 5000) {
        Ok(_) => panic!("expected an invalid token URL to be rejected"),
        Err(error) => error,
    };
    assert!(matches!(error, ProviderError::InvalidBaseUrl { .. }));
}

// ---------------------------------------------------------------------------
// ChatGPT adapter
// ---------------------------------------------------------------------------

#[tokio::test]
async fn chatgpt_adapter_sends_bearer_token_and_parses_completion() {
    let body = serde_json::json!({
        "choices": [{"message": {"role": "assistant", "content": "OAuth works"}}],
        "usage": {"prompt_tokens": 5, "completion_tokens": 2},
    })
    .to_string();
    let mut server = MockServer::spawn(vec![MockResponse::json(body)]).await;

    let store = Arc::new(InMemoryCredentialStore::new());
    let tokens = oauth_tokens(
        "access-token-xyz",
        Some("refresh-1"),
        Some(now_ms() + 600_000),
    );
    store
        .set_secret(
            "chatgpt_primary",
            SecretString::new(serde_json::to_string(&tokens).unwrap()).unwrap(),
        )
        .unwrap();

    let adapter = ChatGptAdapter::new(
        "chatgpt",
        &format!("{}/v1", server.base_url),
        Some("chatgpt_primary".to_string()),
        store,
        None,
        5000,
    )
    .unwrap();

    let response = adapter
        .complete(&completion_request())
        .await
        .expect("chatgpt completion");
    assert_eq!(response.text, "OAuth works");
    assert_eq!(response.usage.input_tokens, 5);
    assert_eq!(response.usage.output_tokens, 2);

    let request = server.next_request().await;
    let lower = request.to_lowercase();
    assert!(request.starts_with("POST /v1/chat/completions "));
    assert!(lower.contains("authorization: bearer access-token-xyz"));
}

#[tokio::test]
async fn chatgpt_adapter_maps_401_without_leaking_token() {
    let token = "access-token-super-secret";
    let body = serde_json::json!({
        "error": {"message": "invalid token access-token-super-secret"},
    })
    .to_string();
    let mut server = MockServer::spawn(vec![MockResponse::error(401, "Unauthorized", body)]).await;

    let store = Arc::new(InMemoryCredentialStore::new());
    let tokens = oauth_tokens(token, None, Some(now_ms() + 600_000));
    store
        .set_secret(
            "chatgpt_primary",
            SecretString::new(serde_json::to_string(&tokens).unwrap()).unwrap(),
        )
        .unwrap();

    let adapter = ChatGptAdapter::new(
        "chatgpt",
        &format!("{}/v1", server.base_url),
        Some("chatgpt_primary".to_string()),
        store,
        None,
        5000,
    )
    .unwrap();

    let error = adapter.complete(&completion_request()).await.unwrap_err();
    match error {
        ProviderError::AuthenticationFailed { provider, message } => {
            assert_eq!(provider, "chatgpt");
            assert!(
                !message.contains(token),
                "authentication error leaked the access token: {message}"
            );
            assert!(message.contains("[REDACTED]"));
        }
        other => panic!("expected AuthenticationFailed, got {other:?}"),
    }

    let request = server.next_request().await;
    assert!(
        request
            .to_lowercase()
            .contains(&format!("authorization: bearer {}", token.to_lowercase()))
    );
}

#[tokio::test]
async fn chatgpt_adapter_maps_429_with_retry_after() {
    let body = serde_json::json!({"error": {"message": "slow down"}}).to_string();
    let mut server = MockServer::spawn(vec![
        MockResponse::error(429, "Too Many Requests", body).with_header("Retry-After", "3"),
    ])
    .await;

    let store = Arc::new(InMemoryCredentialStore::new());
    store
        .set_secret(
            "chatgpt_primary",
            SecretString::new("plain-api-key".to_string()).unwrap(),
        )
        .unwrap();
    let adapter = ChatGptAdapter::new(
        "chatgpt",
        &format!("{}/v1", server.base_url),
        Some("chatgpt_primary".to_string()),
        store,
        None,
        5000,
    )
    .unwrap();

    let error = adapter.complete(&completion_request()).await.unwrap_err();
    assert_eq!(
        error,
        ProviderError::RateLimited {
            provider: "chatgpt".to_string(),
            retry_after_ms: Some(3000),
        }
    );

    let request = server.next_request().await;
    assert!(
        request
            .to_lowercase()
            .contains("authorization: bearer plain-api-key")
    );
}

#[tokio::test]
async fn chatgpt_stream_emits_text_usage_and_done() {
    let sse = concat!(
        "data: {\"choices\":[{\"delta\":{\"role\":\"assistant\"}}]}\n\n",
        "data: {\"choices\":[{\"delta\":{\"content\":\"Hello\"}}]}\n\n",
        "data: {\"choices\":[{\"delta\":{\"content\":\"!\"},\"index\":0}]}\n\n",
        "data: {\"choices\":[{\"delta\":{},\"finish_reason\":\"stop\"}],\"usage\":{\"prompt_tokens\":3,\"completion_tokens\":2}}\n\n",
        "data: [DONE]\n\n",
    );
    let chunks = split_into_chunks(sse, 23);
    let mut server = MockServer::spawn(vec![MockResponse::sse(chunks)]).await;

    let store = Arc::new(InMemoryCredentialStore::new());
    store
        .set_secret(
            "chatgpt_primary",
            SecretString::new("plain-api-key".to_string()).unwrap(),
        )
        .unwrap();
    let adapter = ChatGptAdapter::new(
        "chatgpt",
        &format!("{}/v1", server.base_url),
        Some("chatgpt_primary".to_string()),
        store,
        None,
        5000,
    )
    .unwrap();

    let mut stream = adapter
        .stream(completion_request())
        .expect("chatgpt native stream");
    let mut events = Vec::new();
    while let Some(item) = stream.next().await {
        events.push(item.expect("stream event"));
    }

    let text: String = events
        .iter()
        .filter_map(|event| match event {
            ModelStreamEvent::TextDelta(delta) => Some(delta.as_str()),
            _ => None,
        })
        .collect();
    assert_eq!(text, "Hello!");

    let usage = events
        .iter()
        .find_map(|event| match event {
            ModelStreamEvent::Usage(usage) => Some(usage),
            _ => None,
        })
        .expect("usage event");
    assert_eq!(usage.input_tokens, 3);
    assert_eq!(usage.output_tokens, 2);

    let done = events
        .iter()
        .find_map(|event| match event {
            ModelStreamEvent::Done(response) => Some(response),
            _ => None,
        })
        .expect("done event");
    assert_eq!(done.text, "Hello!");
    assert_eq!(done.usage.input_tokens, 3);

    let request = server.next_request().await;
    let payload: serde_json::Value = serde_json::from_str(request_body(&request)).unwrap();
    assert_eq!(payload["stream"], true);
}

#[tokio::test]
async fn chatgpt_stream_accumulates_tool_call_arguments() {
    let sse = concat!(
        "data: {\"choices\":[{\"delta\":{\"tool_calls\":[{\"index\":0,\"id\":\"call_1\",\"function\":{\"name\":\"fs.read\",\"arguments\":\"{\\\"path\\\":\"}}]}}]}\n\n",
        "data: {\"choices\":[{\"delta\":{\"tool_calls\":[{\"index\":0,\"function\":{\"arguments\":\"\\\"Cargo.toml\\\"}\"}}]}}]}\n\n",
        "data: [DONE]\n\n",
    );
    let chunks = split_into_chunks(sse, 29);
    let _server = MockServer::spawn(vec![MockResponse::sse(chunks)]).await;

    let store = Arc::new(InMemoryCredentialStore::new());
    store
        .set_secret(
            "chatgpt_primary",
            SecretString::new("plain-api-key".to_string()).unwrap(),
        )
        .unwrap();
    let adapter = ChatGptAdapter::new(
        "chatgpt",
        &format!("{}/v1", _server.base_url),
        Some("chatgpt_primary".to_string()),
        store,
        None,
        5000,
    )
    .unwrap();

    let mut stream = adapter.stream(completion_request()).unwrap();
    let mut events = Vec::new();
    while let Some(item) = stream.next().await {
        events.push(item.expect("stream event"));
    }

    let done = events
        .iter()
        .find_map(|event| match event {
            ModelStreamEvent::Done(response) => Some(response),
            _ => None,
        })
        .expect("done event");
    assert_eq!(done.tool_calls.len(), 1);
    assert_eq!(done.tool_calls[0].id, "call_1");
    assert_eq!(done.tool_calls[0].tool_name, "fs.read");
    assert_eq!(
        done.tool_calls[0].arguments_json,
        r#"{"path":"Cargo.toml"}"#
    );
}

#[tokio::test]
async fn chatgpt_adapter_refreshes_expired_token_before_the_call() {
    let refresh_body = serde_json::json!({
        "access_token": "new-access-token",
        "expires_in": 3600,
        "token_type": "Bearer",
    })
    .to_string();
    let chat_body = serde_json::json!({
        "choices": [{"message": {"role": "assistant", "content": "after refresh"}}],
        "usage": {"prompt_tokens": 1, "completion_tokens": 2},
    })
    .to_string();
    let mut server = MockServer::spawn(vec![
        MockResponse::json(refresh_body),
        MockResponse::json(chat_body),
    ])
    .await;

    let store = Arc::new(InMemoryCredentialStore::new());
    let tokens = oauth_tokens("expired-token", Some("refresh-0"), Some(now_ms() - 1_000));
    store
        .set_secret(
            "chatgpt_primary",
            SecretString::new(serde_json::to_string(&tokens).unwrap()).unwrap(),
        )
        .unwrap();

    let oauth = ChatGptOAuthClient::new(
        chatgpt_config(&format!("{}/oauth/token", server.base_url)),
        store.clone(),
        5000,
    )
    .unwrap();

    let adapter = ChatGptAdapter::new(
        "chatgpt",
        &format!("{}/v1", server.base_url),
        Some("chatgpt_primary".to_string()),
        store.clone(),
        Some(oauth),
        5000,
    )
    .unwrap();

    let response = adapter.complete(&completion_request()).await.unwrap();
    assert_eq!(response.text, "after refresh");

    let refresh_request = server.next_request().await;
    assert!(refresh_request.starts_with("POST /oauth/token "));
    let form = parse_form(request_body(&refresh_request));
    assert_eq!(
        form.get("grant_type").map(String::as_str),
        Some("refresh_token")
    );
    assert_eq!(
        form.get("refresh_token").map(String::as_str),
        Some("refresh-0")
    );

    let chat_request = server.next_request().await;
    assert!(
        chat_request
            .to_lowercase()
            .contains("authorization: bearer new-access-token")
    );

    let stored = store.get_secret("chatgpt_primary").unwrap();
    let stored_tokens: OAuthTokenSet = serde_json::from_str(stored.expose_secret()).unwrap();
    assert_eq!(stored_tokens.access_token, "new-access-token");
    assert_eq!(stored_tokens.refresh_token.as_deref(), Some("refresh-0"));
}

// ---------------------------------------------------------------------------
// Error mapping
// ---------------------------------------------------------------------------

#[test]
fn provider_http_error_mapping_handles_auth_rate_limit_and_redaction() {
    let secret = SecretString::new("sk-super-secret-token").unwrap();

    let rate_limited = map_provider_http_error("openai", 429, "slow down", None, Some(2500));
    assert_eq!(
        rate_limited,
        ProviderError::RateLimited {
            provider: "openai".to_string(),
            retry_after_ms: Some(2500),
        }
    );

    let auth = map_provider_http_error(
        "openai",
        401,
        "invalid key sk-super-secret-token",
        Some(&secret),
        None,
    );
    match auth {
        ProviderError::AuthenticationFailed { message, .. } => {
            assert!(!message.contains("sk-super-secret-token"));
            assert!(message.contains("[REDACTED]"));
        }
        other => panic!("expected AuthenticationFailed, got {other:?}"),
    }

    let transport = map_provider_http_error("openai", 500, "boom", None, None);
    assert_eq!(
        transport,
        ProviderError::Transport {
            provider: "openai".to_string(),
            message: "HTTP 500: boom".to_string(),
        }
    );
}

#[test]
fn provider_url_validation_rejects_ssrf_and_embedded_credentials() {
    assert!(validate_provider_base_url("http://127.0.0.1:11434").is_ok());
    assert!(validate_provider_base_url("https://api.openai.com/v1").is_ok());

    for rejected in [
        "http://169.254.169.254/latest/meta-data",
        "http://metadata.google.internal",
        "http://0.0.0.0:8080",
        "https://user:password@api.example.com/v1",
        "file:///etc/passwd",
    ] {
        assert!(
            validate_provider_base_url(rejected).is_err(),
            "expected rejection for {rejected}"
        );
    }
}
