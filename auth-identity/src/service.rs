use crate::{models::*, repository::*, config::*, error::*};
use argon2::{Argon2, PasswordHash, PasswordHasher, PasswordVerifier};
use argon2::password_hash::{SaltString, rand_core::OsRng};
use uuid::Uuid;
use chrono::{Utc, Duration};
use std::sync::Arc;

pub struct IdentityService {
    user_repo: Arc<dyn UserRepository>,
    session_repo: Arc<dyn SessionRepository>,
    config: IdentityConfig,
    argon2: Argon2<'static>,
}

impl IdentityService {
    pub fn new(
        user_repo: Arc<dyn UserRepository>,
        session_repo: Arc<dyn SessionRepository>,
        config: IdentityConfig,
    ) -> Self {
        Self {
            user_repo,
            session_repo,
            config,
            argon2: Argon2::default(),
        }
    }

    pub async fn register_user(&self, request: CreateUserRequest) -> Result<User> {
        // Validate email format
        if !self.is_valid_email(&request.email) {
            return Err(IdentityError::InvalidEmail);
        }

        // Check if user already exists
        if let Some(_) = self.user_repo.find_by_email(&request.email).await? {
            return Err(IdentityError::EmailAlreadyInUse);
        }

        if let Some(ref username) = request.username {
            if let Some(_) = self.user_repo.find_by_username(username).await? {
                return Err(IdentityError::UsernameAlreadyInUse);
            }
        }

        // Validate password strength
        self.validate_password(&request.password)?;

        // Hash password
        let password_hash = self.hash_password(&request.password)?;

        // Create user
        let user = User {
            id: Uuid::new_v4(),
            email: request.email,
            username: request.username,
            password_hash,
            is_active: true,
            is_verified: false,
            created_at: Utc::now(),
            updated_at: Utc::now(),
            last_login: None,
        };

        self.user_repo.create_user(&user).await
    }

    pub async fn authenticate(&self, email: &str, password: &str) -> Result<LoginResponse> {
        let user = self.user_repo.find_by_email(email).await?
            .ok_or(IdentityError::InvalidCredentials)?;

        if !user.is_active {
            return Err(IdentityError::AccountDisabled);
        }

        // Verify password
        self.verify_password(password, &user.password_hash)?;

        // Update last login
        self.user_repo.update_last_login(user.id).await?;

        // Create session
        let session = self.create_session(user.id).await?;

        Ok(LoginResponse {
            user,
            token: session.token,
            expires_at: session.expires_at,
        })
    }

    pub async fn validate_token(&self, token: &str) -> Result<User> {
        let session = self.session_repo.find_by_token(token).await?
            .ok_or(IdentityError::InvalidToken)?;

        if session.expires_at < Utc::now() {
            self.session_repo.delete_session(token).await?;
            return Err(IdentityError::SessionExpired);
        }

        let user = self.user_repo.find_by_id(session.user_id).await?
            .ok_or(IdentityError::UserNotFound)?;

        if !user.is_active {
            return Err(IdentityError::AccountDisabled);
        }

        Ok(user)
    }

    pub async fn logout(&self, token: &str) -> Result<()> {
        self.session_repo.delete_session(token).await
    }

    pub async fn change_password(&self, user_id: Uuid, old_password: &str, new_password: &str) -> Result<()> {
        let user = self.user_repo.find_by_id(user_id).await?
            .ok_or(IdentityError::UserNotFound)?;

        // Verify old password
        self.verify_password(old_password, &user.password_hash)?;

        // Validate new password
        self.validate_password(new_password)?;

        // Hash new password
        let new_password_hash = self.hash_password(new_password)?;

        // Update user
        let mut updated_user = user;
        updated_user.password_hash = new_password_hash;
        updated_user.updated_at = Utc::now();

        self.user_repo.update_user(&updated_user).await?;

        // Invalidate all sessions
        self.session_repo.delete_user_sessions(user_id).await?;

        Ok(())
    }

    async fn create_session(&self, user_id: Uuid) -> Result<Session> {
        let token = self.generate_session_token();
        let expires_at = Utc::now() + Duration::hours(self.config.jwt_expiration_hours);

        let session = Session {
            id: Uuid::new_v4(),
            user_id,
            token: token.clone(),
            expires_at,
            created_at: Utc::now(),
            ip_address: None,
            user_agent: None,
        };

        self.session_repo.create_session(&session).await
    }

    fn hash_password(&self, password: &str) -> Result<String> {
        let salt = SaltString::generate(&mut OsRng);
        let password_hash = self.argon2
            .hash_password(password.as_bytes(), &salt)
            .map_err(|_| IdentityError::HashingError)?
            .to_string();
        Ok(password_hash)
    }

    fn verify_password(&self, password: &str, hash: &str) -> Result<()> {
        let parsed_hash = PasswordHash::new(hash)
            .map_err(|_| IdentityError::HashingError)?;
        
        self.argon2
            .verify_password(password.as_bytes(), &parsed_hash)
            .map_err(|_| IdentityError::InvalidCredentials)
    }

    fn validate_password(&self, password: &str) -> Result<()> {
        if password.len() < self.config.password_min_length {
            return Err(IdentityError::WeakPassword);
        }

        if self.config.password_require_uppercase && !password.chars().any(|c| c.is_uppercase()) {
            return Err(IdentityError::WeakPassword);
        }

        if self.config.password_require_numbers && !password.chars().any(|c| c.is_numeric()) {
            return Err(IdentityError::WeakPassword);
        }

        if self.config.password_require_special_chars && !password.chars().any(|c| !c.is_alphanumeric()) {
            return Err(IdentityError::WeakPassword);
        }

        Ok(())
    }

    fn is_valid_email(&self, email: &str) -> bool {
        email.contains('@') && email.contains('.')
    }

    fn generate_session_token(&self) -> String {
        // In a real implementation, this would generate a proper JWT or secure token
        Uuid::new_v4().to_string()
    }
}