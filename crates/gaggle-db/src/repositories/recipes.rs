use crate::{
    connection::{new_id, now_ms},
    models::{Recipe, RecipeParameter, Schedule},
};
use anyhow::Result;
use sqlx::SqlitePool;

pub struct RecipeRepository {
    pool: SqlitePool,
}

impl RecipeRepository {
    pub fn new(pool: SqlitePool) -> Self {
        Self { pool }
    }

    pub async fn create_recipe(
        &self,
        name: &str,
        title: &str,
        source_kind: &str,
        instructions: Option<&str>,
        prompt: Option<&str>,
        definition: &str,
    ) -> Result<Recipe> {
        let id = new_id();
        let now = now_ms();

        sqlx::query(
            r#"
            INSERT INTO recipes (id, name, title, source_kind, instructions, prompt, definition, version, created_at, updated_at)
            VALUES (?, ?, ?, ?, ?, ?, ?, 1, ?, ?)
            "#,
        )
        .bind(&id)
        .bind(name)
        .bind(title)
        .bind(source_kind)
        .bind(instructions)
        .bind(prompt)
        .bind(definition)
        .bind(now)
        .bind(now)
        .execute(&self.pool)
        .await?;

        let recipe = sqlx::query_as::<_, Recipe>("SELECT * FROM recipes WHERE id = ?")
            .bind(&id)
            .fetch_one(&self.pool)
            .await?;
        Ok(recipe)
    }

    pub async fn list_parameters(&self, recipe_id: &str) -> Result<Vec<RecipeParameter>> {
        let params = sqlx::query_as::<_, RecipeParameter>(
            "SELECT * FROM recipe_parameters WHERE recipe_id = ? ORDER BY idx ASC",
        )
        .bind(recipe_id)
        .fetch_all(&self.pool)
        .await?;
        Ok(params)
    }

    pub async fn list_due_schedules(&self, now: i64) -> Result<Vec<Schedule>> {
        let schedules = sqlx::query_as::<_, Schedule>(
            "SELECT * FROM schedules WHERE enabled = 1 AND next_run_at IS NOT NULL AND next_run_at <= ? ORDER BY next_run_at ASC",
        )
        .bind(now)
        .fetch_all(&self.pool)
        .await?;
        Ok(schedules)
    }
}
