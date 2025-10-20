use thiserror::Error;

#[derive(Error, Debug)]
pub enum OAuthError {
    #[error("Invalid client")]
    InvalidClient,
    
    #[error("Invalid grant")]
    InvalidGrant,
    
    #[error("Invalid request: {0}")]
    InvalidRequest(String),
    
    #[error("Invalid scope")]
    InvalidScope,
    
    #[error("Unauthorized client")]
    UnauthorizedClient,
    
    #[error("Unsupported grant type")]
    UnsupportedGrantType,
    
    #[error("Unsupported response type")]
    UnsupportedResponseType,
    
    #[error("Access denied")]
    AccessDenied,
    
    #[error("Authorization code expired")]
    CodeExpired,
    
    #[error("Invalid authorization code")]
    InvalidCode,
    
    #[error("Invalid redirect URI")]
    InvalidRedirectUri,
    
    #[error("PKCE verification failed")]
    PkceVerificationFailed,
    
    #[error("Token expired")]
    TokenExpired,
    
    #[error("Invalid token")]
    InvalidToken,
    
    #[error("External provider error: {0}")]
    ExternalProviderError(String),
    
    #[error("HTTP error: {0}")]
    HttpError(#[from] reqwest::Error),
    
    #[error("JSON error: {0}")]
    JsonError(#[from] serde_json::Error),
    
    #[error("JWT error: {0}")]
    JwtError(#[from] jsonwebtoken::errors::Error),
    
    #[error("Identity error: {0}")]
    IdentityError(#[from] auth_identity::IdentityError),
    
    #[error("Internal error: {0}")]
    InternalError(#[from] anyhow::Error),
}

pub type Result<T> = std::result::Result<T, OAuthError>;