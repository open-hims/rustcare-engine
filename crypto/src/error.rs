use thiserror::Error;

#[derive(Error, Debug)]
pub enum CryptoError {
    #[error("Encryption failed: {0}")]
    EncryptionFailed(String),
    
    #[error("Decryption failed: {0}")]
    DecryptionFailed(String),
    
    #[error("Key generation failed: {0}")]
    KeyGenerationFailed(String),
    
    #[error("Invalid key: {0}")]
    InvalidKey(String),
    
    #[error("Invalid key length: expected {expected}, got {got}")]
    InvalidKeyLength { expected: usize, got: usize },
    
    #[error("Unsupported key version {version}, only version {supported} is supported")]
    UnsupportedKeyVersion { version: u32, supported: u32 },
    
    #[error("Invalid encrypted data format: {0}")]
    InvalidFormat(String),
    
    #[error("Invalid nonce length: {0}")]
    InvalidNonce(String),
    
    #[error("Invalid UTF-8 in decrypted data: {0}")]
    InvalidUtf8(String),
    
    #[error("Signature verification failed: {0}")]
    SignatureVerificationFailed(String),
    
    #[error("Hash computation failed: {0}")]
    HashFailed(String),
    
    #[error("Key derivation failed: {0}")]
    KeyDerivationFailed(String),
    
    #[error("Configuration error: {0}")]
    Configuration(String),
    
    #[error("Internal error: {0}")]
    InternalError(#[from] anyhow::Error),
}

pub type CryptoResult<T> = Result<T, CryptoError>;