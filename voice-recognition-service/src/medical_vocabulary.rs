/// Medical vocabulary for speech recognition enhancement
/// 
/// Contains medical terms, abbreviations, and clinical phrases to improve
/// transcription accuracy for healthcare documentation.

use serde::{Deserialize, Serialize};
use crate::transcription::{DetectedTerm, TermCategory};

/// Medical vocabulary category
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum VocabularyCategory {
    Diagnoses,
    Medications,
    Procedures,
    Anatomy,
    VitalSigns,
    Laboratory,
    SignsAndSymptoms,
}

/// Medical term entry
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MedicalTerm {
    pub term: String,
    pub aliases: Vec<String>,
    pub category: VocabularyCategory,
    pub code: Option<String>, // ICD-10, CPT, LOINC, SNOMED
}

/// Standard medical vocabulary
pub struct MedicalVocabulary;

impl MedicalVocabulary {
    /// Get common medical abbreviations
    pub fn common_abbreviations() -> Vec<(String, String)> {
        vec![
            // Vital signs
            ("bp".to_string(), "blood pressure".to_string()),
            ("hr".to_string(), "heart rate".to_string()),
            ("rr".to_string(), "respiratory rate".to_string()),
            ("temp".to_string(), "temperature".to_string()),
            ("spo2".to_string(), "oxygen saturation".to_string()),
            
            // Measurements
            ("mg".to_string(), "milligrams".to_string()),
            ("ml".to_string(), "milliliters".to_string()),
            ("kg".to_string(), "kilograms".to_string()),
            ("cm".to_string(), "centimeters".to_string()),
            ("in".to_string(), "inches".to_string()),
            
            // Frequency
            ("bid".to_string(), "twice daily".to_string()),
            ("tid".to_string(), "three times daily".to_string()),
            ("qid".to_string(), "four times daily".to_string()),
            ("qd".to_string(), "once daily".to_string()),
            ("prn".to_string(), "as needed".to_string()),
            
            // Routes
            ("po".to_string(), "by mouth".to_string()),
            ("iv".to_string(), "intravenously".to_string()),
            ("im".to_string(), "intramuscular".to_string()),
            ("sq".to_string(), "subcutaneous".to_string()),
            
            // Common medical terms
            ("chf".to_string(), "congestive heart failure".to_string()),
            ("copd".to_string(), "chronic obstructive pulmonary disease".to_string()),
            ("cva".to_string(), "cerebrovascular accident".to_string()),
            ("mi".to_string(), "myocardial infarction".to_string()),
            ("uti".to_string(), "urinary tract infection".to_string()),
        ]
    }

    /// Get medical vocabulary terms by category
    pub fn get_terms(category: &VocabularyCategory) -> Vec<MedicalTerm> {
        match category {
            VocabularyCategory::VitalSigns => vec![
                MedicalTerm {
                    term: "blood pressure".to_string(),
                    aliases: vec!["bp".to_string(), "blood pressure".to_string()],
                    category: VocabularyCategory::VitalSigns,
                    code: Some("LOINC:85354-9".to_string()),
                },
                MedicalTerm {
                    term: "heart rate".to_string(),
                    aliases: vec!["hr".to_string(), "pulse".to_string()],
                    category: VocabularyCategory::VitalSigns,
                    code: Some("LOINC:8867-4".to_string()),
                },
                MedicalTerm {
                    term: "temperature".to_string(),
                    aliases: vec!["temp".to_string(), "body temperature".to_string()],
                    category: VocabularyCategory::VitalSigns,
                    code: Some("LOINC:8310-5".to_string()),
                },
                MedicalTerm {
                    term: "respiratory rate".to_string(),
                    aliases: vec!["rr".to_string(), "resp rate".to_string()],
                    category: VocabularyCategory::VitalSigns,
                    code: Some("LOINC:9279-1".to_string()),
                },
            ],
            VocabularyCategory::Laboratory => vec![
                MedicalTerm {
                    term: "complete blood count".to_string(),
                    aliases: vec!["cbc".to_string(), "full blood count".to_string()],
                    category: VocabularyCategory::Laboratory,
                    code: Some("LOINC:58410-2".to_string()),
                },
                MedicalTerm {
                    term: "comprehensive metabolic panel".to_string(),
                    aliases: vec!["cmp".to_string(), "chem panel".to_string()],
                    category: VocabularyCategory::Laboratory,
                    code: Some("LOINC:24323-8".to_string()),
                },
            ],
            _ => vec![],
        }
    }

    /// Expand medical abbreviations in text
    pub fn expand_abbreviations(text: &str) -> String {
        let mut result = text.to_lowercase();
        for (abbrev, full) in Self::common_abbreviations() {
            result = result.replace(&abbrev, &full);
        }
        result
    }

    /// Detect medical terms in transcribed text
    pub fn detect_terms(text: &str) -> Vec<DetectedTerm> {
        let mut detected = Vec::new();
        let lower_text = text.to_lowercase();
        
        // Simple keyword matching - in production, use more sophisticated NLP
        if lower_text.contains("blood pressure") || lower_text.contains("bp") {
            detected.push(DetectedTerm {
                term: "blood pressure".to_string(),
                category: TermCategory::VitalSign,
                code: Some("LOINC:85354-9".to_string()),
                confidence: 0.9,
            });
        }
        
        if lower_text.contains("heart rate") || lower_text.contains("hr") || lower_text.contains("pulse") {
            detected.push(DetectedTerm {
                term: "heart rate".to_string(),
                category: TermCategory::VitalSign,
                code: Some("LOINC:8867-4".to_string()),
                confidence: 0.9,
            });
        }
        
        detected
    }
}

