use anyhow::Result;
use gaggle_core::WorkspacePathPolicy;
use gaggle_db::{
    connect_in_memory,
    connection::new_id,
    repositories::{
        chats::ChatRepository, projects::ProjectRepository, worktrees::WorktreeRepository,
    },
};
use gaggle_tools::{WorktreeManager, git_status};
use std::process::Command;

fn init_temp_git_repo() -> Result<std::path::PathBuf> {
    let temp_root = std::env::temp_dir().join(format!("gaggle_wtm_{}", new_id()));
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
async fn test_worktree_manager_create_status_merge() -> Result<()> {
    let temp_root = init_temp_git_repo()?;
    let project_policy = WorkspacePathPolicy::new(&temp_root)?;

    let pool = connect_in_memory().await?;
    let proj_repo = ProjectRepository::new(pool.clone());
    let chat_repo = ChatRepository::new(pool.clone());
    let wt_repo = WorktreeRepository::new(pool.clone());

    let project = proj_repo
        .create(
            "wtm-merge-proj",
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

    let manager = WorktreeManager::new(pool.clone());

    // non-git rejected
    let non_git_pool = connect_in_memory().await?;
    let non_git_proj = ProjectRepository::new(non_git_pool.clone());
    let non_git_chat_repo = ChatRepository::new(non_git_pool.clone());
    let non_git_project = non_git_proj.create("p", "/tmp", "none").await?;
    let non_git_chat = non_git_chat_repo
        .create(&non_git_project.id, "c", "user", "worktree", "ask")
        .await?;
    let non_git_manager = WorktreeManager::new(non_git_pool.clone());
    let non_git_err = non_git_manager
        .create_for_chat(&non_git_chat.id)
        .await
        .unwrap_err();
    assert!(matches!(
        non_git_err,
        gaggle_tools::GitToolError::NotGitRepository
    ));

    // create
    let wt = manager.create_for_chat(&chat.id).await?;
    assert_eq!(wt.status, "active");

    let updated_chat = chat_repo.get_by_id(&chat.id).await?.unwrap();
    assert_eq!(updated_chat.worktree_id.as_deref(), Some(wt.id.as_str()));

    // status
    let status = manager.status(&wt.id).await?;
    assert!(status.is_clean);

    // unconfirmed merge rejected
    let unconfirmed = manager.merge(&wt.id, false).await.unwrap_err();
    assert!(matches!(
        unconfirmed,
        gaggle_tools::GitToolError::ConfirmationRequired("merge")
    ));

    // merge
    manager.merge(&wt.id, true).await?;
    let merged_wt = wt_repo.get_by_id(&wt.id).await?.unwrap();
    assert_eq!(merged_wt.status, "merged");

    let merged_main_readme = std::fs::read_to_string(temp_root.join("README.md"))?;
    // no changes were made in worktree, but merge succeeded
    assert_eq!(merged_main_readme, "# Main Workspace\n");

    let _ = std::fs::remove_dir_all(&temp_root);
    Ok(())
}

#[tokio::test]
async fn test_worktree_manager_discard() -> Result<()> {
    let temp_root = init_temp_git_repo()?;
    let project_policy = WorkspacePathPolicy::new(&temp_root)?;

    let pool = connect_in_memory().await?;
    let proj_repo = ProjectRepository::new(pool.clone());
    let chat_repo = ChatRepository::new(pool.clone());
    let wt_repo = WorktreeRepository::new(pool.clone());

    let project = proj_repo
        .create(
            "wtm-discard-proj",
            &project_policy.canonical_root().to_string_lossy(),
            "git",
        )
        .await?;
    let chat = chat_repo
        .create(&project.id, "Discard Chat", "user", "worktree", "ask")
        .await?;

    let manager = WorktreeManager::new(pool.clone());
    let wt = manager.create_for_chat(&chat.id).await?;

    std::fs::write(
        std::path::Path::new(&wt.path).join("README.md"),
        "discarded change\n",
    )?;

    let unconfirmed = manager.discard(&wt.id, false).await.unwrap_err();
    assert!(matches!(
        unconfirmed,
        gaggle_tools::GitToolError::ConfirmationRequired("discard")
    ));

    manager.discard(&wt.id, true).await?;
    let discarded_wt = wt_repo.get_by_id(&wt.id).await?.unwrap();
    assert_eq!(discarded_wt.status, "discarded");
    assert!(!std::path::Path::new(&wt.path).exists());

    let main_status = git_status(&project_policy)?;
    assert!(main_status.is_clean);

    let _ = std::fs::remove_dir_all(&temp_root);
    Ok(())
}
