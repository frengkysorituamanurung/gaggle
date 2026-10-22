use anyhow::Result;
use gaggle_db::{models::Run, repositories::runs::RunRepository};
use sqlx::SqlitePool;
use tracing::info;

pub struct RunEngine {
    pool: SqlitePool,
}

impl RunEngine {
    pub fn new(pool: SqlitePool) -> Self {
        Self { pool }
    }

    /// Recovers durable runs that were in `running` or `waiting_approval` state prior to restart,
    /// incrementing `resume_count` on each recovered run.
    pub async fn recover_unfinished_runs(&self) -> Result<Vec<Run>> {
        let repo = RunRepository::new(self.pool.clone());
        let recoverable = repo.list_recoverable_runs().await?;

        for run in &recoverable {
            repo.increment_resume_count(&run.id).await?;
            repo.emit_event(
                &run.id,
                "run.recovered",
                &serde_json::json!({
                    "previous_status": run.status,
                    "new_resume_count": run.resume_count + 1
                })
                .to_string(),
            )
            .await?;
            info!(run_id = %run.id, "Recovered durable run from checkpoint");
        }

        Ok(recoverable)
    }
}
