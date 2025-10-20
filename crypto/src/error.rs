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
    
    #[error("Signature verification failed")]
    SignatureVerificationFailed,
    
    #[error("Hash computation failed: {0}")]
    HashFailed(String),
    
    #[error("Internal error: {0}")]
    InternalError(#[from] anyhow::Error),
}

pub type CryptoResult<T> = Result<T, CryptoError>;