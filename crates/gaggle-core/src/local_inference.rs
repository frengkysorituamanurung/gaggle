//! Local inference support (llama.cpp wrapper).
//!
//! Provides in-process model inference using llama.cpp with optional
//! GPU backends (CUDA, Vulkan, Metal, MLX).

use serde::{Deserialize, Serialize};

/// Local inference configuration.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LocalInferenceConfig {
    pub enabled: bool,
    pub model_path: Option<String>,
    pub gpu_backend: Option<String>,
    pub context_size: usize,
    pub threads: usize,
}

impl Default for LocalInferenceConfig {
    fn default() -> Self {
        Self {
            enabled: false,
            model_path: None,
            gpu_backend: None,
            context_size: 4096,
            threads: 4,
        }
    }
}

/// Local inference service.
pub struct LocalInferenceService {
    config: LocalInferenceConfig,
}

impl LocalInferenceService {
    pub fn new(config: LocalInferenceConfig) -> Self {
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
        assert!(!LocalInferenceConfig::default().enabled);
    }
}
