//! Local SQLite database for offline-first operations
//!
//! Provides:
//! - Local persistence of data
//! - Sync queue for pending operations
//! - Vector clock tracking
//! - Conflict detection and resolution

use crate::error::{SyncError, SyncResult};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sqlx::{sqlite::SqlitePool, Row};
use uuid::Uuid;

/// Local database configuration
#[derive(Debug, Clone)]
pub struct LocalDbConfig {
    /// Path to SQLite database file
    pub db_path: String,
    
    /// Node ID (unique identifier for this device/clinic)
    pub node_id: Uuid,
    
    /// Maximum number of connections
    pub max_connections: u32,
    
    /// Enable WAL mode for better concurrency
    pub enable_wal: bool,
}

impl Default for LocalDbConfig {
    fn default() -> Self {
        Self {
            db_path: "rustcare_local.db".to_string(),
            node_id: Uuid::new_v4(),
            max_connections: 5,
            enable_wal: true,
        }
    }
}

/// Operation type in sync queue
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum OperationType {
    Create,
    Update,
    Delete,
}

impl OperationType {
    pub fn as_str(&self) -> &'static str {
        match self {
            OperationType::Create => "create",
            OperationType::Update => "update",
            OperationType::Delete => "delete",
        }
    }
    
    pub fn from_str(s: &str) -> SyncResult<Self> {
        match s {
            "create" => Ok(OperationType::Create),
            "update" => Ok(OperationType::Update),
            "delete" => Ok(OperationType::Delete),
            _ => Err(SyncError::InvalidOperation(format!("Unknown operation type: {}", s))),
        }
    }
}

/// Sync queue entry
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SyncQueueEntry {
    /// Unique operation ID
    pub id: Uuid,
    
    /// Entity type (e.g., "patient", "appointment", "record")
    pub entity_type: String,
    
    /// Entity ID
    pub entity_id: Uuid,
    
    /// Operation type
    pub operation: OperationType,
    
    /// Serialized data payload
    pub data: serde_json::Value,
    
    /// Vector clock at time of operation
    pub vector_clock: String,
    
    /// Timestamp when operation was queued
    pub created_at: DateTime<Utc>,
    
    /// Number of retry attempts
    pub retry_count: i32,
    
    /// Last error message (if any)
    pub last_error: Option<String>,
    
    /// Whether this operation has been synced
    pub synced: bool,
}

/// Local database handle
pub struct LocalDatabase {
    pool: SqlitePool,
    node_id: Uuid,
}

impl LocalDatabase {
    /// Create a new local database
    pub async fn new(config: LocalDbConfig) -> SyncResult<Self> {
        // Create database file if it doesn't exist
        let db_url = format!("sqlite:{}", config.db_path);
        
        // Create connection pool
        let pool = SqlitePool::connect(&db_url).await?;
        
        // Enable WAL mode for better concurrency
        if config.enable_wal {
            sqlx::query("PRAGMA journal_mode = WAL")
                .execute(&pool)
                .await?;
        }
        
        // Enable foreign keys
        sqlx::query("PRAGMA foreign_keys = ON")
            .execute(&pool)
            .await?;
        
        let db = Self {
            pool,
            node_id: config.node_id,
        };
        
        // Initialize schema
        db.initialize_schema().await?;
        
        Ok(db)
    }
    
    /// Initialize database schema
    async fn initialize_schema(&self) -> SyncResult<()> {
        // Create sync queue table
        sqlx::query(
            r#"
            CREATE TABLE IF NOT EXISTS sync_queue (
                id TEXT PRIMARY KEY,
                entity_type TEXT NOT NULL,
                entity_id TEXT NOT NULL,
                operation TEXT NOT NULL,
                data TEXT NOT NULL,
                vector_clock TEXT NOT NULL,
                created_at TEXT NOT NULL,
                retry_count INTEGER NOT NULL DEFAULT 0,
                last_error TEXT,
                synced INTEGER NOT NULL DEFAULT 0
            )
            "#,
        )
        .execute(&self.pool)
        .await?;
        
        // Create indexes for sync_queue
        sqlx::query("CREATE INDEX IF NOT EXISTS idx_sync_queue_synced ON sync_queue(synced)")
            .execute(&self.pool)
            .await?;
        sqlx::query("CREATE INDEX IF NOT EXISTS idx_sync_queue_entity ON sync_queue(entity_type, entity_id)")
            .execute(&self.pool)
            .await?;
        sqlx::query("CREATE INDEX IF NOT EXISTS idx_sync_queue_created ON sync_queue(created_at)")
            .execute(&self.pool)
            .await?;
        
        // Create vector clock table
        sqlx::query(
            r#"
            CREATE TABLE IF NOT EXISTS vector_clock (
                node_id TEXT PRIMARY KEY,
                counter INTEGER NOT NULL DEFAULT 0,
                last_updated TEXT NOT NULL
            )
            "#,
        )
        .execute(&self.pool)
        .await?;
        
        // Create conflict log table
        sqlx::query(
            r#"
            CREATE TABLE IF NOT EXISTS conflict_log (
                id TEXT PRIMARY KEY,
                entity_type TEXT NOT NULL,
                entity_id TEXT NOT NULL,
                local_version TEXT NOT NULL,
                remote_version TEXT NOT NULL,
                resolved INTEGER NOT NULL DEFAULT 0,
                resolution_strategy TEXT,
                created_at TEXT NOT NULL,
                resolved_at TEXT
            )
            "#,
        )
        .execute(&self.pool)
        .await?;
        
        // Create indexes for conflict_log
        sqlx::query("CREATE INDEX IF NOT EXISTS idx_conflict_resolved ON conflict_log(resolved)")
            .execute(&self.pool)
            .await?;
        sqlx::query("CREATE INDEX IF NOT EXISTS idx_conflict_entity ON conflict_log(entity_type, entity_id)")
            .execute(&self.pool)
            .await?;
        
        // Create metadata table for storing sync state
        sqlx::query(
            r#"
            CREATE TABLE IF NOT EXISTS sync_metadata (
                key TEXT PRIMARY KEY,
                value TEXT NOT NULL,
                updated_at TEXT NOT NULL
            )
            "#,
        )
        .execute(&self.pool)
        .await?;
        
        // Initialize node in vector clock if not exists
        sqlx::query(
            r#"
            INSERT OR IGNORE INTO vector_clock (node_id, counter, last_updated)
            VALUES (?, 0, ?)
            "#,
        )
        .bind(self.node_id.to_string())
        .bind(Utc::now().to_rfc3339())
        .execute(&self.pool)
        .await?;
        
        Ok(())
    }
    
    /// Queue an operation for sync
    pub async fn queue_operation(
        &self,
        entity_type: &str,
        entity_id: Uuid,
        operation: OperationType,
        data: serde_json::Value,
        vector_clock: &str,
    ) -> SyncResult<Uuid> {
        let operation_id = Uuid::new_v4();
        let now = Utc::now();
        
        sqlx::query(
            r#"
            INSERT INTO sync_queue (
                id, entity_type, entity_id, operation, data,
                vector_clock, created_at, retry_count, synced
            ) VALUES (?, ?, ?, ?, ?, ?, ?, 0, 0)
            "#,
        )
        .bind(operation_id.to_string())
        .bind(entity_type)
        .bind(entity_id.to_string())
        .bind(operation.as_str())
        .bind(data.to_string())
        .bind(vector_clock)
        .bind(now.to_rfc3339())
        .execute(&self.pool)
        .await?;
        
        tracing::debug!(
            operation_id = %operation_id,
            entity_type = entity_type,
            entity_id = %entity_id,
            operation = ?operation,
            "Queued operation for sync"
        );
        
        Ok(operation_id)
    }
    
    /// Get pending operations to sync
    pub async fn get_pending_operations(&self, limit: i64) -> SyncResult<Vec<SyncQueueEntry>> {
        let rows = sqlx::query(
            r#"
            SELECT id, entity_type, entity_id, operation, data,
                   vector_clock, created_at, retry_count, last_error, synced
            FROM sync_queue
            WHERE synced = 0
            ORDER BY created_at ASC
            LIMIT ?
            "#,
        )
        .bind(limit)
        .fetch_all(&self.pool)
        .await?;
        
        let mut entries = Vec::new();
        for row in rows {
            let id: String = row.try_get("id")?;
            let entity_type: String = row.try_get("entity_type")?;
            let entity_id: String = row.try_get("entity_id")?;
            let operation: String = row.try_get("operation")?;
            let data: String = row.try_get("data")?;
            let vector_clock: String = row.try_get("vector_clock")?;
            let created_at: String = row.try_get("created_at")?;
            let retry_count: i32 = row.try_get("retry_count")?;
            let last_error: Option<String> = row.try_get("last_error")?;
            let synced: i32 = row.try_get("synced")?;
            
            entries.push(SyncQueueEntry {
                id: Uuid::parse_str(&id)
                    .map_err(|e| SyncError::Internal(format!("Invalid UUID: {}", e)))?,
                entity_type,
                entity_id: Uuid::parse_str(&entity_id)
                    .map_err(|e| SyncError::Internal(format!("Invalid UUID: {}", e)))?,
                operation: OperationType::from_str(&operation)?,
                data: serde_json::from_str(&data)?,
                vector_clock,
                created_at: DateTime::parse_from_rfc3339(&created_at)
                    .map_err(|e| SyncError::Internal(format!("Invalid timestamp: {}", e)))?
                    .with_timezone(&Utc),
                retry_count,
                last_error,
                synced: synced != 0,
            });
        }
        
        Ok(entries)
    }
    
    /// Mark operation as synced
    pub async fn mark_synced(&self, operation_id: Uuid) -> SyncResult<()> {
        sqlx::query(
            r#"
            UPDATE sync_queue
            SET synced = 1
            WHERE id = ?
            "#,
        )
        .bind(operation_id.to_string())
        .execute(&self.pool)
        .await?;
        
        tracing::debug!(operation_id = %operation_id, "Marked operation as synced");
        
        Ok(())
    }
    
    /// Mark operation as failed and increment retry count
    pub async fn mark_failed(&self, operation_id: Uuid, error: &str) -> SyncResult<()> {
        sqlx::query(
            r#"
            UPDATE sync_queue
            SET retry_count = retry_count + 1,
                last_error = ?
            WHERE id = ?
            "#,
        )
        .bind(error)
        .bind(operation_id.to_string())
        .execute(&self.pool)
        .await?;
        
        tracing::warn!(
            operation_id = %operation_id,
            error = error,
            "Operation sync failed"
        );
        
        Ok(())
    }
    
    /// Get node's current vector clock counter
    pub async fn get_vector_clock_counter(&self) -> SyncResult<i64> {
        let row = sqlx::query(
            r#"
            SELECT counter FROM vector_clock WHERE node_id = ?
            "#,
        )
        .bind(self.node_id.to_string())
        .fetch_one(&self.pool)
        .await?;
        
        Ok(row.try_get("counter")?)
    }
    
    /// Increment vector clock counter
    pub async fn increment_vector_clock(&self) -> SyncResult<i64> {
        sqlx::query(
            r#"
            UPDATE vector_clock
            SET counter = counter + 1,
                last_updated = ?
            WHERE node_id = ?
            "#,
        )
        .bind(Utc::now().to_rfc3339())
        .bind(self.node_id.to_string())
        .execute(&self.pool)
        .await?;
        
        self.get_vector_clock_counter().await
    }
    
    /// Get node ID
    pub fn node_id(&self) -> Uuid {
        self.node_id
    }
    
    /// Get database pool
    pub fn pool(&self) -> &SqlitePool {
        &self.pool
    }
    
    /// Close database connection
    pub async fn close(self) -> SyncResult<()> {
        self.pool.close().await;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::NamedTempFile;
    
    async fn create_test_db() -> SyncResult<LocalDatabase> {
        let temp_file = NamedTempFile::new().unwrap();
        let db_path = temp_file.path().to_str().unwrap().to_string();
        
        let config = LocalDbConfig {
            db_path,
            node_id: Uuid::new_v4(),
            max_connections: 5,
            enable_wal: true,
        };
        
        LocalDatabase::new(config).await
    }
    
    #[tokio::test]
    async fn test_database_creation() {
        let db = create_test_db().await.unwrap();
        assert_eq!(db.get_vector_clock_counter().await.unwrap(), 0);
    }
    
    #[tokio::test]
    async fn test_queue_operation() {
        let db = create_test_db().await.unwrap();
        
        let data = serde_json::json!({
            "name": "John Doe",
            "age": 30
        });
        
        let op_id = db.queue_operation(
            "patient",
            Uuid::new_v4(),
            OperationType::Create,
            data.clone(),
            "node1:1",
        ).await.unwrap();
        
        assert!(!op_id.is_nil());
        
        let pending = db.get_pending_operations(10).await.unwrap();
        assert_eq!(pending.len(), 1);
        assert_eq!(pending[0].entity_type, "patient");
        assert_eq!(pending[0].operation, OperationType::Create);
        assert_eq!(pending[0].data, data);
    }
    
    #[tokio::test]
    async fn test_mark_synced() {
        let db = create_test_db().await.unwrap();
        
        let op_id = db.queue_operation(
            "patient",
            Uuid::new_v4(),
            OperationType::Update,
            serde_json::json!({}),
            "node1:2",
        ).await.unwrap();
        
        db.mark_synced(op_id).await.unwrap();
        
        let pending = db.get_pending_operations(10).await.unwrap();
        assert_eq!(pending.len(), 0);
    }
    
    #[tokio::test]
    async fn test_vector_clock() {
        let db = create_test_db().await.unwrap();
        
        assert_eq!(db.get_vector_clock_counter().await.unwrap(), 0);
        
        let counter1 = db.increment_vector_clock().await.unwrap();
        assert_eq!(counter1, 1);
        
        let counter2 = db.increment_vector_clock().await.unwrap();
        assert_eq!(counter2, 2);
    }
    
    #[tokio::test]
    async fn test_mark_failed() {
        let db = create_test_db().await.unwrap();
        
        let op_id = db.queue_operation(
            "appointment",
            Uuid::new_v4(),
            OperationType::Delete,
            serde_json::json!({}),
            "node1:3",
        ).await.unwrap();
        
        db.mark_failed(op_id, "Network error").await.unwrap();
        
        let pending = db.get_pending_operations(10).await.unwrap();
        assert_eq!(pending.len(), 1);
        assert_eq!(pending[0].retry_count, 1);
        assert_eq!(pending[0].last_error, Some("Network error".to_string()));
    }
}
