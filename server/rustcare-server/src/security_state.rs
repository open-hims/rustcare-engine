//! Security application state
//!
//! Centralizes all security components including:
//! - Encryption services
//! - Key management (KMS)
//! - Database with TDE
//! - Secure storage backends
//! - Authorization (Zanzibar)

use crypto::{SecurityConfig, KmsProvider as KmsProviderType};
use std::sync::Arc;
use anyhow::Result;

/// Type-erased KMS provider
pub type BoxedKmsProvider = Arc<dyn std::any::Any + Send + Sync>;

/// Application security state
#[derive(Clone)]
pub struct SecurityState {
    /// Security configuration
    pub config: Arc<SecurityConfig>,
    
    /// KMS provider (if enabled, type-erased for feature compatibility)
    pub kms_provider: Option<BoxedKmsProvider>,
    
    // Future: Database pool with TDE
    // pub database: Arc<Database>,
    
    // Future: Object storage with encryption
    // pub storage: Arc<dyn StorageBackend>,
    
    // Future: Zanzibar authorization
    // pub authz: Arc<AuthorizationEngine>,
}

impl SecurityState {
    /// Initialize security state from environment configuration
    pub async fn from_env() -> Result<Self> {
        let config = SecurityConfig::from_env()?;
        let config = Arc::new(config);
        
        // Initialize KMS provider based on configuration
        let kms_provider = Self::initialize_kms(&config).await?;
        
        Ok(Self {
            config,
            kms_provider,
        })
    }
    
    /// Initialize KMS provider based on configuration
    async fn initialize_kms(
        config: &SecurityConfig,
    ) -> Result<Option<BoxedKmsProvider>> {
        match config.kms_provider {
            KmsProviderType::None => {
                println!("⚠️  KMS disabled - using direct master key encryption");
                Ok(None)
            }
            
            #[cfg(feature = "aws-kms")]
            KmsProviderType::AwsKms => {
                use crypto::kms::AwsKmsProvider;
                
                println!("🔐 Initializing AWS KMS provider...");
                let kms_config = config.aws_kms_config.as_ref()
                    .ok_or_else(|| anyhow::anyhow!("AWS KMS config missing"))?;
                
                let provider = AwsKmsProvider::new(
                    kms_config.key_id.clone(),
                    kms_config.region.clone(),
                ).await?;
                
                println!("✅ AWS KMS provider initialized");
                Ok(Some(Arc::new(provider)))
            }
            
            #[cfg(not(feature = "aws-kms"))]
            KmsProviderType::AwsKms => {
                Err(anyhow::anyhow!(
                    "AWS KMS requested but 'aws-kms' feature not enabled. \
                     Rebuild with: cargo build --features aws-kms"
                ))
            }
            
            #[cfg(feature = "vault-kms")]
            KmsProviderType::Vault => {
                use crypto::kms::VaultKmsProvider;
                
                println!("🔐 Initializing HashiCorp Vault provider...");
                let vault_config = config.vault_config.as_ref()
                    .ok_or_else(|| anyhow::anyhow!("Vault config missing"))?;
                
                let provider = VaultKmsProvider::new(
                    vault_config.addr.clone(),
                    vault_config.token.clone(),
                    vault_config.mount_path.clone(),
                    vault_config.key_name.clone(),
                ).await?;
                
                println!("✅ Vault provider initialized");
                Ok(Some(Arc::new(provider)))
            }
            
            #[cfg(not(feature = "vault-kms"))]
            KmsProviderType::Vault => {
                Err(anyhow::anyhow!(
                    "Vault KMS requested but 'vault-kms' feature not enabled. \
                     Rebuild with: cargo build --features vault-kms"
                ))
            }
            
            KmsProviderType::Local => {
                println!("⚠️  Using local key storage (DEVELOPMENT ONLY)");
                println!("⚠️  DO NOT use in production!");
                // TODO: Implement LocalKmsProvider
                Ok(None)
            }
        }
    }
    
    /// Print security configuration summary
    pub fn print_summary(&self) {
        println!("\n╔════════════════════════════════════════════════════════════╗");
        println!("║           RUSTCARE SECURITY CONFIGURATION                  ║");
        println!("╚════════════════════════════════════════════════════════════╝");
        
        println!("\n📊 Encryption:");
        println!("  • Algorithm: {:?}", self.config.encryption_algorithm);
        println!("  • Key Version: {}", self.config.key_version);
        println!("  • Envelope Encryption: {}", 
                 if self.config.enable_envelope_encryption { "✅ Enabled" } else { "❌ Disabled" });
        println!("  • Envelope Threshold: {} bytes ({:.2} MB)", 
                 self.config.envelope_threshold,
                 self.config.envelope_threshold as f64 / 1_048_576.0);
        
        println!("\n🔑 Key Management:");
        println!("  • KMS Provider: {:?}", self.config.kms_provider);
        if let Some(_) = &self.kms_provider {
            println!("  • KMS Status: ✅ Connected");
        } else {
            println!("  • KMS Status: ⚠️  Using direct master key");
        }
        println!("  • Key Rotation: Every {} days", self.config.key_rotation_interval_days);
        
        println!("\n🛡️  Security Hardening:");
        println!("  • Memory Locking: {}", 
                 if self.config.enable_memory_locking { "✅ Enabled" } else { "⚠️  Disabled" });
        println!("  • Constant-Time Ops: {}", 
                 if self.config.enable_constant_time { "✅ Enabled" } else { "⚠️  Disabled" });
        println!("  • Guard Pages: {}", 
                 if self.config.enable_guard_pages { "✅ Enabled" } else { "⚠️  Disabled" });
        
        if self.config.is_fully_hardened() {
            println!("  • Overall Status: ✅ FULLY HARDENED");
        } else {
            println!("  • Overall Status: ⚠️  PARTIALLY HARDENED");
        }
        
        println!("\n💾 Storage:");
        println!("  • Backend: {}", self.config.storage_backend);
        println!("  • Encryption Threshold: {} bytes ({:.2} MB)", 
                 self.config.storage_encryption_threshold,
                 self.config.storage_encryption_threshold as f64 / 1_048_576.0);
        if self.config.storage_backend == "s3" {
            println!("  • S3 Client-Side Encryption: {}", 
                     if self.config.enable_s3_client_side_encryption { "✅ Enabled" } else { "❌ Disabled" });
        }
        
        println!("\n🗄️  Database:");
        println!("  • TDE (Transparent Data Encryption): {}", 
                 if self.config.enable_database_tde { "✅ Enabled" } else { "❌ Disabled" });
        
        println!("\n📋 Compliance:");
        println!("  • FIPS Mode: {}", 
                 if self.config.enable_fips_mode { "✅ Enabled" } else { "❌ Disabled" });
        println!("  • Crypto Audit Logging: {}", 
                 if self.config.enable_crypto_audit { "✅ Enabled" } else { "❌ Disabled" });
        if let Some(path) = &self.config.audit_log_path {
            println!("  • Audit Log: {}", path.display());
        }
        
        println!("\n⚡ Performance:");
        println!("  • DEK Caching: {}", 
                 if self.config.enable_dek_cache { "✅ Enabled" } else { "❌ Disabled" });
        if self.config.enable_dek_cache {
            println!("  • Cache TTL: {} seconds ({:.1} hours)", 
                     self.config.dek_cache_ttl_seconds,
                     self.config.dek_cache_ttl_seconds as f64 / 3600.0);
            println!("  • Cache Size: {} keys", self.config.dek_cache_max_size);
        }
        
        println!("\n╚════════════════════════════════════════════════════════════╝\n");
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_security_state_can_be_cloned() {
        // SecurityState must be Clone for use with Axum
        fn assert_clone<T: Clone>() {}
        assert_clone::<SecurityState>();
    }
}
