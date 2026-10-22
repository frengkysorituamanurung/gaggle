use crate::{
    connection::{new_id, now_ms},
    models::{Project, ProjectSettings},
};
use anyhow::Result;
use sqlx::SqlitePool;

pub struct ProjectRepository {
    pool: SqlitePool,
}

impl ProjectRepository {
    pub fn new(pool: SqlitePool) -> Self {
        Self { pool }
    }

    pub async fn create(&self, name: &str, root_path: &str, vcs_kind: &str) -> Result<Project> {
        let id = new_id();
        let now = now_ms();

        sqlx::query(
            r#"
            INSERT INTO projects (
                id, name, root_path, vcs_kind, default_mode, default_access, created_at, updated_at
            ) VALUES (?, ?, ?, ?, 'local', 'ask', ?, ?)
            "#,
        )
        .bind(&id)
        .bind(name)
        .bind(root_path)
        .bind(vcs_kind)
        .bind(now)
        .bind(now)
        .execute(&self.pool)
        .await?;

        self.get_by_id(&id)
            .await?
            .ok_or_else(|| anyhow::anyhow!("Project not found after insert"))
    }

    pub async fn get_by_id(&self, id: &str) -> Result<Option<Project>> {
        let project = sqlx::query_as::<_, Project>("SELECT * FROM projects WHERE id = ?")
            .bind(id)
            .fetch_optional(&self.pool)
            .await?;
        Ok(project)
    }

    pub async fn list_active(&self) -> Result<Vec<Project>> {
        let projects = sqlx::query_as::<_, Project>(
            "SELECT * FROM projects WHERE archived_at IS NULL ORDER BY updated_at DESC",
        )
        .fetch_all(&self.pool)
        .await?;
        Ok(projects)
    }

    pub async fn upsert_settings(&self, settings: &ProjectSettings) -> Result<()> {
        sqlx::query(
            r#"
            INSERT INTO project_settings (project_id, instructions, ignore_globs, env, shell, setup_command)
            VALUES (?, ?, ?, ?, ?, ?)
            ON CONFLICT(project_id) DO UPDATE SET
                instructions = excluded.instructions,
                ignore_globs = excluded.ignore_globs,
                env = excluded.env,
                shell = excluded.shell,
                setup_command = excluded.setup_command
            "#,
        )
        .bind(&settings.project_id)
        .bind(&settings.instructions)
        .bind(&settings.ignore_globs)
        .bind(&settings.env)
        .bind(&settings.shell)
        .bind(&settings.setup_command)
        .execute(&self.pool)
        .await?;
        Ok(())
    }
}
