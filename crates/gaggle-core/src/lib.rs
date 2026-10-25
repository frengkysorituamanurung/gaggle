pub mod acp;
pub mod agent_loop;
pub mod code_mode;
pub mod compaction;
pub mod structured_summary;

pub use compaction::{CompactionEngine, CompactionResult, should_compact};
pub use structured_summary::{
    CompactionTemplates, FileActivity, StructuredSummary, Templates, auto_compact_threshold,
    format_message_for_compacting, render_template,
};
pub mod context_assembler;
pub mod credentials;
pub mod diagnostics;
pub mod errors;
pub mod observability;
pub mod permission_engine;
pub mod provider;
pub mod provider_oauth;
pub mod run_engine;
pub mod subagent;
pub mod subagent_manager;
pub mod subagents;
pub mod workspace_policy;

pub use acp::{AcpError, AcpMessage, AcpResponse, AcpTransport, InMemoryAcpTransport};
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
pub use diagnostics::{
    BuildIdentity, CapabilityState, CapabilityStatus, CoreHealth, DiagnosticReport,
    DiagnosticsExport, FailureSummary, HealthCheck, HealthStatus, collect_diagnostics,
    export_diagnostics,
};
pub use errors::{ActionableError, DataPreservation, FailureCategory};
pub use gaggle_db::CorrelationIds;
pub use observability::{
    MAX_ARG_VALUE_CHARS, MAX_ARGS_SUMMARY_CHARS, MAX_ERROR_MESSAGE_CHARS, RedactingMakeWriter,
    RedactingWriter, fingerprint_text, init_tracing, redact_log_text, summarize_tool_arguments,
};
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
pub use subagents::{
    DEFAULT_MAX_PARALLEL_SUBAGENTS, DEFAULT_SUBAGENT_DEPTH, DEFAULT_SUBAGENT_MAX_STEPS,
    SubagentConfig, SubagentManager, SubagentOutcome, SubagentSpawnRequest,
};
pub use workspace_policy::{WorkspacePathError, WorkspacePathPolicy};
