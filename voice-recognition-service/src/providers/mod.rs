pub mod whisper;
pub mod ollama;
pub mod bedrock;
pub mod nuance;
pub mod google;
pub mod aws;
pub mod azure;

use async_trait::async_trait;
use crate::error::VoiceResult;
use crate::config::VoiceProvider;
use crate::transcription::TranscriptionResult;

/// Trait for voice recognition providers
#[async_trait]
pub trait VoiceProviderTrait: Send + Sync {
    /// Transcribe audio data to text
    async fn transcribe(&self, audio_data: &[u8], sample_rate: u32) -> VoiceResult<TranscriptionResult>;
    
    /// Start a continuous dictation session
    async fn start_session(&self, user_id: &str) -> VoiceResult<String>; // returns session_id
    
    /// Stop a continuous dictation session
    async fn stop_session(&self, session_id: &str) -> VoiceResult<()>;
}

/// Create a provider instance based on configuration
pub fn create_provider(config: &VoiceProvider) -> VoiceResult<Box<dyn VoiceProviderTrait>> {
    match config {
        VoiceProvider::Whisper { .. } => {
            Ok(Box::new(whisper::WhisperProvider::new(config)?))
        }
        VoiceProvider::Ollama { .. } => {
            Ok(Box::new(ollama::OllamaProvider::new(config)?))
        }
        VoiceProvider::Bedrock { .. } => {
            Ok(Box::new(bedrock::BedrockProvider::new(config)?))
        }
        VoiceProvider::NuanceDragon { .. } => {
            Ok(Box::new(nuance::NuanceProvider::new(config)?))
        }
        VoiceProvider::GoogleCloud { .. } => {
            Ok(Box::new(google::GoogleProvider::new(config)?))
        }
        VoiceProvider::AwsTranscribe { .. } => {
            Ok(Box::new(aws::AwsProvider::new(config)?))
        }
        VoiceProvider::AzureSpeech { .. } => {
            Ok(Box::new(azure::AzureProvider::new(config)?))
        }
    }
}

