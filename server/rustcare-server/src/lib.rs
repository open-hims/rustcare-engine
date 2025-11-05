//! RustCare Server - HIPAA-compliant healthcare platform API
//! 
//! This library provides the core functionality of the RustCare HTTP server,
//! including authentication, authorization, and RESTful API endpoints.

pub mod auth;
pub mod error;
pub mod handlers;
pub mod middleware;
pub mod routes;
pub mod server;
pub mod openapi;
pub mod security_state;
pub mod utils;
pub mod types;
pub mod validation;

// Re-export commonly used types
pub use server::RustCareServer;
pub use error::*;
pub use security_state::SecurityState;

use axum::{middleware::from_fn, Router, Extension};
use tower::ServiceBuilder;
use tower_http::trace::TraceLayer;
use std::sync::Arc;
use crate::middleware::{SecurityConfig, SecurityMiddlewareState, ZanzibarCheck};

/// Create the main application router with all routes and middleware
pub fn create_app(server: RustCareServer) -> Router {
    // Initialize security state with configuration
    let security_config = SecurityConfig {
        rate_limit: Some(crate::middleware::RateLimitConfig {
            max_requests: 100,
            window_seconds: 60,
            by_user: true,
        }),
        csrf: Some(crate::middleware::CsrfValidator::new()),
        strict_same_site: false, // Set to true for strict same-site enforcement
    };
    let security_middleware_state = SecurityMiddlewareState::new(security_config);
    
    // Add Zanzibar engine to extensions if available
    let mut router = routes::create_routes();
    
    if let Some(ref zanzibar_engine) = server.zanzibar_engine {
        router = router.layer(Extension(Arc::clone(zanzibar_engine) as Arc<dyn ZanzibarCheck>));
    }
    
    router
        .layer(
            ServiceBuilder::new()
                .layer(TraceLayer::new_for_http())
                .layer(middleware::create_cors_layer())
                .layer(from_fn(middleware::request_timing_middleware))
                .layer(from_fn(middleware::audit_logging_middleware))
                .layer(Extension(security_middleware_state)) // Make security middleware state available to handlers
        )
        .with_state(server)
}
