// Database encryption utilities
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Encryption configuration for database fields
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EncryptionConfig {
    pub enabled: bool,
    pub field_mappings: HashMap<String, FieldEncryptionConfig>,
    pub master_key: Option<String>,
}

impl Default for EncryptionConfig {
    fn default() -> Self {
        Self {
            enabled: false,
            field_mappings: HashMap::new(),
            master_key: None,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FieldEncryptionConfig {
    pub table: String,
    pub column: String,
    pub algorithm: EncryptionAlgorithm,
    pub searchable: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum EncryptionAlgorithm {
    AES256,
    ChaCha20,
}

pub struct DatabaseEncryption {
    config: EncryptionConfig,
}

impl DatabaseEncryption {
    pub fn new(config: EncryptionConfig) -> Self {
        Self { config }
    }

    pub fn is_enabled(&self) -> bool {
        self.config.enabled
    }

    /// Check if a field should be encrypted
    pub fn should_encrypt(&self, table: &str, column: &str) -> bool {
        if !self.config.enabled {
            return false;
        }

        let key = format!("{}_{}", table, column);
        self.config.field_mappings.contains_key(&key)
    }
}