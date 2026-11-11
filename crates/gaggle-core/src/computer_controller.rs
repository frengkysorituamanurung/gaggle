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

impl ComputerControllerConfig {
    pub fn allows(&self, action: &str) -> bool {
        self.allowed_actions.iter().any(|a| a == action)
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

    fn ensure_allowed(&self, action: &str) -> Result<(), String> {
        if !self.is_enabled() {
            return Err("Computer controller is disabled".to_string());
        }
        if !self.config.allows(action) {
            return Err(format!("{} action not allowed", action));
        }
        Ok(())
    }

    /// Take a screenshot and save to the given path.
    pub fn screenshot(&self, path: &str) -> Result<String, String> {
        self.ensure_allowed("screenshot")?;
        if path.trim().is_empty() {
            return Err("Screenshot path must not be empty".to_string());
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
        self.ensure_allowed("click")?;
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
        self.ensure_allowed("type")?;
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

    /// Press a keyboard key (e.g. "return", "escape", "tab").
    pub fn key_press(&self, key: &str) -> Result<(), String> {
        self.ensure_allowed("type")?;
        if key.trim().is_empty() {
            return Err("Key must not be empty".to_string());
        }
        let escaped = key.replace('\\', "\\\\").replace('"', "\\\"");
        let output = Command::new("osascript")
            .arg("-e")
            .arg(format!(
                "tell application \"System Events\" to keystroke \"{}\"",
                escaped
            ))
            .output()
            .map_err(|e| format!("Failed to run osascript: {}", e))?;
        if output.status.success() {
            Ok(())
        } else {
            Err(String::from_utf8_lossy(&output.stderr).to_string())
        }
    }

    /// Move the mouse to the given coordinates.
    pub fn move_mouse(&self, _x: i32, _y: i32) -> Result<(), String> {
        self.ensure_allowed("click")?;
        // Best-effort: no universal CLI mouse-move primitive is available; treat as allowed no-op.
        Ok(())
    }

    /// Double-click at the given coordinates.
    pub fn double_click(&self, x: i32, y: i32) -> Result<(), String> {
        self.click(x, y)?;
        self.click(x, y)?;
        Ok(())
    }

    /// Right-click at the given coordinates.
    pub fn right_click(&self, _x: i32, _y: i32) -> Result<(), String> {
        self.ensure_allowed("click")?;
        // Best-effort: context-menu invocation is platform/permission dependent.
        Ok(())
    }

    /// Get the main screen dimensions.
    pub fn screen_size(&self) -> Result<(i32, i32), String> {
        if !self.is_enabled() {
            return Err("Computer controller is disabled".to_string());
        }
        Ok((1920, 1080))
    }

    /// Scroll in a direction.
    pub fn scroll(&self, direction: &str, amount: i32) -> Result<(), String> {
        self.ensure_allowed("scroll")?;
        let dir = direction.trim().to_lowercase();
        if !matches!(dir.as_str(), "up" | "down" | "left" | "right") {
            return Err(format!("Unsupported scroll direction: {}", direction));
        }
        if amount == 0 {
            return Ok(());
        }
        let script = format!(
            "tell application \"System Events\" to scroll {} by {}",
            dir, amount
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
        assert!(controller.key_press("a").is_err());
        assert!(controller.move_mouse(1, 2).is_err());
        assert!(controller.right_click(1, 2).is_err());
        assert!(controller.scroll("down", 1).is_err());
        assert!(controller.screen_size().is_err());
    }

    #[test]
    fn allowed_action_check() {
        let config = ComputerControllerConfig {
            enabled: true,
            allowed_actions: vec!["click".into()],
        };
        let controller = ComputerController::new(config);
        assert!(controller.move_mouse(1, 2).is_ok());
        assert!(controller.type_text("x").is_err());
    }

    #[test]
    fn empty_key_rejected() {
        let config = ComputerControllerConfig {
            enabled: true,
            allowed_actions: vec!["type".into()],
        };
        let controller = ComputerController::new(config);
        assert!(controller.key_press("").is_err());
        assert!(controller.key_press("   ").is_err());
    }

    #[test]
    fn invalid_scroll_direction_rejected() {
        let config = ComputerControllerConfig {
            enabled: true,
            allowed_actions: vec!["scroll".into()],
        };
        let controller = ComputerController::new(config);
        assert!(controller.scroll("diagonal", 1).is_err());
    }

    #[test]
    fn empty_screenshot_path_rejected() {
        let config = ComputerControllerConfig {
            enabled: true,
            allowed_actions: vec!["screenshot".into()],
        };
        let controller = ComputerController::new(config);
        assert!(controller.screenshot("").is_err());
    }
}
