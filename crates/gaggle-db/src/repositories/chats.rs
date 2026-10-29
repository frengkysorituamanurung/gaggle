use crate::{
    connection::{new_id, now_ms},
    models::{Chat, ChatSummary, Message, MessagePart},
};
use anyhow::{Context, Result, bail};
use serde::{Deserialize, Serialize};
use sqlx::{FromRow, SqlitePool};
use std::collections::{HashMap, HashSet};

/// Hard upper bound for [`ChatRepository::list_messages_with_parts`] to keep context
/// assembly reads bounded even if a caller passes an unbounded limit.
const MAX_HISTORY_MESSAGES_PER_QUERY: i64 = 500;

/// Format identifier written into portable chat bundle files.
pub const CHAT_BUNDLE_FORMAT: &str = "gaggle.chat-bundle";
/// Version of the portable chat bundle schema.
pub const CHAT_BUNDLE_VERSION: u32 = 1;

/// Maximum number of messages accepted from a single imported bundle.
const MAX_BUNDLE_MESSAGES: usize = 20_000;
/// Maximum number of message parts accepted from a single imported bundle.
const MAX_BUNDLE_PARTS: usize = 100_000;
const MAX_IMPORTED_USAGE_RECORDS: usize = 100_000;
/// Maximum chat title length, in characters.
const MAX_TITLE_CHARS: usize = 500;
/// Maximum length, in characters, of a single message-part body.
const MAX_PART_CHARS: usize = 8 * 1024 * 1024;
/// Maximum length, in characters, of an identifier field.
const MAX_ID_CHARS: usize = 128;

const ALLOWED_ROLES: [&str; 4] = ["user", "assistant", "system", "tool"];
const ALLOWED_PART_KINDS: [&str; 5] = [
    "text",
    "reasoning",
    "tool_call",
    "tool_result",
    "attachment",
];
const ALLOWED_CHAT_TYPES: [&str; 5] = ["user", "subagent", "scheduled", "hidden", "terminal"];
const ALLOWED_MODES: [&str; 3] = ["local", "worktree", "readonly"];
const ALLOWED_PERMISSION_MODES: [&str; 4] = ["full", "ask", "smart", "chat_only"];

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChatExportBundle {
    pub chat: Chat,
    pub messages: Vec<Message>,
    pub message_parts: Vec<MessagePart>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub imported_artifacts: Option<ImportedChatArtifacts>,
}

/// Portable, versioned chat bundle exchanged by `gaggle export chat` and
/// `gaggle import chat`.
///
/// The envelope carries `format`, `version`, and `exported_at` metadata while
/// remaining compatible with bundles that only contain `chat`, `messages`, and
/// `message_parts` (missing metadata is defaulted and then validated).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChatBundle {
    #[serde(default = "default_bundle_format")]
    pub format: String,
    #[serde(default = "default_bundle_version")]
    pub version: u32,
    #[serde(default)]
    pub exported_at: i64,
    pub chat: Chat,
    pub messages: Vec<Message>,
    #[serde(default)]
    pub message_parts: Vec<MessagePart>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub imported_artifacts: Option<ImportedChatArtifacts>,
}

impl ChatBundle {
    /// Wraps an existing repository export in a fresh, versioned bundle.
    pub fn from_export(bundle: ChatExportBundle) -> Self {
        Self {
            format: CHAT_BUNDLE_FORMAT.to_string(),
            version: CHAT_BUNDLE_VERSION,
            exported_at: now_ms(),
            chat: bundle.chat,
            messages: bundle.messages,
            message_parts: bundle.message_parts,
            imported_artifacts: bundle.imported_artifacts,
        }
    }

    /// Validates structural, referential, and enum constraints before import.
    pub fn validate(&self) -> Result<()> {
        validate_chat_bundle(self)?;
        if let Some(artifacts) = &self.imported_artifacts {
            validate_imported_artifacts(self, artifacts)?;
        }
        Ok(())
    }
}

/// Outcome of importing a chat bundle into an existing project.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChatImportResult {
    pub chat: Chat,
    pub message_count: usize,
    pub part_count: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ImportedChatArtifacts {
    pub source_session_id: String,
    pub session_metadata_json: String,
    pub message_metadata: Vec<ImportedMessageMetadata>,
    pub usage_records: Vec<ImportedUsageRecord>,
    pub total_input_tokens: i64,
    pub total_output_tokens: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct ImportedMessageMetadata {
    pub source_message_id: String,
    pub message_seq: i64,
    pub payload_json: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ImportedUsageRecord {
    pub source_record_id: String,
    pub model_key: Option<String>,
    pub input_tokens: i64,
    pub output_tokens: i64,
    pub total_tokens: i64,
    pub cache_read_tokens: i64,
    pub cache_write_tokens: i64,
    pub cost_usd: Option<f64>,
    pub cost_source: String,
    pub is_compaction: bool,
    pub created_at: i64,
}

#[derive(FromRow)]
struct ImportedUsageRow {
    source_record_id: String,
    model_key: Option<String>,
    input_tokens: i64,
    output_tokens: i64,
    total_tokens: i64,
    cache_read_tokens: i64,
    cache_write_tokens: i64,
    cost_usd: Option<f64>,
    cost_source: String,
    is_compaction: i64,
    created_at: i64,
}

fn default_bundle_format() -> String {
    CHAT_BUNDLE_FORMAT.to_string()
}

fn default_bundle_version() -> u32 {
    CHAT_BUNDLE_VERSION
}

pub struct ChatRepository {
    pool: SqlitePool,
}

impl ChatRepository {
    pub fn new(pool: SqlitePool) -> Self {
        Self { pool }
    }

    pub async fn create(
        &self,
        project_id: &str,
        title: &str,
        chat_type: &str,
        mode: &str,
        permission_mode: &str,
    ) -> Result<Chat> {
        let id = new_id();
        let now = now_ms();

        sqlx::query(
            r#"
            INSERT INTO chats (
                id, project_id, chat_type, title, mode, permission_mode, created_at, updated_at
            ) VALUES (?, ?, ?, ?, ?, ?, ?, ?)
            "#,
        )
        .bind(&id)
        .bind(project_id)
        .bind(chat_type)
        .bind(title)
        .bind(mode)
        .bind(permission_mode)
        .bind(now)
        .bind(now)
        .execute(&self.pool)
        .await?;

        self.get_by_id(&id)
            .await?
            .ok_or_else(|| anyhow::anyhow!("Chat not found after insert"))
    }

    /// Creates a child chat (used by subagents/sub-recipes) with parent linkage.
    #[allow(clippy::too_many_arguments)]
    pub async fn create_child_chat(
        &self,
        project_id: &str,
        title: &str,
        chat_type: &str,
        mode: &str,
        permission_mode: &str,
        parent_chat_id: Option<&str>,
        parent_tool_call_id: Option<&str>,
    ) -> Result<Chat> {
        let id = new_id();
        let now = now_ms();

        sqlx::query(
            r#"
            INSERT INTO chats (
                id, project_id, chat_type, title, mode, permission_mode,
                parent_chat_id, parent_tool_call_id, created_at, updated_at
            ) VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?)
            "#,
        )
        .bind(&id)
        .bind(project_id)
        .bind(chat_type)
        .bind(title)
        .bind(mode)
        .bind(permission_mode)
        .bind(parent_chat_id)
        .bind(parent_tool_call_id)
        .bind(now)
        .bind(now)
        .execute(&self.pool)
        .await?;

        self.get_by_id(&id)
            .await?
            .ok_or_else(|| anyhow::anyhow!("Chat not found after insert"))
    }

    /// Convenience for creating a `chat_type = 'subagent'` child chat.
    pub async fn create_subagent_chat(
        &self,
        project_id: &str,
        title: &str,
        mode: &str,
        permission_mode: &str,
        parent_chat_id: &str,
        parent_tool_call_id: Option<&str>,
    ) -> Result<Chat> {
        self.create_child_chat(
            project_id,
            title,
            "subagent",
            mode,
            permission_mode,
            Some(parent_chat_id),
            parent_tool_call_id,
        )
        .await
    }

    /// Creates a chat carrying recipe/schedule context.
    #[allow(clippy::too_many_arguments)]
    pub async fn create_recipe_chat(
        &self,
        project_id: &str,
        title: &str,
        chat_type: &str,
        mode: &str,
        permission_mode: &str,
        recipe_id: Option<&str>,
        recipe_values: Option<&str>,
        schedule_id: Option<&str>,
        parent_chat_id: Option<&str>,
        parent_tool_call_id: Option<&str>,
    ) -> Result<Chat> {
        let id = new_id();
        let now = now_ms();

        sqlx::query(
            r#"
            INSERT INTO chats (
                id, project_id, chat_type, title, mode, permission_mode,
                recipe_id, recipe_values, schedule_id,
                parent_chat_id, parent_tool_call_id, created_at, updated_at
            ) VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)
            "#,
        )
        .bind(&id)
        .bind(project_id)
        .bind(chat_type)
        .bind(title)
        .bind(mode)
        .bind(permission_mode)
        .bind(recipe_id)
        .bind(recipe_values)
        .bind(schedule_id)
        .bind(parent_chat_id)
        .bind(parent_tool_call_id)
        .bind(now)
        .bind(now)
        .execute(&self.pool)
        .await?;

        self.get_by_id(&id)
            .await?
            .ok_or_else(|| anyhow::anyhow!("Chat not found after insert"))
    }

    pub async fn get_by_id(&self, id: &str) -> Result<Option<Chat>> {
        let chat = sqlx::query_as::<_, Chat>("SELECT * FROM chats WHERE id = ?")
            .bind(id)
            .fetch_optional(&self.pool)
            .await?;
        Ok(chat)
    }

    pub async fn list_by_project(&self, project_id: &str) -> Result<Vec<Chat>> {
        let chats = sqlx::query_as::<_, Chat>(
            "SELECT * FROM chats WHERE project_id = ? AND archived_at IS NULL ORDER BY updated_at DESC",
        )
        .bind(project_id)
        .fetch_all(&self.pool)
        .await?;
        Ok(chats)
    }

    pub async fn update_title(&self, chat_id: &str, title: &str) -> Result<Chat> {
        let now = now_ms();
        sqlx::query("UPDATE chats SET title = ?, title_user_set = 1, updated_at = ? WHERE id = ?")
            .bind(title)
            .bind(now)
            .bind(chat_id)
            .execute(&self.pool)
            .await?;

        self.get_by_id(chat_id)
            .await?
            .ok_or_else(|| anyhow::anyhow!("Chat not found after title update"))
    }

    pub async fn append_message(
        &self,
        chat_id: &str,
        run_id: Option<&str>,
        role: &str,
        text_content: &str,
    ) -> Result<Message> {
        let id = new_id();
        let part_id = new_id();
        let now = now_ms();

        let next_seq: i64 =
            sqlx::query_scalar("SELECT COALESCE(MAX(seq), 0) + 1 FROM messages WHERE chat_id = ?")
                .bind(chat_id)
                .fetch_one(&self.pool)
                .await?;

        let mut tx = self.pool.begin().await?;

        sqlx::query(
            r#"
            INSERT INTO messages (id, chat_id, run_id, seq, role, created_at)
            VALUES (?, ?, ?, ?, ?, ?)
            "#,
        )
        .bind(&id)
        .bind(chat_id)
        .bind(run_id)
        .bind(next_seq)
        .bind(role)
        .bind(now)
        .execute(&mut *tx)
        .await?;

        sqlx::query(
            r#"
            INSERT INTO message_parts (id, message_id, idx, kind, content)
            VALUES (?, ?, 0, 'text', ?)
            "#,
        )
        .bind(&part_id)
        .bind(&id)
        .bind(text_content)
        .execute(&mut *tx)
        .await?;

        sqlx::query("UPDATE chats SET updated_at = ? WHERE id = ?")
            .bind(now)
            .bind(chat_id)
            .execute(&mut *tx)
            .await?;

        tx.commit().await?;

        let msg = sqlx::query_as::<_, Message>("SELECT * FROM messages WHERE id = ?")
            .bind(&id)
            .fetch_one(&self.pool)
            .await?;
        Ok(msg)
    }

    /// Loads the most recent `limit` messages for a chat (ascending `seq` order) together
    /// with their ordered parts.
    ///
    /// Two queries are used (messages, then parts for the fetched message ids) to avoid an
    /// N+1 pattern. Messages without any parts are preserved with an empty `Vec`.
    pub async fn list_messages_with_parts(
        &self,
        chat_id: &str,
        limit: i64,
    ) -> Result<Vec<(Message, Vec<MessagePart>)>> {
        let capped_limit = limit.clamp(1, MAX_HISTORY_MESSAGES_PER_QUERY);

        let mut messages = sqlx::query_as::<_, Message>(
            "SELECT * FROM messages WHERE chat_id = ? ORDER BY seq DESC LIMIT ?",
        )
        .bind(chat_id)
        .bind(capped_limit)
        .fetch_all(&self.pool)
        .await?;

        // Query newest-first so the LIMIT keeps the most recent rows, then restore
        // chronological order for callers and model context.
        messages.reverse();

        if messages.is_empty() {
            return Ok(Vec::new());
        }

        let placeholders = std::iter::repeat_n("?", messages.len())
            .collect::<Vec<_>>()
            .join(", ");
        let parts_sql = format!(
            "SELECT * FROM message_parts WHERE message_id IN ({placeholders}) ORDER BY message_id ASC, idx ASC"
        );

        let mut query = sqlx::query_as::<_, MessagePart>(sqlx::AssertSqlSafe(parts_sql));
        for message in &messages {
            query = query.bind(&message.id);
        }
        let parts = query.fetch_all(&self.pool).await?;

        let mut parts_by_message: HashMap<String, Vec<MessagePart>> = HashMap::new();
        for part in parts {
            parts_by_message
                .entry(part.message_id.clone())
                .or_default()
                .push(part);
        }

        Ok(messages
            .into_iter()
            .map(|message| {
                let message_parts = parts_by_message.remove(&message.id).unwrap_or_default();
                (message, message_parts)
            })
            .collect())
    }

    pub async fn export_chat(&self, chat_id: &str) -> Result<Option<ChatExportBundle>> {
        let Some(chat) = self.get_by_id(chat_id).await? else {
            return Ok(None);
        };

        let messages = sqlx::query_as::<_, Message>(
            "SELECT * FROM messages WHERE chat_id = ? ORDER BY seq ASC",
        )
        .bind(chat_id)
        .fetch_all(&self.pool)
        .await?;

        let message_parts = sqlx::query_as::<_, MessagePart>(
            r#"
            SELECT mp.* FROM message_parts mp
            INNER JOIN messages m ON m.id = mp.message_id
            WHERE m.chat_id = ?
            ORDER BY m.seq ASC, mp.idx ASC
            "#,
        )
        .bind(chat_id)
        .fetch_all(&self.pool)
        .await?;

        let imported_artifacts = self.load_imported_artifacts(chat_id, &chat).await?;

        Ok(Some(ChatExportBundle {
            chat,
            messages,
            message_parts,
            imported_artifacts,
        }))
    }

    async fn load_imported_artifacts(
        &self,
        chat_id: &str,
        chat: &Chat,
    ) -> Result<Option<ImportedChatArtifacts>> {
        let session = sqlx::query_as::<_, (String, String, String)>(
            "SELECT source, source_session_id, payload FROM imported_session_metadata WHERE chat_id = ?",
        )
        .bind(chat_id)
        .fetch_optional(&self.pool)
        .await?;
        let Some((source, source_session_id, session_metadata_json)) = session else {
            return Ok(None);
        };
        if source != "goose" {
            return Ok(None);
        }

        let message_metadata = sqlx::query_as::<_, ImportedMessageMetadata>(
            r#"
            SELECT source_message_id, message_seq, payload AS payload_json
            FROM imported_message_metadata
            WHERE chat_id = ? AND source = 'goose'
            ORDER BY message_seq
            "#,
        )
        .bind(chat_id)
        .fetch_all(&self.pool)
        .await?;
        let usage_rows = sqlx::query_as::<_, ImportedUsageRow>(
            r#"
            SELECT source_record_id, model_key, input_tokens, output_tokens, total_tokens,
                   cache_read_tokens, cache_write_tokens, cost_usd, cost_source,
                   is_compaction, created_at
            FROM imported_usage_records
            WHERE chat_id = ? AND source = 'goose'
            ORDER BY created_at, source_record_id
            "#,
        )
        .bind(chat_id)
        .fetch_all(&self.pool)
        .await?;
        let usage_records = usage_rows
            .into_iter()
            .map(|row| ImportedUsageRecord {
                source_record_id: row.source_record_id,
                model_key: row.model_key,
                input_tokens: row.input_tokens,
                output_tokens: row.output_tokens,
                total_tokens: row.total_tokens,
                cache_read_tokens: row.cache_read_tokens,
                cache_write_tokens: row.cache_write_tokens,
                cost_usd: row.cost_usd,
                cost_source: row.cost_source,
                is_compaction: row.is_compaction != 0,
                created_at: row.created_at,
            })
            .collect();

        Ok(Some(ImportedChatArtifacts {
            source_session_id,
            session_metadata_json,
            message_metadata,
            usage_records,
            total_input_tokens: chat.total_input_tokens,
            total_output_tokens: chat.total_output_tokens,
        }))
    }

    /// Imports a validated chat bundle into `project_id` as a brand-new chat.
    ///
    /// Import never mutates or overwrites existing rows: a fresh UUIDv7 chat id
    /// and fresh message/part ids are generated, and all writes happen in a
    /// single short transaction. References to rows that are not part of the
    /// bundle (`runs`, `tool_calls`, `attachments`, worktrees, parent chats,
    /// schedules, and unknown models/recipes) are dropped instead of creating
    /// dangling foreign keys.
    pub async fn import_chat(
        &self,
        project_id: &str,
        bundle: &ChatBundle,
    ) -> Result<ChatImportResult> {
        self.import_chat_with_artifacts(project_id, bundle, bundle.imported_artifacts.as_ref())
            .await
    }

    /// Imports a chat and optional source artifacts in the same transaction.
    pub async fn import_chat_with_artifacts(
        &self,
        project_id: &str,
        bundle: &ChatBundle,
        artifacts: Option<&ImportedChatArtifacts>,
    ) -> Result<ChatImportResult> {
        bundle.validate()?;
        if let Some(artifacts) = artifacts {
            validate_imported_artifacts(bundle, artifacts)?;
        }

        let archived_at: Option<Option<i64>> =
            sqlx::query_scalar("SELECT archived_at FROM projects WHERE id = ?")
                .bind(project_id)
                .fetch_optional(&self.pool)
                .await?;
        let Some(archived_at) = archived_at else {
            bail!("Project {project_id} not found; cannot import chat");
        };
        if archived_at.is_some() {
            bail!("Project {project_id} is archived; cannot import chat into it");
        }

        let model_id = match bundle.chat.model_id.as_deref() {
            Some(candidate) if row_exists(&self.pool, "models", candidate).await? => {
                Some(candidate.to_string())
            }
            _ => None,
        };
        let recipe_id = match bundle.chat.recipe_id.as_deref() {
            Some(candidate) if row_exists(&self.pool, "recipes", candidate).await? => {
                Some(candidate.to_string())
            }
            _ => None,
        };
        let recipe_values = if recipe_id.is_some() {
            bundle
                .chat
                .recipe_values
                .as_deref()
                .filter(|value| serde_json::from_str::<serde_json::Value>(value).is_ok())
                .map(ToString::to_string)
        } else {
            None
        };

        let now = now_ms();
        let new_chat_id = new_id();
        let created_at = normalize_timestamp(bundle.chat.created_at, now);
        let title = bundle.chat.title.trim();

        let mut id_map: HashMap<&str, String> = HashMap::with_capacity(bundle.messages.len());
        for message in &bundle.messages {
            id_map.insert(message.id.as_str(), new_id());
        }

        let mut tx = self.pool.begin().await?;

        sqlx::query(
            r#"
            INSERT INTO chats (
                id, project_id, model_id, worktree_id, parent_chat_id, parent_tool_call_id,
                recipe_id, schedule_id, chat_type, title, title_user_set, mode, permission_mode,
                recipe_values, compact_threshold, system_prompt_override, total_input_tokens,
                total_output_tokens, pinned, archived_at, created_at, updated_at
            ) VALUES (?, ?, ?, NULL, NULL, NULL, ?, NULL, ?, ?, ?, ?, ?, ?, ?, ?, 0, 0, ?, NULL, ?, ?)
            "#,
        )
        .bind(&new_chat_id)
        .bind(project_id)
        .bind(&model_id)
        .bind(&recipe_id)
        .bind(&bundle.chat.chat_type)
        .bind(title)
        .bind(bundle.chat.title_user_set)
        .bind(&bundle.chat.mode)
        .bind(&bundle.chat.permission_mode)
        .bind(&recipe_values)
        .bind(bundle.chat.compact_threshold)
        .bind(&bundle.chat.system_prompt_override)
        .bind(bundle.chat.pinned)
        .bind(created_at)
        .bind(now)
        .execute(&mut *tx)
        .await?;

        for message in &bundle.messages {
            let new_message_id = id_map.get(message.id.as_str()).ok_or_else(|| {
                anyhow::anyhow!("Internal import error: missing mapped message id")
            })?;
            sqlx::query(
                r#"
                INSERT INTO messages (id, chat_id, run_id, parent_id, seq, role, created_at)
                VALUES (?, ?, NULL, NULL, ?, ?, ?)
                "#,
            )
            .bind(new_message_id)
            .bind(&new_chat_id)
            .bind(message.seq)
            .bind(&message.role)
            .bind(normalize_timestamp(message.created_at, now))
            .execute(&mut *tx)
            .await?;
        }

        for part in &bundle.message_parts {
            let new_message_id = id_map.get(part.message_id.as_str()).ok_or_else(|| {
                anyhow::anyhow!("Internal import error: missing mapped message id for part")
            })?;
            sqlx::query(
                r#"
                INSERT INTO message_parts (id, message_id, idx, kind, content, tool_call_id, attachment_id)
                VALUES (?, ?, ?, ?, ?, NULL, NULL)
                "#,
            )
            .bind(new_id())
            .bind(new_message_id)
            .bind(part.idx)
            .bind(&part.kind)
            .bind(&part.content)
            .execute(&mut *tx)
            .await?;
        }

        // Parent links can point forward in the sequence, so they are resolved
        // after every message row exists.
        for message in &bundle.messages {
            let Some(parent_id) = message.parent_id.as_deref() else {
                continue;
            };
            let (Some(new_message_id), Some(new_parent_id)) =
                (id_map.get(message.id.as_str()), id_map.get(parent_id))
            else {
                continue;
            };
            sqlx::query("UPDATE messages SET parent_id = ? WHERE id = ?")
                .bind(new_parent_id)
                .bind(new_message_id)
                .execute(&mut *tx)
                .await?;
        }

        if let Some(artifacts) = artifacts {
            sqlx::query(
                r#"
                INSERT INTO imported_session_metadata (
                    chat_id, source, source_session_id, payload, imported_at
                ) VALUES (?, 'goose', ?, ?, ?)
                "#,
            )
            .bind(&new_chat_id)
            .bind(&artifacts.source_session_id)
            .bind(&artifacts.session_metadata_json)
            .bind(now)
            .execute(&mut *tx)
            .await?;

            for metadata in &artifacts.message_metadata {
                sqlx::query(
                    r#"
                    INSERT INTO imported_message_metadata (
                        id, chat_id, source, source_message_id, message_seq, payload, imported_at
                    ) VALUES (?, ?, 'goose', ?, ?, ?, ?)
                    "#,
                )
                .bind(new_id())
                .bind(&new_chat_id)
                .bind(&metadata.source_message_id)
                .bind(metadata.message_seq)
                .bind(&metadata.payload_json)
                .bind(now)
                .execute(&mut *tx)
                .await?;
            }

            for usage in &artifacts.usage_records {
                sqlx::query(
                    r#"
                    INSERT INTO imported_usage_records (
                        id, chat_id, source, source_record_id, model_key,
                        input_tokens, output_tokens, total_tokens,
                        cache_read_tokens, cache_write_tokens, cost_usd,
                        cost_source, is_compaction, created_at
                    ) VALUES (?, ?, 'goose', ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)
                    "#,
                )
                .bind(new_id())
                .bind(&new_chat_id)
                .bind(&usage.source_record_id)
                .bind(&usage.model_key)
                .bind(usage.input_tokens)
                .bind(usage.output_tokens)
                .bind(usage.total_tokens)
                .bind(usage.cache_read_tokens)
                .bind(usage.cache_write_tokens)
                .bind(usage.cost_usd)
                .bind(&usage.cost_source)
                .bind(i64::from(usage.is_compaction))
                .bind(normalize_timestamp(usage.created_at, now))
                .execute(&mut *tx)
                .await?;
            }

            sqlx::query(
                "UPDATE chats SET total_input_tokens = ?, total_output_tokens = ? WHERE id = ?",
            )
            .bind(artifacts.total_input_tokens.max(0))
            .bind(artifacts.total_output_tokens.max(0))
            .bind(&new_chat_id)
            .execute(&mut *tx)
            .await?;
        }

        tx.commit().await?;

        let chat = self
            .get_by_id(&new_chat_id)
            .await?
            .ok_or_else(|| anyhow::anyhow!("Imported chat {new_chat_id} not found after insert"))?;

        Ok(ChatImportResult {
            chat,
            message_count: bundle.messages.len(),
            part_count: bundle.message_parts.len(),
        })
    }
}

fn validate_chat_bundle(bundle: &ChatBundle) -> Result<()> {
    if bundle.format != CHAT_BUNDLE_FORMAT {
        bail!(
            "Unsupported chat bundle format {:?}; expected {CHAT_BUNDLE_FORMAT:?}",
            bundle.format
        );
    }
    if bundle.version != CHAT_BUNDLE_VERSION {
        bail!(
            "Unsupported chat bundle version {}; this build supports version {CHAT_BUNDLE_VERSION}",
            bundle.version
        );
    }

    validate_id("chat.id", &bundle.chat.id)?;
    if bundle.chat.title.trim().is_empty() {
        bail!("Chat title must not be empty");
    }
    if bundle.chat.title.chars().count() > MAX_TITLE_CHARS {
        bail!("Chat title must be at most {MAX_TITLE_CHARS} characters");
    }
    if !matches!(bundle.chat.title_user_set, 0 | 1) {
        bail!("chat.title_user_set must be 0 or 1");
    }
    if !matches!(bundle.chat.pinned, 0 | 1) {
        bail!("chat.pinned must be 0 or 1");
    }
    if !bundle.chat.compact_threshold.is_finite()
        || !(0.0..=1.0).contains(&bundle.chat.compact_threshold)
    {
        bail!("chat.compact_threshold must be a finite value between 0.0 and 1.0");
    }
    if bundle.chat.created_at < 0 {
        bail!("chat.created_at must be a non-negative Unix timestamp in milliseconds");
    }
    validate_enum(
        "chat.chat_type",
        &bundle.chat.chat_type,
        &ALLOWED_CHAT_TYPES,
    )?;
    validate_enum("chat.mode", &bundle.chat.mode, &ALLOWED_MODES)?;
    validate_enum(
        "chat.permission_mode",
        &bundle.chat.permission_mode,
        &ALLOWED_PERMISSION_MODES,
    )?;

    if bundle.messages.len() > MAX_BUNDLE_MESSAGES {
        bail!(
            "Chat bundle contains {} messages; the maximum is {MAX_BUNDLE_MESSAGES}",
            bundle.messages.len()
        );
    }
    if bundle.message_parts.len() > MAX_BUNDLE_PARTS {
        bail!(
            "Chat bundle contains {} message parts; the maximum is {MAX_BUNDLE_PARTS}",
            bundle.message_parts.len()
        );
    }

    let mut message_ids: HashSet<&str> = HashSet::with_capacity(bundle.messages.len());
    let mut sequences: HashSet<i64> = HashSet::with_capacity(bundle.messages.len());

    for message in &bundle.messages {
        validate_id("message.id", &message.id)?;
        if !message_ids.insert(message.id.as_str()) {
            bail!("Duplicate message id {} in chat bundle", message.id);
        }
        if message.chat_id != bundle.chat.id {
            bail!(
                "Message {} belongs to chat {}, not the bundle chat {}",
                message.id,
                message.chat_id,
                bundle.chat.id
            );
        }
        if message.seq < 1 {
            bail!(
                "Message {} has invalid seq {}; seq must be >= 1",
                message.id,
                message.seq
            );
        }
        if !sequences.insert(message.seq) {
            bail!("Duplicate message seq {} in chat bundle", message.seq);
        }
        validate_enum("message.role", &message.role, &ALLOWED_ROLES)?;
        if message.created_at < 0 {
            bail!("Message {} has a negative created_at", message.id);
        }
        if let Some(parent_id) = message.parent_id.as_deref() {
            validate_id("message.parent_id", parent_id)?;
        }
        if message.id == bundle.chat.id {
            bail!("Message id {} collides with the chat id", message.id);
        }
    }

    let mut part_ids: HashSet<&str> = HashSet::with_capacity(bundle.message_parts.len());
    let mut part_slots: HashSet<(&str, i64)> = HashSet::with_capacity(bundle.message_parts.len());

    for part in &bundle.message_parts {
        validate_id("message_parts.id", &part.id)?;
        if !part_ids.insert(part.id.as_str()) {
            bail!("Duplicate message part id {} in chat bundle", part.id);
        }
        if !message_ids.contains(part.message_id.as_str()) {
            bail!(
                "Message part {} references unknown message {}",
                part.id,
                part.message_id
            );
        }
        if part.idx < 0 {
            bail!("Message part {} has invalid idx {}", part.id, part.idx);
        }
        if !part_slots.insert((part.message_id.as_str(), part.idx)) {
            bail!(
                "Duplicate message part index {} for message {}",
                part.idx,
                part.message_id
            );
        }
        validate_enum("message_parts.kind", &part.kind, &ALLOWED_PART_KINDS)?;
        if part
            .content
            .as_deref()
            .is_some_and(|content| content.chars().count() > MAX_PART_CHARS)
        {
            bail!(
                "Message part {} content exceeds {MAX_PART_CHARS} characters",
                part.id
            );
        }
        if let Some(tool_call_id) = part.tool_call_id.as_deref() {
            validate_id("message_parts.tool_call_id", tool_call_id)?;
        }
        if let Some(attachment_id) = part.attachment_id.as_deref() {
            validate_id("message_parts.attachment_id", attachment_id)?;
        }
    }

    Ok(())
}

fn validate_imported_artifacts(
    bundle: &ChatBundle,
    artifacts: &ImportedChatArtifacts,
) -> Result<()> {
    if artifacts.source_session_id.trim().is_empty()
        || artifacts.source_session_id.chars().count() > 512
    {
        bail!("Imported source session id must contain 1 to 512 characters");
    }
    let session_metadata: serde_json::Value =
        serde_json::from_str(&artifacts.session_metadata_json)
            .context("Imported session metadata must be valid JSON")?;
    validate_redacted_metadata(&session_metadata)?;
    if artifacts.total_input_tokens < 0 || artifacts.total_output_tokens < 0 {
        bail!("Imported session token totals must be non-negative");
    }
    if artifacts.usage_records.len() > MAX_IMPORTED_USAGE_RECORDS {
        bail!(
            "Imported usage ledger contains {}; maximum is {MAX_IMPORTED_USAGE_RECORDS}",
            artifacts.usage_records.len()
        );
    }

    let message_sequences: HashSet<i64> =
        bundle.messages.iter().map(|message| message.seq).collect();
    let mut imported_message_sequences = HashSet::new();
    for metadata in &artifacts.message_metadata {
        if metadata.source_message_id.trim().is_empty()
            || metadata.source_message_id.chars().count() > 512
        {
            bail!("Imported source message id must contain 1 to 512 characters");
        }
        if !message_sequences.contains(&metadata.message_seq)
            || !imported_message_sequences.insert(metadata.message_seq)
        {
            bail!(
                "Imported metadata references a missing or duplicate message sequence {}",
                metadata.message_seq
            );
        }
        let payload: serde_json::Value = serde_json::from_str(&metadata.payload_json)
            .context("Imported message metadata must be valid JSON")?;
        validate_redacted_metadata(&payload)?;
    }

    let mut usage_ids = HashSet::new();
    for usage in &artifacts.usage_records {
        if usage.source_record_id.trim().is_empty()
            || usage.source_record_id.chars().count() > MAX_ID_CHARS
        {
            bail!("Imported usage source id must contain 1 to {MAX_ID_CHARS} characters");
        }
        if !usage_ids.insert(usage.source_record_id.as_str()) {
            bail!(
                "Duplicate imported usage source id {}",
                usage.source_record_id
            );
        }
        if [
            usage.input_tokens,
            usage.output_tokens,
            usage.total_tokens,
            usage.cache_read_tokens,
            usage.cache_write_tokens,
        ]
        .iter()
        .any(|tokens| *tokens < 0)
        {
            bail!("Imported usage token counts must be non-negative");
        }
        if usage.cost_usd.is_some_and(|cost| !cost.is_finite()) {
            bail!("Imported usage cost must be finite");
        }
        validate_enum(
            "imported_usage_records.cost_source",
            &usage.cost_source,
            &["provider_reported", "estimated", "unknown"],
        )?;
        if usage.created_at < 0 {
            bail!("Imported usage timestamp must be non-negative");
        }
    }

    Ok(())
}

fn validate_redacted_metadata(value: &serde_json::Value) -> Result<()> {
    match value {
        serde_json::Value::Object(object) => {
            for (key, value) in object {
                if is_sensitive_metadata_key(key) && value.as_str() != Some("[REDACTED]") {
                    bail!("Imported metadata contains an unredacted sensitive field");
                }
                validate_redacted_metadata(value)?;
            }
        }
        serde_json::Value::Array(items) => {
            for item in items {
                validate_redacted_metadata(item)?;
            }
        }
        _ => {}
    }
    Ok(())
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

fn validate_id(field: &str, value: &str) -> Result<()> {
    if value.is_empty() || value.chars().count() > MAX_ID_CHARS {
        bail!("{field} must be a non-empty identifier of at most {MAX_ID_CHARS} characters");
    }
    if uuid::Uuid::parse_str(value).is_err() {
        bail!("{field} must be a UUID identifier, got {value:?}");
    }
    Ok(())
}

fn validate_enum(field: &str, value: &str, allowed: &[&str]) -> Result<()> {
    if allowed.contains(&value) {
        Ok(())
    } else {
        bail!(
            "Invalid {field} {value:?}; expected one of: {}",
            allowed.join(", ")
        )
    }
}

fn normalize_timestamp(value: i64, now: i64) -> i64 {
    if value > 0 { value.min(now) } else { now }
}

async fn row_exists(pool: &SqlitePool, table: &str, id: &str) -> Result<bool> {
    // `table` is a compile-time constant owned by this module, never user input.
    let sql = format!("SELECT 1 FROM {table} WHERE id = ?");
    let found: Option<i64> = sqlx::query_scalar(sqlx::AssertSqlSafe(sql))
        .bind(id)
        .fetch_optional(pool)
        .await?;
    Ok(found.is_some())
}

impl ChatRepository {
    pub async fn create_summary(
        &self,
        chat_id: &str,
        summary: &str,
        covers_until_seq: i64,
        tokens_before: i64,
        tokens_after: i64,
        trigger: &str,
    ) -> Result<ChatSummary> {
        ChatSummaryRepository::new(self.pool.clone())
            .create(
                chat_id,
                summary,
                covers_until_seq,
                tokens_before,
                tokens_after,
                trigger,
            )
            .await
    }

    pub async fn get_latest_summary(&self, chat_id: &str) -> Result<Option<ChatSummary>> {
        ChatSummaryRepository::new(self.pool.clone())
            .get_latest_for_chat(chat_id)
            .await
    }

    pub async fn list_summaries(&self, chat_id: &str) -> Result<Vec<ChatSummary>> {
        ChatSummaryRepository::new(self.pool.clone())
            .list_for_chat(chat_id)
            .await
    }

    pub async fn adjust_token_totals(
        &self,
        chat_id: &str,
        input_delta: i64,
        output_delta: i64,
    ) -> Result<Chat> {
        let now = now_ms();
        sqlx::query(
            r#"
            UPDATE chats
            SET total_input_tokens = CASE WHEN total_input_tokens + ? < 0 THEN 0 ELSE total_input_tokens + ? END,
                total_output_tokens = CASE WHEN total_output_tokens + ? < 0 THEN 0 ELSE total_output_tokens + ? END,
                updated_at = ?
            WHERE id = ?
            "#,
        )
        .bind(input_delta)
        .bind(input_delta)
        .bind(output_delta)
        .bind(output_delta)
        .bind(now)
        .bind(chat_id)
        .execute(&self.pool)
        .await?;

        self.get_by_id(chat_id)
            .await?
            .ok_or_else(|| anyhow::anyhow!("Chat not found after token update"))
    }

    pub async fn set_token_totals(
        &self,
        chat_id: &str,
        total_input_tokens: i64,
        total_output_tokens: i64,
    ) -> Result<Chat> {
        let now = now_ms();
        sqlx::query(
            "UPDATE chats SET total_input_tokens = ?, total_output_tokens = ?, updated_at = ? WHERE id = ?",
        )
        .bind(total_input_tokens.max(0))
        .bind(total_output_tokens.max(0))
        .bind(now)
        .bind(chat_id)
        .execute(&self.pool)
        .await?;

        self.get_by_id(chat_id)
            .await?
            .ok_or_else(|| anyhow::anyhow!("Chat not found after token update"))
    }
}

pub struct ChatSummaryRepository {
    pool: SqlitePool,
}

impl ChatSummaryRepository {
    pub fn new(pool: SqlitePool) -> Self {
        Self { pool }
    }

    pub async fn create(
        &self,
        chat_id: &str,
        summary: &str,
        covers_until_seq: i64,
        tokens_before: i64,
        tokens_after: i64,
        trigger: &str,
    ) -> Result<ChatSummary> {
        let id = new_id();
        let now = now_ms();
        sqlx::query(
            r#"
            INSERT INTO chat_summaries (
                id, chat_id, summary, covers_until_seq, tokens_before, tokens_after, trigger, created_at
            ) VALUES (?, ?, ?, ?, ?, ?, ?, ?)
            "#,
        )
        .bind(&id)
        .bind(chat_id)
        .bind(summary)
        .bind(covers_until_seq)
        .bind(tokens_before)
        .bind(tokens_after)
        .bind(trigger)
        .bind(now)
        .execute(&self.pool)
        .await?;

        let stored = sqlx::query_as::<_, ChatSummary>("SELECT * FROM chat_summaries WHERE id = ?")
            .bind(&id)
            .fetch_one(&self.pool)
            .await?;
        Ok(stored)
    }

    pub async fn get_latest_for_chat(&self, chat_id: &str) -> Result<Option<ChatSummary>> {
        let row = sqlx::query_as::<_, ChatSummary>(
            "SELECT * FROM chat_summaries WHERE chat_id = ? ORDER BY created_at DESC, id DESC LIMIT 1",
        )
        .bind(chat_id)
        .fetch_optional(&self.pool)
        .await?;
        Ok(row)
    }

    pub async fn list_for_chat(&self, chat_id: &str) -> Result<Vec<ChatSummary>> {
        let rows = sqlx::query_as::<_, ChatSummary>(
            "SELECT * FROM chat_summaries WHERE chat_id = ? ORDER BY created_at ASC, id ASC",
        )
        .bind(chat_id)
        .fetch_all(&self.pool)
        .await?;
        Ok(rows)
    }
}
