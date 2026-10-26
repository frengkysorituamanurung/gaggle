//! Operation pipeline foundation (Phase 0).
//!
//! Provides a small, composable abstraction over discrete agent-loop steps. The
//! existing `AgentRuntime::run_turn` remains the public facade; operations are
//! introduced incrementally so durable checkpointing and event persistence stay
//! the source of truth.

use anyhow::Result;
use async_trait::async_trait;
use serde_json::Value;

/// Outcome of a single operation.
#[derive(Debug, Clone)]
pub enum OperationOutcome {
    Continue,
    WaitingApproval,
    Completed { final_text: String },
    Cancelled,
    Failed { error: String },
}

/// Shared context passed to every operation.
#[derive(Clone)]
pub struct OperationContext {
    pub run_id: String,
    pub chat_id: String,
    pub project_id: String,
    pub step_index: i64,
    pub extra: Value,
}

#[async_trait]
pub trait Operation: Send + Sync {
    fn name(&self) -> &'static str;
    async fn run(&mut self, ctx: &OperationContext) -> Result<OperationOutcome>;
}

/// Sequential runner for a list of operations. Stops early on terminal outcomes.
pub struct OperationRunner {
    operations: Vec<Box<dyn Operation>>,
}

impl OperationRunner {
    pub fn new(operations: Vec<Box<dyn Operation>>) -> Self {
        Self { operations }
    }

    pub async fn run(&mut self, ctx: &OperationContext) -> Result<OperationOutcome> {
        for op in self.operations.iter_mut() {
            match op.run(ctx).await? {
                OperationOutcome::Continue => continue,
                terminal => return Ok(terminal),
            }
        }
        Ok(OperationOutcome::Continue)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    struct NoopOp;
    #[async_trait]
    impl Operation for NoopOp {
        fn name(&self) -> &'static str {
            "noop"
        }
        async fn run(&mut self, _ctx: &OperationContext) -> Result<OperationOutcome> {
            Ok(OperationOutcome::Continue)
        }
    }

    #[tokio::test]
    async fn runner_completes_noops() {
        let mut runner = OperationRunner::new(vec![Box::new(NoopOp), Box::new(NoopOp)]);
        let ctx = OperationContext {
            run_id: "r".to_string(),
            chat_id: "c".to_string(),
            project_id: "p".to_string(),
            step_index: 1,
            extra: Value::Null,
        };
        let out = runner.run(&ctx).await.unwrap();
        assert!(matches!(out, OperationOutcome::Continue));
    }
}
