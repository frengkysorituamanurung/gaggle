use crate::{
    connection::{new_id, now_ms},
    models::Memory,
};
use anyhow::{Result, bail};
use sqlx::{Sqlite, SqlitePool, Transaction};
use std::collections::HashMap;

/// Maximum number of rows returned by any memory listing or search query so a
/// single call can never produce an unbounded result set.
const MAX_LIMIT: i64 = 100;
/// Maximum number of tags accepted for a single memory.
const MAX_TAGS: usize = 16;
/// Maximum length, in characters, of a single tag.
const MAX_TAG_LEN: usize = 64;
/// Maximum number of whitespace-separated terms used to build an FTS5 query.
const MAX_FTS_TERMS: usize = 16;
/// Memory kinds accepted by the `memories.kind` CHECK constraint.
const ALLOWED_KINDS: [&str; 5] = ["fact", "preference", "decision", "todo", "summary"];

/// Placeholder written in place of detected secret material.
const REDACTED: &str = "[REDACTED]";

/// Key names whose values must never be written to an export.
const SENSITIVE_KEYS: [&str; 14] = [
    "authorization",
    "bearer",
    "api_key",
    "api-key",
    "apikey",
    "client_secret",
    "secret",
    "token",
    "access_token",
    "refresh_token",
    "password",
    "passwd",
    "private_key",
    "private-key",
];

pub struct MemoryRepository {
    pool: SqlitePool,
}

impl MemoryRepository {
    pub fn new(pool: SqlitePool) -> Self {
        Self { pool }
    }

    /// Creates a memory without tags or source metadata.
    pub async fn create(
        &self,
        project_id: Option<&str>,
        kind: &str,
        title: &str,
        content: &str,
    ) -> Result<Memory> {
        let id = new_id();
        let now = now_ms();

        sqlx::query(
            r#"
            INSERT INTO memories (id, project_id, kind, title, content, pinned, confidence, created_at, updated_at)
            VALUES (?, ?, ?, ?, ?, 0, 1.0, ?, ?)
            "#,
        )
        .bind(&id)
        .bind(project_id)
        .bind(kind)
        .bind(title)
        .bind(content)
        .bind(now)
        .bind(now)
        .execute(&self.pool)
        .await?;

        let mem = sqlx::query_as::<_, Memory>("SELECT * FROM memories WHERE id = ?")
            .bind(&id)
            .fetch_one(&self.pool)
            .await?;
        Ok(mem)
    }

    /// Creates a memory with optional provenance, tags, and pin state.
    ///
    /// The memory row and its tags are written in a single transaction so a
    /// partially tagged memory can never be observed.
    #[allow(clippy::too_many_arguments)]
    pub async fn create_with_options(
        &self,
        project_id: Option<&str>,
        source_chat_id: Option<&str>,
        source_run_id: Option<&str>,
        kind: &str,
        title: &str,
        content: &str,
        tags: &[String],
        pinned: bool,
    ) -> Result<Memory> {
        validate_kind(kind)?;
        let title = normalize_text("title", title)?;
        let content = normalize_text("content", content)?;
        let tags = normalize_tags(tags)?;

        let id = new_id();
        let now = now_ms();
        let pinned = bool_to_int(pinned);

        let mut tx = self.pool.begin().await?;

        sqlx::query(
            r#"
            INSERT INTO memories (
                id, project_id, source_chat_id, source_run_id, kind, title, content,
                pinned, confidence, created_at, updated_at
            ) VALUES (?, ?, ?, ?, ?, ?, ?, ?, 1.0, ?, ?)
            "#,
        )
        .bind(&id)
        .bind(project_id)
        .bind(source_chat_id)
        .bind(source_run_id)
        .bind(kind)
        .bind(title)
        .bind(content)
        .bind(pinned)
        .bind(now)
        .bind(now)
        .execute(&mut *tx)
        .await?;

        insert_tags(&mut tx, &id, &tags).await?;

        let memory = sqlx::query_as::<_, Memory>("SELECT * FROM memories WHERE id = ?")
            .bind(&id)
            .fetch_one(&mut *tx)
            .await?;

        tx.commit().await?;
        Ok(memory)
    }

    /// Returns an active (not soft-deleted) memory by id.
    pub async fn get_by_id(&self, id: &str) -> Result<Option<Memory>> {
        let memory = sqlx::query_as::<_, Memory>(
            "SELECT * FROM memories WHERE id = ? AND deleted_at IS NULL",
        )
        .bind(id)
        .fetch_optional(&self.pool)
        .await?;
        Ok(memory)
    }

    /// Alias for [`Self::get_by_id`] that makes the soft-delete filter explicit
    /// at call sites that must never observe deleted rows.
    pub async fn get_active_by_id(&self, id: &str) -> Result<Option<Memory>> {
        self.get_by_id(id).await
    }

    /// Full-text search across all active memories via `memories_fts`.
    ///
    /// The user query is sanitized into quoted FTS5 terms before reaching
    /// `MATCH`, so FTS5 operators or unbalanced quotes cannot error.
    pub async fn search_fts(&self, query: &str, limit: i64) -> Result<Vec<Memory>> {
        let Some(match_query) = build_fts_query(query) else {
            return Ok(Vec::new());
        };

        let rows = sqlx::query_as::<_, Memory>(
            r#"
            SELECT m.* FROM memories m
            INNER JOIN memories_fts fts ON m.rowid = fts.rowid
            WHERE memories_fts MATCH ? AND m.deleted_at IS NULL
            ORDER BY bm25(memories_fts) ASC
            LIMIT ?
            "#,
        )
        .bind(match_query)
        .bind(clamp_limit(limit))
        .fetch_all(&self.pool)
        .await?;
        Ok(rows)
    }

    /// Full-text search limited to one project plus global (`project_id IS NULL`)
    /// memories. When no project is supplied only global memories are returned.
    /// Pinned memories sort first, then by bm25 relevance.
    pub async fn search_scoped(
        &self,
        project_id: Option<&str>,
        query: &str,
        limit: i64,
    ) -> Result<Vec<Memory>> {
        let Some(match_query) = build_fts_query(query) else {
            return Ok(Vec::new());
        };
        let limit = clamp_limit(limit);

        let rows = match project_id {
            Some(project_id) => {
                sqlx::query_as::<_, Memory>(
                    r#"
                    SELECT m.* FROM memories m
                    INNER JOIN memories_fts fts ON m.rowid = fts.rowid
                    WHERE memories_fts MATCH ?
                      AND m.deleted_at IS NULL
                      AND (m.project_id = ? OR m.project_id IS NULL)
                    ORDER BY m.pinned DESC, bm25(memories_fts) ASC
                    LIMIT ?
                    "#,
                )
                .bind(&match_query)
                .bind(project_id)
                .bind(limit)
                .fetch_all(&self.pool)
                .await?
            }
            None => {
                sqlx::query_as::<_, Memory>(
                    r#"
                    SELECT m.* FROM memories m
                    INNER JOIN memories_fts fts ON m.rowid = fts.rowid
                    WHERE memories_fts MATCH ?
                      AND m.deleted_at IS NULL
                      AND m.project_id IS NULL
                    ORDER BY m.pinned DESC, bm25(memories_fts) ASC
                    LIMIT ?
                    "#,
                )
                .bind(&match_query)
                .bind(limit)
                .fetch_all(&self.pool)
                .await?
            }
        };

        Ok(rows)
    }

    /// Lists active memories for a project, including global memories. Pinned
    /// memories sort first, then most recently updated.
    pub async fn list_by_project(&self, project_id: &str, limit: i64) -> Result<Vec<Memory>> {
        let rows = sqlx::query_as::<_, Memory>(
            r#"
            SELECT * FROM memories
            WHERE deleted_at IS NULL
              AND (project_id = ? OR project_id IS NULL)
            ORDER BY pinned DESC, updated_at DESC, id ASC
            LIMIT ?
            "#,
        )
        .bind(project_id)
        .bind(clamp_limit(limit))
        .fetch_all(&self.pool)
        .await?;
        Ok(rows)
    }

    /// Lists pinned, active memories scoped to a project plus global memories
    /// (or global memories only when no project is supplied).
    pub async fn list_pinned(&self, project_id: Option<&str>, limit: i64) -> Result<Vec<Memory>> {
        let limit = clamp_limit(limit);

        let rows = match project_id {
            Some(project_id) => {
                sqlx::query_as::<_, Memory>(
                    r#"
                    SELECT * FROM memories
                    WHERE deleted_at IS NULL
                      AND pinned = 1
                      AND (project_id = ? OR project_id IS NULL)
                    ORDER BY updated_at DESC, id ASC
                    LIMIT ?
                    "#,
                )
                .bind(project_id)
                .bind(limit)
                .fetch_all(&self.pool)
                .await?
            }
            None => {
                sqlx::query_as::<_, Memory>(
                    r#"
                    SELECT * FROM memories
                    WHERE deleted_at IS NULL
                      AND pinned = 1
                      AND project_id IS NULL
                    ORDER BY updated_at DESC, id ASC
                    LIMIT ?
                    "#,
                )
                .bind(limit)
                .fetch_all(&self.pool)
                .await?
            }
        };

        Ok(rows)
    }

    /// Updates the mutable fields of an active memory.
    ///
    /// Supplied title/content values are trimmed and must not be empty. The
    /// `updated_at` column is always refreshed.
    pub async fn update(
        &self,
        id: &str,
        kind: Option<&str>,
        title: Option<&str>,
        content: Option<&str>,
        pinned: Option<bool>,
    ) -> Result<Memory> {
        if let Some(kind) = kind {
            validate_kind(kind)?;
        }
        let title = title
            .map(|value| normalize_text("title", value))
            .transpose()?;
        let content = content
            .map(|value| normalize_text("content", value))
            .transpose()?;
        let pinned = pinned.map(bool_to_int);
        let now = now_ms();

        let result = sqlx::query(
            r#"
            UPDATE memories SET
                kind = COALESCE(?, kind),
                title = COALESCE(?, title),
                content = COALESCE(?, content),
                pinned = COALESCE(?, pinned),
                updated_at = ?
            WHERE id = ? AND deleted_at IS NULL
            "#,
        )
        .bind(kind)
        .bind(title)
        .bind(content)
        .bind(pinned)
        .bind(now)
        .bind(id)
        .execute(&self.pool)
        .await?;

        if result.rows_affected() == 0 {
            bail!("Memory {id} not found");
        }

        self.get_by_id(id)
            .await?
            .ok_or_else(|| anyhow::anyhow!("Memory {id} not found after update"))
    }

    /// Soft-deletes an active memory. Returns `false` when the memory was
    /// already deleted or does not exist.
    pub async fn soft_delete(&self, id: &str) -> Result<bool> {
        let now = now_ms();
        let result = sqlx::query(
            "UPDATE memories SET deleted_at = ?, updated_at = ? WHERE id = ? AND deleted_at IS NULL",
        )
        .bind(now)
        .bind(now)
        .bind(id)
        .execute(&self.pool)
        .await?;
        Ok(result.rows_affected() > 0)
    }

    /// Returns the tags attached to a memory, sorted alphabetically.
    pub async fn list_tags(&self, memory_id: &str) -> Result<Vec<String>> {
        let tags = sqlx::query_scalar::<_, String>(
            "SELECT tag FROM memory_tags WHERE memory_id = ? ORDER BY tag ASC",
        )
        .bind(memory_id)
        .fetch_all(&self.pool)
        .await?;
        Ok(tags)
    }

    /// Replaces all tags for an active memory in a single transaction.
    pub async fn set_tags(&self, memory_id: &str, tags: &[String]) -> Result<()> {
        let tags = normalize_tags(tags)?;

        let mut tx = self.pool.begin().await?;

        let exists: Option<i64> =
            sqlx::query_scalar("SELECT 1 FROM memories WHERE id = ? AND deleted_at IS NULL")
                .bind(memory_id)
                .fetch_optional(&mut *tx)
                .await?;
        if exists.is_none() {
            bail!("Memory {memory_id} not found");
        }

        sqlx::query("DELETE FROM memory_tags WHERE memory_id = ?")
            .bind(memory_id)
            .execute(&mut *tx)
            .await?;
        insert_tags(&mut tx, memory_id, &tags).await?;

        tx.commit().await?;
        Ok(())
    }

    /// Records that a memory was used by the agent, for retention heuristics.
    pub async fn touch_last_used(&self, id: &str) -> Result<()> {
        let now = now_ms();
        let result =
            sqlx::query("UPDATE memories SET last_used_at = ? WHERE id = ? AND deleted_at IS NULL")
                .bind(now)
                .bind(id)
                .execute(&self.pool)
                .await?;
        if result.rows_affected() == 0 {
            bail!("Memory {id} not found");
        }
        Ok(())
    }

    /// Renders active project plus global memories as Markdown for export.
    ///
    /// Content is escaped so stored Markdown cannot break the document
    /// structure, and obvious credentials are redacted before writing.
    pub async fn export_markdown(&self, project_id: &str) -> Result<String> {
        let memories = sqlx::query_as::<_, Memory>(
            r#"
            SELECT * FROM memories
            WHERE deleted_at IS NULL
              AND (project_id = ? OR project_id IS NULL)
            ORDER BY pinned DESC, updated_at DESC, id ASC
            "#,
        )
        .bind(project_id)
        .fetch_all(&self.pool)
        .await?;

        if memories.is_empty() {
            return Ok(
                "# Gaggle Memory\n\n_No memories stored for this project yet._\n".to_string(),
            );
        }

        let mut tags_by_memory: HashMap<String, Vec<String>> = HashMap::new();
        let tag_rows = sqlx::query_as::<_, (String, String)>(
            r#"
            SELECT mt.memory_id, mt.tag FROM memory_tags mt
            INNER JOIN memories m ON m.id = mt.memory_id
            WHERE m.deleted_at IS NULL
              AND (m.project_id = ? OR m.project_id IS NULL)
            ORDER BY mt.tag ASC
            "#,
        )
        .bind(project_id)
        .fetch_all(&self.pool)
        .await?;
        for (memory_id, tag) in tag_rows {
            tags_by_memory.entry(memory_id).or_default().push(tag);
        }

        let mut markdown = String::from("# Gaggle Memory\n");

        for memory in memories {
            let title = inline_text(&memory.title);
            markdown.push_str(&format!("\n## {title}\n\n"));
            markdown.push_str(&format!("- **Kind:** `{}`\n", memory.kind));
            let scope = if memory.project_id.is_some() {
                "project"
            } else {
                "global"
            };
            markdown.push_str(&format!("- **Scope:** {scope}\n"));
            markdown.push_str(&format!(
                "- **Pinned:** {}\n",
                if memory.pinned == 1 { "yes" } else { "no" }
            ));
            let tags = tags_by_memory
                .get(&memory.id)
                .map(|tags| {
                    tags.iter()
                        .map(|tag| format!("`{}`", inline_text(tag)))
                        .collect::<Vec<_>>()
                        .join(", ")
                })
                .filter(|tags| !tags.is_empty())
                .unwrap_or_else(|| "(none)".to_string());
            markdown.push_str(&format!("- **Tags:** {tags}\n"));
            markdown.push_str(&format!("- **ID:** `{}`\n", memory.id));
            markdown.push_str(&format!("- **Updated:** {} (unix ms)\n", memory.updated_at));
            markdown.push('\n');
            markdown.push_str(&escape_markdown_content(&redact_secrets(&memory.content)));
            markdown.push('\n');
        }

        Ok(markdown)
    }
}

/// Bounds a caller-supplied limit to a safe range.
fn clamp_limit(limit: i64) -> i64 {
    limit.clamp(1, MAX_LIMIT)
}

fn bool_to_int(value: bool) -> i64 {
    i64::from(value)
}

fn validate_kind(kind: &str) -> Result<()> {
    if ALLOWED_KINDS.contains(&kind) {
        Ok(())
    } else {
        bail!(
            "Invalid memory kind; expected one of: {}",
            ALLOWED_KINDS.join(", ")
        )
    }
}

/// Trims a required text field and rejects it when empty.
fn normalize_text<'a>(field: &str, value: &'a str) -> Result<&'a str> {
    let trimmed = value.trim();
    if trimmed.is_empty() {
        bail!("Memory {field} must not be empty");
    }
    Ok(trimmed)
}

/// Trims, drops empty, and deduplicates tags while enforcing size limits.
fn normalize_tags(tags: &[String]) -> Result<Vec<String>> {
    let mut normalized: Vec<String> = Vec::new();
    for tag in tags {
        let trimmed = tag.trim();
        if trimmed.is_empty() {
            continue;
        }
        if trimmed.chars().count() > MAX_TAG_LEN {
            bail!("Memory tags must be at most {MAX_TAG_LEN} characters");
        }
        if !normalized.iter().any(|existing| existing == trimmed) {
            normalized.push(trimmed.to_string());
        }
    }
    if normalized.len() > MAX_TAGS {
        bail!("A memory can have at most {MAX_TAGS} tags");
    }
    Ok(normalized)
}

async fn insert_tags(
    tx: &mut Transaction<'_, Sqlite>,
    memory_id: &str,
    tags: &[String],
) -> Result<()> {
    for tag in tags {
        sqlx::query("INSERT INTO memory_tags (memory_id, tag) VALUES (?, ?)")
            .bind(memory_id)
            .bind(tag)
            .execute(&mut **tx)
            .await?;
    }
    Ok(())
}

/// Builds a safe FTS5 MATCH expression from untrusted user input.
///
/// Input is split on whitespace, non `[A-Za-z0-9_-]` characters are removed,
/// terms are deduplicated and capped, and every term is quoted so FTS5 cannot
/// interpret user text as operators or syntax.
fn build_fts_query(query: &str) -> Option<String> {
    let mut terms: Vec<String> = Vec::new();

    for raw in query.split_whitespace() {
        let cleaned: String = raw
            .chars()
            .filter(|c| c.is_alphanumeric() || matches!(c, '_' | '-'))
            .collect();
        if cleaned.is_empty() || !cleaned.chars().any(|c| c.is_alphanumeric()) {
            continue;
        }
        if terms
            .iter()
            .any(|existing| existing.eq_ignore_ascii_case(&cleaned))
        {
            continue;
        }
        terms.push(cleaned);
        if terms.len() >= MAX_FTS_TERMS {
            break;
        }
    }

    if terms.is_empty() {
        return None;
    }

    Some(
        terms
            .iter()
            .map(|term| format!("\"{term}\""))
            .collect::<Vec<_>>()
            .join(" OR "),
    )
}

/// Redacts obvious credential material from exported text.
fn redact_secrets(text: &str) -> String {
    let upper = text.to_ascii_uppercase();
    if upper.contains("-----BEGIN") && upper.contains("PRIVATE KEY") {
        return format!("{REDACTED} private key block");
    }

    text.split('\n')
        .map(redact_line)
        .collect::<Vec<_>>()
        .join("\n")
}

fn redact_line(line: &str) -> String {
    let mut out = String::with_capacity(line.len());
    let mut token = String::new();
    let mut redact_rest = false;

    for ch in line.chars() {
        if ch.is_whitespace() {
            push_redacted_token(&mut out, &token, &mut redact_rest);
            token.clear();
            out.push(ch);
        } else {
            token.push(ch);
        }
    }
    push_redacted_token(&mut out, &token, &mut redact_rest);

    out
}

fn push_redacted_token(out: &mut String, token: &str, redact_rest: &mut bool) {
    if token.is_empty() {
        return;
    }
    if *redact_rest || looks_like_secret_value(token) {
        out.push_str(REDACTED);
        return;
    }
    if let Some((prefix, _value)) = split_sensitive_pair(token) {
        out.push_str(prefix);
        out.push_str(REDACTED);
        *redact_rest = true;
        return;
    }
    if is_sensitive_marker(token) {
        *redact_rest = true;
    }
    out.push_str(token);
}

/// Splits a `key=value` / `key:value` token whose key is sensitive, returning
/// the original key plus separator so only the value is redacted.
fn split_sensitive_pair(token: &str) -> Option<(&str, &str)> {
    let separator = token.find(['=', ':'])?;
    let key = &token[..separator];
    if is_sensitive_key(key) {
        Some((&token[..=separator], &token[separator + 1..]))
    } else {
        None
    }
}

fn looks_like_secret_value(token: &str) -> bool {
    let trimmed = token.trim_matches(|c: char| !c.is_alphanumeric() && !matches!(c, '-' | '_'));
    let lower = trimmed.to_ascii_lowercase();
    lower.starts_with("sk-")
        || lower.starts_with("sk_")
        || lower.starts_with("ghp_")
        || lower.starts_with("xoxb-")
        || lower.starts_with("akia")
        || lower.starts_with("eyj")
}

fn is_sensitive_marker(token: &str) -> bool {
    let lower = token.to_ascii_lowercase();
    let trimmed = trim_key(&lower);
    if trimmed.is_empty() {
        return false;
    }
    if trimmed == "bearer" || trimmed == "authorization" {
        return true;
    }
    let has_separator = lower.ends_with(':') || lower.ends_with('=');
    has_separator && SENSITIVE_KEYS.contains(&trimmed)
}

fn is_sensitive_key(token: &str) -> bool {
    let lower = token.to_ascii_lowercase();
    let trimmed = trim_key(&lower);
    !trimmed.is_empty() && SENSITIVE_KEYS.contains(&trimmed)
}

fn trim_key(value: &str) -> &str {
    value.trim_matches(|c: char| !c.is_alphanumeric() && !matches!(c, '_' | '-'))
}

/// Escapes line-leading Markdown that could otherwise change document
/// structure (headings, lists, quotes, fences, tables).
fn escape_markdown_content(text: &str) -> String {
    text.lines()
        .map(|line| {
            let line = line.trim_end();
            if line.starts_with(['#', '>', '-', '+', '*', '|', '=', '`']) {
                format!("\\{line}")
            } else {
                line.to_string()
            }
        })
        .collect::<Vec<_>>()
        .join("\n")
}

/// Renders single-line text (title, tag) safely for use inside a Markdown line.
fn inline_text(text: &str) -> String {
    let single_line = text.replace(['\r', '\n'], " ");
    let single_line = single_line.trim();
    redact_secrets(single_line)
        .replace('`', "'")
        .trim()
        .to_string()
}
