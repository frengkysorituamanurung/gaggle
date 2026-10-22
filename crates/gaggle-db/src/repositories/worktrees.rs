use crate::{
    connection::{new_id, now_ms},
    models::Worktree,
};
use anyhow::{Context, Result, bail};
use sqlx::SqlitePool;

pub struct WorktreeRepository {
    pool: SqlitePool,
}

impl WorktreeRepository {
    pub fn new(pool: SqlitePool) -> Self {
        Self { pool }
    }

    /// Creates a `worktrees` row, links `chats.worktree_id` and `chats.mode = 'worktree'`,
    /// and appends an `audit_logs` entry inside a single SQLite transaction.
    pub async fn create_and_attach_to_chat(
        &self,
        project_id: &str,
        chat_id: &str,
        path: &str,
        branch: &str,
        base_ref: &str,
        base_commit: &str,
    ) -> Result<Worktree> {
        let id = new_id();
        let now = now_ms();
        let mut tx = self.pool.begin().await?;

        sqlx::query(
            r#"
            INSERT INTO worktrees (
                id, project_id, chat_id, path, branch, base_ref, base_commit, status, created_at
            ) VALUES (?, ?, ?, ?, ?, ?, ?, 'active', ?)
            "#,
        )
        .bind(&id)
        .bind(project_id)
        .bind(chat_id)
        .bind(path)
        .bind(branch)
        .bind(base_ref)
        .bind(base_commit)
        .bind(now)
        .execute(&mut *tx)
        .await?;

        sqlx::query(
            r#"
            UPDATE chats
            SET worktree_id = ?, mode = 'worktree', updated_at = ?
            WHERE id = ?
            "#,
        )
        .bind(&id)
        .bind(now)
        .bind(chat_id)
        .execute(&mut *tx)
        .await?;

        let audit_id = new_id();
        let metadata = serde_json::json!({
            "project_id": project_id,
            "chat_id": chat_id,
            "path": path,
            "branch": branch,
            "base_ref": base_ref,
            "base_commit": base_commit,
        })
        .to_string();

        sqlx::query(
            r#"
            INSERT INTO audit_logs (id, actor_kind, action, entity_type, entity_id, metadata, created_at)
            VALUES (?, 'system', 'worktree.created', 'worktree', ?, ?, ?)
            "#,
        )
        .bind(&audit_id)
        .bind(&id)
        .bind(&metadata)
        .bind(now)
        .execute(&mut *tx)
        .await?;

        tx.commit().await?;

        self.get_by_id(&id)
            .await?
            .context("Worktree not found after creation")
    }

    pub async fn get_by_id(&self, id: &str) -> Result<Option<Worktree>> {
        let wt = sqlx::query_as::<_, Worktree>("SELECT * FROM worktrees WHERE id = ?")
            .bind(id)
            .fetch_optional(&self.pool)
            .await?;
        Ok(wt)
    }

    pub async fn get_by_chat_id(&self, chat_id: &str) -> Result<Option<Worktree>> {
        let wt = sqlx::query_as::<_, Worktree>("SELECT * FROM worktrees WHERE chat_id = ?")
            .bind(chat_id)
            .fetch_optional(&self.pool)
            .await?;
        Ok(wt)
    }

    pub async fn list_by_project(&self, project_id: &str) -> Result<Vec<Worktree>> {
        let list = sqlx::query_as::<_, Worktree>(
            "SELECT * FROM worktrees WHERE project_id = ? ORDER BY created_at DESC",
        )
        .bind(project_id)
        .fetch_all(&self.pool)
        .await?;
        Ok(list)
    }

    /// Transitions an `active` worktree to `merged` or `discarded` and records an `audit_logs` entry.
    pub async fn transition_status(
        &self,
        worktree_id: &str,
        new_status: &str,
        actor_device: Option<&str>,
    ) -> Result<Worktree> {
        if !matches!(new_status, "merged" | "discarded") {
            bail!("Invalid worktree target status '{new_status}'");
        }

        let now = now_ms();
        let mut tx = self.pool.begin().await?;

        let existing = sqlx::query_as::<_, Worktree>("SELECT * FROM worktrees WHERE id = ?")
            .bind(worktree_id)
            .fetch_optional(&mut *tx)
            .await?
            .with_context(|| format!("Worktree '{worktree_id}' not found"))?;

        if existing.status != "active" {
            bail!(
                "Worktree '{}' is already in '{}' status and cannot transition to '{}'",
                worktree_id,
                existing.status,
                new_status
            );
        }

        sqlx::query("UPDATE worktrees SET status = ? WHERE id = ?")
            .bind(new_status)
            .bind(worktree_id)
            .execute(&mut *tx)
            .await?;

        let audit_id = new_id();
        let metadata = serde_json::json!({
            "project_id": existing.project_id,
            "chat_id": existing.chat_id,
            "branch": existing.branch,
            "path": existing.path,
            "new_status": new_status,
        })
        .to_string();

        sqlx::query(
            r#"
            INSERT INTO audit_logs (id, actor_kind, actor_id, action, entity_type, entity_id, metadata, created_at)
            VALUES (?, 'user', ?, ?, 'worktree', ?, ?, ?)
            "#,
        )
        .bind(&audit_id)
        .bind(actor_device)
        .bind(format!("worktree.{new_status}"))
        .bind(worktree_id)
        .bind(&metadata)
        .bind(now)
        .execute(&mut *tx)
        .await?;

        tx.commit().await?;

        self.get_by_id(worktree_id)
            .await?
            .context("Worktree missing after status update")
    }
}
