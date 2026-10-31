//! Final output tool for structured agent results.
//!
//! This tool allows the agent to produce a structured final output
//! that is distinct from intermediate tool results.

use serde::{Deserialize, Serialize};

/// A structured final output from the agent.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FinalOutput {
    pub summary: String,
    pub changes: Vec<String>,
    pub tests_run: Vec<String>,
    pub errors: Vec<String>,
}

impl FinalOutput {
    pub fn new(summary: &str) -> Self {
        Self {
            summary: summary.to_string(),
            changes: Vec::new(),
            tests_run: Vec::new(),
            errors: Vec::new(),
        }
    }

    pub fn add_change(&mut self, change: &str) {
        self.changes.push(change.to_string());
    }

    pub fn add_test(&mut self, test: &str) {
        self.tests_run.push(test.to_string());
    }

    pub fn add_error(&mut self, error: &str) {
        self.errors.push(error.to_string());
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn final_output_serializes() {
        let mut output = FinalOutput::new("Task completed");
        output.add_change("Modified file.rs");
        output.add_test("cargo test");
        let json = serde_json::to_string(&output).unwrap();
        assert!(json.contains("Task completed"));
        assert!(json.contains("Modified file.rs"));
    }
}
