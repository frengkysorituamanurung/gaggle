//! Explicit agent run state machine.
//!
//! Mirrors the high-level phases of an agent turn so callers can validate
//! transitions and emit structured state events. Gaggle keeps its durable run
//! and checkpoint model as the source of truth; this module only describes the
//! allowed phase transitions.

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum AgentRunState {
    Created,
    Running,
    AwaitingApproval,
    Compacting,
    Streaming,
    Completed,
    Failed,
    Cancelled,
}

impl AgentRunState {
    pub fn can_transition_to(self, next: AgentRunState) -> bool {
        use AgentRunState::*;
        matches!(
            (self, next),
            (Created, Running)
                | (Running, AwaitingApproval)
                | (Running, Compacting)
                | (Running, Streaming)
                | (Running, Completed)
                | (Running, Failed)
                | (Running, Cancelled)
                | (AwaitingApproval, Running)
                | (AwaitingApproval, Cancelled)
                | (AwaitingApproval, Failed)
                | (Compacting, Running)
                | (Compacting, Failed)
                | (Streaming, Running)
                | (Streaming, Completed)
                | (Streaming, Failed)
                | (Streaming, Cancelled)
        )
    }

    pub fn is_terminal(self) -> bool {
        matches!(
            self,
            AgentRunState::Completed | AgentRunState::Failed | AgentRunState::Cancelled
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn allows_expected_transitions() {
        assert!(AgentRunState::Created.can_transition_to(AgentRunState::Running));
        assert!(AgentRunState::Running.can_transition_to(AgentRunState::Completed));
    }

    #[test]
    fn rejects_invalid_transitions() {
        assert!(!AgentRunState::Completed.can_transition_to(AgentRunState::Running));
    }
}
