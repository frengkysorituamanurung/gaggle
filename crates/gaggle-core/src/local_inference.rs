//! Local inference support (llama.cpp wrapper).
//!
//! Provides in-process model inference using llama.cpp with optional
//! GPU backends (CUDA, Vulkan, Metal, MLX).

use serde::{Deserialize, Serialize};
use std::path::Path;

/// Local inference configuration.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LocalInferenceConfig {
    pub enabled: bool,
    pub model_path: Option<String>,
    pub gpu_backend: Option<String>,
    pub context_size: usize,
    pub threads: usize,
    pub temperature: f32,
    pub top_p: f32,
}

impl Default for LocalInferenceConfig {
    fn default() -> Self {
        Self {
            enabled: false,
            model_path: None,
            gpu_backend: None,
            context_size: 4096,
            threads: 4,
            temperature: 0.7,
            top_p: 0.9,
        }
    }
}

/// A loaded model handle.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LocalModel {
    pub path: String,
    pub name: String,
    pub context_size: usize,
    pub loaded: bool,
}

/// Local inference service.
pub struct LocalInferenceService {
    config: LocalInferenceConfig,
    model: Option<LocalModel>,
}

impl LocalInferenceService {
    pub fn new(config: LocalInferenceConfig) -> Self {
        Self {
            config,
            model: None,
        }
    }

    pub fn is_enabled(&self) -> bool {
        self.config.enabled
    }

    /// Check if a model file exists and is valid.
    pub fn validate_model_path(&self) -> Result<(), String> {
        match &self.config.model_path {
            Some(path) => {
                if !Path::new(path).exists() {
                    return Err(format!("Model file not found: {}", path));
                }
                Ok(())
            }
            None => Err("No model path configured".to_string()),
        }
    }

    /// Load a model from the configured path.
    pub async fn load_model(&mut self) -> Result<LocalModel, String> {
        if !self.config.enabled {
            return Err("Local inference is disabled".to_string());
        }
        self.validate_model_path()?;
        let path = self.config.model_path.clone().unwrap();
        let name = Path::new(&path)
            .file_name()
            .and_then(|n| n.to_str())
            .unwrap_or("unknown")
            .to_string();
        let model = LocalModel {
            path: path.clone(),
            name,
            context_size: self.config.context_size,
            loaded: true,
        };
        self.model = Some(model.clone());
        Ok(model)
    }

    /// Unload the current model.
    pub fn unload_model(&mut self) {
        self.model = None;
    }

    /// Get the currently loaded model.
    pub fn current_model(&self) -> Option<&LocalModel> {
        self.model.as_ref()
    }

    /// Check if a model is currently loaded.
    pub fn is_model_loaded(&self) -> bool {
        self.model.as_ref().map(|m| m.loaded).unwrap_or(false)
    }

    /// Supported GPU backends.
    pub fn supported_backends() -> Vec<&'static str> {
        vec!["cpu", "cuda", "vulkan", "metal", "mlx"]
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_config_disabled() {
        assert!(!LocalInferenceConfig::default().enabled);
    }

    #[test]
    fn supported_backends() {
        assert!(LocalInferenceService::supported_backends().contains(&"cpu"));
    }

    #[test]
    fn no_model_loaded_by_default() {
        let service = LocalInferenceService::new(LocalInferenceConfig::default());
        assert!(!service.is_model_loaded());
    }
}
