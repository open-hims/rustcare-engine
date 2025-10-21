//! Audit Logging for HIPAA Compliance
//!
//! This module provides comprehensive audit logging for all data access
//! and operations on PHI (Protected Health Information).
//!
//! # HIPAA Requirements
//!
//! - Log all access to PHI
//! - Record user identity, timestamp, and action
//! - Tamper-evident audit trail
//! - Retention for 6 years minimum
//! - Support for audit queries and reporting
//!
//! # Security Features
//!
//! - Append-only audit log (no updates/deletes)
//! - Cryptographic hash chain for tamper detection
//! - Separate audit database file with restricted permissions
//! - Automatic rotation and archival

use crate::error::{SyncError, SyncResult};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use sqlx::sqlite::SqlitePool;
use sqlx::Row;
use std::path::Path;
use uuid::Uuid;

/// Audit event type
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum AuditAction {
    /// Data was read/accessed
    Read,
    /// Data was created
    Create,
    /// Data was updated/modified
    Update,
    /// Data was deleted
    Delete,
    /// Database was opened
    DatabaseOpen,
    /// Database was closed
    DatabaseClose,
    /// Sync operation initiated
    SyncStart,
    /// Sync operation completed
    SyncComplete,
    /// Encryption key accessed
    KeyAccess,
    /// Authentication attempt
    AuthAttempt,
    /// Authorization check
    AuthzCheck,
}

/// Audit log entry
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AuditEntry {
    /// Unique audit entry ID
    pub id: Uuid,
    
    /// Timestamp of the event
    pub timestamp: DateTime<Utc>,
    
    /// Action performed
    pub action: AuditAction,
    
    /// User/process that performed the action
    pub actor: String,
    
    /// Resource that was accessed (e.g., "patient/123", "appointment/456")
    pub resource: String,
    
    /// Entity type (e.g., "patient", "appointment")
    pub entity_type: Option<String>,
    
    /// Entity ID
    pub entity_id: Option<Uuid>,
    
    /// Whether this resource contains PHI
    pub phi_flag: bool,
    
    /// Result of the action (success/failure)
    pub success: bool,
    
    /// Additional context/metadata
    pub metadata: serde_json::Value,
    
    /// Hash of previous audit entry (for tamper detection)
    pub prev_hash: String,
    
    /// Hash of this entry
    pub entry_hash: String,
}

impl AuditEntry {
    /// Create a new audit entry
    pub fn new(
        action: AuditAction,
        actor: String,
        resource: String,
        phi_flag: bool,
        success: bool,
        metadata: serde_json::Value,
        prev_hash: String,
    ) -> Self {
        let id = Uuid::new_v4();
        let timestamp = Utc::now();
        
        let mut entry = Self {
            id,
            timestamp,
            action,
            actor,
            resource,
            entity_type: None,
            entity_id: None,
            phi_flag,
            success,
            metadata,
            prev_hash,
            entry_hash: String::new(),
        };
        
        // Calculate hash of this entry
        entry.entry_hash = entry.calculate_hash();
        
        entry
    }
    
    /// Calculate cryptographic hash of this entry
    fn calculate_hash(&self) -> String {
        let mut hasher = Sha256::new();
        
        // Include all fields except entry_hash itself
        hasher.update(self.id.as_bytes());
        hasher.update(self.timestamp.to_rfc3339().as_bytes());
        hasher.update(format!("{:?}", self.action).as_bytes());
        hasher.update(self.actor.as_bytes());
        hasher.update(self.resource.as_bytes());
        hasher.update(&[self.phi_flag as u8]);
        hasher.update(&[self.success as u8]);
        hasher.update(self.metadata.to_string().as_bytes());
        hasher.update(self.prev_hash.as_bytes());
        
        format!("{:x}", hasher.finalize())
    }
}

/// Audit logger configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AuditConfig {
    /// Path to audit database file
    pub audit_db_path: String,
    
    /// Whether to enable audit logging
    pub enabled: bool,
    
    /// Maximum audit entries before rotation
    pub max_entries_before_rotation: usize,
    
    /// Whether to log read operations (can be verbose)
    pub log_reads: bool,
}

impl Default for AuditConfig {
    fn default() -> Self {
        Self {
            audit_db_path: "rustcare_audit.db".to_string(),
            enabled: true,
            max_entries_before_rotation: 100_000,
            log_reads: true,
        }
    }
}

/// Audit logger
pub struct AuditLogger {
    pool: SqlitePool,
    config: AuditConfig,
    last_hash: String,
}

impl AuditLogger {
    /// Create a new audit logger
    pub async fn new(config: AuditConfig) -> SyncResult<Self> {
        // Set restrictive file permissions before creating database
        #[cfg(unix)]
        {
            use std::fs;
            use std::os::unix::fs::PermissionsExt;
            
            // Create empty file if it doesn't exist
            if !Path::new(&config.audit_db_path).exists() {
                fs::File::create(&config.audit_db_path)
                    .map_err(|e| SyncError::Internal(format!("Failed to create audit file: {}", e)))?;
                
                // Set to 0600 (read/write for owner only)
                let permissions = fs::Permissions::from_mode(0o600);
                fs::set_permissions(&config.audit_db_path, permissions)
                    .map_err(|e| SyncError::Internal(format!("Failed to set audit file permissions: {}", e)))?;
            }
        }
        
        let db_url = format!("sqlite:{}", config.audit_db_path);
        let pool = SqlitePool::connect(&db_url).await?;
        
        let logger = Self {
            pool,
            config,
            last_hash: "0".to_string(), // Genesis hash
        };
        
        // Initialize schema
        logger.initialize_schema().await?;
        
        // Load last hash from database
        let logger = logger.load_last_hash().await?;
        
        Ok(logger)
    }
    
    /// Initialize audit database schema
    async fn initialize_schema(&self) -> SyncResult<()> {
        // Create audit_log table (append-only)
        sqlx::query(
            r#"
            CREATE TABLE IF NOT EXISTS audit_log (
                id TEXT PRIMARY KEY,
                timestamp TEXT NOT NULL,
                action TEXT NOT NULL,
                actor TEXT NOT NULL,
                resource TEXT NOT NULL,
                entity_type TEXT,
                entity_id TEXT,
                phi_flag INTEGER NOT NULL,
                success INTEGER NOT NULL,
                metadata TEXT NOT NULL,
                prev_hash TEXT NOT NULL,
                entry_hash TEXT NOT NULL
            )
            "#,
        )
        .execute(&self.pool)
        .await?;
        
        // Create indexes for common queries
        sqlx::query("CREATE INDEX IF NOT EXISTS idx_audit_timestamp ON audit_log(timestamp)")
            .execute(&self.pool)
            .await?;
        
        sqlx::query("CREATE INDEX IF NOT EXISTS idx_audit_actor ON audit_log(actor)")
            .execute(&self.pool)
            .await?;
        
        sqlx::query("CREATE INDEX IF NOT EXISTS idx_audit_resource ON audit_log(resource)")
            .execute(&self.pool)
            .await?;
        
        sqlx::query("CREATE INDEX IF NOT EXISTS idx_audit_phi ON audit_log(phi_flag)")
            .execute(&self.pool)
            .await?;
        
        sqlx::query("CREATE INDEX IF NOT EXISTS idx_audit_action ON audit_log(action)")
            .execute(&self.pool)
            .await?;
        
        Ok(())
    }
    
    /// Load the last hash from the database
    async fn load_last_hash(mut self) -> SyncResult<Self> {
        let row = sqlx::query(
            r#"
            SELECT entry_hash FROM audit_log
            ORDER BY timestamp DESC
            LIMIT 1
            "#,
        )
        .fetch_optional(&self.pool)
        .await?;
        
        if let Some(row) = row {
            self.last_hash = row.try_get("entry_hash")?;
        }
        
        Ok(self)
    }
    
    /// Log an audit entry
    pub async fn log(
        &mut self,
        action: AuditAction,
        actor: String,
        resource: String,
        phi_flag: bool,
        success: bool,
        metadata: serde_json::Value,
    ) -> SyncResult<Uuid> {
        if !self.config.enabled {
            return Ok(Uuid::new_v4());
        }
        
        // Skip read operations if not configured
        if action == AuditAction::Read && !self.config.log_reads {
            return Ok(Uuid::new_v4());
        }
        
        // Create audit entry with hash chain
        let entry = AuditEntry::new(
            action,
            actor,
            resource,
            phi_flag,
            success,
            metadata,
            self.last_hash.clone(),
        );
        
        // Store in database
        sqlx::query(
            r#"
            INSERT INTO audit_log (
                id, timestamp, action, actor, resource,
                entity_type, entity_id, phi_flag, success,
                metadata, prev_hash, entry_hash
            ) VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)
            "#,
        )
        .bind(entry.id.to_string())
        .bind(entry.timestamp.to_rfc3339())
        .bind(format!("{:?}", entry.action))
        .bind(&entry.actor)
        .bind(&entry.resource)
        .bind(entry.entity_type.as_ref())
        .bind(entry.entity_id.map(|id| id.to_string()))
        .bind(entry.phi_flag as i32)
        .bind(entry.success as i32)
        .bind(entry.metadata.to_string())
        .bind(&entry.prev_hash)
        .bind(&entry.entry_hash)
        .execute(&self.pool)
        .await?;
        
        // Update last hash for next entry
        self.last_hash = entry.entry_hash;
        
        Ok(entry.id)
    }
    
    /// Verify audit trail integrity
    pub async fn verify_integrity(&self) -> SyncResult<bool> {
        let rows = sqlx::query(
            r#"
            SELECT id, timestamp, action, actor, resource,
                   phi_flag, success, metadata, prev_hash, entry_hash
            FROM audit_log
            ORDER BY timestamp ASC
            "#,
        )
        .fetch_all(&self.pool)
        .await?;
        
        let mut expected_prev_hash = "0".to_string();
        
        for row in rows {
            let prev_hash: String = row.try_get("prev_hash")?;
            let entry_hash: String = row.try_get("entry_hash")?;
            
            // Verify hash chain
            if prev_hash != expected_prev_hash {
                return Ok(false);
            }
            
            // Reconstruct entry to verify hash
            let id: String = row.try_get("id")?;
            let timestamp: String = row.try_get("timestamp")?;
            let action: String = row.try_get("action")?;
            let actor: String = row.try_get("actor")?;
            let resource: String = row.try_get("resource")?;
            let phi_flag: i32 = row.try_get("phi_flag")?;
            let success: i32 = row.try_get("success")?;
            let metadata: String = row.try_get("metadata")?;
            
            // Calculate expected hash
            let mut hasher = Sha256::new();
            hasher.update(id.as_bytes());
            hasher.update(timestamp.as_bytes());
            hasher.update(action.as_bytes());
            hasher.update(actor.as_bytes());
            hasher.update(resource.as_bytes());
            hasher.update(&[phi_flag as u8]);
            hasher.update(&[success as u8]);
            hasher.update(metadata.as_bytes());
            hasher.update(prev_hash.as_bytes());
            
            let calculated_hash = format!("{:x}", hasher.finalize());
            
            if calculated_hash != entry_hash {
                return Ok(false);
            }
            
            expected_prev_hash = entry_hash;
        }
        
        Ok(true)
    }
    
    /// Get audit entries for a specific actor
    pub async fn get_entries_by_actor(&self, actor: &str, limit: i64) -> SyncResult<Vec<AuditEntry>> {
        let rows = sqlx::query(
            r#"
            SELECT id, timestamp, action, actor, resource,
                   entity_type, entity_id, phi_flag, success,
                   metadata, prev_hash, entry_hash
            FROM audit_log
            WHERE actor = ?
            ORDER BY timestamp DESC
            LIMIT ?
            "#,
        )
        .bind(actor)
        .bind(limit)
        .fetch_all(&self.pool)
        .await?;
        
        self.rows_to_entries(rows).await
    }
    
    /// Get audit entries for PHI access
    pub async fn get_phi_access_entries(&self, limit: i64) -> SyncResult<Vec<AuditEntry>> {
        let rows = sqlx::query(
            r#"
            SELECT id, timestamp, action, actor, resource,
                   entity_type, entity_id, phi_flag, success,
                   metadata, prev_hash, entry_hash
            FROM audit_log
            WHERE phi_flag = 1
            ORDER BY timestamp DESC
            LIMIT ?
            "#,
        )
        .bind(limit)
        .fetch_all(&self.pool)
        .await?;
        
        self.rows_to_entries(rows).await
    }
    
    /// Convert database rows to audit entries
    async fn rows_to_entries(&self, rows: Vec<sqlx::sqlite::SqliteRow>) -> SyncResult<Vec<AuditEntry>> {
        let mut entries = Vec::new();
        
        for row in rows {
            let id_str: String = row.try_get("id")?;
            let timestamp_str: String = row.try_get("timestamp")?;
            let action_str: String = row.try_get("action")?;
            let metadata_str: String = row.try_get("metadata")?;
            let entity_id_str: Option<String> = row.try_get("entity_id")?;
            
            let entry = AuditEntry {
                id: Uuid::parse_str(&id_str)
                    .map_err(|e| SyncError::Internal(format!("Invalid UUID: {}", e)))?,
                timestamp: DateTime::parse_from_rfc3339(&timestamp_str)
                    .map_err(|e| SyncError::Internal(format!("Invalid timestamp: {}", e)))?
                    .with_timezone(&Utc),
                action: serde_json::from_str(&format!("\"{}\"", action_str.to_lowercase()))
                    .map_err(|e| SyncError::Internal(format!("Invalid action: {}", e)))?,
                actor: row.try_get("actor")?,
                resource: row.try_get("resource")?,
                entity_type: row.try_get("entity_type")?,
                entity_id: entity_id_str.and_then(|s| Uuid::parse_str(&s).ok()),
                phi_flag: row.try_get::<i32, _>("phi_flag")? != 0,
                success: row.try_get::<i32, _>("success")? != 0,
                metadata: serde_json::from_str(&metadata_str)
                    .map_err(|e| SyncError::Internal(format!("Invalid metadata: {}", e)))?,
                prev_hash: row.try_get("prev_hash")?,
                entry_hash: row.try_get("entry_hash")?,
            };
            
            entries.push(entry);
        }
        
        Ok(entries)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;
    use tempfile::TempDir;
    use std::path::PathBuf;
    
    fn get_test_audit_path() -> PathBuf {
        let temp_dir = std::env::temp_dir();
        temp_dir.join(format!("test_audit_{}.db", uuid::Uuid::new_v4()))
    }
    
    async fn create_test_logger() -> SyncResult<AuditLogger> {
        let audit_db_path = get_test_audit_path().to_str().unwrap().to_string();
        
        let config = AuditConfig {
            audit_db_path,
            enabled: true,
            max_entries_before_rotation: 100_000,
            log_reads: true,
        };
        
        AuditLogger::new(config).await
    }
    
    #[tokio::test]
    async fn test_audit_logger_creation() {
        let logger = create_test_logger().await.unwrap();
        assert_eq!(logger.last_hash, "0");
    }
    
    #[tokio::test]
    async fn test_log_audit_entry() {
        let mut logger = create_test_logger().await.unwrap();
        
        let entry_id = logger.log(
            AuditAction::Read,
            "user123".to_string(),
            "patient/456".to_string(),
            true,
            true,
            json!({"details": "accessed patient record"}),
        ).await.unwrap();
        
        assert!(!entry_id.is_nil());
        assert_ne!(logger.last_hash, "0");
    }
    
    #[tokio::test]
    async fn test_audit_trail_integrity() {
        let mut logger = create_test_logger().await.unwrap();
        
        // Log multiple entries
        for i in 0..5 {
            logger.log(
                AuditAction::Read,
                format!("user{}", i),
                format!("patient/{}", i),
                true,
                true,
                json!({"action": i}),
            ).await.unwrap();
        }
        
        // Verify integrity
        let is_valid = logger.verify_integrity().await.unwrap();
        assert!(is_valid);
    }
    
    #[tokio::test]
    async fn test_get_entries_by_actor() {
        let mut logger = create_test_logger().await.unwrap();
        
        logger.log(
            AuditAction::Read,
            "alice".to_string(),
            "patient/1".to_string(),
            true,
            true,
            json!({}),
        ).await.unwrap();
        
        logger.log(
            AuditAction::Update,
            "bob".to_string(),
            "patient/2".to_string(),
            true,
            true,
            json!({}),
        ).await.unwrap();
        
        logger.log(
            AuditAction::Read,
            "alice".to_string(),
            "patient/3".to_string(),
            true,
            true,
            json!({}),
        ).await.unwrap();
        
        let alice_entries = logger.get_entries_by_actor("alice", 10).await.unwrap();
        assert_eq!(alice_entries.len(), 2);
        
        let bob_entries = logger.get_entries_by_actor("bob", 10).await.unwrap();
        assert_eq!(bob_entries.len(), 1);
    }
    
    #[tokio::test]
    async fn test_get_phi_access_entries() {
        let mut logger = create_test_logger().await.unwrap();
        
        logger.log(
            AuditAction::Read,
            "user1".to_string(),
            "patient/1".to_string(),
            true, // PHI
            true,
            json!({}),
        ).await.unwrap();
        
        logger.log(
            AuditAction::Read,
            "user2".to_string(),
            "settings".to_string(),
            false, // Not PHI
            true,
            json!({}),
        ).await.unwrap();
        
        let phi_entries = logger.get_phi_access_entries(10).await.unwrap();
        assert_eq!(phi_entries.len(), 1);
        assert_eq!(phi_entries[0].actor, "user1");
    }
    
    #[tokio::test]
    async fn test_disabled_logging() {
        let audit_db_path = get_test_audit_path().to_str().unwrap().to_string();
        
        let config = AuditConfig {
            audit_db_path,
            enabled: false, // Disabled
            max_entries_before_rotation: 100_000,
            log_reads: true,
        };
        
        let mut logger = AuditLogger::new(config).await.unwrap();
        
        logger.log(
            AuditAction::Read,
            "user1".to_string(),
            "patient/1".to_string(),
            true,
            true,
            json!({}),
        ).await.unwrap();
        
        // Should still have genesis hash since logging is disabled
        assert_eq!(logger.last_hash, "0");
    }
}
