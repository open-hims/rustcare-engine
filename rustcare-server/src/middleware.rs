use axum::{
    http::{header, HeaderMap, Method, StatusCode},
    middleware::Next,
    response::Response,
    extract::Request,
};
use tower_http::cors::CorsLayer;
use std::time::{Duration, Instant};

// Re-export auth context module for easy access
pub mod auth_context;
pub use auth_context::AuthContext;

/// HIPAA compliance middleware
pub async fn hipaa_compliance_middleware(
    headers: HeaderMap,
    request: Request,
    next: Next,
) -> Result<Response, StatusCode> {
    // Check for required HIPAA headers
    if !headers.contains_key("X-HIPAA-Consent") {
        return Err(StatusCode::BAD_REQUEST);
    }

    // Add HIPAA compliance headers to response
    let mut response = next.run(request).await;
    
    response.headers_mut().insert(
        "X-HIPAA-Compliant",
        "true".parse().unwrap(),
    );
    
    response.headers_mut().insert(
        "X-Privacy-Policy",
        "https://rustcare.dev/privacy".parse().unwrap(),
    );
    
    Ok(response)
}

/// Request timing middleware for performance monitoring
pub async fn request_timing_middleware(
    request: Request,
    next: Next,
) -> Response {
    let start = Instant::now();
    let response = next.run(request).await;
    let duration = start.elapsed();
    
    // Log slow requests
    if duration > Duration::from_secs(1) {
        tracing::warn!(
            path = %request.uri().path(),
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
        .allow_origin("*".parse::<axum::http::HeaderValue>().unwrap())
        .allow_methods([Method::GET, Method::POST, Method::PUT, Method::DELETE, Method::PATCH])
        .allow_headers([
            header::CONTENT_TYPE,
            header::AUTHORIZATION,
            header::ACCEPT,
        ])
        .max_age(Duration::from_secs(3600))
}
