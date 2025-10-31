use crate::config::VoiceConfig;
use crate::error::{VoiceError, VoiceResult};
use crate::providers::{VoiceProviderTrait, create_provider};
use crate::transcription::{TranscriptionResult, DictationSession, SessionStatus};
use crate::medical_vocabulary::MedicalVocabulary;
use uuid::Uuid;
use std::collections::HashMap;
use tracing::{info, debug};

/// Voice recognition service for healthcare dictation
pub struct VoiceService {
    config: VoiceConfig,
    provider: Box<dyn VoiceProviderTrait>,
    active_sessions: HashMap<Uuid, DictationSession>,
}

impl VoiceService {
    /// Create a new voice recognition service
    pub fn new(config: VoiceConfig) -> VoiceResult<Self> {
        if !config.voice_enabled {
            info!("Voice recognition service disabled by configuration");
        }

        let provider = create_provider(&config.provider)?;

        Ok(Self {
            config,
            provider,
            active_sessions: HashMap::new(),
        })
    }

    /// Start a new dictation session
    pub async fn start_dictation_session(&mut self, user_id: Uuid, provider_name: Option<String>) -> VoiceResult<DictationSession> {
        let session = DictationSession::new(user_id, provider_name.unwrap_or_else(|| "default".to_string()));
        
        info!(session_id = %session.id, user_id = %user_id, "Starting dictation session");
        
        // Start provider session
        let _provider_session_id = self.provider.start_session(&user_id.to_string()).await?;
        
        self.active_sessions.insert(session.id, session.clone());
        
        Ok(session)
    }

    /// Stop a dictation session
    pub async fn stop_dictation_session(&mut self, session_id: Uuid) -> VoiceResult<()> {
        if let Some(mut session) = self.active_sessions.remove(&session_id) {
            info!(session_id = %session_id, "Stopping dictation session");
            
            session.status = SessionStatus::Completed;
            session.updated_at = chrono::Utc::now();
            
            // Stop provider session
            self.provider.stop_session(&session_id.to_string()).await?;
            
            Ok(())
        } else {
            Err(VoiceError::Provider(format!("Session {} not found", session_id)))
        }
    }

    /// Transcribe audio data
    pub async fn transcribe_audio(&self, audio_data: &[u8], sample_rate: u32) -> VoiceResult<TranscriptionResult> {
        debug!(audio_size = audio_data.len(), sample_rate = sample_rate, "Transcribing audio");

        // Transcribe using provider
        let mut result = self.provider.transcribe(audio_data, sample_rate).await?;

        // Apply medical vocabulary enhancement if enabled
        if self.config.enable_medical_vocabulary {
            result = self.enhance_with_medical_vocabulary(result)?;
        }

        Ok(result)
    }

    /// Enhance transcription with medical vocabulary
    fn enhance_with_medical_vocabulary(&self, result: TranscriptionResult) -> VoiceResult<TranscriptionResult> {
        // Expand abbreviations
        let expanded_text = MedicalVocabulary::expand_abbreviations(&result.text);
        
        // Detect medical terms
        let detected_terms = MedicalVocabulary::detect_terms(&expanded_text);
        
        // Update metadata
        let mut metadata = result.metadata;
        metadata.medical_vocabulary_used = true;
        metadata.detected_terms = detected_terms;

        Ok(TranscriptionResult {
            text: expanded_text,
            metadata,
            ..result
        })
    }

    /// Get active sessions
    pub fn get_active_sessions(&self) -> Vec<&DictationSession> {
        self.active_sessions.values().collect()
    }

    /// Check if medical vocabulary is enabled
    pub fn is_medical_vocabulary_enabled(&self) -> bool {
        self.config.enable_medical_vocabulary
    }

    /// Check if voice commands are enabled
    pub fn are_voice_commands_enabled(&self) -> bool {
        self.config.enable_voice_commands
    }
}

