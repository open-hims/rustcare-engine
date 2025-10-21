/// PostgreSQL Transparent Data Encryption (TDE) Integration
/// 
/// Provides Rust wrappers for PostgreSQL pg_crypto functions and encrypted tablespace management

use crate::error::{DatabaseError, DatabaseResult};
use sqlx::{PgPool, Row};
use zeroize::Zeroizing;

/// TDE Configuration for PostgreSQL
#[derive(Debug, Clone)]
pub struct TdeConfig {
    /// Whether TDE is enabled
    pub enabled: bool,
    /// Encrypted tablespace name (if using OS-level encryption)
    pub tablespace_name: Option<String>,
    /// Whether to enforce SSL/TLS connections
    pub require_ssl: bool,
    /// Minimum TLS version
    pub min_tls_version: String,
    /// Key rotation period in days
    pub key_rotation_days: u32,
}

impl Default for TdeConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            tablespace_name: Some("encrypted_data".to_string()),
            require_ssl: true,
            min_tls_version: "TLSv1.2".to_string(),
            key_rotation_days: 90,
        }
    }
}

/// PostgreSQL TDE Manager
pub struct PostgresTdeManager {
    pool: PgPool,
    config: TdeConfig,
}

impl PostgresTdeManager {
    /// Create a new TDE manager
    pub fn new(pool: PgPool, config: TdeConfig) -> Self {
        Self { pool, config }
    }

    /// Initialize TDE: ensure pg_crypto is installed and configured
    pub async fn initialize(&self) -> DatabaseResult<()> {
        // Check if pg_crypto extension is installed
        let result = sqlx::query("SELECT installed_version FROM pg_available_extensions WHERE name = 'pgcrypto'")
            .fetch_optional(&self.pool)
            .await
            .map_err(|e| DatabaseError::QueryError(format!("Failed to check pgcrypto: {}", e)))?;

        if result.is_none() {
            return Err(DatabaseError::ConfigurationError(
                "pg_crypto extension is not available".to_string(),
            ));
        }

        // Enable extension if not already enabled
        sqlx::query("CREATE EXTENSION IF NOT EXISTS pgcrypto")
            .execute(&self.pool)
            .await
            .map_err(|e| DatabaseError::QueryError(format!("Failed to enable pgcrypto: {}", e)))?;

        // Verify SSL configuration if required
        if self.config.require_ssl {
            self.verify_ssl_enabled().await?;
        }

        // Check if encrypted tablespace exists (if configured)
        if let Some(ref tablespace) = self.config.tablespace_name {
            self.verify_tablespace(tablespace).await?;
        }

        Ok(())
    }

    /// Verify that SSL/TLS is enabled for connections
    async fn verify_ssl_enabled(&self) -> DatabaseResult<()> {
        let row = sqlx::query("SHOW ssl")
            .fetch_one(&self.pool)
            .await
            .map_err(|e| DatabaseError::QueryError(format!("Failed to check SSL status: {}", e)))?;

        let ssl_enabled: String = row.try_get(0)
            .map_err(|e| DatabaseError::QueryError(format!("Failed to parse SSL status: {}", e)))?;

        if ssl_enabled.to_lowercase() != "on" {
            return Err(DatabaseError::ConfigurationError(
                "SSL is not enabled in PostgreSQL configuration".to_string(),
            ));
        }

        Ok(())
    }

    /// Verify that encrypted tablespace exists
    async fn verify_tablespace(&self, tablespace_name: &str) -> DatabaseResult<()> {
        let result = sqlx::query("SELECT spcname FROM pg_tablespace WHERE spcname = $1")
            .bind(tablespace_name)
            .fetch_optional(&self.pool)
            .await
            .map_err(|e| DatabaseError::QueryError(format!("Failed to check tablespace: {}", e)))?;

        if result.is_none() {
            return Err(DatabaseError::ConfigurationError(
                format!("Encrypted tablespace '{}' does not exist", tablespace_name),
            ));
        }

        Ok(())
    }

    /// Encrypt text using pg_crypto
    pub async fn encrypt_text(&self, plaintext: &str, key: &Zeroizing<String>) -> DatabaseResult<String> {
        let row = sqlx::query(
            "SELECT encode(pgp_sym_encrypt($1, $2, 'cipher-algo=aes256'), 'base64') as ciphertext"
        )
            .bind(plaintext)
            .bind(key.as_str())
            .fetch_one(&self.pool)
            .await
            .map_err(|e| DatabaseError::QueryError(format!("Encryption failed: {}", e)))?;

        row.try_get("ciphertext")
            .map_err(|e| DatabaseError::QueryError(format!("Failed to get ciphertext: {}", e)))
    }

    /// Decrypt text using pg_crypto
    pub async fn decrypt_text(&self, ciphertext: &str, key: &Zeroizing<String>) -> DatabaseResult<Zeroizing<String>> {
        let row = sqlx::query(
            "SELECT pgp_sym_decrypt(decode($1, 'base64'), $2)::text as plaintext"
        )
            .bind(ciphertext)
            .bind(key.as_str())
            .fetch_one(&self.pool)
            .await
            .map_err(|e| DatabaseError::QueryError(format!("Decryption failed: {}", e)))?;

        let plaintext: String = row.try_get("plaintext")
            .map_err(|e| DatabaseError::QueryError(format!("Failed to get plaintext: {}", e)))?;

        Ok(Zeroizing::new(plaintext))
    }

    /// Encrypt binary data using pg_crypto
    pub async fn encrypt_bytea(&self, plaindata: &[u8], key: &Zeroizing<String>) -> DatabaseResult<Vec<u8>> {
        let row = sqlx::query(
            "SELECT pgp_sym_encrypt_bytea($1, $2, 'cipher-algo=aes256') as cipherdata"
        )
            .bind(plaindata)
            .bind(key.as_str())
            .fetch_one(&self.pool)
            .await
            .map_err(|e| DatabaseError::QueryError(format!("Encryption failed: {}", e)))?;

        row.try_get("cipherdata")
            .map_err(|e| DatabaseError::QueryError(format!("Failed to get cipherdata: {}", e)))
    }

    /// Decrypt binary data using pg_crypto
    pub async fn decrypt_bytea(&self, cipherdata: &[u8], key: &Zeroizing<String>) -> DatabaseResult<Zeroizing<Vec<u8>>> {
        let row = sqlx::query(
            "SELECT pgp_sym_decrypt_bytea($1, $2) as plaindata"
        )
            .bind(cipherdata)
            .bind(key.as_str())
            .fetch_one(&self.pool)
            .await
            .map_err(|e| DatabaseError::QueryError(format!("Decryption failed: {}", e)))?;

        let plaindata: Vec<u8> = row.try_get("plaindata")
            .map_err(|e| DatabaseError::QueryError(format!("Failed to get plaindata: {}", e)))?;

        Ok(Zeroizing::new(plaindata))
    }

    /// Get current encryption key version
    pub async fn get_active_key_version(&self) -> DatabaseResult<Option<i32>> {
        let result = sqlx::query(
            "SELECT version FROM encryption_key_versions WHERE is_active = true LIMIT 1"
        )
            .fetch_optional(&self.pool)
            .await
            .map_err(|e| DatabaseError::QueryError(format!("Failed to get key version: {}", e)))?;

        match result {
            Some(row) => {
                let version: i32 = row.try_get("version")
                    .map_err(|e| DatabaseError::QueryError(format!("Failed to parse version: {}", e)))?;
                Ok(Some(version))
            }
            None => Ok(None),
        }
    }

    /// Rotate encryption keys for all encrypted data
    pub async fn rotate_encryption_keys(
        &self,
        old_key: &Zeroizing<String>,
        new_key: &Zeroizing<String>,
        new_version: i32,
    ) -> DatabaseResult<i64> {
        // Register new key version
        sqlx::query(
            "INSERT INTO encryption_key_versions (version, key_id, algorithm, is_active) 
             VALUES ($1, $2, 'AES-256-GCM', false)"
        )
            .bind(new_version)
            .bind(format!("v{}", new_version))
            .execute(&self.pool)
            .await
            .map_err(|e| DatabaseError::QueryError(format!("Failed to register key version: {}", e)))?;

        // Call rotation function
        let row = sqlx::query(
            "SELECT rotate_encryption_key($1, $2, $3) as records_updated"
        )
            .bind(old_key.as_str())
            .bind(new_key.as_str())
            .bind(new_version)
            .fetch_one(&self.pool)
            .await
            .map_err(|e| DatabaseError::QueryError(format!("Key rotation failed: {}", e)))?;

        let records_updated: i32 = row.try_get("records_updated")
            .map_err(|e| DatabaseError::QueryError(format!("Failed to get update count: {}", e)))?;

        // Mark old version as retired and activate new version
        sqlx::query(
            "UPDATE encryption_key_versions 
             SET retired_at = NOW(), is_active = false 
             WHERE version < $1"
        )
            .bind(new_version)
            .execute(&self.pool)
            .await
            .map_err(|e| DatabaseError::QueryError(format!("Failed to retire old keys: {}", e)))?;

        sqlx::query(
            "UPDATE encryption_key_versions 
             SET is_active = true 
             WHERE version = $1"
        )
            .bind(new_version)
            .execute(&self.pool)
            .await
            .map_err(|e| DatabaseError::QueryError(format!("Failed to activate new key: {}", e)))?;

        Ok(records_updated as i64)
    }

    /// Get encryption compliance status for all tables
    pub async fn get_compliance_status(&self) -> DatabaseResult<Vec<TableComplianceStatus>> {
        let rows = sqlx::query(
            "SELECT 
                table_schema,
                table_name,
                encrypted_columns,
                total_columns,
                tablespace,
                compliance_status
             FROM encryption_compliance_check"
        )
            .fetch_all(&self.pool)
            .await
            .map_err(|e| DatabaseError::QueryError(format!("Failed to get compliance status: {}", e)))?;

        let mut statuses = Vec::new();
        for row in rows {
            statuses.push(TableComplianceStatus {
                schema: row.try_get("table_schema")?,
                table: row.try_get("table_name")?,
                encrypted_columns: row.try_get("encrypted_columns")?,
                total_columns: row.try_get("total_columns")?,
                tablespace: row.try_get("tablespace")?,
                status: row.try_get("compliance_status")?,
            });
        }

        Ok(statuses)
    }

    /// Get SSL/TLS connection statistics
    pub async fn get_ssl_stats(&self) -> DatabaseResult<Vec<SslConnectionInfo>> {
        let rows = sqlx::query(
            "SELECT 
                datname,
                usename,
                application_name,
                client_addr::text,
                CASE WHEN ssl THEN 'Encrypted' ELSE 'Unencrypted' END as security_status,
                version as ssl_version,
                cipher as ssl_cipher
             FROM pg_stat_ssl
             JOIN pg_stat_activity ON pg_stat_ssl.pid = pg_stat_activity.pid
             WHERE pg_stat_activity.datname IS NOT NULL"
        )
            .fetch_all(&self.pool)
            .await
            .map_err(|e| DatabaseError::QueryError(format!("Failed to get SSL stats: {}", e)))?;

        let mut stats = Vec::new();
        for row in rows {
            stats.push(SslConnectionInfo {
                database: row.try_get("datname")?,
                username: row.try_get("usename")?,
                application: row.try_get("application_name")?,
                client_addr: row.try_get("client_addr")?,
                security_status: row.try_get("security_status")?,
                ssl_version: row.try_get("ssl_version").ok(),
                ssl_cipher: row.try_get("ssl_cipher").ok(),
            });
        }

        Ok(stats)
    }

    /// Check if database requires TDE key rotation
    pub async fn needs_key_rotation(&self) -> DatabaseResult<bool> {
        let result = sqlx::query(
            "SELECT 
                EXTRACT(DAY FROM (NOW() - created_at)) as days_since_creation
             FROM encryption_key_versions
             WHERE is_active = true
             LIMIT 1"
        )
            .fetch_optional(&self.pool)
            .await
            .map_err(|e| DatabaseError::QueryError(format!("Failed to check rotation status: {}", e)))?;

        match result {
            Some(row) => {
                let days: f64 = row.try_get("days_since_creation")?;
                Ok(days >= self.config.key_rotation_days as f64)
            }
            None => Ok(true), // No active key version means rotation is needed
        }
    }
}

/// Table encryption compliance status
#[derive(Debug, Clone)]
pub struct TableComplianceStatus {
    pub schema: String,
    pub table: String,
    pub encrypted_columns: i64,
    pub total_columns: i64,
    pub tablespace: String,
    pub status: String,
}

/// SSL/TLS connection information
#[derive(Debug, Clone)]
pub struct SslConnectionInfo {
    pub database: String,
    pub username: String,
    pub application: String,
    pub client_addr: String,
    pub security_status: String,
    pub ssl_version: Option<String>,
    pub ssl_cipher: Option<String>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_tde_config_default() {
        let config = TdeConfig::default();
        assert!(config.enabled);
        assert!(config.require_ssl);
        assert_eq!(config.min_tls_version, "TLSv1.2");
        assert_eq!(config.key_rotation_days, 90);
    }

    #[test]
    fn test_table_compliance_status() {
        let status = TableComplianceStatus {
            schema: "public".to_string(),
            table: "patients".to_string(),
            encrypted_columns: 5,
            total_columns: 20,
            tablespace: "encrypted_data".to_string(),
            status: "COMPLIANT".to_string(),
        };

        assert_eq!(status.encrypted_columns, 5);
        assert_eq!(status.status, "COMPLIANT");
    }
}
