/// Google Cloud Speech-to-Text Provider
/// 
/// WARNING: Google may use your data to train their models unless you
/// have a Business Associate Agreement (BAA) with specific data retention
/// guarantees. Prefer self-hosted Whisper or Ollama for privacy.
use async_trait::async_trait;
use crate::error::{VoiceError, VoiceResult};
use crate::config::VoiceProvider;
use crate::providers::VoiceProviderTrait;

pub struct GoogleProvider {
    config: VoiceProvider,
}

impl GoogleProvider {
    pub fn new(config: &VoiceProvider) -> VoiceResult<Self> {
        Ok(Self {
            config: config.clone(),
        })
    }
}

#[async_trait]
impl VoiceProviderTrait for GoogleProvider {
    async fn transcribe(&self, _audio_data: &[u8], _sample_rate: u32) -> VoiceResult<crate::transcription::TranscriptionResult> {
        Err(VoiceError::Provider("Google Cloud Speech not yet implemented".to_string()))
    }

    async fn start_session(&self, _user_id: &str) -> VoiceResult<String> {
        Err(VoiceError::Provider("Google session management not yet implemented".to_string()))
    }

    async fn stop_session(&self, _session_id: &str) -> VoiceResult<()> {
        Err(VoiceError::Provider("Google session cleanup not yet implemented".to_string()))
    }
}

