//! Computer controller for macOS automation.
//!
//! Provides system automation capabilities like clicking, typing,
//! and taking screenshots on macOS using osascript and screencapture.

use serde::{Deserialize, Serialize};
use std::process::Command;

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
                "scroll".to_string(),
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

    /// Take a screenshot and save to the given path.
    pub fn screenshot(&self, path: &str) -> Result<String, String> {
        if !self.is_enabled()
            || !self
                .config
                .allowed_actions
                .contains(&"screenshot".to_string())
        {
            return Err("Screenshot action not allowed".to_string());
        }
        let output = Command::new("screencapture")
            .arg("-x")
            .arg(path)
            .output()
            .map_err(|e| format!("Failed to run screencapture: {}", e))?;
        if output.status.success() {
            Ok(path.to_string())
        } else {
            Err(String::from_utf8_lossy(&output.stderr).to_string())
        }
    }

    /// Click at the given coordinates.
    pub fn click(&self, x: i32, y: i32) -> Result<(), String> {
        if !self.is_enabled() || !self.config.allowed_actions.contains(&"click".to_string()) {
            return Err("Click action not allowed".to_string());
        }
        let script = format!(
            "tell application \"System Events\" to click at {{{}, {}}}",
            x, y
        );
        let output = Command::new("osascript")
            .arg("-e")
            .arg(&script)
            .output()
            .map_err(|e| format!("Failed to run osascript: {}", e))?;
        if output.status.success() {
            Ok(())
        } else {
            Err(String::from_utf8_lossy(&output.stderr).to_string())
        }
    }

    /// Type text using keyboard simulation.
    pub fn type_text(&self, text: &str) -> Result<(), String> {
        if !self.is_enabled() || !self.config.allowed_actions.contains(&"type".to_string()) {
            return Err("Type action not allowed".to_string());
        }
        let escaped = text.replace('\\', "\\\\").replace('"', "\\\"");
        let script = format!(
            "tell application \"System Events\" to keystroke \"{}\"",
            escaped
        );
        let output = Command::new("osascript")
            .arg("-e")
            .arg(&script)
            .output()
            .map_err(|e| format!("Failed to run osascript: {}", e))?;
        if output.status.success() {
            Ok(())
        } else {
            Err(String::from_utf8_lossy(&output.stderr).to_string())
        }
    }

    /// Scroll in a direction.
    pub fn scroll(&self, direction: &str, amount: i32) -> Result<(), String> {
        if !self.is_enabled() || !self.config.allowed_actions.contains(&"scroll".to_string()) {
            return Err("Scroll action not allowed".to_string());
        }
        let script = format!(
            "tell application \"System Events\" to scroll {} by {}",
            direction, amount
        );
        let output = Command::new("osascript")
            .arg("-e")
            .arg(&script)
            .output()
            .map_err(|e| format!("Failed to run osascript: {}", e))?;
        if output.status.success() {
            Ok(())
        } else {
            Err(String::from_utf8_lossy(&output.stderr).to_string())
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_config_disabled() {
        assert!(!ComputerControllerConfig::default().enabled);
    }

    #[test]
    fn disabled_controller_rejects_actions() {
        let controller = ComputerController::new(ComputerControllerConfig::default());
        assert!(controller.screenshot("/tmp/test.png").is_err());
        assert!(controller.click(100, 100).is_err());
        assert!(controller.type_text("hello").is_err());
    }
}
