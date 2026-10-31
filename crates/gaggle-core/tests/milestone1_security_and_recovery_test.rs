use anyhow::Result;
use gaggle_core::{RunEngine, WorkspacePathError, WorkspacePathPolicy, collect_diagnostics};
use gaggle_db::{
    connect_in_memory,
    models::Approval,
    repositories::{chats::ChatRepository, projects::ProjectRepository, runs::RunRepository},
};
use std::path::Path;

#[test]
fn test_workspace_path_policy_traversal_and_sensitive_files() -> Result<()> {
    let base_temp = std::env::temp_dir().join(format!("gaggle_test_{}", uuid::Uuid::now_v7()));
    let workspace_dir = base_temp.join("project_a");
    let outside_dir = base_temp.join("outside_secret");
    std::fs::create_dir_all(workspace_dir.join("src"))?;
    std::fs::create_dir_all(&outside_dir)?;

    let main_rs = workspace_dir.join("src/main.rs");
    std::fs::write(&main_rs, "fn main() {}")?;
    let env_file = workspace_dir.join(".env");
    std::fs::write(&env_file, "SECRET=123")?;
    let outside_file = outside_dir.join("passwords.txt");
    std::fs::write(&outside_file, "topsecret")?;

    let policy = WorkspacePathPolicy::new(&workspace_dir)?;

    // 1. Valid file inside workspace succeeds
    let resolved_main = policy.resolve_existing_for_read("src/main.rs")?;
    assert!(resolved_main.ends_with("src/main.rs"));

    // 2. Non-existent file inside workspace for write succeeds
    let resolved_new = policy.resolve_for_write("src/new_module.rs")?;
    assert!(resolved_new.ends_with("src/new_module.rs"));

    // 3. Parent traversal (`../`) escaping workspace is rejected
    let traversal_err = policy
        .resolve_existing_for_read("../outside_secret/passwords.txt")
        .unwrap_err();
    assert!(matches!(
        traversal_err,
        WorkspacePathError::OutsideWorkspace { .. }
    ));

    let write_traversal_err = policy.resolve_for_write("../../escaped.txt").unwrap_err();
    assert!(matches!(
        write_traversal_err,
        WorkspacePathError::OutsideWorkspace { .. }
    ));

    // 4. Sensitive file (`.env` and `.ssh/id_ed25519`) inside workspace is denied
    let env_err = policy.resolve_existing_for_read(".env").unwrap_err();
    assert!(matches!(
        env_err,
        WorkspacePathError::SensitivePathDenied(_)
    ));

    let ssh_write_err = policy.resolve_for_write(".ssh/id_ed25519").unwrap_err();
    assert!(matches!(
        ssh_write_err,
        WorkspacePathError::SensitivePathDenied(_)
    ));

    // 5. Symlink escape pointing outside the workspace is rejected
    #[cfg(unix)]
    {
        let symlink_path = workspace_dir.join("src/evil_link.txt");
        std::os::unix::fs::symlink(&outside_file, &symlink_path)?;
        let symlink_err = policy
            .resolve_existing_for_read("src/evil_link.txt")
            .unwrap_err();
        assert!(matches!(
            symlink_err,
            WorkspacePathError::OutsideWorkspace { .. }
        ));
    }

    // 6. Restricted system root (`/`) cannot be registered as a project workspace
    let root_err = WorkspacePathPolicy::new("/").unwrap_err();
    assert!(matches!(
        root_err,
        WorkspacePathError::RestrictedRootPath(_)
    ));

    let _ = std::fs::remove_dir_all(&base_temp);
    Ok(())
}

#[tokio::test]
async fn test_safe_run_recovery_and_diagnostics() -> Result<()> {
    let pool = connect_in_memory().await?;
    let proj_repo = ProjectRepository::new(pool.clone());
    let chat_repo = ChatRepository::new(pool.clone());
    let run_repo = RunRepository::new(pool.clone());

    let project = proj_repo
        .create("gaggle", "/Users/frengky/Documents/projects/gaggle", "git")
        .await?;
    let chat = chat_repo
        .create(&project.id, "Recovery Test Chat", "user", "local", "ask")
        .await?;

    // Simulate a run interrupted while waiting for approval
    let run = run_repo.create(&chat.id, None, None).await?;
    let tc = run_repo
        .record_tool_call(
            &run.id,
            1,
            "native",
            "shell.exec",
            r#"{"command":"cargo test"}"#,
            "awaiting_approval",
        )
        .await?;
    let approval = run_repo
        .create_approval(
            &tc.id,
            "shell",
            "Execute cargo test",
            Some("cargo test"),
            None,
        )
        .await?;
    assert_eq!(approval.status, "pending");

    // Run startup recovery
    let engine = RunEngine::new(pool.clone());
    let recovered = engine.recover_unfinished_runs().await?;
    assert_eq!(recovered.len(), 1);
    assert_eq!(recovered[0].status, "paused");
    assert_eq!(recovered[0].resume_count, 1);

    // Pending approval must be expired so stale approvals cannot be replayed after crash
    let updated_approval: Approval = sqlx::query_as("SELECT * FROM approvals WHERE id = ?")
        .bind(&approval.id)
        .fetch_one(&pool)
        .await?;
    assert_eq!(updated_approval.status, "expired");
    assert!(updated_approval.decided_at.is_some());

    // Diagnostics check
    let diag = collect_diagnostics(&pool, Path::new(":memory:")).await?;
    assert!(diag.db_healthy);
    assert!(diag.foreign_keys_enabled);
    assert_eq!(diag.applied_migrations, 4);
    assert_eq!(diag.active_projects_count, 1);
    assert_eq!(diag.pending_approvals_count, 0);

    Ok(())
}
