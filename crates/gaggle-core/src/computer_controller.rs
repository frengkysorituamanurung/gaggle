//! Computer controller for macOS automation.
//!
//! Provides system automation capabilities like clicking, typing,
//! and taking screenshots on macOS.

use serde::{Deserialize, Serialize};

/// Computer controller configuration.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ComputerControllerConfig {
    pub enabled: bool,
    pub allowed_actions: Vec<String>,
}

impl Default for ComputerControllerConfig {
    fn default() -> Self {
        Self {
            enabled: false,
            allowed_actions: vec![
                "screenshot".to_string(),
                "click".to_string(),
                "type".to_string(),
            ],
        }
    }
}

/// Computer controller service.
pub struct ComputerController {
    config: ComputerControllerConfig,
}

impl ComputerController {
    pub fn new(config: ComputerControllerConfig) -> Self {
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
        assert!(!ComputerControllerConfig::default().enabled);
    }
}
