//! Rate limiting for sync operations
//! 
//! Implements token bucket algorithm to prevent abuse and flooding.
//! Each user gets a bucket of tokens that refills over time.

use std::collections::HashMap;
use std::sync::Arc;
use std::time::{Duration, Instant};
use tokio::sync::RwLock;
use uuid::Uuid;

use crate::error::{SyncError, SyncResult};

/// Rate limiter configuration
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct RateLimiterConfig {
    /// Maximum number of operations allowed in the time window
    pub max_operations: u32,
    /// Time window duration (in seconds, for serialization)
    #[serde(with = "duration_secs")]
    pub window_duration: Duration,
    /// Whether rate limiting is enabled
    pub enabled: bool,
}

// Serialize Duration as seconds
mod duration_secs {
    use serde::{Deserialize, Deserializer, Serializer};
    use std::time::Duration;

    pub fn serialize<S>(duration: &Duration, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        serializer.serialize_u64(duration.as_secs())
    }

    pub fn deserialize<'de, D>(deserializer: D) -> Result<Duration, D::Error>
    where
        D: Deserializer<'de>,
    {
        let secs = u64::deserialize(deserializer)?;
        Ok(Duration::from_secs(secs))
    }
}

impl Default for RateLimiterConfig {
    fn default() -> Self {
        Self {
            max_operations: 10,
            window_duration: Duration::from_secs(1),
            enabled: true,
        }
    }
}

/// Token bucket for a single user
#[derive(Debug, Clone)]
struct TokenBucket {
    /// Number of tokens available
    tokens: f64,
    /// Maximum number of tokens
    capacity: f64,
    /// Last time tokens were refilled
    last_refill: Instant,
    /// Refill rate (tokens per second)
    refill_rate: f64,
}

impl TokenBucket {
    fn new(capacity: u32, refill_rate: f64) -> Self {
        Self {
            tokens: capacity as f64,
            capacity: capacity as f64,
            last_refill: Instant::now(),
            refill_rate,
        }
    }

    /// Refill tokens based on elapsed time
    fn refill(&mut self) {
        let now = Instant::now();
        let elapsed = now.duration_since(self.last_refill).as_secs_f64();
        
        // Add tokens based on refill rate
        self.tokens = (self.tokens + elapsed * self.refill_rate).min(self.capacity);
        self.last_refill = now;
    }

    /// Try to consume a token
    fn try_consume(&mut self) -> bool {
        self.refill();
        
        if self.tokens >= 1.0 {
            self.tokens -= 1.0;
            true
        } else {
            false
        }
    }

    /// Get remaining tokens
    fn available_tokens(&mut self) -> u32 {
        self.refill();
        self.tokens.floor() as u32
    }
}

/// Rate limiter for sync operations
pub struct RateLimiter {
    /// Configuration
    config: RateLimiterConfig,
    /// Token buckets per user
    buckets: Arc<RwLock<HashMap<Uuid, TokenBucket>>>,
}

impl RateLimiter {
    /// Create a new rate limiter
    pub fn new(config: RateLimiterConfig) -> Self {
        Self {
            config,
            buckets: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    /// Check if an operation is allowed for the user
    /// 
    /// Returns Ok(()) if allowed, Err(SyncError::RateLimitExceeded) if denied
    pub async fn check_rate_limit(&self, user_id: Uuid) -> SyncResult<()> {
        if !self.config.enabled {
            return Ok(());
        }

        let mut buckets = self.buckets.write().await;
        
        // Get or create bucket for user
        let bucket = buckets.entry(user_id).or_insert_with(|| {
            let refill_rate = self.config.max_operations as f64 
                / self.config.window_duration.as_secs_f64();
            TokenBucket::new(self.config.max_operations, refill_rate)
        });

        if bucket.try_consume() {
            Ok(())
        } else {
            Err(SyncError::RateLimitExceeded {
                user_id,
                retry_after: Duration::from_secs(1),
            })
        }
    }

    /// Get remaining tokens for a user
    pub async fn get_remaining_tokens(&self, user_id: Uuid) -> u32 {
        if !self.config.enabled {
            return self.config.max_operations;
        }

        let mut buckets = self.buckets.write().await;
        
        if let Some(bucket) = buckets.get_mut(&user_id) {
            bucket.available_tokens()
        } else {
            self.config.max_operations
        }
    }

    /// Reset rate limit for a user (for testing or admin purposes)
    pub async fn reset_user(&self, user_id: Uuid) {
        let mut buckets = self.buckets.write().await;
        buckets.remove(&user_id);
    }

    /// Clear all rate limit data (for testing)
    pub async fn clear_all(&self) {
        let mut buckets = self.buckets.write().await;
        buckets.clear();
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tokio::time::sleep;

    #[tokio::test]
    async fn test_rate_limiter_allows_within_limit() {
        let config = RateLimiterConfig {
            max_operations: 5,
            window_duration: Duration::from_secs(1),
            enabled: true,
        };
        let limiter = RateLimiter::new(config);
        let user_id = Uuid::new_v4();

        // Should allow 5 operations
        for i in 0..5 {
            let result = limiter.check_rate_limit(user_id).await;
            assert!(result.is_ok(), "Operation {} should be allowed", i);
        }
    }

    #[tokio::test]
    async fn test_rate_limiter_blocks_over_limit() {
        let config = RateLimiterConfig {
            max_operations: 3,
            window_duration: Duration::from_secs(1),
            enabled: true,
        };
        let limiter = RateLimiter::new(config);
        let user_id = Uuid::new_v4();

        // Allow 3 operations
        for _ in 0..3 {
            limiter.check_rate_limit(user_id).await.unwrap();
        }

        // 4th operation should be blocked
        let result = limiter.check_rate_limit(user_id).await;
        assert!(result.is_err(), "Should be rate limited");
        
        match result.unwrap_err() {
            SyncError::RateLimitExceeded { user_id: uid, .. } => {
                assert_eq!(uid, user_id);
            }
            _ => panic!("Expected RateLimitExceeded error"),
        }
    }

    #[tokio::test]
    async fn test_rate_limiter_refills_tokens() {
        let config = RateLimiterConfig {
            max_operations: 2,
            window_duration: Duration::from_millis(500),
            enabled: true,
        };
        let limiter = RateLimiter::new(config);
        let user_id = Uuid::new_v4();

        // Use all tokens
        limiter.check_rate_limit(user_id).await.unwrap();
        limiter.check_rate_limit(user_id).await.unwrap();
        
        // Should be blocked now
        assert!(limiter.check_rate_limit(user_id).await.is_err());

        // Wait for refill (500ms window / 2 operations = 250ms per token)
        sleep(Duration::from_millis(300)).await;

        // Should allow 1 more operation after refill
        assert!(limiter.check_rate_limit(user_id).await.is_ok());
    }

    #[tokio::test]
    async fn test_rate_limiter_disabled() {
        let config = RateLimiterConfig {
            max_operations: 1,
            window_duration: Duration::from_secs(1),
            enabled: false,
        };
        let limiter = RateLimiter::new(config);
        let user_id = Uuid::new_v4();

        // Should allow unlimited operations when disabled
        for _ in 0..10 {
            assert!(limiter.check_rate_limit(user_id).await.is_ok());
        }
    }

    #[tokio::test]
    async fn test_rate_limiter_per_user() {
        let config = RateLimiterConfig {
            max_operations: 2,
            window_duration: Duration::from_secs(1),
            enabled: true,
        };
        let limiter = RateLimiter::new(config);
        let user1 = Uuid::new_v4();
        let user2 = Uuid::new_v4();

        // User 1 uses all tokens
        limiter.check_rate_limit(user1).await.unwrap();
        limiter.check_rate_limit(user1).await.unwrap();
        assert!(limiter.check_rate_limit(user1).await.is_err());

        // User 2 should still have tokens
        assert!(limiter.check_rate_limit(user2).await.is_ok());
        assert!(limiter.check_rate_limit(user2).await.is_ok());
    }

    #[tokio::test]
    async fn test_get_remaining_tokens() {
        let config = RateLimiterConfig {
            max_operations: 5,
            window_duration: Duration::from_secs(1),
            enabled: true,
        };
        let limiter = RateLimiter::new(config);
        let user_id = Uuid::new_v4();

        // Initially should have all tokens
        assert_eq!(limiter.get_remaining_tokens(user_id).await, 5);

        // Use 2 tokens
        limiter.check_rate_limit(user_id).await.unwrap();
        limiter.check_rate_limit(user_id).await.unwrap();

        // Should have 3 remaining
        assert_eq!(limiter.get_remaining_tokens(user_id).await, 3);
    }

    #[tokio::test]
    async fn test_reset_user() {
        let config = RateLimiterConfig {
            max_operations: 2,
            window_duration: Duration::from_secs(1),
            enabled: true,
        };
        let limiter = RateLimiter::new(config);
        let user_id = Uuid::new_v4();

        // Use all tokens
        limiter.check_rate_limit(user_id).await.unwrap();
        limiter.check_rate_limit(user_id).await.unwrap();
        assert!(limiter.check_rate_limit(user_id).await.is_err());

        // Reset user
        limiter.reset_user(user_id).await;

        // Should have tokens again
        assert!(limiter.check_rate_limit(user_id).await.is_ok());
    }
}
