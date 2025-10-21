/// OAuth2/OIDC authentication provider
/// 
/// Implements SSO authentication with support for:
/// - OAuth2 authorization code flow (RFC 6749)
/// - OIDC (OpenID Connect)
/// - PKCE (Proof Key for Code Exchange - RFC 7636)
/// - Multiple providers (Google, Azure AD, Okta, GitHub)
/// - State parameter validation (CSRF protection)
/// - Account linking and user provisioning

use super::{AuthResult, Credentials, Provider};
use crate::auth::config::OAuthProviderConfig;
use crate::auth::db::{OAuthRepository, UserRepository};
use crate::auth::models::{OAuthAccount, User, UserStatus};
use anyhow::{anyhow, Context, Result};
use async_trait::async_trait;
use chrono::Utc;
use oauth2::{
    basic::BasicClient, reqwest::async_http_client, AuthUrl, AuthorizationCode, ClientId,
    ClientSecret, CsrfToken, PkceCodeChallenge, PkceCodeVerifier, RedirectUrl, Scope,
    TokenResponse as OAuth2TokenResponse, TokenUrl,
};
use reqwest::Client as HttpClient;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use sqlx::PgPool;
use std::collections::HashMap;
use std::sync::Arc;
use sqlx::types::Uuid;

/// OAuth provider names
pub mod providers {
    pub const GOOGLE: &str = "google";
    pub const AZURE_AD: &str = "azure_ad";
    pub const OKTA: &str = "okta";
    pub const GITHUB: &str = "github";
}

/// PKCE state storage (in production, use Redis)
#[derive(Debug, Clone)]
pub struct PkceState {
    pub verifier: String,
    pub created_at: chrono::DateTime<chrono::Utc>,
}

pub struct OAuthProvider {
    provider_name: String,
    config: OAuthProviderConfig,
    oauth_client: BasicClient,
    http_client: HttpClient,
    user_repo: Arc<UserRepository>,
    oauth_repo: Arc<OAuthRepository>,
    // In production, this would be Redis
    pkce_cache: Arc<tokio::sync::RwLock<HashMap<String, PkceState>>>,
}

impl OAuthProvider {
    /// Create a new OAuth provider
    pub fn new(
        provider_name: String,
        config: OAuthProviderConfig,
        pool: Arc<PgPool>,
    ) -> Result<Self> {
        // Validate URLs
        let auth_url = AuthUrl::new(config.authorization_url.clone())
            .context("Invalid authorization endpoint URL")?;
        
        let token_url = TokenUrl::new(config.token_url.clone())
            .context("Invalid token endpoint URL")?;
        
        // Use a default redirect URI if not set (should be configured via OAuth config)
        let redirect_uri = format!("http://localhost:8080/auth/oauth/{}/callback", provider_name);
        let redirect_url = RedirectUrl::new(redirect_uri)
            .context("Invalid redirect URI")?;
        
        // Create OAuth2 client
        let oauth_client = BasicClient::new(
            ClientId::new(config.client_id.clone()),
            Some(ClientSecret::new(config.client_secret.clone())),
            auth_url,
            Some(token_url),
        )
        .set_redirect_uri(redirect_url);
        
        // Create HTTP client with timeout
        let http_client = HttpClient::builder()
            .timeout(std::time::Duration::from_secs(30))
            .build()
            .context("Failed to create HTTP client")?;
        
        Ok(Self {
            provider_name,
            config,
            oauth_client,
            http_client,
            user_repo: Arc::new(UserRepository::new(crate::auth::db::DbPool::new((*pool).clone()))),
            oauth_repo: Arc::new(OAuthRepository::new(crate::auth::db::DbPool::new((*pool).clone()))),
            pkce_cache: Arc::new(tokio::sync::RwLock::new(HashMap::new())),
        })
    }
    
    /// Generate authorization URL with PKCE and state
    pub async fn get_authorization_url(&self) -> Result<(String, String, String)> {
        // Generate PKCE challenge and verifier
        let (pkce_challenge, pkce_verifier) = PkceCodeChallenge::new_random_sha256();
        
        // Generate state for CSRF protection
        let (auth_url, csrf_state) = self.oauth_client
            .authorize_url(CsrfToken::new_random)
            .set_pkce_challenge(pkce_challenge)
            .add_scopes(self.config.scopes.iter().map(|s| Scope::new(s.clone())))
            .url();
        
        let state = csrf_state.secret().clone();
        let verifier = pkce_verifier.secret().clone();
        
        // Store PKCE verifier (indexed by state)
        {
            let mut cache = self.pkce_cache.write().await;
            cache.insert(state.clone(), PkceState {
                verifier: verifier.clone(),
                created_at: Utc::now(),
            });
        }
        
        // Clean up old PKCE states (older than 10 minutes)
        self.cleanup_old_pkce_states().await;
        
        Ok((auth_url.to_string(), state, verifier))
    }
    
    /// Handle OAuth callback and exchange code for tokens
    pub async fn handle_callback(
        &self,
        code: &str,
        state: &str,
    ) -> Result<OAuthTokens> {
        // Retrieve PKCE verifier from cache
        let pkce_state = {
            let mut cache = self.pkce_cache.write().await;
            cache.remove(state)
                .ok_or_else(|| anyhow!("Invalid or expired state parameter"))?
        };
        
        // Check state is not too old (10 minutes max)
        let age = Utc::now() - pkce_state.created_at;
        if age.num_minutes() > 10 {
            return Err(anyhow!("State parameter has expired"));
        }
        
        // Exchange authorization code for tokens
        let token_result = self.oauth_client
            .exchange_code(AuthorizationCode::new(code.to_string()))
            .set_pkce_verifier(PkceCodeVerifier::new(pkce_state.verifier))
            .request_async(async_http_client)
            .await
            .context("Failed to exchange authorization code for tokens")?;
        
        Ok(OAuthTokens {
            access_token: token_result.access_token().secret().clone(),
            refresh_token: token_result.refresh_token().map(|t| t.secret().clone()),
            expires_in: token_result.expires_in().map(|d| d.as_secs()),
            id_token: None, // EmptyExtraTokenFields doesn't have id_token, would need custom type
        })
    }
    
    /// Fetch user info from OIDC userinfo endpoint
    pub async fn fetch_userinfo(&self, access_token: &str) -> Result<OAuthUserInfo> {
        let userinfo_url = self.config.userinfo_url.as_ref()
            .ok_or_else(|| anyhow!("Provider does not have userinfo endpoint configured"))?;
        
        let response = self.http_client
            .get(userinfo_url)
            .bearer_auth(access_token)
            .send()
            .await
            .context("Failed to fetch user info")?;
        
        if !response.status().is_success() {
            return Err(anyhow!("Userinfo endpoint returned error: {}", response.status()));
        }
        
        let userinfo: OAuthUserInfo = response.json()
            .await
            .context("Failed to parse userinfo response")?;
        
        Ok(userinfo)
    }
    
    /// Find or create user from OAuth account
    async fn find_or_create_user(&self, userinfo: &OAuthUserInfo) -> Result<User> {
        // Check if OAuth account already exists
        if let Some(oauth_account) = self.oauth_repo
            .find_by_provider_and_subject(&self.provider_name, &userinfo.sub)
            .await? {
            // Get existing user
            let user = self.user_repo
                .find_by_id(oauth_account.user_id)
                .await?
                .ok_or_else(|| anyhow!("User not found for OAuth account"))?;
            
            return Ok(user);
        }
        
        // Check if user exists with same email (for account linking)
        let user = if let Some(email) = &userinfo.email {
            if let Some(existing_user) = self.user_repo.find_by_email(email).await? {
                // Link OAuth account to existing user
                existing_user
            } else {
                // Create new user
                self.create_user_from_oauth(userinfo).await?
            }
        } else {
            // No email provided, create new user
            self.create_user_from_oauth(userinfo).await?
        };
        
        // Create OAuth account link
        self.oauth_repo.create(
            user.id,
            &self.provider_name,
            &userinfo.sub,
            userinfo.email.as_deref(),
            None, // access_token
            None, // refresh_token
            None, // id_token
            None, // token_expires_at
            None, // provider_data
            None, // scopes
        ).await?;
        
        Ok(user)
    }
    
    /// Create a new user from OAuth userinfo
    async fn create_user_from_oauth(&self, userinfo: &OAuthUserInfo) -> Result<User> {
        let email = userinfo.email.as_ref()
            .ok_or_else(|| anyhow!("Email not provided by OAuth provider"))?;
        
        let full_name = userinfo.name.clone();
        let display_name = userinfo.given_name.clone()
            .or_else(|| userinfo.name.clone());
        
        let mut user = self.user_repo.create(
            email,
            full_name.as_deref(),
            display_name.as_deref(),
        ).await?;
        
        // Mark email as verified if provider says so
        if userinfo.email_verified.unwrap_or(false) {
            self.user_repo.verify_email(user.id).await?;
            // Re-fetch user to get updated status
            user = self.user_repo.find_by_id(user.id).await?
                .ok_or_else(|| anyhow!("User not found after verification"))?;
        }
        
        Ok(user)
    }
    
    /// Update OAuth account tokens
    async fn update_oauth_tokens(
        &self,
        oauth_account_id: Uuid,
        tokens: &OAuthTokens,
    ) -> Result<()> {
        let expires_at = tokens.expires_in.map(|secs| {
            Utc::now() + chrono::Duration::seconds(secs as i64)
        });
        
        self.oauth_repo.update_tokens(
            oauth_account_id,
            Some(&tokens.access_token),
            tokens.refresh_token.as_deref(),
            tokens.id_token.as_deref(),
            expires_at,
        ).await?;
        
        Ok(())
    }
    
    /// Cleanup old PKCE states
    async fn cleanup_old_pkce_states(&self) {
        let mut cache = self.pkce_cache.write().await;
        let now = Utc::now();
        
        cache.retain(|_, state| {
            let age = now - state.created_at;
            age.num_minutes() <= 10
        });
    }
    
    /// Get user permissions (placeholder)
    async fn get_user_permissions(&self, _user_id: Uuid) -> Result<Vec<String>> {
        // TODO: Implement permission lookup from permission system
        Ok(vec![
            "auth:basic".to_string(),
            "profile:read".to_string(),
            "profile:write".to_string(),
        ])
    }
}

#[async_trait]
impl Provider for OAuthProvider {
    async fn authenticate(&self, credentials: &Credentials) -> Result<AuthResult> {
        match credentials {
            Credentials::OAuth { provider, code, state } => {
                // Validate provider name
                if provider != &self.provider_name {
                    return Err(anyhow!("Invalid OAuth provider"));
                }
                
                // Handle OAuth callback and exchange code for tokens
                let tokens = self.handle_callback(code, state).await?;
                
                // Fetch user info from provider
                let userinfo = self.fetch_userinfo(&tokens.access_token).await?;
                
                // Find or create user
                let user = self.find_or_create_user(&userinfo).await?;
                
                // Check user status
                if user.status != UserStatus::Active {
                    return Err(anyhow!("Account is not active. Status: {:?}", user.status));
                }
                
                // Update OAuth tokens in database
                self.update_oauth_tokens(user.id, &tokens).await?;
                
                // Get user permissions
                let permissions = self.get_user_permissions(user.id).await?;
                
                // Build auth result
                let mut claims = HashMap::new();
                claims.insert("user_id".to_string(), serde_json::json!(user.id.to_string()));
                claims.insert("email".to_string(), serde_json::json!(user.email.clone()));
                claims.insert("oauth_sub".to_string(), serde_json::json!(userinfo.sub.clone()));
                
                if let Some(full_name) = &user.full_name {
                    claims.insert("full_name".to_string(), serde_json::json!(full_name.clone()));
                }
                
                Ok(AuthResult {
                    user_id: user.id.to_string(),
                    email: user.email.clone(),
                    auth_method: format!("oauth_{}", self.provider_name),
                    permissions,
                    claims,
                    cert_serial: None,
                    oauth_provider: Some(self.provider_name.clone()),
                })
            }
            
            _ => Err(anyhow!("Invalid credentials type for OAuth provider")),
        }
    }
    
    async fn user_exists(&self, identifier: &str) -> Result<bool> {
        // Check by email (most OAuth providers provide email)
        Ok(self.user_repo.find_by_email(identifier).await?.is_some())
    }
    
    fn name(&self) -> &str {
        &self.provider_name
    }
}

// =============================================================================
// HELPER STRUCTS
// =============================================================================

/// OAuth tokens received from provider
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OAuthTokens {
    pub access_token: String,
    pub refresh_token: Option<String>,
    pub expires_in: Option<u64>,
    pub id_token: Option<String>,
}

/// OIDC user info (standardized claims)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OAuthUserInfo {
    pub sub: String,  // Subject (unique user ID from provider)
    pub email: Option<String>,
    pub email_verified: Option<bool>,
    pub name: Option<String>,
    pub given_name: Option<String>,
    pub family_name: Option<String>,
    pub picture: Option<String>,
    pub locale: Option<String>,
    #[serde(flatten)]
    pub extra: HashMap<String, serde_json::Value>,
}

// =============================================================================
// PROVIDER-SPECIFIC CONFIGURATIONS
// =============================================================================

impl OAuthProvider {
    /// Create Google OAuth provider
    pub fn google(config: OAuthProviderConfig, pool: Arc<PgPool>) -> Result<Self> {
        Self::new(providers::GOOGLE.to_string(), config, pool)
    }
    
    /// Create Azure AD OAuth provider
    pub fn azure_ad(config: OAuthProviderConfig, pool: Arc<PgPool>) -> Result<Self> {
        Self::new(providers::AZURE_AD.to_string(), config, pool)
    }
    
    /// Create Okta OAuth provider
    pub fn okta(config: OAuthProviderConfig, pool: Arc<PgPool>) -> Result<Self> {
        Self::new(providers::OKTA.to_string(), config, pool)
    }
    
    /// Create GitHub OAuth provider
    pub fn github(config: OAuthProviderConfig, pool: Arc<PgPool>) -> Result<Self> {
        Self::new(providers::GITHUB.to_string(), config, pool)
    }
}

// =============================================================================
// UTILITY FUNCTIONS
// =============================================================================

/// Generate a secure random state parameter
pub fn generate_state() -> String {
    use rand::Rng;
    use base64::{Engine, engine::general_purpose::URL_SAFE_NO_PAD};
    let mut rng = rand::thread_rng();
    let random_bytes: Vec<u8> = (0..32).map(|_| rng.gen()).collect();
    URL_SAFE_NO_PAD.encode(&random_bytes)
}

/// Hash state parameter for storage (to prevent leakage)
pub fn hash_state(state: &str) -> String {
    let mut hasher = Sha256::new();
    hasher.update(state.as_bytes());
    hex::encode(hasher.finalize())
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_state_generation() {
        let state1 = generate_state();
        let state2 = generate_state();
        
        assert!(!state1.is_empty());
        assert!(!state2.is_empty());
        assert_ne!(state1, state2); // Should be unique
    }
    
    #[test]
    fn test_state_hashing() {
        let state = "test_state_123";
        let hash1 = hash_state(state);
        let hash2 = hash_state(state);
        
        assert_eq!(hash1, hash2); // Same input = same hash
        assert_eq!(hash1.len(), 64); // SHA-256 = 32 bytes = 64 hex chars
    }
    
    #[test]
    fn test_oauth_userinfo_deserialization() {
        let json = r#"{
            "sub": "123456789",
            "email": "user@example.com",
            "email_verified": true,
            "name": "John Doe",
            "given_name": "John",
            "family_name": "Doe",
            "picture": "https://example.com/photo.jpg",
            "custom_field": "custom_value"
        }"#;
        
        let userinfo: OAuthUserInfo = serde_json::from_str(json).unwrap();
        
        assert_eq!(userinfo.sub, "123456789");
        assert_eq!(userinfo.email, Some("user@example.com".to_string()));
        assert_eq!(userinfo.email_verified, Some(true));
        assert_eq!(userinfo.name, Some("John Doe".to_string()));
        assert!(userinfo.extra.contains_key("custom_field"));
    }
}
