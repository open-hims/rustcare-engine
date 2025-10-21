/// Authentication provider implementations
/// 
/// Each provider implements the `AuthProvider` trait and handles
/// a specific authentication method.

pub mod email_password;
pub mod oauth;
pub mod certificate;

pub use email_password::EmailPasswordProvider;
pub use oauth::OAuthProvider;
pub use certificate::CertificateProvider;

use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Authentication result
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AuthResult {
    /// User ID
    pub user_id: String,
    
    /// User email
    pub email: String,
    
    /// Authentication method used
    pub auth_method: String,
    
    /// User permissions/roles
    pub permissions: Vec<String>,
    
    /// Additional user claims
    pub claims: HashMap<String, serde_json::Value>,
    
    /// Certificate serial (if cert auth)
    pub cert_serial: Option<String>,
    
    /// OAuth provider (if OAuth auth)
    pub oauth_provider: Option<String>,
}

/// Common trait for all authentication providers
#[async_trait]
pub trait Provider: Send + Sync {
    /// Authenticate a user and return auth result
    async fn authenticate(&self, credentials: &Credentials) -> anyhow::Result<AuthResult>;
    
    /// Validate if a user exists
    async fn user_exists(&self, identifier: &str) -> anyhow::Result<bool>;
    
    /// Get provider name
    fn name(&self) -> &str;
}

/// Generic credentials enum for all auth methods
#[derive(Debug, Clone)]
pub enum Credentials {
    EmailPassword {
        email: String,
        password: String,
    },
    EmailPasswordMfa {
        email: String,
        password: String,
        totp_token: String,
    },
    OAuth {
        provider: String,
        code: String,
        state: String,
    },
    Certificate {
        cert_pem: String,
        cert_serial: String,
        subject_dn: String,
    },
    RefreshToken {
        token: String,
    },
}
