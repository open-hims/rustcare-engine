//! Secret caching implementation

use crate::{Result, Secret};
use moka::future::Cache;
use std::time::Duration;

pub struct SecretCache {
    cache: Cache<String, Secret>,
    ttl: Duration,
}

impl SecretCache {
    pub fn new(ttl_seconds: u64, max_entries: usize) -> Self {
        let cache = Cache::builder()
            .max_capacity(max_entries as u64)
            .time_to_live(Duration::from_secs(ttl_seconds))
            .build();
        
        Self {
            cache,
            ttl: Duration::from_secs(ttl_seconds),
        }
    }
    
    pub async fn get(&self, key: &str) -> Option<Secret> {
        self.cache.get(key).await
    }
    
    pub async fn set(&self, key: String, secret: Secret) -> Result<()> {
        self.cache.insert(key, secret).await;
        Ok(())
    }
    
    pub async fn invalidate(&self, key: &str) -> Result<()> {
        self.cache.invalidate(key).await;
        Ok(())
    }
    
    pub async fn clear(&self) -> Result<()> {
        self.cache.invalidate_all();
        Ok(())
    }
    
    pub fn ttl(&self) -> Duration {
        self.ttl
    }
}
