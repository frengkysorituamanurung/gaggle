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

    /// Recovers durable runs that were in `running` or `waiting_approval` state prior to restart.
    /// Transitions them safely to `paused`, expires stale pending approvals, and marks uncertain
    /// in-flight tool calls as interrupted rather than re-running non-idempotent side effects.
    pub async fn recover_unfinished_runs(&self) -> Result<Vec<Run>> {
        let repo = RunRepository::new(self.pool.clone());
        let recoverable = repo.list_recoverable_runs().await?;
        let mut recovered_runs = Vec::with_capacity(recoverable.len());

        for run in &recoverable {
            let updated = repo.recover_interrupted_run(&run.id).await?;
            info!(
                run_id = %updated.id,
                previous_status = %run.status,
                new_status = %updated.status,
                resume_count = updated.resume_count,
                "Recovered interrupted durable run safely to paused state"
            );
            recovered_runs.push(updated);
        }

        Ok(recovered_runs)
    }
}
