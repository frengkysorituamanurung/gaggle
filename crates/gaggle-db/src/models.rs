use serde::{Deserialize, Serialize};
use sqlx::FromRow;

// ============================================================================
// Domain 2: Configuration, Workspace, and Extensions
// ============================================================================

#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct Project {
    pub id: String,
    pub name: String,
    pub root_path: String,
    pub vcs_kind: String, // "git" | "none"
    pub default_branch: Option<String>,
    pub default_mode: String,   // "local" | "worktree" | "readonly"
    pub default_access: String, // "ask" | "full"
    pub default_model_id: Option<String>,
    pub archived_at: Option<i64>,
    pub created_at: i64,
    pub updated_at: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct ProjectSettings {
    pub project_id: String,
    pub instructions: Option<String>,
    pub ignore_globs: Option<String>, // JSON
    pub env: Option<String>,          // JSON
    pub shell: Option<String>,
    pub setup_command: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct Provider {
    pub id: String,
    pub kind: String, // "chatgpt"|"openrouter"|"openai"|"anthropic"|"deepseek"|"ollama"|"openai_compat"
    pub name: String,
    pub base_url: Option<String>,
    pub auth_kind: String, // "oauth" | "api_key" | "none"
    pub secret_ref: Option<String>,
    pub enabled: i64,
    pub created_at: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct Model {
    pub id: String,
    pub provider_id: String,
    pub model_key: String,
    pub display_name: String,
    pub context_window: i64,
    pub supports_tools: i64,
    pub supports_vision: i64,
    pub supports_reasoning: i64,
    pub price_in_per_mtok: Option<f64>,
    pub price_out_per_mtok: Option<f64>,
    pub enabled: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct Device {
    pub id: String,
    pub name: String,
    pub token_hash: String,
    pub last_ip: Option<String>,
    pub last_seen_at: Option<i64>,
    pub revoked_at: Option<i64>,
    pub created_at: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct AppSetting {
    pub key: String,
    pub value: String, // JSON
    pub updated_at: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct AuditLog {
    pub id: String,
    pub actor_kind: String, // "user" | "agent" | "extension" | "system"
    pub actor_id: Option<String>,
    pub action: String,
    pub entity_type: String,
    pub entity_id: Option<String>,
    pub metadata: Option<String>, // JSON
    pub created_at: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct Extension {
    pub id: String,
    pub name: String,
    pub description: Option<String>,
    pub kind: String, // "builtin" | "stdio" | "streamable_http" | "sse" | "wasm"
    pub version: String,
    pub source: String, // "bundled" | "registry" | "local" | "git"
    pub command: Option<String>,
    pub args: Option<String>, // JSON
    pub url: Option<String>,
    pub env_secret_refs: Option<String>, // JSON
    pub timeout_secs: i64,
    pub install_path: Option<String>,
    pub manifest: Option<String>, // JSON
    pub checksum: Option<String>,
    pub bundled: i64,
    pub enabled: i64,
    pub installed_at: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct ExtensionPermission {
    pub id: String,
    pub extension_id: String,
    pub permission: String, // "fs.read" | "fs.write" | "shell" | "net" | "browser" | "memory"
    pub scope: Option<String>,
    pub granted: i64,
    pub granted_at: Option<i64>,
}

#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct ExtensionTool {
    pub id: String,
    pub extension_id: String,
    pub tool_name: String,
    pub description: Option<String>,
    pub input_schema: Option<String>, // JSON
    pub is_read_only: i64,
    pub enabled: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct ExtensionKv {
    pub extension_id: String,
    pub key: String,
    pub value: String, // JSON
    pub updated_at: i64,
}

// ============================================================================
// Domain 3: Chat, Durable Run, Tool, and Approval
// ============================================================================

#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct Chat {
    pub id: String,
    pub project_id: String,
    pub model_id: Option<String>,
    pub worktree_id: Option<String>,
    pub parent_chat_id: Option<String>,
    pub parent_tool_call_id: Option<String>,
    pub recipe_id: Option<String>,
    pub schedule_id: Option<String>,
    pub chat_type: String, // "user" | "subagent" | "scheduled" | "hidden" | "terminal"
    pub title: String,
    pub title_user_set: i64,
    pub mode: String,                  // "local" | "worktree" | "readonly"
    pub permission_mode: String,       // "full" | "ask" | "smart" | "chat_only"
    pub recipe_values: Option<String>, // JSON
    pub compact_threshold: f64,
    pub system_prompt_override: Option<String>,
    pub total_input_tokens: i64,
    pub total_output_tokens: i64,
    pub pinned: i64,
    pub archived_at: Option<i64>,
    pub created_at: i64,
    pub updated_at: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct Message {
    pub id: String,
    pub chat_id: String,
    pub run_id: Option<String>,
    pub parent_id: Option<String>,
    pub seq: i64,
    pub role: String, // "user" | "assistant" | "system" | "tool"
    pub created_at: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct MessagePart {
    pub id: String,
    pub message_id: String,
    pub idx: i64,
    pub kind: String, // "text" | "reasoning" | "tool_call" | "tool_result" | "attachment"
    pub content: Option<String>,
    pub tool_call_id: Option<String>,
    pub attachment_id: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct Attachment {
    pub id: String,
    pub chat_id: String,
    pub file_name: String,
    pub mime: String,
    pub size_bytes: i64,
    pub sha256: String,
    pub storage_path: String,
    pub created_at: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct ChatSummary {
    pub id: String,
    pub chat_id: String,
    pub summary: String,
    pub covers_until_seq: i64,
    pub tokens_before: i64,
    pub tokens_after: i64,
    pub trigger: String, // "auto" | "manual"
    pub created_at: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct Run {
    pub id: String,
    pub chat_id: String,
    pub trigger_message_id: Option<String>,
    pub model_id: Option<String>,
    pub status: String, // "queued"|"running"|"waiting_approval"|"paused"|"completed"|"failed"|"cancelled"
    pub resume_count: i64,
    pub error: Option<String>,
    pub last_heartbeat_at: Option<i64>,
    pub started_at: Option<i64>,
    pub finished_at: Option<i64>,
}

#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct RunEvent {
    pub id: String,
    pub run_id: String,
    pub seq: i64,
    pub kind: String,
    pub payload: String, // JSON
    pub created_at: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct RunCheckpoint {
    pub id: String,
    pub run_id: String,
    pub step_index: i64,
    pub state: String, // JSON
    pub created_at: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct ToolCall {
    pub id: String,
    pub run_id: String,
    pub code_execution_id: Option<String>,
    pub extension_id: Option<String>,
    pub step_index: i64,
    pub source: String, // "code_mode" | "native" | "extension"
    pub tool_name: String,
    pub args: String,           // JSON
    pub result: Option<String>, // JSON
    pub status: String,         // "pending"|"awaiting_approval"|"running"|"ok"|"error"|"denied"
    pub started_at: Option<i64>,
    pub duration_ms: Option<i64>,
}

#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct CodeExecution {
    pub id: String,
    pub run_id: String,
    pub code: String,
    pub stdout: Option<String>,
    pub stderr: Option<String>,
    pub memory_limit_mb: i64,
    pub timeout_ms: i64,
    pub status: String, // "ok" | "error" | "timeout"
    pub started_at: i64,
    pub finished_at: Option<i64>,
}

#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct Approval {
    pub id: String,
    pub tool_call_id: String,
    pub kind: String, // "shell" | "fs_write" | "net" | "browser"
    pub summary: String,
    pub command: Option<String>,
    pub cwd: Option<String>,
    pub status: String, // "pending" | "approved" | "denied" | "expired"
    pub decided_by_device: Option<String>,
    pub remember_rule_id: Option<String>,
    pub decided_at: Option<i64>,
    pub created_at: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct CommandRule {
    pub id: String,
    pub project_id: Option<String>,
    pub pattern: String,
    pub match_kind: String, // "exact" | "prefix" | "regex"
    pub action: String,     // "allow" | "deny" | "ask"
    pub created_at: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct UsageRecord {
    pub id: String,
    pub run_id: String,
    pub model_id: Option<String>,
    pub input_tokens: i64,
    pub output_tokens: i64,
    pub cached_tokens: i64,
    pub cost_usd: f64,
    pub latency_ms: i64,
    pub created_at: i64,
}

// ============================================================================
// Domain 4: Git, Changes Panel, and Project Memory
// ============================================================================

#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct Worktree {
    pub id: String,
    pub project_id: String,
    pub chat_id: String,
    pub path: String,
    pub branch: String,
    pub base_ref: String,
    pub base_commit: String,
    pub status: String, // "active" | "merged" | "discarded"
    pub created_at: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct FileChange {
    pub id: String,
    pub run_id: String,
    pub chat_id: String,
    pub path: String,
    pub old_path: Option<String>,
    pub change_type: String, // "add" | "modify" | "delete" | "rename"
    pub additions: i64,
    pub deletions: i64,
    pub patch: Option<String>,
    pub before_sha: Option<String>,
    pub after_sha: Option<String>,
    pub status: String, // "pending" | "accepted" | "reverted"
    pub created_at: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct ReviewComment {
    pub id: String,
    pub file_change_id: String,
    pub author_device_id: Option<String>,
    pub line_start: i64,
    pub line_end: i64,
    pub side: String, // "old" | "new"
    pub body: String,
    pub status: String, // "open" | "sent_to_agent" | "resolved"
    pub created_at: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct Memory {
    pub id: String,
    pub project_id: Option<String>,
    pub source_chat_id: Option<String>,
    pub source_run_id: Option<String>,
    pub kind: String, // "fact" | "preference" | "decision" | "todo" | "summary"
    pub title: String,
    pub content: String,
    pub pinned: i64,
    pub confidence: f64,
    pub last_used_at: Option<i64>,
    pub created_at: i64,
    pub updated_at: i64,
    pub deleted_at: Option<i64>,
}

#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct MemoryTag {
    pub memory_id: String,
    pub tag: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct MemoryEmbedding {
    pub memory_id: String,
    pub embedding_model: String,
    pub dim: i64,
    pub vector: Vec<u8>,
    pub created_at: i64,
}

// ============================================================================
// Domain 5: Terminal and Browser Sandbox
// ============================================================================

#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct TerminalSession {
    pub id: String,
    pub project_id: String,
    pub chat_id: Option<String>,
    pub worktree_id: Option<String>,
    pub cwd: String,
    pub shell: String,
    pub cols: i64,
    pub rows: i64,
    pub pid: Option<i64>,
    pub status: String, // "running" | "exited"
    pub exit_code: Option<i64>,
    pub started_at: i64,
    pub ended_at: Option<i64>,
}

#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct BrowserSession {
    pub id: String,
    pub chat_id: String,
    pub run_id: Option<String>,
    pub profile_dir: String,
    pub headless: i64,
    pub status: String, // "running" | "closed" | "crashed"
    pub started_at: i64,
    pub ended_at: Option<i64>,
}

#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct BrowserAction {
    pub id: String,
    pub session_id: String,
    pub tool_call_id: Option<String>,
    pub seq: i64,
    pub action: String, // "navigate" | "click" | "type" | "scroll" | "screenshot" | "extract"
    pub url: Option<String>,
    pub selector: Option<String>,
    pub value: Option<String>,
    pub result: Option<String>, // JSON
    pub screenshot_attachment_id: Option<String>,
    pub created_at: i64,
}

// ============================================================================
// Domain 6: Recipe, Schedule, and Subagent
// ============================================================================

#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct Recipe {
    pub id: String,
    pub name: String,
    pub title: String,
    pub description: Option<String>,
    pub source_kind: String, // "inline" | "file" | "github" | "url"
    pub source_ref: Option<String>,
    pub instructions: Option<String>,
    pub prompt: Option<String>,
    pub definition: String,
    pub default_model_id: Option<String>,
    pub version: i64,
    pub created_at: i64,
    pub updated_at: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct RecipeParameter {
    pub id: String,
    pub recipe_id: String,
    pub name: String,
    pub param_type: String,
    pub description: Option<String>,
    pub required: i64,
    pub default_value: Option<String>,
    pub idx: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct RecipeSubRecipe {
    pub id: String,
    pub recipe_id: String,
    pub child_recipe_id: String,
    pub name: String,
    pub values_json: Option<String>, // JSON
    pub sequential_when_repeated: i64,
    pub idx: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct RecipeExtension {
    pub recipe_id: String,
    pub extension_id: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct Schedule {
    pub id: String,
    pub recipe_id: String,
    pub project_id: Option<String>,
    pub cron_expr: String,
    pub timezone: String,
    pub param_values: Option<String>, // JSON
    pub enabled: i64,
    pub last_run_at: Option<i64>,
    pub last_status: Option<String>, // "ok" | "error" | "skipped"
    pub next_run_at: Option<i64>,
    pub created_at: i64,
}

// ============================================================================
// Domain 7: Tool Permissions and Chat Extensions
// ============================================================================

#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct ToolPermission {
    pub id: String,
    pub project_id: Option<String>,
    pub extension_tool_id: String,
    pub level: String, // "always_allow" | "ask_before" | "never_allow"
    pub updated_at: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct ToolRiskCache {
    pub id: String,
    pub tool_name: String,
    pub args_fingerprint: String,
    pub is_read_only: i64,
    pub source: String, // "annotation" | "llm_judge" | "user"
    pub model_id: Option<String>,
    pub created_at: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct ChatExtension {
    pub chat_id: String,
    pub extension_id: String,
    pub enabled: i64,
    pub state: Option<String>, // JSON
    pub enabled_at: i64,
}
