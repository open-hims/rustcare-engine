// HTTP handlers for the identity service
// This would typically integrate with axum or another web framework

use crate::{models::*, service::IdentityService, error::*};
use serde_json::json;
use std::sync::Arc;

pub struct IdentityHandlers {
    service: Arc<IdentityService>,
}

impl IdentityHandlers {
    pub fn new(service: Arc<IdentityService>) -> Self {
        Self { service }
    }

    // These would be actual HTTP handlers in a real implementation
    // For now, they serve as examples of the service interface

    pub async fn register(&self, request: CreateUserRequest) -> Result<serde_json::Value> {
        let user = self.service.register_user(request).await?;
        Ok(json!({
            "success": true,
            "user": user,
            "message": "User registered successfully"
        }))
    }

    pub async fn login(&self, request: LoginRequest) -> Result<serde_json::Value> {
        let response = self.service.authenticate(&request.email, &request.password).await?;
        Ok(json!({
            "success": true,
            "user": response.user,
            "token": response.token,
            "expires_at": response.expires_at
        }))
    }

    pub async fn validate_token(&self, token: &str) -> Result<serde_json::Value> {
        let user = self.service.validate_token(token).await?;
        Ok(json!({
            "success": true,
            "user": user
        }))
    }

    pub async fn logout(&self, token: &str) -> Result<serde_json::Value> {
        self.service.logout(token).await?;
        Ok(json!({
            "success": true,
            "message": "Logged out successfully"
        }))
    }
}