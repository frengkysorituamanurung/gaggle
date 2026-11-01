//! GenAI telemetry for OpenTelemetry integration.
//!
//! This module provides OpenTelemetry spans and metrics for GenAI operations
//! like model completions, tool calls, and agent runs.

use serde::Serialize;
use std::time::{Instant, SystemTime, UNIX_EPOCH};

/// A telemetry span for a GenAI operation.
#[derive(Debug, Clone, Serialize)]
pub struct GenAiSpan {
    pub operation: String,
    pub model: String,
    #[serde(skip)]
    pub start: Instant,
    pub start_time_unix_ms: i64,
    pub input_tokens: i64,
    pub output_tokens: i64,
    pub cached_tokens: i64,
    pub latency_ms: i64,
    pub status: String,
    pub trace_id: String,
    pub span_id: String,
}

impl GenAiSpan {
    pub fn new(operation: &str, model: &str) -> Self {
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map(|d| d.as_millis() as i64)
            .unwrap_or(0);
        Self {
            operation: operation.to_string(),
            model: model.to_string(),
            start: Instant::now(),
            start_time_unix_ms: now,
            input_tokens: 0,
            output_tokens: 0,
            cached_tokens: 0,
            latency_ms: 0,
            status: "OK".to_string(),
            trace_id: uuid::Uuid::new_v4().to_string().replace('-', ""),
            span_id: uuid::Uuid::new_v4().to_string()[..16].to_string(),
        }
    }

    pub fn record_usage(&mut self, input: i64, output: i64, cached: i64) {
        self.input_tokens = input;
        self.output_tokens = output;
        self.cached_tokens = cached;
    }

    pub fn set_status(&mut self, status: &str) {
        self.status = status.to_string();
    }

    pub fn finish(&mut self) {
        self.latency_ms = self.start.elapsed().as_millis() as i64;
    }

    /// Returns a JSON representation for logging.
    pub fn to_json(&self) -> serde_json::Value {
        serde_json::json!({
            "operation": self.operation,
            "model": self.model,
            "gen_ai.system": "gaggle",
            "gen_ai.request.model": self.model,
            "gen_ai.usage.input_tokens": self.input_tokens,
            "gen_ai.usage.output_tokens": self.output_tokens,
            "gen_ai.usage.cached_tokens": self.cached_tokens,
            "gen_ai.response.latency_ms": self.latency_ms,
            "input_tokens": self.input_tokens,
            "output_tokens": self.output_tokens,
            "cached_tokens": self.cached_tokens,
            "latency_ms": self.latency_ms,
            "status": self.status,
            "trace_id": self.trace_id,
            "span_id": self.span_id,
        })
    }

    /// Returns OpenTelemetry-compatible span attributes.
    pub fn to_otel_attributes(&self) -> Vec<(String, serde_json::Value)> {
        vec![
            ("gen_ai.system".to_string(), serde_json::json!("gaggle")),
            (
                "gen_ai.operation.name".to_string(),
                serde_json::json!(self.operation),
            ),
            (
                "gen_ai.request.model".to_string(),
                serde_json::json!(self.model),
            ),
            (
                "gen_ai.usage.input_tokens".to_string(),
                serde_json::json!(self.input_tokens),
            ),
            (
                "gen_ai.usage.output_tokens".to_string(),
                serde_json::json!(self.output_tokens),
            ),
            (
                "gen_ai.usage.cached_tokens".to_string(),
                serde_json::json!(self.cached_tokens),
            ),
            (
                "gen_ai.response.latency_ms".to_string(),
                serde_json::json!(self.latency_ms),
            ),
        ]
    }

    /// End time in unix millis.
    pub fn end_time_unix_ms(&self) -> i64 {
        self.start_time_unix_ms + self.latency_ms
    }
}

/// Telemetry collector.
pub struct TelemetryCollector {
    spans: Vec<GenAiSpan>,
    service_name: String,
    service_version: String,
}

impl TelemetryCollector {
    pub fn new() -> Self {
        Self {
            spans: Vec::new(),
            service_name: "gaggle".to_string(),
            service_version: env!("CARGO_PKG_VERSION").to_string(),
        }
    }

    pub fn with_service(service_name: &str, service_version: &str) -> Self {
        Self {
            spans: Vec::new(),
            service_name: service_name.to_string(),
            service_version: service_version.to_string(),
        }
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

    /// Export all spans as OpenTelemetry-compatible OTLP JSON.
    pub fn export_otel(&self) -> serde_json::Value {
        let spans: Vec<serde_json::Value> = self
            .spans
            .iter()
            .map(|s| {
                serde_json::json!({
                    "traceId": s.trace_id,
                    "spanId": s.span_id,
                    "name": s.operation,
                    "kind": 1,
                    "startTimeUnixNano": (s.start_time_unix_ms as u128) * 1_000_000,
                    "endTimeUnixNano": (s.end_time_unix_ms() as u128) * 1_000_000,
                    "attributes": s.to_otel_attributes().iter().map(|(k, v)| {
                        serde_json::json!({
                            "key": k,
                            "value": { "stringValue": v.to_string().trim_matches('"') }
                        })
                    }).collect::<Vec<_>>(),
                    "status": { "code": if s.status == "OK" { 1 } else { 2 }, "message": s.status }
                })
            })
            .collect();

        serde_json::json!({
            "resourceSpans": [{
                "resource": {
                    "attributes": [
                        { "key": "service.name", "value": { "stringValue": self.service_name } },
                        { "key": "service.version", "value": { "stringValue": self.service_version } }
                    ]
                },
                "scopeSpans": [{
                    "scope": { "name": "gaggle.genai", "version": self.service_version },
                    "spans": spans
                }]
            }]
        })
    }
}

impl Default for TelemetryCollector {
    fn default() -> Self {
        Self::new()
    }
}

/// Initialize the OpenTelemetry tracer (no-op when feature disabled).
#[cfg(feature = "telemetry")]
pub fn init_tracer(service_name: &str) {
    let tracer = opentelemetry::global::tracer(service_name.to_string());
    let _ = tracer;
}

#[cfg(not(feature = "telemetry"))]
pub fn init_tracer(_service_name: &str) {}

/// Record a GenAI span via OpenTelemetry (no-op when feature disabled).
#[cfg(feature = "telemetry")]
pub fn record_span(span: &GenAiSpan) {
    use opentelemetry::trace::Tracer;
    let tracer = opentelemetry::global::tracer("gaggle");
    let _otel_span = tracer.start(span.operation.clone());
}

#[cfg(not(feature = "telemetry"))]
pub fn record_span(_span: &GenAiSpan) {}

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

    #[test]
    fn export_otel_structure() {
        let mut collector = TelemetryCollector::new();
        let mut span = GenAiSpan::new("chat", "gpt-4");
        span.record_usage(10, 5, 1);
        span.finish();
        collector.record(span);
        let json = collector.export_otel();
        assert!(json.get("resourceSpans").is_some());
    }
}

#[cfg(feature = "telemetry")]
mod feature_tests {
    use super::*;

    #[test]
    fn feature_enabled_compiles() {
        init_tracer("gaggle");
        let span = GenAiSpan::new("chat", "gpt-4");
        record_span(&span);
    }
}
