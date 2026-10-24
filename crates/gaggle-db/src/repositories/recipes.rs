use crate::{
    connection::{new_id, now_ms},
    models::{Recipe, RecipeExtension, RecipeParameter, RecipeSubRecipe, Schedule},
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

    // ---------------------------------------------------------------- recipes

    pub async fn create_recipe(
        &self,
        name: &str,
        title: &str,
        source_kind: &str,
        instructions: Option<&str>,
        prompt: Option<&str>,
        definition: &str,
    ) -> Result<Recipe> {
        self.create_recipe_full(
            name,
            title,
            None,
            source_kind,
            None,
            instructions,
            prompt,
            definition,
            None,
        )
        .await
    }

    #[allow(clippy::too_many_arguments)]
    #[allow(clippy::too_many_arguments)]
    pub async fn create_recipe_full(
        &self,
        name: &str,
        title: &str,
        description: Option<&str>,
        source_kind: &str,
        source_ref: Option<&str>,
        instructions: Option<&str>,
        prompt: Option<&str>,
        definition: &str,
        default_model_id: Option<&str>,
    ) -> Result<Recipe> {
        let id = new_id();
        let now = now_ms();

        sqlx::query(
            r#"
            INSERT INTO recipes (
                id, name, title, description, source_kind, source_ref,
                instructions, prompt, definition, default_model_id, version,
                created_at, updated_at
            ) VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, 1, ?, ?)
            "#,
        )
        .bind(&id)
        .bind(name)
        .bind(title)
        .bind(description)
        .bind(source_kind)
        .bind(source_ref)
        .bind(instructions)
        .bind(prompt)
        .bind(definition)
        .bind(default_model_id)
        .bind(now)
        .bind(now)
        .execute(&self.pool)
        .await?;

        self.get_by_id(&id)
            .await?
            .ok_or_else(|| anyhow::anyhow!("Recipe not found after insert"))
    }

    pub async fn get_by_id(&self, id: &str) -> Result<Option<Recipe>> {
        let row = sqlx::query_as::<_, Recipe>("SELECT * FROM recipes WHERE id = ?")
            .bind(id)
            .fetch_optional(&self.pool)
            .await?;
        Ok(row)
    }

    pub async fn get_by_name(&self, name: &str) -> Result<Option<Recipe>> {
        let row = sqlx::query_as::<_, Recipe>("SELECT * FROM recipes WHERE name = ?")
            .bind(name)
            .fetch_optional(&self.pool)
            .await?;
        Ok(row)
    }

    pub async fn list_all(&self) -> Result<Vec<Recipe>> {
        let rows = sqlx::query_as::<_, Recipe>("SELECT * FROM recipes ORDER BY created_at DESC")
            .fetch_all(&self.pool)
            .await?;
        Ok(rows)
    }

    pub async fn delete_recipe(&self, id: &str) -> Result<bool> {
        let res = sqlx::query("DELETE FROM recipes WHERE id = ?")
            .bind(id)
            .execute(&self.pool)
            .await?;
        Ok(res.rows_affected() > 0)
    }

    // ------------------------------------------------------------- parameters

    #[allow(clippy::too_many_arguments)]
    pub async fn create_parameter(
        &self,
        recipe_id: &str,
        name: &str,
        param_type: &str,
        description: Option<&str>,
        required: i64,
        default_value: Option<&str>,
        idx: i64,
    ) -> Result<RecipeParameter> {
        let id = new_id();
        sqlx::query(
            r#"
            INSERT INTO recipe_parameters (id, recipe_id, name, param_type, description, required, default_value, idx)
            VALUES (?, ?, ?, ?, ?, ?, ?, ?)
            "#,
        )
        .bind(&id)
        .bind(recipe_id)
        .bind(name)
        .bind(param_type)
        .bind(description)
        .bind(required)
        .bind(default_value)
        .bind(idx)
        .execute(&self.pool)
        .await?;

        let row =
            sqlx::query_as::<_, RecipeParameter>("SELECT * FROM recipe_parameters WHERE id = ?")
                .bind(&id)
                .fetch_one(&self.pool)
                .await?;
        Ok(row)
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

    pub async fn get_parameter(
        &self,
        recipe_id: &str,
        name: &str,
    ) -> Result<Option<RecipeParameter>> {
        let row = sqlx::query_as::<_, RecipeParameter>(
            "SELECT * FROM recipe_parameters WHERE recipe_id = ? AND name = ?",
        )
        .bind(recipe_id)
        .bind(name)
        .fetch_optional(&self.pool)
        .await?;
        Ok(row)
    }

    pub async fn delete_parameters(&self, recipe_id: &str) -> Result<()> {
        sqlx::query("DELETE FROM recipe_parameters WHERE recipe_id = ?")
            .bind(recipe_id)
            .execute(&self.pool)
            .await?;
        Ok(())
    }

    // ------------------------------------------------------------- sub-recipes

    pub async fn create_sub_recipe(
        &self,
        recipe_id: &str,
        child_recipe_id: &str,
        name: &str,
        values_json: Option<&str>,
        sequential_when_repeated: i64,
        idx: i64,
    ) -> Result<RecipeSubRecipe> {
        let id = new_id();
        sqlx::query(
            r#"
            INSERT INTO recipe_sub_recipes (id, recipe_id, child_recipe_id, name, values_json, sequential_when_repeated, idx)
            VALUES (?, ?, ?, ?, ?, ?, ?)
            "#,
        )
        .bind(&id)
        .bind(recipe_id)
        .bind(child_recipe_id)
        .bind(name)
        .bind(values_json)
        .bind(sequential_when_repeated)
        .bind(idx)
        .execute(&self.pool)
        .await?;

        let row =
            sqlx::query_as::<_, RecipeSubRecipe>("SELECT * FROM recipe_sub_recipes WHERE id = ?")
                .bind(&id)
                .fetch_one(&self.pool)
                .await?;
        Ok(row)
    }

    pub async fn list_sub_recipes(&self, recipe_id: &str) -> Result<Vec<RecipeSubRecipe>> {
        let rows = sqlx::query_as::<_, RecipeSubRecipe>(
            "SELECT * FROM recipe_sub_recipes WHERE recipe_id = ? ORDER BY idx ASC",
        )
        .bind(recipe_id)
        .fetch_all(&self.pool)
        .await?;
        Ok(rows)
    }

    // ------------------------------------------------------------- extensions

    pub async fn add_extension(&self, recipe_id: &str, extension_id: &str) -> Result<()> {
        sqlx::query(
            "INSERT OR IGNORE INTO recipe_extensions (recipe_id, extension_id) VALUES (?, ?)",
        )
        .bind(recipe_id)
        .bind(extension_id)
        .execute(&self.pool)
        .await?;
        Ok(())
    }

    pub async fn remove_extension(&self, recipe_id: &str, extension_id: &str) -> Result<()> {
        sqlx::query("DELETE FROM recipe_extensions WHERE recipe_id = ? AND extension_id = ?")
            .bind(recipe_id)
            .bind(extension_id)
            .execute(&self.pool)
            .await?;
        Ok(())
    }

    pub async fn list_recipe_extensions(&self, recipe_id: &str) -> Result<Vec<RecipeExtension>> {
        let rows = sqlx::query_as::<_, RecipeExtension>(
            "SELECT * FROM recipe_extensions WHERE recipe_id = ?",
        )
        .bind(recipe_id)
        .fetch_all(&self.pool)
        .await?;
        Ok(rows)
    }

    pub async fn list_extension_ids(&self, recipe_id: &str) -> Result<Vec<String>> {
        let rows: Vec<String> =
            sqlx::query_scalar("SELECT extension_id FROM recipe_extensions WHERE recipe_id = ?")
                .bind(recipe_id)
                .fetch_all(&self.pool)
                .await?;
        Ok(rows)
    }

    // ---------------------------------------------------------------- schedules

    pub async fn create_schedule(
        &self,
        recipe_id: &str,
        project_id: Option<&str>,
        cron_expr: &str,
        timezone: &str,
        param_values: Option<&str>,
    ) -> Result<Schedule> {
        self.create_schedule_with_options(
            recipe_id,
            project_id,
            cron_expr,
            timezone,
            param_values,
            1,
            None,
        )
        .await
    }

    #[allow(clippy::too_many_arguments)]
    pub async fn create_schedule_with_options(
        &self,
        recipe_id: &str,
        project_id: Option<&str>,
        cron_expr: &str,
        timezone: &str,
        param_values: Option<&str>,
        enabled: i64,
        next_run_at: Option<i64>,
    ) -> Result<Schedule> {
        let id = new_id();
        let now = now_ms();
        sqlx::query(
            r#"
            INSERT INTO schedules (id, recipe_id, project_id, cron_expr, timezone, param_values, enabled, next_run_at, created_at)
            VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?)
            "#,
        )
        .bind(&id)
        .bind(recipe_id)
        .bind(project_id)
        .bind(cron_expr)
        .bind(timezone)
        .bind(param_values)
        .bind(enabled)
        .bind(next_run_at)
        .bind(now)
        .execute(&self.pool)
        .await?;

        self.get_schedule(&id)
            .await?
            .ok_or_else(|| anyhow::anyhow!("Schedule not found after insert"))
    }

    pub async fn get_schedule(&self, id: &str) -> Result<Option<Schedule>> {
        let row = sqlx::query_as::<_, Schedule>("SELECT * FROM schedules WHERE id = ?")
            .bind(id)
            .fetch_optional(&self.pool)
            .await?;
        Ok(row)
    }

    pub async fn list_schedules(&self, project_id: Option<&str>) -> Result<Vec<Schedule>> {
        let rows = if let Some(pid) = project_id {
            sqlx::query_as::<_, Schedule>(
                "SELECT * FROM schedules WHERE project_id = ? ORDER BY created_at DESC",
            )
            .bind(pid)
            .fetch_all(&self.pool)
            .await?
        } else {
            sqlx::query_as::<_, Schedule>("SELECT * FROM schedules ORDER BY created_at DESC")
                .fetch_all(&self.pool)
                .await?
        };
        Ok(rows)
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

    pub async fn update_schedule_next_run(&self, id: &str, next_run_at: Option<i64>) -> Result<()> {
        sqlx::query("UPDATE schedules SET next_run_at = ? WHERE id = ?")
            .bind(next_run_at)
            .bind(id)
            .execute(&self.pool)
            .await?;
        Ok(())
    }

    pub async fn update_schedule_status(
        &self,
        id: &str,
        last_run_at: Option<i64>,
        last_status: Option<&str>,
        next_run_at: Option<i64>,
    ) -> Result<()> {
        sqlx::query(
            "UPDATE schedules SET last_run_at = ?, last_status = ?, next_run_at = ? WHERE id = ?",
        )
        .bind(last_run_at)
        .bind(last_status)
        .bind(next_run_at)
        .bind(id)
        .execute(&self.pool)
        .await?;
        Ok(())
    }

    pub async fn set_schedule_enabled(&self, id: &str, enabled: bool) -> Result<()> {
        sqlx::query("UPDATE schedules SET enabled = ? WHERE id = ?")
            .bind(if enabled { 1 } else { 0 })
            .bind(id)
            .execute(&self.pool)
            .await?;
        Ok(())
    }

    pub async fn delete_schedule(&self, id: &str) -> Result<bool> {
        let res = sqlx::query("DELETE FROM schedules WHERE id = ?")
            .bind(id)
            .execute(&self.pool)
            .await?;
        Ok(res.rows_affected() > 0)
    }

    // convenience aliases for schedule creation naming
    #[allow(clippy::too_many_arguments)]
    pub async fn insert_schedule(
        &self,
        recipe_id: &str,
        project_id: Option<&str>,
        cron_expr: &str,
        timezone: &str,
        param_values: Option<&str>,
        enabled: i64,
        next_run_at: Option<i64>,
    ) -> Result<Schedule> {
        self.create_schedule_with_options(
            recipe_id,
            project_id,
            cron_expr,
            timezone,
            param_values,
            enabled,
            next_run_at,
        )
        .await
    }
}

// Alias for callers using a schedule-focused repository name.
pub type ScheduleRepository = RecipeRepository;
