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
}

impl Default for LiveVoiceConfig {
    fn default() -> Self {
        Self {
            enabled: false,
            provider: "openai".to_string(),
            language: "en".to_string(),
            auto_transcribe: true,
        }
    }
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
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_config_disabled() {
        assert!(!LiveVoiceConfig::default().enabled);
    }
}
