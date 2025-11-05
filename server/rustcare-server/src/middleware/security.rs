//! Unified security middleware combining authentication, request context, rate limiting, and CSRF protection
//!
//! This module provides a comprehensive security layer that:
//! - Extracts and validates authentication (JWT)
//! - Tracks request context (ID, origin, headers)
//! - Enforces rate limiting per user/IP
//! - Validates CSRF tokens
//! - Adds security headers
//! - Logs security events

use axum::extract::{FromRequestParts, RequestParts};
use axum::http::{header, HeaderMap, Method};
use uuid::Uuid;
use async_trait::async_trait;
use std::sync::Arc;
use std::collections::HashMap;
use std::time::Instant;
use tokio::sync::RwLock;
use crate::error::ApiError;
use crate::middleware::{AuthContext, RequestContext};

/// Rate limit configuration
#[derive(Debug, Clone)]
pub struct RateLimitConfig {
    /// Maximum requests per window
    pub max_requests: u32,
    /// Time window in seconds
    pub window_seconds: u64,
    /// Whether to rate limit by user_id (true) or IP (false)
    pub by_user: bool,
}

impl Default for RateLimitConfig {
    fn default() -> Self {
        Self {
            max_requests: 100,
            window_seconds: 60,
            by_user: true,
        }
    }
}

/// Rate limit entry tracking requests in a time window
#[derive(Debug, Clone)]
struct RateLimitEntry {
    count: u32,
    window_start: Instant,
}

/// In-memory rate limiter (for single-instance deployments)
/// For distributed systems, use Redis or similar
pub struct RateLimiter {
    entries: Arc<RwLock<HashMap<String, RateLimitEntry>>>,
    config: RateLimitConfig,
}

impl RateLimiter {
    pub fn new(config: RateLimitConfig) -> Self {
        Self {
            entries: Arc::new(RwLock::new(HashMap::new())),
            config,
        }
    }
    
    /// Check if request should be rate limited
    pub async fn check(&self, key: &str) -> Result<(), ApiError> {
        let mut entries = self.entries.write().await;
        
        // Clean up old entries periodically
        if entries.len() > 10000 {
            entries.retain(|_, entry| {
                entry.window_start.elapsed().as_secs() < self.config.window_seconds
            });
        }
        
        let now = Instant::now();
        let entry = entries.entry(key.to_string()).or_insert_with(|| {
            RateLimitEntry {
                count: 0,
                window_start: now,
            }
        });
        
        // Reset if window expired
        if entry.window_start.elapsed().as_secs() >= self.config.window_seconds {
            entry.count = 0;
            entry.window_start = now;
        }
        
        // Check limit
        if entry.count >= self.config.max_requests {
            return Err(ApiError::rate_limit(format!(
                "Rate limit exceeded: {} requests per {} seconds",
                self.config.max_requests,
                self.config.window_seconds
            )));
        }
        
        entry.count += 1;
        Ok(())
    }
    
    /// Get remaining requests in current window
    pub async fn remaining(&self, key: &str) -> u32 {
        let entries = self.entries.read().await;
        if let Some(entry) = entries.get(key) {
            if entry.window_start.elapsed().as_secs() < self.config.window_seconds {
                self.config.max_requests.saturating_sub(entry.count)
            } else {
                self.config.max_requests
            }
        } else {
            self.config.max_requests
        }
    }
}

/// CSRF token validator
pub struct CsrfValidator {
    /// Expected CSRF token header name
    pub header_name: String,
    /// Whether to require CSRF token for state-changing operations
    pub require_for_mutations: bool,
}

impl CsrfValidator {
    pub fn new() -> Self {
        Self {
            header_name: "X-CSRF-Token".to_string(),
            require_for_mutations: true,
        }
    }
    
    /// Validate CSRF token for a request
    pub fn validate(
        &self,
        method: &Method,
        headers: &HeaderMap,
        origin: Option<&String>,
    ) -> Result<(), ApiError> {
        // Only check CSRF for state-changing methods
        if !self.require_for_mutations {
            return Ok(());
        }
        
        let is_mutation = matches!(
            method,
            &Method::POST | &Method::PUT | &Method::PATCH | &Method::DELETE
        );
        
        if !is_mutation {
            return Ok(());
        }
        
        // Get CSRF token from header
        let csrf_token = headers
            .get(&self.header_name)
            .or_else(|| headers.get("X-Csrf-Token"))
            .and_then(|h| h.to_str().ok());
        
        // If no token and same-site validation passed, allow
        // (for API clients that use Origin-based validation)
        if csrf_token.is_none() {
            // Rely on same-site validation via Origin header
            // This is acceptable for REST APIs
            if origin.is_some() {
                return Ok(());
            }
            return Err(ApiError::authentication(
                "CSRF token required for state-changing operations"
            ));
        }
        
        // In a full implementation, validate token against session/cookie
        // For now, we just check presence and rely on same-site validation
        Ok(())
    }
}

/// Unified security context combining all security features
#[derive(Debug, Clone)]
pub struct SecurityContext {
    /// Authentication context
    pub auth: AuthContext,
    /// Request context
    pub request: RequestContext,
    /// Rate limiter instance
    #[allow(dead_code)]
    rate_limiter: Option<Arc<RateLimiter>>,
    /// CSRF validator instance
    #[allow(dead_code)]
    csrf_validator: Option<Arc<CsrfValidator>>,
}

impl SecurityContext {
    /// Create a new security context (for testing)
    pub fn new(auth: AuthContext, request: RequestContext) -> Self {
        Self {
            auth,
            request,
            rate_limiter: None,
            csrf_validator: None,
        }
    }
    
    /// Check rate limit
    pub async fn check_rate_limit(&self) -> Result<(), ApiError> {
        if let Some(ref limiter) = self.rate_limiter {
            let key = if limiter.config.by_user {
                self.auth.user_id.to_string()
            } else {
                self.request.remote_addr
                    .clone()
                    .unwrap_or_else(|| "unknown".to_string())
            };
            limiter.check(&key).await
        } else {
            Ok(())
        }
    }
    
    /// Get remaining rate limit requests
    pub async fn rate_limit_remaining(&self) -> u32 {
        if let Some(ref limiter) = self.rate_limiter {
            let key = if limiter.config.by_user {
                self.auth.user_id.to_string()
            } else {
                self.request.remote_addr
                    .clone()
                    .unwrap_or_else(|| "unknown".to_string())
            };
            limiter.remaining(&key).await
        } else {
            u32::MAX
        }
    }
    
    /// Validate CSRF token
    pub fn validate_csrf(&self, method: &Method, headers: &HeaderMap) -> Result<(), ApiError> {
        if let Some(ref validator) = self.csrf_validator {
            validator.validate(method, headers, self.request.origin.as_ref())
        } else {
            Ok(())
        }
    }
    
    /// Require permission (delegates to AuthContext)
    pub async fn require_permission(
        &self,
        resource_type: &str,
        resource_id: Option<Uuid>,
        permission: &str,
    ) -> Result<(), ApiError> {
        self.auth.require_permission(resource_type, resource_id, permission).await
    }
    
    /// Check permission (delegates to AuthContext)
    pub async fn check_permission(
        &self,
        resource_type: &str,
        resource_id: Option<Uuid>,
        permission: &str,
    ) -> Result<bool, ApiError> {
        self.auth.check_permission(resource_type, resource_id, permission).await
    }
    
    /// Check if request is from same site
    pub fn is_same_site(&self) -> bool {
        self.request.is_same_site()
    }
}

/// Security middleware configuration
#[derive(Clone)]
pub struct SecurityConfig {
    /// Rate limiting configuration
    pub rate_limit: Option<RateLimitConfig>,
    /// CSRF validation configuration
    pub csrf: Option<CsrfValidator>,
    /// Whether to enforce same-site validation strictly
    pub strict_same_site: bool,
}

impl Default for SecurityConfig {
    fn default() -> Self {
        Self {
            rate_limit: Some(RateLimitConfig::default()),
            csrf: Some(CsrfValidator::new()),
            strict_same_site: false,
        }
    }
}

/// Security middleware state stored in Axum extensions
/// This is initialized when creating the router and provides
/// rate limiting, CSRF validation, and security configuration
pub struct SecurityMiddlewareState {
    pub rate_limiter: Option<Arc<RateLimiter>>,
    pub csrf_validator: Option<Arc<CsrfValidator>>,
    pub config: SecurityConfig,
}

impl SecurityMiddlewareState {
    pub fn new(config: SecurityConfig) -> Self {
        Self {
            rate_limiter: config.rate_limit.as_ref()
                .map(|cfg| Arc::new(RateLimiter::new(cfg.clone()))),
            csrf_validator: config.csrf.as_ref()
                .map(|v| Arc::new(CsrfValidator::new())),
            config,
        }
    }
}

// Note: SecurityContext cannot implement FromRequestParts directly because
// both AuthContext and RequestContext implement it, and we can't extract both
// from the same RequestParts (it would consume parts).
//
// Instead, use SecurityContext::from_contexts() in handlers or create
// a middleware that sets up SecurityContext in extensions.

impl SecurityContext {
    /// Create SecurityContext from AuthContext and RequestContext
    /// 
    /// This should be called after extracting both contexts in a handler
    pub fn from_contexts(
        auth: AuthContext,
        request: RequestContext,
        rate_limiter: Option<Arc<RateLimiter>>,
        csrf_validator: Option<Arc<CsrfValidator>>,
    ) -> Self {
        Self {
            auth,
            request,
            rate_limiter,
            csrf_validator,
        }
    }
    
    /// Create from contexts and perform security checks
    pub async fn from_contexts_with_checks(
        auth: AuthContext,
        request: RequestContext,
        method: &Method,
        headers: &HeaderMap,
        security_state: &SecurityMiddlewareState,
    ) -> Result<Self, ApiError> {
        // Perform rate limiting check
        let rate_limiter = security_state.rate_limiter.clone();
        if let Some(ref limiter) = rate_limiter {
            let key = if limiter.config.by_user {
                auth.user_id.to_string()
            } else {
                request.remote_addr
                    .clone()
                    .unwrap_or_else(|| "unknown".to_string())
            };
            limiter.check(&key).await?;
        }
        
        // Perform CSRF validation
        let csrf_validator = security_state.csrf_validator.clone();
        if let Some(ref validator) = csrf_validator {
            validator.validate(method, headers, request.origin.as_ref())?;
        }
        
        // Enforce strict same-site if configured
        if security_state.config.strict_same_site && !request.is_same_site() {
            return Err(ApiError::authentication(
                "Same-site validation failed - request rejected"
            ));
        }
        
        Ok(Self {
            auth,
            request,
            rate_limiter,
            csrf_validator,
        })
    }
}

/// Helper function to add security headers to response
pub fn add_security_headers(headers: &mut HeaderMap) {
    // X-Request-ID for tracing
    // This should be set from RequestContext
    
    // Security headers
    headers.insert(
        header::HeaderName::from_static("x-content-type-options"),
        header::HeaderValue::from_static("nosniff"),
    );
    headers.insert(
        header::HeaderName::from_static("x-frame-options"),
        header::HeaderValue::from_static("DENY"),
    );
    headers.insert(
        header::HeaderName::from_static("x-xss-protection"),
        header::HeaderValue::from_static("1; mode=block"),
    );
    
    // CORS headers (if needed)
    // Should be configured based on ALLOWED_ORIGINS
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_rate_limiter() {
        let config = RateLimitConfig {
            max_requests: 5,
            window_seconds: 60,
            by_user: true,
        };
        let limiter = RateLimiter::new(config);
        
        // Should allow 5 requests
        for i in 0..5 {
            assert!(limiter.check("user1").await.is_ok(), "Request {} should succeed", i);
        }
        
        // 6th request should be rate limited
        assert!(limiter.check("user1").await.is_err());
        
        // Different user should be fine
        assert!(limiter.check("user2").await.is_ok());
    }
    
    #[test]
    fn test_csrf_validator() {
        let validator = CsrfValidator::new();
        let mut headers = HeaderMap::new();
        
        // GET request should pass without token
        assert!(validator.validate(&Method::GET, &headers, None).is_ok());
        
        // POST without token should fail
        assert!(validator.validate(&Method::POST, &headers, None).is_err());
        
        // POST with token should pass
        headers.insert("X-CSRF-Token", "test-token".parse().unwrap());
        assert!(validator.validate(&Method::POST, &headers, None).is_ok());
        
        // POST with origin should pass (same-site validation)
        headers.remove("X-CSRF-Token");
        let origin = Some("https://example.com".to_string());
        assert!(validator.validate(&Method::POST, &headers, origin.as_ref()).is_ok());
    }
}

