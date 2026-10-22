use crate::{connection::now_ms, models::AppSetting};
use anyhow::{Context, Result};
use sqlx::SqlitePool;

pub struct SettingsRepository {
    pool: SqlitePool,
}

impl SettingsRepository {
    pub fn new(pool: SqlitePool) -> Self {
        Self { pool }
    }

    pub async fn get(&self, key: &str) -> Result<Option<AppSetting>> {
        let setting = sqlx::query_as::<_, AppSetting>("SELECT * FROM app_settings WHERE key = ?")
            .bind(key)
            .fetch_optional(&self.pool)
            .await?;
        Ok(setting)
    }

    pub async fn list_all(&self) -> Result<Vec<AppSetting>> {
        let settings =
            sqlx::query_as::<_, AppSetting>("SELECT * FROM app_settings ORDER BY key ASC")
                .fetch_all(&self.pool)
                .await?;
        Ok(settings)
    }

    pub async fn upsert(&self, key: &str, value_json: &str) -> Result<AppSetting> {
        let now = now_ms();

        sqlx::query(
            r#"
            INSERT INTO app_settings (key, value, updated_at)
            VALUES (?, ?, ?)
            ON CONFLICT(key) DO UPDATE SET
                value = excluded.value,
                updated_at = excluded.updated_at
            "#,
        )
        .bind(key)
        .bind(value_json)
        .bind(now)
        .execute(&self.pool)
        .await?;

        self.get(key)
            .await?
            .with_context(|| format!("AppSetting '{key}' not found after upsert"))
    }
}
