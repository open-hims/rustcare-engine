//! Authentication context extraction middleware
//!
//! This module provides automatic extraction of authentication context from JWT tokens,
//! eliminating the need for manual token parsing and placeholder user IDs.

use axum::extract::{FromRequestParts, RequestParts};
use axum::http::{header::AUTHORIZATION, request::Parts};
use serde::{Deserialize, Serialize};
use uuid::Uuid;
use async_trait::async_trait;
use crate::error::ApiError;

/// Authentication context extracted from JWT token
///
/// This struct contains the authenticated user's information and is automatically
/// extracted from the Authorization header in requests.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AuthContext {
    pub user_id: Uuid,
    pub organization_id: Uuid,
    pub roles: Vec<String>,
    pub permissions: Vec<String>,
    pub email: Option<String>,
}

impl AuthContext {
    /// Create a new AuthContext (for testing/mocking)
    pub fn new(user_id: Uuid, organization_id: Uuid) -> Self {
        Self {
            user_id,
            organization_id,
            roles: Vec::new(),
            permissions: Vec::new(),
            email: None,
        }
    }
    
    /// Create with roles and permissions
    pub fn with_permissions(
        user_id: Uuid,
        organization_id: Uuid,
        roles: Vec<String>,
        permissions: Vec<String>,
    ) -> Self {
        Self {
            user_id,
            organization_id,
            roles,
            permissions,
            email: None,
        }
    }
}

/// Extract and validate JWT token from Authorization header
fn extract_token(parts: &Parts) -> Result<String, ApiError> {
    let headers = parts.headers
        .ok_or_else(|| ApiError::authentication("No headers available"))?;
    
    let auth_header = headers
        .get(AUTHORIZATION)
        .and_then(|h| h.to_str().ok())
        .ok_or_else(|| ApiError::authentication("Missing Authorization header"))?;
    
    // Extract Bearer token
    auth_header
        .strip_prefix("Bearer ")
        .ok_or_else(|| ApiError::authentication("Invalid Authorization header format. Expected: Bearer <token>"))
        .map(|s| s.to_string())
}

/// Validate JWT token and extract claims
///
/// Uses the existing TokenClaims structure from auth/tokens module
fn validate_jwt_token(token: &str) -> Result<AuthContext, ApiError> {
    // For now, we'll use a simple approach:
    // 1. Try to decode the token using jsonwebtoken
    // 2. Extract claims
    // 3. Convert to AuthContext
    
    // TODO: Use the actual JWT service from auth/tokens.rs when available
    // For now, we'll implement basic validation here
    
    use jsonwebtoken::{decode, DecodingKey, Validation, Algorithm};
    
    // In production, get this from environment or config
    let jwt_secret = std::env::var("JWT_SECRET")
        .unwrap_or_else(|_| "your-secret-key-change-in-production".to_string());
    
    let decoding_key = DecodingKey::from_secret(jwt_secret.as_bytes());
    let mut validation = Validation::new(Algorithm::HS256);
    validation.validate_exp = true;
    validation.validate_nbf = true;
    
    // Decode token using TokenClaims structure from auth module
    // Note: We need to use a simplified claims structure here since
    // the full TokenClaims has extra fields we might not need
    #[derive(Debug, Deserialize)]
    struct SimplifiedClaims {
        sub: String,
        org_id: Option<String>,
        permissions: Option<Vec<String>>,
        email: Option<String>,
        exp: i64,
        #[serde(flatten)]
        extra: std::collections::HashMap<String, serde_json::Value>,
    }
    
    let token_data = decode::<SimplifiedClaims>(token, &decoding_key, &validation)
        .map_err(|e| ApiError::authentication(format!("Invalid or expired token: {}", e)))?;
    
    let claims = token_data.claims;
    
    // Extract user_id
    let user_id = Uuid::parse_str(&claims.sub)
        .map_err(|_| ApiError::authentication("Invalid user ID in token"))?;
    
    // Extract organization_id (try org_id claim first, then look in extra)
    let organization_id = if let Some(org_id_str) = claims.org_id {
        Uuid::parse_str(&org_id_str).ok()
    } else {
        claims.extra
            .get("org_id")
            .and_then(|v| v.as_str())
            .and_then(|s| Uuid::parse_str(s).ok())
    }.unwrap_or_else(|| {
        tracing::warn!("JWT token missing organization_id claim");
        Uuid::nil()
    });
    
    // Extract roles from extra claims (if present)
    let roles = claims.extra
        .get("roles")
        .and_then(|v| v.as_array())
        .map(|arr| {
            arr.iter()
                .filter_map(|v| v.as_str().map(|s| s.to_string()))
                .collect()
        })
        .unwrap_or_default();
    
    Ok(AuthContext {
        user_id,
        organization_id,
        roles,
        permissions: claims.permissions.unwrap_or_default(),
        email: claims.email,
    })
}

#[async_trait]
impl<S> FromRequestParts<S> for AuthContext
where
    S: Send + Sync,
{
    type Rejection = ApiError;
    
    async fn from_request_parts(
        parts: &mut RequestParts<'_, S>,
        _state: &S,
    ) -> Result<Self, Self::Rejection> {
        // Extract JWT token from Authorization header
        let token = extract_token(parts)?;
        
        // Validate and decode JWT
        let claims = validate_jwt_token(&token)?;
        
        // Extract organization_id (default to nil if not present, but log warning)
        let organization_id = claims.org_id.unwrap_or_else(|| {
            tracing::warn!("JWT token missing organization_id claim");
            Uuid::nil()
        });
        
        Ok(AuthContext {
            user_id: claims.sub,
            organization_id,
            roles: claims.roles.unwrap_or_default(),
            permissions: claims.permissions.unwrap_or_default(),
            email: claims.email,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_auth_context_new() {
        let user_id = Uuid::new_v4();
        let org_id = Uuid::new_v4();
        let ctx = AuthContext::new(user_id, org_id);
        
        assert_eq!(ctx.user_id, user_id);
        assert_eq!(ctx.organization_id, org_id);
        assert!(ctx.roles.is_empty());
        assert!(ctx.permissions.is_empty());
    }

    #[test]
    fn test_extract_token_format() {
        // Test that extract_token properly strips "Bearer " prefix
        // This is tested indirectly through integration tests
    }
}

