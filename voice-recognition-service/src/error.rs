use thiserror::Error;

#[derive(Error, Debug)]
pub enum VoiceError {
    #[error("Configuration error: {0}")]
    Config(String),

    #[error("Provider error: {0}")]
    Provider(String),

    #[error("Authentication failed: {0}")]
    Authentication(String),

    #[error("Audio processing error: {0}")]
    AudioProcessing(String),

    #[error("Transcription error: {0}")]
    Transcription(String),

    #[error("Network error: {0}")]
    Network(#[from] reqwest::Error),

    #[error("Serialization error: {0}")]
    Serialization(#[from] serde_json::Error),

    #[error("Unknown error: {0}")]
    Unknown(String),
}

pub type VoiceResult<T> = Result<T, VoiceError>;

