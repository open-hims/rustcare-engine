/// Authentication middleware for Axum
/// 
/// Validates JWT tokens, checks sessions, enforces permissions, and provides
/// authentication/authorization context for route handlers.

use crate::auth::{
    tokens::JwtService,
    session::{SessionManager, SessionValidation},
    db::PermissionRepository,
};
use axum::{
    extract::{Request, State},
    middleware::Next,
    response::{Response, IntoResponse},
    http::{StatusCode, header::{AUTHORIZATION, COOKIE}},
    body::Body,
};
use std::sync::Arc;
use uuid::Uuid;
use serde::{Serialize, Deserialize};
use std::collections::HashMap;

/// Authentication context injected into request extensions
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AuthContext {
    pub user_id: Uuid,
    pub username: String,
    pub email: Option<String>,
    pub organization_id: Option<Uuid>,
    pub roles: Vec<String>,
    pub permissions: Vec<String>,
    pub session_id: String,
    pub auth_method: String,
    pub cert_serial: Option<String>,
    pub step_up: Option<bool>,
}

impl AuthContext {
    /// Check if user has a specific permission
    pub fn has_permission(&self, permission: &str) -> bool {
        self.permissions.iter().any(|p| {
            // Exact match
            if p == permission {
                return true;
            }
            // Wildcard match: "patient:*" matches "patient:read"
            if p.ends_with(":*") {
                let prefix = &p[..p.len()-1]; // Remove '*', keep ':'
                return permission.starts_with(prefix);
            }
            false
        })
    }
    
    /// Check if user has a specific role
    pub fn has_role(&self, role: &str) -> bool {
        self.roles.iter().any(|r| r == role)
    }
    
    /// Check if user has any of the specified roles
    pub fn has_any_role(&self, roles: &[&str]) -> bool {
        roles.iter().any(|r| self.has_role(r))
    }
    
    /// Check if user has all of the specified permissions
    pub fn has_all_permissions(&self, permissions: &[&str]) -> bool {
        permissions.iter().all(|p| self.has_permission(p))
    }
    
    /// Check if user has any of the specified permissions
    pub fn has_any_permission(&self, permissions: &[&str]) -> bool {
        permissions.iter().any(|p| self.has_permission(p))
    }
    
    /// Check if user is a super admin
    pub fn is_super_admin(&self) -> bool {
        self.has_role("super_admin")
    }
    
    /// Check if step-up authentication is active
    pub fn has_step_up(&self) -> bool {
        self.step_up.unwrap_or(false)
    }
}

/// Shared authentication service state
#[derive(Clone)]
pub struct AuthService {
    pub jwt_service: Arc<JwtService>,
    pub session_manager: Arc<SessionManager>,
    pub permission_repo: PermissionRepository,
}

/// Main authentication middleware
/// 
/// Validates JWT token and session, loads user permissions, and injects AuthContext
pub async fn auth_middleware(
    State(auth_service): State<Arc<AuthService>>,
    mut request: Request<Body>,
    next: Next,
) -> Result<Response, AuthError> {
    // Extract JWT token
    let token = extract_token(&request)
        .ok_or(AuthError::MissingToken)?;
    
    // Validate JWT
    let token_data = auth_service.jwt_service
        .validate_token(&token)
        .await
        .map_err(|_| AuthError::InvalidToken)?;
    
    let claims = &token_data.claims;
    
    // Extract session ID from claims
    let session_id = &claims.sid;
    
    // Build session validation
    let validation = build_session_validation(&request);
    
    // Validate session
    let session_result = auth_service.session_manager
        .validate_session(&session_id, validation)
        .await
        .map_err(|_| AuthError::SessionValidationFailed)?;
    
    if !session_result.valid {
        let reason = session_result.reason.unwrap_or_else(|| "Unknown".to_string());
        tracing::warn!(
            user_id = %claims.sub,
            session_id = %session_id,
            reason = %reason,
            "Session validation failed"
        );
        return Err(AuthError::InvalidSession);
    }
    
    let session = session_result.session
        .ok_or(AuthError::SessionNotFound)?;
    
    // Update session activity
    auth_service.session_manager
        .update_activity(&session_id)
        .await
        .ok(); // Don't fail request if update fails
    
    // Parse user ID
    let user_id = Uuid::parse_str(&session.user_id)
        .map_err(|_| AuthError::InvalidUserId)?;
    
    // Load user permissions (from claims or database)
    let permissions = if let Some(perms) = &claims.permissions {
        if !perms.is_empty() {
            // Use permissions from JWT claims (cached)
            perms.clone()
        } else {
            // Load from database
            load_user_permissions(&auth_service.permission_repo, user_id)
                .await
                .unwrap_or_default()
        }
    } else {
        // Load from database
        load_user_permissions(&auth_service.permission_repo, user_id)
            .await
            .unwrap_or_default()
    };
    
    // Extract roles from claims or load from database
    let roles = load_user_roles(&auth_service.permission_repo, user_id)
        .await
        .unwrap_or_default();
    
    // Build auth context
    let auth_ctx = AuthContext {
        user_id,
        username: claims.sub.clone(),
        email: claims.email.clone(),
        organization_id: parse_uuid_opt(&session.metadata.get("organization_id")),
        roles,
        permissions,
        session_id: session.session_id.clone(),
        auth_method: session.auth_method.clone(),
        cert_serial: claims.cert_serial.clone(),
        step_up: claims.step_up,
    };
    
    // Inject into request extensions
    request.extensions_mut().insert(auth_ctx.clone());
    request.extensions_mut().insert(token_data.claims.clone());
    
    // Log successful authentication
    tracing::debug!(
        user_id = %auth_ctx.user_id,
        session_id = %auth_ctx.session_id,
        auth_method = %auth_ctx.auth_method,
        "Request authenticated"
    );
    
    Ok(next.run(request).await)
}

/// Optional authentication middleware
/// 
/// Attempts to authenticate but continues even if authentication fails.
/// Useful for endpoints that have both public and authenticated behavior.
pub async fn optional_auth_middleware(
    State(auth_service): State<Arc<AuthService>>,
    mut request: Request<Body>,
    next: Next,
) -> Response {
    // Try to extract token
    if let Some(token) = extract_token(&request) {
        // Try to validate
        if let Ok(token_data) = auth_service.jwt_service.validate_token(&token).await {
            let claims = &token_data.claims;
            let session_id = &claims.sid;
            let validation = build_session_validation(&request);
            
            if let Ok(result) = auth_service.session_manager.validate_session(session_id, validation).await {
                if result.valid && result.session.is_some() {
                    let session = result.session.unwrap();
                    
                    if let Ok(user_id) = Uuid::parse_str(&session.user_id) {
                        let permissions = load_user_permissions(&auth_service.permission_repo, user_id).await.unwrap_or_default();
                        let roles = load_user_roles(&auth_service.permission_repo, user_id).await.unwrap_or_default();
                        
                        let auth_ctx = AuthContext {
                            user_id,
                            username: claims.sub.clone(),
                            email: claims.email.clone(),
                            organization_id: parse_uuid_opt(&session.metadata.get("organization_id")),
                            roles,
                            permissions,
                            session_id: session.session_id.clone(),
                            auth_method: session.auth_method.clone(),
                            cert_serial: claims.cert_serial.clone(),
                            step_up: claims.step_up,
                        };
                        
                        request.extensions_mut().insert(auth_ctx);
                        request.extensions_mut().insert(token_data.claims.clone());
                    }
                }
            }
        }
    }
    
    // Continue regardless of authentication result
    next.run(request).await
}

/// Step-up authentication middleware
/// 
/// Requires elevated authentication (recent re-authentication) for sensitive operations
pub async fn step_up_middleware(
    request: Request<Body>,
    next: Next,
) -> Result<Response, AuthError> {
    // Extract auth context
    let auth_ctx = request.extensions()
        .get::<AuthContext>()
        .ok_or(AuthError::Unauthenticated)?;
    
    // Check if step-up is active
    if !auth_ctx.has_step_up() {
        tracing::warn!(
            user_id = %auth_ctx.user_id,
            "Step-up authentication required but not present"
        );
        return Err(AuthError::StepUpRequired);
    }
    
    Ok(next.run(request).await)
}

/// Middleware layer that requires a specific permission
pub struct RequirePermission {
    permission: String,
}

impl RequirePermission {
    pub fn new(permission: impl Into<String>) -> Self {
        Self {
            permission: permission.into(),
        }
    }
    
    pub async fn middleware(
        permission: String,
        request: Request<Body>,
        next: Next,
    ) -> Result<Response, AuthError> {
        let auth_ctx = request.extensions()
            .get::<AuthContext>()
            .ok_or(AuthError::Unauthenticated)?;
        
        if !auth_ctx.has_permission(&permission) {
            tracing::warn!(
                user_id = %auth_ctx.user_id,
                required_permission = %permission,
                user_permissions = ?auth_ctx.permissions,
                "Permission denied"
            );
            return Err(AuthError::Forbidden);
        }
        
        Ok(next.run(request).await)
    }
}

/// Middleware layer that requires a specific role
pub struct RequireRole {
    role: String,
}

impl RequireRole {
    pub fn new(role: impl Into<String>) -> Self {
        Self {
            role: role.into(),
        }
    }
    
    pub async fn middleware(
        role: String,
        request: Request<Body>,
        next: Next,
    ) -> Result<Response, AuthError> {
        let auth_ctx = request.extensions()
            .get::<AuthContext>()
            .ok_or(AuthError::Unauthenticated)?;
        
        if !auth_ctx.has_role(&role) {
            tracing::warn!(
                user_id = %auth_ctx.user_id,
                required_role = %role,
                user_roles = ?auth_ctx.roles,
                "Role check failed"
            );
            return Err(AuthError::Forbidden);
        }
        
        Ok(next.run(request).await)
    }
}

/// Middleware layer that requires any of the specified permissions
pub struct RequireAnyPermission {
    permissions: Vec<String>,
}

impl RequireAnyPermission {
    pub fn new(permissions: Vec<impl Into<String>>) -> Self {
        Self {
            permissions: permissions.into_iter().map(|p| p.into()).collect(),
        }
    }
    
    pub async fn middleware(
        permissions: Vec<String>,
        request: Request<Body>,
        next: Next,
    ) -> Result<Response, AuthError> {
        let auth_ctx = request.extensions()
            .get::<AuthContext>()
            .ok_or(AuthError::Unauthenticated)?;
        
        let has_any = permissions.iter().any(|p| auth_ctx.has_permission(p));
        
        if !has_any {
            tracing::warn!(
                user_id = %auth_ctx.user_id,
                required_permissions = ?permissions,
                user_permissions = ?auth_ctx.permissions,
                "Permission denied (requires any of)"
            );
            return Err(AuthError::Forbidden);
        }
        
        Ok(next.run(request).await)
    }
}

/// Middleware layer that requires all of the specified permissions
pub struct RequireAllPermissions {
    permissions: Vec<String>,
}

impl RequireAllPermissions {
    pub fn new(permissions: Vec<impl Into<String>>) -> Self {
        Self {
            permissions: permissions.into_iter().map(|p| p.into()).collect(),
        }
    }
    
    pub async fn middleware(
        permissions: Vec<String>,
        request: Request<Body>,
        next: Next,
    ) -> Result<Response, AuthError> {
        let auth_ctx = request.extensions()
            .get::<AuthContext>()
            .ok_or(AuthError::Unauthenticated)?;
        
        if !auth_ctx.has_all_permissions(&permissions.iter().map(|s| s.as_str()).collect::<Vec<_>>()) {
            tracing::warn!(
                user_id = %auth_ctx.user_id,
                required_permissions = ?permissions,
                user_permissions = ?auth_ctx.permissions,
                "Permission denied (requires all)"
            );
            return Err(AuthError::Forbidden);
        }
        
        Ok(next.run(request).await)
    }
}

// =============================================================================
// HELPER FUNCTIONS
// =============================================================================

/// Extract JWT token from Authorization header or Cookie
fn extract_token(request: &Request<Body>) -> Option<String> {
    // Try Authorization header first
    if let Some(auth_header) = request.headers().get(AUTHORIZATION) {
        if let Ok(auth_str) = auth_header.to_str() {
            if auth_str.starts_with("Bearer ") {
                return Some(auth_str[7..].to_string());
            }
        }
    }
    
    // Try Cookie header
    if let Some(cookie_header) = request.headers().get(COOKIE) {
        if let Ok(cookie_str) = cookie_header.to_str() {
            for cookie in cookie_str.split(';') {
                let parts: Vec<&str> = cookie.trim().splitn(2, '=').collect();
                if parts.len() == 2 && parts[0] == "access_token" {
                    return Some(parts[1].to_string());
                }
            }
        }
    }
    
    None
}

/// Build session validation from request
fn build_session_validation(request: &Request<Body>) -> SessionValidation {
    let ip_address = extract_client_ip(request);
    let user_agent = extract_user_agent(request);
    let additional_headers = extract_additional_headers(request);
    
    SessionValidation {
        ip_address,
        user_agent,
        additional_headers,
    }
}

/// Extract client IP address from request
fn extract_client_ip(request: &Request<Body>) -> String {
    // Try X-Forwarded-For header first
    if let Some(forwarded) = request.headers().get("x-forwarded-for") {
        if let Ok(forwarded_str) = forwarded.to_str() {
            if let Some(first_ip) = forwarded_str.split(',').next() {
                return first_ip.trim().to_string();
            }
        }
    }
    
    // Try X-Real-IP header
    if let Some(real_ip) = request.headers().get("x-real-ip") {
        if let Ok(ip_str) = real_ip.to_str() {
            return ip_str.to_string();
        }
    }
    
    // Fallback to unknown
    "unknown".to_string()
}

/// Extract User-Agent from request
fn extract_user_agent(request: &Request<Body>) -> String {
    request.headers()
        .get("user-agent")
        .and_then(|v| v.to_str().ok())
        .unwrap_or("unknown")
        .to_string()
}

/// Extract additional headers for fingerprinting
fn extract_additional_headers(request: &Request<Body>) -> HashMap<String, String> {
    let mut headers = HashMap::new();
    
    let header_names = [
        "accept-language",
        "accept-encoding",
        "accept",
    ];
    
    for name in &header_names {
        if let Some(value) = request.headers().get(*name) {
            if let Ok(value_str) = value.to_str() {
                headers.insert(name.to_string(), value_str.to_string());
            }
        }
    }
    
    headers
}

/// Load user permissions from database
async fn load_user_permissions(
    _permission_repo: &PermissionRepository,
    _user_id: Uuid,
) -> anyhow::Result<Vec<String>> {
    // TODO: Query the database for user permissions
    // Will implement when PermissionRepository methods are added
    Ok(Vec::new())
}

/// Load user roles from database
async fn load_user_roles(
    _permission_repo: &PermissionRepository,
    _user_id: Uuid,
) -> anyhow::Result<Vec<String>> {
    // TODO: Query the database for user roles
    // Will implement when PermissionRepository methods are added
    Ok(Vec::new())
}

/// Parse optional UUID from JSON value
fn parse_uuid_opt(value: &Option<&serde_json::Value>) -> Option<Uuid> {
    value
        .and_then(|v| v.as_str())
        .and_then(|s| Uuid::parse_str(s).ok())
}

// =============================================================================
// ERROR TYPES
// =============================================================================

#[derive(Debug, thiserror::Error)]
pub enum AuthError {
    #[error("Missing authentication token")]
    MissingToken,
    
    #[error("Invalid authentication token")]
    InvalidToken,
    
    #[error("Missing session ID in token claims")]
    MissingSessionId,
    
    #[error("Session validation failed")]
    SessionValidationFailed,
    
    #[error("Invalid session")]
    InvalidSession,
    
    #[error("Session not found")]
    SessionNotFound,
    
    #[error("Invalid user ID")]
    InvalidUserId,
    
    #[error("User not authenticated")]
    Unauthenticated,
    
    #[error("Insufficient permissions")]
    Forbidden,
    
    #[error("Step-up authentication required")]
    StepUpRequired,
}

impl IntoResponse for AuthError {
    fn into_response(self) -> Response {
        let (status, message) = match self {
            AuthError::MissingToken => (StatusCode::UNAUTHORIZED, "Missing authentication token"),
            AuthError::InvalidToken => (StatusCode::UNAUTHORIZED, "Invalid authentication token"),
            AuthError::MissingSessionId => (StatusCode::UNAUTHORIZED, "Invalid token: missing session ID"),
            AuthError::SessionValidationFailed => (StatusCode::UNAUTHORIZED, "Session validation failed"),
            AuthError::InvalidSession => (StatusCode::UNAUTHORIZED, "Invalid or expired session"),
            AuthError::SessionNotFound => (StatusCode::UNAUTHORIZED, "Session not found"),
            AuthError::InvalidUserId => (StatusCode::INTERNAL_SERVER_ERROR, "Invalid user ID"),
            AuthError::Unauthenticated => (StatusCode::UNAUTHORIZED, "Authentication required"),
            AuthError::Forbidden => (StatusCode::FORBIDDEN, "Insufficient permissions"),
            AuthError::StepUpRequired => (StatusCode::FORBIDDEN, "Step-up authentication required"),
        };
        
        let body = serde_json::json!({
            "error": message,
            "status": status.as_u16(),
        });
        
        (status, axum::Json(body)).into_response()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_auth_context_permission_checks() {
        let ctx = AuthContext {
            user_id: Uuid::new_v4(),
            username: "test".to_string(),
            email: Some("test@example.com".to_string()),
            organization_id: Some(Uuid::new_v4()),
            roles: vec!["doctor".to_string()],
            permissions: vec![
                "patient:read".to_string(),
                "patient:write".to_string(),
                "appointment:*".to_string(),
            ],
            session_id: "session123".to_string(),
            auth_method: "password".to_string(),
            cert_serial: None,
            step_up: None,
        };
        
        // Exact permission match
        assert!(ctx.has_permission("patient:read"));
        assert!(ctx.has_permission("patient:write"));
        
        // Wildcard permission match
        assert!(ctx.has_permission("appointment:read"));
        assert!(ctx.has_permission("appointment:write"));
        assert!(ctx.has_permission("appointment:delete"));
        
        // Non-existent permission
        assert!(!ctx.has_permission("billing:read"));
        
        // Role checks
        assert!(ctx.has_role("doctor"));
        assert!(!ctx.has_role("admin"));
        
        // Multiple permission checks
        assert!(ctx.has_all_permissions(&["patient:read", "patient:write"]));
        assert!(!ctx.has_all_permissions(&["patient:read", "billing:read"]));
        
        assert!(ctx.has_any_permission(&["patient:read", "billing:read"]));
        assert!(!ctx.has_any_permission(&["billing:read", "billing:write"]));
    }
    
    #[test]
    fn test_auth_context_step_up() {
        // Active step-up
        let ctx_active = AuthContext {
            user_id: Uuid::new_v4(),
            username: "test".to_string(),
            email: None,
            organization_id: None,
            roles: vec![],
            permissions: vec![],
            session_id: "session123".to_string(),
            auth_method: "password".to_string(),
            cert_serial: None,
            step_up: Some(true),
        };
        
        assert!(ctx_active.has_step_up());
        
        // No step-up
        let ctx_none = AuthContext {
            step_up: None,
            ..ctx_active.clone()
        };
        
        assert!(!ctx_none.has_step_up());
        
        // Explicit false
        let ctx_false = AuthContext {
            step_up: Some(false),
            ..ctx_active
        };
        
        assert!(!ctx_false.has_step_up());
    }
}
