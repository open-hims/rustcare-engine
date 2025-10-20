use thiserror::Error;

#[derive(Error, Debug)]
pub enum IdentityError {
    #[error("User not found")]
    UserNotFound,
    
    #[error("Invalid credentials")]
    InvalidCredentials,
    
    #[error("User already exists")]
    UserAlreadyExists,
    
    #[error("Email already in use")]
    EmailAlreadyInUse,
    
    #[error("Username already in use")]
    UsernameAlreadyInUse,
    
    #[error("Invalid email format")]
    InvalidEmail,
    
    #[error("Password too weak")]
    WeakPassword,
    
    #[error("Session expired")]
    SessionExpired,
    
    #[error("Invalid token")]
    InvalidToken,
    
    #[error("Account not verified")]
    AccountNotVerified,
    
    #[error("Account disabled")]
    AccountDisabled,
    
    #[error("Database error: {0}")]
    DatabaseError(#[from] sqlx::Error),
    
    #[error("Hashing error")]
    HashingError,
    
    #[error("JWT error: {0}")]
    JwtError(String),
    
    #[error("Internal error: {0}")]
    InternalError(#[from] anyhow::Error),
}

pub type Result<T> = std::result::Result<T, IdentityError>;