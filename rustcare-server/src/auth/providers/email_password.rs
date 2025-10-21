/// Email/Password authentication provider
/// 
/// Implements traditional username/password authentication with:
/// - Argon2id password hashing with secure parameters
/// - Rate limiting (max 5 attempts, 30-min lockout)
/// - Optional MFA (TOTP) support
/// - Password expiration checking
/// - Password strength validation

use super::{AuthResult, Credentials, Provider};
use crate::auth::db::{CredentialRepository, DbPool, RateLimitRepository, UserRepository};
use crate::auth::models::{User, UserCredential, UserStatus};
use anyhow::{anyhow, Context, Result};
use argon2::{
    password_hash::{PasswordHash, PasswordHasher, PasswordVerifier, SaltString},
    Argon2, Params, Version,
};
use async_trait::async_trait;
use chrono::{Duration, Utc};
use data_encoding::BASE32;
use rand::{rngs::OsRng, RngCore};
use sqlx::PgPool;
use std::collections::HashMap;
use std::sync::Arc;
use subtle::ConstantTimeEq;
use totp_lite::{totp_custom, Sha1};
use sqlx::types::Uuid;

/// Maximum failed login attempts before lockout
const MAX_FAILED_ATTEMPTS: i32 = 5;

/// Lockout duration in minutes
const LOCKOUT_DURATION_MINUTES: i64 = 30;

/// Password expiration in days (configurable)
const PASSWORD_EXPIRATION_DAYS: i64 = 90;

/// Minimum password length
const MIN_PASSWORD_LENGTH: usize = 12;

/// TOTP time step in seconds
const TOTP_TIME_STEP: u64 = 30;

/// TOTP digits
const TOTP_DIGITS: u32 = 6;

/// Argon2 algorithm identifier
const PASSWORD_ALGORITHM: &str = "argon2id";

pub struct EmailPasswordProvider {
    user_repo: Arc<UserRepository>,
    credential_repo: Arc<CredentialRepository>,
    rate_limit_repo: Arc<RateLimitRepository>,
    argon2: Argon2<'static>,
    enforce_password_expiration: bool,
}

impl EmailPasswordProvider {
    /// Create a new EmailPasswordProvider with secure Argon2id configuration
    pub fn new(
        pool: Arc<PgPool>,
        enforce_password_expiration: bool,
    ) -> Result<Self> {
        // Configure Argon2id with secure parameters
        // Memory: 19MB (19456 KiB)
        // Iterations: 2
        // Parallelism: 1 thread
        // Output length: 32 bytes
        let params = Params::new(
            19456,  // m_cost (memory in KiB)
            2,      // t_cost (iterations)
            1,      // p_cost (parallelism)
            Some(32), // output length
        )
        .map_err(|e| anyhow!("Failed to build Argon2 params: {}", e))?;
        
        let argon2 = Argon2::new(
            argon2::Algorithm::Argon2id,
            Version::V0x13,
            params,
        );
        
        Ok(Self {
            user_repo: Arc::new(UserRepository::new(DbPool::new((*pool).clone()))),
            credential_repo: Arc::new(CredentialRepository::new(DbPool::new((*pool).clone()))),
            rate_limit_repo: Arc::new(RateLimitRepository::new(DbPool::new((*pool).clone()))),
            argon2,
            enforce_password_expiration,
        })
    }
    
    /// Hash a password using Argon2id
    /// 
    /// This is CPU-intensive and should be run in a blocking task
    pub async fn hash_password(&self, password: &str) -> Result<String> {
        // Validate password strength before hashing
        self.validate_password_strength(password)?;
        
        let password = password.to_string();
        let argon2 = self.argon2.clone();
        
        // Run hashing in blocking task to avoid blocking async runtime
        let hash = tokio::task::spawn_blocking(move || {
            let salt = SaltString::generate(&mut OsRng);
            argon2
                .hash_password(password.as_bytes(), &salt)
                .map(|hash| hash.to_string())
                .map_err(|e| anyhow!("Failed to hash password: {}", e))
        })
        .await
        .context("Password hashing task panicked")??;
        
        Ok(hash)
    }
    
    /// Verify a password against its hash with constant-time comparison
    pub async fn verify_password(&self, password: &str, hash: &str) -> Result<bool> {
        let password = password.to_string();
        let hash = hash.to_string();
        let argon2 = self.argon2.clone();
        
        // Run verification in blocking task
        let result = tokio::task::spawn_blocking(move || {
            let parsed_hash = PasswordHash::new(&hash)
                .map_err(|e| anyhow!("Failed to parse password hash: {}", e))?;
            
            // Verify password (includes constant-time comparison)
            match argon2.verify_password(password.as_bytes(), &parsed_hash) {
                Ok(_) => Ok(true),
                Err(argon2::password_hash::Error::Password) => Ok(false),
                Err(e) => Err(anyhow!("Password verification error: {}", e)),
            }
        })
        .await
        .context("Password verification task panicked")??;
        
        Ok(result)
    }
    
    /// Validate password strength
    pub fn validate_password_strength(&self, password: &str) -> Result<()> {
        if password.len() < MIN_PASSWORD_LENGTH {
            return Err(anyhow!(
                "Password must be at least {} characters long",
                MIN_PASSWORD_LENGTH
            ));
        }
        
        let has_uppercase = password.chars().any(|c| c.is_uppercase());
        let has_lowercase = password.chars().any(|c| c.is_lowercase());
        let has_digit = password.chars().any(|c| c.is_numeric());
        let has_special = password.chars().any(|c| !c.is_alphanumeric());
        
        let mut missing = Vec::new();
        if !has_uppercase {
            missing.push("uppercase letter");
        }
        if !has_lowercase {
            missing.push("lowercase letter");
        }
        if !has_digit {
            missing.push("digit");
        }
        if !has_special {
            missing.push("special character");
        }
        
        if !missing.is_empty() {
            return Err(anyhow!(
                "Password must contain: {}",
                missing.join(", ")
            ));
        }
        
        Ok(())
    }
    
    /// Check rate limit for login attempts
    async fn check_rate_limit(&self, email: &str) -> Result<bool> {
        let rate_limit = self.rate_limit_repo
            .get_by_key("login", email)
            .await?;
        
        if let Some(limit) = rate_limit {
            // Check if locked out
            if limit.is_locked() {
                return Ok(false); // Still locked out
            }
        }
        
        Ok(true)
    }
    
    /// Record failed login attempt and trigger lockout if needed
    async fn record_failed_attempt(&self, email: &str) -> Result<()> {
        self.rate_limit_repo
            .increment("login", email, Some("/auth/login"), 300)
            .await?;
        
        // Check if we need to lock the account
        let rate_limit = self.rate_limit_repo
            .get_by_key("login", email)
            .await?;
        
        if let Some(limit) = rate_limit {
            if limit.request_count >= MAX_FAILED_ATTEMPTS {
                self.rate_limit_repo
                    .lock("login", email, LOCKOUT_DURATION_MINUTES as i32)
                    .await?;
            }
        }
        
        Ok(())
    }
    
    /// Reset failed attempts on successful login
    async fn reset_failed_attempts(&self, email: &str) -> Result<()> {
        self.rate_limit_repo
            .reset("login", email, None)
            .await?;
        
        Ok(())
    }
    
    /// Generate a new TOTP secret for MFA
    pub fn generate_totp_secret() -> Result<String> {
        let mut secret = vec![0u8; 20]; // 160 bits
        OsRng.fill_bytes(&mut secret);
        Ok(BASE32.encode(&secret))
    }
    
    /// Generate backup codes for MFA
    pub fn generate_backup_codes(count: usize) -> Result<Vec<String>> {
        let mut codes = Vec::with_capacity(count);
        for _ in 0..count {
            let mut code = vec![0u8; 8];
            OsRng.fill_bytes(&mut code);
            codes.push(hex::encode(code));
        }
        Ok(codes)
    }
    
    /// Verify a TOTP token
    pub fn verify_totp(secret: &str, token: &str) -> Result<bool> {
        let secret_bytes = BASE32
            .decode(secret.as_bytes())
            .map_err(|e| anyhow!("Failed to decode TOTP secret: {}", e))?;
        
        let time = (Utc::now().timestamp() as u64) / TOTP_TIME_STEP;
        
        // Check current time window and one window before/after for clock skew
        for time_offset in [-1i64, 0, 1] {
            let check_time = time.wrapping_add(time_offset as u64);
            let expected_token = totp_custom::<Sha1>(
                TOTP_TIME_STEP,
                TOTP_DIGITS,
                &secret_bytes,
                check_time,
            );
            
            // Constant-time comparison to prevent timing attacks
            if token.as_bytes().ct_eq(expected_token.as_bytes()).into() {
                return Ok(true);
            }
        }
        
        Ok(false)
    }
    
    /// Check if password has expired
    fn is_password_expired(&self, credential: &UserCredential) -> bool {
        if !self.enforce_password_expiration {
            return false;
        }
        
        credential.is_password_expired()
    }
    
    /// Get user permissions from database
    async fn get_user_permissions(&self, user_id: Uuid) -> Result<Vec<String>> {
        // TODO: Implement permission lookup from permission system
        // For now, return basic permissions based on user status
        Ok(vec![
            "auth:basic".to_string(),
            "profile:read".to_string(),
            "profile:write".to_string(),
        ])
    }
}

#[async_trait]
impl Provider for EmailPasswordProvider {
    async fn authenticate(&self, credentials: &Credentials) -> Result<AuthResult> {
        match credentials {
            Credentials::EmailPassword { email, password } => {
                // Check rate limit first
                if !self.check_rate_limit(email).await? {
                    return Err(anyhow!(
                        "Account temporarily locked due to too many failed attempts. Try again in {} minutes.",
                        LOCKOUT_DURATION_MINUTES
                    ));
                }
                
                // Fetch user from database
                let user = self.user_repo
                    .find_by_email(email)
                    .await?
                    .ok_or_else(|| anyhow!("Invalid credentials"))?;
                
                // Check user status
                if user.status != UserStatus::Active {
                    return Err(anyhow!("Account is not active. Status: {:?}", user.status));
                }
                
                // Check if user is locked
                if user.is_locked() {
                    return Err(anyhow!("Account is locked"));
                }
                
                // Fetch password credential
                let credential = self.credential_repo
                    .find_by_user_id(user.id)
                    .await?
                    .ok_or_else(|| anyhow!("Invalid credentials"))?;
                
                // Verify password
                let password_valid = self.verify_password(password, &credential.password_hash).await?;
                
                if !password_valid {
                    // Record failed attempt
                    self.record_failed_attempt(email).await?;
                    return Err(anyhow!("Invalid credentials"));
                }
                
                // Check if password has expired
                if self.is_password_expired(&credential) {
                    return Err(anyhow!(
                        "Password has expired. Please reset your password."
                    ));
                }
                
                // Reset failed attempts on successful password verification
                self.reset_failed_attempts(email).await?;
                
                // Get user permissions
                let permissions = self.get_user_permissions(user.id).await?;
                
                // Build auth result
                let mut claims = HashMap::new();
                claims.insert("user_id".to_string(), serde_json::json!(user.id.to_string()));
                claims.insert("email".to_string(), serde_json::json!(user.email.clone()));
                if let Some(full_name) = &user.full_name {
                    claims.insert("full_name".to_string(), serde_json::json!(full_name.clone()));
                }
                
                Ok(AuthResult {
                    user_id: user.id.to_string(),
                    email: user.email.clone(),
                    auth_method: "email_password".to_string(),
                    permissions,
                    claims,
                    cert_serial: None,
                    oauth_provider: None,
                    organization_id: user.organization_id,
                })
            }
            
            Credentials::EmailPasswordMfa { email, password, totp_token } => {
                // First, authenticate with email/password
                let initial_result = self.authenticate(&Credentials::EmailPassword {
                    email: email.clone(),
                    password: password.clone(),
                }).await?;
                
                // Fetch user to get TOTP secret
                let user = self.user_repo
                    .find_by_email(email)
                    .await?
                    .ok_or_else(|| anyhow!("User not found"))?;
                
                // Check if MFA is enabled
                let credential = self.credential_repo
                    .find_by_user_id(user.id)
                    .await?
                    .ok_or_else(|| anyhow!("Credentials not found"))?;
                
                if !credential.mfa_enabled {
                    return Err(anyhow!("MFA is not enabled for this account"));
                }
                
                let mfa_secret = credential.mfa_secret
                    .ok_or_else(|| anyhow!("MFA secret not found"))?;
                
                // Verify TOTP token
                let totp_valid = Self::verify_totp(&mfa_secret, totp_token)?;
                
                if !totp_valid {
                    return Err(anyhow!("Invalid MFA token"));
                }
                
                // Add MFA claim to result
                let mut result = initial_result;
                result.claims.insert("mfa_verified".to_string(), serde_json::json!("true"));
                result.auth_method = "email_password_mfa".to_string();
                
                Ok(result)
            }
            
            _ => Err(anyhow!("Invalid credentials type for email/password provider")),
        }
    }
    
    async fn user_exists(&self, identifier: &str) -> Result<bool> {
        Ok(self.user_repo.find_by_email(identifier).await?.is_some())
    }
    
    fn name(&self) -> &str {
        "email_password"
    }
}

impl EmailPasswordProvider {
    /// Register a new user with email and password
    pub async fn register_user(
        &self,
        email: &str,
        password: &str,
        full_name: Option<&str>,
        display_name: Option<&str>,
        organization_id: Uuid,
    ) -> Result<User> {
        // Check if user already exists
        if self.user_exists(email).await? {
            return Err(anyhow!("User with this email already exists"));
        }
        
        // Hash password
        let password_hash = self.hash_password(password).await?;
        
        // Create user
        let user = self.user_repo.create(
            email,
            full_name,
            display_name,
            organization_id,
        ).await?;
        
        // Create password credential with expiration
        let expires_at = Some(Utc::now() + Duration::days(PASSWORD_EXPIRATION_DAYS));
        
        self.credential_repo.create(
            user.id,
            &password_hash,
            PASSWORD_ALGORITHM,
            expires_at,
        ).await?;
        
        Ok(user)
    }
    
    /// Change user password
    pub async fn change_password(
        &self,
        user_id: Uuid,
        old_password: &str,
        new_password: &str,
    ) -> Result<()> {
        // Fetch user
        let user = self.user_repo
            .find_by_id(user_id)
            .await?
            .ok_or_else(|| anyhow!("User not found"))?;
        
        // Fetch current password credential
        let credential = self.credential_repo
            .find_by_user_id(user.id)
            .await?
            .ok_or_else(|| anyhow!("Password credential not found"))?;
        
        // Verify old password
        let old_password_valid = self.verify_password(old_password, &credential.password_hash).await?;
        
        if !old_password_valid {
            return Err(anyhow!("Invalid current password"));
        }
        
        // Hash new password
        let new_password_hash = self.hash_password(new_password).await?;
        
        // Update credential with new password and expiration
        let expires_at = Some(Utc::now() + Duration::days(PASSWORD_EXPIRATION_DAYS));
        
        self.credential_repo.update_password(
            user.id,
            &new_password_hash,
            PASSWORD_ALGORITHM,
            expires_at,
        ).await?;
        
        Ok(())
    }
    
    /// Enable MFA for a user
    pub async fn enable_mfa(&self, user_id: Uuid) -> Result<(String, Vec<String>)> {
        // Check if MFA already enabled
        let credential = self.credential_repo
            .find_by_user_id(user_id)
            .await?
            .ok_or_else(|| anyhow!("User credentials not found"))?;
        
        if credential.mfa_enabled {
            return Err(anyhow!("MFA is already enabled for this user"));
        }
        
        // Generate TOTP secret and backup codes
        let secret = Self::generate_totp_secret()?;
        let backup_codes = Self::generate_backup_codes(10)?;
        
        // Enable MFA in database
        self.credential_repo.enable_mfa(
            user_id,
            &secret,
            &backup_codes,
        ).await?;
        
        Ok((secret, backup_codes))
    }
    
    /// Disable MFA for a user
    pub async fn disable_mfa(&self, user_id: Uuid) -> Result<()> {
        self.credential_repo.disable_mfa(user_id).await?;
        Ok(())
    }
    
    /// Verify MFA token for a user
    pub async fn verify_user_mfa(&self, user_id: Uuid, token: &str) -> Result<bool> {
        let credential = self.credential_repo
            .find_by_user_id(user_id)
            .await?
            .ok_or_else(|| anyhow!("User credentials not found"))?;
        
        if !credential.mfa_enabled {
            return Err(anyhow!("MFA is not enabled for this user"));
        }
        
        let mfa_secret = credential.mfa_secret
            .ok_or_else(|| anyhow!("MFA secret not found"))?;
        
        Self::verify_totp(&mfa_secret, token)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_password_strength_validation() {
        // This test doesn't require database connection
        let params = Params::new(19456, 2, 1, Some(32)).unwrap();
        let argon2 = Argon2::new(argon2::Algorithm::Argon2id, Version::V0x13, params);
        
        let pool = PgPool::connect_lazy("").unwrap();
        let provider = EmailPasswordProvider {
            user_repo: Arc::new(UserRepository::new(DbPool::new(pool.clone()))),
            credential_repo: Arc::new(CredentialRepository::new(DbPool::new(pool.clone()))),
            rate_limit_repo: Arc::new(RateLimitRepository::new(DbPool::new(pool))),
            argon2,
            enforce_password_expiration: true,
        };
        
        // Too short
        assert!(provider.validate_password_strength("Short1!").is_err());
        
        // No uppercase
        assert!(provider.validate_password_strength("lowercase123!").is_err());
        
        // No lowercase
        assert!(provider.validate_password_strength("UPPERCASE123!").is_err());
        
        // No digit
        assert!(provider.validate_password_strength("NoDigitsHere!").is_err());
        
        // No special character
        assert!(provider.validate_password_strength("NoSpecial123").is_err());
        
        // Valid password
        assert!(provider.validate_password_strength("ValidPassword123!").is_ok());
    }
    
    #[test]
    fn test_totp_generation_and_verification() {
        let secret = EmailPasswordProvider::generate_totp_secret().unwrap();
        assert!(!secret.is_empty());
        
        // Generate a valid token
        let secret_bytes = BASE32.decode(secret.as_bytes()).unwrap();
        let time = (Utc::now().timestamp() as u64) / TOTP_TIME_STEP;
        let valid_token = totp_custom::<Sha1>(TOTP_TIME_STEP, TOTP_DIGITS, &secret_bytes, time);
        
        // Verify the token
        assert!(EmailPasswordProvider::verify_totp(&secret, &valid_token).unwrap());
        
        // Invalid token should fail
        assert!(!EmailPasswordProvider::verify_totp(&secret, "000000").unwrap());
    }
    
    #[test]
    fn test_backup_code_generation() {
        let codes = EmailPasswordProvider::generate_backup_codes(10).unwrap();
        assert_eq!(codes.len(), 10);
        
        // Each code should be unique and non-empty
        for code in &codes {
            assert!(!code.is_empty());
            assert_eq!(code.len(), 16); // 8 bytes = 16 hex chars
        }
    }
}
