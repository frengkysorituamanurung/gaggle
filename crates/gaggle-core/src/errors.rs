//! Actionable failure reporting.
//!
//! Gaggle failures are reported with four pieces of information so a user can decide what to do
//! next without reading internal logs:
//!
//! 1. the failure category,
//! 2. the operation that failed,
//! 3. whether local data was preserved,
//! 4. concrete retry guidance.
//!
//! Messages are redacted and bounded before they are stored on a run or returned to a client.

use crate::observability::{redact_log_text, truncate_for_message};
use crate::provider::ProviderError;
use gaggle_db::CorrelationIds;
use serde::{Deserialize, Serialize};
use std::fmt;

/// Coarse failure categories shown to users and included in structured logs and run events.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum FailureCategory {
    Configuration,
    Authentication,
    PermissionDenied,
    Network,
    ProviderTimeout,
    RateLimited,
    ProviderResponse,
    Tool,
    BudgetExhausted,
    Cancelled,
    Database,
    Internal,
}

impl FailureCategory {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Configuration => "configuration",
            Self::Authentication => "authentication",
            Self::PermissionDenied => "permission_denied",
            Self::Network => "network",
            Self::ProviderTimeout => "provider_timeout",
            Self::RateLimited => "rate_limited",
            Self::ProviderResponse => "provider_response",
            Self::Tool => "tool",
            Self::BudgetExhausted => "budget_exhausted",
            Self::Cancelled => "cancelled",
            Self::Database => "database",
            Self::Internal => "internal",
        }
    }

    /// Whether retrying the same operation can reasonably succeed without user changes.
    pub fn is_retryable(self) -> bool {
        matches!(
            self,
            Self::Network | Self::ProviderTimeout | Self::RateLimited | Self::ProviderResponse
        )
    }

    /// Default retry guidance for the category. Specific errors may override it.
    pub fn retry_guidance(self) -> &'static str {
        match self {
            Self::Configuration => {
                "Fix the reported configuration value, then retry the operation."
            }
            Self::Authentication => {
                "Open Settings → Providers & Models, re-enter or re-authorize the credential, then retry."
            }
            Self::PermissionDenied => {
                "Review the permission mode, command rules, or tool permissions and retry if the action should be allowed."
            }
            Self::Network => {
                "Check network connectivity and the provider endpoint, then retry the turn."
            }
            Self::ProviderTimeout => {
                "Check provider status or raise the provider timeout, then retry the turn."
            }
            Self::RateLimited => "Wait for the provider rate limit to reset, then retry the turn.",
            Self::ProviderResponse => {
                "Retry once. If the provider keeps returning an invalid response, switch models or report the provider."
            }
            Self::Tool => {
                "Review the tool arguments and the reported error, then retry the tool call."
            }
            Self::BudgetExhausted => {
                "Review the conversation, reduce the task scope, or continue with a new turn."
            }
            Self::Cancelled => {
                "Retry the turn when you are ready; no partial tool side effects are replayed automatically."
            }
            Self::Database => {
                "Retry the operation. If it persists, run `gaggle doctor` and export redacted diagnostics."
            }
            Self::Internal => {
                "Retry the operation. If it persists, run `gaggle doctor` and export redacted diagnostics."
            }
        }
    }
}

/// Whether local user data was preserved when an operation failed.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum DataPreservation {
    /// Chat history, run events, and checkpoints were committed before the failure.
    Preserved,
    /// A write may not have completed and the user should inspect before retrying.
    NotPreserved,
    /// The failure happened before a write was attempted, or the outcome is unknown.
    Unknown,
}

impl DataPreservation {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Preserved => "preserved",
            Self::NotPreserved => "not_preserved",
            Self::Unknown => "unknown",
        }
    }
}

/// A user-facing, non-sensitive description of a failed operation.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ActionableError {
    pub category: FailureCategory,
    pub operation: String,
    pub message: String,
    pub data_preservation: DataPreservation,
    pub retryable: bool,
    pub retry_guidance: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub chat_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub run_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tool_call_id: Option<String>,
}

impl ActionableError {
    /// Creates an error using the category's default retry guidance.
    pub fn new(
        category: FailureCategory,
        operation: impl Into<String>,
        message: impl Into<String>,
    ) -> Self {
        let message = message.into();
        Self {
            category,
            operation: operation.into(),
            message: truncate_for_message(&redact_log_text(&message)),
            data_preservation: DataPreservation::Unknown,
            retryable: category.is_retryable(),
            retry_guidance: category.retry_guidance().to_string(),
            chat_id: None,
            run_id: None,
            tool_call_id: None,
        }
    }

    pub fn with_data_preservation(mut self, status: DataPreservation) -> Self {
        self.data_preservation = status;
        self
    }

    pub fn with_retryable(mut self, retryable: bool) -> Self {
        self.retryable = retryable;
        self
    }

    pub fn with_retry_guidance(mut self, guidance: impl Into<String>) -> Self {
        self.retry_guidance = guidance.into();
        self
    }

    pub fn with_correlation(mut self, ids: &CorrelationIds) -> Self {
        self.chat_id = ids.chat_id.clone();
        self.run_id = ids.run_id.clone();
        self.tool_call_id = ids.tool_call_id.clone();
        self
    }

    /// Maps a normalized provider error to an actionable, redacted failure.
    pub fn from_provider_error(
        operation: impl Into<String>,
        error: &ProviderError,
        ids: &CorrelationIds,
    ) -> Self {
        let (category, guidance) = match error {
            ProviderError::InvalidBaseUrl { .. } => (
                FailureCategory::Configuration,
                Some(
                    "Fix the provider base URL in Settings → Providers & Models, then retry."
                        .to_string(),
                ),
            ),
            ProviderError::AuthenticationFailed { .. }
            | ProviderError::OAuth { .. }
            | ProviderError::Credential(_) => (FailureCategory::Authentication, None),
            ProviderError::RateLimited { retry_after_ms, .. } => (
                FailureCategory::RateLimited,
                retry_after_ms.map(|delay| {
                    format!(
                        "Wait about {delay} ms for the provider rate limit to reset, then retry the turn."
                    )
                }),
            ),
            ProviderError::Timeout { .. } => (FailureCategory::ProviderTimeout, None),
            ProviderError::MalformedResponse { .. } => (FailureCategory::ProviderResponse, None),
            ProviderError::Transport { .. } => (FailureCategory::Network, None),
        };

        let mut failure = Self::new(category, operation, error.to_string())
            .with_data_preservation(DataPreservation::Preserved)
            .with_correlation(ids);
        if let Some(guidance) = guidance {
            failure = failure.with_retry_guidance(guidance);
        }
        failure
    }

    /// Serializes the failure for a `run.failed` event or API response.
    pub fn to_json(&self) -> String {
        serde_json::to_string(self).unwrap_or_else(|_| {
            format!(
                "{{\"category\":\"{}\",\"operation\":\"{}\"}}",
                self.category.as_str(),
                self.operation
            )
        })
    }
}

impl fmt::Display for ActionableError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "category={} operation={} data_preservation={} retryable={}: {} Next step: {}",
            self.category.as_str(),
            self.operation,
            self.data_preservation.as_str(),
            self.retryable,
            self.message,
            self.retry_guidance
        )
    }
}

#[cfg(test)]
mod tests {
    use super::{ActionableError, DataPreservation, FailureCategory};
    use crate::provider::ProviderError;
    use gaggle_db::CorrelationIds;

    #[test]
    fn display_contains_category_operation_preservation_and_retry_guidance() {
        let failure = ActionableError::new(
            FailureCategory::Database,
            "run.persist",
            "database is locked",
        )
        .with_data_preservation(DataPreservation::Unknown);

        let text = failure.to_string();
        assert!(text.contains("category=database"));
        assert!(text.contains("operation=run.persist"));
        assert!(text.contains("data_preservation=unknown"));
        assert!(text.contains("retryable=false"));
        assert!(text.contains("gaggle doctor"));
    }

    #[test]
    fn provider_mapping_selects_category_and_preserves_correlation() {
        let ids = CorrelationIds::for_chat("chat-1")
            .with_run("run-1")
            .with_tool_call("call-1");
        let failure = ActionableError::from_provider_error(
            "agent.turn",
            &ProviderError::RateLimited {
                provider: "openai".to_string(),
                retry_after_ms: Some(2_000),
            },
            &ids,
        );

        assert_eq!(failure.category, FailureCategory::RateLimited);
        assert!(failure.retryable);
        assert_eq!(failure.data_preservation, DataPreservation::Preserved);
        assert_eq!(failure.chat_id.as_deref(), Some("chat-1"));
        assert_eq!(failure.run_id.as_deref(), Some("run-1"));
        assert_eq!(failure.tool_call_id.as_deref(), Some("call-1"));
        assert!(failure.retry_guidance.contains("2000 ms"));
    }

    #[test]
    fn messages_are_redacted_and_bounded() {
        let secret = "sk-live-DO-NOT-LEAK-1234567890";
        let long_tail = "z".repeat(4_000);
        let failure = ActionableError::new(
            FailureCategory::Internal,
            "agent.turn",
            format!("provider said {secret} {long_tail}"),
        );

        assert!(!failure.message.contains(secret));
        assert!(failure.message.contains("[REDACTED]"));
        assert!(failure.message.len() <= 600);
    }
}
