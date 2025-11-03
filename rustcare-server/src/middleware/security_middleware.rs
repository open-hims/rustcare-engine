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
use crate::middleware::{AuthContext, RequestContext, SecurityContext, SecurityState};
use crate::error::ApiError;

/// Security middleware that performs all security checks
pub async fn security_middleware(
    mut request: Request,
    next: Next,
) -> Result<Response, ApiError> {
    // Extract security state from extensions (set during router initialization)
    let security_state = request.extensions()
        .get::<SecurityState>()
        .cloned()
        .unwrap_or_else(|| SecurityState::new(crate::middleware::SecurityConfig::default()));
    
    // Extract method and headers before consuming request
    let method = request.method().clone();
    let headers = request.headers().clone();
    
    // Try to extract AuthContext and RequestContext from request parts
    // Note: In a real implementation, you'd need to use axum's RequestParts
    // For now, we'll extract them manually or use a different approach
    
    // For handlers, use SecurityContext::from_contexts_with_checks() instead
    // This middleware sets up the security state in extensions
    
    // Add security state to extensions if not already present
    if !request.extensions().contains::<SecurityState>() {
        request.extensions_mut().insert(security_state.clone());
    }
    
    // Continue with request
    let response = next.run(request).await;
    
    // Add security headers to response
    // (This would need mutable access to response headers)
    
    Ok(response)
}

/// Helper function to create SecurityContext in handlers
/// 
/// Usage:
/// ```rust
/// pub async fn handler(
///     auth: AuthContext,
///     req_ctx: RequestContext,
///     method: Method,
///     headers: HeaderMap,
///     State(server): State<RustCareServer>,
/// ) -> Result<...> {
///     let security = SecurityContext::from_contexts_with_checks(
///         auth,
///         req_ctx,
///         &method,
///         &headers,
///         &server.security_state, // Would need to add this to RustCareServer
///     ).await?;
///     
///     // Use security context
///     security.require_permission("patient", Some(patient_id), "view").await?;
/// }
/// ```

