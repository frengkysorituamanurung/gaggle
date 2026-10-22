use anyhow::Result;
use gaggle_db::{
    connect_in_memory,
    repositories::{
        chats::ChatRepository, extensions::ExtensionRepository, memories::MemoryRepository,
        permissions::PermissionRepository, projects::ProjectRepository, runs::RunRepository,
    },
};

#[tokio::test]
async fn test_schema_migrations_and_repositories() -> Result<()> {
    let pool = connect_in_memory().await?;

    // 1. Create Project
    let proj_repo = ProjectRepository::new(pool.clone());
    let project = proj_repo
        .create("gaggle", "/Users/frengky/Documents/projects/gaggle", "git")
        .await?;
    assert_eq!(project.name, "gaggle");
    assert_eq!(project.default_mode, "local");
    assert_eq!(project.default_access, "ask");

    // 2. Create Chat & Messages
    let chat_repo = ChatRepository::new(pool.clone());
    let chat = chat_repo
        .create(
            &project.id,
            "Initial Architecture Chat",
            "user",
            "local",
            "ask",
        )
        .await?;
    assert_eq!(chat.permission_mode, "ask");

    let msg = chat_repo
        .append_message(&chat.id, None, "user", "Hello Gaggle workspace!")
        .await?;
    assert_eq!(msg.seq, 1);

    // 3. Export Chat
    let exported = chat_repo.export_chat(&chat.id).await?.expect("chat bundle");
    assert_eq!(exported.messages.len(), 1);
    assert_eq!(exported.message_parts.len(), 1);
    assert_eq!(
        exported.message_parts[0].content.as_deref(),
        Some("Hello Gaggle workspace!")
    );

    // 4. Durable Run, Events, Checkpoints, Tool Calls & Approvals
    let run_repo = RunRepository::new(pool.clone());
    let run = run_repo.create(&chat.id, Some(&msg.id), None).await?;
    assert_eq!(run.status, "running");

    let ev1 = run_repo
        .emit_event(&run.id, "run.started", r#"{"step":0}"#)
        .await?;
    assert_eq!(ev1.seq, 1);

    let cp = run_repo
        .save_checkpoint(&run.id, 1, r#"{"cursor":"step_1"}"#)
        .await?;
    assert_eq!(cp.step_index, 1);

    let tc = run_repo
        .record_tool_call(
            &run.id,
            1,
            "code_mode",
            "shell.exec",
            r#"{"command":"cargo test"}"#,
            "awaiting_approval",
        )
        .await?;
    let approval = run_repo
        .create_approval(
            &tc.id,
            "shell",
            "Run cargo test",
            Some("cargo test"),
            Some(&project.root_path),
        )
        .await?;
    assert_eq!(approval.status, "pending");

    // 5. Memories + FTS5 Search
    let mem_repo = MemoryRepository::new(pool.clone());
    mem_repo
        .create(
            Some(&project.id),
            "decision",
            "SQLite WAL Mode",
            "Use global SQLite database with WAL and FTS5 tables",
        )
        .await?;
    let search_hits = mem_repo.search_fts("SQLite", 10).await?;
    assert_eq!(search_hits.len(), 1);
    assert_eq!(search_hits[0].title, "SQLite WAL Mode");

    // 6. Extensions & Tool Permissions (COALESCE unique index)
    let ext_repo = ExtensionRepository::new(pool.clone());
    let ext = ext_repo
        .upsert_builtin("gaggle.shell", "Built-in shell extension", "0.1.0")
        .await?;
    let tool = ext_repo
        .upsert_tool(
            &ext.id,
            "shell.exec",
            "Run command",
            r#"{"type":"object"}"#,
            false,
        )
        .await?;

    let perm_repo = PermissionRepository::new(pool.clone());
    let perm = perm_repo
        .set_tool_permission(None, &tool.id, "ask_before")
        .await?;
    assert_eq!(perm.level, "ask_before");

    Ok(())
}
