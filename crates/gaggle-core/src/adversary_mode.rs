//! Adversary mode for testing agent robustness.
//!
//! Adversary mode runs the agent with adversarial prompts to test
//! its ability to handle edge cases, prompt injection, and unsafe requests.

use serde::{Deserialize, Serialize};

/// Adversary test configuration.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AdversaryConfig {
    pub enabled: bool,
    pub test_prompts: Vec<String>,
    pub max_iterations: usize,
}

impl Default for AdversaryConfig {
    fn default() -> Self {
        Self {
            enabled: false,
            test_prompts: Vec::new(),
            max_iterations: 10,
        }
    }
}

/// Adversary mode runner.
pub struct AdversaryRunner {
    config: AdversaryConfig,
}

impl AdversaryRunner {
    pub fn new(config: AdversaryConfig) -> Self {
        Self { config }
    }

    pub fn is_enabled(&self) -> bool {
        self.config.enabled
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_config_disabled() {
        assert!(!AdversaryConfig::default().enabled);
    }
}
