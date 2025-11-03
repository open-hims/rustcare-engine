//! Middleware modules for request processing

pub mod auth_context;
pub mod request_context;
pub mod security;
pub mod security_middleware;
pub mod extractors;

// Re-export for convenience
pub use auth_context::AuthContext;
pub use request_context::RequestContext;
pub use security::{SecurityContext, SecurityConfig, SecurityState, RateLimiter, RateLimitConfig, CsrfValidator};
pub use security_middleware::security_middleware;
pub use extractors::{SecureContext, ReqContext};

