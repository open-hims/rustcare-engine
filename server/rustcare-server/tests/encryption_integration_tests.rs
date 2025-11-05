//! Integration tests for encryption across repositories
//!
//! These tests verify that:
//! 1. Sensitive data is encrypted before storage
//! 2. Encrypted data is automatically decrypted on retrieval
//! 3. Encryption works with different field types (tokens, keys, secrets)
//! 4. Encryption is transparent to application logic

use database_layer::{DatabaseEncryption, EncryptionConfig};

/// Helper to create a test encryption instance
fn create_test_encryption() -> DatabaseEncryption {
    // Generate a random 32-byte key for testing
    let master_key: [u8; 32] = rand::random();
    let master_key_b64 = base64::Engine::encode(&base64::engine::general_purpose::STANDARD, master_key);
    
    let config = EncryptionConfig::from_master_key(&master_key_b64)
        .expect("Failed to create encryption config");
    
    DatabaseEncryption::new(config).expect("Failed to create encryption instance")
}

#[test]
fn test_encryption_basic_flow() {
    let encryption = create_test_encryption();
    
    let plaintext = "secret_access_token_12345";
    
    // Encrypt
    let ciphertext = encryption.encrypt_value(plaintext)
        .expect("Failed to encrypt");
    
    // Verify it's actually encrypted (has version prefix)
    assert!(ciphertext.starts_with("v1:"), "Ciphertext should have v1: prefix");
    assert_ne!(ciphertext, plaintext, "Ciphertext should not equal plaintext");
    
    // Decrypt
    let decrypted = encryption.decrypt_value(&ciphertext)
        .expect("Failed to decrypt");
    
    assert_eq!(decrypted, plaintext, "Decrypted value should match original");
}

#[test]
fn test_encryption_different_plaintexts() {
    let encryption = create_test_encryption();
    
    let test_values = vec![
        "short",
        "a_longer_value_with_mixed_123_characters",
        "special!@#$%^&*()_+characters",
        "unicode_test_🔐🔑",
        "",
    ];
    
    for value in test_values {
        let encrypted = encryption.encrypt_value(value)
            .expect("Failed to encrypt");
        let decrypted = encryption.decrypt_value(&encrypted)
            .expect("Failed to decrypt");
        
        assert_eq!(decrypted, value, "Round-trip should preserve value");
    }
}

#[test]
fn test_encryption_nonce_randomization() {
    let encryption = create_test_encryption();
    
    let plaintext = "same_value_encrypted_twice";
    
    // Encrypt the same value twice
    let ciphertext1 = encryption.encrypt_value(plaintext)
        .expect("Failed to encrypt first time");
    let ciphertext2 = encryption.encrypt_value(plaintext)
        .expect("Failed to encrypt second time");
    
    // Ciphertexts should be different (due to random nonce)
    assert_ne!(ciphertext1, ciphertext2, "Same plaintext should produce different ciphertexts");
    
    // But both should decrypt to the same plaintext
    let decrypted1 = encryption.decrypt_value(&ciphertext1)
        .expect("Failed to decrypt first");
    let decrypted2 = encryption.decrypt_value(&ciphertext2)
        .expect("Failed to decrypt second");
    
    assert_eq!(decrypted1, plaintext);
    assert_eq!(decrypted2, plaintext);
}

#[test]
fn test_encryption_tampering_detection() {
    let encryption = create_test_encryption();
    
    let plaintext = "sensitive_data";
    let ciphertext = encryption.encrypt_value(plaintext)
        .expect("Failed to encrypt");
    
    // Tamper with the ciphertext by changing one character
    let mut tampered = ciphertext.clone();
    if let Some(last_char) = tampered.pop() {
        tampered.push(if last_char == 'a' { 'b' } else { 'a' });
    }
    
    // Decryption should fail on tampered data
    let result = encryption.decrypt_value(&tampered);
    assert!(result.is_err(), "Decryption should fail on tampered ciphertext");
}

#[test]
fn test_encryption_format_validation() {
    let encryption = create_test_encryption();
    
    // Invalid formats should fail
    let invalid_formats = vec![
        "plaintext_without_version",
        "v2:invalid:format",
        "v1:only_one_colon",
        "v1::empty_parts",
        ":missing:version",
    ];
    
    for invalid in invalid_formats {
        let result = encryption.decrypt_value(invalid);
        assert!(result.is_err(), "Should fail on invalid format: {}", invalid);
    }
}

#[test]
fn test_encryption_version_handling() {
    let encryption = create_test_encryption();
    
    let plaintext = "test_value";
    let encrypted = encryption.encrypt_value(plaintext)
        .expect("Failed to encrypt");
    
    // Should be encrypted with version 1
    assert!(encrypted.starts_with("v1:"), "Should use version 1");
    
    // Decryption should work with v1
    let decrypted = encryption.decrypt_value(&encrypted)
        .expect("Failed to decrypt v1");
    assert_eq!(decrypted, plaintext);
}

#[test]
fn test_encryption_empty_string() {
    let encryption = create_test_encryption();
    
    let empty = "";
    let encrypted = encryption.encrypt_value(empty)
        .expect("Failed to encrypt empty string");
    let decrypted = encryption.decrypt_value(&encrypted)
        .expect("Failed to decrypt empty string");
    
    assert_eq!(decrypted, empty, "Empty string should round-trip correctly");
}

#[test]
fn test_encryption_large_values() {
    let encryption = create_test_encryption();
    
    // Test with a large value (simulating a JWT or large token)
    let large_value = "a".repeat(10000);
    let encrypted = encryption.encrypt_value(&large_value)
        .expect("Failed to encrypt large value");
    let decrypted = encryption.decrypt_value(&encrypted)
        .expect("Failed to decrypt large value");
    
    assert_eq!(decrypted, large_value, "Large value should round-trip correctly");
}

#[test]
fn test_encryption_binary_safe() {
    let encryption = create_test_encryption();
    
    // Test with binary-like data (base64 encoded data)
    let binary_data = "data:image/png;base64,iVBORw0KGgoAAAANSUhEUgAAAAEAAAABCAYAAAAfFcSJAAAADUlEQVR42mNk+M9QDwADhgGAWjR9awAAAABJRU5ErkJggg==";
    let encrypted = encryption.encrypt_value(binary_data)
        .expect("Failed to encrypt binary data");
    let decrypted = encryption.decrypt_value(&encrypted)
        .expect("Failed to decrypt binary data");
    
    assert_eq!(decrypted, binary_data, "Binary data should round-trip correctly");
}

/// This test documents expected behavior when encryption is disabled
#[test]
fn test_encryption_disabled_behavior() {
    let config = EncryptionConfig::default(); // disabled by default
    let encryption = DatabaseEncryption::new(config)
        .expect("Failed to create encryption with default config");
    
    assert!(!encryption.is_enabled(), "Default config should have encryption disabled");
    
    let plaintext = "test_value";
    let result = encryption.encrypt_value(plaintext)
        .expect("Should not fail when disabled");
    
    // When disabled, should return plaintext unchanged
    assert_eq!(result, plaintext, "Disabled encryption should return plaintext");
}
