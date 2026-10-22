use anyhow::Result;
use gaggle_core::{ToolExecutor, WorkspacePathPolicy};
use gaggle_db::{
    connect_in_memory,
    connection::new_id,
    repositories::{
        chats::ChatRepository, projects::ProjectRepository, runs::RunRepository,
        worktrees::WorktreeRepository,
    },
};
use gaggle_tools::{
    BuiltinToolExecutor, DEFAULT_MAX_DIFF_BYTES, GitToolError, git_diff, git_status,
    git_worktree_create, git_worktree_discard, git_worktree_merge, register_builtin_extensions,
};
use std::process::Command;

fn init_temp_git_repo() -> Result<std::path::PathBuf> {
    let temp_root = std::env::temp_dir().join(format!("gaggle_wt_{}", new_id()));
    std::fs::create_dir_all(&temp_root)?;

    assert!(
        Command::new("git")
            .arg("-C")
            .arg(&temp_root)
            .args(["init", "-b", "main"])
            .status()?
            .success()
    );
    assert!(
        Command::new("git")
            .arg("-C")
            .arg(&temp_root)
            .args(["config", "user.email", "test@gaggle.local"])
            .status()?
            .success()
    );
    assert!(
        Command::new("git")
            .arg("-C")
            .arg(&temp_root)
            .args(["config", "user.name", "Gaggle Test"])
            .status()?
            .success()
    );

    std::fs::write(temp_root.join("README.md"), "# Main Workspace\n")?;
    std::fs::write(temp_root.join(".gitignore"), ".gaggle/\n")?;
    assert!(
        Command::new("git")
            .arg("-C")
            .arg(&temp_root)
            .args(["add", "README.md", ".gitignore"])
            .status()?
            .success()
    );
    assert!(
        Command::new("git")
            .arg("-C")
            .arg(&temp_root)
            .args(["commit", "-m", "initial commit"])
            .status()?
            .success()
    );

    Ok(temp_root)
}

#[tokio::test]
async fn test_worktree_creation_tool_isolation_and_merge() -> Result<()> {
    let temp_root = init_temp_git_repo()?;
    let project_policy = WorkspacePathPolicy::new(&temp_root)?;

    let pool = connect_in_memory().await?;
    register_builtin_extensions(&pool).await?;

    let proj_repo = ProjectRepository::new(pool.clone());
    let chat_repo = ChatRepository::new(pool.clone());
    let run_repo = RunRepository::new(pool.clone());
    let wt_repo = WorktreeRepository::new(pool.clone());

    let project = proj_repo
        .create(
            "wt-merge-proj",
            &project_policy.canonical_root().to_string_lossy(),
            "git",
        )
        .await?;
    let chat = chat_repo
        .create(
            &project.id,
            "Worktree Feature Chat",
            "user",
            "worktree",
            "ask",
        )
        .await?;

    // 1. Create Git worktree and attach to chat
    let spec = git_worktree_create(&project_policy, &project.vcs_kind, &chat.id)?;
    assert_eq!(spec.base_ref, "main");
    assert_eq!(spec.branch, format!("gaggle/chat-{}", chat.id));

    let wt = wt_repo
        .create_and_attach_to_chat(
            &project.id,
            &chat.id,
            &spec.path,
            &spec.branch,
            &spec.base_ref,
            &spec.base_commit,
        )
        .await?;
    assert_eq!(wt.status, "active");

    let updated_chat = chat_repo.get_by_id(&chat.id).await?.unwrap();
    assert_eq!(updated_chat.worktree_id.as_deref(), Some(wt.id.as_str()));

    // 2. Execute `fs.write` via BuiltinToolExecutor on the worktree chat
    let run = run_repo.create(&chat.id, None, None).await?;
    let executor = BuiltinToolExecutor::new(pool.clone());
    let write_args = serde_json::json!({
        "path": "README.md",
        "content": "# Main Workspace\nIsolated worktree line\n"
    })
    .to_string();
    executor
        .execute_tool(
            &run.id,
            &updated_chat,
            &project.root_path,
            "fs.write",
            &write_args,
        )
        .await
        .map_err(anyhow::Error::msg)?;

    // Main project workspace must remain untouched prior to merge
    let main_readme = std::fs::read_to_string(temp_root.join("README.md"))?;
    assert_eq!(main_readme, "# Main Workspace\n");

    // Worktree directory contains the modification and shows up in worktree git_diff
    let wt_policy = WorkspacePathPolicy::new(&wt.path)?;
    let wt_diff = git_diff(&wt_policy, None, DEFAULT_MAX_DIFF_BYTES)?;
    assert!(wt_diff.patch.contains("+Isolated worktree line"));

    // 3. Merging without `confirm = true` is rejected
    let unconfirmed_err =
        git_worktree_merge(&project_policy, &wt.path, &wt.branch, false).unwrap_err();
    assert!(matches!(
        unconfirmed_err,
        GitToolError::ConfirmationRequired("merge")
    ));

    // 4. Merging with `confirm = true` merges into main and transitions worktree status to `merged`
    git_worktree_merge(&project_policy, &wt.path, &wt.branch, true)?;
    let merged_wt = wt_repo.transition_status(&wt.id, "merged", None).await?;
    assert_eq!(merged_wt.status, "merged");

    let merged_main_readme = std::fs::read_to_string(temp_root.join("README.md"))?;
    assert!(merged_main_readme.contains("Isolated worktree line"));

    // Audit logs recorded both creation and merge
    let audits = run_repo.list_audit_logs(10).await?;
    assert!(audits.iter().any(|a| a.action == "worktree.created"));
    assert!(audits.iter().any(|a| a.action == "worktree.merged"));

    let _ = std::fs::remove_dir_all(&temp_root);
    Ok(())
}

#[tokio::test]
async fn test_worktree_discard_and_security_validations() -> Result<()> {
    let temp_root = init_temp_git_repo()?;
    let project_policy = WorkspacePathPolicy::new(&temp_root)?;

    // 1. Non-git project (`vcs_kind = "none"`) is rejected
    let non_git_err = git_worktree_create(&project_policy, "none", "chat-123").unwrap_err();
    assert!(matches!(non_git_err, GitToolError::NotGitRepository));

    // 2. Path traversal in chat_id is rejected
    let bad_id_err = git_worktree_create(&project_policy, "git", "../escape").unwrap_err();
    assert!(matches!(bad_id_err, GitToolError::InvalidIdentifier(_)));

    // 3. Create worktree, modify a file inside it, then discard it
    let pool = connect_in_memory().await?;
    let proj_repo = ProjectRepository::new(pool.clone());
    let chat_repo = ChatRepository::new(pool.clone());
    let wt_repo = WorktreeRepository::new(pool.clone());

    let project = proj_repo
        .create(
            "wt-discard-proj",
            &project_policy.canonical_root().to_string_lossy(),
            "git",
        )
        .await?;
    let chat = chat_repo
        .create(&project.id, "Discard Chat", "user", "worktree", "ask")
        .await?;

    let spec = git_worktree_create(&project_policy, "git", &chat.id)?;
    let wt = wt_repo
        .create_and_attach_to_chat(
            &project.id,
            &chat.id,
            &spec.path,
            &spec.branch,
            &spec.base_ref,
            &spec.base_commit,
        )
        .await?;

    std::fs::write(
        std::path::Path::new(&wt.path).join("README.md"),
        "discarded change\n",
    )?;

    // Discard without confirmation fails
    let unconfirmed_err =
        git_worktree_discard(&project_policy, &wt.path, &wt.branch, false).unwrap_err();
    assert!(matches!(
        unconfirmed_err,
        GitToolError::ConfirmationRequired("discard")
    ));

    // Discard with confirmation removes worktree and deletes branch
    git_worktree_discard(&project_policy, &wt.path, &wt.branch, true)?;
    let discarded_wt = wt_repo.transition_status(&wt.id, "discarded", None).await?;
    assert_eq!(discarded_wt.status, "discarded");
    assert!(!std::path::Path::new(&wt.path).exists());

    // Main project workspace remains clean and unchanged
    let main_status = git_status(&project_policy)?;
    assert!(main_status.is_clean);
    let main_readme = std::fs::read_to_string(temp_root.join("README.md"))?;
    assert_eq!(main_readme, "# Main Workspace\n");

    let _ = std::fs::remove_dir_all(&temp_root);
    Ok(())
}
