//! Secret rotation management

use crate::{SecretProvider, Result, SecretsError};
use chrono::{DateTime, Duration, Utc};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use tracing::{debug, info};

/// Rotation policy for a secret
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RotationPolicy {
    /// Secret key
    pub key: String,
    
    /// Rotation interval in days
    pub interval_days: u32,
    
    /// Last rotation timestamp
    pub last_rotated: DateTime<Utc>,
    
    /// Whether rotation is enabled
    pub enabled: bool,
    
    /// Custom rotation handler (optional)
    pub custom_handler: Option<String>,
}

impl RotationPolicy {
    /// Check if secret needs rotation
    pub fn needs_rotation(&self) -> bool {
        if !self.enabled {
            return false;
        }
        
        let next_rotation = self.last_rotated + Duration::days(self.interval_days as i64);
        Utc::now() >= next_rotation
    }
    
    /// Get days until next rotation
    pub fn days_until_rotation(&self) -> i64 {
        if !self.enabled {
            return -1;
        }
        
        let next_rotation = self.last_rotated + Duration::days(self.interval_days as i64);
        (next_rotation - Utc::now()).num_days()
    }
}

/// Rotation manager
pub struct RotationManager {
    policies: HashMap<String, RotationPolicy>,
}

impl RotationManager {
    pub fn new() -> Self {
        Self {
            policies: HashMap::new(),
        }
    }
    
    /// Add or update a rotation policy
    pub fn add_policy(&mut self, policy: RotationPolicy) {
        self.policies.insert(policy.key.clone(), policy);
    }
    
    /// Remove a rotation policy
    pub fn remove_policy(&mut self, key: &str) {
        self.policies.remove(key);
    }
    
    /// Get a rotation policy
    pub fn get_policy(&self, key: &str) -> Option<&RotationPolicy> {
        self.policies.get(key)
    }
    
    /// Check which secrets need rotation
    pub fn get_secrets_needing_rotation(&self) -> Vec<String> {
        self.policies
            .values()
            .filter(|p| p.needs_rotation())
            .map(|p| p.key.clone())
            .collect()
    }
    
    /// Rotate a secret using the provider
    pub async fn rotate_secret<P: SecretProvider>(
        &mut self,
        key: &str,
        provider: &P,
    ) -> Result<String> {
        debug!("Rotating secret: {}", key);
        
        // Check if policy exists
        let policy = self.policies.get_mut(key)
            .ok_or_else(|| SecretsError::NotFound(format!("No rotation policy for key: {}", key)))?;
        
        if !policy.enabled {
            return Err(SecretsError::ConfigurationError(format!("Rotation disabled for key: {}", key)));
        }
        
        // Perform rotation
        let version = provider.rotate_secret(key).await?;
        
        // Update last rotation time
        policy.last_rotated = Utc::now();
        
        info!("Secret rotated successfully: {} (version: {})", key, version);
        
        Ok(version)
    }
    
    /// Rotate all secrets that need rotation
    pub async fn rotate_all<P: SecretProvider>(
        &mut self,
        provider: &P,
    ) -> HashMap<String, Result<String>> {
        let keys: Vec<String> = self.get_secrets_needing_rotation();
        let mut results = HashMap::new();
        
        for key in keys {
            let result = self.rotate_secret(&key, provider).await;
            results.insert(key, result);
        }
        
        results
    }
}

impl Default for RotationManager {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_rotation_policy() {
        let mut policy = RotationPolicy {
            key: "test/secret".to_string(),
            interval_days: 30,
            last_rotated: Utc::now() - Duration::days(31),
            enabled: true,
            custom_handler: None,
        };
        
        // Should need rotation (last rotated 31 days ago)
        assert!(policy.needs_rotation());
        
        // Update last rotation
        policy.last_rotated = Utc::now();
        assert!(!policy.needs_rotation());
        
        // Check days until rotation
        assert!(policy.days_until_rotation() > 25);
    }
    
    #[test]
    fn test_rotation_manager() {
        let mut manager = RotationManager::new();
        
        let policy = RotationPolicy {
            key: "test/secret".to_string(),
            interval_days: 30,
            last_rotated: Utc::now() - Duration::days(31),
            enabled: true,
            custom_handler: None,
        };
        
        manager.add_policy(policy);
        
        let needs_rotation = manager.get_secrets_needing_rotation();
        assert_eq!(needs_rotation.len(), 1);
        assert_eq!(needs_rotation[0], "test/secret");
    }
}
