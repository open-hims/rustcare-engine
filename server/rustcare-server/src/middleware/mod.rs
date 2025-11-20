//! Middleware modules for request processing

pub mod auth_context;
pub mod request_context;
pub mod security;
pub mod security_middleware;
pub mod extractors;
pub mod zanzibar_engine;

// Re-export for convenience
pub use auth_context::AuthContext;
pub use request_context::RequestContext;
pub use security::{SecurityContext, SecurityConfig, SecurityMiddlewareState, RateLimiter, RateLimitConfig, CsrfValidator};
pub use security_middleware::security_middleware;
pub use extractors::{SecureContext, ReqContext};
pub use zanzibar_engine::ZanzibarEngineWrapper;
pub use auth_context::ZanzibarCheck;

use axum::{
    http::{header, Method},
    middleware::Next,
    response::Response,
    extract::Request,
};
use tower_http::cors::CorsLayer;
use std::time::{Duration, Instant};

/// Request timing middleware for performance monitoring
pub async fn request_timing_middleware(
    request: Request,
    next: Next,
) -> Response {
    let path = request.uri().path().to_string();
    let start = Instant::now();
    let response = next.run(request).await;
    let duration = start.elapsed();
    
    // Log slow requests
    if duration > Duration::from_secs(1) {
        tracing::warn!(
            path = %path,
            duration_ms = duration.as_millis(),
            "Slow request detected"
        );
    }
    
    response
}

/// Audit logging middleware for HIPAA compliance
pub async fn audit_logging_middleware(
    request: Request,
    next: Next,
) -> Response {
    // Extract audit information
    let method = request.method().clone();
    let path = request.uri().path().to_string();
    let user_agent = request.headers()
        .get(header::USER_AGENT)
        .and_then(|h| h.to_str().ok())
        .map(|s| s.to_string());
    
    // Execute request
    let response = next.run(request).await;
    
    // Log audit event (TODO: Integrate with audit-engine)
    tracing::info!(
        method = %method,
        path = %path,
        status = %response.status(),
        user_agent = ?user_agent,
        "API request audit"
    );
    
    response
}

/// Create CORS layer with HIPAA-compliant configuration
pub fn create_cors_layer() -> CorsLayer {
    CorsLayer::new()
        .allow_origin(tower_http::cors::Any)
        .allow_methods([Method::GET, Method::POST, Method::PUT, Method::DELETE, Method::PATCH])
        .allow_headers([
            header::CONTENT_TYPE,
            header::AUTHORIZATION,
            header::ACCEPT,
        ])
        .max_age(Duration::from_secs(3600))
}

