//! Health check implementation

use crate::HealthStatus;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ServiceHealth {
    pub provider: HealthStatus,
    pub cache: bool,
    pub overall_healthy: bool,
}

impl ServiceHealth {
    pub fn new(provider: HealthStatus, cache_enabled: bool) -> Self {
        let overall_healthy = provider.healthy;
        
        Self {
            provider,
            cache: cache_enabled,
            overall_healthy,
        }
    }
}
