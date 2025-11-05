//! Local Database Encryption
//!
//! Provides encryption key management for SQLite databases using SQLCipher.
//! Implements HIPAA-compliant AES-256 encryption at rest.
//!
//! Features:
//! - Key derivation from master key using existing crypto::kdf
//! - Integration with existing crypto crate
//! - Secure key storage and rotation
//! - Automatic encryption pragma configuration
//!
//! Security:
//! - AES-256-CBC encryption (SQLCipher)
//! - PBKDF2 key derivation (600,000 iterations default)
//! - Random salt per database
//! - Memory-safe key handling with zeroize

use crate::error::{SyncError, SyncResult};
use crypto::kdf::{Kdf, Pbkdf2Params as CryptoPbkdf2Params};
use serde::{Deserialize, Serialize};
use std::path::Path;
use uuid::Uuid;
use zeroize::{Zeroize, ZeroizeOnDrop};

/// Serializable PBKDF2 parameters
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Pbkdf2Params {
    /// Number of iterations
    pub iterations: u32,
    /// Salt length in bytes
    pub salt_length: usize,
}

impl Default for Pbkdf2Params {
    fn default() -> Self {
        Self {
            iterations: 600_000, // OWASP 2023 recommendation
            salt_length: 32,
        }
    }
}

impl From<Pbkdf2Params> for CryptoPbkdf2Params {
    fn from(params: Pbkdf2Params) -> Self {
        CryptoPbkdf2Params {
            iterations: params.iterations,
            salt_length: params.salt_length,
        }
    }
}

/// Database encryption configuration
#[derive(Debug, Clone)]
pub struct EncryptionConfig {
    /// Enable encryption (requires SQLCipher)
    pub enabled: bool,
    /// Master key for key derivation
    pub master_key: Option<Vec<u8>>,
    /// PBKDF2 parameters
    pub pbkdf2_params: Pbkdf2Params,
}

impl Default for EncryptionConfig {
    fn default() -> Self {
        Self {
            enabled: false,
            master_key: None,
            pbkdf2_params: Pbkdf2Params::default(), // 600,000 iterations from crypto crate
        }
    }
}

/// Database encryption key manager
pub struct EncryptionKeyManager {
    config: EncryptionConfig,
}

/// Database encryption key (zeroized on drop)
#[derive(Zeroize, ZeroizeOnDrop)]
pub struct DatabaseKey {
    key: Vec<u8>,
    salt: Vec<u8>,
}

impl DatabaseKey {
    /// Create a new database key
    pub fn new(key: Vec<u8>, salt: Vec<u8>) -> Self {
        Self { key, salt }
    }
    
    /// Get the key bytes
    pub fn key(&self) -> &[u8] {
        &self.key
    }
    
    /// Get the salt bytes
    pub fn salt(&self) -> &[u8] {
        &self.salt
    }
    
    /// Convert key to hex string for SQLCipher pragma
    pub fn to_hex(&self) -> String {
        hex::encode(&self.key)
    }
}

/// Encryption metadata stored alongside database
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EncryptionMetadata {
    /// Database ID
    pub database_id: Uuid,
    /// Salt for key derivation
    pub salt: Vec<u8>,
    /// PBKDF2 parameters
    pub pbkdf2_params: Pbkdf2Params,
    /// Creation timestamp
    pub created_at: chrono::DateTime<chrono::Utc>,
    /// Last key rotation timestamp
    pub last_rotated: Option<chrono::DateTime<chrono::Utc>>,
}

impl EncryptionKeyManager {
    /// Create a new encryption key manager
    pub fn new(config: EncryptionConfig) -> Self {
        Self { config }
    }
    
    /// Generate a new database encryption key
    ///
    /// Derives a key from the master key using PBKDF2 with a random salt.
    pub fn generate_key(&self) -> SyncResult<DatabaseKey> {
        if !self.config.enabled {
            return Err(SyncError::Internal("Encryption not enabled".to_string()));
        }
        
        let master_key = self.config.master_key.as_ref()
            .ok_or_else(|| SyncError::Internal("Master key not configured".to_string()))?;
        
        // Generate random salt using crypto crate
        let salt = Kdf::generate_salt(self.config.pbkdf2_params.salt_length);
        
        // Derive 32-byte AES-256 key using crypto::kdf
        let crypto_params = self.config.pbkdf2_params.clone().into();
        let key = Kdf::derive_aes256_key(
            master_key,
            &salt,
            &crypto_params,
        ).map_err(|e| SyncError::Encryption(e))?;
        
        Ok(DatabaseKey::new(key.to_vec(), salt))
    }
    
    /// Derive key from existing salt
    ///
    /// Used when opening an existing encrypted database.
    pub fn derive_key(&self, salt: &[u8]) -> SyncResult<DatabaseKey> {
        if !self.config.enabled {
            return Err(SyncError::Internal("Encryption not enabled".to_string()));
        }
        
        let master_key = self.config.master_key.as_ref()
            .ok_or_else(|| SyncError::Internal("Master key not configured".to_string()))?;
        
        // Derive key using crypto::kdf
        let crypto_params = self.config.pbkdf2_params.clone().into();
        let key = Kdf::derive_aes256_key(
            master_key,
            salt,
            &crypto_params,
        ).map_err(|e| SyncError::Encryption(e))?;
        
        Ok(DatabaseKey::new(key.to_vec(), salt.to_vec()))
    }
    
    /// Create encryption metadata for a new database
    pub fn create_metadata(&self, key: &DatabaseKey) -> EncryptionMetadata {
        EncryptionMetadata {
            database_id: Uuid::new_v4(),
            salt: key.salt().to_vec(),
            pbkdf2_params: self.config.pbkdf2_params.clone(),
            created_at: chrono::Utc::now(),
            last_rotated: None,
        }
    }
    
    /// Save encryption metadata to file
    pub fn save_metadata(&self, metadata: &EncryptionMetadata, db_path: &Path) -> SyncResult<()> {
        let metadata_path = Self::metadata_path(db_path);
        let json = serde_json::to_string_pretty(metadata)
            .map_err(|e| SyncError::Serialization(e.to_string()))?;
        
        std::fs::write(&metadata_path, json)
            .map_err(|e| SyncError::Internal(format!("Failed to write metadata: {}", e)))?;
        
        // Set restrictive permissions (owner read/write only)
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            let perms = std::fs::Permissions::from_mode(0o600);
            std::fs::set_permissions(&metadata_path, perms)
                .map_err(|e| SyncError::Internal(format!("Failed to set permissions: {}", e)))?;
        }
        
        Ok(())
    }
    
    /// Load encryption metadata from file
    pub fn load_metadata(&self, db_path: &Path) -> SyncResult<EncryptionMetadata> {
        let metadata_path = Self::metadata_path(db_path);
        let json = std::fs::read_to_string(&metadata_path)
            .map_err(|e| SyncError::NotFound(format!("Metadata not found: {}", e)))?;
        
        let metadata: EncryptionMetadata = serde_json::from_str(&json)
            .map_err(|e| SyncError::Deserialization(e.to_string()))?;
        
        Ok(metadata)
    }
    
    /// Get metadata file path for a database
    fn metadata_path(db_path: &Path) -> std::path::PathBuf {
        let mut path = db_path.to_path_buf();
        path.set_extension("db.meta");
        path
    }
    
    /// Generate SQLCipher pragma statements for database encryption
    pub fn generate_pragma_statements(&self, key: &DatabaseKey) -> Vec<String> {
        vec![
            // Set encryption key (must be first)
            format!("PRAGMA key = \"x'{}'\";", key.to_hex()),
            // Use AES-256-CBC (SQLCipher 4.x default)
            "PRAGMA cipher_page_size = 4096;".to_string(),
            // Use PBKDF2 with configured iterations
            format!("PRAGMA kdf_iter = {};", self.config.pbkdf2_params.iterations),
            // Disable plaintext header (more secure)
            "PRAGMA cipher_plaintext_header_size = 0;".to_string(),
        ]
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::NamedTempFile;
    
    fn create_test_config() -> EncryptionConfig {
        EncryptionConfig {
            enabled: true,
            master_key: Some(b"test_master_key_32_bytes_long!!".to_vec()),
            pbkdf2_params: Pbkdf2Params {
                iterations: 1000, // Low for testing
                salt_length: 32,
            },
        }
    }
    
    #[test]
    fn test_key_generation() {
        let config = create_test_config();
        let manager = EncryptionKeyManager::new(config);
        
        let key = manager.generate_key().unwrap();
        assert_eq!(key.key().len(), 32);
        assert!(!key.salt().is_empty());
    }
    
    #[test]
    fn test_key_derivation() {
        let config = create_test_config();
        let manager = EncryptionKeyManager::new(config);
        
        // Generate initial key
        let key1 = manager.generate_key().unwrap();
        let salt = key1.salt().to_vec();
        
        // Derive key with same salt
        let key2 = manager.derive_key(&salt).unwrap();
        
        // Keys should be identical
        assert_eq!(key1.key(), key2.key());
    }
    
    #[test]
    fn test_metadata_creation() {
        let config = create_test_config();
        let manager = EncryptionKeyManager::new(config);
        let key = manager.generate_key().unwrap();
        
        let metadata = manager.create_metadata(&key);
        assert!(!metadata.database_id.is_nil());
        assert_eq!(metadata.salt, key.salt());
        assert_eq!(metadata.pbkdf2_params.iterations, 1000);
    }
    
    #[test]
    fn test_metadata_save_load() {
        let config = create_test_config();
        let manager = EncryptionKeyManager::new(config);
        let key = manager.generate_key().unwrap();
        let metadata = manager.create_metadata(&key);
        
        let temp_file = NamedTempFile::new().unwrap();
        let db_path = temp_file.path();
        
        // Save metadata
        manager.save_metadata(&metadata, db_path).unwrap();
        
        // Load metadata
        let loaded = manager.load_metadata(db_path).unwrap();
        assert_eq!(loaded.database_id, metadata.database_id);
        assert_eq!(loaded.salt, metadata.salt);
    }
    
    #[test]
    fn test_pragma_generation() {
        let config = create_test_config();
        let manager = EncryptionKeyManager::new(config);
        let key = manager.generate_key().unwrap();
        
        let pragmas = manager.generate_pragma_statements(&key);
        assert!(!pragmas.is_empty());
        assert!(pragmas[0].starts_with("PRAGMA key"));
    }
    
    #[test]
    fn test_disabled_encryption() {
        let config = EncryptionConfig {
            enabled: false,
            ..Default::default()
        };
        let manager = EncryptionKeyManager::new(config);
        
        let result = manager.generate_key();
        assert!(result.is_err());
    }
}
