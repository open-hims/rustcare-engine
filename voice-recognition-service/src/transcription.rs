use serde::{Deserialize, Serialize};
use uuid::Uuid;
use chrono::{DateTime, Utc};

/// Transcription result from voice recognition
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TranscriptionResult {
    pub id: Uuid,
    pub session_id: Option<Uuid>,
    pub text: String,
    pub confidence: f32,
    pub language: String,
    pub duration_ms: u64,
    pub created_at: DateTime<Utc>,
    pub metadata: TranscriptionMetadata,
}

/// Metadata associated with transcription
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TranscriptionMetadata {
    pub provider: String,
    pub model: Option<String>,
    pub medical_vocabulary_used: bool,
    pub detected_terms: Vec<DetectedTerm>,
    pub alternative_transcriptions: Vec<String>,
}

/// Medical term detected in transcription
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DetectedTerm {
    pub term: String,
    pub category: TermCategory,
    pub code: Option<String>, // ICD-10, CPT, SNOMED, etc.
    pub confidence: f32,
}

/// Category of medical term
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum TermCategory {
    Diagnosis,
    Procedure,
    Medication,
    Anatomy,
    Symptom,
    VitalSign,
    Laboratory,
    Other,
}

/// Voice dictation session
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DictationSession {
    pub id: Uuid,
    pub user_id: Uuid,
    pub patient_id: Option<Uuid>,
    pub encounter_id: Option<Uuid>,
    pub status: SessionStatus,
    pub provider: String,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

/// Dictation session status
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SessionStatus {
    Active,
    Paused,
    Completed,
    Cancelled,
    Error,
}

impl DictationSession {
    pub fn new(user_id: Uuid, provider: String) -> Self {
        Self {
            id: Uuid::new_v4(),
            user_id,
            patient_id: None,
            encounter_id: None,
            status: SessionStatus::Active,
            provider,
            created_at: Utc::now(),
            updated_at: Utc::now(),
        }
    }
}

