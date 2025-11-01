use async_trait::async_trait;
use crate::error::{VoiceError, VoiceResult};
use crate::config::VoiceProvider;
use crate::providers::VoiceProviderTrait;

pub struct AwsProvider {
    config: VoiceProvider,
}

impl AwsProvider {
    pub fn new(config: &VoiceProvider) -> VoiceResult<Self> {
        Ok(Self {
            config: config.clone(),
        })
    }
}

#[async_trait]
impl VoiceProviderTrait for AwsProvider {
    async fn transcribe(&self, _audio_data: &[u8], _sample_rate: u32) -> VoiceResult<crate::transcription::TranscriptionResult> {
        Err(VoiceError::Provider("AWS Transcribe Medical not yet implemented".to_string()))
    }

    async fn start_session(&self, _user_id: &str) -> VoiceResult<String> {
        Err(VoiceError::Provider("AWS session management not yet implemented".to_string()))
    }

    async fn stop_session(&self, _session_id: &str) -> VoiceResult<()> {
        Err(VoiceError::Provider("AWS session cleanup not yet implemented".to_string()))
    }
}

