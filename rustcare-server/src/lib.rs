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

// Re-export commonly used types
pub use server::RustCareServer;
pub use error::*;
pub use security_state::SecurityState;

use axum::{middleware::from_fn, Router};
use tower::ServiceBuilder;
use tower_http::trace::TraceLayer;

/// Create the main application router with all routes and middleware
pub fn create_app(server: RustCareServer) -> Router {
    routes::create_routes()
        .layer(
            ServiceBuilder::new()
                .layer(TraceLayer::new_for_http())
                .layer(middleware::create_cors_layer())
                .layer(from_fn(middleware::request_timing_middleware))
                .layer(from_fn(middleware::audit_logging_middleware))
        )
        .with_state(server)
}
