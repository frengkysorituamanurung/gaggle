use gaggle_core::WorkspacePathPolicy;
use gaggle_db::{
    models::{Chat, Worktree},
    repositories::{
        chats::ChatRepository, projects::ProjectRepository, worktrees::WorktreeRepository,
    },
};

/// Trait allowing [`WorktreeManager::create_for_chat`] to accept either a chat id
/// (`&str`/`String`) or a [`Chat`] reference.
pub trait CreateForChatArg {
    fn chat_id(&self) -> &str;
}

impl CreateForChatArg for str {
    fn chat_id(&self) -> &str {
        self
    }
}

impl CreateForChatArg for &str {
    fn chat_id(&self) -> &str {
        self
    }
}

impl CreateForChatArg for String {
    fn chat_id(&self) -> &str {
        self.as_str()
    }
}

impl CreateForChatArg for &String {
    fn chat_id(&self) -> &str {
        self.as_str()
    }
}

impl CreateForChatArg for Chat {
    fn chat_id(&self) -> &str {
        &self.id
    }
}

impl CreateForChatArg for &Chat {
    fn chat_id(&self) -> &str {
        &self.id
    }
}

/// Trait allowing worktree operations to accept a worktree id (`&str`/`String`),
/// a [`Worktree`] reference, or a [`Chat`] (resolved via its chat id).
pub trait WorktreeArg {
    fn worktree_key(&self) -> &str;
}

impl WorktreeArg for str {
    fn worktree_key(&self) -> &str {
        self
    }
}

impl WorktreeArg for &str {
    fn worktree_key(&self) -> &str {
        self
    }
}

impl WorktreeArg for String {
    fn worktree_key(&self) -> &str {
        self.as_str()
    }
}

impl WorktreeArg for &String {
    fn worktree_key(&self) -> &str {
        self.as_str()
    }
}

impl WorktreeArg for Worktree {
    fn worktree_key(&self) -> &str {
        &self.id
    }
}

impl WorktreeArg for &Worktree {
    fn worktree_key(&self) -> &str {
        &self.id
    }
}

impl WorktreeArg for Chat {
    fn worktree_key(&self) -> &str {
        &self.id
    }
}

impl WorktreeArg for &Chat {
    fn worktree_key(&self) -> &str {
        &self.id
    }
}
use sqlx::SqlitePool;

use crate::git_tools::{
    GitToolError, GitWorkspaceStatus, git_status, git_worktree_create, git_worktree_discard,
    git_worktree_merge,
};

/// Lifecycle manager for Git worktrees associated with chats.
///
/// `WorktreeManager` centralises the create/merge/discard/status operations that
/// combine Git commands (via the existing `git_worktree_*` helpers) with the
/// `worktrees`/`chats` database rows and audit logs.
#[derive(Clone)]
pub struct WorktreeManager {
    pub pool: SqlitePool,
}

impl WorktreeManager {
    pub fn new(pool: SqlitePool) -> Self {
        Self { pool }
    }

    pub fn pool(&self) -> SqlitePool {
        self.pool.clone()
    }

    /// Create and attach an isolated Git worktree for `chat_id`.
    ///
    /// The chat's project is resolved from the database; non-Git projects
    /// (`vcs_kind != "git"`) are rejected.
    pub async fn create_for_chat<T: CreateForChatArg>(
        &self,
        chat: T,
    ) -> Result<Worktree, GitToolError> {
        let chat_id = chat.chat_id();
        let chat_repo = ChatRepository::new(self.pool.clone());
        let chat = chat_repo
            .get_by_id(chat_id)
            .await
            .map_err(db_err)?
            .ok_or_else(|| GitToolError::ExecutionFailed(format!("Chat '{chat_id}' not found")))?;

        let proj_repo = ProjectRepository::new(self.pool.clone());
        let project = proj_repo
            .get_by_id(&chat.project_id)
            .await
            .map_err(db_err)?
            .ok_or_else(|| {
                GitToolError::ExecutionFailed(format!("Project '{}' not found", chat.project_id))
            })?;

        if project.vcs_kind != "git" {
            return Err(GitToolError::NotGitRepository);
        }

        let policy = WorkspacePathPolicy::new(&project.root_path).map_err(GitToolError::Policy)?;

        let vcs_kind = project.vcs_kind.clone();
        let chat_id_clone = chat.id.clone();
        let spec = tokio::task::spawn_blocking(move || {
            git_worktree_create(&policy, &vcs_kind, &chat_id_clone)
        })
        .await
        .map_err(|e| GitToolError::ExecutionFailed(e.to_string()))??;

        let wt_repo = WorktreeRepository::new(self.pool.clone());
        let wt = wt_repo
            .create_and_attach_to_chat(
                &project.id,
                &chat.id,
                &spec.path,
                &spec.branch,
                &spec.base_ref,
                &spec.base_commit,
            )
            .await
            .map_err(db_err)?;

        Ok(wt)
    }

    /// Convenience variant accepting a project id explicitly (resolves the chat internally).
    pub async fn create_for_chat_in_project(
        &self,
        _project_id: &str,
        chat_id: &str,
    ) -> Result<Worktree, GitToolError> {
        self.create_for_chat(chat_id).await
    }

    async fn resolve_worktree<T: WorktreeArg>(
        &self,
        worktree: T,
    ) -> Result<Worktree, GitToolError> {
        let id = worktree.worktree_key();
        let wt_repo = WorktreeRepository::new(self.pool.clone());
        if let Some(wt) = wt_repo.get_by_id(id).await.map_err(db_err)? {
            return Ok(wt);
        }
        if let Some(wt) = wt_repo.get_by_chat_id(id).await.map_err(db_err)? {
            return Ok(wt);
        }
        Err(GitToolError::ExecutionFailed(format!(
            "Worktree '{id}' not found"
        )))
    }

    pub async fn get<T: WorktreeArg>(&self, worktree: T) -> Result<Worktree, GitToolError> {
        self.resolve_worktree(worktree).await
    }

    /// Merge a worktree branch into the main project repository.
    ///
    /// `confirm` must be `true` to perform the destructive merge; otherwise the
    /// underlying Git helper rejects the operation.
    pub async fn merge<T: WorktreeArg>(
        &self,
        worktree: T,
        confirm: bool,
    ) -> Result<Worktree, GitToolError> {
        self.merge_with_actor(worktree, confirm, None).await
    }

    pub async fn merge_with_actor<T: WorktreeArg>(
        &self,
        worktree: T,
        confirm: bool,
        decided_by_device: Option<&str>,
    ) -> Result<Worktree, GitToolError> {
        let wt = self.resolve_worktree(worktree).await?;
        let proj_repo = ProjectRepository::new(self.pool.clone());
        let project = proj_repo
            .get_by_id(&wt.project_id)
            .await
            .map_err(db_err)?
            .ok_or_else(|| {
                GitToolError::ExecutionFailed(format!("Project '{}' not found", wt.project_id))
            })?;

        let policy = WorkspacePathPolicy::new(&project.root_path).map_err(GitToolError::Policy)?;

        let wt_path = wt.path.clone();
        let wt_branch = wt.branch.clone();
        tokio::task::spawn_blocking(move || {
            git_worktree_merge(&policy, &wt_path, &wt_branch, confirm)
        })
        .await
        .map_err(|e| GitToolError::ExecutionFailed(e.to_string()))??;

        let wt_repo = WorktreeRepository::new(self.pool.clone());
        let updated = wt_repo
            .transition_status(&wt.id, "merged", decided_by_device)
            .await
            .map_err(db_err)?;
        Ok(updated)
    }

    /// Discard an isolated worktree and its branch.
    pub async fn discard<T: WorktreeArg>(
        &self,
        worktree: T,
        confirm: bool,
    ) -> Result<Worktree, GitToolError> {
        self.discard_with_actor(worktree, confirm, None).await
    }

    pub async fn discard_with_actor<T: WorktreeArg>(
        &self,
        worktree: T,
        confirm: bool,
        decided_by_device: Option<&str>,
    ) -> Result<Worktree, GitToolError> {
        let wt = self.resolve_worktree(worktree).await?;
        let proj_repo = ProjectRepository::new(self.pool.clone());
        let project = proj_repo
            .get_by_id(&wt.project_id)
            .await
            .map_err(db_err)?
            .ok_or_else(|| {
                GitToolError::ExecutionFailed(format!("Project '{}' not found", wt.project_id))
            })?;

        let policy = WorkspacePathPolicy::new(&project.root_path).map_err(GitToolError::Policy)?;

        let wt_path = wt.path.clone();
        let wt_branch = wt.branch.clone();
        tokio::task::spawn_blocking(move || {
            git_worktree_discard(&policy, &wt_path, &wt_branch, confirm)
        })
        .await
        .map_err(|e| GitToolError::ExecutionFailed(e.to_string()))??;

        let wt_repo = WorktreeRepository::new(self.pool.clone());
        let updated = wt_repo
            .transition_status(&wt.id, "discarded", decided_by_device)
            .await
            .map_err(db_err)?;
        Ok(updated)
    }

    /// Inspect the Git status of a worktree.
    pub async fn status<T: WorktreeArg>(
        &self,
        worktree: T,
    ) -> Result<GitWorkspaceStatus, GitToolError> {
        let wt = self.resolve_worktree(worktree).await?;
        let policy = WorkspacePathPolicy::new(&wt.path).map_err(GitToolError::Policy)?;
        tokio::task::spawn_blocking(move || git_status(&policy))
            .await
            .map_err(|e| GitToolError::ExecutionFailed(e.to_string()))?
    }

    /// List worktrees for a project.
    pub async fn list_by_project(&self, project_id: &str) -> Result<Vec<Worktree>, GitToolError> {
        let wt_repo = WorktreeRepository::new(self.pool.clone());
        wt_repo.list_by_project(project_id).await.map_err(db_err)
    }
}

fn db_err(e: impl std::fmt::Display) -> GitToolError {
    GitToolError::ExecutionFailed(e.to_string())
}
