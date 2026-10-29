//! Integration tests for Issue #22: chat bundle export/import and memory export.

use anyhow::Result;
use gaggle_db::{
    Chat, connect_in_memory,
    repositories::{
        chats::{
            CHAT_BUNDLE_FORMAT, CHAT_BUNDLE_VERSION, ChatBundle, ChatRepository,
            ImportedChatArtifacts, ImportedMessageMetadata, ImportedUsageRecord,
        },
        memories::MemoryRepository,
        projects::ProjectRepository,
    },
};

struct Fixture {
    chats: ChatRepository,
    project_a: String,
    project_b: String,
    chat: Chat,
}

async fn fixture() -> Result<Fixture> {
    let pool = connect_in_memory().await?;
    let projects = ProjectRepository::new(pool.clone());
    let chats = ChatRepository::new(pool);

    let project_a = projects
        .create("source-project", "/tmp/gaggle-issue22-source", "git")
        .await?
        .id;
    let project_b = projects
        .create("target-project", "/tmp/gaggle-issue22-target", "git")
        .await?
        .id;

    let chat = chats
        .create(&project_a, "Portable chat", "user", "local", "ask")
        .await?;
    chats
        .append_message(&chat.id, None, "user", "Hello export")
        .await?;
    chats
        .append_message(&chat.id, None, "assistant", "Hello import")
        .await?;

    Ok(Fixture {
        chats,
        project_a,
        project_b,
        chat,
    })
}

async fn export_bundle(fx: &Fixture, chat_id: &str) -> Result<ChatBundle> {
    let export = fx.chats.export_chat(chat_id).await?.expect("chat export");
    Ok(ChatBundle::from_export(export))
}

#[tokio::test]
async fn export_import_round_trip_creates_new_chat() -> Result<()> {
    let fx = fixture().await?;
    let bundle = export_bundle(&fx, &fx.chat.id).await?;

    assert_eq!(bundle.format, CHAT_BUNDLE_FORMAT);
    assert_eq!(bundle.version, CHAT_BUNDLE_VERSION);
    assert_eq!(bundle.messages.len(), 2);
    assert_eq!(bundle.message_parts.len(), 2);

    let imported = fx.chats.import_chat(&fx.project_b, &bundle).await?;
    assert_eq!(imported.chat.project_id, fx.project_b);
    assert_ne!(imported.chat.id, fx.chat.id);
    assert_eq!(imported.message_count, 2);
    assert_eq!(imported.part_count, 2);

    let round_trip = fx
        .chats
        .export_chat(&imported.chat.id)
        .await?
        .expect("imported chat");
    assert_eq!(round_trip.messages.len(), 2);
    assert_eq!(round_trip.message_parts.len(), 2);
    assert_eq!(round_trip.messages[0].seq, 1);
    assert_eq!(round_trip.messages[1].seq, 2);
    assert_eq!(round_trip.messages[0].role, "user");
    assert_eq!(round_trip.messages[1].role, "assistant");
    assert_eq!(
        round_trip.message_parts[0].content.as_deref(),
        Some("Hello export")
    );
    assert_eq!(
        round_trip.message_parts[1].content.as_deref(),
        Some("Hello import")
    );

    // Importing twice must never overwrite the earlier import.
    let second = fx.chats.import_chat(&fx.project_a, &bundle).await?;
    assert_ne!(second.chat.id, imported.chat.id);
    assert_ne!(second.chat.id, fx.chat.id);

    // The source chat is untouched.
    let source = fx
        .chats
        .export_chat(&fx.chat.id)
        .await?
        .expect("source chat");
    assert_eq!(source.chat.project_id, fx.project_a);
    assert_eq!(source.messages.len(), 2);

    Ok(())
}

#[tokio::test]
async fn goose_usage_and_metadata_survive_chat_bundle_round_trip() -> Result<()> {
    let fx = fixture().await?;
    let mut bundle = export_bundle(&fx, &fx.chat.id).await?;
    bundle.imported_artifacts = Some(ImportedChatArtifacts {
        source_session_id: "goose-session-1".to_string(),
        session_metadata_json: r#"{"extension_data":{"enabled":true}}"#.to_string(),
        message_metadata: vec![ImportedMessageMetadata {
            source_message_id: "goose-message-1".to_string(),
            message_seq: 1,
            payload_json: r#"{"audience":"user"}"#.to_string(),
        }],
        usage_records: vec![ImportedUsageRecord {
            source_record_id: "1".to_string(),
            model_key: Some("gemini-test".to_string()),
            input_tokens: 11,
            output_tokens: 7,
            total_tokens: 18,
            cache_read_tokens: 2,
            cache_write_tokens: 0,
            cost_usd: Some(0.01),
            cost_source: "provider_reported".to_string(),
            is_compaction: false,
            created_at: 1_730_000_000_000,
        }],
        total_input_tokens: 11,
        total_output_tokens: 7,
    });

    let imported = fx.chats.import_chat(&fx.project_b, &bundle).await?;
    assert_eq!(imported.chat.total_input_tokens, 11);
    assert_eq!(imported.chat.total_output_tokens, 7);

    let round_trip = fx
        .chats
        .export_chat(&imported.chat.id)
        .await?
        .expect("imported chat");
    let artifacts = round_trip.imported_artifacts.expect("import artifacts");
    assert_eq!(artifacts.source_session_id, "goose-session-1");
    assert_eq!(artifacts.message_metadata.len(), 1);
    assert_eq!(artifacts.usage_records.len(), 1);
    assert_eq!(
        artifacts.usage_records[0].model_key.as_deref(),
        Some("gemini-test")
    );
    assert_eq!(artifacts.usage_records[0].input_tokens, 11);

    Ok(())
}

#[tokio::test]
async fn import_drops_references_to_rows_not_in_bundle() -> Result<()> {
    let fx = fixture().await?;
    let mut bundle = export_bundle(&fx, &fx.chat.id).await?;

    bundle.chat.model_id = Some(uuid::Uuid::now_v7().to_string());
    bundle.chat.recipe_id = Some(uuid::Uuid::now_v7().to_string());
    bundle.chat.recipe_values = Some("{\"not\":\"kept\"}".to_string());
    bundle.chat.worktree_id = Some(uuid::Uuid::now_v7().to_string());
    bundle.chat.parent_chat_id = Some(uuid::Uuid::now_v7().to_string());
    bundle.chat.schedule_id = Some(uuid::Uuid::now_v7().to_string());
    bundle.chat.archived_at = Some(1);
    bundle.messages[1].parent_id = Some(bundle.messages[0].id.clone());
    bundle.message_parts[1].attachment_id = Some(uuid::Uuid::now_v7().to_string());
    bundle.message_parts[1].tool_call_id = Some(uuid::Uuid::now_v7().to_string());

    let imported = fx.chats.import_chat(&fx.project_b, &bundle).await?;
    assert_eq!(imported.chat.model_id, None);
    assert_eq!(imported.chat.recipe_id, None);
    assert_eq!(imported.chat.recipe_values, None);
    assert_eq!(imported.chat.worktree_id, None);
    assert_eq!(imported.chat.parent_chat_id, None);
    assert_eq!(imported.chat.schedule_id, None);
    assert_eq!(imported.chat.archived_at, None);

    let round_trip = fx
        .chats
        .export_chat(&imported.chat.id)
        .await?
        .expect("imported chat");
    assert_eq!(round_trip.message_parts[1].attachment_id, None);
    assert_eq!(round_trip.message_parts[1].tool_call_id, None);
    assert_eq!(
        round_trip.messages[1].parent_id.as_deref(),
        Some(round_trip.messages[0].id.as_str())
    );

    Ok(())
}

#[tokio::test]
async fn import_rejects_invalid_bundles() -> Result<()> {
    let fx = fixture().await?;
    let valid = export_bundle(&fx, &fx.chat.id).await?;

    let mut bad_format = valid.clone();
    bad_format.format = "other.export".to_string();
    assert!(bad_format.validate().is_err());

    let mut bad_version = valid.clone();
    bad_version.version = CHAT_BUNDLE_VERSION + 1;
    assert!(bad_version.validate().is_err());

    let mut empty_title = valid.clone();
    empty_title.chat.title = "   ".to_string();
    assert!(empty_title.validate().is_err());

    let mut wrong_chat = valid.clone();
    wrong_chat.messages[0].chat_id = uuid::Uuid::now_v7().to_string();
    assert!(wrong_chat.validate().is_err());

    let mut bad_role = valid.clone();
    bad_role.messages[0].role = "superuser".to_string();
    assert!(bad_role.validate().is_err());

    let mut duplicate_seq = valid.clone();
    duplicate_seq.messages[1].seq = duplicate_seq.messages[0].seq;
    assert!(duplicate_seq.validate().is_err());

    let mut unknown_message = valid.clone();
    unknown_message.message_parts[0].message_id = uuid::Uuid::now_v7().to_string();
    assert!(unknown_message.validate().is_err());

    let mut duplicate_part_slot = valid.clone();
    duplicate_part_slot.message_parts[1].message_id =
        duplicate_part_slot.message_parts[0].message_id.clone();
    duplicate_part_slot.message_parts[1].idx = duplicate_part_slot.message_parts[0].idx;
    assert!(duplicate_part_slot.validate().is_err());

    let mut bad_id = valid.clone();
    bad_id.messages[0].id = "not-a-uuid".to_string();
    assert!(bad_id.validate().is_err());

    let mut bad_kind = valid.clone();
    bad_kind.message_parts[0].kind = "image".to_string();
    assert!(bad_kind.validate().is_err());

    let mut bad_threshold = valid.clone();
    bad_threshold.chat.compact_threshold = f64::NAN;
    assert!(bad_threshold.validate().is_err());

    // Every rejection must happen before a row is written.
    assert!(fx.chats.import_chat(&fx.project_b, &bad_id).await.is_err());
    let chats = fx.chats.list_by_project(&fx.project_b).await?;
    assert!(chats.is_empty());

    Ok(())
}

#[tokio::test]
async fn import_rejects_unknown_or_archived_project() -> Result<()> {
    let fx = fixture().await?;
    let bundle = export_bundle(&fx, &fx.chat.id).await?;

    assert!(
        fx.chats
            .import_chat(&uuid::Uuid::now_v7().to_string(), &bundle)
            .await
            .is_err()
    );

    let pool = connect_in_memory().await?;
    let projects = ProjectRepository::new(pool.clone());
    let archived = projects
        .create("archived", "/tmp/gaggle-issue22-archived", "git")
        .await?;
    sqlx::query("UPDATE projects SET archived_at = 1 WHERE id = ?")
        .bind(&archived.id)
        .execute(&pool)
        .await?;
    let chats = ChatRepository::new(pool);
    assert!(chats.import_chat(&archived.id, &bundle).await.is_err());

    Ok(())
}

#[tokio::test]
async fn messages_without_parts_round_trip() -> Result<()> {
    let fx = fixture().await?;
    let mut bundle = export_bundle(&fx, &fx.chat.id).await?;
    bundle.message_parts.clear();

    let imported = fx.chats.import_chat(&fx.project_b, &bundle).await?;
    assert_eq!(imported.part_count, 0);
    let round_trip = fx
        .chats
        .export_chat(&imported.chat.id)
        .await?
        .expect("imported chat");
    assert_eq!(round_trip.messages.len(), 2);
    assert!(round_trip.message_parts.is_empty());

    Ok(())
}

#[tokio::test]
async fn memory_markdown_files_are_safe_unique_and_redacted() -> Result<()> {
    let pool = connect_in_memory().await?;
    let projects = ProjectRepository::new(pool.clone());
    let memories = MemoryRepository::new(pool);
    let project = projects
        .create("memory-project", "/tmp/gaggle-issue22-memory", "git")
        .await?;

    memories
        .create_with_options(
            Some(&project.id),
            None,
            None,
            "fact",
            "../../Etc/Passwd: weird title",
            "api_key=sk-live-super-secret",
            &["sqlite".to_string()],
            true,
        )
        .await?;
    memories
        .create_with_options(
            None,
            None,
            None,
            "preference",
            "../../Etc/Passwd: weird title",
            "global memory",
            &[],
            false,
        )
        .await?;
    let deleted = memories
        .create(Some(&project.id), "todo", "Delete me", "not exported")
        .await?;
    memories.soft_delete(&deleted.id).await?;

    let files = memories.export_markdown_files(&project.id).await?;
    assert_eq!(files.len(), 2);

    let mut names = std::collections::HashSet::new();
    for file in &files {
        assert!(
            names.insert(file.file_name.clone()),
            "file names must be unique"
        );
        assert!(file.file_name.ends_with(".md"));
        assert!(
            file.file_name.chars().all(|ch| ch.is_ascii_lowercase()
                || ch.is_ascii_digit()
                || matches!(ch, '-' | '_' | '.')),
            "unsafe export file name: {}",
            file.file_name
        );
        assert!(!file.file_name.contains(".."));
        assert!(!file.markdown.contains("sk-live-super-secret"));
        assert!(file.markdown.starts_with("# "));
        assert!(!file.markdown.contains("not exported"));
    }

    Ok(())
}

#[tokio::test]
async fn chat_bundle_metadata_is_optional_for_legacy_payloads() -> Result<()> {
    let fx = fixture().await?;
    let bundle = export_bundle(&fx, &fx.chat.id).await?;
    let export = fx.chats.export_chat(&fx.chat.id).await?.expect("chat");

    let legacy = serde_json::json!({
        "chat": export.chat,
        "messages": export.messages,
        "message_parts": export.message_parts,
    });
    let parsed: ChatBundle = serde_json::from_value(legacy)?;
    assert_eq!(parsed.format, CHAT_BUNDLE_FORMAT);
    assert_eq!(parsed.version, CHAT_BUNDLE_VERSION);
    parsed.validate()?;

    let imported = fx.chats.import_chat(&fx.project_b, &parsed).await?;
    assert_eq!(imported.chat.title, bundle.chat.title);
    assert_eq!(imported.message_count, 2);

    Ok(())
}
