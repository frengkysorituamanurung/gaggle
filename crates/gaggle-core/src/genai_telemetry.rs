//! GenAI telemetry for OpenTelemetry integration.
//!
//! This module provides OpenTelemetry spans and metrics for GenAI operations
//! like model completions, tool calls, and agent runs.

use std::time::Instant;

/// A telemetry span for a GenAI operation.
pub struct GenAiSpan {
    pub operation: String,
    pub model: String,
    pub start: Instant,
    pub input_tokens: i64,
    pub output_tokens: i64,
    pub cached_tokens: i64,
    pub latency_ms: i64,
}

impl GenAiSpan {
    pub fn new(operation: &str, model: &str) -> Self {
        Self {
            operation: operation.to_string(),
            model: model.to_string(),
            start: Instant::now(),
            input_tokens: 0,
            output_tokens: 0,
            cached_tokens: 0,
            latency_ms: 0,
        }
    }

    pub fn record_usage(&mut self, input: i64, output: i64, cached: i64) {
        self.input_tokens = input;
        self.output_tokens = output;
        self.cached_tokens = cached;
    }

    pub fn finish(&mut self) {
        self.latency_ms = self.start.elapsed().as_millis() as i64;
    }

    /// Returns a JSON representation for logging.
    pub fn to_json(&self) -> serde_json::Value {
        serde_json::json!({
            "operation": self.operation,
            "model": self.model,
            "input_tokens": self.input_tokens,
            "output_tokens": self.output_tokens,
            "cached_tokens": self.cached_tokens,
            "latency_ms": self.latency_ms,
        })
    }
}

/// Telemetry collector.
pub struct TelemetryCollector {
    spans: Vec<GenAiSpan>,
}

impl TelemetryCollector {
    pub fn new() -> Self {
        Self { spans: Vec::new() }
    }

    pub fn record(&mut self, span: GenAiSpan) {
        self.spans.push(span);
    }

    pub fn spans(&self) -> &[GenAiSpan] {
        &self.spans
    }

    pub fn clear(&mut self) {
        self.spans.clear();
    }
}

impl Default for TelemetryCollector {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn genai_span_records_usage() {
        let mut span = GenAiSpan::new("chat", "gpt-4");
        span.record_usage(100, 50, 10);
        span.finish();
        assert_eq!(span.input_tokens, 100);
        assert_eq!(span.output_tokens, 50);
        assert!(span.latency_ms >= 0);
    }
}
