use serde::{Deserialize, Serialize};
use crate::error::{VoiceError, VoiceResult};

/// Voice recognition provider type
#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum VoiceProviderType {
    /// OpenAI Whisper (self-hosted) - PREFERRED open-source option
    Whisper,
    /// Ollama local LLMs (fully private)
    Ollama,
    /// AWS Bedrock (customer data NOT used for training)
    Bedrock,
    /// Nuance Dragon Medical One
    NuanceDragon,
    /// Google Cloud Speech (WHEN NEEDED - data may be used for training)
    #[serde(rename = "google-cloud")]
    GoogleCloud,
    /// AWS Transcribe Medical (WHEN NEEDED)
    #[serde(rename = "aws-transcribe")]
    AwsTranscribe,
    /// Azure Speech Service (WHEN NEEDED)
    #[serde(rename = "azure-speech")]
    AzureSpeech,
}

/// Provider-specific configuration
#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(tag = "type", rename_all = "lowercase")]
pub enum VoiceProvider {
    /// OpenAI Whisper (self-hosted) - PREFERRED open-source
    Whisper {
        api_url: String,
        api_key: Option<String>,
        model_size: Option<String>, // e.g., "base", "small", "medium", "large-v2"
    },
    /// Ollama local LLMs (fully private)
    Ollama {
        api_url: String,
        model: String, // e.g., "whisper-large-v3", "asr-llama3"
        api_key: Option<String>,
    },
    /// AWS Bedrock (customer data NOT used for training)
    Bedrock {
        region: String,
        access_key_id: String,
        secret_access_key: String,
        model_id: String, // e.g., "anthropic.claude-3-opus", "amazon.nova-pro"
    },
    /// Nuance Dragon Medical One
    NuanceDragon {
        api_url: String,
        client_id: String,
        client_secret: String,
        model_name: Option<String>, // e.g., "DragonMedicalPracticeEdition"
    },
    /// Google Cloud Speech-to-Text (WHEN NEEDED - data may be used for training)
    #[serde(rename = "google-cloud")]
    GoogleCloud {
        project_id: String,
        credentials_path: Option<String>,
        model: Option<String>, // e.g., "medical_conversation", "medical_dictation"
    },
    /// AWS Transcribe Medical (WHEN NEEDED)
    #[serde(rename = "aws-transcribe")]
    AwsTranscribe {
        region: String,
        access_key_id: String,
        secret_access_key: String,
    },
    /// Azure Speech Service (WHEN NEEDED)
    #[serde(rename = "azure-speech")]
    AzureSpeech {
        region: String,
        subscription_key: String,
    },
}

/// Voice recognition service configuration
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct VoiceConfig {
    pub provider: VoiceProvider,
    pub enable_medical_vocabulary: bool,
    pub enable_voice_commands: bool,
    pub default_sample_rate: u32,
    pub default_channels: u16,
    pub max_audio_duration_ms: u64,
    pub voice_enabled: bool,
}

impl VoiceConfig {
    /// Load configuration from environment variables
    pub fn from_env() -> VoiceResult<Self> {
        let voice_enabled = std::env::var("VOICE_ENABLED")
            .ok()
            .and_then(|s| s.parse().ok())
            .unwrap_or(true);

        let enable_medical_vocabulary = std::env::var("VOICE_MEDICAL_VOCABULARY")
            .ok()
            .and_then(|s| s.parse().ok())
            .unwrap_or(true);

        let enable_voice_commands = std::env::var("VOICE_COMMANDS_ENABLED")
            .ok()
            .and_then(|s| s.parse().ok())
            .unwrap_or(true);

        let default_sample_rate = std::env::var("VOICE_SAMPLE_RATE")
            .ok()
            .and_then(|s| s.parse().ok())
            .unwrap_or(16000);

        let default_channels = std::env::var("VOICE_CHANNELS")
            .ok()
            .and_then(|s| s.parse().ok())
            .unwrap_or(1);

        let max_audio_duration_ms = std::env::var("VOICE_MAX_DURATION_MS")
            .ok()
            .and_then(|s| s.parse().ok())
            .unwrap_or(300000); // 5 minutes

        // Detect provider from environment
        let provider = if let Ok(provider_type) = std::env::var("VOICE_PROVIDER") {
            match provider_type.to_lowercase().as_str() {
                "whisper" => VoiceProvider::Whisper {
                    api_url: std::env::var("WHISPER_API_URL")
                        .unwrap_or_else(|_| "http://localhost:8000".to_string()),
                    api_key: std::env::var("WHISPER_API_KEY").ok(),
                    model_size: std::env::var("WHISPER_MODEL_SIZE").ok(),
                },
                "ollama" => VoiceProvider::Ollama {
                    api_url: std::env::var("OLLAMA_API_URL")
                        .unwrap_or_else(|_| "http://localhost:11434".to_string()),
                    model: std::env::var("OLLAMA_MODEL")
                        .unwrap_or_else(|_| "whisper-large-v3".to_string()),
                    api_key: std::env::var("OLLAMA_API_KEY").ok(),
                },
                "bedrock" => VoiceProvider::Bedrock {
                    region: std::env::var("AWS_REGION")
                        .unwrap_or_else(|_| "us-east-1".to_string()),
                    access_key_id: std::env::var("AWS_ACCESS_KEY_ID")
                        .unwrap_or_default(),
                    secret_access_key: std::env::var("AWS_SECRET_ACCESS_KEY")
                        .unwrap_or_default(),
                    model_id: std::env::var("BEDROCK_MODEL_ID")
                        .unwrap_or_else(|_| "anthropic.claude-3-opus".to_string()),
                },
                "nuance" | "dragon" => VoiceProvider::NuanceDragon {
                    api_url: std::env::var("NUANCE_API_URL")
                        .unwrap_or_else(|_| "https://dragon.api.nuance.com".to_string()),
                    client_id: std::env::var("NUANCE_CLIENT_ID")
                        .unwrap_or_default(),
                    client_secret: std::env::var("NUANCE_CLIENT_SECRET")
                        .unwrap_or_default(),
                    model_name: std::env::var("NUANCE_MODEL_NAME").ok(),
                },
                "google" => VoiceProvider::GoogleCloud {
                    project_id: std::env::var("GOOGLE_PROJECT_ID")
                        .unwrap_or_default(),
                    credentials_path: std::env::var("GOOGLE_CREDENTIALS_PATH").ok(),
                    model: std::env::var("GOOGLE_SPEECH_MODEL").ok(),
                },
                "aws" => VoiceProvider::AwsTranscribe {
                    region: std::env::var("AWS_REGION")
                        .unwrap_or_else(|_| "us-east-1".to_string()),
                    access_key_id: std::env::var("AWS_ACCESS_KEY_ID")
                        .unwrap_or_default(),
                    secret_access_key: std::env::var("AWS_SECRET_ACCESS_KEY")
                        .unwrap_or_default(),
                },
                "azure" => VoiceProvider::AzureSpeech {
                    region: std::env::var("AZURE_SPEECH_REGION")
                        .unwrap_or_else(|_| "eastus".to_string()),
                    subscription_key: std::env::var("AZURE_SPEECH_KEY")
                        .unwrap_or_default(),
                },
                _ => return Err(VoiceError::Config(
                    format!("Unknown voice provider: {}", provider_type)
                )),
            }
        } else {
            // Default to Whisper (open-source, self-hosted)
            VoiceProvider::Whisper {
                api_url: "http://localhost:8000".to_string(),
                api_key: None,
                model_size: Some("base".to_string()),
            }
        };

        Ok(Self {
            provider,
            enable_medical_vocabulary,
            enable_voice_commands,
            default_sample_rate,
            default_channels,
            max_audio_duration_ms,
            voice_enabled,
        })
    }
}

