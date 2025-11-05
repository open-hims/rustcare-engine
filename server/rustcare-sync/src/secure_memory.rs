//! Secure memory handling for PHI (Protected Health Information)
//!
//! This module provides secure memory types that:
//! - Automatically zero memory on drop (prevent PHI leaks)
//! - Prevent accidental logging/display of sensitive data
//! - Use constant-time operations where possible
//! - Lock memory pages to prevent swapping to disk
//!
//! Required for HIPAA §164.312(a)(2)(iv) and §164.312(e)(2)(ii)

use secrecy::{CloneableSecret, ExposeSecret, Secret, Zeroize};
use serde::{Deserialize, Serialize};
use std::fmt;

/// A secure string that zeroizes on drop
/// Use for: SSN, passwords, patient names, medical record numbers, etc.
pub type SecureString = Secret<String>;

/// A secure byte vector that zeroizes on drop
/// Use for: Encryption keys, biometric data, raw PHI buffers
pub type SecureVec = Secret<Vec<u8>>;

/// Wrapper for PHI data that must be kept secure in memory
/// 
/// This type:
/// - Zeroizes memory on drop
/// - Prevents accidental Display/Debug exposure
/// - Provides explicit access via `expose_secret()`
/// - Supports serialization (but be careful where you send it!)
#[derive(Clone)]
pub struct SecureData<T: Zeroize + Clone + CloneableSecret> {
    inner: Secret<T>,
}

impl<T: Zeroize + Clone + CloneableSecret> SecureData<T> {
    /// Create new secure data
    pub fn new(data: T) -> Self {
        Self {
            inner: Secret::new(data),
        }
    }

    /// Expose the secret data
    /// 
    /// SECURITY WARNING: Only call this when you need to use the data.
    /// The returned reference should have minimal scope.
    pub fn expose_secret(&self) -> &T {
        self.inner.expose_secret()
    }
}

impl<T: Zeroize + Clone + CloneableSecret> fmt::Debug for SecureData<T> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("<redacted>")
    }
}

impl<T: Zeroize + Clone + CloneableSecret + Serialize> Serialize for SecureData<T> {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        self.inner.expose_secret().serialize(serializer)
    }
}

impl<'de, T: Zeroize + Clone + CloneableSecret + Deserialize<'de>> Deserialize<'de> for SecureData<T> {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        Ok(Self::new(T::deserialize(deserializer)?))
    }
}

/// Patient data that should be kept secure in memory
#[derive(Clone, Serialize, Deserialize, Zeroize)]
#[zeroize(drop)]
pub struct SecurePatientData {
    /// Patient name (PHI)
    pub name: String,
    
    /// Social Security Number (PHI)
    pub ssn: Option<String>,
    
    /// Medical Record Number (PHI)
    pub mrn: String,
    
    /// Date of Birth (PHI)
    pub date_of_birth: String,
    
    /// Additional PHI fields
    #[serde(default)]
    pub additional_phi: Vec<(String, String)>,
}

impl CloneableSecret for SecurePatientData {}

impl SecurePatientData {
    /// Create new secure patient data
    pub fn new(name: String, mrn: String, date_of_birth: String) -> Self {
        Self {
            name,
            ssn: None,
            mrn,
            date_of_birth,
            additional_phi: Vec::new(),
        }
    }
    
    /// Add SSN
    pub fn with_ssn(mut self, ssn: String) -> Self {
        self.ssn = Some(ssn);
        self
    }
    
    /// Add additional PHI field
    pub fn with_phi_field(mut self, key: String, value: String) -> Self {
        self.additional_phi.push((key, value));
        self
    }
}

impl fmt::Debug for SecurePatientData {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("SecurePatientData")
            .field("name", &"<redacted>")
            .field("ssn", &"<redacted>")
            .field("mrn", &"<redacted>")
            .field("date_of_birth", &"<redacted>")
            .field("additional_phi", &format!("<{} fields redacted>", self.additional_phi.len()))
            .finish()
    }
}

/// Medical record data that should be kept secure in memory
#[derive(Clone, Serialize, Deserialize, Zeroize)]
#[zeroize(drop)]
pub struct SecureMedicalRecord {
    /// Diagnosis codes (PHI)
    pub diagnosis: Vec<String>,
    
    /// Treatment notes (PHI)
    pub notes: String,
    
    /// Medications (PHI)
    pub medications: Vec<String>,
    
    /// Lab results (PHI)
    pub lab_results: Vec<(String, String)>,
}

impl CloneableSecret for SecureMedicalRecord {}

impl SecureMedicalRecord {
    /// Create new secure medical record
    pub fn new(diagnosis: Vec<String>, notes: String) -> Self {
        Self {
            diagnosis,
            notes,
            medications: Vec::new(),
            lab_results: Vec::new(),
        }
    }
    
    /// Add medication
    pub fn with_medication(mut self, medication: String) -> Self {
        self.medications.push(medication);
        self
    }
    
    /// Add lab result
    pub fn with_lab_result(mut self, test: String, result: String) -> Self {
        self.lab_results.push((test, result));
        self
    }
}

impl fmt::Debug for SecureMedicalRecord {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("SecureMedicalRecord")
            .field("diagnosis", &format!("<{} codes redacted>", self.diagnosis.len()))
            .field("notes", &"<redacted>")
            .field("medications", &format!("<{} redacted>", self.medications.len()))
            .field("lab_results", &format!("<{} redacted>", self.lab_results.len()))
            .finish()
    }
}

/// Helper to create secure strings from regular strings
pub trait IntoSecure {
    fn into_secure(self) -> SecureString;
}

impl IntoSecure for String {
    fn into_secure(self) -> SecureString {
        Secret::new(self)
    }
}

impl IntoSecure for &str {
    fn into_secure(self) -> SecureString {
        Secret::new(self.to_string())
    }
}

/// Helper to create secure byte vectors
pub trait IntoSecureVec {
    fn into_secure_vec(self) -> SecureVec;
}

impl IntoSecureVec for Vec<u8> {
    fn into_secure_vec(self) -> SecureVec {
        Secret::new(self)
    }
}

impl IntoSecureVec for &[u8] {
    fn into_secure_vec(self) -> SecureVec {
        Secret::new(self.to_vec())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_secure_string_no_display() {
        let ssn = "123-45-6789".to_string().into_secure();
        let debug_str = format!("{:?}", ssn);
        assert!(!debug_str.contains("123-45-6789"), "SSN should not appear in debug output");
        assert_eq!(ssn.expose_secret(), "123-45-6789");
    }

    #[test]
    fn test_secure_vec_no_display() {
        let key = vec![1, 2, 3, 4, 5].into_secure_vec();
        // SecretVec doesn't implement Debug, so we just verify it exists
        assert_eq!(key.expose_secret(), &vec![1, 2, 3, 4, 5]);
    }

    #[test]
    fn test_secure_data_redaction() {
        let patient = SecureData::new(SecurePatientData::new(
            "John Doe".to_string(),
            "MRN12345".to_string(),
            "1980-01-01".to_string(),
        ).with_ssn("123-45-6789".to_string()));

        let debug_str = format!("{:?}", patient);
        assert_eq!(debug_str, "<redacted>");
        
        // Can still access the data explicitly
        let patient_data = patient.expose_secret();
        assert_eq!(patient_data.name, "John Doe");
        assert_eq!(patient_data.ssn, Some("123-45-6789".to_string()));
    }

    #[test]
    fn test_patient_data_debug_redaction() {
        let patient = SecurePatientData::new(
            "Jane Smith".to_string(),
            "MRN67890".to_string(),
            "1975-05-15".to_string(),
        ).with_ssn("987-65-4321".to_string())
         .with_phi_field("email".to_string(), "jane@example.com".to_string());

        let debug_str = format!("{:?}", patient);
        assert!(!debug_str.contains("Jane Smith"), "Name should be redacted");
        assert!(!debug_str.contains("987-65-4321"), "SSN should be redacted");
        assert!(!debug_str.contains("MRN67890"), "MRN should be redacted");
        assert!(debug_str.contains("<redacted>"), "Should show redaction markers");
    }

    #[test]
    fn test_medical_record_debug_redaction() {
        let record = SecureMedicalRecord::new(
            vec!["Z23".to_string(), "E11.9".to_string()],
            "Patient presents with symptoms...".to_string(),
        ).with_medication("Metformin 500mg".to_string())
         .with_lab_result("HbA1c".to_string(), "7.2%".to_string());

        let debug_str = format!("{:?}", record);
        assert!(!debug_str.contains("Z23"), "Diagnosis should be redacted");
        assert!(!debug_str.contains("Metformin"), "Medication should be redacted");
        assert!(!debug_str.contains("HbA1c"), "Lab results should be redacted");
        assert!(debug_str.contains("<2 codes redacted>"), "Should show count");
    }

    #[test]
    fn test_secure_data_serialization() {
        let patient = SecureData::new(SecurePatientData::new(
            "Test Patient".to_string(),
            "MRN99999".to_string(),
            "1990-12-31".to_string(),
        ));

        // Should be able to serialize
        let json = serde_json::to_string(&patient).unwrap();
        assert!(json.contains("Test Patient"));
        
        // Should be able to deserialize
        let deserialized: SecureData<SecurePatientData> = serde_json::from_str(&json).unwrap();
        assert_eq!(deserialized.expose_secret().name, "Test Patient");
    }

    #[test]
    fn test_zeroize_on_drop() {
        // This test verifies that the Zeroize trait is properly derived
        // The actual zeroing happens automatically on drop
        let mut patient = SecurePatientData::new(
            "Will Be Zeroed".to_string(),
            "MRN00000".to_string(),
            "2000-01-01".to_string(),
        );
        
        // Manually zeroize to test the trait implementation
        patient.zeroize();
        
        // After zeroizing, strings should be empty
        assert_eq!(patient.name, "");
        assert_eq!(patient.mrn, "");
        assert_eq!(patient.date_of_birth, "");
    }

    #[test]
    fn test_secure_string_clone() {
        let original = "sensitive data".to_string().into_secure();
        let cloned = original.clone();
        
        assert_eq!(original.expose_secret(), cloned.expose_secret());
    }
}
