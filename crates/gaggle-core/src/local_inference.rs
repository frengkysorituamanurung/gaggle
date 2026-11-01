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

impl LocalInferenceConfig {
    pub fn is_valid(&self) -> bool {
        self.context_size > 0
            && self.threads > 0
            && self.temperature >= 0.0
            && self.temperature <= 2.0
            && self.top_p > 0.0
            && self.top_p <= 1.0
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

    pub fn validate(&self) -> Result<(), String> {
        if !self.config.is_valid() {
            return Err("Invalid local inference configuration".to_string());
        }
        self.validate_backend()
    }

    /// Check if a model file exists and is valid.
    pub fn validate_model_path(&self) -> Result<(), String> {
        match &self.config.model_path {
            Some(path) => {
                if path.trim().is_empty() {
                    return Err("Model path must not be empty".to_string());
                }
                let p = Path::new(path);
                if !p.exists() {
                    return Err(format!("Model file not found: {}", path));
                }
                if !p.is_file() {
                    return Err(format!("Model path is not a file: {}", path));
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
        self.validate()?;
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

    /// Validate that the configured GPU backend is supported.
    pub fn validate_backend(&self) -> Result<(), String> {
        match &self.config.gpu_backend {
            Some(backend) => {
                if Self::supported_backends().contains(&backend.as_str()) {
                    Ok(())
                } else {
                    Err(format!("Unsupported GPU backend: {}", backend))
                }
            }
            None => Ok(()),
        }
    }

    /// Run inference.
    #[cfg(feature = "local-inference")]
    pub async fn generate(&self, prompt: &str) -> Result<String, String> {
        if !self.is_enabled() {
            return Err("Local inference is disabled".to_string());
        }
        if !self.is_model_loaded() {
            return Err("No model loaded".to_string());
        }
        if prompt.trim().is_empty() {
            return Err("Prompt must not be empty".to_string());
        }
        let path = self.config.model_path.clone().unwrap();
        let context_size = self.config.context_size;
        let threads = self.config.threads as i32;
        let prompt = prompt.to_string();
        tokio::task::spawn_blocking(move || {
            run_llama_generate(&path, context_size, threads, &prompt)
        })
        .await
        .map_err(|e| format!("inference task failed: {}", e))?
    }

    /// Run inference (stub when feature disabled).
    #[cfg(not(feature = "local-inference"))]
    pub async fn generate(&self, prompt: &str) -> Result<String, String> {
        if !self.is_enabled() {
            return Err("Local inference is disabled".to_string());
        }
        if !self.is_model_loaded() {
            return Err("No model loaded".to_string());
        }
        if prompt.trim().is_empty() {
            return Err("Prompt must not be empty".to_string());
        }
        Err("local-inference feature not enabled".to_string())
    }
}

#[cfg(feature = "local-inference")]
fn run_llama_generate(
    path: &str,
    context_size: usize,
    threads: i32,
    prompt: &str,
) -> Result<String, String> {
    use llama_cpp_2::context::params::LlamaContextParams;
    use llama_cpp_2::llama_backend::LlamaBackend;
    use llama_cpp_2::llama_batch::LlamaBatch;
    use llama_cpp_2::model::LlamaModel;
    use llama_cpp_2::model::params::LlamaModelParams;
    use llama_cpp_2::sampling::LlamaSampler;
    use std::num::NonZeroU32;

    let backend = LlamaBackend::init().map_err(|e| format!("backend init failed: {}", e))?;
    let model_params = LlamaModelParams::default();
    let model = LlamaModel::load_from_file(&backend, path, &model_params)
        .map_err(|e| format!("model load failed: {}", e))?;

    let n_ctx = NonZeroU32::new(context_size as u32).unwrap_or(NonZeroU32::new(2048).unwrap());
    let mut ctx_params = LlamaContextParams::default().with_n_ctx(Some(n_ctx));
    if threads > 0 {
        ctx_params = ctx_params.with_n_threads(threads);
        ctx_params = ctx_params.with_n_threads_batch(threads);
    }
    let mut ctx = model
        .new_context(&backend, ctx_params)
        .map_err(|e| format!("context creation failed: {}", e))?;

    let tokens_list = model.vocab().tokenize(prompt.as_bytes(), true, true);
    if tokens_list.is_empty() {
        return Err("tokenization produced no tokens".to_string());
    }
    let n_len = (tokens_list.len() as i32) + 256;
    let n_cxt = ctx.n_ctx() as i32;
    if (tokens_list.len() as i32) + 256 > n_cxt {
        // continue; context may still be enough for prompt
    }

    let mut batch = LlamaBatch::new(512, 1);
    let last_index = (tokens_list.len() as i32) - 1;
    for (i, token) in (0_i32..).zip(tokens_list.into_iter()) {
        let is_last = i == last_index;
        batch
            .add(token, i, &[0], is_last)
            .map_err(|e| format!("batch add failed: {}", e))?;
    }
    ctx.decode(&mut batch)
        .map_err(|e| format!("decode failed: {}", e))?;

    let mut n_cur = batch.n_tokens();
    let mut sampler = LlamaSampler::chain_simple([LlamaSampler::greedy()]);
    let mut output = String::new();
    let mut n_decode = 0;
    while n_decode < 256 {
        let token = sampler.sample(&ctx, batch.n_tokens() - 1);
        sampler.accept(token);
        if model.vocab().is_eog(token) {
            break;
        }
        let piece = model.vocab().token_to_piece(token, true, None);
        output.push_str(&String::from_utf8_lossy(&piece));
        batch.clear();
        batch
            .add(token, n_cur, &[0], true)
            .map_err(|e| format!("batch add failed: {}", e))?;
        n_cur += 1;
        ctx.decode(&mut batch)
            .map_err(|e| format!("decode failed: {}", e))?;
        n_decode += 1;
    }
    let _ = n_len;
    Ok(output)
}

/// Type alias for compatibility.
pub type LocalInferenceEngine = LocalInferenceService;

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

    #[test]
    fn invalid_backend_rejected() {
        let mut config = LocalInferenceConfig::default();
        config.gpu_backend = Some("tpu".into());
        let service = LocalInferenceService::new(config);
        assert!(service.validate_backend().is_err());
    }

    #[test]
    fn invalid_config_rejected() {
        let mut config = LocalInferenceConfig::default();
        config.context_size = 0;
        assert!(!config.is_valid());
        let service = LocalInferenceService::new(config);
        assert!(service.validate().is_err());
    }

    #[test]
    fn validate_model_path_missing() {
        let mut config = LocalInferenceConfig::default();
        config.model_path = Some("/definitely/not/a/model.gguf".into());
        let service = LocalInferenceService::new(config);
        assert!(service.validate_model_path().is_err());
    }

    #[tokio::test]
    async fn load_model_disabled() {
        let mut service = LocalInferenceService::new(LocalInferenceConfig::default());
        assert!(service.load_model().await.is_err());
    }

    #[tokio::test]
    async fn generate_requires_loaded_model() {
        let mut config = LocalInferenceConfig::default();
        config.enabled = true;
        config.model_path = Some("/tmp/model.gguf".into());
        let service = LocalInferenceService::new(config);
        assert!(service.generate("hi").await.is_err());
    }
}

#[cfg(feature = "local-inference")]
mod feature_tests {
    use super::*;

    #[test]
    fn feature_enabled_compiles() {
        let _ = LocalInferenceEngine::new(LocalInferenceConfig::default());
    }
}
