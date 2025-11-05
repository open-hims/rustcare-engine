/// JWT and Refresh Token Services
/// 
/// Implements secure token generation, validation, and rotation for authentication

use crate::auth::config::TokenConfig;
use crate::auth::db::{JwtKeyRepository, RefreshTokenRepository, DbPool};
use anyhow::{anyhow, Context, Result};
use base64::{engine::general_purpose::STANDARD as BASE64, Engine};
use chrono::{DateTime, Duration, Utc};
use jsonwebtoken::{
    decode, encode, Algorithm, DecodingKey, EncodingKey, Header, Validation, TokenData,
};
use rand::Rng;
use rsa::{RsaPrivateKey, RsaPublicKey};
use rsa::pkcs1::{EncodeRsaPrivateKey, EncodeRsaPublicKey, LineEnding};
use serde::{Deserialize, Serialize};
use sha2::{Sha256, Digest};
use ipnetwork::IpNetwork;
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;
use uuid::Uuid;

// =============================================================================
// JWT TOKEN CLAIMS
// =============================================================================

/// JWT token claims structure
/// 
/// Implements standard JWT claims plus custom claims for authentication
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TokenClaims {
    /// Subject (user ID)
    pub sub: String,
    
    /// JWT ID (unique token identifier)
    pub jti: String,
    
    /// Session ID
    pub sid: String,
    
    /// Issued at timestamp (seconds since epoch)
    pub iat: i64,
    
    /// Expiration timestamp (seconds since epoch)
    pub exp: i64,
    
    /// Not before timestamp
    #[serde(skip_serializing_if = "Option::is_none")]
    pub nbf: Option<i64>,
    
    /// Issuer
    pub iss: String,
    
    /// Audience
    #[serde(skip_serializing_if = "Option::is_none")]
    pub aud: Option<String>,
    
    /// Authentication method (email, oauth, certificate)
    pub auth_method: String,
    
    /// Certificate serial (if cert auth)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub cert_serial: Option<String>,
    
    /// User email
    #[serde(skip_serializing_if = "Option::is_none")]
    pub email: Option<String>,
    
    /// User permissions
    #[serde(skip_serializing_if = "Option::is_none")]
    pub permissions: Option<Vec<String>>,
    
    /// Step-up authentication flag (for sensitive operations)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub step_up: Option<bool>,
    
    /// Additional custom claims
    #[serde(flatten)]
    pub extra: HashMap<String, serde_json::Value>,
}

impl TokenClaims {
    /// Create new token claims
    pub fn new(
        user_id: Uuid,
        session_id: Uuid,
        auth_method: String,
        issuer: String,
        ttl_seconds: i64,
    ) -> Self {
        let now = Utc::now().timestamp();
        Self {
            sub: user_id.to_string(),
            jti: Uuid::new_v4().to_string(),
            sid: session_id.to_string(),
            iat: now,
            exp: now + ttl_seconds,
            nbf: Some(now),
            iss: issuer,
            aud: None,
            auth_method,
            cert_serial: None,
            email: None,
            permissions: None,
            step_up: None,
            extra: HashMap::new(),
        }
    }
    
    /// Check if token is expired
    pub fn is_expired(&self) -> bool {
        let now = Utc::now().timestamp();
        self.exp <= now
    }
    
    /// Check if token is not yet valid
    pub fn is_not_yet_valid(&self) -> bool {
        if let Some(nbf) = self.nbf {
            let now = Utc::now().timestamp();
            now < nbf
        } else {
            false
        }
    }
    
    /// Get user ID as UUID
    pub fn user_id(&self) -> Result<Uuid> {
        Uuid::parse_str(&self.sub).context("Invalid user ID in token")
    }
    
    /// Get session ID as UUID
    pub fn session_id(&self) -> Result<Uuid> {
        Uuid::parse_str(&self.sid).context("Invalid session ID in token")
    }
}

// =============================================================================
// JWKS (JSON Web Key Set)
// =============================================================================

/// JSON Web Key for JWKS endpoint
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct JsonWebKey {
    /// Key type (RSA)
    pub kty: String,
    
    /// Key ID
    pub kid: String,
    
    /// Algorithm
    pub alg: String,
    
    /// Usage (sig for signature)
    #[serde(rename = "use")]
    pub usage: String,
    
    /// RSA modulus (base64url encoded)
    pub n: String,
    
    /// RSA exponent (base64url encoded)
    pub e: String,
}

/// JWKS response
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct JwksResponse {
    pub keys: Vec<JsonWebKey>,
}

// =============================================================================
// JWT SERVICE
// =============================================================================

/// JWT token service
/// 
/// Handles JWT generation, validation, and key rotation
pub struct JwtService {
    config: TokenConfig,
    key_repo: Arc<JwtKeyRepository>,
    
    /// Cache of current signing key
    current_key_cache: Arc<RwLock<Option<CachedSigningKey>>>,
    
    /// Cache of JWKS for validation
    jwks_cache: Arc<RwLock<Option<(DateTime<Utc>, JwksResponse)>>>,
}

#[derive(Clone)]
struct CachedSigningKey {
    kid: String,
    encoding_key: EncodingKey,
    cached_at: DateTime<Utc>,
}

impl JwtService {
    /// Create new JWT service
    pub fn new(config: TokenConfig, pool: DbPool) -> Self {
        Self {
            config,
            key_repo: Arc::new(JwtKeyRepository::new(pool)),
            current_key_cache: Arc::new(RwLock::new(None)),
            jwks_cache: Arc::new(RwLock::new(None)),
        }
    }
    
    /// Generate a new JWT access token
    pub async fn generate_token(&self, claims: &TokenClaims) -> Result<String> {
        // Get current signing key
        let cached_key = self.get_current_signing_key().await?;
        
        // Create JWT header with key ID
        let mut header = Header::new(Algorithm::RS256);
        header.kid = Some(cached_key.kid.clone());
        
        // Encode token
        let token = encode(&header, claims, &cached_key.encoding_key)
            .context("Failed to encode JWT token")?;
        
        // Increment usage counter in background
        let kid = cached_key.kid.clone();
        let key_repo = self.key_repo.clone();
        tokio::spawn(async move {
            let _ = key_repo.increment_tokens_signed(&kid).await;
        });
        
        Ok(token)
    }
    
    /// Validate and decode JWT token
    pub async fn validate_token(&self, token: &str) -> Result<TokenData<TokenClaims>> {
        // Get JWKS for validation
        let jwks = self.get_jwks().await?;
        
        // Try to decode with each key (for key rotation support)
        let mut last_error = None;
        for key in &jwks.keys {
            match self.validate_with_key(token, key).await {
                Ok(token_data) => return Ok(token_data),
                Err(e) => last_error = Some(e),
            }
        }
        
        Err(last_error.unwrap_or_else(|| anyhow!("No valid signing keys available")))
    }
    
    /// Validate token with specific key
    async fn validate_with_key(
        &self,
        token: &str,
        jwk: &JsonWebKey,
    ) -> Result<TokenData<TokenClaims>> {
        // Decode RSA public key from JWK
        let n_bytes = BASE64.decode(&jwk.n).context("Invalid modulus in JWK")?;
        let e_bytes = BASE64.decode(&jwk.e).context("Invalid exponent in JWK")?;
        
        let public_key = RsaPublicKey::new(
            rsa::BigUint::from_bytes_be(&n_bytes),
            rsa::BigUint::from_bytes_be(&e_bytes),
        )
        .context("Failed to create RSA public key")?;
        
        let pem = public_key.to_pkcs1_pem(LineEnding::LF)
            .context("Failed to encode public key as PEM")?;
        
        let decoding_key = DecodingKey::from_rsa_pem(pem.as_bytes())
            .context("Failed to create decoding key")?;
        
        // Set up validation
        let mut validation = Validation::new(Algorithm::RS256);
        validation.set_issuer(&[&self.config.issuer]);
        if let Some(ref audience) = self.config.audience {
            validation.set_audience(&[audience]);
        }
        validation.validate_exp = true;
        validation.validate_nbf = true;
        
        // Decode and validate token
        decode::<TokenClaims>(token, &decoding_key, &validation)
            .context("Token validation failed")
    }
    
    /// Get current signing key (with caching)
    async fn get_current_signing_key(&self) -> Result<CachedSigningKey> {
        // Check cache first
        {
            let cache = self.current_key_cache.read().await;
            if let Some(ref cached) = *cache {
                // Cache valid for 5 minutes
                if Utc::now() - cached.cached_at < Duration::minutes(5) {
                    return Ok(cached.clone());
                }
            }
        }
        
        // Cache miss or expired, fetch from database
        let key = self.key_repo.get_primary().await?
            .ok_or_else(|| anyhow!("No primary signing key found"))?;
        
        // Parse private key
        let encoding_key = EncodingKey::from_rsa_pem(key.private_key_pem.as_bytes())
            .context("Failed to parse private key")?;
        
        let cached = CachedSigningKey {
            kid: key.kid.clone(),
            encoding_key,
            cached_at: Utc::now(),
        };
        
        // Update cache
        {
            let mut cache = self.current_key_cache.write().await;
            *cache = Some(cached.clone());
        }
        
        Ok(cached)
    }
    
    /// Get JWKS (JSON Web Key Set) for token validation
    pub async fn get_jwks(&self) -> Result<JwksResponse> {
        // Check cache first
        {
            let cache = self.jwks_cache.read().await;
            if let Some((cached_at, ref jwks)) = *cache {
                // Cache valid for 5 minutes
                if Utc::now() - cached_at < Duration::minutes(5) {
                    return Ok(jwks.clone());
                }
            }
        }
        
        // Cache miss or expired, fetch from database
        let keys = self.key_repo.get_active_keys().await?;
        
        let mut jwks = JwksResponse { keys: Vec::new() };
        
        for key in keys {
            // Parse public key from PEM
            use rsa::pkcs1::DecodeRsaPublicKey;
            use rsa::traits::PublicKeyParts;
            
            let public_key = RsaPublicKey::from_pkcs1_pem(&key.public_key_pem)
                .context("Failed to parse public key")?;
            
            // Extract modulus and exponent using PublicKeyParts trait
            let n = BASE64.encode(public_key.n().to_bytes_be());
            let e = BASE64.encode(public_key.e().to_bytes_be());
            
            jwks.keys.push(JsonWebKey {
                kty: "RSA".to_string(),
                kid: key.kid.clone(),
                alg: key.algorithm.clone(),
                usage: "sig".to_string(),
                n,
                e,
            });
        }
        
        // Update cache
        {
            let mut cache = self.jwks_cache.write().await;
            *cache = Some((Utc::now(), jwks.clone()));
        }
        
        Ok(jwks)
    }
    
    /// Generate a new RSA-4096 signing key pair
    pub async fn generate_new_key_pair(&self) -> Result<(String, String)> {
        // Generate RSA-4096 key pair (in blocking task for CPU-intensive work)
        let (private_pem, public_pem) = tokio::task::spawn_blocking(|| -> Result<(String, String)> {
            let mut rng = rand::thread_rng();
            let bits = 4096;
            
            let private_key = RsaPrivateKey::new(&mut rng, bits)
                .context("Failed to generate RSA private key")?;
            
            let public_key = RsaPublicKey::from(&private_key);
            
            let private_pem = private_key.to_pkcs1_pem(LineEnding::LF)
                .context("Failed to encode private key")?
                .to_string();
            
            let public_pem = public_key.to_pkcs1_pem(LineEnding::LF)
                .context("Failed to encode public key")?;
            
            Ok((private_pem, public_pem))
        })
        .await
        .context("Key generation task failed")??;
        
        Ok((private_pem, public_pem))
    }
    
    /// Initialize signing keys (create first key if none exists)
    pub async fn initialize(&self) -> Result<()> {
        // Check if we have a primary key
        if let Some(_) = self.key_repo.get_primary().await? {
            tracing::info!("JWT signing key already initialized");
            return Ok(());
        }
        
        tracing::info!("Initializing first JWT signing key...");
        
        // Generate new key pair
        let (private_pem, public_pem) = self.generate_new_key_pair().await?;
        
        // Create key ID
        let kid = format!("key-{}", Uuid::new_v4());
        
        // Store in database as primary key (SYSTEM_ORGANIZATION_ID for system-wide keys)
        self.key_repo.create(
            Some(crate::auth::models::SYSTEM_ORGANIZATION_ID),
            &kid,
            "RS256",
            &private_pem,
            &public_pem,
            Some(4096),
            true, // is_primary
        ).await?;
        
        tracing::info!("JWT signing key initialized: {}", kid);
        
        Ok(())
    }
    
    /// Rotate signing key (generate new key and retire old one)
    pub async fn rotate_key(&self, reason: Option<&str>) -> Result<String> {
        tracing::info!("Starting JWT key rotation");
        
        // Get current primary key
        let old_key = self.key_repo.get_primary().await?
            .ok_or_else(|| anyhow!("No primary key to rotate"))?;
        
        // Generate new key pair
        let (private_pem, public_pem) = self.generate_new_key_pair().await?;
        let new_kid = format!("key-{}", Uuid::new_v4());
        
        // Create new key (not primary yet) - SYSTEM_ORGANIZATION_ID for system-wide
        self.key_repo.create(
            Some(crate::auth::models::SYSTEM_ORGANIZATION_ID),
            &new_kid,
            "RS256",
            &private_pem,
            &public_pem,
            Some(4096),
            false,
        ).await?;
        
        // Promote new key to primary (this demotes the old one)
        self.key_repo.set_primary(&new_kid).await?;
        
        // Mark old key as rotating (keep for grace period)
        self.key_repo.start_rotation(&old_key.kid).await?;
        
        // Schedule retirement after grace period
        let grace_period_days = self.config.key_rotation_grace_period_days;
        let expires_at = Utc::now() + Duration::days(grace_period_days as i64);
        
        tokio::spawn({
            let key_repo = self.key_repo.clone();
            let old_kid = old_key.kid.clone();
            let reason = reason.map(|s| s.to_string());
            async move {
                tokio::time::sleep(tokio::time::Duration::from_secs(
                    (grace_period_days * 24 * 60 * 60) as u64
                )).await;
                
                let _ = key_repo.retire(&old_kid, reason.as_deref(), Some(expires_at)).await;
                tracing::info!("Retired old JWT signing key: {}", old_kid);
            }
        });
        
        // Clear caches
        {
            let mut cache = self.current_key_cache.write().await;
            *cache = None;
        }
        {
            let mut cache = self.jwks_cache.write().await;
            *cache = None;
        }
        
        tracing::info!("JWT key rotation complete: {} -> {}", old_key.kid, new_kid);
        
        Ok(new_kid)
    }
}

// =============================================================================
// REFRESH TOKEN SERVICE
// =============================================================================

/// Refresh token service
/// 
/// Handles refresh token generation, validation, rotation, and revocation
pub struct RefreshTokenService {
    config: TokenConfig,
    token_repo: Arc<RefreshTokenRepository>,
}

impl RefreshTokenService {
    /// Create new refresh token service
    pub fn new(config: TokenConfig, pool: DbPool) -> Self {
        Self {
            config,
            token_repo: Arc::new(RefreshTokenRepository::new(pool)),
        }
    }
    
    /// Generate a new refresh token
    #[allow(clippy::too_many_arguments)]
    pub async fn generate_refresh_token(
        &self,
        user_id: Uuid,
        token_family: Option<Uuid>,
        device_name: Option<&str>,
        device_fingerprint: Option<&str>,
        user_agent: Option<&str>,
        ip_address: Option<IpNetwork>,
        auth_method: Option<&str>,
        cert_serial: Option<&str>,
        parent_token_id: Option<Uuid>,
    ) -> Result<(String, Uuid)> {
        // Generate cryptographically secure random token (32 bytes)
        let mut rng = rand::thread_rng();
        let token_bytes: [u8; 32] = rng.gen();
        let token = BASE64.encode(token_bytes);
        
        // Hash token for storage
        let token_hash = self.hash_token(&token);
        
        // Determine token family (create new if not rotating)
        let family = token_family.unwrap_or_else(Uuid::new_v4);
        
        // Calculate expiration
        let expires_at = Utc::now() + Duration::days(self.config.refresh_token_ttl_days as i64);
        
        // Store in database
        let stored_token = self.token_repo.create(
            user_id,
            &token_hash,
            family,
            device_name,
            device_fingerprint,
            user_agent,
            ip_address,
            expires_at,
            auth_method,
            cert_serial,
            parent_token_id,
        ).await?;
        
        Ok((token, stored_token.id))
    }
    
    /// Validate refresh token and rotate it
    pub async fn validate_and_rotate(
        &self,
        token: &str,
        device_fingerprint: Option<&str>,
        user_agent: Option<&str>,
        ip_address: Option<IpNetwork>,
    ) -> Result<(Uuid, String, Uuid)> {
        // Hash token
        let token_hash = self.hash_token(token);
        
        // Look up token
        let stored_token = self.token_repo.find_by_hash(&token_hash).await?
            .ok_or_else(|| anyhow!("Invalid refresh token"))?;
        
        // Check if token is valid
        if !stored_token.is_valid() {
            // If token was revoked due to rotation, this might be a reuse attack
            if stored_token.revoked {
                // Revoke entire token family (all tokens in rotation chain)
                self.token_repo.revoke_token_family(
                    stored_token.token_family,
                    "Token reuse detected - possible theft"
                ).await?;
                
                tracing::warn!(
                    "Refresh token reuse detected for user {}, revoking token family {}",
                    stored_token.user_id,
                    stored_token.token_family
                );
                
                return Err(anyhow!("Token reuse detected - security violation"));
            }
            
            return Err(anyhow!("Refresh token expired or revoked"));
        }
        
        // Validate device fingerprint if provided
        if let Some(fingerprint) = device_fingerprint {
            if let Some(ref stored_fp) = stored_token.device_fingerprint {
                if fingerprint != stored_fp {
                    tracing::warn!(
                        "Device fingerprint mismatch for refresh token (user {})",
                        stored_token.user_id
                    );
                    return Err(anyhow!("Device fingerprint mismatch"));
                }
            }
        }
        
        // Update last used timestamp
        self.token_repo.update_last_used(stored_token.id).await?;
        
        // Generate new refresh token (rotation)
        let (new_token, new_token_id) = self.generate_refresh_token(
            stored_token.user_id,
            Some(stored_token.token_family), // Same family
            stored_token.device_name.as_deref(),
            device_fingerprint.or(stored_token.device_fingerprint.as_deref()),
            user_agent.or(stored_token.user_agent.as_deref()),
            ip_address.or(stored_token.ip_address),
            stored_token.auth_method.as_deref(),
            stored_token.cert_serial.as_deref(),
            Some(stored_token.id), // Track parent
        ).await?;
        
        // Mark old token as replaced
        self.token_repo.mark_replaced(stored_token.id, new_token_id).await?;
        
        Ok((stored_token.user_id, new_token, new_token_id))
    }
    
    /// Revoke a refresh token
    pub async fn revoke_token(&self, token: &str, reason: &str) -> Result<()> {
        let token_hash = self.hash_token(token);
        
        let stored_token = self.token_repo.find_by_hash(&token_hash).await?
            .ok_or_else(|| anyhow!("Refresh token not found"))?;
        
        self.token_repo.revoke(stored_token.id, reason).await?;
        
        Ok(())
    }
    
    /// Revoke all refresh tokens for a user
    pub async fn revoke_all_user_tokens(&self, user_id: Uuid, reason: &str) -> Result<u64> {
        let count = self.token_repo.revoke_all_user_tokens(user_id, reason).await?;
        Ok(count)
    }
    
    /// Hash token with SHA-256
    fn hash_token(&self, token: &str) -> String {
        let mut hasher = Sha256::new();
        hasher.update(token.as_bytes());
        let result = hasher.finalize();
        BASE64.encode(result)
    }
    
    /// Cleanup expired tokens
    pub async fn cleanup_expired(&self) -> Result<u64> {
        let count = self.token_repo.cleanup_expired().await?;
        Ok(count)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_token_claims_expiration() {
        let claims = TokenClaims::new(
            Uuid::new_v4(),
            Uuid::new_v4(),
            "email".to_string(),
            "rustcare".to_string(),
            300, // 5 minutes
        );
        
        assert!(!claims.is_expired());
        assert!(!claims.is_not_yet_valid());
    }
}
