//! Convenience extractors for security contexts
//!
//! These extractors make it easy to use security features in handlers
//! without manually combining AuthContext and RequestContext.

use axum::extract::{FromRequestParts, RequestParts};
use axum::http::{Method, HeaderMap};
use async_trait::async_trait;
use crate::error::ApiError;
use crate::middleware::{AuthContext, RequestContext, SecurityContext, SecurityMiddlewareState};

/// Extractor that automatically creates SecurityContext with all checks
/// 
/// Usage:
/// ```rust
/// pub async fn handler(
///     security: SecureContext,
///     // ... other params
/// ) -> Result<...> {
///     security.require_permission("patient", Some(id), "view").await?;
/// }
/// ```
pub struct SecureContext(pub SecurityContext);

#[async_trait]
impl<S> FromRequestParts<S> for SecureContext
where
    S: Send + Sync,
{
    type Rejection = ApiError;
    
    async fn from_request_parts(
        parts: &mut RequestParts<'_, S>,
        _state: &S,
    ) -> Result<Self, Self::Rejection> {
        // Extract AuthContext and RequestContext
        let auth = AuthContext::from_request_parts(parts, _state).await?;
        let request = RequestContext::from_request_parts(parts, _state).await?;
        
        // Get security middleware state from extensions
        let security_state = parts.extensions
            .get::<SecurityMiddlewareState>()
            .ok_or_else(|| ApiError::internal("Security middleware state not configured"))?;
        
        // Get method and headers
        let method = parts.method
            .ok_or_else(|| ApiError::internal("Method not available"))?;
        let headers = parts.headers
            .as_ref()
            .ok_or_else(|| ApiError::internal("Headers not available"))?;
        
        // Create security context with all checks
        let security = SecurityContext::from_contexts_with_checks(
            auth,
            request,
            &method,
            headers,
            security_state,
        ).await?;
        
        Ok(SecureContext(security))
    }
}

impl std::ops::Deref for SecureContext {
    type Target = SecurityContext;
    
    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl std::ops::DerefMut for SecureContext {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.0
    }
}

/// Extractor for just RequestContext (useful for non-authenticated endpoints)
pub struct ReqContext(pub RequestContext);

#[async_trait]
impl<S> FromRequestParts<S> for ReqContext
where
    S: Send + Sync,
{
    type Rejection = ApiError;
    
    async fn from_request_parts(
        parts: &mut RequestParts<'_, S>,
        _state: &S,
    ) -> Result<Self, Self::Rejection> {
        let req_ctx = RequestContext::from_request_parts(parts, _state).await?;
        Ok(ReqContext(req_ctx))
    }
}

impl std::ops::Deref for ReqContext {
    type Target = RequestContext;
    
    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

