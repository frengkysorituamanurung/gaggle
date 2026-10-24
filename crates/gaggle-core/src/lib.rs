pub mod agent_loop;
pub mod code_mode;
pub mod compaction;

pub use compaction::{CompactionEngine, CompactionResult, should_compact};
pub mod context_assembler;
pub mod credentials;
pub mod diagnostics;
pub mod permission_engine;
pub mod provider;
pub mod provider_oauth;
pub mod run_engine;
pub mod workspace_policy;

pub use agent_loop::{
    ActiveRunRegistry, AgentLoopOutcome, AgentRuntime, CancellationToken, DEFAULT_MAX_AGENT_STEPS,
    ToolExecutor,
};
pub use context_assembler::{
    AgentsDocument, AssembledContext, ContextAssembler, ContextAssemblyOptions,
    DEFAULT_CONTEXT_WINDOW, DEFAULT_MAX_AGENTS_MD_BYTES, DEFAULT_MAX_AGENTS_MD_FILES,
    DEFAULT_MAX_HISTORY_MESSAGES, DEFAULT_MAX_MEMORIES, DEFAULT_MAX_TOOLS, discover_agents_md,
    estimate_tokens, redact_context_secrets,
};
pub use credentials::{
    CredentialError, CredentialStore, InMemoryCredentialStore, SecretString, redact_sensitive_text,
};
pub use diagnostics::{DiagnosticReport, collect_diagnostics};
pub use permission_engine::{PermissionDecision, PermissionEngine, ToolCallContext};
pub use provider::{
    AnthropicAdapter, MockProviderAdapter, ModelCompletionRequest, ModelCompletionResponse,
    ModelCompletionStream, ModelProviderAdapter, ModelStreamEvent, OllamaAdapter,
    OpenAiCompatibleAdapter, ProposedToolCall, ProviderError, ProviderMessage, TokenUsage,
    ToolDefinition, map_provider_http_error, validate_provider_base_url,
};
pub use provider_oauth::{
    ChatGptAdapter, ChatGptOAuthClient, ChatGptOAuthConfig, OAuthTokenSet, PkceAuthorization,
};
pub use run_engine::RunEngine;
pub use workspace_policy::{WorkspacePathError, WorkspacePathPolicy};
