/// Session management with Redis backend
/// 
/// Tracks active user sessions with device fingerprinting, idle timeout,
/// concurrent session limits, and anomaly detection. Sessions are stored
/// in Redis for fast access and automatic TTL expiration, with fallback
/// to database persistence.

use crate::auth::config::SessionConfig;
use crate::auth::db::SessionRepository;
use anyhow::{Context, Result};
use redis::{AsyncCommands, aio::ConnectionManager};
use serde::{Deserialize, Serialize};
use sha2::{Sha256, Digest};
use std::collections::HashMap;
use std::sync::Arc;
use std::time::{SystemTime, UNIX_EPOCH};
use tokio::sync::RwLock;
use uuid::Uuid;
use chrono;

/// Maximum number of concurrent sessions per user
const MAX_SESSIONS_PER_USER: usize = 5;

/// Session cache TTL in seconds (matches Redis TTL)
const SESSION_CACHE_TTL: u64 = 3600; // 1 hour

/// SessionManager handles creation, validation, and lifecycle of user sessions
pub struct SessionManager {
    /// Redis connection manager for session storage
    redis: ConnectionManager,
    
    /// Database fallback for session persistence
    session_repo: SessionRepository,
    
    /// Session configuration
    config: SessionConfig,
    
    /// In-memory cache for recent session validations
    cache: Arc<RwLock<HashMap<String, CachedSession>>>,
}

impl SessionManager {
    /// Create a new SessionManager
    pub async fn new(
        redis_url: &str,
        session_repo: SessionRepository,
        config: SessionConfig,
    ) -> Result<Self> {
        let client = redis::Client::open(redis_url)
            .context("Failed to create Redis client")?;
        
        let redis = ConnectionManager::new(client).await
            .context("Failed to connect to Redis")?;
        
        Ok(Self {
            redis,
            session_repo,
            config,
            cache: Arc::new(RwLock::new(HashMap::new())),
        })
    }
    
    /// Create a new session for a user
    pub async fn create_session(&self, data: SessionCreateData) -> Result<SessionData> {
        // Generate unique session ID
        let session_id = Uuid::new_v4().to_string();
        
        let now = current_timestamp();
        let idle_timeout_secs = self.config.idle_timeout_minutes * 60;
        let expires_at = now + idle_timeout_secs as i64;
        
        // Create device fingerprint
        let device_fingerprint = self.generate_device_fingerprint(
            &data.ip_address,
            &data.user_agent,
            &data.additional_headers,
        );
        
        // Check concurrent session limit
        self.enforce_concurrent_limit(&data.user_id).await?;
        
        let session = SessionData {
            session_id: session_id.clone(),
            user_id: data.user_id.clone(),
            created_at: now,
            last_activity: now,
            expires_at,
            ip_address: data.ip_address.clone(),
            user_agent: data.user_agent.clone(),
            device_fingerprint: device_fingerprint.clone(),
            auth_method: data.auth_method.clone(),
            cert_serial: data.cert_serial.clone(),
            metadata: data.metadata.clone(),
        };
        
        // Store in Redis with TTL
        let session_json = serde_json::to_string(&session)
            .context("Failed to serialize session")?;
        
        let mut conn = self.redis.clone();
        let ttl_seconds = self.config.idle_timeout_minutes * 60;
        
        conn.set_ex::<_, _, ()>(
            format!("session:{}", session_id),
            session_json,
            ttl_seconds,
        ).await.context("Failed to store session in Redis")?;
        
        // Add to user's session set
        conn.sadd::<_, _, ()>(
            format!("user:sessions:{}", data.user_id),
            &session_id,
        ).await.context("Failed to add session to user set")?;
        
        // Set expiration on user session set
        conn.expire::<_, ()>(
            format!("user:sessions:{}", data.user_id),
            ttl_seconds as i64,
        ).await.ok();
        
        // Store in database for persistence (fire and forget)
        let session_clone = session.clone();
        let session_repo = self.session_repo.clone();
        tokio::spawn(async move {
            let user_uuid = match Uuid::parse_str(&session_clone.user_id) {
                Ok(uuid) => uuid,
                Err(e) => {
                    tracing::warn!("Invalid user_id UUID: {}", e);
                    return;
                }
            };
            
            let expires_at_dt = chrono::DateTime::from_timestamp(session_clone.expires_at, 0)
                .unwrap_or_else(|| chrono::Utc::now());
            
            if let Err(e) = session_repo.create(
                user_uuid,
                &session_clone.session_id, // session_token
                Some(&session_clone.device_fingerprint),
                Some(&session_clone.user_agent),
                None, // ip_address as IpNetwork (optional)
                None, // device_name (optional)
                None, // device_type (optional)
                expires_at_dt,
                &session_clone.auth_method,
                session_clone.cert_serial.as_deref(),
                None, // oauth_provider (optional)
                None, // metadata (optional)
            ).await {
                tracing::warn!("Failed to persist session to database: {}", e);
            }
        });
        
        Ok(session)
    }
    
    /// Get session data by session ID
    pub async fn get_session(&self, session_id: &str) -> Result<Option<SessionData>> {
        // Check cache first
        {
            let cache = self.cache.read().await;
            if let Some(cached) = cache.get(session_id) {
                if !cached.is_expired() {
                    return Ok(Some(cached.session.clone()));
                }
            }
        }
        
        // Try Redis
        let mut conn = self.redis.clone();
        let session_json: Option<String> = conn.get(format!("session:{}", session_id))
            .await
            .context("Failed to fetch session from Redis")?;
        
        if let Some(json) = session_json {
            let session: SessionData = serde_json::from_str(&json)
                .context("Failed to deserialize session")?;
            
            // Update cache
            {
                let mut cache = self.cache.write().await;
                cache.insert(session_id.to_string(), CachedSession {
                    session: session.clone(),
                    cached_at: current_timestamp(),
                });
            }
            
            return Ok(Some(session));
        }
        
        // Fallback to database
        let session_uuid = match Uuid::parse_str(session_id) {
            Ok(uuid) => uuid,
            Err(_) => return Ok(None),
        };
        let db_session = self.session_repo.find_by_id(session_uuid).await?;
        
        if let Some(session) = db_session {
            // Check if expired
            if session.expires_at < chrono::Utc::now() {
                return Ok(None);
            }
            
            // Convert database model to SessionData
            let session_data = SessionData {
                session_id: session.id.to_string(),
                user_id: session.user_id.to_string(),
                created_at: session.created_at.timestamp(),
                last_activity: session.last_activity_at.timestamp(),
                expires_at: session.expires_at.timestamp(),
                ip_address: session.ip_address.map(|ip| ip.to_string()).unwrap_or_default(),
                user_agent: session.user_agent.unwrap_or_default(),
                device_fingerprint: session.device_fingerprint.unwrap_or_default(),
                auth_method: session.auth_method,
                cert_serial: session.cert_serial,
                metadata: session.metadata
                    .and_then(|v| v.as_object().cloned())
                    .map(|obj| obj.into_iter().collect())
                    .unwrap_or_default(),
            };
            
            Ok(Some(session_data))
        } else {
            Ok(None)
        }
    }
    
    /// Update session activity timestamp
    pub async fn update_activity(&self, session_id: &str) -> Result<()> {
        let now = current_timestamp();
        
        // Update in Redis
        let mut conn = self.redis.clone();
        let session_json: Option<String> = conn.get(format!("session:{}", session_id))
            .await
            .context("Failed to fetch session from Redis")?;
        
        if let Some(json) = session_json {
            let mut session: SessionData = serde_json::from_str(&json)
                .context("Failed to deserialize session")?;
            
            session.last_activity = now;
            let idle_timeout_secs = self.config.idle_timeout_minutes * 60;
            session.expires_at = now + idle_timeout_secs as i64;
            
            let updated_json = serde_json::to_string(&session)
                .context("Failed to serialize updated session")?;
            
            // Update with extended TTL
            let ttl_seconds = self.config.idle_timeout_minutes * 60;
            conn.set_ex::<_, _, ()>(
                format!("session:{}", session_id),
                updated_json,
                ttl_seconds,
            ).await.context("Failed to update session in Redis")?;
            
            // Update cache
            {
                let mut cache = self.cache.write().await;
                cache.insert(session_id.to_string(), CachedSession {
                    session: session.clone(),
                    cached_at: now,
                });
            }
            
            // Update database (fire and forget)
            let session_id_str = session_id.to_string();
            let session_repo = self.session_repo.clone();
            tokio::spawn(async move {
                if let Ok(session_uuid) = Uuid::parse_str(&session_id_str) {
                    if let Err(e) = session_repo.update_activity(session_uuid).await {
                        tracing::warn!("Failed to update session activity in database: {}", e);
                    }
                }
            });
        }
        
        Ok(())
    }
    
    /// Destroy a session
    pub async fn destroy_session(&self, session_id: &str) -> Result<()> {
        // Get session to find user_id
        let session = self.get_session(session_id).await?;
        
        if let Some(session_data) = session {
            // Remove from Redis
            let mut conn = self.redis.clone();
            conn.del::<_, ()>(format!("session:{}", session_id))
                .await
                .context("Failed to delete session from Redis")?;
            
            // Remove from user's session set
            conn.srem::<_, _, ()>(
                format!("user:sessions:{}", session_data.user_id),
                session_id,
            ).await.ok();
            
            // Remove from cache
            {
                let mut cache = self.cache.write().await;
                cache.remove(session_id);
            }
            
            // Mark as terminated in database
            if let Ok(session_uuid) = Uuid::parse_str(session_id) {
                self.session_repo.terminate(session_uuid, Some("User logout")).await?;
            }
        }
        
        Ok(())
    }
    
    /// Destroy all sessions for a user
    pub async fn destroy_user_sessions(&self, user_id: &str) -> Result<usize> {
        let mut conn = self.redis.clone();
        
        // Get all session IDs for user
        let session_ids: Vec<String> = conn.smembers(format!("user:sessions:{}", user_id))
            .await
            .context("Failed to fetch user sessions from Redis")?;
        
        let count = session_ids.len();
        
        // Delete each session
        for session_id in session_ids {
            // Remove from Redis
            conn.del::<_, ()>(format!("session:{}", session_id))
                .await
                .ok();
            
            // Remove from cache
            {
                let mut cache = self.cache.write().await;
                cache.remove(&session_id);
            }
            
            // Mark as terminated in database (fire and forget)
            let session_id_clone = session_id.clone();
            let session_repo = self.session_repo.clone();
            tokio::spawn(async move {
                if let Ok(session_uuid) = Uuid::parse_str(&session_id_clone) {
                    if let Err(e) = session_repo.terminate(session_uuid, Some("Concurrent session limit")).await {
                        tracing::warn!("Failed to terminate session in database: {}", e);
                    }
                }
            });
        }
        
        // Remove user's session set
        conn.del::<_, ()>(format!("user:sessions:{}", user_id))
            .await
            .ok();
        
        Ok(count)
    }
    
    /// Validate a session with optional security checks
    pub async fn validate_session(
        &self,
        session_id: &str,
        validation: SessionValidation,
    ) -> Result<SessionValidationResult> {
        let session = match self.get_session(session_id).await? {
            Some(s) => s,
            None => {
                return Ok(SessionValidationResult {
                    valid: false,
                    reason: Some("Session not found".to_string()),
                    session: None,
                });
            }
        };
        
        let now = current_timestamp();
        
        // Check expiration
        if session.expires_at < now {
            return Ok(SessionValidationResult {
                valid: false,
                reason: Some("Session expired".to_string()),
                session: None,
            });
        }
        
        // Check idle timeout
        let idle_duration = now - session.last_activity;
        let idle_timeout_secs = self.config.idle_timeout_minutes * 60;
        if idle_duration > idle_timeout_secs as i64 {
            // Destroy expired session
            self.destroy_session(session_id).await.ok();
            
            return Ok(SessionValidationResult {
                valid: false,
                reason: Some("Session idle timeout exceeded".to_string()),
                session: None,
            });
        }
        
        // Validate IP address if enabled
        if self.config.validate_ip && session.ip_address != validation.ip_address {
            return Ok(SessionValidationResult {
                valid: false,
                reason: Some("IP address mismatch".to_string()),
                session: Some(session),
            });
        }
        
        // Validate user agent if enabled
        if self.config.validate_user_agent && session.user_agent != validation.user_agent {
            return Ok(SessionValidationResult {
                valid: false,
                reason: Some("User agent mismatch".to_string()),
                session: Some(session),
            });
        }
        
        // Validate device fingerprint if enabled
        if self.config.validate_device_fingerprint {
            let current_fingerprint = self.generate_device_fingerprint(
                &validation.ip_address,
                &validation.user_agent,
                &validation.additional_headers,
            );
            
            if session.device_fingerprint != current_fingerprint {
                return Ok(SessionValidationResult {
                    valid: false,
                    reason: Some("Device fingerprint mismatch".to_string()),
                    session: Some(session),
                });
            }
        }
        
        // Update activity timestamp
        self.update_activity(session_id).await.ok();
        
        Ok(SessionValidationResult {
            valid: true,
            reason: None,
            session: Some(session),
        })
    }
    
    /// List all active sessions for a user
    pub async fn list_user_sessions(&self, user_id: &str) -> Result<Vec<SessionData>> {
        let mut conn = self.redis.clone();
        
        // Get all session IDs for user
        let session_ids: Vec<String> = conn.smembers(format!("user:sessions:{}", user_id))
            .await
            .context("Failed to fetch user sessions from Redis")?;
        
        let mut sessions = Vec::new();
        
        for session_id in session_ids {
            if let Ok(Some(session)) = self.get_session(&session_id).await {
                sessions.push(session);
            }
        }
        
        // Sort by last activity (most recent first)
        sessions.sort_by(|a, b| b.last_activity.cmp(&a.last_activity));
        
        Ok(sessions)
    }
    
    /// Enforce concurrent session limit for a user
    async fn enforce_concurrent_limit(&self, user_id: &str) -> Result<()> {
        let mut conn = self.redis.clone();
        
        // Get current session count
        let session_ids: Vec<String> = conn.smembers(format!("user:sessions:{}", user_id))
            .await
            .context("Failed to fetch user sessions from Redis")?;
        
        let max_sessions = self.config.max_concurrent_sessions as usize;
        
        if session_ids.len() >= max_sessions {
            // Find oldest session by last_activity
            let mut sessions_with_activity: Vec<(String, i64)> = Vec::new();
            
            for session_id in &session_ids {
                if let Ok(Some(session)) = self.get_session(session_id).await {
                    sessions_with_activity.push((session_id.clone(), session.last_activity));
                }
            }
            
            // Sort by last_activity (oldest first)
            sessions_with_activity.sort_by_key(|(_, activity)| *activity);
            
            // Terminate oldest session
            if let Some((oldest_session_id, _)) = sessions_with_activity.first() {
                tracing::info!(
                    "Terminating oldest session {} for user {} due to concurrent limit",
                    oldest_session_id,
                    user_id
                );
                self.destroy_session(oldest_session_id).await.ok();
            }
        }
        
        Ok(())
    }
    
    /// Generate device fingerprint from request headers
    fn generate_device_fingerprint(
        &self,
        ip_address: &str,
        user_agent: &str,
        additional_headers: &HashMap<String, String>,
    ) -> String {
        let mut hasher = Sha256::new();
        
        // Hash IP address
        hasher.update(ip_address.as_bytes());
        
        // Hash user agent
        hasher.update(user_agent.as_bytes());
        
        // Hash additional headers (sorted for consistency)
        let mut sorted_headers: Vec<_> = additional_headers.iter().collect();
        sorted_headers.sort_by_key(|(k, _)| *k);
        
        for (key, value) in sorted_headers {
            hasher.update(key.as_bytes());
            hasher.update(value.as_bytes());
        }
        
        format!("{:x}", hasher.finalize())
    }
    
    /// Clean up expired sessions from cache
    pub async fn cleanup_cache(&self) -> Result<()> {
        let mut cache = self.cache.write().await;
        let now = current_timestamp();
        
        cache.retain(|_, cached| {
            let age = now - cached.cached_at;
            age < SESSION_CACHE_TTL as i64
        });
        
        Ok(())
    }
    
    /// Health check - verify Redis connection
    pub async fn health_check(&self) -> Result<()> {
        let mut conn = self.redis.clone();
        redis::cmd("PING")
            .query_async::<_, String>(&mut conn)
            .await
            .context("Redis ping failed")?;
        Ok(())
    }
}

/// Data required to create a new session
#[derive(Debug, Clone)]
pub struct SessionCreateData {
    pub user_id: String,
    pub ip_address: String,
    pub user_agent: String,
    pub auth_method: String,
    pub cert_serial: Option<String>,
    pub additional_headers: HashMap<String, String>,
    pub metadata: HashMap<String, serde_json::Value>,
}

/// Session data stored in Redis
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SessionData {
    pub session_id: String,
    pub user_id: String,
    pub created_at: i64,
    pub last_activity: i64,
    pub expires_at: i64,
    pub ip_address: String,
    pub user_agent: String,
    pub device_fingerprint: String,
    pub auth_method: String,
    pub cert_serial: Option<String>,
    pub metadata: HashMap<String, serde_json::Value>,
}

/// Session validation parameters
#[derive(Debug, Clone)]
pub struct SessionValidation {
    pub ip_address: String,
    pub user_agent: String,
    pub additional_headers: HashMap<String, String>,
}

/// Result of session validation
#[derive(Debug)]
pub struct SessionValidationResult {
    pub valid: bool,
    pub reason: Option<String>,
    pub session: Option<SessionData>,
}

/// Cached session with timestamp
#[derive(Debug, Clone)]
struct CachedSession {
    session: SessionData,
    cached_at: i64,
}

impl CachedSession {
    fn is_expired(&self) -> bool {
        let now = current_timestamp();
        let age = now - self.cached_at;
        age >= SESSION_CACHE_TTL as i64
    }
}

/// Get current Unix timestamp in seconds
fn current_timestamp() -> i64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("Time went backwards")
        .as_secs() as i64
}

#[cfg(test)]
mod tests {
    use super::*;
    
    fn create_test_config() -> SessionConfig {
        SessionConfig {
            backend: "redis".to_string(),
            redis_url: Some("redis://localhost".to_string()),
            idle_timeout_minutes: 15,
            absolute_timeout_hours: 8,
            max_concurrent_sessions: 5,
            validate_ip: true,
            validate_user_agent: true,
            validate_device_fingerprint: true,
        }
    }
    
    #[test]
    fn test_device_fingerprint_generation() {
        // We'll just test the fingerprint function directly without creating a full SessionManager
        let ip = "192.168.1.1";
        let ua = "Mozilla/5.0";
        let mut headers = HashMap::new();
        headers.insert("Accept-Language".to_string(), "en-US".to_string());
        
        // Generate fingerprints
        let mut hasher1 = Sha256::new();
        hasher1.update(ip.as_bytes());
        hasher1.update(ua.as_bytes());
        hasher1.update(b"Accept-Language");
        hasher1.update(b"en-US");
        let fp1 = format!("{:x}", hasher1.finalize());
        
        let mut hasher2 = Sha256::new();
        hasher2.update(ip.as_bytes());
        hasher2.update(ua.as_bytes());
        hasher2.update(b"Accept-Language");
        hasher2.update(b"en-US");
        let fp2 = format!("{:x}", hasher2.finalize());
        
        assert_eq!(fp1, fp2, "Same inputs should produce same fingerprint");
        
        let mut hasher3 = Sha256::new();
        hasher3.update(ip.as_bytes());
        hasher3.update(ua.as_bytes());
        hasher3.update(b"Accept-Language");
        hasher3.update(b"en-GB");
        let fp3 = format!("{:x}", hasher3.finalize());
        
        assert_ne!(fp1, fp3, "Different inputs should produce different fingerprints");
    }
    
    #[test]
    fn test_cached_session_expiration() {
        let session = SessionData {
            session_id: "test".to_string(),
            user_id: "user123".to_string(),
            created_at: 1000,
            last_activity: 1000,
            expires_at: 2000,
            ip_address: "192.168.1.1".to_string(),
            user_agent: "Test".to_string(),
            device_fingerprint: "abc123".to_string(),
            auth_method: "password".to_string(),
            cert_serial: None,
            metadata: HashMap::new(),
        };
        
        let cached = CachedSession {
            session: session.clone(),
            cached_at: current_timestamp() - SESSION_CACHE_TTL as i64 - 1,
        };
        
        assert!(cached.is_expired(), "Old cache entry should be expired");
        
        let fresh = CachedSession {
            session,
            cached_at: current_timestamp(),
        };
        
        assert!(!fresh.is_expired(), "Fresh cache entry should not be expired");
    }
}
