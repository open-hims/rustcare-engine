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
pub mod crdt;
pub mod sync_protocol;
pub mod p2p;
pub mod encryption;
pub mod field_encryption;
pub mod audit;
pub mod rate_limiter;
pub mod key_manager;
pub mod secure_memory;

pub use error::{SyncError, SyncResult};
pub use local_db::{LocalDatabase, LocalDbConfig, OperationType, SyncQueueEntry};
pub use hlc::{HybridLogicalClock, HybridTimestamp};
pub use causality::{VectorClock, Conflict, ConflictDetector};
pub use crdt::{Crdt, LwwRegister, GCounter, PnCounter, OrSet, Rga};
pub use sync_protocol::{SyncProtocol, SyncConfig, SyncStats};
pub use p2p::{P2PSync, P2PConfig, PeerInfo, PeerStatus};
pub use encryption::{EncryptionConfig, EncryptionKeyManager, DatabaseKey, EncryptionMetadata};
pub use field_encryption::{FieldEncryption, FieldEncryptionConfig};
pub use audit::{AuditLogger, AuditConfig, AuditAction, AuditEntry};
pub use rate_limiter::{RateLimiter, RateLimiterConfig};
pub use key_manager::{LocalDbKeyManager, KeyManagerConfig, LocalDbKeyMetadata};
pub use secure_memory::{
    SecureString, SecureVec, SecureData, SecurePatientData, SecureMedicalRecord,
    IntoSecure, IntoSecureVec,
};

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
            enable_secure_delete: true,
            audit_config: None,
            user_id: None,
            user_email: None,
            rate_limiter_config: None,
            kms_config: None,
        };
        
        let engine = SyncEngine::new(config).await.unwrap();
        assert!(!engine.node_id().is_nil());
    }
}
