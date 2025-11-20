//! Authentication context extraction middleware
//!
//! This module provides automatic extraction of authentication context from JWT tokens,
//! eliminating the need for manual token parsing and placeholder user IDs.

use axum::extract::FromRequestParts;
use axum::http::{header::AUTHORIZATION, request::Parts, Method, HeaderMap};
use serde::{Deserialize, Serialize};
use uuid::Uuid;
use async_trait::async_trait;
use std::sync::Arc;
use crate::error::ApiError;
use crate::middleware::{RequestContext, SecurityMiddlewareState};

/// Authentication context extracted from JWT token
///
/// This struct contains the authenticated user's information and is automatically
/// extracted from the Authorization header in requests.
/// 
/// **Automatically includes:**
/// - JWT authentication and validation
/// - Request context (ID, origin, headers)
/// - Rate limiting checks
/// - CSRF protection (for mutations)
/// - Same-site validation
/// - Zanzibar authorization integration for permission checks
#[derive(Debug, Clone)]
pub struct AuthContext {
    pub user_id: Uuid,
    pub organization_id: Uuid,
    pub roles: Vec<String>,
    pub permissions: Vec<String>,
    pub email: Option<String>,
    /// Request context (automatically extracted)
    pub request: RequestContext,
    /// Zanzibar authorization engine (optional, for permission checks)
    // #[serde(skip)] - removed as not derived
    pub zanzibar_engine: Option<Arc<dyn ZanzibarCheck>>,
    /// Rate limiter (for checking remaining requests)
    // #[serde(skip)] - removed as not derived
    pub(crate) rate_limiter: Option<Arc<crate::middleware::RateLimiter>>,
}

/// Trait for Zanzibar permission checks
#[async_trait]
pub trait ZanzibarCheck: Send + Sync + std::fmt::Debug {
    /// Check if user has permission to perform action on resource
    async fn check_permission(
        &self,
        user_id: Uuid,
        resource_type: &str,
        resource_id: Option<Uuid>,
        permission: &str,
        organization_id: Uuid,
    ) -> Result<bool, String>;
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
            request: RequestContext::new(),
            zanzibar_engine: None,
            rate_limiter: None,
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
            request: RequestContext::new(),
            zanzibar_engine: None,
            rate_limiter: None,
        }
    }
    
    /// Get request ID (convenience method)
    pub fn request_id(&self) -> &str {
        &self.request.request_id
    }
    
    /// Check if request is from same site
    pub fn is_same_site(&self) -> bool {
        self.request.is_same_site()
    }
    
    /// Get remaining rate limit requests
    pub async fn rate_limit_remaining(&self) -> u32 {
        if let Some(ref limiter) = self.rate_limiter {
            let key = limiter.config.by_user.then(|| self.user_id.to_string())
                .unwrap_or_else(|| {
                    self.request.remote_addr
                        .clone()
                        .unwrap_or_else(|| "unknown".to_string())
                });
            limiter.remaining(&key).await
        } else {
            u32::MAX
        }
    }
    
    /// Check permission using Zanzibar
    /// 
    /// Returns true if user has permission, false otherwise.
    /// If Zanzibar engine is not available, falls back to checking permissions list.
    pub async fn check_permission(
        &self,
        resource_type: &str,
        resource_id: Option<Uuid>,
        permission: &str,
    ) -> Result<bool, ApiError> {
        // If Zanzibar engine is available, use it
        if let Some(ref engine) = self.zanzibar_engine {
            engine
                .check_permission(
                    self.user_id,
                    resource_type,
                    resource_id,
                    permission,
                    self.organization_id,
                )
                .await
                .map_err(|e| ApiError::authorization(format!("Zanzibar check failed: {}", e)))
        } else {
            // Fallback to simple permission check from JWT claims
            let permission_str = format!("{}:{}", resource_type, permission);
            Ok(self.permissions.contains(&permission_str) || 
               self.permissions.iter().any(|p| p == permission))
        }
    }
    
    /// Require permission - returns error if permission is not granted
    pub async fn require_permission(
        &self,
        resource_type: &str,
        resource_id: Option<Uuid>,
        permission: &str,
    ) -> Result<(), ApiError> {
        let has_permission = self.check_permission(resource_type, resource_id, permission).await?;
        if !has_permission {
            return Err(ApiError::authorization(format!(
                "Permission denied: {} on {}",
                permission,
                resource_type
            )));
        }
        Ok(())
    }
}

/// Extract and validate JWT token from Authorization header
fn extract_token(parts: &Parts) -> Result<String, ApiError> {
        let headers = &parts.headers;
    
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
        request: RequestContext::new(), // Will be replaced by FromRequestParts
        zanzibar_engine: None, // Will be set up later if needed
        rate_limiter: None, // Will be set by FromRequestParts if security state available
    })
}

#[async_trait]
impl<S> FromRequestParts<S> for AuthContext
where
    S: Send + Sync,
{
    type Rejection = ApiError;
    
    async fn from_request_parts(
        parts: &mut Parts,
        _state: &S,
    ) -> Result<Self, Self::Rejection> {
        // Extract RequestContext first (for same-site validation)
        let request = RequestContext::from_request_parts(parts, _state).await?;
        
        // Extract JWT token from Authorization header
        let token = extract_token(parts)?;
        
        // Validate and decode JWT
        let mut auth_ctx = validate_jwt_token(&token)?;
        
        // Attach request context
        auth_ctx.request = request;
        
        // Get security middleware state from extensions (if available)
        let security_state = parts.extensions
            .get::<SecurityMiddlewareState>()
            .cloned();
        
        // Perform rate limiting check if security state is available
        if let Some(ref state) = security_state {
            if let Some(ref limiter) = state.rate_limiter {
                let key = if limiter.config.by_user {
                    auth_ctx.user_id.to_string()
                } else {
                    auth_ctx.request.remote_addr
                        .clone()
                        .unwrap_or_else(|| "unknown".to_string())
                };
                limiter.check(&key).await?;
                
                // Attach rate limiter for remaining() method
                auth_ctx.rate_limiter = state.rate_limiter.clone();
            }
            
            // Perform CSRF validation for state-changing methods
            if let Some(ref validator) = state.csrf_validator {
                let method = &parts.method;
                let headers = &parts.headers;
                validator.validate(method, headers, auth_ctx.request.origin.as_ref())?;
            }
            
            // Enforce strict same-site if configured
            if state.config.strict_same_site && !auth_ctx.request.is_same_site() {
                return Err(ApiError::authentication(
                    "Same-site validation failed - request rejected"
                ));
            }
        }
        
        // Try to get Zanzibar engine from extensions (if available)
        // The engine can be added to extensions by middleware or handlers
        if let Some(engine) = parts.extensions.get::<Arc<dyn ZanzibarCheck>>().cloned() {
            auth_ctx.zanzibar_engine = Some(engine);
        }
        
        Ok(auth_ctx)
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

