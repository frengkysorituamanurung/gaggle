use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CodeModeConfig {
    pub memory_limit_mb: i64,
    pub timeout_ms: i64,
}

impl Default for CodeModeConfig {
    fn default() -> Self {
        Self {
            memory_limit_mb: 128,
            timeout_ms: 30_000,
        }
    }
}
