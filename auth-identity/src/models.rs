use serde::{Deserialize, Serialize};
use uuid::Uuid;
use chrono::{DateTime, Utc};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct User {
    pub id: Uuid,
    pub email: String,
    pub username: Option<String>,
    pub password_hash: String,
    pub is_active: bool,
    pub is_verified: bool,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    pub last_login: Option<DateTime<Utc>>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UserProfile {
    pub user_id: Uuid,
    pub first_name: Option<String>,
    pub last_name: Option<String>,
    pub display_name: Option<String>,
    pub avatar_url: Option<String>,
    pub bio: Option<String>,
    pub phone: Option<String>,
    pub timezone: Option<String>,
    pub locale: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Session {
    pub id: Uuid,
    pub user_id: Uuid,
    pub token: String,
    pub expires_at: DateTime<Utc>,
    pub created_at: DateTime<Utc>,
    pub ip_address: Option<String>,
    pub user_agent: Option<String>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct CreateUserRequest {
    pub email: String,
    pub username: Option<String>,
    pub password: String,
    pub profile: Option<UserProfile>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct LoginRequest {
    pub email: String,
    pub password: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct LoginResponse {
    pub user: User,
    pub token: String,
    pub expires_at: DateTime<Utc>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct PasswordResetRequest {
    pub email: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct PasswordResetConfirm {
    pub token: String,
    pub new_password: String,
}