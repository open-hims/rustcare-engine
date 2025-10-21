use crate::error::CryptoError;
use async_trait::async_trait;

/// Result type for encryption operations
pub type EncryptionResult<T> = Result<T, CryptoError>;

/// Trait for encryption/decryption operations
#[async_trait]
pub trait Encryptor: Send + Sync {
    /// Encrypt data
    fn encrypt(&self, plaintext: &[u8]) -> EncryptionResult<Vec<u8>>;

    /// Decrypt data
    fn decrypt(&self, ciphertext: &[u8]) -> EncryptionResult<Vec<u8>>;

    /// Get the encryption algorithm name
    fn algorithm(&self) -> &str;
}

/// No-op encryptor for testing/development
pub struct NoOpEncryptor;

impl NoOpEncryptor {
    pub fn new() -> Self {
        Self
    }
}

impl Default for NoOpEncryptor {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl Encryptor for NoOpEncryptor {
    fn encrypt(&self, plaintext: &[u8]) -> EncryptionResult<Vec<u8>> {
        Ok(plaintext.to_vec())
    }

    fn decrypt(&self, ciphertext: &[u8]) -> EncryptionResult<Vec<u8>> {
        Ok(ciphertext.to_vec())
    }

    fn algorithm(&self) -> &str {
        "none"
    }
}
