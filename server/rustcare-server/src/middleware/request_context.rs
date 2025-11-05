//! Request context middleware for security and tracing
//!
//! This module provides:
//! - Request ID extraction/generation
//! - Same-site security checks
//! - Request metadata collection
//! - Security headers validation

use axum::extract::{FromRequestParts, RequestParts};
use axum::http::{header, request::Parts, HeaderMap};
use serde::{Deserialize, Serialize};
use uuid::Uuid;
use async_trait::async_trait;
use crate::error::ApiError;
use std::time::{SystemTime, UNIX_EPOCH};

/// Request context containing security and tracing information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RequestContext {
    /// Unique request ID for tracing
    pub request_id: String,
    /// Origin header value (for same-site checks)
    pub origin: Option<String>,
    /// Referer header value
    pub referer: Option<String>,
    /// User-Agent header value
    pub user_agent: Option<String>,
    /// Remote IP address
    pub remote_addr: Option<String>,
    /// Request timestamp
    pub timestamp: u64,
    /// Same-site validation result
    pub same_site_valid: bool,
}

impl RequestContext {
    /// Create a new request context with generated request ID
    pub fn new() -> Self {
        Self {
            request_id: Uuid::new_v4().to_string(),
            origin: None,
            referer: None,
            user_agent: None,
            remote_addr: None,
            timestamp: SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap()
                .as_secs(),
            same_site_valid: true, // Default to true, will be validated
        }
    }
    
    /// Create from headers with same-site validation
    pub fn from_headers(headers: &HeaderMap, remote_addr: Option<String>) -> Self {
        let origin = headers
            .get(header::ORIGIN)
            .and_then(|h| h.to_str().ok())
            .map(|s| s.to_string());
        
        let referer = headers
            .get(header::REFERER)
            .and_then(|h| h.to_str().ok())
            .map(|s| s.to_string());
        
        let user_agent = headers
            .get(header::USER_AGENT)
            .and_then(|h| h.to_str().ok())
            .map(|s| s.to_string());
        
        // Extract request ID from headers or generate new one
        let request_id = headers
            .get("X-Request-ID")
            .or_else(|| headers.get("x-request-id"))
            .and_then(|h| h.to_str().ok())
            .map(|s| s.to_string())
            .unwrap_or_else(|| Uuid::new_v4().to_string());
        
        // Perform same-site validation
        let same_site_valid = Self::validate_same_site(&origin, &referer);
        
        Self {
            request_id,
            origin,
            referer,
            user_agent,
            remote_addr,
            timestamp: SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap()
                .as_secs(),
            same_site_valid,
        }
    }
    
    /// Validate same-site security
    /// 
    /// Checks that Origin and Referer headers match expected patterns
    /// This helps prevent CSRF attacks
    fn validate_same_site(origin: &Option<String>, referer: &Option<String>) -> bool {
        // Get allowed origins from environment or config
        let allowed_origins = std::env::var("ALLOWED_ORIGINS")
            .unwrap_or_else(|_| "localhost,127.0.0.1".to_string())
            .split(',')
            .map(|s| s.trim().to_string())
            .collect::<Vec<_>>();
        
        // If no origin/referer, allow (might be same-origin request)
        if origin.is_none() && referer.is_none() {
            return true;
        }
        
        // Check origin if present
        if let Some(ref origin_str) = origin {
            // Extract host from origin (e.g., "https://example.com" -> "example.com")
            let origin_host = origin_str
                .trim_start_matches("http://")
                .trim_start_matches("https://")
                .split('/')
                .next()
                .unwrap_or(origin_str);
            
            // Check if origin matches allowed origins
            let is_allowed = allowed_origins.iter().any(|allowed| {
                origin_host == allowed || origin_host.ends_with(&format!(".{}", allowed))
            });
            
            if !is_allowed {
                tracing::warn!(
                    origin = %origin_str,
                    "Same-site validation failed: origin not in allowed list"
                );
                return false;
            }
        }
        
        // Check referer if present and origin is not
        if origin.is_none() {
            if let Some(ref referer_str) = referer {
                let referer_host = referer_str
                    .trim_start_matches("http://")
                    .trim_start_matches("https://")
                    .split('/')
                    .next()
                    .unwrap_or(referer_str);
                
                let is_allowed = allowed_origins.iter().any(|allowed| {
                    referer_host == allowed || referer_host.ends_with(&format!(".{}", allowed))
                });
                
                if !is_allowed {
                    tracing::warn!(
                        referer = %referer_str,
                        "Same-site validation failed: referer not in allowed list"
                    );
                    return false;
                }
            }
        }
        
        true
    }
    
    /// Check if request is from same site (stricter check)
    pub fn is_same_site(&self) -> bool {
        self.same_site_valid
    }
}

impl Default for RequestContext {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl<S> FromRequestParts<S> for RequestContext
where
    S: Send + Sync,
{
    type Rejection = ApiError;
    
    async fn from_request_parts(
        parts: &mut RequestParts<'_, S>,
        _state: &S,
    ) -> Result<Self, Self::Rejection> {
        let headers = parts.headers
            .as_ref()
            .ok_or_else(|| ApiError::internal("No headers available"))?;
        
        // Extract remote address from extensions or headers
        let remote_addr = parts
            .extensions
            .get::<axum::extract::ConnectInfo<std::net::SocketAddr>>()
            .map(|ci| ci.0.ip().to_string())
            .or_else(|| {
                headers
                    .get("X-Forwarded-For")
                    .and_then(|h| h.to_str().ok())
                    .map(|s| s.split(',').next().unwrap_or("").trim().to_string())
            })
            .or_else(|| {
                headers
                    .get("X-Real-IP")
                    .and_then(|h| h.to_str().ok())
                    .map(|s| s.to_string())
            });
        
        let ctx = RequestContext::from_headers(headers, remote_addr);
        
        // If same-site validation fails, log warning but don't reject
        // (allows API clients that don't send Origin/Referer)
        if !ctx.same_site_valid {
            tracing::warn!(
                request_id = %ctx.request_id,
                origin = ?ctx.origin,
                referer = ?ctx.referer,
                "Same-site validation failed - request may be vulnerable to CSRF"
            );
        }
        
        Ok(ctx)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use axum::http::HeaderValue;

    #[test]
    fn test_request_context_new() {
        let ctx = RequestContext::new();
        assert!(!ctx.request_id.is_empty());
        assert!(ctx.same_site_valid);
    }
    
    #[test]
    fn test_same_site_validation_allowed() {
        std::env::set_var("ALLOWED_ORIGINS", "localhost,example.com");
        
        let origin = Some("https://localhost".to_string());
        let referer = None;
        assert!(RequestContext::validate_same_site(&origin, &referer));
        
        let origin = Some("https://example.com".to_string());
        assert!(RequestContext::validate_same_site(&origin, &referer));
        
        std::env::remove_var("ALLOWED_ORIGINS");
    }
    
    #[test]
    fn test_same_site_validation_blocked() {
        std::env::set_var("ALLOWED_ORIGINS", "localhost");
        
        let origin = Some("https://evil.com".to_string());
        let referer = None;
        assert!(!RequestContext::validate_same_site(&origin, &referer));
        
        std::env::remove_var("ALLOWED_ORIGINS");
    }
    
    #[test]
    fn test_no_origin_referer_allowed() {
        let origin = None;
        let referer = None;
        assert!(RequestContext::validate_same_site(&origin, &referer));
    }
}

