//! Structured logging, correlation, and redaction primitives.
//!
//! Gaggle deliberately keeps observability local: logs are written to the process stderr, no
//! telemetry is uploaded, and diagnostics are only produced when the user explicitly asks for
//! them. Two guarantees are enforced in this module:
//!
//! 1. Every runtime log entry carries the `chat_id`, `run_id`, and `tool_call_id` correlation
//!    fields (missing identifiers render as `-`).
//! 2. Redaction happens at the tracing writer boundary, so a secret cannot reach the log sink even
//!    if an individual call site forgets to sanitize a value.
//!
//! The default log policy never records prompts, source-file contents, raw command output, or
//! database row payloads. Tool arguments are summarized with a fingerprint and redacted values.

use crate::context_assembler::redact_context_secrets;
use serde_json::{Value, json};
use sha2::{Digest, Sha256};
use std::io::{self, Write};
use tracing_subscriber::{EnvFilter, fmt::MakeWriter};

/// Maximum characters kept for a single non-sensitive argument value in a log summary.
pub const MAX_ARG_VALUE_CHARS: usize = 80;

/// Maximum characters of a serialized tool-argument summary.
pub const MAX_ARGS_SUMMARY_CHARS: usize = 1024;

/// Maximum characters kept in a user-facing failure message.
pub const MAX_ERROR_MESSAGE_CHARS: usize = 512;

/// Maximum bytes buffered by the redacting writer before a partial line is force-flushed.
pub const MAX_LOG_LINE_BYTES: usize = 16 * 1024;

/// JSON-style key names whose values must never be written to a log sink.
const SENSITIVE_LOG_KEYS: &[&str] = &[
    "access_token",
    "api-key",
    "api_key",
    "apikey",
    "args",
    "authorization",
    "body",
    "client_secret",
    "code",
    "command",
    "content",
    "credential",
    "credentials",
    "diff",
    "file_content",
    "id_token",
    "messages",
    "output",
    "passwd",
    "password",
    "patch",
    "private_key",
    "prompt",
    "refresh_token",
    "secret",
    "source",
    "stderr",
    "stdout",
    "text",
    "token",
];

/// Returns a stable, non-sensitive correlation fingerprint for a string.
///
/// The fingerprint is a SHA-256 hex digest. It is safe to log and lets a user correlate two
/// entries that refer to the same arguments without exposing the arguments themselves.
pub fn fingerprint_text(input: &str) -> String {
    let mut hasher = Sha256::new();
    hasher.update(input.as_bytes());
    let digest = hasher.finalize();
    let mut hex = String::with_capacity(digest.len() * 2);
    for byte in digest.as_slice() {
        use std::fmt::Write as _;
        let _ = write!(&mut hex, "{byte:02x}");
    }
    hex
}

fn normalize_key(key: &str) -> String {
    key.trim().to_ascii_lowercase().replace(['-', ' '], "_")
}

fn is_sensitive_key(key: &str) -> bool {
    let normalized = normalize_key(key);
    SENSITIVE_LOG_KEYS
        .iter()
        .any(|sensitive| normalize_key(sensitive) == normalized)
}

fn truncate_chars(input: &str, limit: usize) -> String {
    if input.chars().count() <= limit {
        return input.to_string();
    }
    let mut output: String = input.chars().take(limit).collect();
    output.push_str("…[truncated]");
    output
}

/// Bounds a user-facing error message while keeping the useful leading context.
pub fn truncate_for_message(input: &str) -> String {
    truncate_chars(input, MAX_ERROR_MESSAGE_CHARS)
}

/// Redacts secrets and sensitive values from an arbitrary log line.
///
/// This is the single redaction entry point used by the tracing writer. It combines the
/// context-assembly redactor (private keys, `Bearer` tokens, known token prefixes, `key=value`
/// assignments) with JSON-aware redaction for structured log fields such as `{"api_key":"..."}`
/// and `{"command":"..."}`.
pub fn redact_log_text(input: &str) -> String {
    if input.is_empty() {
        return String::new();
    }
    // `redact_context_secrets` reconstructs text with `lines()`, which drops trailing newlines.
    // Preserve the exact trailing line terminator so the tracing writer never concatenates
    // adjacent log records.
    let trailing_start = input.trim_end_matches(['\n', '\r']).len();
    let (body, suffix) = input.split_at(trailing_start);
    let context_redacted = redact_context_secrets(body);
    let redacted = redact_json_like_values(&context_redacted);
    format!("{redacted}{suffix}")
}

/// Redacts values of sensitive JSON-style keys without parsing the surrounding line.
fn redact_json_like_values(input: &str) -> String {
    let chars: Vec<char> = input.chars().collect();
    let mut output = String::with_capacity(input.len());
    let mut index = 0;

    while index < chars.len() {
        if chars[index] == '"'
            && let Some(key_end) = find_closing_quote(&chars, index + 1)
        {
            let key: String = chars[index + 1..key_end].iter().collect();
            if is_sensitive_key(&key) {
                let mut cursor = key_end + 1;
                while cursor < chars.len() && chars[cursor].is_whitespace() {
                    cursor += 1;
                }
                if cursor < chars.len() && chars[cursor] == ':' {
                    cursor += 1;
                    while cursor < chars.len() && chars[cursor].is_whitespace() {
                        cursor += 1;
                    }
                    if cursor < chars.len() && chars[cursor] == '"' {
                        output.extend(chars[index..=cursor].iter());
                        let value_start = cursor + 1;
                        let value_end =
                            find_closing_quote(&chars, value_start).unwrap_or(chars.len());
                        output.push_str("[REDACTED]");
                        index = value_end;
                        continue;
                    }
                }
            }
        }
        output.push(chars[index]);
        index += 1;
    }

    output
}

/// Finds the next unescaped double quote at or after `start`, returning its index.
fn find_closing_quote(chars: &[char], start: usize) -> Option<usize> {
    let mut escaped = false;
    for (offset, character) in chars.iter().enumerate().skip(start) {
        if escaped {
            escaped = false;
            continue;
        }
        match character {
            '\\' => escaped = true,
            '"' => return Some(offset),
            _ => {}
        }
    }
    None
}

/// Builds a redacted, bounded summary of tool-call arguments for structured logging.
///
/// Sensitive keys (credentials, prompts, commands, source/content fields) are replaced with
/// `[REDACTED]`. Every value is truncated, structured values collapse to `[structured]`, and the
/// whole summary is capped. The full argument JSON is never written to the log sink.
pub fn summarize_tool_arguments(tool_name: &str, args_json: &str) -> String {
    let mut summary = json!({
        "tool": tool_name,
        "args_bytes": args_json.len(),
        "args_fingerprint": fingerprint_text(args_json),
    });

    match serde_json::from_str::<Value>(args_json) {
        Ok(Value::Object(map)) => {
            let mut keys: Vec<&String> = map.keys().collect();
            keys.sort();
            let mut args = serde_json::Map::new();
            for key in keys {
                let Some(value) = map.get(key) else {
                    continue;
                };
                let redacted_value = if is_sensitive_key(key) {
                    json!("[REDACTED]")
                } else {
                    summarize_argument_value(value)
                };
                args.insert(key.clone(), redacted_value);
            }
            summary["arg_keys"] = json!(map.keys().cloned().collect::<Vec<_>>());
            summary["args"] = Value::Object(args);
        }
        _ => {
            summary["args"] = json!("[UNPARSEABLE]");
        }
    }

    let rendered = summary.to_string();
    truncate_chars(&rendered, MAX_ARGS_SUMMARY_CHARS)
}

fn summarize_argument_value(value: &Value) -> Value {
    match value {
        Value::String(text) => {
            Value::String(truncate_chars(&redact_log_text(text), MAX_ARG_VALUE_CHARS))
        }
        Value::Number(number) => Value::Number(number.clone()),
        Value::Bool(flag) => Value::Bool(*flag),
        Value::Null => Value::Null,
        Value::Array(_) | Value::Object(_) => Value::String("[structured]".to_string()),
    }
}

/// Initializes the process-wide tracing subscriber with a redacting stderr writer.
///
/// The format is plain (no ANSI) and identical on macOS and Linux. `RUST_LOG` controls verbosity
/// and defaults to `info`. Initialization is additive: if a subscriber is already installed the
/// call returns an error instead of panicking so tests and embedders can provide their own.
pub fn init_tracing() -> anyhow::Result<()> {
    let filter = EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new("info"));
    tracing_subscriber::fmt()
        .with_env_filter(filter)
        .with_ansi(false)
        .with_target(true)
        .with_writer(RedactingMakeWriter::new(io::stderr))
        .try_init()
        .map_err(|error| anyhow::anyhow!("failed to initialize tracing subscriber: {error}"))
}

/// A [`MakeWriter`] wrapper that redacts every completed line before it reaches the inner writer.
#[derive(Debug, Clone, Copy)]
pub struct RedactingMakeWriter<M> {
    inner: M,
}

impl<M> RedactingMakeWriter<M> {
    pub fn new(inner: M) -> Self {
        Self { inner }
    }
}

impl<'a, M> MakeWriter<'a> for RedactingMakeWriter<M>
where
    M: MakeWriter<'a>,
{
    type Writer = RedactingWriter<M::Writer>;

    fn make_writer(&'a self) -> Self::Writer {
        RedactingWriter::new(self.inner.make_writer())
    }
}

/// Line-buffering writer that applies [`redact_log_text`] before forwarding bytes.
///
/// Buffering until a newline prevents a secret from being split across two `write` calls (the
/// tracing formatter writes fields and the trailing newline separately).
#[derive(Debug)]
pub struct RedactingWriter<W> {
    inner: W,
    buffer: Vec<u8>,
}

impl<W: Write> RedactingWriter<W> {
    pub fn new(inner: W) -> Self {
        Self {
            inner,
            buffer: Vec::new(),
        }
    }

    /// Returns the wrapped writer. Useful for tests that inspect a `Cursor` after flushing.
    pub fn into_inner(self) -> W {
        self.inner
    }

    fn flush_buffer(&mut self) -> io::Result<()> {
        if self.buffer.is_empty() {
            return Ok(());
        }
        let raw = String::from_utf8_lossy(&self.buffer);
        let redacted = redact_log_text(&raw);
        self.inner.write_all(redacted.as_bytes())?;
        self.buffer.clear();
        Ok(())
    }

    fn flush_complete_lines(&mut self) -> io::Result<()> {
        while let Some(position) = self.buffer.iter().position(|byte| *byte == b'\n') {
            let line: Vec<u8> = self.buffer.drain(..=position).collect();
            let raw = String::from_utf8_lossy(&line);
            let redacted = redact_log_text(&raw);
            self.inner.write_all(redacted.as_bytes())?;
        }
        Ok(())
    }
}

impl<W: Write> Write for RedactingWriter<W> {
    fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
        self.buffer.extend_from_slice(buf);
        self.flush_complete_lines()?;
        if self.buffer.len() >= MAX_LOG_LINE_BYTES {
            let raw = String::from_utf8_lossy(&self.buffer);
            let redacted = redact_log_text(&raw);
            let bounded = truncate_chars(&redacted, MAX_LOG_LINE_BYTES);
            self.inner.write_all(bounded.as_bytes())?;
            self.inner.write_all(b"\n")?;
            self.buffer.clear();
        }
        Ok(buf.len())
    }

    fn flush(&mut self) -> io::Result<()> {
        self.flush_complete_lines()?;
        self.flush_buffer()?;
        self.inner.flush()
    }
}

#[cfg(test)]
mod tests {
    use super::{RedactingWriter, fingerprint_text, redact_log_text, summarize_tool_arguments};
    use std::io::{Cursor, Write};

    #[test]
    fn redacts_known_token_shapes_and_assignments() {
        let raw = "provider key sk-live-SECRET-1234567890 and Authorization: Bearer abc.def.ghi \
                   plus password=hunter2 and github token ghp_abcdefghijklmnopqrstuvwxyz0123456789";
        let redacted = redact_log_text(raw);

        assert!(!redacted.contains("sk-live-SECRET-1234567890"));
        assert!(!redacted.contains("abc.def.ghi"));
        assert!(!redacted.contains("hunter2"));
        assert!(!redacted.contains("ghp_abcdefghijklmnopqrstuvwxyz0123456789"));
        assert!(redacted.contains("[REDACTED]"));
    }

    #[test]
    fn redacts_private_key_blocks() {
        let raw =
            "before\n-----BEGIN RSA PRIVATE KEY-----\nMIIEow\n-----END RSA PRIVATE KEY-----\nafter";
        let redacted = redact_log_text(raw);
        assert!(!redacted.contains("MIIEow"));
        assert!(redacted.contains("[REDACTED PRIVATE KEY]"));
    }

    #[test]
    fn redacts_json_style_sensitive_values() {
        let raw = r#"args={"api_key":"sk-live-999","path":"src/main.rs","command":"rm -rf /","prompt":"full prompt text"}"#;
        let redacted = redact_log_text(raw);

        assert!(!redacted.contains("sk-live-999"));
        assert!(!redacted.contains("rm -rf /"));
        assert!(!redacted.contains("full prompt text"));
        assert!(redacted.contains("\"path\":\"src/main.rs\""));
        assert!(redacted.contains("[REDACTED]"));
    }

    #[test]
    fn tool_argument_summary_never_contains_full_values() {
        let long_content = "x".repeat(500);
        let args = serde_json::json!({
            "path": "src/main.rs",
            "content": long_content,
            "command": "cat /etc/passwd",
            "password": "hunter2",
            "nested": {"a": 1}
        })
        .to_string();

        let summary = summarize_tool_arguments("fs.write", &args);

        assert!(!summary.contains("hunter2"));
        assert!(!summary.contains("cat /etc/passwd"));
        assert!(!summary.contains(&"x".repeat(200)));
        assert!(summary.contains("\"content\":\"[REDACTED]\""));
        assert!(summary.contains("\"password\":\"[REDACTED]\""));
        assert!(summary.contains("\"path\":\"src/main.rs\""));
        assert!(summary.contains("args_fingerprint"));
    }

    #[test]
    fn tool_argument_summary_redacts_unparseable_input() {
        let summary = summarize_tool_arguments("shell.exec", "not-json sk-live-SECRET-1234567890");
        assert!(!summary.contains("sk-live-SECRET-1234567890"));
        assert!(summary.contains("[UNPARSEABLE]"));
    }

    #[test]
    fn fingerprint_is_stable_and_not_reversible() {
        let first = fingerprint_text("hello");
        let second = fingerprint_text("hello");
        assert_eq!(first, second);
        assert_eq!(first.len(), 64);
        assert_ne!(first, fingerprint_text("hello!"));
    }

    #[test]
    fn redacting_writer_redacts_writes_split_across_calls() {
        let mut writer = RedactingWriter::new(Cursor::new(Vec::new()));
        writer.write_all(b"token=").expect("write prefix");
        writer
            .write_all(b"sk-live-SPLIT-1234567890\nnext line\n")
            .expect("write rest");
        writer.flush().expect("flush writer");

        let output = String::from_utf8(writer.into_inner().into_inner()).expect("utf8 output");
        assert!(!output.contains("sk-live-SPLIT-1234567890"));
        assert!(output.contains("[REDACTED]"));
        assert!(output.contains("next line"));
    }

    #[test]
    fn redacting_writer_preserves_non_sensitive_lines_byte_for_byte() {
        let mut writer = RedactingWriter::new(Cursor::new(Vec::new()));
        writer
            .write_all(b"INFO chat_id=chat-1 run_id=run-1 tool_call_id=- agent run started\n")
            .expect("write line");
        writer.flush().expect("flush writer");

        let output = String::from_utf8(writer.into_inner().into_inner()).expect("utf8 output");
        assert_eq!(
            output,
            "INFO chat_id=chat-1 run_id=run-1 tool_call_id=- agent run started\n"
        );
    }
}
