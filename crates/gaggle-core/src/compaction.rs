//! Context compaction and chat summarization (Issue #16).
//!
//! [`CompactionEngine`] evaluates whether a chat's accumulated/context token count has crossed
//! its `compact_threshold` and, when it has, summarizes older messages through the active
//! provider, stores a `chat_summaries` row, and returns a compacted message list where the
//! summarized messages are replaced by a single `system` message containing the summary.

use crate::{
    ModelCompletionRequest, ModelProviderAdapter, ProviderMessage,
    context_assembler::estimate_tokens,
    context_limit::exceeds_context_limit,
    structured_summary::{
        CompactionTemplates, StructuredSummary, auto_compact_threshold,
        format_message_for_compacting, render_template,
    },
};
use anyhow::{Context, Result};
use gaggle_db::{
    models::MessagePart,
    repositories::chats::{ChatRepository, ChatSummaryRepository},
};
use sqlx::SqlitePool;
use std::sync::Arc;

/// Result of a compaction pass.
#[derive(Debug, Clone)]
pub struct CompactionResult {
    /// Compacted message list with summarized messages replaced by the summary.
    pub messages: Vec<ProviderMessage>,
    /// Generated summary text.
    pub summary: String,
    /// Last message `seq` included in the summary.
    pub covers_until_seq: i64,
    /// Estimated tokens before compaction.
    pub tokens_before: i64,
    /// Estimated tokens after compaction.
    pub tokens_after: i64,
    /// Whether compaction actually ran.
    pub compacted: bool,
    /// Trigger that caused compaction (`auto` or `manual`).
    pub trigger: String,
}

/// Evaluates whether a chat's token count has crossed its compaction threshold (default 0.75).
pub fn should_compact(total_tokens: i64, context_window: i64, threshold: f64) -> bool {
    exceeds_context_limit(total_tokens, context_window, threshold)
}

/// Marker appended to compacted conversation summaries to signal continuation.
pub const COMPACTION_CONTINUATION_MARKER: &str = "[Context compaction continued]";
/// Alias for [`COMPACTION_CONTINUATION_MARKER`].
pub const CONTEXT_COMPACTION_CONTINUED: &str = COMPACTION_CONTINUATION_MARKER;
/// Alias for [`COMPACTION_CONTINUATION_MARKER`].
pub const CONTEXT_COMPACTION_CONTINUATION_MARKER: &str = COMPACTION_CONTINUATION_MARKER;
/// Alias for [`COMPACTION_CONTINUATION_MARKER`].
pub const CONTINUATION_MARKER: &str = COMPACTION_CONTINUATION_MARKER;

/// Engine that performs context compaction using the active provider.
pub struct CompactionEngine {
    pool: SqlitePool,
    provider: Option<Arc<dyn ModelProviderAdapter>>,
}

impl CompactionEngine {
    pub fn new(pool: SqlitePool) -> Self {
        Self {
            pool,
            provider: None,
        }
    }

    pub fn with_provider(pool: SqlitePool, provider: Arc<dyn ModelProviderAdapter>) -> Self {
        Self {
            pool,
            provider: Some(provider),
        }
    }

    pub fn with_provider_value<P>(pool: SqlitePool, provider: P) -> Self
    where
        P: ModelProviderAdapter + Send + Sync + 'static,
    {
        Self {
            pool,
            provider: Some(Arc::new(provider)),
        }
    }

    pub fn set_provider(&mut self, provider: Arc<dyn ModelProviderAdapter>) {
        self.provider = Some(provider);
    }

    /// Auto/manual compaction using a stored provider, if available.
    pub async fn maybe_compact(
        &self,
        chat_id: &str,
        context_window: i64,
    ) -> Result<Option<CompactionResult>> {
        let Some(provider) = self.provider.clone() else {
            return Ok(None);
        };
        // Threshold gate using the context-limit helper (ratio + absolute cap).
        let chat_repo = ChatRepository::new(self.pool.clone());
        if let Ok(Some(chat)) = chat_repo.get_by_id(chat_id).await {
            let rows = chat_repo
                .list_messages_with_parts(chat_id, 500)
                .await
                .unwrap_or_default();
            let mut total_tokens: i64 = 0;
            for (message, parts) in &rows {
                let content = render_parts(parts);
                let content = content.trim();
                if content.is_empty() || message.role == "system" {
                    continue;
                }
                total_tokens += estimate_tokens(&message.role) + estimate_tokens(content);
            }
            let threshold =
                auto_compact_threshold(context_window.max(0) as usize, chat.compact_threshold);
            if !exceeds_context_limit(total_tokens, context_window, threshold) {
                return Ok(None);
            }
        }
        Self::compact_chat(
            self.pool.clone(),
            chat_id,
            context_window,
            provider.as_ref(),
            "auto",
            false,
        )
        .await
    }

    /// Compaction with an explicitly supplied provider and trigger.
    pub async fn maybe_compact_with_provider(
        &self,
        chat_id: &str,
        context_window: i64,
        provider: &dyn ModelProviderAdapter,
        trigger: &str,
    ) -> Result<Option<CompactionResult>> {
        Self::compact_chat(
            self.pool.clone(),
            chat_id,
            context_window,
            provider,
            trigger,
            trigger == "manual",
        )
        .await
    }

    /// Force compaction (manual) with an explicit provider.
    pub async fn compact(
        &self,
        chat_id: &str,
        context_window: i64,
        provider: &dyn ModelProviderAdapter,
        trigger: &str,
    ) -> Result<Option<CompactionResult>> {
        Self::compact_chat(
            self.pool.clone(),
            chat_id,
            context_window,
            provider,
            trigger,
            true,
        )
        .await
    }

    /// Shared computation for compaction. Returns the generated summary and
    /// accounting values. Persists to the database and adjusts token totals
    /// unless `dry_run` is true.
    async fn compute_summary(
        pool: SqlitePool,
        chat_id: &str,
        context_window: i64,
        provider: &dyn ModelProviderAdapter,
        trigger: &str,
        force: bool,
        dry_run: bool,
    ) -> Result<Option<(String, i64, i64, i64, Vec<(String, Option<String>)>)>> {
        let chat_repo = ChatRepository::new(pool.clone());
        let chat = chat_repo
            .get_by_id(chat_id)
            .await?
            .with_context(|| format!("Chat '{chat_id}' not found"))?;

        let rows = chat_repo.list_messages_with_parts(chat_id, 500).await?;

        // Separate system prompts (preserved verbatim) from summarizable history.
        let mut system_prompts: Vec<String> = Vec::new();
        let mut summarizable: Vec<(i64, String, String)> = Vec::new();
        let mut total_tokens: i64 = 0;
        for (message, parts) in &rows {
            let content = render_parts(parts);
            let content = content.trim().to_string();
            if content.is_empty() {
                continue;
            }
            let toks = estimate_tokens(&message.role) + estimate_tokens(&content);
            if message.role == "system" {
                system_prompts.push(content);
                continue;
            }
            total_tokens += toks;
            summarizable.push((message.seq, message.role.clone(), content));
        }

        if summarizable.is_empty() {
            return Ok(None);
        }

        let threshold =
            auto_compact_threshold(context_window.max(0) as usize, chat.compact_threshold);
        if !force && !should_compact(total_tokens, context_window, threshold) {
            return Ok(None);
        }

        let covers_until_seq = summarizable.iter().map(|(seq, _, _)| *seq).max().unwrap_or(0);

        let tokens_before = total_tokens;

        // Preserve unresolved approvals by loading pending approvals for this chat.
        let pending_approvals: Vec<(String, Option<String>)> = sqlx::query_as(
            r#"
            SELECT a.id, a.command
            FROM approvals a
            JOIN tool_calls tc ON tc.id = a.tool_call_id
            JOIN runs r ON r.id = tc.run_id
            WHERE r.chat_id = ? AND a.status = 'pending'
            "#,
        )
        .bind(chat_id)
        .fetch_all(&pool)
        .await
        .unwrap_or_default();

        // Build the summarization prompt using the ported compaction template.
        // Tool request/response pairs are grouped so they are summarized as a unit.
        let mut message_chunks: Vec<String> = Vec::new();
        let mut i = 0;
        while i < summarizable.len() {
            if i + 1 < summarizable.len()
                && summarizable[i].1 == "assistant"
                && summarizable[i + 1].1 == "tool"
            {
                message_chunks.push(format!(
                    "[tool pair seq {}-{}]\nassistant: {}\ntool: {}",
                    summarizable[i].0, summarizable[i + 1].0, summarizable[i].2, summarizable[i + 1].2
                ));
                i += 2;
            } else {
                let (seq, role, content) = &summarizable[i];
                message_chunks.push(format_message_for_compacting(*seq, role, content));
                i += 1;
            }
        }
        let messages_text = message_chunks.join("\n");
        let templates = CompactionTemplates::default();
        let mut prompt = render_template(
            &templates.compaction,
            &serde_json::json!({ "messages": messages_text }),
        )
        .unwrap_or_else(|_| {
            // Fallback to the previous hard-coded prompt if rendering fails.
            let mut fallback = String::from(
                "Summarize the following conversation history into a concise, faithful summary. \
                 Preserve all critical constraints, unresolved approvals, deny rules, security-relevant \
                 context, and important decisions. Do not invent facts.\n\n",
            );
            for (seq, _, content) in &summarizable {
                fallback.push_str(&format!("[seq {}] {}\n", seq, content));
            }
            fallback
        });

        // Preserve system prompts and unresolved approvals explicitly.
        if !system_prompts.is_empty() {
            prompt.push_str("\n\nSystem prompts that must be preserved:\n");
            for s in &system_prompts {
                prompt.push_str(s);
                prompt.push('\n');
            }
        }
        if !pending_approvals.is_empty() {
            prompt.push_str("\n\nUnresolved approvals that must be preserved:\n");
            for (id, command) in &pending_approvals {
                prompt.push_str(&format!(
                    "- approval {} (command: {})\n",
                    id,
                    command.clone().unwrap_or_default()
                ));
            }
        }

        let req = ModelCompletionRequest {
            model_key: "compaction".to_string(),
            messages: vec![ProviderMessage {
                role: "user".to_string(),
                content: prompt,
                ..Default::default()
            }],
            tools: Vec::new(),
            max_output_tokens: Some(512),
        };

        let completion = provider
            .complete(&req)
            .await
            .with_context(|| "Provider failed during compaction summarization")?;

        let mut summary = completion.text.trim().to_string();
        // If the model followed the structured output contract, render the
        // structured summary into markdown context. Otherwise keep the raw
        // response as a lossless fallback.
        if let Some(structured) = StructuredSummary::parse(&summary)
            && let Ok(rendered) = structured.render_with(&templates.summary)
            && !rendered.trim().is_empty()
        {
            summary = rendered;
        }
        if summary.is_empty() {
            summary = "Summary unavailable.".to_string();
        }

        // Append unresolved approvals explicitly so they are never silently dropped.
        if !pending_approvals.is_empty() {
            summary.push_str("\n\nUnresolved approvals preserved:");
            for (id, command) in &pending_approvals {
                summary.push_str(&format!(
                    "\n- {} ({})",
                    id,
                    command.clone().unwrap_or_default()
                ));
            }
        }

        let tokens_after = estimate_tokens(&summary);

        // Persist unless dry_run.
        if !dry_run {
            let summary_repo = ChatSummaryRepository::new(pool.clone());
            let _stored = summary_repo
                .create(
                    chat_id,
                    &summary,
                    covers_until_seq,
                    tokens_before,
                    tokens_after,
                    trigger,
                )
                .await?;

            // Update token counts.
            let reduction = tokens_before.saturating_sub(tokens_after);
            let _ = chat_repo
                .adjust_token_totals(chat_id, -reduction, completion.usage.output_tokens)
                .await;
            let _ = chat_repo
                .adjust_token_totals(chat_id, completion.usage.input_tokens, 0)
                .await;
        }

        Ok(Some((summary, covers_until_seq, tokens_before, tokens_after, pending_approvals)))
    }

    /// Core implementation shared by all entry points.
    pub async fn compact_chat(
        pool: SqlitePool,
        chat_id: &str,
        context_window: i64,
        provider: &dyn ModelProviderAdapter,
        trigger: &str,
        force: bool,
    ) -> Result<Option<CompactionResult>> {
        let computed = Self::compute_summary(
            pool,
            chat_id,
            context_window,
            provider,
            trigger,
            force,
            false,
        )
        .await?;
        let Some((summary, covers_until_seq, tokens_before, tokens_after, _pending)) = computed else {
            return Ok(None);
        };
        let compacted_messages = vec![ProviderMessage {
            role: "system".to_string(),
            content: format!(
                "Conversation summary:\n{}\n\n{}",
                summary, COMPACTION_CONTINUATION_MARKER
            ),
            ..Default::default()
        }];
        Ok(Some(CompactionResult {
            messages: compacted_messages,
            summary,
            covers_until_seq,
            tokens_before,
            tokens_after,
            compacted: true,
            trigger: trigger.to_string(),
        }))
    }

    /// Dry-run variant that computes a summary without persisting it.
    pub async fn compact_chat_preview(
        pool: SqlitePool,
        chat_id: &str,
        context_window: i64,
        provider: &dyn ModelProviderAdapter,
        trigger: &str,
        force: bool,
    ) -> Result<Option<CompactionResult>> {
        let computed = Self::compute_summary(
            pool,
            chat_id,
            context_window,
            provider,
            trigger,
            force,
            true,
        )
        .await?;
        let Some((summary, covers_until_seq, tokens_before, tokens_after, _pending)) = computed else {
            return Ok(None);
        };
        let compacted_messages = vec![ProviderMessage {
            role: "system".to_string(),
            content: format!(
                "Conversation summary:\n{}\n\n{}",
                summary, COMPACTION_CONTINUATION_MARKER
            ),
            ..Default::default()
        }];
        Ok(Some(CompactionResult {
            messages: compacted_messages,
            summary,
            covers_until_seq,
            tokens_before,
            tokens_after,
            compacted: true,
            trigger: trigger.to_string(),
        }))
    }

    /// Convenience alias for `compact_chat` with auto trigger and threshold check.
    pub async fn maybe_compact_chat(
        pool: SqlitePool,
        chat_id: &str,
        context_window: i64,
        provider: &dyn ModelProviderAdapter,
        trigger: &str,
    ) -> Result<Option<CompactionResult>> {
        let force = trigger == "manual";
        Self::compact_chat(pool, chat_id, context_window, provider, trigger, force).await
    }
}

fn render_parts(parts: &[MessagePart]) -> String {
    let mut chunks = Vec::new();
    for part in parts {
        if part.kind != "text" && part.kind != "tool_result" {
            continue;
        }
        if let Some(content) = part.content.as_deref()
            && !content.trim().is_empty()
        {
            let content = content.trim();
            if part.kind == "tool_result" {
                chunks.push(format!(
                    "<<<TOOL_RESULT>>>\n{content}\n<<<END_TOOL_RESULT>>>"
                ));
            } else {
                chunks.push(content.to_string());
            }
        }
    }
    chunks.join("\n")
}
