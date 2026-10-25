//! Structured conversation summary used by context compaction (ported from
//! Block Goose's `goose-context-management` crate).
//!
//! A compaction LLM call is asked to emit a JSON document matching
//! [`StructuredSummary`]. [`StructuredSummary::parse`] extracts it from the
//! model's response (including an `<analysis>` scratchpad) and
//! [`StructuredSummary::render`] / [`render_with`] turn it into the markdown
//! context that is stored as the compaction summary. When no usable JSON is
//! found, parsing returns `None` and callers keep the raw model output as a
//! lossless fallback.

use minijinja::value::Serde;
use minijinja::{Environment, Error as MiniJinjaError, Value as MJValue};
use serde::{Deserialize, Serialize};
use serde_json::Value as JsonValue;

/// Activity recorded for a single file during compaction.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct FileActivity {
    #[serde(default, deserialize_with = "lenient_string")]
    pub path: String,
    #[serde(default, deserialize_with = "lenient_string")]
    pub summary: String,
    #[serde(default, deserialize_with = "lenient_string_opt")]
    pub key_code: Option<String>,
}

/// Structured output of a compaction LLM call.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct StructuredSummary {
    #[serde(default, deserialize_with = "lenient_string_list")]
    pub user_intent: Vec<String>,
    #[serde(default, deserialize_with = "lenient_string_list")]
    pub technical_concepts: Vec<String>,
    #[serde(default, deserialize_with = "lenient_file_list")]
    pub files: Vec<FileActivity>,
    #[serde(default, deserialize_with = "lenient_string_list")]
    pub errors_and_fixes: Vec<String>,
    #[serde(default, deserialize_with = "lenient_string_list")]
    pub problem_solving: Vec<String>,
    #[serde(default, deserialize_with = "lenient_string_list")]
    pub user_messages: Vec<String>,
    #[serde(default, deserialize_with = "lenient_string_list")]
    pub pending_tasks: Vec<String>,
    #[serde(default, deserialize_with = "lenient_string_opt")]
    pub current_work: Option<String>,
    #[serde(default, deserialize_with = "lenient_string_opt")]
    pub next_step: Option<String>,
    #[serde(flatten)]
    pub extra: serde_json::Map<String, JsonValue>,
}

fn stringify_lenient(value: &JsonValue) -> String {
    match value {
        JsonValue::String(s) => s.clone(),
        JsonValue::Null => String::new(),
        JsonValue::Object(map) => map
            .iter()
            .map(|(k, v)| format!("{k}: {}", stringify_lenient(v)))
            .collect::<Vec<_>>()
            .join("; "),
        JsonValue::Array(items) => items
            .iter()
            .map(stringify_lenient)
            .collect::<Vec<_>>()
            .join("; "),
        other => other.to_string(),
    }
}

fn lenient_string_list<'de, D>(deserializer: D) -> Result<Vec<String>, D::Error>
where
    D: serde::Deserializer<'de>,
{
    let value = JsonValue::deserialize(deserializer)?;
    Ok(match value {
        JsonValue::Array(items) => items.iter().map(stringify_lenient).collect(),
        JsonValue::Null => Vec::new(),
        other => vec![stringify_lenient(&other)],
    })
}

fn lenient_string<'de, D>(deserializer: D) -> Result<String, D::Error>
where
    D: serde::Deserializer<'de>,
{
    let value = JsonValue::deserialize(deserializer)?;
    Ok(stringify_lenient(&value))
}

fn lenient_string_opt<'de, D>(deserializer: D) -> Result<Option<String>, D::Error>
where
    D: serde::Deserializer<'de>,
{
    let value = JsonValue::deserialize(deserializer)?;
    Ok(match value {
        JsonValue::Null => None,
        other => Some(stringify_lenient(&other)),
    })
}

fn lenient_file_list<'de, D>(deserializer: D) -> Result<Vec<FileActivity>, D::Error>
where
    D: serde::Deserializer<'de>,
{
    let value = JsonValue::deserialize(deserializer)?;
    let items = match value {
        JsonValue::Array(items) => items,
        JsonValue::Null => return Ok(Vec::new()),
        other => vec![other],
    };
    Ok(items
        .into_iter()
        .filter_map(|item| match item {
            JsonValue::Object(_) => serde_json::from_value(item).ok(),
            other => {
                let path = stringify_lenient(&other);
                (!path.trim().is_empty()).then_some(FileActivity {
                    path,
                    summary: String::new(),
                    key_code: None,
                })
            }
        })
        .collect())
}

impl StructuredSummary {
    /// Returns `None` when no usable JSON document is found.
    pub fn parse(response_text: &str) -> Option<Self> {
        json_candidates(response_text)
            .into_iter()
            .find_map(|candidate| {
                let value = safely_parse_json(candidate).ok()?;
                let mut summary: Self = serde_json::from_value(value).ok()?;
                summary.normalize();
                (!summary.is_empty()).then_some(summary)
            })
    }

    pub fn render(&self) -> Result<String, MiniJinjaError> {
        self.render_with(DEFAULT_SUMMARY_TEMPLATE)
    }

    pub fn render_with(&self, template: &str) -> Result<String, MiniJinjaError> {
        render_template(template, self)
    }

    fn normalize(&mut self) {
        fn blank(s: &str) -> bool {
            s.trim().is_empty()
        }
        for list in [
            &mut self.user_intent,
            &mut self.technical_concepts,
            &mut self.errors_and_fixes,
            &mut self.problem_solving,
            &mut self.user_messages,
            &mut self.pending_tasks,
        ] {
            list.retain(|s| !blank(s));
        }
        for file in &mut self.files {
            if file.key_code.as_deref().is_some_and(blank) {
                file.key_code = None;
            }
        }
        self.files
            .retain(|f| !blank(&f.path) || !blank(&f.summary) || f.key_code.is_some());
        if self.current_work.as_deref().is_some_and(blank) {
            self.current_work = None;
        }
        if self.next_step.as_deref().is_some_and(blank) {
            self.next_step = None;
        }
    }

    fn is_empty(&self) -> bool {
        self.user_intent.is_empty()
            && self.technical_concepts.is_empty()
            && self.files.is_empty()
            && self.errors_and_fixes.is_empty()
            && self.problem_solving.is_empty()
            && self.user_messages.is_empty()
            && self.pending_tasks.is_empty()
            && self.current_work.is_none()
            && self.next_step.is_none()
    }
}

/// Default render template (ported from Goose's `compaction_summary.md`).
pub const DEFAULT_SUMMARY_TEMPLATE: &str = r#"# Conversation Summary

{% if user_intent %}
## User Intent
{% for item in user_intent %}
- {{ item }}
{% endfor %}

{% endif %}
{% if technical_concepts %}
## Technical Concepts
{% for item in technical_concepts %}
- {{ item }}
{% endfor %}

{% endif %}
{% if files %}
## Files + Code
{% for file in files %}
{% if file.path %}
### {{ file.path }}
{% endif %}
{{ file.summary }}
{% if file.key_code %}
{{ file.key_code | code_fence }}
{% endif %}

{% endfor %}
{% endif %}
{% if errors_and_fixes %}
## Errors + Fixes
{% for item in errors_and_fixes %}
- {{ item }}
{% endfor %}

{% endif %}
{% if problem_solving %}
## Problem Solving
{% for item in problem_solving %}
- {{ item }}
{% endfor %}

{% endif %}
{% if user_messages %}
## User Messages
{% for item in user_messages %}
- {{ item }}
{% endfor %}

{% endif %}
{% if pending_tasks %}
## Pending Tasks
{% for item in pending_tasks %}
- {{ item }}
{% endfor %}

{% endif %}
{% if current_work %}
## Current Work
{{ current_work }}

{% endif %}
{% if next_step %}
## Next Step
{{ next_step }}
{% endif %}"#;

/// Default compaction prompt template (ported from Goose's `compaction.md`).
pub const DEFAULT_COMPACTION_TEMPLATE: &str = r#"## Task Context
- An llm context limit was reached when a user was in a working session with an agent
- Distill the conversation below into a structured summary with only the most verbose parts removed
- Include user requests, your responses, all technical content, and as much of the original context as possible
- This will be used to let the user continue the working session

**Conversation History:**
{{ messages }}

Wrap reasoning in `<analysis>` tags:
- Review conversation chronologically: user goals, your methods, key decisions, files, errors, fixes
- Keep this brief - the analysis is discarded, so it is a checklist of what to include, not the place for detail

After the closing `</analysis>` tag, output exactly one ```json code block and nothing else, matching this schema:

```json
{
  "user_intent": ["every user goal and request, most important first"],
  "technical_concepts": ["all discussed tools, methods, and concepts"],
  "files": [
    {
      "path": "path of a file that was viewed or edited",
      "summary": "what was done to it and why",
      "key_code": "important code, signatures, or diffs from this file (omit if none)"
    }
  ],
  "errors_and_fixes": ["bugs hit, their resolutions, and user-driven changes"],
  "problem_solving": ["issues solved or in progress, and key decisions: what was chosen, what was rejected, and why"],
  "user_messages": ["all user messages, truncating long tool call arguments or results"],
  "pending_tasks": ["all unresolved user requests, most important first"],
  "current_work": "active work at summary request time: filenames, code, alignment to latest instruction",
  "next_step": "include only if it directly continues a user instruction, otherwise omit"
}
```

Rules for the JSON:
- The `<analysis>` block is a discarded scratchpad: only the JSON survives, so it must be self-contained and repeat every detail from the analysis that matters for continuing
- Order every list from most to least important
- Every list entry must be a plain string, not a nested object - except `files`, whose entries are objects shaped as shown above
- Quote error messages, panic text, and failing test output verbatim in `errors_and_fixes`
- Omit a field rather than inventing content for it
- No new ideas unless user confirmed"#;

/// Prompt sources for a compaction run.
#[derive(Debug, Clone)]
pub struct CompactionTemplates {
    pub compaction: String,
    pub summary: String,
}

impl Default for CompactionTemplates {
    fn default() -> Self {
        Self {
            compaction: DEFAULT_COMPACTION_TEMPLATE.to_string(),
            summary: DEFAULT_SUMMARY_TEMPLATE.to_string(),
        }
    }
}

/// Alias matching Goose's naming for callers/tests that expect `Templates`.
pub type Templates = CompactionTemplates;

/// Renders a minijinja template with a serializable context, including the
/// `code_fence` filter used by the summary template.
pub fn render_template<T: Serialize>(
    template: &str,
    context: &T,
) -> Result<String, MiniJinjaError> {
    let mut env = Environment::new();
    env.set_syntax(
        minijinja::syntax::SyntaxConfig::builder()
            .trim_blocks(true)
            .lstrip_blocks(true)
            .build()
            .unwrap(),
    );
    env.add_filter("code_fence", code_fence_filter);
    env.add_template("template", template)?;
    let rendered = env
        .get_template("template")?
        .render(MJValue::from(Serde(context)))?;
    Ok(rendered.trim().to_string())
}

fn code_fence_filter(code: String) -> String {
    let longest_run = code
        .chars()
        .fold((0usize, 0usize), |(max, run), c| {
            if c == '`' {
                (max.max(run + 1), run + 1)
            } else {
                (max, 0)
            }
        })
        .0;
    let fence = "`".repeat((longest_run + 1).max(3));
    format!("{fence}\n{}\n{fence}", code.trim_end_matches('\n'))
}

/// Formats a single message for inclusion in the compaction prompt.
pub fn format_message_for_compacting(seq: i64, role: &str, content: &str) -> String {
    format!("[seq {}] [{}]: {}", seq, role, content)
}

// --- JSON candidate extraction (ported from Goose) ---

fn safely_parse_json(s: &str) -> Result<JsonValue, serde_json::Error> {
    match serde_json::from_str(s) {
        Ok(value) => Ok(value),
        Err(_) => {
            for candidate in [repair_truncated_json(s), json_escape_control_chars(s)] {
                if let Ok(value) = serde_json::from_str(&candidate) {
                    return Ok(value);
                }
            }
            let repaired = repair_truncated_json(&json_escape_control_chars(s));
            serde_json::from_str(&repaired)
        }
    }
}

fn repair_truncated_json(s: &str) -> String {
    let mut repaired = String::with_capacity(s.len() + 8);
    let mut in_string = false;
    let mut escape_next = false;
    let mut closers = Vec::new();
    for c in s.chars() {
        repaired.push(c);
        if in_string {
            if escape_next {
                escape_next = false;
                continue;
            }
            match c {
                '\\' => escape_next = true,
                '"' => in_string = false,
                _ => {}
            }
            continue;
        }
        match c {
            '"' => in_string = true,
            '{' => closers.push('}'),
            '[' => closers.push(']'),
            '}' | ']' if closers.last() == Some(&c) => {
                closers.pop();
            }
            _ => {}
        }
    }
    if in_string {
        if escape_next {
            repaired.push('\\');
        }
        repaired.push('"');
    }
    while let Some(closer) = closers.pop() {
        repaired.push(closer);
    }
    repaired
}

fn json_escape_control_chars(s: &str) -> String {
    let mut r = String::with_capacity(s.len());
    for c in s.chars() {
        match c {
            '\u{0000}'..='\u{001F}' => match c {
                '\u{0008}' => r.push_str("\\b"),
                '\u{000C}' => r.push_str("\\f"),
                '\n' => r.push_str("\\n"),
                '\r' => r.push_str("\\r"),
                '\t' => r.push_str("\\t"),
                _ => r.push_str(&format!("\\u{:04x}", c as u32)),
            },
            _ => r.push(c),
        }
    }
    r
}

fn json_candidates(text: &str) -> Vec<&str> {
    const TERMINATOR: &str = "</analysis>";
    let mut cuts: Vec<usize> = text
        .match_indices(TERMINATOR)
        .map(|(idx, _)| idx + TERMINATOR.len())
        .collect();
    if cuts.is_empty() {
        cuts.push(0);
    }
    let mut candidates: Vec<&str> = Vec::new();
    for &cut in cuts.iter().rev() {
        let tail = &text[cut..];
        let later_terminators = tail.matches(TERMINATOR).count();
        candidates.extend(
            fenced_json_blocks(tail)
                .chain(leading_object(tail))
                .filter(|candidate| candidate.matches(TERMINATOR).count() == later_terminators),
        );
    }
    candidates.extend(leading_object(text));
    candidates.dedup();
    candidates
}

fn fenced_json_blocks(text: &str) -> impl Iterator<Item = &str> {
    text.match_indices("```json")
        .collect::<Vec<_>>()
        .into_iter()
        .rev()
        .filter_map(|(idx, marker)| leading_object(&text[idx + marker.len()..]))
}

fn leading_object(text: &str) -> Option<&str> {
    let text = text.trim_start();
    if !text.starts_with('{') {
        return None;
    }
    let mut depth = 0usize;
    let mut in_string = false;
    let mut escaped = false;
    for (idx, ch) in text.char_indices() {
        if in_string {
            if escaped {
                escaped = false;
            } else if ch == '\\' {
                escaped = true;
            } else if ch == '"' {
                in_string = false;
            }
            continue;
        }
        match ch {
            '"' => in_string = true,
            '{' => depth += 1,
            '}' => {
                depth -= 1;
                if depth == 0 {
                    return Some(&text[..=idx]);
                }
            }
            _ => {}
        }
    }
    None
}

// --- auto compact threshold ---

/// Lowers the configured threshold so compaction also starts by an absolute token cap.
///
/// Mirrors Goose's `auto_compact_threshold`: combines the configured ratio with
/// `GOOSE_AUTO_COMPACT_TOKEN_LIMIT` (default 225_000) so very large context
/// windows compact before hitting provider limits.
pub fn auto_compact_threshold(context_window: usize, configured_threshold: f64) -> f64 {
    const DEFAULT_TOKEN_LIMIT: usize = 225_000;
    if configured_threshold <= 0.0 || configured_threshold >= 1.0 || context_window == 0 {
        return configured_threshold;
    }
    configured_threshold.min(DEFAULT_TOKEN_LIMIT as f64 / context_window as f64)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn structured_summary_parses_fenced_json() {
        let text = r#"<analysis>x</analysis>

```json
{"user_intent": ["Fix bug"], "pending_tasks": ["Add test"]}
```"#;
        let summary = StructuredSummary::parse(text).expect("should parse");
        assert_eq!(summary.user_intent, vec!["Fix bug"]);
        assert_eq!(summary.pending_tasks, vec!["Add test"]);
    }

    #[test]
    fn structured_summary_falls_back_for_plain_text() {
        assert!(StructuredSummary::parse("just a summary").is_none());
    }

    #[test]
    fn auto_compact_threshold_lowers_for_large_windows() {
        let t = auto_compact_threshold(400_000, 0.75);
        assert!(t < 0.75);
        assert!(t > 0.0);
        let t2 = auto_compact_threshold(1000, 0.75);
        assert!((t2 - 0.75).abs() < f64::EPSILON);
    }
}
