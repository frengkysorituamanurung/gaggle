use crate::{
    connection::{new_id, now_ms},
    models::{Model, Provider},
};
use anyhow::{Context, Result};
use sqlx::SqlitePool;

pub struct ProviderRepository {
    pool: SqlitePool,
}

impl ProviderRepository {
    pub fn new(pool: SqlitePool) -> Self {
        Self { pool }
    }

    pub async fn create_provider(
        &self,
        kind: &str,
        name: &str,
        base_url: Option<&str>,
        auth_kind: &str,
        secret_ref: Option<&str>,
    ) -> Result<Provider> {
        let id = new_id();
        let now = now_ms();

        sqlx::query(
            r#"
            INSERT INTO providers (id, kind, name, base_url, auth_kind, secret_ref, enabled, created_at)
            VALUES (?, ?, ?, ?, ?, ?, 1, ?)
            "#,
        )
        .bind(&id)
        .bind(kind)
        .bind(name)
        .bind(base_url)
        .bind(auth_kind)
        .bind(secret_ref)
        .bind(now)
        .execute(&self.pool)
        .await?;

        self.get_provider_by_id(&id)
            .await?
            .context("Provider not found after insert")
    }

    pub async fn get_provider_by_id(&self, id: &str) -> Result<Option<Provider>> {
        let provider = sqlx::query_as::<_, Provider>("SELECT * FROM providers WHERE id = ?")
            .bind(id)
            .fetch_optional(&self.pool)
            .await?;
        Ok(provider)
    }

    pub async fn list_providers(&self) -> Result<Vec<Provider>> {
        let providers =
            sqlx::query_as::<_, Provider>("SELECT * FROM providers ORDER BY created_at DESC")
                .fetch_all(&self.pool)
                .await?;
        Ok(providers)
    }

    #[allow(clippy::too_many_arguments)]
    pub async fn create_model(
        &self,
        provider_id: &str,
        model_key: &str,
        display_name: &str,
        context_window: i64,
        supports_tools: bool,
        supports_vision: bool,
        supports_reasoning: bool,
        price_in_per_mtok: Option<f64>,
        price_out_per_mtok: Option<f64>,
    ) -> Result<Model> {
        let id = new_id();

        sqlx::query(
            r#"
            INSERT INTO models (
                id, provider_id, model_key, display_name, context_window,
                supports_tools, supports_vision, supports_reasoning,
                price_in_per_mtok, price_out_per_mtok, enabled
            ) VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, 1)
            "#,
        )
        .bind(&id)
        .bind(provider_id)
        .bind(model_key)
        .bind(display_name)
        .bind(context_window)
        .bind(if supports_tools { 1 } else { 0 })
        .bind(if supports_vision { 1 } else { 0 })
        .bind(if supports_reasoning { 1 } else { 0 })
        .bind(price_in_per_mtok)
        .bind(price_out_per_mtok)
        .execute(&self.pool)
        .await?;

        let model = sqlx::query_as::<_, Model>("SELECT * FROM models WHERE id = ?")
            .bind(&id)
            .fetch_one(&self.pool)
            .await?;
        Ok(model)
    }

    pub async fn list_models_by_provider(&self, provider_id: &str) -> Result<Vec<Model>> {
        let models = sqlx::query_as::<_, Model>(
            "SELECT * FROM models WHERE provider_id = ? AND enabled = 1 ORDER BY display_name ASC",
        )
        .bind(provider_id)
        .fetch_all(&self.pool)
        .await?;
        Ok(models)
    }
}
