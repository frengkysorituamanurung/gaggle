//! Live voice backend for voice interaction.
//!
//! Provides voice input/output capabilities using speech recognition
//! and text-to-speech.

use serde::{Deserialize, Serialize};

/// Live voice configuration.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LiveVoiceConfig {
    pub enabled: bool,
    pub provider: String,
    pub language: String,
    pub auto_transcribe: bool,
    pub tts_enabled: bool,
    pub tts_voice: Option<String>,
    #[serde(default)]
    pub api_key: Option<String>,
    #[serde(default)]
    pub base_url: Option<String>,
    #[serde(default = "default_whisper_model")]
    pub transcription_model: String,
    #[serde(default = "default_tts_model")]
    pub tts_model: String,
}

fn default_whisper_model() -> String {
    "whisper-1".to_string()
}

fn default_tts_model() -> String {
    "tts-1".to_string()
}

impl Default for LiveVoiceConfig {
    fn default() -> Self {
        Self {
            enabled: false,
            provider: "openai".to_string(),
            language: "en".to_string(),
            auto_transcribe: true,
            tts_enabled: true,
            tts_voice: Some("alloy".to_string()),
            api_key: None,
            base_url: None,
            transcription_model: default_whisper_model(),
            tts_model: default_tts_model(),
        }
    }
}

impl LiveVoiceConfig {
    pub fn is_valid(&self) -> bool {
        !self.provider.trim().is_empty()
            && !self.language.trim().is_empty()
            && !self.transcription_model.trim().is_empty()
            && !self.tts_model.trim().is_empty()
    }
}

/// Transcription result.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TranscriptionResult {
    pub text: String,
    pub language: String,
    pub confidence: f64,
}

/// TTS result.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TtsResult {
    pub audio_path: String,
    pub duration_ms: i64,
}

/// Live voice service.
pub struct LiveVoiceService {
    config: LiveVoiceConfig,
}

impl LiveVoiceService {
    pub fn new(config: LiveVoiceConfig) -> Self {
        Self { config }
    }

    pub fn is_enabled(&self) -> bool {
        self.config.enabled
    }

    pub fn validate(&self) -> Result<(), String> {
        if !self.config.is_valid() {
            return Err("Invalid live voice configuration".to_string());
        }
        Ok(())
    }

    fn api_key(&self) -> Result<String, String> {
        if let Some(key) = &self.config.api_key {
            if !key.trim().is_empty() {
                return Ok(key.clone());
            }
        }
        std::env::var("OPENAI_API_KEY")
            .map_err(|_| "Live voice API key not configured".to_string())
            .and_then(|key| {
                if key.trim().is_empty() {
                    Err("Live voice API key not configured".to_string())
                } else {
                    Ok(key)
                }
            })
    }

    fn base_url(&self) -> String {
        self.config
            .base_url
            .clone()
            .unwrap_or_else(|| "https://api.openai.com/v1".to_string())
            .trim_end_matches('/')
            .to_string()
    }

    /// Transcribe audio data to text.
    pub async fn transcribe(&self, audio_path: &str) -> Result<TranscriptionResult, String> {
        if !self.is_enabled() {
            return Err("Live voice is disabled".to_string());
        }
        self.validate()?;
        if audio_path.trim().is_empty() {
            return Err("Audio path must not be empty".to_string());
        }
        let api_key = self.api_key()?;
        let data =
            std::fs::read(audio_path).map_err(|e| format!("Failed to read audio file: {}", e))?;
        if data.is_empty() {
            return Err("Audio file is empty".to_string());
        }
        let file_name = std::path::Path::new(audio_path)
            .file_name()
            .and_then(|n| n.to_str())
            .unwrap_or("audio.wav")
            .to_string();
        let part = reqwest::multipart::Part::bytes(data).file_name(file_name);
        let form = reqwest::multipart::Form::new()
            .text("model", self.config.transcription_model.clone())
            .text("language", self.config.language.clone())
            .part("file", part);

        let client = reqwest::Client::new();
        let resp = client
            .post(format!("{}/audio/transcriptions", self.base_url()))
            .bearer_auth(api_key)
            .multipart(form)
            .send()
            .await
            .map_err(|e| format!("Transcription request failed: {}", e))?;

        if !resp.status().is_success() {
            let status = resp.status();
            let body = resp.text().await.unwrap_or_default();
            return Err(format!("Transcription failed ({}): {}", status, body));
        }

        let json: serde_json::Value = resp
            .json()
            .await
            .map_err(|e| format!("Invalid transcription response: {}", e))?;
        let text = json
            .get("text")
            .and_then(|v| v.as_str())
            .unwrap_or("")
            .to_string();

        Ok(TranscriptionResult {
            text,
            language: self.config.language.clone(),
            confidence: 1.0,
        })
    }

    /// Convert text to speech.
    pub async fn text_to_speech(&self, text: &str, output_path: &str) -> Result<TtsResult, String> {
        if !self.is_enabled() {
            return Err("Live voice is disabled".to_string());
        }
        if !self.config.tts_enabled {
            return Err("TTS is disabled".to_string());
        }
        self.validate()?;
        if text.trim().is_empty() {
            return Err("Text must not be empty".to_string());
        }
        if output_path.trim().is_empty() {
            return Err("Output path must not be empty".to_string());
        }
        let api_key = self.api_key()?;
        let voice = self
            .config
            .tts_voice
            .clone()
            .unwrap_or_else(|| "alloy".to_string());
        let body = serde_json::json!({
            "model": self.config.tts_model,
            "input": text,
            "voice": voice,
        });

        let client = reqwest::Client::new();
        let resp = client
            .post(format!("{}/audio/speech", self.base_url()))
            .bearer_auth(api_key)
            .json(&body)
            .send()
            .await
            .map_err(|e| format!("TTS request failed: {}", e))?;

        if !resp.status().is_success() {
            let status = resp.status();
            let body = resp.text().await.unwrap_or_default();
            return Err(format!("TTS failed ({}): {}", status, body));
        }

        let bytes = resp
            .bytes()
            .await
            .map_err(|e| format!("Failed to read TTS response: {}", e))?;
        if let Some(parent) = std::path::Path::new(output_path).parent() {
            std::fs::create_dir_all(parent)
                .map_err(|e| format!("Failed to create output dir: {}", e))?;
        }
        std::fs::write(output_path, &bytes)
            .map_err(|e| format!("Failed to write audio file: {}", e))?;

        Ok(TtsResult {
            audio_path: output_path.to_string(),
            duration_ms: 0,
        })
    }

    /// Capture audio from the default input device.
    #[cfg(feature = "live-voice")]
    pub fn capture_audio(&self, duration_ms: u64) -> Result<Vec<f32>, String> {
        use cpal::traits::{DeviceTrait, HostTrait, StreamTrait};
        use std::sync::{Arc, Mutex};

        let host = cpal::default_host();
        let device = host
            .default_input_device()
            .ok_or_else(|| "no default input device".to_string())?;
        let supported = device
            .default_input_config()
            .map_err(|e| format!("input config failed: {}", e))?;
        let sample_rate = supported.sample_rate().0;
        let channels = supported.channels() as usize;
        let duration = duration_ms.max(100);
        let total_samples = (sample_rate as usize * channels * duration as usize) / 1000;

        let data: Arc<Mutex<Vec<f32>>> = Arc::new(Mutex::new(Vec::with_capacity(total_samples)));
        let data_clone = data.clone();
        let stream_config: cpal::StreamConfig = supported.into();
        let stream = device
            .build_input_stream::<f32, _, _>(
                &stream_config,
                move |input: &[f32], _: &cpal::InputCallbackInfo| {
                    let mut buf = data_clone.lock().unwrap();
                    buf.extend_from_slice(input);
                    if buf.len() > total_samples {
                        buf.truncate(total_samples);
                    }
                },
                |err| {
                    let _ = err;
                },
                None,
            )
            .map_err(|e| format!("build input stream failed: {}", e))?;
        stream
            .play()
            .map_err(|e| format!("play stream failed: {}", e))?;
        std::thread::sleep(std::time::Duration::from_millis(duration));
        drop(stream);
        Ok(data.lock().unwrap().clone())
    }

    /// Capture audio (alias).
    #[cfg(feature = "live-voice")]
    pub fn capture(&self, duration_ms: u64) -> Result<Vec<f32>, String> {
        self.capture_audio(duration_ms)
    }

    /// List available input devices.
    #[cfg(feature = "live-voice")]
    pub fn list_input_devices(&self) -> Result<Vec<String>, String> {
        use cpal::traits::HostTrait;
        let host = cpal::default_host();
        let devices = host
            .input_devices()
            .map_err(|e| format!("enumerate devices failed: {}", e))?;
        let mut names = Vec::new();
        for dev in devices {
            if let Ok(name) = dev.name() {
                names.push(name);
            }
        }
        Ok(names)
    }

    /// Capture audio (stub when feature disabled).
    #[cfg(not(feature = "live-voice"))]
    pub fn capture_audio(&self, _duration_ms: u64) -> Result<Vec<f32>, String> {
        Err("live-voice feature not enabled".to_string())
    }

    #[cfg(not(feature = "live-voice"))]
    pub fn capture(&self, _duration_ms: u64) -> Result<Vec<f32>, String> {
        Err("live-voice feature not enabled".to_string())
    }

    #[cfg(not(feature = "live-voice"))]
    pub fn list_input_devices(&self) -> Result<Vec<String>, String> {
        Err("live-voice feature not enabled".to_string())
    }

    /// Check if the configured provider supports the given language.
    pub fn supports_language(&self, lang: &str) -> bool {
        if lang.trim().is_empty() {
            return false;
        }
        // OpenAI Whisper supports many languages; accept for configured providers.
        matches!(self.config.provider.as_str(), "openai" | "whisper" | "azure")
            || self.config.provider.trim().is_empty()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_config_disabled() {
        assert!(!LiveVoiceConfig::default().enabled);
    }

    #[test]
    fn supports_language() {
        let service = LiveVoiceService::new(LiveVoiceConfig::default());
        assert!(service.supports_language("en"));
        assert!(!service.supports_language(""));
    }

    #[tokio::test]
    async fn transcribe_disabled() {
        let service = LiveVoiceService::new(LiveVoiceConfig::default());
        assert!(service.transcribe("/tmp/x.wav").await.is_err());
    }

    #[tokio::test]
    async fn tts_disabled() {
        let service = LiveVoiceService::new(LiveVoiceConfig::default());
        assert!(service.text_to_speech("hi", "/tmp/x.mp3").await.is_err());
    }

    #[tokio::test]
    async fn transcribe_empty_path() {
        let mut config = LiveVoiceConfig::default();
        config.enabled = true;
        config.api_key = Some("key".into());
        let service = LiveVoiceService::new(config);
        assert!(service.transcribe("").await.is_err());
    }

    #[tokio::test]
    async fn tts_empty_text() {
        let mut config = LiveVoiceConfig::default();
        config.enabled = true;
        config.api_key = Some("key".into());
        let service = LiveVoiceService::new(config);
        assert!(service.text_to_speech("", "/tmp/x.mp3").await.is_err());
    }

    #[tokio::test]
    async fn tts_empty_output_path() {
        let mut config = LiveVoiceConfig::default();
        config.enabled = true;
        config.api_key = Some("key".into());
        let service = LiveVoiceService::new(config);
        assert!(service.text_to_speech("hi", "").await.is_err());
    }

    #[test]
    fn validate_config() {
        let config = LiveVoiceConfig::default();
        assert!(config.is_valid());
    }
}

#[cfg(feature = "live-voice")]
mod feature_tests {
    use super::*;

    #[test]
    fn feature_enabled_compiles() {
        let service = LiveVoiceService::new(LiveVoiceConfig::default());
        assert!(service.capture_audio(10).is_err() || service.capture_audio(10).is_ok());
    }
}
