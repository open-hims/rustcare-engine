//! Field-Level Encryption for PHI Data
//!
//! This module provides double-layer encryption for sensitive PHI fields
//! in sync queue operations. It uses AES-256-GCM for field-level encryption
//! on top of database-level encryption (SQLCipher).
//!
//! # Security Model
//!
//! - Database-level encryption: Protects entire database at rest (LS1)
//! - Field-level encryption: Protects specific PHI fields even if DB is decrypted
//! - Defense in depth: Two independent encryption layers
//!
//! # Usage
//!
//! ```no_run
//! use rustcare_sync::field_encryption::{FieldEncryption, FieldEncryptionConfig};
//!
//! let config = FieldEncryptionConfig {
//!     enabled: true,
//!     phi_fields: vec![
//!         "ssn".to_string(),
//!         "medical_record_number".to_string(),
//!         "diagnosis".to_string(),
//!     ],
//! };
//!
//! let encryptor = FieldEncryption::new(config, master_key);
//!
//! // Encrypt PHI fields before storing in sync queue
//! let encrypted_data = encryptor.encrypt_phi_fields(&original_data).await?;
//!
//! // Decrypt PHI fields when retrieving from sync queue
//! let decrypted_data = encryptor.decrypt_phi_fields(&encrypted_data).await?;
//! ```

use crate::error::SyncResult;
use crypto::Aes256GcmEncryptor;
use crypto::Encryptor;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::HashSet;
use base64::{engine::general_purpose::STANDARD as BASE64, Engine};

/// Configuration for field-level encryption
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FieldEncryptionConfig {
    /// Whether field-level encryption is enabled
    pub enabled: bool,
    
    /// List of field names that contain PHI and should be encrypted
    /// These are JSON keys in the data payload
    pub phi_fields: Vec<String>,
}

impl Default for FieldEncryptionConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            phi_fields: vec![
                // Patient identifiers
                "ssn".to_string(),
                "social_security_number".to_string(),
                "medical_record_number".to_string(),
                "mrn".to_string(),
                "patient_id".to_string(),
                
                // Personal information
                "first_name".to_string(),
                "last_name".to_string(),
                "full_name".to_string(),
                "date_of_birth".to_string(),
                "dob".to_string(),
                "birth_date".to_string(),
                "address".to_string(),
                "street_address".to_string(),
                "phone".to_string(),
                "phone_number".to_string(),
                "email".to_string(),
                
                // Medical information
                "diagnosis".to_string(),
                "diagnoses".to_string(),
                "treatment".to_string(),
                "medication".to_string(),
                "medications".to_string(),
                "notes".to_string(),
                "clinical_notes".to_string(),
                "lab_results".to_string(),
                "test_results".to_string(),
            ],
        }
    }
}

/// Field-level encryption handler
pub struct FieldEncryption {
    config: FieldEncryptionConfig,
    encryptor: Aes256GcmEncryptor,
    phi_fields_set: HashSet<String>,
}

impl FieldEncryption {
    /// Create a new field encryption handler
    pub fn new(config: FieldEncryptionConfig, master_key: &[u8]) -> SyncResult<Self> {
        // Convert slice to array
        if master_key.len() != 32 {
            return Err(crate::error::SyncError::Internal(
                format!("Invalid key length: expected 32 bytes, got {}", master_key.len())
            ));
        }
        
        let mut key = [0u8; 32];
        key.copy_from_slice(master_key);
        
        let encryptor = Aes256GcmEncryptor::new(key)
            .map_err(|e| crate::error::SyncError::Internal(format!("Failed to initialize AES-GCM: {}", e)))?;
        
        let phi_fields_set: HashSet<String> = config.phi_fields.iter().cloned().collect();
        
        Ok(Self {
            config,
            encryptor,
            phi_fields_set,
        })
    }
    
    /// Encrypt PHI fields in a JSON value
    ///
    /// This recursively traverses the JSON structure and encrypts any fields
    /// that match the configured PHI field names. The encrypted data is stored
    /// as a base64-encoded string with a prefix to indicate it's encrypted.
    pub fn encrypt_phi_fields(&self, data: &Value) -> SyncResult<Value> {
        if !self.config.enabled {
            return Ok(data.clone());
        }
        
        self.encrypt_value(data)
    }
    
    /// Decrypt PHI fields in a JSON value
    ///
    /// This recursively traverses the JSON structure and decrypts any fields
    /// that have the encrypted data prefix.
    pub fn decrypt_phi_fields(&self, data: &Value) -> SyncResult<Value> {
        if !self.config.enabled {
            return Ok(data.clone());
        }
        
        self.decrypt_value(data)
    }
    
    /// Recursively encrypt a JSON value
    fn encrypt_value(&self, value: &Value) -> SyncResult<Value> {
        match value {
            Value::Object(map) => {
                let mut encrypted_map = serde_json::Map::new();
                for (key, val) in map {
                    if self.phi_fields_set.contains(key) {
                        // This is a PHI field, encrypt it
                        encrypted_map.insert(key.clone(), self.encrypt_field(val)?);
                    } else {
                        // Recursively process nested structures
                        encrypted_map.insert(key.clone(), self.encrypt_value(val)?);
                    }
                }
                Ok(Value::Object(encrypted_map))
            }
            Value::Array(arr) => {
                let encrypted_arr: Result<Vec<Value>, _> = arr
                    .iter()
                    .map(|v| self.encrypt_value(v))
                    .collect();
                Ok(Value::Array(encrypted_arr?))
            }
            _ => Ok(value.clone()),
        }
    }
    
    /// Recursively decrypt a JSON value
    fn decrypt_value(&self, value: &Value) -> SyncResult<Value> {
        match value {
            Value::Object(map) => {
                let mut decrypted_map = serde_json::Map::new();
                for (key, val) in map {
                    // Check if this value is encrypted (starts with our prefix)
                    if let Value::String(s) = val {
                        if s.starts_with("ENC:") {
                            decrypted_map.insert(key.clone(), self.decrypt_field(val)?);
                            continue;
                        }
                    }
                    // Recursively process nested structures
                    decrypted_map.insert(key.clone(), self.decrypt_value(val)?);
                }
                Ok(Value::Object(decrypted_map))
            }
            Value::Array(arr) => {
                let decrypted_arr: Result<Vec<Value>, _> = arr
                    .iter()
                    .map(|v| self.decrypt_value(v))
                    .collect();
                Ok(Value::Array(decrypted_arr?))
            }
            _ => Ok(value.clone()),
        }
    }
    
    /// Encrypt a single field value
    fn encrypt_field(&self, value: &Value) -> SyncResult<Value> {
        // Convert value to JSON string for encryption
        let plaintext = serde_json::to_string(value)
            .map_err(|e| crate::error::SyncError::Internal(format!("Failed to serialize value: {}", e)))?;
        
        // Encrypt the plaintext
        let ciphertext = self.encryptor.encrypt(plaintext.as_bytes())
            .map_err(|e| crate::error::SyncError::Internal(format!("Failed to encrypt field: {}", e)))?;
        
        // Encode as base64 with prefix
        let encoded = format!("ENC:{}", BASE64.encode(&ciphertext));
        
        Ok(Value::String(encoded))
    }
    
    /// Decrypt a single field value
    fn decrypt_field(&self, value: &Value) -> SyncResult<Value> {
        if let Value::String(s) = value {
            if !s.starts_with("ENC:") {
                return Err(crate::error::SyncError::Internal("Invalid encrypted field format".to_string()));
            }
            
            // Remove prefix and decode from base64
            let encoded = &s[4..];
            let ciphertext = BASE64.decode(encoded)
                .map_err(|e| crate::error::SyncError::Internal(format!("Failed to decode base64: {}", e)))?;
            
            // Decrypt the ciphertext
            let plaintext_bytes = self.encryptor.decrypt(&ciphertext)
                .map_err(|e| crate::error::SyncError::Internal(format!("Failed to decrypt field: {}", e)))?;
            
            // Parse back to JSON value
            let plaintext = String::from_utf8(plaintext_bytes)
                .map_err(|e| crate::error::SyncError::Internal(format!("Invalid UTF-8 in decrypted data: {}", e)))?;
            
            let value: Value = serde_json::from_str(&plaintext)
                .map_err(|e| crate::error::SyncError::Internal(format!("Failed to parse decrypted JSON: {}", e)))?;
            
            Ok(value)
        } else {
            Err(crate::error::SyncError::Internal("Expected string value for encrypted field".to_string()))
        }
    }
    
    /// Check if a field name is configured as a PHI field
    pub fn is_phi_field(&self, field_name: &str) -> bool {
        self.phi_fields_set.contains(field_name)
    }
    
    /// Get the list of configured PHI fields
    pub fn phi_fields(&self) -> &[String] {
        &self.config.phi_fields
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;
    
    fn create_test_key() -> [u8; 32] {
        // Generate a test key (32 bytes for AES-256)
        [42u8; 32]  // Use a consistent non-zero value for testing
    }
    
    #[test]
    fn test_field_encryption_creation() {
        let config = FieldEncryptionConfig::default();
        let key = create_test_key();
        
        let encryptor = FieldEncryption::new(config, &key).unwrap();
        
        assert!(encryptor.is_phi_field("ssn"));
        assert!(encryptor.is_phi_field("diagnosis"));
        assert!(!encryptor.is_phi_field("non_phi_field"));
    }
    
    #[test]
    fn test_encrypt_decrypt_simple_field() {
        let config = FieldEncryptionConfig {
            enabled: true,
            phi_fields: vec!["ssn".to_string()],
        };
        let key = create_test_key();
        let encryptor = FieldEncryption::new(config, &key).unwrap();
        
        let original = json!({
            "ssn": "123-45-6789",
            "name": "John Doe"
        });
        
        // Encrypt
        let encrypted = encryptor.encrypt_phi_fields(&original).unwrap();
        
        // SSN should be encrypted
        assert!(encrypted["ssn"].is_string());
        assert!(encrypted["ssn"].as_str().unwrap().starts_with("ENC:"));
        
        // Name should not be encrypted (not in PHI fields list)
        assert_eq!(encrypted["name"], "John Doe");
        
        // Decrypt
        let decrypted = encryptor.decrypt_phi_fields(&encrypted).unwrap();
        
        // Should match original
        assert_eq!(decrypted, original);
    }
    
    #[test]
    fn test_encrypt_decrypt_nested_structure() {
        let config = FieldEncryptionConfig {
            enabled: true,
            phi_fields: vec!["diagnosis".to_string(), "ssn".to_string()],
        };
        let key = create_test_key();
        let encryptor = FieldEncryption::new(config, &key).unwrap();
        
        let original = json!({
            "patient": {
                "ssn": "123-45-6789",
                "name": "John Doe"
            },
            "visit": {
                "diagnosis": "Hypertension",
                "visit_date": "2024-01-01"
            }
        });
        
        // Encrypt
        let encrypted = encryptor.encrypt_phi_fields(&original).unwrap();
        
        // Check encryption
        assert!(encrypted["patient"]["ssn"].as_str().unwrap().starts_with("ENC:"));
        assert!(encrypted["visit"]["diagnosis"].as_str().unwrap().starts_with("ENC:"));
        assert_eq!(encrypted["patient"]["name"], "John Doe");
        assert_eq!(encrypted["visit"]["visit_date"], "2024-01-01");
        
        // Decrypt
        let decrypted = encryptor.decrypt_phi_fields(&encrypted).unwrap();
        
        // Should match original
        assert_eq!(decrypted, original);
    }
    
    #[test]
    fn test_encrypt_decrypt_array() {
        let config = FieldEncryptionConfig {
            enabled: true,
            phi_fields: vec!["medication".to_string()],
        };
        let key = create_test_key();
        let encryptor = FieldEncryption::new(config, &key).unwrap();
        
        let original = json!({
            "medications": [
                {
                    "medication": "Aspirin",
                    "dosage": "100mg"
                },
                {
                    "medication": "Lisinopril",
                    "dosage": "10mg"
                }
            ]
        });
        
        // Encrypt
        let encrypted = encryptor.encrypt_phi_fields(&original).unwrap();
        
        // Check encryption
        let medications = encrypted["medications"].as_array().unwrap();
        assert!(medications[0]["medication"].as_str().unwrap().starts_with("ENC:"));
        assert!(medications[1]["medication"].as_str().unwrap().starts_with("ENC:"));
        assert_eq!(medications[0]["dosage"], "100mg");
        assert_eq!(medications[1]["dosage"], "10mg");
        
        // Decrypt
        let decrypted = encryptor.decrypt_phi_fields(&encrypted).unwrap();
        
        // Should match original
        assert_eq!(decrypted, original);
    }
    
    #[test]
    fn test_disabled_encryption() {
        let config = FieldEncryptionConfig {
            enabled: false,
            phi_fields: vec!["ssn".to_string()],
        };
        let key = create_test_key();
        let encryptor = FieldEncryption::new(config, &key).unwrap();
        
        let original = json!({
            "ssn": "123-45-6789",
            "name": "John Doe"
        });
        
        // When disabled, should return original unchanged
        let encrypted = encryptor.encrypt_phi_fields(&original).unwrap();
        assert_eq!(encrypted, original);
        
        let decrypted = encryptor.decrypt_phi_fields(&encrypted).unwrap();
        assert_eq!(decrypted, original);
    }
    
    #[test]
    fn test_encrypt_various_types() {
        let config = FieldEncryptionConfig {
            enabled: true,
            phi_fields: vec!["phi_field".to_string()],
        };
        let key = create_test_key();
        let encryptor = FieldEncryption::new(config, &key).unwrap();
        
        // Test with different value types
        let test_cases = vec![
            json!({"phi_field": "string value"}),
            json!({"phi_field": 12345}),
            json!({"phi_field": 123.45}),
            json!({"phi_field": true}),
            json!({"phi_field": null}),
            json!({"phi_field": ["array", "values"]}),
            json!({"phi_field": {"nested": "object"}}),
        ];
        
        for original in test_cases {
            let encrypted = encryptor.encrypt_phi_fields(&original).unwrap();
            assert!(encrypted["phi_field"].as_str().unwrap().starts_with("ENC:"));
            
            let decrypted = encryptor.decrypt_phi_fields(&encrypted).unwrap();
            assert_eq!(decrypted, original);
        }
    }
}
