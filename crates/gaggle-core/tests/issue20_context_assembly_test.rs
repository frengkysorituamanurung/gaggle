//! Issue #20: bounded context assembly, project instructions, repository instructions,
//! memories, history, tools, and secret redaction.

use anyhow::Result;
use gaggle_core::context_assembler::{
    ContextAssembler, ContextAssemblyOptions, compute_budget, discover_agents_md, estimate_tokens,
    redact_context_secrets,
};
use gaggle_db::{
    connect_in_memory,
    models::Chat,
    repositories::{
        chats::ChatRepository, extensions::ExtensionRepository, memories::MemoryRepository,
        projects::ProjectRepository, providers::ProviderRepository,
    },
};
use sqlx::SqlitePool;
use std::path::{Path, PathBuf};

struct Workspace {
    root: PathBuf,
}

impl Workspace {
    fn new(label: &str) -> Result<Self> {
        let root =
            std::env::temp_dir().join(format!("gaggle_issue20_{label}_{}", uuid::Uuid::now_v7()));
        std::fs::create_dir_all(&root)?;
        Ok(Self { root })
    }

    fn write(&self, relative: &str, content: &str) -> Result<()> {
        let path = self.root.join(relative);
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)?;
        }
        std::fs::write(path, content)?;
        Ok(())
    }

    fn path(&self) -> &Path {
        &self.root
    }
}

impl Drop for Workspace {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.root);
    }
}

async fn create_project_and_chat(
    pool: &SqlitePool,
    workspace: &Workspace,
    context_window: i64,
) -> Result<Chat> {
    let project_repo = ProjectRepository::new(pool.clone());
    let chat_repo = ChatRepository::new(pool.clone());
    let provider_repo = ProviderRepository::new(pool.clone());

    let project = project_repo
        .create(
            "Issue 20 Project",
            workspace.path().to_str().expect("temp path is UTF-8"),
            "none",
        )
        .await?;
    let chat = chat_repo
        .create(&project.id, "Issue 20 Chat", "user", "local", "ask")
        .await?;

    let provider = provider_repo
        .create_provider("openai_compat", "Issue 20 Provider", None, "none", None)
        .await?;
    let model = provider_repo
        .create_model(
            &provider.id,
            "issue20-model",
            "Issue 20 Model",
            context_window,
            true,
            false,
            false,
            None,
            None,
        )
        .await?;

    sqlx::query("UPDATE chats SET model_id = ? WHERE id = ?")
        .bind(&model.id)
        .bind(&chat.id)
        .execute(pool)
        .await?;

    chat_repo
        .get_by_id(&chat.id)
        .await?
        .ok_or_else(|| anyhow::anyhow!("chat missing after model assignment"))
}

async fn save_project_instructions(pool: &SqlitePool, project_id: &str, text: &str) -> Result<()> {
    let project_repo = ProjectRepository::new(pool.clone());
    let settings = gaggle_db::models::ProjectSettings {
        project_id: project_id.to_string(),
        instructions: Some(text.to_string()),
        ignore_globs: None,
        env: None,
        shell: None,
        setup_command: None,
    };
    project_repo.upsert_settings(&settings).await?;
    Ok(())
}

#[test]
fn test_estimate_tokens_and_budget() {
    assert_eq!(estimate_tokens(""), 0);
    assert_eq!(estimate_tokens("a"), 1);
    assert_eq!(estimate_tokens("abcd"), 1);
    assert_eq!(estimate_tokens("abcde"), 2);
    assert_eq!(estimate_tokens("🚀🚀🚀🚀"), 1);

    assert_eq!(compute_budget(8192, 0.75), 6144);
    assert_eq!(compute_budget(512, 0.75), 512);
    assert_eq!(compute_budget(8192, 0.0), 6144);
    assert_eq!(compute_budget(8192, 1.5), 6144);
    assert_eq!(compute_budget(1024, 1.0), 1024);
}

#[test]
fn test_redact_context_secrets_patterns() {
    let input = "\
api_key = abc123value
Authorization: Bearer topsecretbearer
use sk-live-secret-abc123456789 today
github token ghp_abcdefghijklmnopqrst
fine-grained github_pat_11ABCDEFG0abcdefghijkl
slack xoxb-1234567890-abcdefghij
aws AKIAABCDEFGHIJKLMNOP
-----BEGIN OPENSSH PRIVATE KEY-----
MIIEvQIBADANBgkqhkiG9w0BAQEFAASC
-----END OPENSSH PRIVATE KEY-----
harmless text";

    let output = redact_context_secrets(input);

    for raw in [
        "abc123value",
        "topsecretbearer",
        "sk-live-secret-abc123456789",
        "ghp_abcdefghijklmnopqrst",
        "github_pat_11ABCDEFG0abcdefghijkl",
        "xoxb-1234567890-abcdefghij",
        "AKIAABCDEFGHIJKLMNOP",
        "MIIEvQIBADANBgkqhkiG9w0BAQEFAASC",
    ] {
        assert!(!output.contains(raw), "raw secret leaked: {raw}");
    }
    assert!(output.contains("[REDACTED]"));
    assert!(output.contains("[REDACTED PRIVATE KEY]"));
    assert!(output.contains("harmless text"));
}

#[test]
fn test_discover_agents_md_skips_hidden_and_dependency_directories() -> Result<()> {
    let workspace = Workspace::new("discover")?;
    workspace.write("AGENTS.md", "Root repository rule.")?;
    workspace.write("src/AGENTS.md", "Source repository rule.")?;
    workspace.write(".git/AGENTS.md", "Git internal rule must be skipped.")?;
    workspace.write("node_modules/AGENTS.md", "Dependency rule must be skipped.")?;
    workspace.write("target/AGENTS.md", "Build rule must be skipped.")?;
    workspace.write("src/nested/deep/AGENTS.md", "Nested rule.")?;
    // Kept in its own directory: macOS default volumes are case-insensitive, so writing
    // `src/agents.md` would shadow `src/AGENTS.md` on that platform.
    workspace.write("src/lowercase/agents.md", "Wrong case must be skipped.")?;

    let options = ContextAssemblyOptions::default();
    let documents = discover_agents_md(workspace.path(), &options)?;

    let paths: Vec<&str> = documents
        .iter()
        .map(|document| document.relative_path.as_str())
        .collect();
    assert_eq!(
        paths,
        vec!["AGENTS.md", "src/AGENTS.md", "src/nested/deep/AGENTS.md"]
    );
    assert!(documents[0].content.contains("Root repository rule."));
    assert!(documents[1].content.contains("Source repository rule."));
    assert!(
        documents
            .iter()
            .all(|document| !document.content.contains("must be skipped"))
    );

    // Per-file/total byte caps are enforced and marked.
    let capped = ContextAssemblyOptions {
        max_agents_md_files: 2,
        max_agents_md_bytes: 16,
        ..ContextAssemblyOptions::default()
    };
    let capped_documents = discover_agents_md(workspace.path(), &capped)?;
    assert_eq!(capped_documents.len(), 1);
    assert!(capped_documents[0].content.ends_with("[truncated]"));

    Ok(())
}

#[tokio::test]
async fn test_list_messages_with_parts_orders_parts_and_respects_limit() -> Result<()> {
    let pool = connect_in_memory().await?;
    let workspace = Workspace::new("history")?;
    let chat = create_project_and_chat(&pool, &workspace, 1024).await?;
    let chat_repo = ChatRepository::new(pool.clone());

    let first = chat_repo
        .append_message(&chat.id, None, "user", "first")
        .await?;
    let second = chat_repo
        .append_message(&chat.id, None, "assistant", "second")
        .await?;
    let third = chat_repo
        .append_message(&chat.id, None, "user", "third")
        .await?;

    sqlx::query(
        "INSERT INTO message_parts (id, message_id, idx, kind, content) VALUES (?, ?, 1, 'tool_result', ?)",
    )
    .bind(uuid::Uuid::now_v7().to_string())
    .bind(&second.id)
    .bind("tool output")
    .execute(&pool)
    .await?;

    // A message without parts must be preserved with an empty part list.
    let empty_id = uuid::Uuid::now_v7().to_string();
    let now = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)?
        .as_millis() as i64;
    sqlx::query(
        "INSERT INTO messages (id, chat_id, seq, role, created_at) VALUES (?, ?, 4, 'system', ?)",
    )
    .bind(&empty_id)
    .bind(&chat.id)
    .bind(now)
    .execute(&pool)
    .await?;

    let all = chat_repo.list_messages_with_parts(&chat.id, 10).await?;
    assert_eq!(all.len(), 4);
    let seqs: Vec<i64> = all.iter().map(|(message, _)| message.seq).collect();
    assert!(seqs.windows(2).all(|window| window[0] < window[1]));
    assert_eq!(all[0].0.id, first.id);
    assert_eq!(all[1].0.id, second.id);
    let second_part_kinds: Vec<&str> = all[1].1.iter().map(|part| part.kind.as_str()).collect();
    assert_eq!(second_part_kinds, vec!["text", "tool_result"]);
    assert!(all[3].1.is_empty());

    // Only the newest `limit` messages are returned, still in ascending order.
    let limited = chat_repo.list_messages_with_parts(&chat.id, 2).await?;
    assert_eq!(limited.len(), 2);
    assert_eq!(limited[0].0.id, third.id);
    assert_eq!(limited[1].0.id, empty_id);
    assert!(limited[1].1.is_empty());

    // Limits are clamped to 1..=500.
    let clamped = chat_repo.list_messages_with_parts(&chat.id, 0).await?;
    assert_eq!(clamped.len(), 1);
    assert_eq!(clamped[0].0.id, empty_id);

    Ok(())
}

#[tokio::test]
async fn test_assemble_includes_instructions_agents_memories_history_and_tools() -> Result<()> {
    let pool = connect_in_memory().await?;
    let workspace = Workspace::new("assemble")?;
    workspace.write(
        "AGENTS.md",
        "Root AGENTS rule: run cargo test before reporting.",
    )?;
    workspace.write(
        "src/AGENTS.md",
        "Source AGENTS rule: prefer small focused functions.",
    )?;

    let chat = create_project_and_chat(&pool, &workspace, 512).await?;
    save_project_instructions(&pool, &chat.project_id, "Always answer in English.").await?;

    let extension_repo = ExtensionRepository::new(pool.clone());
    let fs_extension = extension_repo
        .upsert_builtin("gaggle.fs", "Filesystem tools", "1.0.0")
        .await?;
    extension_repo
        .upsert_tool(
            &fs_extension.id,
            "fs.read",
            "Read a file from the workspace",
            r#"{"type":"object","properties":{"path":{"type":"string"}}}"#,
            true,
        )
        .await?;
    let web_extension = extension_repo
        .upsert_builtin("gaggle.web", "Web tools", "1.0.0")
        .await?;
    extension_repo
        .upsert_tool(
            &web_extension.id,
            "web.fetch",
            "Fetch a URL",
            r#"{"type":"object"}"#,
            true,
        )
        .await?;
    extension_repo.set_enabled(&web_extension.id, false).await?;

    let chat_repo = ChatRepository::new(pool.clone());
    for (role, text) in [
        ("user", "hello from earlier"),
        ("assistant", "earlier assistant reply"),
        ("user", "another earlier question"),
        ("assistant", "another earlier answer"),
    ] {
        chat_repo.append_message(&chat.id, None, role, text).await?;
    }

    // A historical tool result must be labelled as untrusted tool output.
    let tool_message = chat_repo
        .append_message(&chat.id, None, "tool", "placeholder")
        .await?;
    sqlx::query(
        "UPDATE message_parts SET kind = 'tool_result', content = 'command output' \
         WHERE message_id = ? AND idx = 0",
    )
    .bind(&tool_message.id)
    .execute(&pool)
    .await?;

    let memory_repo = MemoryRepository::new(pool.clone());
    let pinned = memory_repo
        .create(
            Some(&chat.project_id),
            "fact",
            "Release process",
            "The release command is make release.",
        )
        .await?;
    memory_repo
        .update(&pinned.id, None, None, None, Some(true))
        .await?;
    memory_repo
        .create(
            Some(&chat.project_id),
            "decision",
            "Schema changes",
            "Use the migration workflow for schema changes.",
        )
        .await?;

    let assembler = ContextAssembler::new(pool.clone());
    let prompt = "migration";
    let context = assembler.assemble(&chat.id, prompt).await?;

    assert_eq!(context.budget_tokens, 512);
    assert!(context.estimated_tokens <= context.budget_tokens);
    assert_eq!(
        context.agents_md_paths,
        vec!["AGENTS.md".to_string(), "src/AGENTS.md".to_string()]
    );

    // Exactly one system message, and it is first.
    assert_eq!(context.messages[0].role, "system");
    let system = &context.messages[0].content;
    assert!(system.contains("Always answer in English."));
    assert!(system.contains("Root AGENTS rule: run cargo test before reporting."));
    assert!(system.contains("Source AGENTS rule: prefer small focused functions."));
    assert!(system.contains("untrusted data"));

    // Pinned memories are ordered before search results.
    let pinned_index = system
        .find("Release process")
        .expect("pinned memory included");
    let search_index = system
        .find("Schema changes")
        .expect("search memory included");
    assert!(
        pinned_index < search_index,
        "pinned memory must appear before searched memory"
    );

    // Recent history is included in chronological order ahead of the current prompt.
    let history_position = context
        .messages
        .iter()
        .position(|message| message.content.contains("earlier assistant reply"))
        .expect("history included");
    assert!(history_position < context.messages.len() - 1);
    assert!(
        context
            .messages
            .iter()
            .any(|message| message.content.contains("<<<TOOL_RESULT>>>")
                && message.content.contains("command output")),
        "tool results must be wrapped with an untrusted-data boundary"
    );
    assert_eq!(context.messages.last().expect("prompt").role, "user");
    assert_eq!(context.messages.last().expect("prompt").content, prompt);
    let prompt_occurrences = context
        .messages
        .iter()
        .filter(|message| message.role == "user" && message.content == prompt)
        .count();
    assert_eq!(prompt_occurrences, 1, "current prompt must appear once");

    // Enabled extension tools only.
    let tool_names: Vec<&str> = context
        .tools
        .iter()
        .map(|tool| tool.name.as_str())
        .collect();
    assert!(tool_names.contains(&"fs.read"));
    assert!(!tool_names.contains(&"web.fetch"));

    Ok(())
}

#[tokio::test]
async fn test_assemble_enforces_budget_and_drops_oldest_history() -> Result<()> {
    let pool = connect_in_memory().await?;
    let workspace = Workspace::new("budget")?;
    workspace.write("AGENTS.md", "Root rule for the budget test.")?;

    let chat = create_project_and_chat(&pool, &workspace, 512).await?;
    let chat_repo = ChatRepository::new(pool.clone());

    chat_repo
        .append_message(&chat.id, None, "user", "oldest history message")
        .await?;
    for index in 0..24 {
        chat_repo
            .append_message(
                &chat.id,
                None,
                if index % 2 == 0 { "user" } else { "assistant" },
                &format!(
                    "history filler {index}: {}",
                    "lorem ipsum dolor sit amet ".repeat(12)
                ),
            )
            .await?;
    }
    chat_repo
        .append_message(&chat.id, None, "assistant", "newest history message")
        .await?;

    let memory_repo = MemoryRepository::new(pool.clone());
    for index in 0..4 {
        let memory = memory_repo
            .create(
                Some(&chat.project_id),
                "fact",
                &format!("Pinned memory {index}"),
                &format!("Pinned context {index}: {}", "important detail ".repeat(10)),
            )
            .await?;
        memory_repo
            .update(&memory.id, None, None, None, Some(true))
            .await?;
    }

    let assembler = ContextAssembler::new(pool.clone());
    let context = assembler.assemble(&chat.id, "budget question").await?;

    assert!(context.estimated_tokens <= context.budget_tokens);
    assert!(context.truncated, "oversized context must be truncated");
    assert!(context.messages.len() > 1);

    let all_content = context
        .messages
        .iter()
        .map(|message| message.content.as_str())
        .collect::<Vec<_>>()
        .join("\n");
    assert!(all_content.contains("newest history message"));
    assert!(!all_content.contains("oldest history message"));
    assert_eq!(
        context.messages.last().expect("prompt").content,
        "budget question"
    );

    Ok(())
}

#[tokio::test]
async fn test_assemble_redacts_secrets_from_all_sources() -> Result<()> {
    let pool = connect_in_memory().await?;
    let workspace = Workspace::new("secrets")?;
    workspace.write(
        "AGENTS.md",
        "Repository rule with sk-live-secret-abc123456789.\n\
         Authorization: Bearer topsecretbearertoken\n\
         API_KEY=supersecretvalue123\n\
         -----BEGIN RSA PRIVATE KEY-----\n\
         MIIEvQIBADANBgkqhkiG9w0BAQEFAASC\n\
         -----END RSA PRIVATE KEY-----\n",
    )?;

    let chat = create_project_and_chat(&pool, &workspace, 4096).await?;
    save_project_instructions(
        &pool,
        &chat.project_id,
        "Service password: hunter2secretvalue",
    )
    .await?;

    let chat_repo = ChatRepository::new(pool.clone());
    chat_repo
        .append_message(
            &chat.id,
            None,
            "assistant",
            "history token ghp_abcdefghijklmnopqrst and AKIAABCDEFGHIJKLMNOP",
        )
        .await?;

    let memory_repo = MemoryRepository::new(pool.clone());
    memory_repo
        .create(
            Some(&chat.project_id),
            "fact",
            "Credentials",
            "token: memorytokenvalue",
        )
        .await?;

    let assembler = ContextAssembler::new(pool);
    let prompt = "please use sk-live-secret-abc123456789 for the request";
    let context = assembler.assemble(&chat.id, prompt).await?;

    let all_content = context
        .messages
        .iter()
        .map(|message| message.content.as_str())
        .collect::<Vec<_>>()
        .join("\n");

    for raw in [
        "sk-live-secret-abc123456789",
        "topsecretbearertoken",
        "supersecretvalue123",
        "MIIEvQIBADANBgkqhkiG9w0BAQEFAASC",
        "hunter2secretvalue",
        "memorytokenvalue",
        "ghp_abcdefghijklmnopqrst",
        "AKIAABCDEFGHIJKLMNOP",
    ] {
        assert!(
            !all_content.contains(raw),
            "raw secret leaked into assembled context: {raw}"
        );
    }
    assert!(all_content.contains("[REDACTED PRIVATE KEY]"));

    Ok(())
}
