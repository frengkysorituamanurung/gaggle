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
        match (self, next) {
            (Created, Running) => true,
            (Running, AwaitingApproval) => true,
            (Running, Compacting) => true,
            (Running, Streaming) => true,
            (Running, Completed) => true,
            (Running, Failed) => true,
            (Running, Cancelled) => true,
            (AwaitingApproval, Running) => true,
            (AwaitingApproval, Cancelled) => true,
            (AwaitingApproval, Failed) => true,
            (Compacting, Running) => true,
            (Compacting, Failed) => true,
            (Streaming, Running) => true,
            (Streaming, Completed) => true,
            (Streaming, Failed) => true,
            (Streaming, Cancelled) => true,
            _ => false,
        }
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
