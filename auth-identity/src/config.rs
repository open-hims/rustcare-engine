use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IdentityConfig {
    pub jwt_secret: String,
    pub jwt_expiration_hours: i64,
    pub password_min_length: usize,
    pub password_require_special_chars: bool,
    pub password_require_numbers: bool,
    pub password_require_uppercase: bool,
    pub session_timeout_minutes: i64,
    pub max_login_attempts: u32,
    pub lockout_duration_minutes: i64,
}

impl Default for IdentityConfig {
    fn default() -> Self {
        Self {
            jwt_secret: "your-secret-key".to_string(),
            jwt_expiration_hours: 24,
            password_min_length: 8,
            password_require_special_chars: true,
            password_require_numbers: true,
            password_require_uppercase: true,
            session_timeout_minutes: 60,
            max_login_attempts: 5,
            lockout_duration_minutes: 30,
        }
    }
}