use serde::{Deserialize, Serialize};
use tracing::Span;

/// Correlation identifiers that tie a log entry, durable run, and tool call together.
///
/// The type is shared by the persistence, runtime, and server layers so a single value can be
/// propagated without re-parsing string identifiers. Missing identifiers are rendered as `-` in
/// logs so every structured entry still has a stable shape.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct CorrelationIds {
    pub chat_id: Option<String>,
    pub run_id: Option<String>,
    pub tool_call_id: Option<String>,
}

impl CorrelationIds {
    /// Creates a context anchored to a chat. An empty chat id is treated as unknown.
    pub fn for_chat(chat_id: impl Into<String>) -> Self {
        let chat_id = chat_id.into();
        Self {
            chat_id: (!chat_id.is_empty()).then_some(chat_id),
            run_id: None,
            tool_call_id: None,
        }
    }

    /// Adds the durable run identifier.
    pub fn with_run(mut self, run_id: impl Into<String>) -> Self {
        let run_id = run_id.into();
        self.run_id = (!run_id.is_empty()).then_some(run_id);
        self
    }

    /// Adds the tool call identifier.
    pub fn with_tool_call(mut self, tool_call_id: impl Into<String>) -> Self {
        let tool_call_id = tool_call_id.into();
        self.tool_call_id = (!tool_call_id.is_empty()).then_some(tool_call_id);
        self
    }

    /// Log-safe chat identifier (`-` when unknown).
    pub fn chat_id_field(&self) -> &str {
        self.chat_id.as_deref().unwrap_or("-")
    }

    /// Log-safe run identifier (`-` when unknown).
    pub fn run_id_field(&self) -> &str {
        self.run_id.as_deref().unwrap_or("-")
    }

    /// Log-safe tool call identifier (`-` when unknown).
    pub fn tool_call_id_field(&self) -> &str {
        self.tool_call_id.as_deref().unwrap_or("-")
    }

    /// Builds a structured span carrying all three correlation fields.
    ///
    /// Use this together with [`tracing::Instrument`] on asynchronous work so the fields are
    /// attached to every event emitted inside the operation without repeating them at each call
    /// site:
    ///
    /// ```ignore
    /// let span = ids.span("agent.run");
    /// async move { /* ... */ }.instrument(span).await;
    /// ```
    pub fn span(&self, operation: &'static str) -> Span {
        tracing::info_span!(
            "gaggle.operation",
            operation,
            chat_id = %self.chat_id_field(),
            run_id = %self.run_id_field(),
            tool_call_id = %self.tool_call_id_field(),
        )
    }
}

#[cfg(test)]
mod tests {
    use super::CorrelationIds;

    #[test]
    fn correlation_ids_render_unknown_fields_as_dash() {
        let ids = CorrelationIds::for_chat("chat-1").with_run("run-1");
        assert_eq!(ids.chat_id_field(), "chat-1");
        assert_eq!(ids.run_id_field(), "run-1");
        assert_eq!(ids.tool_call_id_field(), "-");

        let empty = CorrelationIds::default();
        assert_eq!(empty.chat_id_field(), "-");
        assert_eq!(empty.run_id_field(), "-");
        assert_eq!(empty.tool_call_id_field(), "-");
    }

    #[test]
    fn correlation_ids_ignore_empty_identifiers() {
        let ids = CorrelationIds::for_chat("").with_run("").with_tool_call("");
        assert_eq!(ids, CorrelationIds::default());
    }
}
