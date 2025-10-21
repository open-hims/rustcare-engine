use thiserror::Error;

#[derive(Error, Debug)]
pub enum CryptoError {
    #[error("Encryption failed")]
    EncryptionFailed,
    
    #[error("Decryption failed")]
    DecryptionFailed,
    
    #[error("Key generation failed: {0}")]
    KeyGenerationFailed(String),
    
    #[error("Invalid key")]
    InvalidKey,
    
    #[error("Invalid key length: expected {expected}, got {got}")]
    InvalidKeyLength { expected: usize, got: usize },
    
    #[error("Unsupported key version {version}, only version {supported} is supported")]
    UnsupportedKeyVersion { version: u32, supported: u32 },
    
    #[error("Invalid encrypted data format")]
    InvalidFormat,
    
    #[error("Invalid nonce length")]
    InvalidNonce,
    
    #[error("Invalid UTF-8 in decrypted data")]
    InvalidUtf8,
    
    #[error("Signature verification failed")]
    SignatureVerificationFailed,
    
    #[error("Hash computation failed: {0}")]
    HashFailed(String),
    
    #[error("Key derivation failed: {0}")]
    KeyDerivationFailed(String),
    
    #[error("Internal error: {0}")]
    InternalError(#[from] anyhow::Error),
}

pub type CryptoResult<T> = Result<T, CryptoError>;