use serde::{Deserialize, Serialize};
use uuid::Uuid;
use chrono::{DateTime, Utc};
use std::collections::HashMap;

/// OAuth 2.0 Authorization Server Configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OAuthConfig {
    pub issuer: String,
    pub authorization_endpoint: String,
    pub token_endpoint: String,
    pub userinfo_endpoint: String,
    pub jwks_uri: String,
    pub scopes_supported: Vec<String>,
    pub response_types_supported: Vec<String>,
    pub grant_types_supported: Vec<String>,
    pub token_endpoint_auth_methods_supported: Vec<String>,
}

/// OAuth Client Application
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OAuthClient {
    pub client_id: String,
    pub client_secret: String,
    pub name: String,
    pub description: Option<String>,
    pub redirect_uris: Vec<String>,
    pub grant_types: Vec<String>,
    pub scopes: Vec<String>,
    pub created_at: DateTime<Utc>,
    pub is_active: bool,
}

/// OAuth Authorization Code
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AuthorizationCode {
    pub code: String,
    pub client_id: String,
    pub user_id: Uuid,
    pub redirect_uri: String,
    pub scopes: Vec<String>,
    pub expires_at: DateTime<Utc>,
    pub created_at: DateTime<Utc>,
    pub code_challenge: Option<String>,
    pub code_challenge_method: Option<String>,
}

/// OAuth Access Token
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AccessToken {
    pub token: String,
    pub client_id: String,
    pub user_id: Uuid,
    pub scopes: Vec<String>,
    pub expires_at: DateTime<Utc>,
    pub created_at: DateTime<Utc>,
}

/// OAuth Refresh Token
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RefreshToken {
    pub token: String,
    pub client_id: String,
    pub user_id: Uuid,
    pub scopes: Vec<String>,
    pub expires_at: Option<DateTime<Utc>>,
    pub created_at: DateTime<Utc>,
}

/// Authorization Request
#[derive(Debug, Serialize, Deserialize)]
pub struct AuthorizationRequest {
    pub response_type: String,
    pub client_id: String,
    pub redirect_uri: String,
    pub scope: Option<String>,
    pub state: Option<String>,
    pub code_challenge: Option<String>,
    pub code_challenge_method: Option<String>,
}

/// Token Request
#[derive(Debug, Serialize, Deserialize)]
pub struct TokenRequest {
    pub grant_type: String,
    pub code: Option<String>,
    pub redirect_uri: Option<String>,
    pub client_id: String,
    pub client_secret: Option<String>,
    pub code_verifier: Option<String>,
    pub refresh_token: Option<String>,
    pub scope: Option<String>,
}

/// Token Response
#[derive(Debug, Serialize, Deserialize)]
pub struct TokenResponse {
    pub access_token: String,
    pub token_type: String,
    pub expires_in: Option<i64>,
    pub refresh_token: Option<String>,
    pub scope: Option<String>,
}

/// User Info Response
#[derive(Debug, Serialize, Deserialize)]
pub struct UserInfoResponse {
    pub sub: String,
    pub email: Option<String>,
    pub email_verified: Option<bool>,
    pub name: Option<String>,
    pub given_name: Option<String>,
    pub family_name: Option<String>,
    pub picture: Option<String>,
    pub locale: Option<String>,
    #[serde(flatten)]
    pub additional_claims: HashMap<String, serde_json::Value>,
}

/// External OAuth Provider Configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExternalProvider {
    pub name: String,
    pub client_id: String,
    pub client_secret: String,
    pub authorization_url: String,
    pub token_url: String,
    pub userinfo_url: String,
    pub scopes: Vec<String>,
    pub user_id_field: String,
    pub email_field: String,
    pub name_field: String,
}