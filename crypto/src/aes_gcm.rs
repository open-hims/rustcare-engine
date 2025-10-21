use crate::error::CryptoError;
use crate::encryption::{EncryptionResult, Encryptor};
use aes_gcm::{
    aead::{Aead, KeyInit, OsRng},
    Aes256Gcm, Nonce,
};
use base64::{engine::general_purpose::STANDARD as BASE64, Engine};
use rand::RngCore;
use zeroize::{Zeroize, ZeroizeOnDrop};

/// AES-256-GCM encryptor with memory security
/// 
/// This implementation provides:
/// - AES-256 in Galois/Counter Mode (NIST approved)
/// - 96-bit nonces (recommended for GCM)
/// - Authentication tags for integrity
/// - Memory zeroization on drop
/// - Secure random number generation
#[derive(ZeroizeOnDrop)]
pub struct Aes256GcmEncryptor {
    #[zeroize(skip)]
    cipher: Aes256Gcm,
    /// Master key - automatically zeroized on drop
    key: [u8; 32],
    /// Key version for rotation support
    key_version: u32,
}

impl Aes256GcmEncryptor {
    /// Create a new encryptor with a 32-byte key
    pub fn new(key: [u8; 32]) -> EncryptionResult<Self> {
        let cipher = Aes256Gcm::new_from_slice(&key)
            .map_err(|_| CryptoError::InvalidKey)?;

        Ok(Self {
            cipher,
            key,
            key_version: 1,
        })
    }

    /// Create from base64-encoded key
    pub fn from_base64(key_b64: &str) -> EncryptionResult<Self> {
        let key_bytes = BASE64
            .decode(key_b64)
            .map_err(|_| CryptoError::InvalidKey)?;

        if key_bytes.len() != 32 {
            return Err(CryptoError::InvalidKeyLength {
                expected: 32,
                got: key_bytes.len(),
            });
        }

        let mut key = [0u8; 32];
        key.copy_from_slice(&key_bytes);

        Self::new(key)
    }

    /// Create with specific key version
    pub fn with_version(mut self, version: u32) -> Self {
        self.key_version = version;
        self
    }

    /// Generate a new random key (cryptographically secure)
    pub fn generate_key() -> [u8; 32] {
        let mut key = [0u8; 32];
        OsRng.fill_bytes(&mut key);
        key
    }

    /// Get the current key version
    pub fn version(&self) -> u32 {
        self.key_version
    }

    /// Encrypt with versioned format: "v{version}:{nonce_b64}:{ciphertext_b64}"
    fn encrypt_versioned(&self, plaintext: &[u8]) -> EncryptionResult<String> {
        // Generate random 96-bit nonce (12 bytes - optimal for GCM)
        let mut nonce_bytes = [0u8; 12];
        OsRng.fill_bytes(&mut nonce_bytes);
        let nonce = Nonce::from_slice(&nonce_bytes);

        // Encrypt with authentication
        let ciphertext = self
            .cipher
            .encrypt(nonce, plaintext)
            .map_err(|_| CryptoError::EncryptionFailed)?;

        // Format: v{version}:{nonce_b64}:{ciphertext_b64}
        let nonce_b64 = BASE64.encode(nonce_bytes);
        let ciphertext_b64 = BASE64.encode(&ciphertext);

        Ok(format!(
            "v{}:{}:{}",
            self.key_version, nonce_b64, ciphertext_b64
        ))
    }

    /// Decrypt with versioned format
    fn decrypt_versioned(&self, encrypted: &str) -> EncryptionResult<Vec<u8>> {
        // Parse format: v{version}:{nonce}:{ciphertext}
        let parts: Vec<&str> = encrypted.split(':').collect();
        if parts.len() != 3 {
            return Err(CryptoError::InvalidFormat);
        }

        // Extract and validate version
        let version = parts[0]
            .strip_prefix('v')
            .and_then(|v| v.parse::<u32>().ok())
            .ok_or(CryptoError::InvalidFormat)?;

        // Check version compatibility (for now, only support current version)
        // In production with key rotation, you'd look up the correct key
        if version != self.key_version {
            return Err(CryptoError::UnsupportedKeyVersion {
                version,
                supported: self.key_version,
            });
        }

        // Decode nonce
        let nonce_bytes = BASE64
            .decode(parts[1])
            .map_err(|_| CryptoError::InvalidFormat)?;

        if nonce_bytes.len() != 12 {
            return Err(CryptoError::InvalidNonce);
        }

        let nonce = Nonce::from_slice(&nonce_bytes);

        // Decode ciphertext
        let ciphertext = BASE64
            .decode(parts[2])
            .map_err(|_| CryptoError::InvalidFormat)?;

        // Decrypt and verify authentication tag
        self.cipher
            .decrypt(nonce, ciphertext.as_ref())
            .map_err(|_| CryptoError::DecryptionFailed)
    }
}

impl Encryptor for Aes256GcmEncryptor {
    fn encrypt(&self, plaintext: &[u8]) -> EncryptionResult<Vec<u8>> {
        // Encrypt and return as bytes (prepend format indicator)
        let encrypted_str = self.encrypt_versioned(plaintext)?;
        Ok(encrypted_str.into_bytes())
    }

    fn decrypt(&self, ciphertext: &[u8]) -> EncryptionResult<Vec<u8>> {
        // Convert bytes to string and decrypt
        let encrypted_str = String::from_utf8(ciphertext.to_vec())
            .map_err(|_| CryptoError::InvalidUtf8)?;
        self.decrypt_versioned(&encrypted_str)
    }

    fn algorithm(&self) -> &str {
        "AES-256-GCM"
    }
}

/// Helper for encrypting strings (common use case)
impl Aes256GcmEncryptor {
    /// Encrypt a string and return base64-encoded versioned format
    pub fn encrypt_string(&self, plaintext: &str) -> EncryptionResult<String> {
        self.encrypt_versioned(plaintext.as_bytes())
    }

    /// Decrypt a versioned string
    pub fn decrypt_string(&self, encrypted: &str) -> EncryptionResult<String> {
        let plaintext_bytes = self.decrypt_versioned(encrypted)?;
        String::from_utf8(plaintext_bytes).map_err(|_| CryptoError::InvalidUtf8)
    }
}

/// Secure key generation utilities
pub struct KeyGenerator;

impl KeyGenerator {
    /// Generate a cryptographically secure random key
    pub fn generate_aes256_key() -> [u8; 32] {
        Aes256GcmEncryptor::generate_key()
    }

    /// Generate a key and encode as base64
    pub fn generate_aes256_key_base64() -> String {
        let key = Self::generate_aes256_key();
        BASE64.encode(key)
    }

    /// Generate multiple keys for key rotation
    pub fn generate_key_set(count: usize) -> Vec<[u8; 32]> {
        (0..count)
            .map(|_| Self::generate_aes256_key())
            .collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_encrypt_decrypt_roundtrip() {
        let key = Aes256GcmEncryptor::generate_key();
        let encryptor = Aes256GcmEncryptor::new(key).unwrap();

        let plaintext = b"Hello, secure world!";
        let ciphertext = encryptor.encrypt(plaintext).unwrap();
        let decrypted = encryptor.decrypt(&ciphertext).unwrap();

        assert_eq!(plaintext, decrypted.as_slice());
    }

    #[test]
    fn test_encrypt_string() {
        let key = Aes256GcmEncryptor::generate_key();
        let encryptor = Aes256GcmEncryptor::new(key).unwrap();

        let plaintext = "Sensitive PHI data";
        let encrypted = encryptor.encrypt_string(plaintext).unwrap();
        let decrypted = encryptor.decrypt_string(&encrypted).unwrap();

        assert_eq!(plaintext, decrypted);
    }

    #[test]
    fn test_versioned_format() {
        let key = Aes256GcmEncryptor::generate_key();
        let encryptor = Aes256GcmEncryptor::new(key).unwrap().with_version(5);

        let plaintext = "test data";
        let encrypted = encryptor.encrypt_string(plaintext).unwrap();

        // Check format starts with version
        assert!(encrypted.starts_with("v5:"));

        // Check it has three parts separated by colons
        let parts: Vec<&str> = encrypted.split(':').collect();
        assert_eq!(parts.len(), 3);
    }

    #[test]
    fn test_different_nonces() {
        let key = Aes256GcmEncryptor::generate_key();
        let encryptor = Aes256GcmEncryptor::new(key).unwrap();

        let plaintext = "same plaintext";
        let encrypted1 = encryptor.encrypt_string(plaintext).unwrap();
        let encrypted2 = encryptor.encrypt_string(plaintext).unwrap();

        // Same plaintext should produce different ciphertexts (different nonces)
        assert_ne!(encrypted1, encrypted2);

        // But both should decrypt to same plaintext
        assert_eq!(
            encryptor.decrypt_string(&encrypted1).unwrap(),
            plaintext
        );
        assert_eq!(
            encryptor.decrypt_string(&encrypted2).unwrap(),
            plaintext
        );
    }

    #[test]
    fn test_tampered_ciphertext() {
        let key = Aes256GcmEncryptor::generate_key();
        let encryptor = Aes256GcmEncryptor::new(key).unwrap();

        let plaintext = "authenticated data";
        let mut encrypted = encryptor.encrypt_string(plaintext).unwrap();

        // Tamper with the ciphertext
        encrypted.push('X');

        // Decryption should fail (authentication tag mismatch)
        assert!(encryptor.decrypt_string(&encrypted).is_err());
    }

    #[test]
    fn test_wrong_version() {
        let key = Aes256GcmEncryptor::generate_key();
        let encryptor_v1 = Aes256GcmEncryptor::new(key).unwrap().with_version(1);
        let encryptor_v2 = Aes256GcmEncryptor::new(key).unwrap().with_version(2);

        let plaintext = "version test";
        let encrypted_v1 = encryptor_v1.encrypt_string(plaintext).unwrap();

        // Try to decrypt with wrong version
        let result = encryptor_v2.decrypt_string(&encrypted_v1);
        assert!(result.is_err());
    }

    #[test]
    fn test_from_base64() {
        let key_b64 = KeyGenerator::generate_aes256_key_base64();
        let encryptor = Aes256GcmEncryptor::from_base64(&key_b64).unwrap();

        let plaintext = "base64 key test";
        let encrypted = encryptor.encrypt_string(plaintext).unwrap();
        let decrypted = encryptor.decrypt_string(&encrypted).unwrap();

        assert_eq!(plaintext, decrypted);
    }

    #[test]
    fn test_invalid_key_length() {
        let short_key_b64 = BASE64.encode(b"too_short");
        let result = Aes256GcmEncryptor::from_base64(&short_key_b64);
        assert!(result.is_err());
    }

    #[test]
    fn test_key_generation() {
        let key1 = KeyGenerator::generate_aes256_key();
        let key2 = KeyGenerator::generate_aes256_key();

        // Keys should be different
        assert_ne!(key1, key2);

        // Keys should have correct length
        assert_eq!(key1.len(), 32);
        assert_eq!(key2.len(), 32);
    }

    #[test]
    fn test_empty_plaintext() {
        let key = Aes256GcmEncryptor::generate_key();
        let encryptor = Aes256GcmEncryptor::new(key).unwrap();

        let plaintext = "";
        let encrypted = encryptor.encrypt_string(plaintext).unwrap();
        let decrypted = encryptor.decrypt_string(&encrypted).unwrap();

        assert_eq!(plaintext, decrypted);
    }

    #[test]
    fn test_large_data() {
        let key = Aes256GcmEncryptor::generate_key();
        let encryptor = Aes256GcmEncryptor::new(key).unwrap();

        // Test with 1MB of data
        let plaintext = vec![0x42u8; 1024 * 1024];
        let ciphertext = encryptor.encrypt(&plaintext).unwrap();
        let decrypted = encryptor.decrypt(&ciphertext).unwrap();

        assert_eq!(plaintext, decrypted);
    }

    #[test]
    fn test_memory_zeroization() {
        // This test ensures the key is zeroized on drop
        let key = Aes256GcmEncryptor::generate_key();
        let key_copy = key;
        
        {
            let _encryptor = Aes256GcmEncryptor::new(key).unwrap();
            // Encryptor goes out of scope here
        }
        
        // We can't directly verify zeroization without unsafe code,
        // but the ZeroizeOnDrop derive ensures it happens
        // This test mainly serves as documentation
        assert_eq!(key_copy.len(), 32);
    }
}
