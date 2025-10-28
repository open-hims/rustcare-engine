// Encryption tests for field-level encryption
use database_layer::encryption::{
    DatabaseEncryption, EncryptionAlgorithm, EncryptionConfig, EncryptionError,
    EncryptionKeyStore, FieldEncryptionConfig,
};
use std::collections::HashMap;

// =============================================================================
// HELPER FUNCTIONS
// =============================================================================

/// Generate a test encryption key (32 random bytes)
fn generate_test_key() -> Vec<u8> {
    (0..32).map(|_| rand::random::<u8>()).collect()
}

/// Create a test encryption config
fn test_config() -> EncryptionConfig {
    EncryptionConfig {
        enabled: true,
        field_mappings: HashMap::new(),
        master_key: generate_test_key(),
        key_version: 1,
    }
}

// =============================================================================
// UNIT TESTS - ENCRYPTION/DECRYPTION
// =============================================================================

#[test]
fn test_encrypt_decrypt_roundtrip() {
    let config = test_config();
    let encryption = DatabaseEncryption::new(config).unwrap();
    
    let plaintext = "sensitive-ssn-123-45-6789";
    
    // Encrypt
    let encrypted = encryption.encrypt_value(plaintext).unwrap();
    
    // Verify format: v{version}:{nonce}:{ciphertext}
    assert!(encrypted.starts_with("v1:"));
    assert_eq!(encrypted.split(':').count(), 3);
    
    // Decrypt
    let decrypted = encryption.decrypt_value(&encrypted).unwrap();
    assert_eq!(decrypted, plaintext);
}

#[test]
fn test_encrypt_different_values_produce_different_ciphertexts() {
    let config = test_config();
    let encryption = DatabaseEncryption::new(config).unwrap();
    
    let plaintext1 = "patient-123";
    let plaintext2 = "patient-456";
    
    let encrypted1 = encryption.encrypt_value(plaintext1).unwrap();
    let encrypted2 = encryption.encrypt_value(plaintext2).unwrap();
    
    // Different plaintexts should produce different ciphertexts
    assert_ne!(encrypted1, encrypted2);
}

#[test]
fn test_encrypt_same_value_produces_different_ciphertexts() {
    let config = test_config();
    let encryption = DatabaseEncryption::new(config).unwrap();
    
    let plaintext = "patient-123";
    
    let encrypted1 = encryption.encrypt_value(plaintext).unwrap();
    let encrypted2 = encryption.encrypt_value(plaintext).unwrap();
    
    // Same plaintext should produce different ciphertexts (due to random nonce)
    assert_ne!(encrypted1, encrypted2);
    
    // But both should decrypt to the same plaintext
    let decrypted1 = encryption.decrypt_value(&encrypted1).unwrap();
    let decrypted2 = encryption.decrypt_value(&encrypted2).unwrap();
    assert_eq!(decrypted1, plaintext);
    assert_eq!(decrypted2, plaintext);
}

#[test]
fn test_encrypt_empty_string() {
    let config = test_config();
    let encryption = DatabaseEncryption::new(config).unwrap();
    
    let plaintext = "";
    let encrypted = encryption.encrypt_value(plaintext).unwrap();
    let decrypted = encryption.decrypt_value(&encrypted).unwrap();
    
    assert_eq!(decrypted, plaintext);
}

#[test]
fn test_encrypt_unicode_text() {
    let config = test_config();
    let encryption = DatabaseEncryption::new(config).unwrap();
    
    let plaintext = "患者名: 田中太郎 🏥";
    let encrypted = encryption.encrypt_value(plaintext).unwrap();
    let decrypted = encryption.decrypt_value(&encrypted).unwrap();
    
    assert_eq!(decrypted, plaintext);
}

#[test]
fn test_encrypt_long_text() {
    let config = test_config();
    let encryption = DatabaseEncryption::new(config).unwrap();
    
    // Long medical note
    let plaintext = "Patient presents with acute myocardial infarction. \
                    ECG shows ST elevation in leads II, III, and aVF. \
                    Troponin levels elevated at 15.2 ng/mL. \
                    Immediate cardiac catheterization recommended. \
                    Patient consented to procedure.".repeat(10);
    
    let encrypted = encryption.encrypt_value(&plaintext).unwrap();
    let decrypted = encryption.decrypt_value(&encrypted).unwrap();
    
    assert_eq!(decrypted, plaintext);
}

#[test]
fn test_decrypt_invalid_format() {
    let config = test_config();
    let encryption = DatabaseEncryption::new(config).unwrap();
    
    // Missing version prefix
    let result = encryption.decrypt_value("invalid:format");
    assert!(result.is_ok()); // Returns as-is if not encrypted format
    
    // Invalid base64
    let result = encryption.decrypt_value("v1:!!!:!!!");
    assert!(matches!(result, Err(EncryptionError::InvalidFormat)));
}

#[test]
fn test_decrypt_wrong_key() {
    let config1 = test_config();
    let encryption1 = DatabaseEncryption::new(config1).unwrap();
    
    let plaintext = "secret-data";
    let encrypted = encryption1.encrypt_value(plaintext).unwrap();
    
    // Try to decrypt with different key
    let config2 = test_config(); // Different key
    let encryption2 = DatabaseEncryption::new(config2).unwrap();
    
    let result = encryption2.decrypt_value(&encrypted);
    assert!(matches!(result, Err(EncryptionError::DecryptionFailed)));
}

#[test]
fn test_encryption_disabled() {
    let mut config = test_config();
    config.enabled = false;
    let encryption = DatabaseEncryption::new(config).unwrap();
    
    let plaintext = "not-encrypted";
    
    // Should return plaintext as-is
    let encrypted = encryption.encrypt_value(plaintext).unwrap();
    assert_eq!(encrypted, plaintext);
    
    let decrypted = encryption.decrypt_value(&encrypted).unwrap();
    assert_eq!(decrypted, plaintext);
}

// =============================================================================
// UNIT TESTS - FIELD CONFIGURATION
// =============================================================================

#[test]
fn test_should_encrypt_field() {
    let mut config = test_config();
    config.field_mappings.insert(
        "users.ssn".to_string(),
        FieldEncryptionConfig {
            table: "users".to_string(),
            column: "ssn".to_string(),
            algorithm: EncryptionAlgorithm::AES256GCM,
            searchable: false,
            key_version: 1,
        },
    );
    
    let encryption = DatabaseEncryption::new(config).unwrap();
    
    assert!(encryption.should_encrypt("users", "ssn"));
    assert!(!encryption.should_encrypt("users", "email"));
    assert!(!encryption.should_encrypt("other_table", "ssn"));
}

#[test]
fn test_get_field_config() {
    let mut config = test_config();
    let field_config = FieldEncryptionConfig {
        table: "tokens".to_string(),
        column: "access_token".to_string(),
        algorithm: EncryptionAlgorithm::AES256GCM,
        searchable: false,
        key_version: 1,
    };
    config.field_mappings.insert("tokens.access_token".to_string(), field_config.clone());
    
    let encryption = DatabaseEncryption::new(config).unwrap();
    
    let retrieved = encryption.get_field_config("tokens", "access_token");
    assert!(retrieved.is_some());
    assert_eq!(retrieved.unwrap().table, "tokens");
    
    let missing = encryption.get_field_config("tokens", "refresh_token");
    assert!(missing.is_none());
}

#[test]
fn test_default_field_mappings() {
    let mappings = EncryptionConfig::default_field_mappings();
    
    // Verify key healthcare fields are included
    assert!(mappings.contains_key("users.ssn"));
    assert!(mappings.contains_key("tokens.access_token"));
    assert!(mappings.contains_key("tokens.refresh_token"));
    assert!(mappings.contains_key("credentials.mfa_secret"));
    assert!(mappings.contains_key("credentials.mfa_backup_codes"));
    assert!(mappings.contains_key("jwt_keys.private_key_pem"));
    assert!(mappings.contains_key("certificates.private_key_pem"));
    
    // Verify they're all AES-256-GCM
    for config in mappings.values() {
        assert_eq!(config.algorithm, EncryptionAlgorithm::AES256GCM);
        assert_eq!(config.key_version, 1);
    }
}

// =============================================================================
// UNIT TESTS - KEY MANAGEMENT
// =============================================================================

#[test]
fn test_encryption_from_master_key() {
    use base64::{Engine as _, engine::general_purpose::STANDARD as BASE64};
    
    let key_bytes = generate_test_key();
    let key_b64 = BASE64.encode(&key_bytes);
    
    let config = EncryptionConfig::from_master_key(&key_b64).unwrap();
    
    assert!(config.enabled);
    assert_eq!(config.master_key, key_bytes);
    assert_eq!(config.key_version, 1);
}

#[test]
fn test_encryption_from_invalid_base64() {
    let result = EncryptionConfig::from_master_key("not-valid-base64!!!");
    assert!(matches!(result, Err(EncryptionError::InvalidKey)));
}

#[test]
fn test_encryption_from_wrong_key_length() {
    use base64::{Engine as _, engine::general_purpose::STANDARD as BASE64};
    
    // Too short (16 bytes instead of 32)
    let short_key = vec![0u8; 16];
    let short_key_b64 = BASE64.encode(&short_key);
    
    let result = EncryptionConfig::from_master_key(&short_key_b64);
    assert!(matches!(result, Err(EncryptionError::InvalidKeyLength)));
}

#[test]
fn test_key_store_add_and_retrieve() {
    let mut store = EncryptionKeyStore::new();
    
    let key1 = generate_test_key();
    let key2 = generate_test_key();
    
    store.add_key(1, key1.clone()).unwrap();
    store.add_key(2, key2.clone()).unwrap();
    
    assert_eq!(store.get_key_by_version(1), Some(&key1));
    assert_eq!(store.get_key_by_version(2), Some(&key2));
    assert_eq!(store.get_key_by_version(3), None);
}

#[test]
fn test_key_store_rotation() {
    let mut store = EncryptionKeyStore::new();
    
    let key1 = generate_test_key();
    let key2 = generate_test_key();
    
    store.add_key(1, key1.clone()).unwrap();
    store.add_key(2, key2.clone()).unwrap();
    
    // Initially on version 1
    assert_eq!(store.current_version(), 1);
    assert_eq!(store.get_current_key(), Some(&key1));
    
    // Rotate to version 2
    store.rotate(2);
    assert_eq!(store.current_version(), 2);
    assert_eq!(store.get_current_key(), Some(&key2));
}

#[test]
fn test_key_store_invalid_key_length() {
    let mut store = EncryptionKeyStore::new();
    
    let short_key = vec![0u8; 16]; // Too short
    let result = store.add_key(1, short_key);
    
    assert!(matches!(result, Err(EncryptionError::InvalidKeyLength)));
}

// =============================================================================
// INTEGRATION TESTS - HEALTHCARE SCENARIOS
// =============================================================================

#[test]
fn test_encrypt_patient_ssn() {
    let config = test_config();
    let encryption = DatabaseEncryption::new(config).unwrap();
    
    let ssn = "123-45-6789";
    let encrypted = encryption.encrypt_value(ssn).unwrap();
    
    // Verify encrypted
    assert_ne!(encrypted, ssn);
    assert!(encrypted.contains("v1:"));
    
    // Verify can decrypt
    let decrypted = encryption.decrypt_value(&encrypted).unwrap();
    assert_eq!(decrypted, ssn);
}

#[test]
fn test_encrypt_access_token() {
    let config = test_config();
    let encryption = DatabaseEncryption::new(config).unwrap();
    
    let token = "eyJhbGciOiJIUzI1NiIsInR5cCI6IkpXVCJ9.eyJzdWIiOiIxMjM0NTY3ODkwIn0.abc123";
    let encrypted = encryption.encrypt_value(token).unwrap();
    let decrypted = encryption.decrypt_value(&encrypted).unwrap();
    
    assert_eq!(decrypted, token);
}

#[test]
fn test_encrypt_mfa_secret() {
    let config = test_config();
    let encryption = DatabaseEncryption::new(config).unwrap();
    
    let secret = "JBSWY3DPEHPK3PXP";
    let encrypted = encryption.encrypt_value(secret).unwrap();
    let decrypted = encryption.decrypt_value(&encrypted).unwrap();
    
    assert_eq!(decrypted, secret);
}

#[test]
fn test_encrypt_private_key_pem() {
    let config = test_config();
    let encryption = DatabaseEncryption::new(config).unwrap();
    
    let pem = "-----BEGIN PRIVATE KEY-----\n\
               MIIEvQIBADANBgkqhkiG9w0BAQEFAASCBKcwggSjAgEAAoIBAQC...\n\
               -----END PRIVATE KEY-----";
    
    let encrypted = encryption.encrypt_value(pem).unwrap();
    let decrypted = encryption.decrypt_value(&encrypted).unwrap();
    
    assert_eq!(decrypted, pem);
}

// =============================================================================
// PERFORMANCE TESTS
// =============================================================================

#[test]
fn test_performance_encryption() {
    use std::time::Instant;
    
    let config = test_config();
    let encryption = DatabaseEncryption::new(config).unwrap();
    
    let plaintext = "patient-data-123";
    let iterations = 1000;
    
    let start = Instant::now();
    for _ in 0..iterations {
        let _ = encryption.encrypt_value(plaintext).unwrap();
    }
    let elapsed = start.elapsed();
    
    let avg_time = elapsed.as_micros() / iterations;
    println!("Average encryption time: {}μs", avg_time);
    
    // Should be fast (< 100μs per encryption)
    assert!(avg_time < 100, "Encryption too slow: {}μs", avg_time);
}

#[test]
fn test_performance_decryption() {
    use std::time::Instant;
    
    let config = test_config();
    let encryption = DatabaseEncryption::new(config).unwrap();
    
    let plaintext = "patient-data-123";
    let encrypted = encryption.encrypt_value(plaintext).unwrap();
    let iterations = 1000;
    
    let start = Instant::now();
    for _ in 0..iterations {
        let _ = encryption.decrypt_value(&encrypted).unwrap();
    }
    let elapsed = start.elapsed();
    
    let avg_time = elapsed.as_micros() / iterations;
    println!("Average decryption time: {}μs", avg_time);
    
    // Should be fast (< 100μs per decryption)
    assert!(avg_time < 100, "Decryption too slow: {}μs", avg_time);
}

#[test]
fn test_performance_large_payload() {
    use std::time::Instant;
    
    let config = test_config();
    let encryption = DatabaseEncryption::new(config).unwrap();
    
    // 1MB medical record
    let large_plaintext = "A".repeat(1024 * 1024);
    
    let start = Instant::now();
    let encrypted = encryption.encrypt_value(&large_plaintext).unwrap();
    let encrypt_time = start.elapsed();
    
    let start = Instant::now();
    let decrypted = encryption.decrypt_value(&encrypted).unwrap();
    let decrypt_time = start.elapsed();
    
    println!("1MB encryption time: {:?}", encrypt_time);
    println!("1MB decryption time: {:?}", decrypt_time);
    
    assert_eq!(decrypted, large_plaintext);
    
    // Should handle 1MB in < 500ms (relaxed for CI environments)
    // Note: Performance can vary significantly based on hardware and system load
    assert!(encrypt_time.as_millis() < 500, "Large encryption too slow: {:?}", encrypt_time);
    assert!(decrypt_time.as_millis() < 500, "Large decryption too slow: {:?}", decrypt_time);
}

// =============================================================================
// EDGE CASES
// =============================================================================

#[test]
fn test_encrypt_special_characters() {
    let config = test_config();
    let encryption = DatabaseEncryption::new(config).unwrap();
    
    let special = "!@#$%^&*()_+-=[]{}|;':\",./<>?";
    let encrypted = encryption.encrypt_value(special).unwrap();
    let decrypted = encryption.decrypt_value(&encrypted).unwrap();
    
    assert_eq!(decrypted, special);
}

#[test]
fn test_encrypt_newlines_and_tabs() {
    let config = test_config();
    let encryption = DatabaseEncryption::new(config).unwrap();
    
    let text = "Line 1\nLine 2\tTabbed\r\nWindows Line";
    let encrypted = encryption.encrypt_value(text).unwrap();
    let decrypted = encryption.decrypt_value(&encrypted).unwrap();
    
    assert_eq!(decrypted, text);
}

#[test]
fn test_decrypt_plaintext_passthrough() {
    let config = test_config();
    let encryption = DatabaseEncryption::new(config).unwrap();
    
    // Plaintext that doesn't start with "v" should pass through
    let plaintext = "not-encrypted-yet";
    let result = encryption.decrypt_value(plaintext).unwrap();
    
    assert_eq!(result, plaintext);
}
