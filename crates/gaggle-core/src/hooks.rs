//! Lightweight hook manager foundation (Phase 0).
//!
//! Mirrors Goose's `hooks` module at a minimal level. Hooks are advisory/auditable
//! callbacks that can observe or block tool execution. They never replace backend
//! policy enforcement.

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum HookEvent {
    SessionStart,
    SessionEnd,
    UserPromptSubmit,
    PreToolUse,
    PostToolUse,
    PostToolUseFailure,
    Stop,
    BeforeReadFile,
    AfterFileEdit,
    BeforeShellExecution,
    AfterShellExecution,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum HookDecision {
    Allow,
    Deny,
    Modify,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HookContext {
    pub event: HookEvent,
    pub run_id: Option<String>,
    pub chat_id: Option<String>,
    pub tool_name: Option<String>,
    pub tool_call_id: Option<String>,
    pub payload: serde_json::Value,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HookResult {
    pub decision: HookDecision,
    pub message: Option<String>,
    pub modified_payload: Option<serde_json::Value>,
}

impl HookResult {
    pub fn allow() -> Self {
        Self {
            decision: HookDecision::Allow,
            message: None,
            modified_payload: None,
        }
    }

    pub fn deny(message: impl Into<String>) -> Self {
        Self {
            decision: HookDecision::Deny,
            message: Some(message.into()),
            modified_payload: None,
        }
    }
}

use std::sync::{Arc, RwLock};

/// A registered hook callback.
pub type HookCallback = Arc<dyn Fn(&HookContext) -> HookResult + Send + Sync>;

/// In-process hook manager. Hooks are registered as callbacks; the default
/// implementation is a no-op that allows everything.
#[derive(Clone, Default)]
pub struct HookManager {
    callbacks: Arc<RwLock<Vec<(HookEvent, HookCallback)>>>,
}

impl HookManager {
    pub fn new() -> Self {
        Self::default()
    }

    /// Register a hook callback for a specific event.
    pub fn register(&self, event: HookEvent, callback: HookCallback) {
        if let Ok(mut guard) = self.callbacks.write() {
            guard.push((event, callback));
        }
    }

    /// Emit a hook event and collect results from all registered callbacks.
    pub fn emit(&self, ctx: &HookContext) -> HookResult {
        let callbacks = self.callbacks.read().ok();
        if let Some(guard) = callbacks {
            for (event, callback) in guard.iter() {
                if *event == ctx.event {
                    let result = callback(ctx);
                    if result.decision == HookDecision::Deny {
                        return result;
                    }
                }
            }
        }
        HookResult::allow()
    }

    pub fn emit_blocking(&self, ctx: &HookContext) -> HookResult {
        self.emit(ctx)
    }

    /// Get the number of registered hooks.
    pub fn hook_count(&self) -> usize {
        self.callbacks.read().map(|g| g.len()).unwrap_or(0)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_hooks_allow() {
        let mgr = HookManager::new();
        let ctx = HookContext {
            event: HookEvent::PreToolUse,
            run_id: None,
            chat_id: None,
            tool_name: Some("fs.read".to_string()),
            tool_call_id: None,
            payload: serde_json::json!({}),
        };
        assert_eq!(mgr.emit(&ctx).decision, HookDecision::Allow);
    }
}
