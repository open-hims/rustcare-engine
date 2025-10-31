//! Voice Recognition Service for Healthcare EMR
//! 
//! Provides voice dictation capabilities for clinical documentation with
//! **privacy-first** design, supporting self-hosted and HIPAA-compliant providers.
//!
//! # Provider Hierarchy (Privacy-First)
//!
//! **RECOMMENDED (Self-Hosted, Fully Private):**
//! 1. **Whisper** - OpenAI's open-source model, default, self-hosted
//! 2. **Ollama** - Local LLMs, completely private
//! 3. **Bedrock** - AWS Bedrock, HIPAA-eligible, no training data retention
//!
//! **WHEN NEEDED (Requires BAA):**
//! - Nuance Dragon Medical One
//! - Google Cloud Speech (WARNING: may use data for training)
//! - AWS Transcribe Medical
//! - Azure Speech Service
//!
//! # Features
//!
//! - **Privacy-First**: Defaults to self-hosted Whisper
//! - Medical vocabulary support (SNOMED, ICD-10, CPT codes)
//! - Voice commands for EMR navigation
//! - Real-time transcription
//! - HIPAA-compliant audio processing
//! - Audio format conversion and normalization
//! - Configurable provider selection
//!
//! # Example Usage
//!
//! ```rust,no_run
//! use voice_recognition_service::VoiceService;
//! use voice_recognition_service::VoiceConfig;
//!
//! # async fn example() -> Result<(), Box<dyn std::error::Error>> {
//! let config = VoiceConfig::from_env()?;
//! let voice_service = VoiceService::new(config)?;
//!
//! // Start dictation session
//! let session = voice_service.start_dictation_session("patient_123").await?;
//!
//! // Transcribe audio
//! let transcription = voice_service
//!     .transcribe_audio(session.id, audio_data)
//!     .await?;
//!
//! println!("Transcription: {}", transcription.text);
//! # Ok(())
//! # }
//! ```

pub mod service;
pub mod config;
pub mod providers;
pub mod error;
pub mod transcription;
pub mod medical_vocabulary;

pub use service::*;
pub use config::*;
pub use error::*;
pub use transcription::*;
pub use medical_vocabulary::*;

