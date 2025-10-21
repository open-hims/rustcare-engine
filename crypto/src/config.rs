//! Security configuration module
//!
//! Provides comprehensive configuration for all cryptographic operations:
//! - Encryption algorithms and modes
//! - Key management service (KMS) providers
//! - Key derivation parameters
//! - Memory security settings
//! - Compliance requirements

use crate::error::{CryptoError, CryptoResult};
use std::path::PathBuf;
use std::str::FromStr;
use std::time::Duration;
use base64::{Engine as _, engine::general_purpose};

/// KMS Provider type
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum KmsProvider {
    /// AWS Key Management Service
    AwsKms,
    /// HashiCorp Vault Transit Engine
    Vault,
    /// Local file-based key storage (development only)
    Local,
    /// No KMS (uses master key directly)
    None,
}

impl FromStr for KmsProvider {
    type Err = CryptoError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.to_lowercase().as_str() {
            "aws_kms" | "aws-kms" | "awskms" => Ok(KmsProvider::AwsKms),
            "vault" | "hashicorp-vault" | "hcvault" => Ok(KmsProvider::Vault),
            "local" | "file" => Ok(KmsProvider::Local),
            "none" | "disabled" => Ok(KmsProvider::None),
            _ => Err(CryptoError::Configuration(format!(
                "Unknown KMS provider: {}. Valid options: aws_kms, vault, local, none",
                s
            ))),
        }
    }
}

impl Default for KmsProvider {
    fn default() -> Self {
        KmsProvider::None
    }
}

/// Encryption algorithm configuration
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum EncryptionAlgorithm {
    /// AES-256-GCM (recommended)
    Aes256Gcm,
    /// AES-256-CBC (legacy)
    Aes256Cbc,
    /// ChaCha20-Poly1305 (alternative)
    ChaCha20Poly1305,
}

impl FromStr for EncryptionAlgorithm {
    type Err = CryptoError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.to_lowercase().as_str() {
            "aes-256-gcm" | "aes256gcm" => Ok(EncryptionAlgorithm::Aes256Gcm),
            "aes-256-cbc" | "aes256cbc" => Ok(EncryptionAlgorithm::Aes256Cbc),
            "chacha20-poly1305" | "chacha20poly1305" => Ok(EncryptionAlgorithm::ChaCha20Poly1305),
            _ => Err(CryptoError::Configuration(format!(
                "Unknown encryption algorithm: {}. Valid options: aes-256-gcm, aes-256-cbc, chacha20-poly1305",
                s
            ))),
        }
    }
}

impl Default for EncryptionAlgorithm {
    fn default() -> Self {
        EncryptionAlgorithm::Aes256Gcm
    }
}

/// Key Derivation Function configuration
#[derive(Debug, Clone)]
pub enum KdfAlgorithm {
    /// PBKDF2-HMAC-SHA256
    Pbkdf2 { iterations: u32 },
    /// Argon2id (recommended)
    Argon2id { memory_cost: u32, iterations: u32, parallelism: u32 },
    /// HKDF-SHA256
    Hkdf,
}

impl Default for KdfAlgorithm {
    fn default() -> Self {
        // Argon2id with OWASP recommended parameters
        KdfAlgorithm::Argon2id {
            memory_cost: 19 * 1024, // 19 MB
            iterations: 2,
            parallelism: 1,
        }
    }
}

/// Security configuration
#[derive(Debug, Clone)]
pub struct SecurityConfig {
    // === KMS Configuration ===
    /// KMS provider (aws_kms, vault, local, none)
    pub kms_provider: KmsProvider,
    
    /// AWS KMS configuration
    pub aws_kms_config: Option<AwsKmsConfig>,
    
    /// Vault configuration
    pub vault_config: Option<VaultConfig>,
    
    /// Local key storage path (development only)
    pub local_key_path: Option<PathBuf>,
    
    // === Encryption Configuration ===
    /// Encryption algorithm
    pub encryption_algorithm: EncryptionAlgorithm,
    
    /// Master encryption key (base64-encoded, 32 bytes for AES-256)
    /// This is used when KMS is disabled or as a fallback
    pub master_key: Option<Vec<u8>>,
    
    /// Current key version (for rotation support)
    pub key_version: u32,
    
    /// Enable envelope encryption (DEK/KEK pattern)
    pub enable_envelope_encryption: bool,
    
    /// Threshold for envelope encryption (bytes)
    /// Objects larger than this use envelope encryption
    pub envelope_threshold: usize,
    
    // === Key Derivation Configuration ===
    /// Key derivation function for password-based keys
    pub kdf_algorithm: KdfAlgorithm,
    
    // === Memory Security ===
    /// Enable memory locking (mlock) for sensitive data
    pub enable_memory_locking: bool,
    
    /// Enable constant-time operations
    pub enable_constant_time: bool,
    
    /// Enable guard pages for buffer overflow detection
    pub enable_guard_pages: bool,
    
    // === Database Encryption ===
    /// Enable database Transparent Data Encryption (TDE)
    pub enable_database_tde: bool,
    
    /// Database encryption key (separate from object encryption)
    pub database_encryption_key: Option<Vec<u8>>,
    
    // === Object Storage Encryption ===
    /// Storage backend type (filesystem, s3)
    pub storage_backend: String,
    
    /// Encryption threshold for object storage (bytes)
    /// Objects smaller than this may skip encryption for performance
    pub storage_encryption_threshold: usize,
    
    /// Enable client-side encryption for S3
    pub enable_s3_client_side_encryption: bool,
    
    // === Compliance & Audit ===
    /// Enable FIPS 140-2 mode (if available)
    pub enable_fips_mode: bool,
    
    /// Security audit log path
    pub audit_log_path: Option<PathBuf>,
    
    /// Enable detailed crypto operation logging
    pub enable_crypto_audit: bool,
    
    /// Key rotation interval (days)
    pub key_rotation_interval_days: u32,
    
    // === Performance ===
    /// Enable DEK caching
    pub enable_dek_cache: bool,
    
    /// DEK cache TTL (seconds)
    pub dek_cache_ttl_seconds: u64,
    
    /// Maximum DEK cache size (number of keys)
    pub dek_cache_max_size: usize,
}

/// AWS KMS configuration
#[derive(Debug, Clone)]
pub struct AwsKmsConfig {
    /// AWS KMS Key ID or ARN
    pub key_id: String,
    
    /// AWS Region
    pub region: String,
    
    /// AWS Access Key ID (optional, uses IAM role if not provided)
    pub access_key_id: Option<String>,
    
    /// AWS Secret Access Key (optional)
    pub secret_access_key: Option<String>,
    
    /// Connection timeout
    pub timeout: Duration,
    
    /// Enable key rotation
    pub enable_rotation: bool,
}

/// HashiCorp Vault configuration
#[derive(Debug, Clone)]
pub struct VaultConfig {
    /// Vault server address (e.g., https://vault.company.com:8200)
    pub addr: String,
    
    /// Vault authentication token
    pub token: String,
    
    /// Transit engine mount path
    pub mount_path: String,
    
    /// Key name in Vault
    pub key_name: String,
    
    /// Connection timeout
    pub timeout: Duration,
    
    /// Enable TLS certificate verification
    pub verify_tls: bool,
    
    /// CA certificate path (optional)
    pub ca_cert_path: Option<PathBuf>,
}

impl Default for SecurityConfig {
    fn default() -> Self {
        Self {
            kms_provider: KmsProvider::None,
            aws_kms_config: None,
            vault_config: None,
            local_key_path: None,
            encryption_algorithm: EncryptionAlgorithm::Aes256Gcm,
            master_key: None,
            key_version: 1,
            enable_envelope_encryption: true,
            envelope_threshold: 1024 * 1024, // 1 MB
            kdf_algorithm: KdfAlgorithm::default(),
            enable_memory_locking: true,
            enable_constant_time: true,
            enable_guard_pages: true,
            enable_database_tde: false,
            database_encryption_key: None,
            storage_backend: "filesystem".to_string(),
            storage_encryption_threshold: 1024 * 1024, // 1 MB
            enable_s3_client_side_encryption: true,
            enable_fips_mode: false,
            audit_log_path: None,
            enable_crypto_audit: false,
            key_rotation_interval_days: 90,
            enable_dek_cache: true,
            dek_cache_ttl_seconds: 3600, // 1 hour
            dek_cache_max_size: 1000,
        }
    }
}

impl SecurityConfig {
    /// Create a new security configuration from environment variables
    pub fn from_env() -> CryptoResult<Self> {
        let mut config = Self::default();
        
        // === KMS Provider ===
        if let Ok(provider) = std::env::var("KMS_PROVIDER") {
            config.kms_provider = provider.parse()?;
        }
        
        // === AWS KMS Configuration ===
        if config.kms_provider == KmsProvider::AwsKms {
            config.aws_kms_config = Some(AwsKmsConfig {
                key_id: std::env::var("AWS_KMS_KEY_ID")
                    .map_err(|_| CryptoError::Configuration(
                        "AWS_KMS_KEY_ID is required when using AWS KMS".to_string()
                    ))?,
                region: std::env::var("AWS_REGION")
                    .or_else(|_| std::env::var("AWS_DEFAULT_REGION"))
                    .unwrap_or_else(|_| "us-east-1".to_string()),
                access_key_id: std::env::var("AWS_ACCESS_KEY_ID").ok(),
                secret_access_key: std::env::var("AWS_SECRET_ACCESS_KEY").ok(),
                timeout: Duration::from_secs(
                    std::env::var("AWS_KMS_TIMEOUT_SECONDS")
                        .ok()
                        .and_then(|s| s.parse().ok())
                        .unwrap_or(30)
                ),
                enable_rotation: std::env::var("AWS_KMS_ENABLE_ROTATION")
                    .ok()
                    .and_then(|s| s.parse().ok())
                    .unwrap_or(true),
            });
        }
        
        // === Vault Configuration ===
        if config.kms_provider == KmsProvider::Vault {
            config.vault_config = Some(VaultConfig {
                addr: std::env::var("VAULT_ADDR")
                    .map_err(|_| CryptoError::Configuration(
                        "VAULT_ADDR is required when using Vault KMS".to_string()
                    ))?,
                token: std::env::var("VAULT_TOKEN")
                    .map_err(|_| CryptoError::Configuration(
                        "VAULT_TOKEN is required when using Vault KMS".to_string()
                    ))?,
                mount_path: std::env::var("VAULT_MOUNT_PATH")
                    .unwrap_or_else(|_| "transit".to_string()),
                key_name: std::env::var("VAULT_KEY_NAME")
                    .unwrap_or_else(|_| "rustcare-master-key".to_string()),
                timeout: Duration::from_secs(
                    std::env::var("VAULT_TIMEOUT_SECONDS")
                        .ok()
                        .and_then(|s| s.parse().ok())
                        .unwrap_or(30)
                ),
                verify_tls: std::env::var("VAULT_VERIFY_TLS")
                    .ok()
                    .and_then(|s| s.parse().ok())
                    .unwrap_or(true),
                ca_cert_path: std::env::var("VAULT_CA_CERT_PATH")
                    .ok()
                    .map(PathBuf::from),
            });
        }
        
        // === Local Key Storage ===
        if config.kms_provider == KmsProvider::Local {
            config.local_key_path = Some(
                std::env::var("LOCAL_KEY_PATH")
                    .map(PathBuf::from)
                    .unwrap_or_else(|_| PathBuf::from("./config/keys/master.key"))
            );
        }
        
        // === Encryption Configuration ===
        if let Ok(algo) = std::env::var("ENCRYPTION_ALGORITHM") {
            config.encryption_algorithm = algo.parse()?;
        }
        
        if let Ok(key_b64) = std::env::var("MASTER_ENCRYPTION_KEY") {
            if key_b64 != "CHANGE_ME_generate_with_openssl_rand_base64_32" {
                config.master_key = Some(
                    general_purpose::STANDARD.decode(&key_b64)
                        .map_err(|e| CryptoError::Configuration(
                            format!("Invalid base64 master key: {}", e)
                        ))?
                );
            }
        }
        
        if let Ok(version) = std::env::var("ENCRYPTION_KEY_VERSION") {
            config.key_version = version.parse()
                .map_err(|e| CryptoError::Configuration(
                    format!("Invalid key version: {}", e)
                ))?;
        }
        
        config.enable_envelope_encryption = std::env::var("ENABLE_ENVELOPE_ENCRYPTION")
            .ok()
            .and_then(|s| s.parse().ok())
            .unwrap_or(true);
        
        if let Ok(threshold) = std::env::var("ENVELOPE_THRESHOLD_BYTES") {
            config.envelope_threshold = threshold.parse()
                .map_err(|e| CryptoError::Configuration(
                    format!("Invalid envelope threshold: {}", e)
                ))?;
        }
        
        // === Memory Security ===
        config.enable_memory_locking = std::env::var("ENABLE_MEMORY_LOCKING")
            .ok()
            .and_then(|s| s.parse().ok())
            .unwrap_or(true);
        
        config.enable_constant_time = std::env::var("ENABLE_CONSTANT_TIME_OPS")
            .ok()
            .and_then(|s| s.parse().ok())
            .unwrap_or(true);
        
        config.enable_guard_pages = std::env::var("ENABLE_GUARD_PAGES")
            .ok()
            .and_then(|s| s.parse().ok())
            .unwrap_or(true);
        
        // === Database Encryption ===
        config.enable_database_tde = std::env::var("ENABLE_DATABASE_TDE")
            .ok()
            .and_then(|s| s.parse().ok())
            .unwrap_or(false);
        
        if let Ok(key_b64) = std::env::var("DATABASE_ENCRYPTION_KEY") {
            config.database_encryption_key = Some(
                general_purpose::STANDARD.decode(&key_b64)
                    .map_err(|e| CryptoError::Configuration(
                        format!("Invalid base64 database key: {}", e)
                    ))?
            );
        }
        
        // === Object Storage ===
        config.storage_backend = std::env::var("STORAGE_BACKEND")
            .unwrap_or_else(|_| "filesystem".to_string());
        
        if let Ok(threshold) = std::env::var("STORAGE_ENCRYPTION_THRESHOLD") {
            config.storage_encryption_threshold = threshold.parse()
                .map_err(|e| CryptoError::Configuration(
                    format!("Invalid storage threshold: {}", e)
                ))?;
        }
        
        config.enable_s3_client_side_encryption = std::env::var("ENABLE_S3_CLIENT_SIDE_ENCRYPTION")
            .ok()
            .and_then(|s| s.parse().ok())
            .unwrap_or(true);
        
        // === Compliance & Audit ===
        config.enable_fips_mode = std::env::var("ENABLE_FIPS_MODE")
            .ok()
            .and_then(|s| s.parse().ok())
            .unwrap_or(false);
        
        config.audit_log_path = std::env::var("SECURITY_AUDIT_LOG")
            .ok()
            .map(PathBuf::from);
        
        config.enable_crypto_audit = std::env::var("ENABLE_CRYPTO_AUDIT")
            .ok()
            .and_then(|s| s.parse().ok())
            .unwrap_or(false);
        
        if let Ok(days) = std::env::var("KEY_ROTATION_INTERVAL_DAYS") {
            config.key_rotation_interval_days = days.parse()
                .map_err(|e| CryptoError::Configuration(
                    format!("Invalid rotation interval: {}", e)
                ))?;
        }
        
        // === Performance ===
        config.enable_dek_cache = std::env::var("ENABLE_DEK_CACHE")
            .ok()
            .and_then(|s| s.parse().ok())
            .unwrap_or(true);
        
        if let Ok(ttl) = std::env::var("DEK_CACHE_TTL_SECONDS") {
            config.dek_cache_ttl_seconds = ttl.parse()
                .map_err(|e| CryptoError::Configuration(
                    format!("Invalid DEK cache TTL: {}", e)
                ))?;
        }
        
        if let Ok(size) = std::env::var("DEK_CACHE_MAX_SIZE") {
            config.dek_cache_max_size = size.parse()
                .map_err(|e| CryptoError::Configuration(
                    format!("Invalid DEK cache size: {}", e)
                ))?;
        }
        
        // Validate configuration
        config.validate()?;
        
        Ok(config)
    }
    
    /// Validate the security configuration
    pub fn validate(&self) -> CryptoResult<()> {
        // Ensure we have a master key when KMS is disabled
        if self.kms_provider == KmsProvider::None && self.master_key.is_none() {
            return Err(CryptoError::Configuration(
                "MASTER_ENCRYPTION_KEY is required when KMS is disabled".to_string()
            ));
        }
        
        // Validate master key length (should be 32 bytes for AES-256)
        if let Some(key) = &self.master_key {
            if key.len() != 32 {
                return Err(CryptoError::Configuration(
                    format!("Master key must be 32 bytes (256 bits), got {} bytes", key.len())
                ));
            }
        }
        
        // Validate database key length
        if let Some(key) = &self.database_encryption_key {
            if key.len() != 32 {
                return Err(CryptoError::Configuration(
                    format!("Database key must be 32 bytes (256 bits), got {} bytes", key.len())
                ));
            }
        }
        
        // Validate KMS-specific configs
        match &self.kms_provider {
            KmsProvider::AwsKms => {
                if self.aws_kms_config.is_none() {
                    return Err(CryptoError::Configuration(
                        "AWS KMS configuration is required when using AWS KMS provider".to_string()
                    ));
                }
            }
            KmsProvider::Vault => {
                if self.vault_config.is_none() {
                    return Err(CryptoError::Configuration(
                        "Vault configuration is required when using Vault provider".to_string()
                    ));
                }
            }
            KmsProvider::Local => {
                if self.local_key_path.is_none() {
                    return Err(CryptoError::Configuration(
                        "Local key path is required when using local provider".to_string()
                    ));
                }
            }
            KmsProvider::None => {}
        }
        
        Ok(())
    }
    
    /// Check if KMS is enabled
    pub fn is_kms_enabled(&self) -> bool {
        self.kms_provider != KmsProvider::None
    }
    
    /// Check if memory security features are enabled
    pub fn is_memory_security_enabled(&self) -> bool {
        self.enable_memory_locking || self.enable_guard_pages
    }
    
    /// Check if all security hardening features are enabled
    pub fn is_fully_hardened(&self) -> bool {
        self.enable_memory_locking 
            && self.enable_constant_time 
            && self.enable_guard_pages
            && self.enable_envelope_encryption
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_kms_provider_from_str() {
        assert_eq!("aws_kms".parse::<KmsProvider>().unwrap(), KmsProvider::AwsKms);
        assert_eq!("vault".parse::<KmsProvider>().unwrap(), KmsProvider::Vault);
        assert_eq!("local".parse::<KmsProvider>().unwrap(), KmsProvider::Local);
        assert_eq!("none".parse::<KmsProvider>().unwrap(), KmsProvider::None);
        assert!("invalid".parse::<KmsProvider>().is_err());
    }
    
    #[test]
    fn test_encryption_algorithm_from_str() {
        assert_eq!(
            "aes-256-gcm".parse::<EncryptionAlgorithm>().unwrap(),
            EncryptionAlgorithm::Aes256Gcm
        );
        assert_eq!(
            "aes-256-cbc".parse::<EncryptionAlgorithm>().unwrap(),
            EncryptionAlgorithm::Aes256Cbc
        );
        assert!("invalid".parse::<EncryptionAlgorithm>().is_err());
    }
    
    #[test]
    fn test_default_config() {
        let config = SecurityConfig::default();
        assert_eq!(config.kms_provider, KmsProvider::None);
        assert_eq!(config.encryption_algorithm, EncryptionAlgorithm::Aes256Gcm);
        assert!(config.enable_envelope_encryption);
        assert!(config.enable_memory_locking);
        assert!(config.enable_constant_time);
        assert!(config.enable_guard_pages);
    }
    
    #[test]
    fn test_validate_requires_master_key() {
        let mut config = SecurityConfig::default();
        config.kms_provider = KmsProvider::None;
        config.master_key = None;
        
        assert!(config.validate().is_err());
        
        config.master_key = Some(vec![0u8; 32]);
        assert!(config.validate().is_ok());
    }
    
    #[test]
    fn test_validate_master_key_length() {
        let mut config = SecurityConfig::default();
        config.master_key = Some(vec![0u8; 16]); // Wrong length
        
        assert!(config.validate().is_err());
        
        config.master_key = Some(vec![0u8; 32]); // Correct length
        assert!(config.validate().is_ok());
    }
    
    #[test]
    fn test_is_kms_enabled() {
        let mut config = SecurityConfig::default();
        assert!(!config.is_kms_enabled());
        
        config.kms_provider = KmsProvider::AwsKms;
        assert!(config.is_kms_enabled());
    }
    
    #[test]
    fn test_is_fully_hardened() {
        let config = SecurityConfig::default();
        assert!(config.is_fully_hardened());
        
        let mut config = SecurityConfig::default();
        config.enable_memory_locking = false;
        assert!(!config.is_fully_hardened());
    }
}
