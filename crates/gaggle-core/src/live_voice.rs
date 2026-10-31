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
        }
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

    /// Transcribe audio data to text.
    /// In a full implementation, this would call the configured speech recognition provider.
    pub async fn transcribe(&self, _audio_path: &str) -> Result<TranscriptionResult, String> {
        if !self.is_enabled() {
            return Err("Live voice is disabled".to_string());
        }
        // Stub: return empty transcription
        Ok(TranscriptionResult {
            text: String::new(),
            language: self.config.language.clone(),
            confidence: 0.0,
        })
    }

    /// Convert text to speech.
    /// In a full implementation, this would call the configured TTS provider.
    pub async fn text_to_speech(
        &self,
        _text: &str,
        output_path: &str,
    ) -> Result<TtsResult, String> {
        if !self.is_enabled() {
            return Err("Live voice is disabled".to_string());
        }
        // Stub: return empty result
        Ok(TtsResult {
            audio_path: output_path.to_string(),
            duration_ms: 0,
        })
    }

    /// Check if the configured provider supports the given language.
    pub fn supports_language(&self, _lang: &str) -> bool {
        // OpenAI Whisper supports many languages
        true
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
    }
}
