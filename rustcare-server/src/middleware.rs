use axum::{
    http::{header, HeaderMap, HeaderValue, Method, StatusCode},
    middleware::Next,
    response::Response,
    extract::Request,
};
use tower_http::cors::CorsLayer;
use std::time::{Duration, Instant};

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
        "https://rustcare.com/privacy".parse().unwrap(),
    );

    Ok(response)
}

/// Request timing middleware
pub async fn request_timing_middleware(
    request: Request,
    next: Next,
) -> Response {
    let start = Instant::now();
    let method = request.method().clone();
    let uri = request.uri().clone();
    
    let response = next.run(request).await;
    
    let elapsed = start.elapsed();
    
    tracing::info!(
        method = %method,
        uri = %uri,
        duration_ms = elapsed.as_millis(),
        status = response.status().as_u16(),
        "Request processed"
    );

    response
}

/// Audit logging middleware
pub async fn audit_logging_middleware(
    headers: HeaderMap,
    request: Request,
    next: Next,
) -> Response {
    let method = request.method().clone();
    let uri = request.uri().clone();
    let user_id = headers
        .get("Authorization")
        .and_then(|h| h.to_str().ok())
        .unwrap_or("anonymous");

    // Log the request for audit purposes
    tracing::info!(
        method = %method,
        uri = %uri,
        user_id = user_id,
        timestamp = %chrono::Utc::now().to_rfc3339(),
        "Audit log: Request received"
    );

    let response = next.run(request).await;

    // Log the response for audit purposes
    tracing::info!(
        method = %method,
        uri = %uri,
        user_id = user_id,
        status = response.status().as_u16(),
        timestamp = %chrono::Utc::now().to_rfc3339(),
        "Audit log: Response sent"
    );

    response
}

/// Authentication middleware
pub async fn auth_middleware(
    headers: HeaderMap,
    request: Request,
    next: Next,
) -> Result<Response, StatusCode> {
    // Skip authentication for health check endpoints
    let path = request.uri().path();
    if path.starts_with("/health") || path.starts_with("/version") {
        return Ok(next.run(request).await);
    }

    // Check for Authorization header
    let auth_header = headers
        .get(header::AUTHORIZATION)
        .and_then(|header| header.to_str().ok());

    match auth_header {
        Some(token) if token.starts_with("Bearer ") => {
            // TODO: Validate JWT token with auth-gateway module
            // For now, accept any bearer token
            Ok(next.run(request).await)
        }
        _ => Err(StatusCode::UNAUTHORIZED),
    }
}

/// Rate limiting middleware
pub async fn rate_limiting_middleware(
    headers: HeaderMap,
    request: Request,
    next: Next,
) -> Result<Response, StatusCode> {
    // TODO: Implement proper rate limiting with Redis or in-memory store
    // For now, this is a placeholder that allows all requests
    
    let client_ip = headers
        .get("X-Forwarded-For")
        .or_else(|| headers.get("X-Real-IP"))
        .and_then(|h| h.to_str().ok())
        .unwrap_or("unknown");

    // Log rate limiting info
    tracing::debug!(
        client_ip = client_ip,
        path = %request.uri().path(),
        "Rate limiting check"
    );

    Ok(next.run(request).await)
}

/// Create CORS layer for the application
pub fn create_cors_layer() -> CorsLayer {
    CorsLayer::new()
        .allow_origin([
            "https://localhost:8443".parse::<HeaderValue>().unwrap(),
            "http://localhost:8081".parse::<HeaderValue>().unwrap(),
            "https://api.openhims.health".parse::<HeaderValue>().unwrap(),
            "http://localhost:3000".parse::<HeaderValue>().unwrap(),
            "https://localhost:3000".parse::<HeaderValue>().unwrap(),
            "http://localhost:8080".parse::<HeaderValue>().unwrap(),
            "http://127.0.0.1:3000".parse::<HeaderValue>().unwrap(),
        ])
        .allow_methods([
            Method::GET,
            Method::POST,
            Method::PUT,
            Method::DELETE,
            Method::PATCH,
            Method::OPTIONS,
        ])
        .allow_headers([
            header::AUTHORIZATION,
            header::CONTENT_TYPE,
            header::ACCEPT,
            "X-HIPAA-Consent".parse().unwrap(),
        ])
        .max_age(Duration::from_secs(3600))
}