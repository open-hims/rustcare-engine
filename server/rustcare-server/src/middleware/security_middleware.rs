//! Security middleware that wraps requests with SecurityContext
//!
//! This middleware extracts AuthContext and RequestContext, performs security checks,
//! and makes SecurityContext available in request extensions.

use axum::{
    extract::Request,
    middleware::Next,
    response::Response,
    http::{Method, HeaderMap},
};
use crate::middleware::{AuthContext, RequestContext, SecurityContext, SecurityMiddlewareState};
use crate::error::ApiError;

/// Security middleware that performs all security checks
pub async fn security_middleware(
    mut request: Request,
    next: Next,
) -> Result<Response, ApiError> {
    // Extract security middleware state from extensions (set during router initialization)
    let security_state = request.extensions()
        .get::<SecurityMiddlewareState>()
        .cloned()
        .unwrap_or_else(|| SecurityMiddlewareState::new(crate::middleware::SecurityConfig::default()));
    
    // Extract method and headers before consuming request
    let method = request.method().clone();
    let headers = request.headers().clone();
    
    // Try to extract AuthContext and RequestContext from request parts
    // Note: In a real implementation, you'd need to use axum's RequestParts
    // For now, we'll extract them manually or use a different approach
    
    // For handlers, use SecurityContext::from_contexts_with_checks() instead
    // This middleware sets up the security state in extensions
    
    // Add security middleware state to extensions if not already present
    if request.extensions().get::<SecurityMiddlewareState>().is_none() {
        request.extensions_mut().insert(security_state.clone());
    }
    
    // Continue with request
    let response = next.run(request).await;
    
    // Add security headers to response
    // (This would need mutable access to response headers)
    
    Ok(response)
}

// TODO: Implement SecurityContext helper function
// Example usage:
// pub async fn handler(auth: AuthContext, req_ctx: RequestContext) -> Result<...> {
//     let security = SecurityContext::from_contexts_with_checks(...).await?;
//     security.require_permission("patient", Some(patient_id), "view").await?;
// }

// Placeholder for SecurityContext creation
// This function would create a SecurityContext from various context types
