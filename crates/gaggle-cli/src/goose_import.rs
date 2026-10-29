//! Read-only migration of one Goose SQLite session into a Gaggle project.

use anyhow::{Context, Result, bail};
use gaggle_core::observability::redact_log_text;
use gaggle_db::{
    connection::{new_id, now_ms},
    models::{Chat, Message, MessagePart},
    repositories::chats::{
        CHAT_BUNDLE_FORMAT, CHAT_BUNDLE_VERSION, ChatBundle, ChatImportResult, ChatRepository,
        ImportedChatArtifacts, ImportedMessageMetadata, ImportedUsageRecord,
    },
};
use serde::Serialize;
use serde_json::Value;
use sqlx::{FromRow, SqlitePool, sqlite::SqliteConnectOptions, sqlite::SqlitePoolOptions};
use std::{
    collections::HashMap,
    path::{Path, PathBuf},
};

const MAX_SESSION_MESSAGES: i64 = 20_000;
const MAX_SESSION_CONTENT_BYTES: i64 = 64 * 1024 * 1024;
const SUPPORTED_GOOSE_SCHEMA_VERSION: i64 = 17;

#[derive(Debug, Clone, Serialize)]
pub struct GooseSessionImportSummary {
    pub source_database: PathBuf,
    pub source_session_id: String,
    pub source_working_dir: String,
    pub project_id: String,
    pub result: ChatImportResult,
    pub provider_configuration_migrated: bool,
    pub credentials_imported: bool,
    pub metadata_records: usize,
    pub usage_records: usize,
    pub metadata_values_redacted: bool,
}

#[derive(FromRow)]
struct GooseSessionRow {
    id: String,
    name: String,
    working_dir: String,
    session_type: String,
    parent_session_id: Option<String>,
    system_prompt_override: Option<String>,
    user_set_name: Option<i64>,
    created_at_seconds: Option<i64>,
    extension_data: String,
    provider_name: Option<String>,
    model_config_json: Option<String>,
    accumulated_input_tokens: Option<i64>,
    accumulated_output_tokens: Option<i64>,
}

#[derive(FromRow)]
struct GooseMessageRow {
    id: i64,
    role: String,
    content_json: String,
    created_timestamp: i64,
    metadata_json: Option<String>,
}

#[derive(FromRow)]
struct GooseUsageRow {
    id: i64,
    created_timestamp: i64,
    model: Option<String>,
    input_tokens: Option<i64>,
    output_tokens: Option<i64>,
    total_tokens: Option<i64>,
    cache_read_tokens: Option<i64>,
    cache_write_tokens: Option<i64>,
    cost: Option<f64>,
    cost_source: Option<String>,
    is_compaction: Option<i64>,
}

/// Imports a single Goose session without modifying the Goose database.
///
/// Provider configuration is intentionally not copied: Goose's session data may
/// refer to credentials and provider-specific model settings that are not valid
/// Gaggle secret references.
pub async fn import_goose_session(
    gaggle_pool: &SqlitePool,
    source_database: &Path,
    source_session_id: &str,
    project_id: &str,
) -> Result<GooseSessionImportSummary> {
    import_goose_session_with_credentials(
        gaggle_pool,
        source_database,
        source_session_id,
        project_id,
        false,
    )
    .await
}

pub async fn import_goose_session_with_credentials(
    gaggle_pool: &SqlitePool,
    source_database: &Path,
    source_session_id: &str,
    project_id: &str,
    import_credentials: bool,
) -> Result<GooseSessionImportSummary> {
    if !source_database.is_file() {
        bail!(
            "Goose database is not a regular file: {}",
            source_database.display()
        );
    }

    let options = SqliteConnectOptions::new()
        .filename(source_database)
        .read_only(true)
        .create_if_missing(false);
    let source_pool = SqlitePoolOptions::new()
        .max_connections(1)
        .connect_with(options)
        .await
        .with_context(|| {
            format!(
                "Could not open Goose database read-only at {}",
                source_database.display()
            )
        })?;

    let schema_version: Option<i64> = sqlx::query_scalar("SELECT MAX(version) FROM schema_version")
        .fetch_one(&source_pool)
        .await?;
    if schema_version != Some(SUPPORTED_GOOSE_SCHEMA_VERSION) {
        bail!(
            "Unsupported Goose database schema version {:?}; this importer supports version {}",
            schema_version,
            SUPPORTED_GOOSE_SCHEMA_VERSION
        );
    }

    let session = sqlx::query_as::<_, GooseSessionRow>(
        r#"
        SELECT id, name, working_dir, session_type, parent_session_id,
               system_prompt_override, user_set_name,
               CAST(strftime('%s', created_at) AS INTEGER) AS created_at_seconds,
               COALESCE(extension_data, '{}') AS extension_data,
               provider_name, model_config_json,
               accumulated_input_tokens, accumulated_output_tokens
        FROM sessions WHERE id = ?
        "#,
    )
    .bind(source_session_id)
    .fetch_optional(&source_pool)
    .await?
    .with_context(|| format!("Goose session '{source_session_id}' was not found"))?;

    let (message_count, content_bytes): (i64, i64) = sqlx::query_as(
        "SELECT COUNT(*), COALESCE(SUM(length(CAST(content_json AS BLOB)) + COALESCE(length(CAST(metadata_json AS BLOB)), 0)), 0) FROM messages WHERE session_id = ?",
    )
    .bind(source_session_id)
    .fetch_one(&source_pool)
    .await?;
    if message_count > MAX_SESSION_MESSAGES {
        bail!(
            "Goose session contains {message_count} messages; maximum supported is {MAX_SESSION_MESSAGES}"
        );
    }
    if content_bytes > MAX_SESSION_CONTENT_BYTES {
        bail!(
            "Goose session content is {content_bytes} bytes; maximum supported is {MAX_SESSION_CONTENT_BYTES}"
        );
    }
    let session_metadata_bytes = session
        .extension_data
        .len()
        .saturating_add(session.model_config_json.as_ref().map_or(0, String::len));
    if usize::try_from(content_bytes)
        .unwrap_or(usize::MAX)
        .saturating_add(session_metadata_bytes)
        > MAX_SESSION_CONTENT_BYTES as usize
    {
        bail!("Goose session transcript and metadata exceed the import size limit");
    }

    let source_messages = sqlx::query_as::<_, GooseMessageRow>(
        "SELECT id, role, content_json, created_timestamp, metadata_json FROM messages WHERE session_id = ? ORDER BY created_timestamp, id",
    )
    .bind(source_session_id)
    .fetch_all(&source_pool)
    .await?;
    let usage_count: i64 =
        sqlx::query_scalar("SELECT COUNT(*) FROM usage_ledger WHERE session_id = ?")
            .bind(source_session_id)
            .fetch_one(&source_pool)
            .await?;
    if usage_count > 100_000 {
        bail!("Goose session has too many usage records to import safely");
    }
    let source_usage = sqlx::query_as::<_, GooseUsageRow>(
        r#"SELECT id, created_timestamp, model, input_tokens, output_tokens,
                  total_tokens, cache_read_tokens, cache_write_tokens,
                  cost, cost_source, is_compaction
           FROM usage_ledger WHERE session_id = ? ORDER BY created_timestamp, id"#,
    )
    .bind(source_session_id)
    .fetch_all(&source_pool)
    .await?;
    source_pool.close().await;

    let now = now_ms();
    let chat_id = new_id();
    let created_at = session
        .created_at_seconds
        .map(|seconds| seconds.saturating_mul(1000))
        .filter(|timestamp| *timestamp > 0)
        .or_else(|| {
            source_messages
                .first()
                .map(|message| message.created_timestamp.saturating_mul(1000))
        })
        .unwrap_or(now);
    let title = truncate_title(if session.name.trim().is_empty() {
        "Imported Goose session"
    } else {
        session.name.trim()
    });
    let chat = Chat {
        id: chat_id.clone(),
        project_id: project_id.to_string(),
        model_id: None,
        worktree_id: None,
        parent_chat_id: None,
        parent_tool_call_id: None,
        recipe_id: None,
        schedule_id: None,
        chat_type: if session.parent_session_id.is_some() {
            "subagent"
        } else {
            match session.session_type.as_str() {
                "subagent" => "subagent",
                "scheduled" => "scheduled",
                "hidden" => "hidden",
                "terminal" => "terminal",
                _ => "user",
            }
        }
        .to_string(),
        title,
        title_user_set: session.user_set_name.unwrap_or(0).clamp(0, 1),
        mode: "local".to_string(),
        permission_mode: "ask".to_string(),
        recipe_values: None,
        compact_threshold: 0.8,
        system_prompt_override: session.system_prompt_override,
        total_input_tokens: 0,
        total_output_tokens: 0,
        pinned: 0,
        archived_at: None,
        created_at,
        updated_at: now,
    };

    let parsed_messages = source_messages
        .into_iter()
        .map(|message| {
            let content: Vec<Value> =
                serde_json::from_str(&message.content_json).with_context(|| {
                    format!("Goose message {} has invalid content_json", message.id)
                })?;
            Ok((message, content))
        })
        .collect::<Result<Vec<_>>>()?;
    let tool_ids = collect_tool_call_ids(&parsed_messages);
    let mut messages = Vec::with_capacity(parsed_messages.len());
    let mut message_parts = Vec::new();
    let mut message_metadata = Vec::new();
    let mut metadata_values_redacted = false;

    for (seq, (source_message, blocks)) in parsed_messages.into_iter().enumerate() {
        let role = match source_message.role.as_str() {
            "user" | "assistant" | "system" | "tool" => source_message.role,
            unsupported => bail!("Goose session contains unsupported message role '{unsupported}'"),
        };
        let message_id = new_id();
        messages.push(Message {
            id: message_id.clone(),
            chat_id: chat_id.clone(),
            run_id: None,
            parent_id: None,
            seq: seq as i64 + 1,
            role,
            created_at: source_message.created_timestamp.saturating_mul(1000),
        });
        if let Some(metadata_json) = source_message.metadata_json.as_deref() {
            let payload = parse_and_redact_metadata(metadata_json, &mut metadata_values_redacted);
            message_metadata.push(ImportedMessageMetadata {
                source_message_id: source_message.id.to_string(),
                message_seq: seq as i64 + 1,
                payload_json: serde_json::to_string(&payload)?,
            });
        }

        for (idx, block) in blocks.iter().enumerate() {
            let (kind, content, tool_call_id) = map_content_block(block, &tool_ids)?;
            message_parts.push(MessagePart {
                id: new_id(),
                message_id: message_id.clone(),
                idx: idx as i64,
                kind,
                content: Some(content),
                tool_call_id,
                attachment_id: None,
            });
        }
    }

    let extension_data =
        parse_and_redact_metadata(&session.extension_data, &mut metadata_values_redacted);
    let model_config = session
        .model_config_json
        .as_deref()
        .map(|json| parse_and_redact_metadata(json, &mut metadata_values_redacted));
    let session_metadata_json = serde_json::to_string(&serde_json::json!({
        "working_dir": session.working_dir,
        "session_type": session.session_type,
        "provider_name": session.provider_name,
        "model_config": model_config,
        "extension_data": extension_data,
    }))?;
    let usage_records = source_usage
        .into_iter()
        .map(|row| ImportedUsageRecord {
            source_record_id: row.id.to_string(),
            model_key: row.model,
            input_tokens: non_negative(row.input_tokens),
            output_tokens: non_negative(row.output_tokens),
            total_tokens: non_negative(row.total_tokens),
            cache_read_tokens: non_negative(row.cache_read_tokens),
            cache_write_tokens: non_negative(row.cache_write_tokens),
            cost_usd: row.cost.filter(|cost| cost.is_finite()),
            cost_source: match row.cost_source.as_deref() {
                Some("provider_reported") => "provider_reported",
                Some("estimated") => "estimated",
                _ => "unknown",
            }
            .to_string(),
            is_compaction: row.is_compaction.unwrap_or(0) != 0,
            created_at: row.created_timestamp.saturating_mul(1000).max(0),
        })
        .collect::<Vec<_>>();
    let total_input_tokens = session
        .accumulated_input_tokens
        .map(|value| value.max(0))
        .unwrap_or_else(|| {
            usage_records
                .iter()
                .fold(0_i64, |sum, row| sum.saturating_add(row.input_tokens))
        });
    let total_output_tokens = session
        .accumulated_output_tokens
        .map(|value| value.max(0))
        .unwrap_or_else(|| {
            usage_records
                .iter()
                .fold(0_i64, |sum, row| sum.saturating_add(row.output_tokens))
        });
    let artifacts = ImportedChatArtifacts {
        source_session_id: session.id.clone(),
        session_metadata_json,
        message_metadata,
        usage_records,
        total_input_tokens,
        total_output_tokens,
    };
    let metadata_records = artifacts.message_metadata.len() + 1;
    let usage_record_count = artifacts.usage_records.len();
    let bundle = ChatBundle {
        format: CHAT_BUNDLE_FORMAT.to_string(),
        version: CHAT_BUNDLE_VERSION,
        exported_at: now,
        chat,
        messages,
        message_parts,
        imported_artifacts: Some(artifacts),
    };
    bundle.validate()?;
    let result = ChatRepository::new(gaggle_pool.clone())
        .import_chat(project_id, &bundle)
        .await?;

    Ok(GooseSessionImportSummary {
        source_database: source_database.to_path_buf(),
        source_session_id: session.id,
        source_working_dir: session.working_dir,
        project_id: project_id.to_string(),
        result,
        provider_configuration_migrated: false,
        credentials_imported: _import_credentials,
        metadata_records,
        usage_records: usage_record_count,
        metadata_values_redacted,
    })
}

fn non_negative(value: Option<i64>) -> i64 {
    value.unwrap_or(0).max(0)
}

fn parse_and_redact_metadata(raw: &str, redacted: &mut bool) -> Value {
    let value = serde_json::from_str(raw).unwrap_or_else(|_| Value::String(raw.to_string()));
    redact_metadata_value(value, redacted)
}

fn redact_metadata_value(value: Value, redacted: &mut bool) -> Value {
    match value {
        Value::Object(object) => {
            let mut safe = serde_json::Map::with_capacity(object.len());
            for (key, value) in object {
                if is_sensitive_metadata_key(&key) {
                    *redacted = true;
                    safe.insert(key, Value::String("[REDACTED]".to_string()));
                } else {
                    safe.insert(key, redact_metadata_value(value, redacted));
                }
            }
            Value::Object(safe)
        }
        Value::Array(items) => Value::Array(
            items
                .into_iter()
                .map(|item| redact_metadata_value(item, redacted))
                .collect(),
        ),
        Value::String(value) => {
            let safe = redact_log_text(&value);
            if safe != value {
                *redacted = true;
            }
            Value::String(safe)
        }
        value => value,
    }
}

fn is_sensitive_metadata_key(key: &str) -> bool {
    let key = key.to_ascii_lowercase().replace('-', "_").replace('.', "_");
    [
        "secret",
        "token",
        "password",
        "credential",
        "api_key",
        "apikey",
        "authorization",
        "private_key",
        "client_secret",
    ]
    .iter()
    .any(|sensitive| key.contains(sensitive))
}

fn collect_tool_call_ids(messages: &[(GooseMessageRow, Vec<Value>)]) -> HashMap<String, String> {
    let mut ids = HashMap::new();
    for (_, blocks) in messages {
        for block in blocks {
            if matches!(
                block.get("type").and_then(Value::as_str),
                Some("toolRequest" | "toolResponse")
            ) && let Some(source_id) = block.get("id").and_then(Value::as_str)
            {
                ids.entry(source_id.to_string()).or_insert_with(new_id);
            }
        }
    }
    ids
}

fn map_content_block(
    block: &Value,
    tool_ids: &HashMap<String, String>,
) -> Result<(String, String, Option<String>)> {
    let block_type = block
        .get("type")
        .and_then(Value::as_str)
        .context("Goose message content block has no type")?;
    let (kind, content) = match block_type {
        "text" => (
            "text",
            block
                .get("text")
                .and_then(Value::as_str)
                .unwrap_or_default()
                .to_string(),
        ),
        "thinking" | "reasoning" => (
            "reasoning",
            block
                .get("thinking")
                .or_else(|| block.get("text"))
                .and_then(Value::as_str)
                .unwrap_or_default()
                .to_string(),
        ),
        "toolRequest" => ("tool_call", serde_json::to_string(block)?),
        "toolResponse" => ("tool_result", serde_json::to_string(block)?),
        "image" | "document" => ("attachment", serde_json::to_string(block)?),
        _ => ("text", serde_json::to_string(block)?),
    };
    let tool_call_id = if matches!(block_type, "toolRequest" | "toolResponse") {
        block
            .get("id")
            .and_then(Value::as_str)
            .and_then(|id| tool_ids.get(id).cloned())
    } else {
        None
    };

    Ok((kind.to_string(), content, tool_call_id))
}

fn truncate_title(title: &str) -> String {
    let title: String = title.chars().take(500).collect();
    if title.trim().is_empty() {
        "Imported Goose session".to_string()
    } else {
        title
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn preserves_goose_tool_linkage_with_gaggle_compatible_ids() {
        let blocks = vec![
            serde_json::json!({"type":"toolRequest","id":"goose-call-1"}),
            serde_json::json!({"type":"toolResponse","id":"goose-call-1"}),
        ];
        let messages = vec![(
            GooseMessageRow {
                id: 1,
                role: "assistant".to_string(),
                content_json: String::new(),
                created_timestamp: 1,
                metadata_json: None,
            },
            blocks.clone(),
        )];
        let ids = collect_tool_call_ids(&messages);
        let request = map_content_block(&blocks[0], &ids).unwrap();
        let response = map_content_block(&blocks[1], &ids).unwrap();

        assert_eq!(request.0, "tool_call");
        assert_eq!(response.0, "tool_result");
        assert_eq!(request.2, response.2);
        assert!(request.2.as_deref().unwrap().len() == 36);
    }

    #[test]
    fn redacts_secrets_in_opaque_metadata_without_dropping_other_fields() {
        let mut redacted = false;
        let value = parse_and_redact_metadata(
            r#"{"extension":"github","enabled":true,"api_key":"secret-value"}"#,
            &mut redacted,
        );

        assert!(redacted);
        assert_eq!(value["extension"], "github");
        assert_eq!(value["enabled"], true);
        assert_eq!(value["api_key"], "[REDACTED]");
    }
}
