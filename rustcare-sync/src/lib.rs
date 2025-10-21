//! Offline-first synchronization engine for RustCare
//!
//! Provides:
//! - Local SQLite database for offline operations
//! - Sync queue with automatic retry
//! - Vector clocks for causality tracking
//! - CRDTs for conflict-free replication
//! - P2P sync for local collaboration

pub mod error;
pub mod local_db;
pub mod hlc;
pub mod causality;

pub use error::{SyncError, SyncResult};
pub use local_db::{LocalDatabase, LocalDbConfig, OperationType, SyncQueueEntry};
pub use hlc::{HybridLogicalClock, HybridTimestamp};
pub use causality::{VectorClock, Conflict, ConflictDetector};

/// Sync engine for offline-first operations
pub struct SyncEngine {
    local_db: LocalDatabase,
}

impl SyncEngine {
    /// Create a new sync engine
    pub async fn new(config: LocalDbConfig) -> SyncResult<Self> {
        let local_db = LocalDatabase::new(config).await?;
        
        Ok(Self {
            local_db,
        })
    }
    
    /// Get the local database
    pub fn local_db(&self) -> &LocalDatabase {
        &self.local_db
    }
    
    /// Get node ID
    pub fn node_id(&self) -> uuid::Uuid {
        self.local_db.node_id()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::NamedTempFile;
    use uuid::Uuid;
    
    #[tokio::test]
    async fn test_sync_engine_creation() {
        let temp_file = NamedTempFile::new().unwrap();
        let db_path = temp_file.path().to_str().unwrap().to_string();
        
        let config = LocalDbConfig {
            db_path,
            node_id: Uuid::new_v4(),
            max_connections: 5,
            enable_wal: true,
        };
        
        let engine = SyncEngine::new(config).await.unwrap();
        assert!(!engine.node_id().is_nil());
    }
}
