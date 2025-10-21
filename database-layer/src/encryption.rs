// Database encryption and field masking utilities
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

// Encryption imports
use aes_gcm::{
    aead::{Aead, KeyInit},
    Aes256Gcm, Nonce,
};
use base64::{Engine as _, engine::general_purpose::STANDARD as BASE64};


// =============================================================================
// FIELD MASKING FOR HIPAA COMPLIANCE
// =============================================================================

/// Sensitivity levels for healthcare data (HIPAA-aligned)
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum SensitivityLevel {
    /// Public data - no masking needed
    Public,
    /// Internal - mask in logs only (e.g., email, phone)
    Internal,
    /// Confidential - mask in logs + limited API access (e.g., address, DOB)
    Confidential,
    /// Restricted - encrypt + mask + audit all access (e.g., SSN, MRN)
    Restricted,
    /// ePHI - Protected Health Information - highest security
    ProtectedHealthInfo,
}

impl SensitivityLevel {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Public => "public",
            Self::Internal => "internal",
            Self::Confidential => "confidential",
            Self::Restricted => "restricted",
            Self::ProtectedHealthInfo => "ephi",
        }
    }
}

/// Masking patterns for different field types
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum MaskPattern {
    /// Show first N and last M characters: "John" -> "J**n"
    Partial { show_first: usize, show_last: usize },
    /// Replace entire value with asterisks
    Full,
    /// Redact with placeholder: "sensitive" -> "[REDACTED]"
    Redacted,
    /// Hash value: "sensitive" -> "sha256:abc123..."
    Hashed,
    /// Tokenize: "123-45-6789" -> "TOK_abc123"
    Tokenized,
    /// Custom regex-based masking
    Custom(String),
}

/// Configuration for a sensitive field
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SensitiveField {
    pub name: String,
    pub level: SensitivityLevel,
    pub mask_pattern: MaskPattern,
    pub encryption_required: bool,
    pub audit_access: bool,
}

impl SensitiveField {
    pub fn new(name: &str, level: SensitivityLevel) -> Self {
        Self {
            name: name.to_string(),
            level,
            mask_pattern: MaskPattern::Full,
            encryption_required: false,
            audit_access: false,
        }
    }

    pub fn with_pattern(mut self, pattern: MaskPattern) -> Self {
        self.mask_pattern = pattern;
        self
    }

    pub fn with_encryption(mut self, required: bool) -> Self {
        self.encryption_required = required;
        self
    }

    pub fn with_audit(mut self, audit: bool) -> Self {
        self.audit_access = audit;
        self
    }

    /// Healthcare field definitions per HIPAA Safe Harbor
    pub fn healthcare_fields() -> Vec<Self> {
        vec![
            // HIPAA Identifiers - Must be removed/masked per Safe Harbor rule
            Self::new("ssn", SensitivityLevel::Restricted)
                .with_pattern(MaskPattern::Partial { show_first: 0, show_last: 4 })
                .with_encryption(true)
                .with_audit(true),
            
            Self::new("social_security_number", SensitivityLevel::Restricted)
                .with_pattern(MaskPattern::Partial { show_first: 0, show_last: 4 })
                .with_encryption(true)
                .with_audit(true),
            
            Self::new("medical_record_number", SensitivityLevel::Restricted)
                .with_pattern(MaskPattern::Tokenized)
                .with_encryption(true)
                .with_audit(true),
            
            Self::new("mrn", SensitivityLevel::Restricted)
                .with_pattern(MaskPattern::Tokenized)
                .with_encryption(true)
                .with_audit(true),
            
            Self::new("health_insurance_number", SensitivityLevel::Restricted)
                .with_pattern(MaskPattern::Tokenized)
                .with_encryption(true)
                .with_audit(true),
            
            Self::new("patient_id", SensitivityLevel::Restricted)
                .with_pattern(MaskPattern::Tokenized)
                .with_encryption(false)
                .with_audit(true),
            
            // Demographics
            Self::new("date_of_birth", SensitivityLevel::Confidential)
                .with_pattern(MaskPattern::Partial { show_first: 4, show_last: 0 }) // Show year only
                .with_encryption(false)
                .with_audit(true),
            
            Self::new("dob", SensitivityLevel::Confidential)
                .with_pattern(MaskPattern::Partial { show_first: 4, show_last: 0 })
                .with_encryption(false)
                .with_audit(true),
            
            Self::new("birth_date", SensitivityLevel::Confidential)
                .with_pattern(MaskPattern::Partial { show_first: 4, show_last: 0 })
                .with_encryption(false)
                .with_audit(true),
            
            Self::new("full_name", SensitivityLevel::Confidential)
                .with_pattern(MaskPattern::Partial { show_first: 1, show_last: 1 })
                .with_encryption(false)
                .with_audit(false),
            
            Self::new("first_name", SensitivityLevel::Internal)
                .with_pattern(MaskPattern::Partial { show_first: 1, show_last: 0 })
                .with_encryption(false)
                .with_audit(false),
            
            Self::new("last_name", SensitivityLevel::Internal)
                .with_pattern(MaskPattern::Partial { show_first: 1, show_last: 0 })
                .with_encryption(false)
                .with_audit(false),
            
            Self::new("email", SensitivityLevel::Internal)
                .with_pattern(MaskPattern::Partial { show_first: 3, show_last: 0 })
                .with_encryption(false)
                .with_audit(false),
            
            Self::new("phone_number", SensitivityLevel::Internal)
                .with_pattern(MaskPattern::Partial { show_first: 0, show_last: 4 })
                .with_encryption(false)
                .with_audit(false),
            
            Self::new("phone", SensitivityLevel::Internal)
                .with_pattern(MaskPattern::Partial { show_first: 0, show_last: 4 })
                .with_encryption(false)
                .with_audit(false),
            
            Self::new("address", SensitivityLevel::Confidential)
                .with_pattern(MaskPattern::Partial { show_first: 0, show_last: 5 }) // Show zip only
                .with_encryption(false)
                .with_audit(true),
            
            Self::new("street_address", SensitivityLevel::Confidential)
                .with_pattern(MaskPattern::Redacted)
                .with_encryption(false)
                .with_audit(true),
            
            Self::new("city", SensitivityLevel::Confidential)
                .with_pattern(MaskPattern::Redacted)
                .with_encryption(false)
                .with_audit(false),
            
            Self::new("state", SensitivityLevel::Internal)
                .with_pattern(MaskPattern::Partial { show_first: 2, show_last: 0 })
                .with_encryption(false)
                .with_audit(false),
            
            Self::new("zip_code", SensitivityLevel::Confidential)
                .with_pattern(MaskPattern::Partial { show_first: 3, show_last: 0 })
                .with_encryption(false)
                .with_audit(false),
            
            Self::new("postal_code", SensitivityLevel::Confidential)
                .with_pattern(MaskPattern::Partial { show_first: 3, show_last: 0 })
                .with_encryption(false)
                .with_audit(false),
            
            // Protected Health Information (ePHI)
            Self::new("diagnosis", SensitivityLevel::ProtectedHealthInfo)
                .with_pattern(MaskPattern::Redacted)
                .with_encryption(true)
                .with_audit(true),
            
            Self::new("diagnosis_code", SensitivityLevel::ProtectedHealthInfo)
                .with_pattern(MaskPattern::Redacted)
                .with_encryption(true)
                .with_audit(true),
            
            Self::new("medication", SensitivityLevel::ProtectedHealthInfo)
                .with_pattern(MaskPattern::Redacted)
                .with_encryption(true)
                .with_audit(true),
            
            Self::new("prescription", SensitivityLevel::ProtectedHealthInfo)
                .with_pattern(MaskPattern::Redacted)
                .with_encryption(true)
                .with_audit(true),
            
            Self::new("lab_result", SensitivityLevel::ProtectedHealthInfo)
                .with_pattern(MaskPattern::Redacted)
                .with_encryption(true)
                .with_audit(true),
            
            Self::new("test_result", SensitivityLevel::ProtectedHealthInfo)
                .with_pattern(MaskPattern::Redacted)
                .with_encryption(true)
                .with_audit(true),
            
            Self::new("treatment_notes", SensitivityLevel::ProtectedHealthInfo)
                .with_pattern(MaskPattern::Redacted)
                .with_encryption(true)
                .with_audit(true),
            
            Self::new("clinical_notes", SensitivityLevel::ProtectedHealthInfo)
                .with_pattern(MaskPattern::Redacted)
                .with_encryption(true)
                .with_audit(true),
            
            Self::new("procedure", SensitivityLevel::ProtectedHealthInfo)
                .with_pattern(MaskPattern::Redacted)
                .with_encryption(true)
                .with_audit(true),
            
            Self::new("procedure_code", SensitivityLevel::ProtectedHealthInfo)
                .with_pattern(MaskPattern::Redacted)
                .with_encryption(true)
                .with_audit(true),
            
            // Financial/Insurance
            Self::new("insurance_id", SensitivityLevel::Restricted)
                .with_pattern(MaskPattern::Partial { show_first: 0, show_last: 4 })
                .with_encryption(true)
                .with_audit(true),
            
            Self::new("credit_card", SensitivityLevel::Restricted)
                .with_pattern(MaskPattern::Partial { show_first: 0, show_last: 4 })
                .with_encryption(true)
                .with_audit(true),
            
            Self::new("bank_account", SensitivityLevel::Restricted)
                .with_pattern(MaskPattern::Partial { show_first: 0, show_last: 4 })
                .with_encryption(true)
                .with_audit(true),
            
            // Authentication & Secrets
            Self::new("password", SensitivityLevel::Restricted)
                .with_pattern(MaskPattern::Full)
                .with_encryption(true)
                .with_audit(true),
            
            Self::new("password_hash", SensitivityLevel::Restricted)
                .with_pattern(MaskPattern::Full)
                .with_encryption(false) // Already hashed
                .with_audit(true),
            
            Self::new("api_key", SensitivityLevel::Restricted)
                .with_pattern(MaskPattern::Partial { show_first: 8, show_last: 0 })
                .with_encryption(true)
                .with_audit(true),
            
            Self::new("access_token", SensitivityLevel::Restricted)
                .with_pattern(MaskPattern::Partial { show_first: 10, show_last: 0 })
                .with_encryption(false)
                .with_audit(true),
            
            Self::new("refresh_token", SensitivityLevel::Restricted)
                .with_pattern(MaskPattern::Partial { show_first: 10, show_last: 0 })
                .with_encryption(false)
                .with_audit(true),
            
            Self::new("secret", SensitivityLevel::Restricted)
                .with_pattern(MaskPattern::Full)
                .with_encryption(true)
                .with_audit(true),
            
            Self::new("private_key", SensitivityLevel::Restricted)
                .with_pattern(MaskPattern::Full)
                .with_encryption(true)
                .with_audit(true),
            
            Self::new("certificate", SensitivityLevel::Restricted)
                .with_pattern(MaskPattern::Hashed)
                .with_encryption(false)
                .with_audit(true),
        ]
    }
}

/// Masking engine for field-level data protection
pub struct MaskingEngine {
    field_configs: HashMap<String, SensitiveField>,
}

impl MaskingEngine {
    pub fn new() -> Self {
        let mut field_configs = HashMap::new();
        for field in SensitiveField::healthcare_fields() {
            field_configs.insert(field.name.clone(), field);
        }
        
        Self { field_configs }
    }
    
    /// Mask a single value based on field configuration
    pub fn mask_value(&self, field_name: &str, value: &str) -> String {
        if let Some(config) = self.field_configs.get(field_name) {
            self.apply_mask(value, &config.mask_pattern)
        } else {
            value.to_string() // No masking for unknown fields
        }
    }
    
    /// Apply mask pattern to value
    fn apply_mask(&self, value: &str, pattern: &MaskPattern) -> String {
        match pattern {
            MaskPattern::Partial { show_first, show_last } => {
                let len = value.chars().count();
                if len <= show_first + show_last {
                    return "*".repeat(len);
                }
                
                let chars: Vec<char> = value.chars().collect();
                let first: String = chars.iter().take(*show_first).collect();
                let last: String = chars.iter().skip(len - show_last).collect();
                let masked_middle = "*".repeat(len - show_first - show_last);
                
                format!("{}{}{}", first, masked_middle, last)
            }
            
            MaskPattern::Full => "*".repeat(value.len().min(10)),
            
            MaskPattern::Redacted => "[REDACTED]".to_string(),
            
            MaskPattern::Hashed => {
                use sha2::{Sha256, Digest};
                let mut hasher = Sha256::new();
                hasher.update(value.as_bytes());
                let hash = hasher.finalize();
                format!("sha256:{:x}", hash)
            }
            
            MaskPattern::Tokenized => {
                // Generate deterministic token (same input = same token)
                use sha2::{Sha256, Digest};
                let mut hasher = Sha256::new();
                hasher.update(value.as_bytes());
                let hash = hasher.finalize();
                format!("TOK_{}", hex::encode(&hash[0..8]))
            }
            
            MaskPattern::Custom(_regex) => {
                // TODO: Implement regex-based masking
                value.to_string()
            }
        }
    }
    
    /// Mask a JSON object (for API responses)
    pub fn mask_json(&self, mut json: serde_json::Value) -> serde_json::Value {
        if let Some(obj) = json.as_object_mut() {
            for (key, value) in obj.iter_mut() {
                if let Some(_config) = self.field_configs.get(key.as_str()) {
                    if let Some(str_value) = value.as_str() {
                        *value = serde_json::Value::String(
                            self.mask_value(key, str_value)
                        );
                    }
                }
            }
        }
        json
    }
    
    /// Check if user has permission to see unmasked value
    pub fn can_view_unmasked(
        &self,
        field_name: &str,
        user_permissions: &[String],
    ) -> bool {
        if let Some(config) = self.field_configs.get(field_name) {
            let required_perm = format!("phi:view:{}", config.level.as_str());
            
            user_permissions.iter().any(|p| {
                p == &required_perm ||
                p == "phi:view:unmasked" ||
                p == "admin:*" ||
                (p.ends_with(":*") && required_perm.starts_with(&p[..p.len()-1]))
            })
        } else {
            true // No restriction for unknown fields
        }
    }
    
    /// Get sensitivity level for a field
    pub fn get_sensitivity_level(&self, field_name: &str) -> Option<SensitivityLevel> {
        self.field_configs.get(field_name).map(|c| c.level)
    }
    
    /// Check if field requires encryption
    pub fn requires_encryption(&self, field_name: &str) -> bool {
        self.field_configs.get(field_name)
            .map(|c| c.encryption_required)
            .unwrap_or(false)
    }
    
    /// Check if field access should be audited
    pub fn requires_audit(&self, field_name: &str) -> bool {
        self.field_configs.get(field_name)
            .map(|c| c.audit_access)
            .unwrap_or(false)
    }
}

impl Default for MaskingEngine {
    fn default() -> Self {
        Self::new()
    }
}

/// Masking middleware for API responses
pub struct MaskingMiddleware {
    engine: MaskingEngine,
}

impl MaskingMiddleware {
    pub fn new() -> Self {
        Self {
            engine: MaskingEngine::new(),
        }
    }
    
    /// Mask response data based on user permissions
    pub fn mask_response(
        &self,
        data: serde_json::Value,
        user_permissions: &[String],
    ) -> serde_json::Value {
        let mut masked = data.clone();
        
        if let Some(obj) = masked.as_object_mut() {
            for (key, value) in obj.iter_mut() {
                if !self.engine.can_view_unmasked(key, user_permissions) {
                    // Mask the value
                    if let Some(str_value) = value.as_str() {
                        *value = serde_json::Value::String(
                            self.engine.mask_value(key, str_value)
                        );
                    }
                }
            }
        }
        
        masked
    }
}

impl Default for MaskingMiddleware {
    fn default() -> Self {
        Self::new()
    }
}

// =============================================================================
// FIELD-LEVEL ENCRYPTION
// =============================================================================

/// Encryption configuration for database fields
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EncryptionConfig {
    pub enabled: bool,
    pub field_mappings: HashMap<String, FieldEncryptionConfig>,
    pub master_key: Vec<u8>, // 32 bytes for AES-256
    pub key_version: u32,
}

impl Default for EncryptionConfig {
    fn default() -> Self {
        Self {
            enabled: false,
            field_mappings: HashMap::new(),
            master_key: vec![0u8; 32], // Default (insecure) key
            key_version: 1,
        }
    }
}

impl EncryptionConfig {
    /// Create from base64-encoded master key
    pub fn from_master_key(master_key_b64: &str) -> Result<Self, EncryptionError> {
        let key_bytes = BASE64.decode(master_key_b64)
            .map_err(|_| EncryptionError::InvalidKey)?;
        
        if key_bytes.len() != 32 {
            return Err(EncryptionError::InvalidKeyLength);
        }
        
        Ok(Self {
            enabled: true,
            field_mappings: Self::default_field_mappings(),
            master_key: key_bytes,
            key_version: 1,
        })
    }
    
    /// Default field mappings for sensitive healthcare data
    pub fn default_field_mappings() -> HashMap<String, FieldEncryptionConfig> {
        let mut mappings = HashMap::new();
        
        // User credentials
        mappings.insert("users.ssn".to_string(), FieldEncryptionConfig {
            table: "users".to_string(),
            column: "ssn".to_string(),
            algorithm: EncryptionAlgorithm::AES256GCM,
            searchable: false,
            key_version: 1,
        });
        
        // Tokens
        mappings.insert("tokens.access_token".to_string(), FieldEncryptionConfig {
            table: "tokens".to_string(),
            column: "access_token".to_string(),
            algorithm: EncryptionAlgorithm::AES256GCM,
            searchable: false,
            key_version: 1,
        });
        
        mappings.insert("tokens.refresh_token".to_string(), FieldEncryptionConfig {
            table: "tokens".to_string(),
            column: "refresh_token".to_string(),
            algorithm: EncryptionAlgorithm::AES256GCM,
            searchable: false,
            key_version: 1,
        });
        
        // Credentials
        mappings.insert("credentials.mfa_secret".to_string(), FieldEncryptionConfig {
            table: "credentials".to_string(),
            column: "mfa_secret".to_string(),
            algorithm: EncryptionAlgorithm::AES256GCM,
            searchable: false,
            key_version: 1,
        });
        
        mappings.insert("credentials.mfa_backup_codes".to_string(), FieldEncryptionConfig {
            table: "credentials".to_string(),
            column: "mfa_backup_codes".to_string(),
            algorithm: EncryptionAlgorithm::AES256GCM,
            searchable: false,
            key_version: 1,
        });
        
        // JWT keys
        mappings.insert("jwt_keys.private_key_pem".to_string(), FieldEncryptionConfig {
            table: "jwt_keys".to_string(),
            column: "private_key_pem".to_string(),
            algorithm: EncryptionAlgorithm::AES256GCM,
            searchable: false,
            key_version: 1,
        });
        
        // Certificates
        mappings.insert("certificates.private_key_pem".to_string(), FieldEncryptionConfig {
            table: "certificates".to_string(),
            column: "private_key_pem".to_string(),
            algorithm: EncryptionAlgorithm::AES256GCM,
            searchable: false,
            key_version: 1,
        });
        
        mappings
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FieldEncryptionConfig {
    pub table: String,
    pub column: String,
    pub algorithm: EncryptionAlgorithm,
    pub searchable: bool, // Future: deterministic encryption for searchable fields
    pub key_version: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum EncryptionAlgorithm {
    AES256GCM, // AES-256 in GCM mode (recommended)
}

/// Database encryption service
pub struct DatabaseEncryption {
    config: EncryptionConfig,
    cipher: Aes256Gcm,
}

impl DatabaseEncryption {
    pub fn new(config: EncryptionConfig) -> Result<Self, EncryptionError> {
        let cipher = Aes256Gcm::new_from_slice(&config.master_key)
            .map_err(|_| EncryptionError::InvalidKey)?;
        
        Ok(Self { config, cipher })
    }

    pub fn is_enabled(&self) -> bool {
        self.config.enabled
    }
    
    /// Encrypt a value with the current key version
    /// Returns: "v{version}:{base64_nonce}:{base64_ciphertext}"
    pub fn encrypt_value(&self, plaintext: &str) -> Result<String, EncryptionError> {
        if !self.config.enabled {
            return Ok(plaintext.to_string());
        }
        
        // Generate random 96-bit nonce (12 bytes for GCM)
        let nonce_bytes: [u8; 12] = rand::random();
        let nonce = Nonce::from_slice(&nonce_bytes);
        
        // Encrypt
        let ciphertext = self.cipher
            .encrypt(nonce, plaintext.as_bytes())
            .map_err(|_| EncryptionError::EncryptionFailed)?;
        
        // Format: v{version}:{nonce_b64}:{ciphertext_b64}
        let nonce_b64 = BASE64.encode(nonce_bytes);
        let ciphertext_b64 = BASE64.encode(&ciphertext);
        
        Ok(format!("v{}:{}:{}", self.config.key_version, nonce_b64, ciphertext_b64))
    }
    
    /// Decrypt a value (supports versioned keys)
    /// Expects format: "v{version}:{base64_nonce}:{base64_ciphertext}"
    pub fn decrypt_value(&self, encrypted: &str) -> Result<String, EncryptionError> {
        if !self.config.enabled || !encrypted.starts_with("v") {
            return Ok(encrypted.to_string());
        }
        
        // Parse format: v{version}:{nonce}:{ciphertext}
        let parts: Vec<&str> = encrypted.split(':').collect();
        if parts.len() != 3 {
            return Err(EncryptionError::InvalidFormat);
        }
        
        let version = parts[0]
            .strip_prefix('v')
            .and_then(|v| v.parse::<u32>().ok())
            .ok_or(EncryptionError::InvalidFormat)?;
        
        // For now, only support current version
        // TODO: Add key rotation support with EncryptionKeyStore
        if version != self.config.key_version {
            return Err(EncryptionError::UnsupportedKeyVersion(version));
        }
        
        let nonce_bytes = BASE64.decode(parts[1])
            .map_err(|_| EncryptionError::InvalidFormat)?;
        let ciphertext = BASE64.decode(parts[2])
            .map_err(|_| EncryptionError::InvalidFormat)?;
        
        if nonce_bytes.len() != 12 {
            return Err(EncryptionError::InvalidNonce);
        }
        
        let nonce = Nonce::from_slice(&nonce_bytes);
        
        // Decrypt
        let plaintext = self.cipher
            .decrypt(nonce, ciphertext.as_ref())
            .map_err(|_| EncryptionError::DecryptionFailed)?;
        
        String::from_utf8(plaintext)
            .map_err(|_| EncryptionError::InvalidUtf8)
    }
    
    /// Check if a field should be encrypted
    pub fn should_encrypt(&self, table: &str, column: &str) -> bool {
        if !self.config.enabled {
            return false;
        }
        
        let key = format!("{}.{}", table, column);
        self.config.field_mappings.contains_key(&key)
    }
    
    /// Get encryption config for a field
    pub fn get_field_config(&self, table: &str, column: &str) -> Option<&FieldEncryptionConfig> {
        let key = format!("{}.{}", table, column);
        self.config.field_mappings.get(&key)
    }
}

/// Encryption errors
#[derive(Debug, thiserror::Error)]
pub enum EncryptionError {
    #[error("Encryption is not enabled")]
    NotEnabled,
    
    #[error("Invalid encryption key")]
    InvalidKey,
    
    #[error("Invalid key length (expected 32 bytes for AES-256)")]
    InvalidKeyLength,
    
    #[error("Unsupported key version: {0}")]
    UnsupportedKeyVersion(u32),
    
    #[error("Invalid encrypted data format")]
    InvalidFormat,
    
    #[error("Invalid nonce length")]
    InvalidNonce,
    
    #[error("Encryption failed")]
    EncryptionFailed,
    
    #[error("Decryption failed")]
    DecryptionFailed,
    
    #[error("Invalid UTF-8 in decrypted data")]
    InvalidUtf8,
}

// =============================================================================
// KEY ROTATION SUPPORT (FUTURE)
// =============================================================================

/// Manages multiple encryption keys for rotation
pub struct EncryptionKeyStore {
    keys: HashMap<u32, Vec<u8>>, // version -> key
    current_version: u32,
}

impl EncryptionKeyStore {
    pub fn new() -> Self {
        Self {
            keys: HashMap::new(),
            current_version: 1,
        }
    }
    
    /// Add a key version
    pub fn add_key(&mut self, version: u32, key: Vec<u8>) -> Result<(), EncryptionError> {
        if key.len() != 32 {
            return Err(EncryptionError::InvalidKeyLength);
        }
        self.keys.insert(version, key);
        Ok(())
    }
    
    /// Get the current encryption key
    pub fn get_current_key(&self) -> Option<&Vec<u8>> {
        self.keys.get(&self.current_version)
    }
    
    /// Get a key by version (for decryption)
    pub fn get_key_by_version(&self, version: u32) -> Option<&Vec<u8>> {
        self.keys.get(&version)
    }
    
    /// Rotate to a new key version
    pub fn rotate(&mut self, new_version: u32) {
        self.current_version = new_version;
    }
    
    /// Get current version
    pub fn current_version(&self) -> u32 {
        self.current_version
    }
}