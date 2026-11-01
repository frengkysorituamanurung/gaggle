use anyhow::{Context, Result, bail};
use async_trait::async_trait;
use gaggle_core::ToolExecutor;
use gaggle_db::{
    connection::{new_id, now_ms},
    models::{Extension, ExtensionPermission, ExtensionTool},
    repositories::extensions::ExtensionRepository,
};
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};
use sqlx::SqlitePool;
use std::{
    collections::HashMap,
    process::Stdio,
    sync::{
        Arc,
        atomic::{AtomicI64, Ordering},
    },
    time::Duration,
};
use tokio::{
    io::{AsyncBufReadExt, AsyncWriteExt, BufReader},
    process::{Child, Command},
    sync::{Mutex, RwLock, oneshot},
};
use tracing::warn;
use url::Url;

pub mod bundled;
pub mod connection_manager;
pub mod wasm_runtime;
pub use bundled::{BUNDLED_MCP_SERVERS, BundledMcpServer, install_bundled};
pub use connection_manager::McpConnectionManager;
pub use wasm_runtime::WasmExtensionRuntime;

pub const RECOMMENDED_MAX_ACTIVE_TOOLS: usize = 25;

// ---------------------------------------------------------------------------
// JSON-RPC 2.0
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Serialize)]
struct JsonRpcRequest {
    jsonrpc: &'static str,
    id: i64,
    method: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    params: Option<Value>,
}

#[derive(Debug, Clone, Deserialize)]
struct JsonRpcResponse {
    #[allow(dead_code)]
    jsonrpc: Option<String>,
    id: Option<Value>,
    result: Option<Value>,
    error: Option<JsonRpcError>,
}

#[derive(Debug, Clone, Deserialize)]
struct JsonRpcError {
    code: i64,
    message: String,
    #[allow(dead_code)]
    data: Option<Value>,
}

impl JsonRpcError {
    fn to_anyhow(&self) -> anyhow::Error {
        anyhow::anyhow!("JSON-RPC error {}: {}", self.code, self.message)
    }
}

// ---------------------------------------------------------------------------
// Domain types
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct McpTool {
    pub name: String,
    pub description: Option<String>,
    pub input_schema: Value,
    pub is_read_only: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct McpResource {
    pub uri: String,
    pub name: Option<String>,
    pub description: Option<String>,
    pub mime_type: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct McpResourceContent {
    pub uri: String,
    pub mime_type: Option<String>,
    pub text: Option<String>,
    pub blob: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct McpPrompt {
    pub name: String,
    pub description: Option<String>,
    pub arguments: Vec<Value>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct McpPromptResult {
    pub description: Option<String>,
    pub messages: Vec<Value>,
    pub raw: Value,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct McpToolResult {
    pub content: Vec<Value>,
    pub is_error: bool,
    pub raw: Value,
}

// ---------------------------------------------------------------------------
// Typed JSON-RPC request/response structs for resources and prompts
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct ResourcesListRequest {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub cursor: Option<String>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct ListResourcesRequest {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub cursor: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ResourcesListResponse {
    #[serde(default)]
    pub resources: Vec<McpResource>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub next_cursor: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ListResourcesResponse {
    #[serde(default)]
    pub resources: Vec<McpResource>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub next_cursor: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ReadResourceRequest {
    pub uri: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ReadResourceResponse {
    #[serde(default)]
    pub contents: Vec<McpResourceContent>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct PromptsListRequest {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub cursor: Option<String>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct ListPromptsRequest {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub cursor: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PromptsListResponse {
    #[serde(default)]
    pub prompts: Vec<McpPrompt>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub next_cursor: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ListPromptsResponse {
    #[serde(default)]
    pub prompts: Vec<McpPrompt>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub next_cursor: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GetPromptRequest {
    pub name: String,
    #[serde(default)]
    pub arguments: Value,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PromptGetRequest {
    pub name: String,
    #[serde(default)]
    pub arguments: Value,
}

// Type aliases for alternate naming conventions.
pub type ResourcesListResult = ResourcesListResponse;
pub type ReadResourceResult = ReadResourceResponse;
pub type PromptsListResult = PromptsListResponse;
pub type GetPromptResponse = McpPromptResult;
pub type PromptGetResponse = McpPromptResult;
pub type PromptsGetResponse = McpPromptResult;

// Conversions for ergonomic method calls.
impl From<&str> for ReadResourceRequest {
    fn from(value: &str) -> Self {
        Self { uri: value.to_string() }
    }
}

impl From<String> for ReadResourceRequest {
    fn from(value: String) -> Self {
        Self { uri: value }
    }
}


impl From<(&str, Value)> for GetPromptRequest {
    fn from((name, arguments): (&str, Value)) -> Self {
        Self {
            name: name.to_string(),
            arguments,
        }
    }
}

impl From<(String, Value)> for GetPromptRequest {
    fn from((name, arguments): (String, Value)) -> Self {
        Self { name, arguments }
    }
}

impl From<ListResourcesRequest> for ResourcesListRequest {
    fn from(value: ListResourcesRequest) -> Self {
        Self { cursor: value.cursor }
    }
}

impl From<ResourcesListRequest> for ListResourcesRequest {
    fn from(value: ResourcesListRequest) -> Self {
        Self { cursor: value.cursor }
    }
}

impl From<ListPromptsRequest> for PromptsListRequest {
    fn from(value: ListPromptsRequest) -> Self {
        Self { cursor: value.cursor }
    }
}

impl From<PromptsListRequest> for ListPromptsRequest {
    fn from(value: PromptsListRequest) -> Self {
        Self { cursor: value.cursor }
    }
}

impl From<PromptGetRequest> for GetPromptRequest {
    fn from(value: PromptGetRequest) -> Self {
        Self { name: value.name, arguments: value.arguments }
    }
}

impl From<GetPromptRequest> for PromptGetRequest {
    fn from(value: GetPromptRequest) -> Self {
        Self { name: value.name, arguments: value.arguments }
    }
}

impl McpToolResult {
    pub fn text(&self) -> String {
        self.content
            .iter()
            .filter_map(|c| c.get("text").and_then(|v| v.as_str()))
            .collect::<Vec<_>>()
            .join("\n")
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct McpInitializeResult {
    pub protocol_version: String,
    pub capabilities: Value,
    pub server_info: Value,
}

#[derive(Debug, Clone)]
pub struct McpServerConfig {
    pub kind: String,
    pub command: Option<String>,
    pub args: Vec<String>,
    pub url: Option<String>,
    pub env: HashMap<String, String>,
    pub timeout: Duration,
}

impl McpServerConfig {
    pub fn from_extension(ext: &Extension, timeout_override: Option<Duration>) -> Self {
        let args = ext
            .args
            .as_deref()
            .and_then(|s| serde_json::from_str::<Vec<String>>(s).ok())
            .unwrap_or_default();
        Self {
            kind: ext.kind.clone(),
            command: ext.command.clone(),
            args,
            url: ext.url.clone(),
            env: HashMap::new(),
            timeout: timeout_override.unwrap_or(Duration::from_secs(ext.timeout_secs as u64)),
        }
    }
}

// ---------------------------------------------------------------------------
// Transport trait
// ---------------------------------------------------------------------------

#[async_trait]
pub trait McpTransport: Send + Sync {
    async fn request(&self, method: &str, params: Option<Value>) -> Result<Value>;
    async fn notify(&self, method: &str, params: Option<Value>) -> Result<()>;
    async fn close(&self) -> Result<()>;
}

// ---------------------------------------------------------------------------
// SSRF validation
// ---------------------------------------------------------------------------

pub fn validate_mcp_url(raw: &str) -> Result<String> {
    let trimmed = raw.trim();
    let parsed = Url::parse(trimmed).context("Invalid MCP URL")?;

    if !matches!(parsed.scheme(), "http" | "https") {
        bail!("Unsupported URL scheme '{}'", parsed.scheme());
    }
    if !parsed.username().is_empty() || parsed.password().is_some() {
        bail!("Credentials must not be embedded in the MCP URL");
    }

    let host = parsed.host_str().context("Missing host in MCP URL")?;
    let normalized = host
        .trim_start_matches('[')
        .trim_end_matches(']')
        .to_ascii_lowercase();

    // Block cloud metadata endpoints.
    if matches!(
        normalized.as_str(),
        "metadata.google.internal" | "metadata.azure.internal" | "metadata" | "169.254.169.254"
    ) {
        bail!("Cloud metadata endpoints are not allowed");
    }

    // If host is an IP literal, enforce scope rules.
    if let Ok(ip) = normalized.parse::<std::net::IpAddr>()
        && !ip_allowed(&ip)
    {
        bail!("Destination address is not allowed");
    }

    Ok(trimmed.to_string())
}

fn ip_allowed(ip: &std::net::IpAddr) -> bool {
    match ip {
        std::net::IpAddr::V4(v4) => {
            if v4.is_unspecified() || v4.is_multicast() || v4.is_broadcast() {
                return false;
            }
            // link-local
            if v4.is_link_local() {
                return false;
            }
            // private ranges (RFC1918 + CGNAT)
            let o = v4.octets();
            if o[0] == 10 {
                return false;
            }
            if o[0] == 172 && (16..=31).contains(&o[1]) {
                return false;
            }
            if o[0] == 192 && o[1] == 168 {
                return false;
            }
            if o[0] == 100 && (64..=127).contains(&o[1]) {
                return false;
            }
            // cloud metadata (already handled but keep)
            if *v4 == std::net::Ipv4Addr::new(169, 254, 169, 254) {
                return false;
            }
            true
        }
        std::net::IpAddr::V6(v6) => {
            if v6.is_unspecified() || v6.is_multicast() {
                return false;
            }
            // unique local fc00::/7
            let seg = v6.segments()[0];
            if seg & 0xfe00 == 0xfc00 {
                return false;
            }
            // link-local fe80::/10
            if seg & 0xffc0 == 0xfe80 {
                return false;
            }
            true
        }
    }
}

// ---------------------------------------------------------------------------
// Stdio transport
// ---------------------------------------------------------------------------

pub struct StdioTransport {
    state: Arc<Mutex<StdioState>>,
    timeout: Duration,
    next_id: AtomicI64,
}

struct StdioState {
    child: Child,
    stdin: tokio::process::ChildStdin,
    stdout: BufReader<tokio::process::ChildStdout>,
}

impl StdioTransport {
    pub fn new(
        command: impl Into<String>,
        args: Vec<String>,
        env: HashMap<String, String>,
        timeout: Duration,
    ) -> Result<Self> {
        let command = command.into();
        let mut cmd = Command::new(&command);
        cmd.args(&args)
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .kill_on_drop(true);
        cmd.env_clear();
        if let Ok(path) = std::env::var("PATH") {
            cmd.env("PATH", path);
        }
        for (k, v) in &env {
            cmd.env(k, v);
        }
        #[cfg(unix)]
        {
            cmd.process_group(0);
        }
        let mut child = cmd.spawn().context("Failed to spawn MCP stdio process")?;
        let stdin = child.stdin.take().context("missing stdin")?;
        let stdout = child.stdout.take().context("missing stdout")?;
        let state = StdioState {
            child,
            stdin,
            stdout: BufReader::new(stdout),
        };
        Ok(Self {
            state: Arc::new(Mutex::new(state)),
            timeout,
            next_id: AtomicI64::new(1),
        })
    }
}

#[async_trait]
impl McpTransport for StdioTransport {
    async fn request(&self, method: &str, params: Option<Value>) -> Result<Value> {
        let id = self.next_id.fetch_add(1, Ordering::SeqCst);
        let req = JsonRpcRequest {
            jsonrpc: "2.0",
            id,
            method: method.to_string(),
            params,
        };
        let mut line = serde_json::to_string(&req)?;
        line.push('\n');

        let mut state = self.state.lock().await;
        state.stdin.write_all(line.as_bytes()).await?;
        state.stdin.flush().await?;

        let deadline = tokio::time::timeout(self.timeout, async {
            loop {
                let mut buf = String::new();
                let n = state.stdout.read_line(&mut buf).await?;
                if n == 0 {
                    bail!("MCP stdio server closed stdout");
                }
                let trimmed = buf.trim();
                if trimmed.is_empty() {
                    continue;
                }
                let resp: JsonRpcResponse = match serde_json::from_str(trimmed) {
                    Ok(r) => r,
                    Err(_) => continue, // skip non-JSON
                };
                if resp.id.as_ref().and_then(|v| v.as_i64()) != Some(id) {
                    continue; // notification or other
                }
                if let Some(err) = resp.error {
                    return Err(err.to_anyhow());
                }
                return Ok(resp.result.unwrap_or(Value::Null));
            }
        })
        .await
        .map_err(|_| anyhow::anyhow!("MCP stdio request timed out"))??;

        Ok(deadline)
    }

    async fn notify(&self, method: &str, params: Option<Value>) -> Result<()> {
        let req = json!({
            "jsonrpc": "2.0",
            "method": method,
            "params": params,
        });
        let mut line = serde_json::to_string(&req)?;
        line.push('\n');
        let mut state = self.state.lock().await;
        state.stdin.write_all(line.as_bytes()).await?;
        state.stdin.flush().await?;
        Ok(())
    }

    async fn close(&self) -> Result<()> {
        let mut state = self.state.lock().await;
        let _ = state.child.kill().await;
        Ok(())
    }
}

// ---------------------------------------------------------------------------
// Streamable HTTP transport
// ---------------------------------------------------------------------------

#[allow(dead_code)]
pub struct StreamableHttpTransport {
    client: reqwest::Client,
    url: String,
    timeout: Duration,
    next_id: AtomicI64,
}

impl StreamableHttpTransport {
    pub fn new(url: impl Into<String>, timeout: Duration) -> Result<Self> {
        let url = validate_mcp_url(&url.into())?;
        let client = reqwest::Client::builder()
            .timeout(timeout)
            .redirect(reqwest::redirect::Policy::none())
            .build()
            .context("Failed to build HTTP client")?;
        Ok(Self {
            client,
            url,
            timeout,
            next_id: AtomicI64::new(1),
        })
    }
}

#[async_trait]
impl McpTransport for StreamableHttpTransport {
    async fn request(&self, method: &str, params: Option<Value>) -> Result<Value> {
        validate_mcp_url(&self.url)?;
        let id = self.next_id.fetch_add(1, Ordering::SeqCst);
        let req = JsonRpcRequest {
            jsonrpc: "2.0",
            id,
            method: method.to_string(),
            params,
        };

        let resp = self
            .client
            .post(&self.url)
            .header("Content-Type", "application/json")
            .header("Accept", "application/json, text/event-stream")
            .json(&req)
            .send()
            .await
            .context("MCP HTTP request failed")?;

        if !resp.status().is_success() {
            bail!("MCP HTTP returned status {}", resp.status());
        }

        let content_type = resp
            .headers()
            .get("content-type")
            .and_then(|v| v.to_str().ok())
            .unwrap_or("")
            .to_string();

        let body = resp.text().await.context("Failed to read MCP HTTP body")?;
        if body.len() > 4 * 1024 * 1024 {
            bail!("MCP HTTP response too large");
        }

        if content_type.contains("text/event-stream") {
            // Parse SSE frames.
            for block in body.split("\n\n") {
                for line in block.lines() {
                    if let Some(data) = line.strip_prefix("data:") {
                        let data = data.trim();
                        if data.is_empty() {
                            continue;
                        }
                        if let Ok(parsed) = serde_json::from_str::<JsonRpcResponse>(data)
                            && parsed.id.as_ref().and_then(|v| v.as_i64()) == Some(id)
                        {
                            if let Some(err) = parsed.error {
                                return Err(err.to_anyhow());
                            }
                            return Ok(parsed.result.unwrap_or(Value::Null));
                        }
                    }
                }
            }
            bail!("No matching JSON-RPC response in SSE stream");
        } else {
            let parsed: JsonRpcResponse =
                serde_json::from_str(&body).context("Invalid MCP HTTP JSON response")?;
            if parsed.id.as_ref().and_then(|v| v.as_i64()) != Some(id) && parsed.id.is_some() {
                // still accept
            }
            if let Some(err) = parsed.error {
                return Err(err.to_anyhow());
            }
            Ok(parsed.result.unwrap_or(Value::Null))
        }
    }

    async fn notify(&self, method: &str, params: Option<Value>) -> Result<()> {
        let body = json!({"jsonrpc":"2.0","method":method,"params":params});
        let _ = self
            .client
            .post(&self.url)
            .header("Content-Type", "application/json")
            .json(&body)
            .send()
            .await;
        Ok(())
    }

    async fn close(&self) -> Result<()> {
        Ok(())
    }
}

// ---------------------------------------------------------------------------
// SSE transport
// ---------------------------------------------------------------------------

pub struct SseTransport {
    client: reqwest::Client,
    sse_url: String,
    post_url: Arc<RwLock<String>>,
    pending: Arc<Mutex<HashMap<i64, oneshot::Sender<Result<Value>>>>>,
    timeout: Duration,
    next_id: AtomicI64,
    reader_handle: Option<tokio::task::JoinHandle<()>>,
    #[allow(dead_code)]
    notification_tx: Option<tokio::sync::mpsc::UnboundedSender<Value>>,
    notification_slot: Arc<std::sync::RwLock<Option<tokio::sync::mpsc::UnboundedSender<Value>>>>,
}

impl SseTransport {
    pub fn with_notification_tx(
        url: impl Into<String>,
        timeout: Duration,
        notification_tx: Option<tokio::sync::mpsc::UnboundedSender<Value>>,
    ) -> Result<Self> {
        let sse_url = validate_mcp_url(&url.into())?;
        let client = reqwest::Client::builder()
            .timeout(timeout)
            .redirect(reqwest::redirect::Policy::none())
            .build()
            .context("Failed to build HTTP client")?;

        let post_url = Arc::new(RwLock::new(sse_url.clone()));
        let pending: Arc<Mutex<HashMap<i64, oneshot::Sender<Result<Value>>>>> =
            Arc::new(Mutex::new(HashMap::new()));
        let notification_slot = Arc::new(std::sync::RwLock::new(notification_tx.clone()));

        // Spawn background reader.
        let client_c = client.clone();
        let sse_url_c = sse_url.clone();
        let post_url_c = post_url.clone();
        let pending_c = pending.clone();
        let slot_c = notification_slot.clone();
        let handle = tokio::spawn(async move {
            let resp = match client_c
                .get(&sse_url_c)
                .header("Accept", "text/event-stream")
                .send()
                .await
            {
                Ok(r) => r,
                Err(e) => {
                    warn!("SSE connect failed: {e}");
                    return;
                }
            };
            let mut stream = resp.bytes_stream();
            use futures::StreamExt;
            let mut buffer = String::new();
            while let Some(chunk) = stream.next().await {
                let chunk = match chunk {
                    Ok(c) => c,
                    Err(e) => {
                        warn!("SSE stream error: {e}");
                        break;
                    }
                };
                buffer.push_str(&String::from_utf8_lossy(&chunk));
                while let Some(pos) = buffer.find("\n\n") {
                    let event_block: String = buffer.drain(..=pos + 1).collect();
                    // parse
                    let mut event_type = String::from("message");
                    let mut data_lines: Vec<String> = Vec::new();
                    for line in event_block.lines() {
                        if let Some(rest) = line.strip_prefix("event:") {
                            event_type = rest.trim().to_string();
                        } else if let Some(rest) = line.strip_prefix("data:") {
                            data_lines.push(rest.trim_start().to_string());
                        }
                    }
                    let data = data_lines.join("\n");
                    if data.is_empty() {
                        continue;
                    }
                    if event_type == "endpoint" {
                        // resolve relative
                        if let Ok(base) = Url::parse(&sse_url_c) {
                            if let Ok(resolved) = base.join(&data) {
                                let resolved_str = resolved.to_string();
                                if validate_mcp_url(&resolved_str).is_ok() {
                                    *post_url_c.write().await = resolved_str;
                                }
                            }
                        } else if validate_mcp_url(&data).is_ok() {
                            *post_url_c.write().await = data;
                        }
                        continue;
                    }
                    // Dispatch notifications: events lacking an id or carrying a method.
                    if let Ok(value) = serde_json::from_str::<Value>(&data) {
                        let is_notification =
                            value.get("id").is_none() || value.get("method").is_some();
                        if is_notification
                            && let Ok(guard) = slot_c.read()
                            && let Some(tx) = guard.as_ref()
                        {
                            let _ = tx.send(value.clone());
                        }
                    }

                    // try parse JSON-RPC response
                    if let Ok(resp) = serde_json::from_str::<JsonRpcResponse>(&data)
                        && let Some(idv) = resp.id.clone()
                        && let Some(id) = idv.as_i64()
                    {
                        let mut guard = pending_c.lock().await;
                        if let Some(tx) = guard.remove(&id) {
                            let result = if let Some(err) = resp.error {
                                Err(err.to_anyhow())
                            } else {
                                Ok(resp.result.unwrap_or(Value::Null))
                            };
                            let _ = tx.send(result);
                        }
                    }
                }
            }
        });

        Ok(Self {
            client,
            sse_url,
            post_url,
            pending,
            timeout,
            next_id: AtomicI64::new(1),
            reader_handle: Some(handle),
            notification_tx,
            notification_slot,
        })
    }

    pub fn new(url: impl Into<String>, timeout: Duration) -> Result<Self> {
        Self::with_notification_tx(url, timeout, None)
    }

    pub fn new_with_notification_tx(
        url: impl Into<String>,
        timeout: Duration,
        notification_tx: Option<tokio::sync::mpsc::UnboundedSender<Value>>,
    ) -> Result<Self> {
        Self::with_notification_tx(url, timeout, notification_tx)
    }

    pub fn subscribe_notifications(&self) -> tokio::sync::mpsc::UnboundedReceiver<Value> {
        let (tx, rx) = tokio::sync::mpsc::unbounded_channel();
        if let Ok(mut guard) = self.notification_slot.write() {
            *guard = Some(tx);
        }
        rx
    }
}

#[async_trait]
impl McpTransport for SseTransport {
    async fn request(&self, method: &str, params: Option<Value>) -> Result<Value> {
        validate_mcp_url(&self.sse_url)?;
        let id = self.next_id.fetch_add(1, Ordering::SeqCst);
        let req = JsonRpcRequest {
            jsonrpc: "2.0",
            id,
            method: method.to_string(),
            params,
        };

        let (tx, rx) = oneshot::channel();
        {
            let mut guard = self.pending.lock().await;
            guard.insert(id, tx);
        }

        let post_url = self.post_url.read().await.clone();
        let post_url = if post_url.is_empty() {
            self.sse_url.clone()
        } else {
            post_url
        };
        validate_mcp_url(&post_url)?;

        let resp = self
            .client
            .post(&post_url)
            .header("Content-Type", "application/json")
            .json(&req)
            .send()
            .await
            .context("MCP SSE POST failed")?;
        // Accept any 2xx; response body may be empty.
        if !resp.status().is_success() && resp.status().as_u16() != 202 {
            // still continue to wait
        }

        let result = tokio::time::timeout(self.timeout, rx)
            .await
            .map_err(|_| anyhow::anyhow!("MCP SSE request timed out"))?
            .map_err(|_| anyhow::anyhow!("MCP SSE response channel closed"))??;
        Ok(result)
    }

    async fn notify(&self, method: &str, params: Option<Value>) -> Result<()> {
        let body = json!({"jsonrpc":"2.0","method":method,"params":params});
        let post_url = self.post_url.read().await.clone();
        let post_url = if post_url.is_empty() {
            self.sse_url.clone()
        } else {
            post_url
        };
        let _ = self.client.post(&post_url).json(&body).send().await;
        Ok(())
    }

    async fn close(&self) -> Result<()> {
        if let Some(h) = &self.reader_handle {
            h.abort();
        }
        Ok(())
    }
}

// ---------------------------------------------------------------------------
// McpClient
// ---------------------------------------------------------------------------

#[allow(dead_code)]
pub struct McpClient {
    transport: Arc<dyn McpTransport>,
    timeout: Duration,
}

impl McpClient {
    pub fn new(transport: Arc<dyn McpTransport>, timeout: Duration) -> Self {
        Self { transport, timeout }
    }

    pub fn from_transport(transport: Arc<dyn McpTransport>) -> Self {
        Self {
            transport,
            timeout: Duration::from_secs(120),
        }
    }

    pub async fn close(&self) -> Result<()> {
        self.transport.close().await
    }

    pub async fn connect(config: McpServerConfig) -> Result<Self> {
        let timeout = config.timeout;
        let transport: Arc<dyn McpTransport> = match config.kind.as_str() {
            "stdio" => {
                let command = config
                    .command
                    .clone()
                    .ok_or_else(|| anyhow::anyhow!("stdio extension missing command"))?;
                let t = StdioTransport::new(command, config.args, config.env, timeout)?;
                Arc::new(t)
            }
            "streamable_http" => {
                let url = config
                    .url
                    .clone()
                    .ok_or_else(|| anyhow::anyhow!("http extension missing url"))?;
                let t = StreamableHttpTransport::new(url, timeout)?;
                Arc::new(t)
            }
            "sse" => {
                let url = config
                    .url
                    .clone()
                    .ok_or_else(|| anyhow::anyhow!("sse extension missing url"))?;
                let t = SseTransport::new(url, timeout)?;
                Arc::new(t)
            }
            other => bail!("Unsupported MCP transport kind '{}'", other),
        };
        Ok(Self { transport, timeout })
    }

    pub async fn connect_stdio(
        command: impl Into<String>,
        args: Vec<String>,
        env: HashMap<String, String>,
        timeout: Duration,
    ) -> Result<Self> {
        let t = StdioTransport::new(command, args, env, timeout)?;
        Ok(Self {
            transport: Arc::new(t),
            timeout,
        })
    }

    pub async fn connect_streamable_http(
        url: impl Into<String>,
        timeout: Duration,
    ) -> Result<Self> {
        let t = StreamableHttpTransport::new(url, timeout)?;
        Ok(Self {
            transport: Arc::new(t),
            timeout,
        })
    }

    pub async fn connect_sse(url: impl Into<String>, timeout: Duration) -> Result<Self> {
        let t = SseTransport::new(url, timeout)?;
        Ok(Self {
            transport: Arc::new(t),
            timeout,
        })
    }

    pub async fn from_extension(
        ext: &Extension,
        timeout_override: Option<Duration>,
    ) -> Result<Self> {
        Self::connect(McpServerConfig::from_extension(ext, timeout_override)).await
    }

    pub async fn initialize(&self) -> Result<McpInitializeResult> {
        let params = json!({
            "protocolVersion": "2024-11-05",
            "capabilities": {},
            "clientInfo": {"name": "gaggle", "version": env!("CARGO_PKG_VERSION")}
        });
        let result = self.transport.request("initialize", Some(params)).await?;
        let protocol_version = result
            .get("protocolVersion")
            .and_then(|v| v.as_str())
            .unwrap_or("unknown")
            .to_string();
        let capabilities = result.get("capabilities").cloned().unwrap_or(Value::Null);
        let server_info = result.get("serverInfo").cloned().unwrap_or(Value::Null);

        // Validate capabilities: require tools capability.
        if capabilities.get("tools").is_none() {
            // Some servers omit; allow but warn.
            warn!("MCP server did not advertise tools capability");
        }

        Ok(McpInitializeResult {
            protocol_version,
            capabilities,
            server_info,
        })
    }

    pub async fn list_tools(&self) -> Result<Vec<McpTool>> {
        let result = self
            .transport
            .request("tools/list", Some(json!({})))
            .await?;
        let tools = result
            .get("tools")
            .and_then(|v| v.as_array())
            .cloned()
            .unwrap_or_default();

        let mut out = Vec::new();
        for t in tools {
            let name = t
                .get("name")
                .and_then(|v| v.as_str())
                .ok_or_else(|| anyhow::anyhow!("MCP tool missing name"))?
                .to_string();
            let description = t
                .get("description")
                .and_then(|v| v.as_str())
                .map(|s| s.to_string());
            let input_schema = t
                .get("inputSchema")
                .cloned()
                .unwrap_or_else(|| json!({"type":"object"}));
            let is_read_only = t
                .get("annotations")
                .and_then(|a| a.get("readOnlyHint"))
                .and_then(|v| v.as_bool())
                .unwrap_or(false);
            out.push(McpTool {
                name,
                description,
                input_schema,
                is_read_only,
            });
        }
        Ok(out)
    }

    pub async fn call_tool(&self, name: &str, arguments: Value) -> Result<McpToolResult> {
        let params = json!({"name": name, "arguments": arguments});
        let result = self.transport.request("tools/call", Some(params)).await?;
        let content = result
            .get("content")
            .and_then(|v| v.as_array())
            .cloned()
            .unwrap_or_default();
        let is_error = result
            .get("isError")
            .and_then(|v| v.as_bool())
            .unwrap_or(false);
        Ok(McpToolResult {
            content,
            is_error,
            raw: result,
        })
    }

    pub async fn list_resources(&self) -> Result<Vec<McpResource>> {
        let result = self
            .transport
            .request("resources/list", Some(json!({})))
            .await?;
        let resources = result
            .get("resources")
            .and_then(|v| v.as_array())
            .cloned()
            .unwrap_or_default();
        let mut out = Vec::new();
        for r in resources {
            out.push(McpResource {
                uri: r
                    .get("uri")
                    .and_then(|v| v.as_str())
                    .unwrap_or_default()
                    .to_string(),
                name: r
                    .get("name")
                    .and_then(|v| v.as_str())
                    .map(|s| s.to_string()),
                description: r
                    .get("description")
                    .and_then(|v| v.as_str())
                    .map(|s| s.to_string()),
                mime_type: r
                    .get("mimeType")
                    .and_then(|v| v.as_str())
                    .map(|s| s.to_string()),
            });
        }
        Ok(out)
    }

    pub async fn read_resource(&self, uri: &str) -> Result<Vec<McpResourceContent>> {
        let params = json!({"uri": uri});
        let result = self
            .transport
            .request("resources/read", Some(params))
            .await?;
        let contents = result
            .get("contents")
            .and_then(|v| v.as_array())
            .cloned()
            .unwrap_or_default();
        let mut out = Vec::new();
        for c in contents {
            out.push(McpResourceContent {
                uri: c
                    .get("uri")
                    .and_then(|v| v.as_str())
                    .unwrap_or(uri)
                    .to_string(),
                mime_type: c
                    .get("mimeType")
                    .and_then(|v| v.as_str())
                    .map(|s| s.to_string()),
                text: c
                    .get("text")
                    .and_then(|v| v.as_str())
                    .map(|s| s.to_string()),
                blob: c
                    .get("blob")
                    .and_then(|v| v.as_str())
                    .map(|s| s.to_string()),
            });
        }
        Ok(out)
    }

    pub async fn list_prompts(&self) -> Result<Vec<McpPrompt>> {
        let result = self
            .transport
            .request("prompts/list", Some(json!({})))
            .await?;
        let prompts = result
            .get("prompts")
            .and_then(|v| v.as_array())
            .cloned()
            .unwrap_or_default();
        let mut out = Vec::new();
        for p in prompts {
            let arguments = p
                .get("arguments")
                .and_then(|v| v.as_array())
                .cloned()
                .unwrap_or_default();
            out.push(McpPrompt {
                name: p
                    .get("name")
                    .and_then(|v| v.as_str())
                    .unwrap_or_default()
                    .to_string(),
                description: p
                    .get("description")
                    .and_then(|v| v.as_str())
                    .map(|s| s.to_string()),
                arguments,
            });
        }
        Ok(out)
    }

    pub async fn get_prompt(&self, name: &str, arguments: Value) -> Result<McpPromptResult> {
        let params = json!({"name": name, "arguments": arguments});
        let result = self.transport.request("prompts/get", Some(params)).await?;
        let messages = result
            .get("messages")
            .and_then(|v| v.as_array())
            .cloned()
            .unwrap_or_default();
        Ok(McpPromptResult {
            description: result
                .get("description")
                .and_then(|v| v.as_str())
                .map(|s| s.to_string()),
            messages,
            raw: result,
        })
    }

    // Typed resource/prompt helpers (JSON-RPC method wrappers).
    pub async fn resources_list(&self) -> Result<ResourcesListResponse> {
        let result = self
            .transport
            .request("resources/list", Some(json!({})))
            .await?;
        let resources = result
            .get("resources")
            .and_then(|v| v.as_array())
            .cloned()
            .unwrap_or_default();
        let mut out = Vec::new();
        for r in resources {
            out.push(McpResource {
                uri: r.get("uri").and_then(|v| v.as_str()).unwrap_or_default().to_string(),
                name: r.get("name").and_then(|v| v.as_str()).map(|s| s.to_string()),
                description: r.get("description").and_then(|v| v.as_str()).map(|s| s.to_string()),
                mime_type: r.get("mimeType").and_then(|v| v.as_str()).map(|s| s.to_string()),
            });
        }
        let next_cursor = result.get("nextCursor").and_then(|v| v.as_str()).map(|s| s.to_string());
        Ok(ResourcesListResponse { resources: out, next_cursor })
    }

    pub async fn resources_list_with<R>(&self, _req: R) -> Result<ResourcesListResponse>
    where
        R: Into<ResourcesListRequest>,
    {
        let _ = _req.into();
        self.resources_list().await
    }

    pub async fn list_resources_typed(&self, _req: ListResourcesRequest) -> Result<ListResourcesResponse> {
        let r = self.resources_list().await?;
        Ok(ListResourcesResponse {
            resources: r.resources,
            next_cursor: r.next_cursor,
        })
    }

    pub async fn resources_read<R>(&self, request: R) -> Result<ReadResourceResponse>
    where
        R: Into<ReadResourceRequest>,
    {
        let req = request.into();
        let params = json!({"uri": req.uri});
        let result = self.transport.request("resources/read", Some(params)).await?;
        let contents = result
            .get("contents")
            .and_then(|v| v.as_array())
            .cloned()
            .unwrap_or_default();
        let mut out = Vec::new();
        for c in contents {
            out.push(McpResourceContent {
                uri: c.get("uri").and_then(|v| v.as_str()).unwrap_or(&req.uri).to_string(),
                mime_type: c.get("mimeType").and_then(|v| v.as_str()).map(|s| s.to_string()),
                text: c.get("text").and_then(|v| v.as_str()).map(|s| s.to_string()),
                blob: c.get("blob").and_then(|v| v.as_str()).map(|s| s.to_string()),
            });
        }
        Ok(ReadResourceResponse { contents: out })
    }

    pub async fn read_resource_typed(&self, req: ReadResourceRequest) -> Result<ReadResourceResponse> {
        self.resources_read(req).await
    }

    pub async fn resources_read_request(&self, req: ReadResourceRequest) -> Result<ReadResourceResponse> {
        self.resources_read(req).await
    }

    pub async fn read_resource_request(&self, req: ReadResourceRequest) -> Result<ReadResourceResponse> {
        self.resources_read(req).await
    }

    pub async fn prompts_list(&self) -> Result<PromptsListResponse> {
        let result = self
            .transport
            .request("prompts/list", Some(json!({})))
            .await?;
        let prompts = result
            .get("prompts")
            .and_then(|v| v.as_array())
            .cloned()
            .unwrap_or_default();
        let mut out = Vec::new();
        for p in prompts {
            out.push(McpPrompt {
                name: p.get("name").and_then(|v| v.as_str()).unwrap_or_default().to_string(),
                description: p.get("description").and_then(|v| v.as_str()).map(|s| s.to_string()),
                arguments: p.get("arguments").and_then(|v| v.as_array()).cloned().unwrap_or_default(),
            });
        }
        let next_cursor = result.get("nextCursor").and_then(|v| v.as_str()).map(|s| s.to_string());
        Ok(PromptsListResponse { prompts: out, next_cursor })
    }

    pub async fn prompts_list_with<R>(&self, _req: R) -> Result<PromptsListResponse>
    where
        R: Into<PromptsListRequest>,
    {
        let _ = _req.into();
        self.prompts_list().await
    }

    pub async fn list_prompts_typed(&self, _req: ListPromptsRequest) -> Result<ListPromptsResponse> {
        let r = self.prompts_list().await?;
        Ok(ListPromptsResponse {
            prompts: r.prompts,
            next_cursor: r.next_cursor,
        })
    }

    pub async fn prompts_get(&self, name: &str, arguments: Value) -> Result<McpPromptResult> {
        self.get_prompt(name, arguments).await
    }

    pub async fn prompts_get_request(&self, req: GetPromptRequest) -> Result<McpPromptResult> {
        self.get_prompt(&req.name, req.arguments).await
    }

    pub async fn get_prompt_request(&self, req: GetPromptRequest) -> Result<McpPromptResult> {
        self.prompts_get_request(req).await
    }

    pub async fn get_prompt_with(&self, req: GetPromptRequest) -> Result<McpPromptResult> {
        self.prompts_get_request(req).await
    }

    pub async fn prompts_get_with(&self, req: GetPromptRequest) -> Result<McpPromptResult> {
        self.prompts_get_request(req).await
    }

    pub async fn notify(&self, method: &str, params: Option<Value>) -> Result<()> {
        self.transport.notify(method, params).await
    }

    pub fn transport(&self) -> Arc<dyn McpTransport> {
        self.transport.clone()
    }
}

// Aliases for compatibility
pub type McpTransportRef = Arc<dyn McpTransport>;

// ---------------------------------------------------------------------------
// Extension registry / lifecycle
// ---------------------------------------------------------------------------

#[derive(Debug, Clone)]
pub struct InstallExtensionRequest {
    pub name: String,
    pub description: Option<String>,
    pub kind: String,
    pub version: String,
    pub source: String,
    pub command: Option<String>,
    pub args: Vec<String>,
    pub url: Option<String>,
    pub env_secret_refs: Option<Value>,
    pub timeout_secs: i64,
    pub manifest: Option<Value>,
    pub bundled: bool,
    pub enabled: bool,
}

pub struct ExtensionRegistry {
    pool: SqlitePool,
    connections: McpConnectionManager,
    resources: std::sync::Arc<tokio::sync::RwLock<std::collections::HashMap<String, Vec<McpResource>>>>,
    prompts: std::sync::Arc<tokio::sync::RwLock<std::collections::HashMap<String, Vec<McpPrompt>>>>,
}

impl ExtensionRegistry {
    pub fn new(pool: SqlitePool) -> Self {
        Self {
            pool: pool.clone(),
            connections: McpConnectionManager::new(),
            resources: std::sync::Arc::new(tokio::sync::RwLock::new(std::collections::HashMap::new())),
            prompts: std::sync::Arc::new(tokio::sync::RwLock::new(std::collections::HashMap::new())),
        }
    }

    pub fn with_connection_manager(pool: SqlitePool, connections: McpConnectionManager) -> Self {
        Self {
            pool: pool.clone(),
            connections,
            resources: std::sync::Arc::new(tokio::sync::RwLock::new(std::collections::HashMap::new())),
            prompts: std::sync::Arc::new(tokio::sync::RwLock::new(std::collections::HashMap::new())),
        }
    }

    pub async fn list_enabled_extensions(&self) -> Result<Vec<Extension>> {
        let repo = ExtensionRepository::new(self.pool.clone());
        repo.list_enabled_extensions().await
    }

    pub async fn list_active_tools_for_chat(&self, chat_id: &str) -> Result<Vec<ExtensionTool>> {
        let tools = sqlx::query_as::<_, ExtensionTool>(
            r#"
            SELECT et.* FROM extension_tools et
            INNER JOIN chat_extensions ce ON ce.extension_id = et.extension_id
            WHERE ce.chat_id = ? AND ce.enabled = 1 AND et.enabled = 1
            ORDER BY et.tool_name ASC
            "#,
        )
        .bind(chat_id)
        .fetch_all(&self.pool)
        .await?;
        Ok(tools)
    }

    pub fn exceeds_recommended_tool_limit(active_tool_count: usize) -> bool {
        active_tool_count > RECOMMENDED_MAX_ACTIVE_TOOLS
    }

    pub async fn install_extension(&self, req: InstallExtensionRequest) -> Result<Extension> {
        // Validate transport-specific fields.
        match req.kind.as_str() {
            "stdio" => {
                if req.command.is_none() {
                    bail!("stdio extension requires command");
                }
            }
            "streamable_http" | "sse" => {
                let url = req.url.as_ref().context("HTTP extension requires url")?;
                validate_mcp_url(url)?;
            }
            "builtin" | "wasm" => {}
            other => bail!("Unsupported extension kind '{}'", other),
        }

        let repo = ExtensionRepository::new(self.pool.clone());
        let args_json = if req.args.is_empty() {
            None
        } else {
            Some(serde_json::to_string(&req.args)?)
        };
        let env_json = req.env_secret_refs.map(|v| v.to_string());
        let manifest_json = req.manifest.map(|v| v.to_string());

        let ext = repo
            .create_extension(
                &req.name,
                req.description.as_deref(),
                &req.kind,
                &req.version,
                &req.source,
                req.command.as_deref(),
                args_json.as_deref(),
                req.url.as_deref(),
                env_json.as_deref(),
                if req.timeout_secs > 0 {
                    req.timeout_secs
                } else {
                    120
                },
                manifest_json.as_deref(),
                req.bundled,
                req.enabled,
            )
            .await?;

        repo.record_audit_log(
            "user",
            None,
            "extension.install",
            "extension",
            Some(&ext.id),
            Some(&json!({"name": ext.name, "kind": ext.kind}).to_string()),
        )
        .await?;

        Ok(ext)
    }

    pub async fn enable_extension(&self, id: &str) -> Result<Extension> {
        let repo = ExtensionRepository::new(self.pool.clone());
        let ext = repo.set_enabled(id, true).await?;
        repo.record_audit_log(
            "user",
            None,
            "extension.enable",
            "extension",
            Some(id),
            None,
        )
        .await?;
        Ok(ext)
    }

    pub async fn disable_extension(&self, id: &str) -> Result<Extension> {
        let repo = ExtensionRepository::new(self.pool.clone());
        let ext = repo.set_enabled(id, false).await?;
        repo.record_audit_log(
            "user",
            None,
            "extension.disable",
            "extension",
            Some(id),
            None,
        )
        .await?;
        Ok(ext)
    }

    pub async fn uninstall_extension(&self, id: &str) -> Result<()> {
        let repo = ExtensionRepository::new(self.pool.clone());
        repo.delete_extension(id).await?;
        repo.record_audit_log(
            "user",
            None,
            "extension.uninstall",
            "extension",
            Some(id),
            None,
        )
        .await?;
        Ok(())
    }

    pub async fn grant_permission(
        &self,
        extension_id: &str,
        permission: &str,
        scope: Option<&str>,
    ) -> Result<ExtensionPermission> {
        let repo = ExtensionRepository::new(self.pool.clone());
        let perm = repo
            .grant_permission(extension_id, permission, scope)
            .await?;
        repo.record_audit_log(
            "user",
            None,
            "extension.permission.grant",
            "extension",
            Some(extension_id),
            Some(&json!({"permission": permission, "scope": scope}).to_string()),
        )
        .await?;
        Ok(perm)
    }

    pub async fn revoke_permission(&self, extension_id: &str, permission: &str) -> Result<()> {
        let repo = ExtensionRepository::new(self.pool.clone());
        repo.revoke_permission(extension_id, permission).await?;
        repo.record_audit_log(
            "user",
            None,
            "extension.permission.revoke",
            "extension",
            Some(extension_id),
            Some(&json!({"permission": permission}).to_string()),
        )
        .await?;
        Ok(())
    }

    pub async fn list_permissions(&self, extension_id: &str) -> Result<Vec<ExtensionPermission>> {
        let repo = ExtensionRepository::new(self.pool.clone());
        repo.list_permissions(extension_id).await
    }

    pub async fn discover_tools(&self, extension_id: &str) -> Result<Vec<ExtensionTool>> {
        let repo = ExtensionRepository::new(self.pool.clone());
        let ext = repo
            .get_extension_by_id(extension_id)
            .await?
            .ok_or_else(|| anyhow::anyhow!("Extension {extension_id} not found"))?;

        let client = self.connections.get_or_connect(&ext).await?;
        let tools = client.list_tools().await?;

        let mut out = Vec::new();
        for t in tools {
            let schema_str = serde_json::to_string(&t.input_schema)?;
            let tool = repo
                .upsert_tool(
                    &ext.id,
                    &t.name,
                    t.description.as_deref().unwrap_or(""),
                    &schema_str,
                    t.is_read_only,
                )
                .await?;
            out.push(tool);
        }

        repo.record_audit_log(
            "system",
            None,
            "extension.tools.discover",
            "extension",
            Some(extension_id),
            Some(&json!({"count": out.len()}).to_string()),
        )
        .await?;

        Ok(out)
    }

    pub async fn invalidate_and_rediscover(&self, ext_id: &str) -> Result<Vec<ExtensionTool>> {
        // Invalidate any cached connection so the next discovery reconnects.
        let _ = self.connections.invalidate(ext_id).await;
        self.discover_tools(ext_id).await
    }

    pub async fn discover_tools_with_client(
        &self,
        extension_id: &str,
        client: &mut McpClient,
    ) -> Result<Vec<ExtensionTool>> {
        let repo = ExtensionRepository::new(self.pool.clone());
        let ext = repo
            .get_extension_by_id(extension_id)
            .await?
            .ok_or_else(|| anyhow::anyhow!("Extension {extension_id} not found"))?;

        client.initialize().await?;
        let tools = client.list_tools().await?;

        let mut out = Vec::new();
        for t in tools {
            let schema_str = serde_json::to_string(&t.input_schema)?;
            let tool = repo
                .upsert_tool(
                    &ext.id,
                    &t.name,
                    t.description.as_deref().unwrap_or(""),
                    &schema_str,
                    t.is_read_only,
                )
                .await?;
            out.push(tool);
        }
        Ok(out)
    }

    pub async fn discover_resources(&self, extension_id: &str) -> Result<Vec<McpResource>> {
        let repo = ExtensionRepository::new(self.pool.clone());
        let ext = repo
            .get_extension_by_id(extension_id)
            .await?
            .ok_or_else(|| anyhow::anyhow!("Extension {extension_id} not found"))?;

        let client = self.connections.get_or_connect(&ext).await?;
        let resources = client.list_resources().await?;

        let mut guard = self.resources.write().await;
        guard.insert(ext.id.clone(), resources.clone());
        Ok(resources)
    }

    pub async fn discover_prompts(&self, extension_id: &str) -> Result<Vec<McpPrompt>> {
        let repo = ExtensionRepository::new(self.pool.clone());
        let ext = repo
            .get_extension_by_id(extension_id)
            .await?
            .ok_or_else(|| anyhow::anyhow!("Extension {extension_id} not found"))?;

        let client = self.connections.get_or_connect(&ext).await?;
        let prompts = client.list_prompts().await?;

        let mut guard = self.prompts.write().await;
        guard.insert(ext.id.clone(), prompts.clone());
        Ok(prompts)
    }

    pub async fn discover_resources_with_client(
        &self,
        extension_id: &str,
        client: &mut McpClient,
    ) -> Result<Vec<McpResource>> {
        let repo = ExtensionRepository::new(self.pool.clone());
        let ext = repo
            .get_extension_by_id(extension_id)
            .await?
            .ok_or_else(|| anyhow::anyhow!("Extension {extension_id} not found"))?;

        client.initialize().await?;
        let resources = client.list_resources().await?;

        let mut guard = self.resources.write().await;
        guard.insert(ext.id.clone(), resources.clone());
        Ok(resources)
    }

    pub async fn discover_prompts_with_client(
        &self,
        extension_id: &str,
        client: &mut McpClient,
    ) -> Result<Vec<McpPrompt>> {
        let repo = ExtensionRepository::new(self.pool.clone());
        let ext = repo
            .get_extension_by_id(extension_id)
            .await?
            .ok_or_else(|| anyhow::anyhow!("Extension {extension_id} not found"))?;

        client.initialize().await?;
        let prompts = client.list_prompts().await?;

        let mut guard = self.prompts.write().await;
        guard.insert(ext.id.clone(), prompts.clone());
        Ok(prompts)
    }

    pub async fn invalidate_and_rediscover_resources(&self, ext_id: &str) -> Result<Vec<McpResource>> {
        let _ = self.connections.invalidate(ext_id).await;
        self.discover_resources(ext_id).await
    }

    pub async fn invalidate_and_rediscover_prompts(&self, ext_id: &str) -> Result<Vec<McpPrompt>> {
        let _ = self.connections.invalidate(ext_id).await;
        self.discover_prompts(ext_id).await
    }

    pub async fn store_resources(&self, extension_id: &str, resources: Vec<McpResource>) {
        let mut guard = self.resources.write().await;
        guard.insert(extension_id.to_string(), resources);
    }

    pub async fn stored_resources(&self, extension_id: &str) -> Vec<McpResource> {
        let guard = self.resources.read().await;
        guard.get(extension_id).cloned().unwrap_or_default()
    }

    pub async fn resources(&self, extension_id: &str) -> Vec<McpResource> {
        self.stored_resources(extension_id).await
    }

    pub async fn cached_resources(&self, extension_id: &str) -> Vec<McpResource> {
        self.stored_resources(extension_id).await
    }

    pub async fn get_resources(&self, extension_id: &str) -> Vec<McpResource> {
        self.stored_resources(extension_id).await
    }

    pub async fn list_resources_for_extension(&self, extension_id: &str) -> Vec<McpResource> {
        self.stored_resources(extension_id).await
    }

    pub async fn store_prompts(&self, extension_id: &str, prompts: Vec<McpPrompt>) {
        let mut guard = self.prompts.write().await;
        guard.insert(extension_id.to_string(), prompts);
    }

    pub async fn stored_prompts(&self, extension_id: &str) -> Vec<McpPrompt> {
        let guard = self.prompts.read().await;
        guard.get(extension_id).cloned().unwrap_or_default()
    }

    pub async fn prompts(&self, extension_id: &str) -> Vec<McpPrompt> {
        self.stored_prompts(extension_id).await
    }

    pub async fn cached_prompts(&self, extension_id: &str) -> Vec<McpPrompt> {
        self.stored_prompts(extension_id).await
    }

    pub async fn get_prompts(&self, extension_id: &str) -> Vec<McpPrompt> {
        self.stored_prompts(extension_id).await
    }

    pub async fn list_prompts_for_extension(&self, extension_id: &str) -> Vec<McpPrompt> {
        self.stored_prompts(extension_id).await
    }

    pub async fn enable_for_chat(&self, chat_id: &str, extension_id: &str) -> Result<()> {
        let repo = ExtensionRepository::new(self.pool.clone());
        repo.enable_extension_for_chat(chat_id, extension_id).await
    }

    pub async fn disable_for_chat(&self, chat_id: &str, extension_id: &str) -> Result<()> {
        let repo = ExtensionRepository::new(self.pool.clone());
        repo.disable_extension_for_chat(chat_id, extension_id).await
    }
}

// ---------------------------------------------------------------------------
// McpToolExecutor
// ---------------------------------------------------------------------------

pub struct McpToolExecutor {
    pool: SqlitePool,
    connections: McpConnectionManager,
}

impl McpToolExecutor {
    pub fn new(pool: SqlitePool) -> Self {
        Self {
            pool: pool.clone(),
            connections: McpConnectionManager::new(),
        }
    }

    pub fn with_connection_manager(pool: SqlitePool, connections: McpConnectionManager) -> Self {
        Self {
            pool: pool.clone(),
            connections,
        }
    }

    async fn client_for(&self, ext: &Extension) -> Result<Arc<McpClient>, String> {
        self.connections
            .get_or_connect(ext)
            .await
            .map_err(|e| e.to_string())
    }
}

#[async_trait]
impl ToolExecutor for McpToolExecutor {
    async fn execute_tool(
        &self,
        _run_id: &str,
        _chat: &gaggle_db::models::Chat,
        _project_root: &str,
        tool_name: &str,
        args_json: &str,
    ) -> Result<String, String> {
        let repo = ExtensionRepository::new(self.pool.clone());
        let tool = repo
            .find_enabled_tool_by_name(tool_name)
            .await
            .map_err(|e| e.to_string())?
            .ok_or_else(|| format!("MCP tool '{tool_name}' not found or disabled"))?;

        let ext = repo
            .get_extension_by_id(&tool.extension_id)
            .await
            .map_err(|e| e.to_string())?
            .ok_or_else(|| "Extension not found".to_string())?;

        // Capability check.
        let has_cap = repo
            .any_granted_permission(&ext.id)
            .await
            .map_err(|e| e.to_string())?;
        if !has_cap {
            return Err(format!("Extension '{}' lacks capability grants", ext.name));
        }

        let client = self.client_for(&ext).await?;

        let args: Value =
            serde_json::from_str(args_json).unwrap_or(Value::Object(Default::default()));
        let result = match client.call_tool(tool_name, args).await {
            Ok(result) => result,
            Err(err) => {
                // Drop a potentially stale connection so the next call reconnects.
                let _ = self.connections.invalidate(&ext.id).await;
                return Err(err.to_string());
            }
        };

        if result.is_error {
            return Err(result.text());
        }
        serde_json::to_string(&result.raw).map_err(|e| e.to_string())
    }
}

// Ensure unused imports not warnings
#[allow(dead_code)]
fn _use_now_ms() {
    let _ = now_ms();
    let _ = new_id();
}

// ---------------------------------------------------------------------------
// In-memory capability storage (separate from DB-backed ExtensionTool)
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Default)]
pub struct ExtensionCapabilities {
    pub extension_id: String,
    pub tools: Vec<ExtensionTool>,
    pub resources: Vec<McpResource>,
    pub prompts: Vec<McpPrompt>,
}

impl ExtensionCapabilities {
    pub fn new(extension_id: impl Into<String>) -> Self {
        Self {
            extension_id: extension_id.into(),
            tools: Vec::new(),
            resources: Vec::new(),
            prompts: Vec::new(),
        }
    }
}

#[derive(Clone, Default)]
pub struct McpCapabilityStore {
    resources: std::sync::Arc<tokio::sync::RwLock<std::collections::HashMap<String, Vec<McpResource>>>>,
    prompts: std::sync::Arc<tokio::sync::RwLock<std::collections::HashMap<String, Vec<McpPrompt>>>>,
}

impl McpCapabilityStore {
    pub fn new() -> Self {
        Self::default()
    }

    pub async fn store_resources(&self, extension_id: &str, resources: Vec<McpResource>) {
        let mut guard = self.resources.write().await;
        guard.insert(extension_id.to_string(), resources);
    }

    pub async fn resources(&self, extension_id: &str) -> Vec<McpResource> {
        let guard = self.resources.read().await;
        guard.get(extension_id).cloned().unwrap_or_default()
    }

    pub async fn get_resources(&self, extension_id: &str) -> Vec<McpResource> {
        self.resources(extension_id).await
    }

    pub async fn cached_resources(&self, extension_id: &str) -> Vec<McpResource> {
        self.resources(extension_id).await
    }

    pub async fn stored_resources(&self, extension_id: &str) -> Vec<McpResource> {
        self.resources(extension_id).await
    }

    pub async fn list_resources(&self, extension_id: &str) -> Vec<McpResource> {
        self.resources(extension_id).await
    }

    pub async fn store_prompts(&self, extension_id: &str, prompts: Vec<McpPrompt>) {
        let mut guard = self.prompts.write().await;
        guard.insert(extension_id.to_string(), prompts);
    }

    pub async fn prompts(&self, extension_id: &str) -> Vec<McpPrompt> {
        let guard = self.prompts.read().await;
        guard.get(extension_id).cloned().unwrap_or_default()
    }

    pub async fn get_prompts(&self, extension_id: &str) -> Vec<McpPrompt> {
        self.prompts(extension_id).await
    }

    pub async fn cached_prompts(&self, extension_id: &str) -> Vec<McpPrompt> {
        self.prompts(extension_id).await
    }

    pub async fn stored_prompts(&self, extension_id: &str) -> Vec<McpPrompt> {
        self.prompts(extension_id).await
    }

    pub async fn list_prompts(&self, extension_id: &str) -> Vec<McpPrompt> {
        self.prompts(extension_id).await
    }
}

// Aliases for storage helper naming variants.
pub type McpResourceStore = McpCapabilityStore;
pub type McpPromptStore = McpCapabilityStore;
pub type McpExtensionStore = McpCapabilityStore;
pub type ExtensionCapabilityStore = McpCapabilityStore;
pub type ExtensionResourceStore = McpCapabilityStore;
pub type ExtensionPromptStore = McpCapabilityStore;

// Convenience methods on McpConnectionManager (defined in connection_manager module).
impl McpConnectionManager {
    pub async fn list_resources<T>(&self, target: T) -> Result<Vec<McpResource>>
    where
        T: crate::connection_manager::ConnectTarget,
    {
        let client = self.get_or_connect(target).await?;
        client.list_resources().await
    }

    pub async fn read_resource<T>(&self, target: T, uri: &str) -> Result<Vec<McpResourceContent>>
    where
        T: crate::connection_manager::ConnectTarget,
    {
        let client = self.get_or_connect(target).await?;
        client.read_resource(uri).await
    }

    pub async fn list_prompts<T>(&self, target: T) -> Result<Vec<McpPrompt>>
    where
        T: crate::connection_manager::ConnectTarget,
    {
        let client = self.get_or_connect(target).await?;
        client.list_prompts().await
    }

    pub async fn get_prompt<T>(
        &self,
        target: T,
        name: &str,
        arguments: Value,
    ) -> Result<McpPromptResult>
    where
        T: crate::connection_manager::ConnectTarget,
    {
        let client = self.get_or_connect(target).await?;
        client.get_prompt(name, arguments).await
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn typed_structs_roundtrip() {
        let req = ReadResourceRequest { uri: "file:///a".into() };
        let json = serde_json::to_string(&req).unwrap();
        let back: ReadResourceRequest = serde_json::from_str(&json).unwrap();
        assert_eq!(back.uri, "file:///a");

        let prompt = GetPromptRequest {
            name: "summarize".into(),
            arguments: serde_json::json!({"x": 1}),
        };
        assert_eq!(prompt.name, "summarize");
    }

    #[tokio::test]
    async fn capability_store_roundtrip() {
        let store = McpCapabilityStore::new();
        store
            .store_resources("ext", vec![McpResource {
                uri: "u".into(),
                name: Some("r".into()),
                description: None,
                mime_type: None,
            }])
            .await;
        assert_eq!(store.resources("ext").await.len(), 1);
        assert!(store.resources("missing").await.is_empty());
    }
}

// Re-export extension registry alias
pub type McpExtensionRegistry = ExtensionRegistry;
