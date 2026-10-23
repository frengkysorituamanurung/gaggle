//! Integration tests for Issue #17: memory system persistence layer.

use anyhow::Result;
use gaggle_db::{
    ProjectSettings, connect_in_memory,
    repositories::{memories::MemoryRepository, projects::ProjectRepository},
};

struct Fixture {
    memories: MemoryRepository,
    projects: ProjectRepository,
    project_a: String,
    project_b: String,
}

async fn fixture() -> Result<Fixture> {
    let pool = connect_in_memory().await?;
    let projects = ProjectRepository::new(pool.clone());
    let memories = MemoryRepository::new(pool);

    let project_a = projects
        .create("alpha-project", "/tmp/gaggle-issue17-alpha", "git")
        .await?
        .id;
    let project_b = projects
        .create("beta-project", "/tmp/gaggle-issue17-beta", "git")
        .await?
        .id;

    Ok(Fixture {
        memories,
        projects,
        project_a,
        project_b,
    })
}

fn tags(values: &[&str]) -> Vec<String> {
    values.iter().map(|value| (*value).to_string()).collect()
}

#[tokio::test]
async fn create_with_options_persists_memory_and_tags() -> Result<()> {
    let fx = fixture().await?;

    let memory = fx
        .memories
        .create_with_options(
            Some(&fx.project_a),
            None,
            None,
            "decision",
            "  Use WAL mode  ",
            " Enable WAL for durability ",
            &tags(&["sqlite", " sqlite ", "", "storage"]),
            true,
        )
        .await?;

    assert_eq!(memory.title, "Use WAL mode");
    assert_eq!(memory.content, "Enable WAL for durability");
    assert_eq!(memory.kind, "decision");
    assert_eq!(memory.project_id.as_deref(), Some(fx.project_a.as_str()));
    assert!(memory.source_chat_id.is_none());
    assert!(memory.source_run_id.is_none());
    assert_eq!(memory.pinned, 1);
    assert!(memory.deleted_at.is_none());
    assert!(memory.last_used_at.is_none());

    let fetched = fx
        .memories
        .get_by_id(&memory.id)
        .await?
        .expect("active memory");
    assert_eq!(fetched.id, memory.id);
    assert_eq!(fetched.title, memory.title);

    let active = fx
        .memories
        .get_active_by_id(&memory.id)
        .await?
        .expect("active memory");
    assert_eq!(active.id, memory.id);

    // Tags are trimmed, deduplicated, and sorted on read.
    let stored_tags = fx.memories.list_tags(&memory.id).await?;
    assert_eq!(stored_tags, tags(&["sqlite", "storage"]));

    // The legacy `create` entry point keeps working.
    let legacy = fx
        .memories
        .create(Some(&fx.project_a), "fact", "Legacy", "Legacy content")
        .await?;
    assert_eq!(legacy.kind, "fact");
    assert_eq!(legacy.pinned, 0);
    assert_eq!(legacy.project_id.as_deref(), Some(fx.project_a.as_str()));

    Ok(())
}

#[tokio::test]
async fn create_with_options_validates_input_and_allows_global_memories() -> Result<()> {
    let fx = fixture().await?;

    assert!(
        fx.memories
            .create_with_options(
                Some(&fx.project_a),
                None,
                None,
                "nonsense",
                "Title",
                "Content",
                &[],
                false,
            )
            .await
            .is_err()
    );
    assert!(
        fx.memories
            .create_with_options(
                Some(&fx.project_a),
                None,
                None,
                "fact",
                "   ",
                "Content",
                &[],
                false,
            )
            .await
            .is_err()
    );
    assert!(
        fx.memories
            .create_with_options(
                Some(&fx.project_a),
                None,
                None,
                "fact",
                "Title",
                "\n\t ",
                &[],
                false,
            )
            .await
            .is_err()
    );

    let too_many_tags: Vec<String> = (0..17).map(|index| format!("tag-{index}")).collect();
    assert!(
        fx.memories
            .create_with_options(
                Some(&fx.project_a),
                None,
                None,
                "fact",
                "Title",
                "Content",
                &too_many_tags,
                false,
            )
            .await
            .is_err()
    );

    let too_long_tag = vec!["x".repeat(65)];
    assert!(
        fx.memories
            .create_with_options(
                Some(&fx.project_a),
                None,
                None,
                "fact",
                "Title",
                "Content",
                &too_long_tag,
                false,
            )
            .await
            .is_err()
    );

    let global = fx
        .memories
        .create_with_options(
            None,
            None,
            None,
            "preference",
            "Global preference",
            "Applies to every project",
            &[],
            false,
        )
        .await?;
    assert!(global.project_id.is_none());

    Ok(())
}

#[tokio::test]
async fn fts_search_is_safe_and_finds_title_and_content() -> Result<()> {
    let fx = fixture().await?;

    let memory = fx
        .memories
        .create_with_options(
            Some(&fx.project_a),
            None,
            None,
            "fact",
            "SQLite WAL Mode",
            "Durable storage notes for the workspace",
            &tags(&["sqlite"]),
            false,
        )
        .await?;

    let by_title = fx.memories.search_fts("SQLite", 10).await?;
    assert_eq!(by_title.len(), 1);
    assert_eq!(by_title[0].id, memory.id);

    let by_content = fx.memories.search_fts("durable", 10).await?;
    assert_eq!(by_content.len(), 1);
    assert_eq!(by_content[0].id, memory.id);

    // Unbalanced quotes, raw FTS operators, and punctuation-only input must
    // never surface an error from the FTS5 parser.
    assert!(fx.memories.search_fts("\"unbalanced", 10).await.is_ok());
    assert!(
        fx.memories
            .search_fts("durable OR \"unclosed NEAR(", 10)
            .await
            .is_ok()
    );
    assert!(fx.memories.search_fts("-", 10).await?.is_empty());
    assert!(fx.memories.search_fts("***", 10).await?.is_empty());
    assert!(fx.memories.search_fts("   ", 10).await?.is_empty());
    assert!(fx.memories.search_fts("", 10).await?.is_empty());

    // Limit is clamped rather than trusted.
    assert!(!fx.memories.search_fts("SQLite", 0).await?.is_empty());
    assert!(fx.memories.search_fts("SQLite", -5).await?.len() <= 100);

    Ok(())
}

#[tokio::test]
async fn scoped_search_includes_global_and_excludes_other_projects() -> Result<()> {
    let fx = fixture().await?;

    let project_memory = fx
        .memories
        .create_with_options(
            Some(&fx.project_a),
            None,
            None,
            "todo",
            "Release checklist",
            "alpha release steps",
            &[],
            false,
        )
        .await?;
    let other_memory = fx
        .memories
        .create_with_options(
            Some(&fx.project_b),
            None,
            None,
            "todo",
            "Release checklist",
            "alpha release steps",
            &[],
            false,
        )
        .await?;
    let global_memory = fx
        .memories
        .create_with_options(
            None,
            None,
            None,
            "summary",
            "Release notes",
            "alpha release template",
            &[],
            false,
        )
        .await?;

    let scoped = fx
        .memories
        .search_scoped(Some(&fx.project_a), "alpha", 10)
        .await?;
    let scoped_ids: Vec<&str> = scoped.iter().map(|memory| memory.id.as_str()).collect();
    assert!(scoped_ids.contains(&project_memory.id.as_str()));
    assert!(scoped_ids.contains(&global_memory.id.as_str()));
    assert!(!scoped_ids.contains(&other_memory.id.as_str()));

    let global_only = fx.memories.search_scoped(None, "alpha", 10).await?;
    assert_eq!(global_only.len(), 1);
    assert_eq!(global_only[0].id, global_memory.id);

    // An unsanitizable query returns no rows instead of an error.
    assert!(
        fx.memories
            .search_scoped(Some(&fx.project_a), "-", 10)
            .await?
            .is_empty()
    );

    Ok(())
}

#[tokio::test]
async fn pinned_memories_sort_first() -> Result<()> {
    let fx = fixture().await?;

    let regular = fx
        .memories
        .create_with_options(
            Some(&fx.project_a),
            None,
            None,
            "fact",
            "Zebra habits",
            "zebra grazing patterns",
            &[],
            false,
        )
        .await?;
    let pinned_global = fx
        .memories
        .create_with_options(
            None,
            None,
            None,
            "fact",
            "Zebra facts",
            "zebra stripes are unique",
            &[],
            true,
        )
        .await?;

    let scoped = fx
        .memories
        .search_scoped(Some(&fx.project_a), "zebra", 10)
        .await?;
    assert_eq!(scoped.len(), 2);
    assert_eq!(scoped[0].id, pinned_global.id);

    let listed = fx.memories.list_by_project(&fx.project_a, 10).await?;
    assert_eq!(listed.len(), 2);
    assert_eq!(listed[0].id, pinned_global.id);

    let pinned = fx.memories.list_pinned(Some(&fx.project_a), 10).await?;
    assert_eq!(pinned.len(), 1);
    assert_eq!(pinned[0].id, pinned_global.id);
    assert!(!pinned.iter().any(|memory| memory.id == regular.id));

    let global_pinned = fx.memories.list_pinned(None, 10).await?;
    assert_eq!(global_pinned.len(), 1);
    assert_eq!(global_pinned[0].id, pinned_global.id);

    Ok(())
}

#[tokio::test]
async fn set_tags_replaces_existing_tags() -> Result<()> {
    let fx = fixture().await?;

    let memory = fx
        .memories
        .create_with_options(
            Some(&fx.project_a),
            None,
            None,
            "fact",
            "Tagged",
            "Tagged content",
            &tags(&["alpha", "beta"]),
            false,
        )
        .await?;
    assert_eq!(
        fx.memories.list_tags(&memory.id).await?,
        tags(&["alpha", "beta"])
    );

    fx.memories
        .set_tags(&memory.id, &tags(&[" beta ", "gamma", "gamma", ""]))
        .await?;
    assert_eq!(
        fx.memories.list_tags(&memory.id).await?,
        tags(&["beta", "gamma"])
    );

    fx.memories.set_tags(&memory.id, &[]).await?;
    assert!(fx.memories.list_tags(&memory.id).await?.is_empty());

    assert!(
        fx.memories
            .set_tags("missing-memory", &tags(&["x"]))
            .await
            .is_err()
    );

    Ok(())
}

#[tokio::test]
async fn update_and_soft_delete_behave_consistently() -> Result<()> {
    let fx = fixture().await?;

    let memory = fx
        .memories
        .create_with_options(
            Some(&fx.project_a),
            None,
            None,
            "fact",
            "Draft note",
            "first version",
            &[],
            false,
        )
        .await?;

    let updated = fx
        .memories
        .update(
            &memory.id,
            Some("todo"),
            Some("  Refined note  "),
            Some(" second version "),
            Some(true),
        )
        .await?;
    assert_eq!(updated.id, memory.id);
    assert_eq!(updated.kind, "todo");
    assert_eq!(updated.title, "Refined note");
    assert_eq!(updated.content, "second version");
    assert_eq!(updated.pinned, 1);
    assert!(updated.updated_at >= memory.updated_at);

    assert!(
        fx.memories
            .update(&memory.id, Some("bogus"), None, None, None)
            .await
            .is_err()
    );
    assert!(
        fx.memories
            .update(&memory.id, None, Some("   "), None, None)
            .await
            .is_err()
    );
    assert!(
        fx.memories
            .update("missing-memory", None, Some("Title"), None, None)
            .await
            .is_err()
    );

    fx.memories.touch_last_used(&memory.id).await?;
    let touched = fx.memories.get_by_id(&memory.id).await?.expect("memory");
    assert!(touched.last_used_at.is_some());
    assert!(fx.memories.touch_last_used("missing-memory").await.is_err());

    assert!(fx.memories.soft_delete(&memory.id).await?);
    assert!(!fx.memories.soft_delete(&memory.id).await?);

    assert!(fx.memories.get_by_id(&memory.id).await?.is_none());
    assert!(fx.memories.search_fts("refined", 10).await?.is_empty());
    assert!(
        fx.memories
            .search_scoped(Some(&fx.project_a), "refined", 10)
            .await?
            .is_empty()
    );
    assert!(
        fx.memories
            .list_by_project(&fx.project_a, 10)
            .await?
            .is_empty()
    );
    assert!(
        fx.memories
            .list_pinned(Some(&fx.project_a), 10)
            .await?
            .is_empty()
    );
    assert!(
        fx.memories
            .update(&memory.id, None, Some("Nope"), None, None)
            .await
            .is_err()
    );
    assert!(fx.memories.touch_last_used(&memory.id).await.is_err());
    assert!(
        fx.memories
            .set_tags(&memory.id, &tags(&["x"]))
            .await
            .is_err()
    );

    Ok(())
}

#[tokio::test]
async fn export_markdown_scopes_memories_and_redacts_secrets() -> Result<()> {
    let fx = fixture().await?;

    let pinned = fx
        .memories
        .create_with_options(
            Some(&fx.project_a),
            None,
            None,
            "decision",
            "Deployment rule",
            "- never deploy on Friday\npassword=hunter2\nAuthorization: Bearer sk-live-secret",
            &tags(&["ops"]),
            true,
        )
        .await?;
    let global = fx
        .memories
        .create_with_options(
            None,
            None,
            None,
            "fact",
            "Global style",
            "Use tabs for shell scripts",
            &tags(&["style"]),
            false,
        )
        .await?;
    let other_project = fx
        .memories
        .create_with_options(
            Some(&fx.project_b),
            None,
            None,
            "fact",
            "Other project",
            "beta only",
            &[],
            false,
        )
        .await?;
    let deleted = fx
        .memories
        .create_with_options(
            Some(&fx.project_a),
            None,
            None,
            "todo",
            "Deleted todo",
            "should not be exported",
            &[],
            false,
        )
        .await?;
    assert!(fx.memories.soft_delete(&deleted.id).await?);

    let markdown = fx.memories.export_markdown(&fx.project_a).await?;

    assert!(markdown.starts_with("# Gaggle Memory"));
    assert!(markdown.contains("## Deployment rule"));
    assert!(markdown.contains("## Global style"));
    assert!(markdown.contains("- **Kind:** `decision`"));
    assert!(markdown.contains("- **Scope:** project"));
    assert!(markdown.contains("- **Scope:** global"));
    assert!(markdown.contains("`ops`"));
    assert!(markdown.contains("`style`"));

    assert!(!markdown.contains(&other_project.title));
    assert!(!markdown.contains(&other_project.content));
    assert!(!markdown.contains(&deleted.title));
    assert!(!markdown.contains(&deleted.content));

    // Secrets never leave the database.
    assert!(!markdown.contains("hunter2"));
    assert!(!markdown.contains("sk-live-secret"));
    assert!(markdown.contains("[REDACTED]"));

    // Pinned memories are exported first.
    let pinned_position = markdown
        .find(&format!("## {}", pinned.title))
        .expect("pinned heading");
    let global_position = markdown
        .find(&format!("## {}", global.title))
        .expect("global heading");
    assert!(pinned_position < global_position);

    // A project with no memories still exports a valid document.
    let empty_markdown = fx.memories.export_markdown("missing-project").await?;
    assert!(empty_markdown.starts_with("# Gaggle Memory"));

    Ok(())
}

#[tokio::test]
async fn project_settings_round_trip_and_missing() -> Result<()> {
    let fx = fixture().await?;

    assert!(fx.projects.get_settings(&fx.project_a).await?.is_none());

    let settings = ProjectSettings {
        project_id: fx.project_a.clone(),
        instructions: Some("Be careful".to_string()),
        ignore_globs: Some(r#"["*.log"]"#.to_string()),
        env: None,
        shell: Some("/bin/zsh".to_string()),
        setup_command: None,
    };
    fx.projects.upsert_settings(&settings).await?;

    let loaded = fx
        .projects
        .get_settings(&fx.project_a)
        .await?
        .expect("project settings");
    assert_eq!(loaded.project_id, fx.project_a);
    assert_eq!(loaded.instructions.as_deref(), Some("Be careful"));
    assert_eq!(loaded.ignore_globs.as_deref(), Some(r#"["*.log"]"#));
    assert_eq!(loaded.shell.as_deref(), Some("/bin/zsh"));

    assert!(fx.projects.get_settings(&fx.project_b).await?.is_none());

    Ok(())
}
