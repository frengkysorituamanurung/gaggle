pub mod code_mode;
pub mod compaction;
pub mod credentials;
pub mod diagnostics;
pub mod permission_engine;
pub mod provider;
pub mod run_engine;
pub mod workspace_policy;

pub use credentials::{
    CredentialError, CredentialStore, InMemoryCredentialStore, SecretString, redact_sensitive_text,
};
pub use diagnostics::{DiagnosticReport, collect_diagnostics};
pub use permission_engine::{PermissionDecision, PermissionEngine, ToolCallContext};
pub use provider::{
    MockProviderAdapter, ModelCompletionRequest, ModelCompletionResponse, ModelProviderAdapter,
    OpenAiCompatibleAdapter, ProposedToolCall, ProviderError, ProviderMessage, TokenUsage,
    ToolDefinition, validate_provider_base_url,
};
pub use run_engine::RunEngine;
pub use workspace_policy::{WorkspacePathError, WorkspacePathPolicy};
