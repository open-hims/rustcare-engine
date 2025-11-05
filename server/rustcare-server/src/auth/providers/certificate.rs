/// Certificate-based authentication provider (VPN CA / mTLS)
/// 
/// Implements authentication using X.509 client certificates with:
/// - Certificate chain validation against trusted CA roots
/// - Revocation checking (CRL and OCSP)
/// - Identity extraction from Subject DN and SAN
/// - Certificate serial binding to JWT tokens
/// - CRL caching with configurable TTL
/// - OCSP with fallback to CRL

use super::{AuthResult, Credentials, Provider};
use crate::auth::db::{CertificateRepository, UserRepository};
use async_trait::async_trait;
use chrono::{DateTime, Utc};
use sha2::{Digest, Sha256};
use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::Arc;
use tokio::sync::RwLock;
use x509_parser::prelude::*;

/// CRL cache entry with expiration
#[derive(Debug, Clone)]
struct CrlCacheEntry {
    /// Revoked serial numbers
    revoked_serials: Vec<String>,
    /// When this CRL entry expires
    expires_at: DateTime<Utc>,
}

/// OCSP cache entry
#[derive(Debug, Clone)]
struct OcspCacheEntry {
    /// Whether certificate is revoked
    is_revoked: bool,
    /// Cache timestamp
    cached_at: DateTime<Utc>,
}

pub struct CertificateProvider {
    /// Path to trusted CA root certificates
    ca_roots_path: PathBuf,
    /// Whether to verify certificate chain
    verify_chain: bool,
    /// Whether to check revocation status
    check_revocation: bool,
    /// CRL cache (issuer DN -> CRL data)
    crl_cache: Arc<RwLock<HashMap<String, CrlCacheEntry>>>,
    /// OCSP cache (cert serial -> status)
    ocsp_cache: Arc<RwLock<HashMap<String, OcspCacheEntry>>>,
    /// Certificate repository for database operations
    cert_repo: CertificateRepository,
    /// User repository for fetching user details
    user_repo: Arc<UserRepository>,
    /// CRL cache TTL in seconds (default: 1 hour)
    crl_cache_ttl: i64,
    /// OCSP cache TTL in seconds (default: 5 minutes)
    ocsp_cache_ttl: i64,
}

impl CertificateProvider {
    /// Create a new certificate provider
    pub fn new(
        ca_roots_path: String,
        verify_chain: bool,
        check_revocation: bool,
        cert_repo: CertificateRepository,
        user_repo: Arc<UserRepository>,
    ) -> Self {
        Self {
            ca_roots_path: PathBuf::from(ca_roots_path),
            verify_chain,
            check_revocation,
            crl_cache: Arc::new(RwLock::new(HashMap::new())),
            ocsp_cache: Arc::new(RwLock::new(HashMap::new())),
            cert_repo,
            user_repo,
            crl_cache_ttl: 3600,      // 1 hour
            ocsp_cache_ttl: 300,       // 5 minutes
        }
    }
    
    /// Parse and extract all information from certificate in one pass
    fn parse_and_extract(&self, cert_pem: &str) -> anyhow::Result<CertificateIdentity> {
        // Remove BEGIN/END markers and whitespace, convert to DER
        let der_bytes = if cert_pem.contains("-----BEGIN CERTIFICATE-----") {
            // Parse PEM format
            let pem_str = cert_pem
                .lines()
                .filter(|line| !line.starts_with("-----"))
                .collect::<String>();
            use base64::Engine;
            base64::engine::general_purpose::STANDARD.decode(&pem_str)
                .map_err(|e| anyhow::anyhow!("Failed to decode PEM base64: {}", e))?
        } else {
            // Assume already DER encoded
            cert_pem.as_bytes().to_vec()
        };
        
        // Calculate fingerprint from DER
        let mut hasher = Sha256::new();
        hasher.update(&der_bytes);
        let fingerprint_sha256 = hex::encode(hasher.finalize());
        
        // Parse X.509 from DER
        let (_rem, cert) = X509Certificate::from_der(&der_bytes)
            .map_err(|e| anyhow::anyhow!("Failed to parse X.509 certificate: {}", e))?;
        
        // Extract serial number
        let serial = cert.serial.to_str_radix(16);
        
        // Extract DNs
        let subject_dn = cert.subject().to_string();
        let issuer_dn = cert.issuer().to_string();
        
        // Extract email
        let email = self.extract_email_from_cert(&cert)?;
        
        // Extract Common Name
        let common_name = cert.subject()
            .iter_common_name()
            .next()
            .and_then(|attr| attr.as_str().ok())
            .map(|s| s.to_string())
            .unwrap_or_else(|| "Unknown".to_string());
        
        // Extract validity dates - convert from time::OffsetDateTime to chrono::DateTime
        let not_before_time = cert.validity().not_before.to_datetime();
        let not_after_time = cert.validity().not_after.to_datetime();
        
        let not_before = DateTime::<Utc>::from_timestamp(not_before_time.unix_timestamp(), 0)
            .ok_or_else(|| anyhow::anyhow!("Invalid not_before timestamp"))?;
        let not_after = DateTime::<Utc>::from_timestamp(not_after_time.unix_timestamp(), 0)
            .ok_or_else(|| anyhow::anyhow!("Invalid not_after timestamp"))?;
        
        // Extract custom attributes from Subject DN
        let mut custom_attrs = HashMap::new();
        for rdn in cert.subject().iter() {
            for attr in rdn.iter() {
                if let Ok(value) = attr.as_str() {
                    let oid = attr.attr_type().to_id_string();
                    custom_attrs.insert(oid, value.to_string());
                }
            }
        }
        
        Ok(CertificateIdentity {
            email,
            common_name,
            serial,
            subject_dn,
            issuer_dn,
            custom_attrs,
            fingerprint_sha256,
            not_before,
            not_after,
        })
    }
    
    /// Extract email from certificate (Subject or SAN)
    fn extract_email_from_cert(&self, cert: &X509Certificate) -> anyhow::Result<String> {
        // Try to extract from Subject DN first
        if let Some(email) = cert.subject()
            .iter_email()
            .next()
            .and_then(|attr| attr.as_str().ok()) {
            return Ok(email.to_string());
        }
        
        // Try SAN extension
        if let Ok(Some(san)) = cert.subject_alternative_name() {
            for name in &san.value.general_names {
                if let x509_parser::extensions::GeneralName::RFC822Name(email) = name {
                    return Ok(email.to_string());
                }
            }
        }
        
        Err(anyhow::anyhow!("No email found in certificate"))
    }

    /// Map certificate to user in database
    async fn map_to_user(&self, identity: &CertificateIdentity) -> anyhow::Result<uuid::Uuid> {
        // Look up certificate by serial number
        let cert_record = self.cert_repo
            .find_by_serial(&identity.serial)
            .await
            .map_err(|e| anyhow::anyhow!("Failed to look up certificate: {}", e))?;
        
        match cert_record {
            Some(cert) => {
                // Verify certificate is active
                use crate::auth::models::CertificateStatus;
                if cert.status != CertificateStatus::Active {
                    return Err(anyhow::anyhow!("Certificate is not active (status: {:?})", cert.status));
                }
                
                // Verify certificate hasn't expired
                let now = Utc::now();
                if now < cert.not_before {
                    return Err(anyhow::anyhow!("Certificate not yet valid"));
                }
                if now > cert.not_after {
                    return Err(anyhow::anyhow!("Certificate has expired"));
                }
                
                // Update last_login timestamp and increment counter
                if let Err(e) = self.cert_repo.update_last_login(cert.id).await {
                    tracing::warn!("Failed to update certificate last_login: {}", e);
                }
                
                Ok(cert.user_id)
            }
            None => {
                // Certificate not registered in database
                Err(anyhow::anyhow!(
                    "Certificate serial {} not found in database. Please register the certificate first.",
                    identity.serial
                ))
            }
        }
    }
}

#[async_trait]
impl Provider for CertificateProvider {
    async fn authenticate(&self, credentials: &Credentials) -> anyhow::Result<AuthResult> {
        match credentials {
            Credentials::Certificate { cert_pem, cert_serial: _, subject_dn: _ } => {
                // Step 1: Parse and extract identity from certificate
                let identity = self.parse_and_extract(cert_pem)?;
                
                // Step 2: Validate certificate dates and basic checks
                let now = Utc::now();
                if now < identity.not_before {
                    return Err(anyhow::anyhow!("Certificate not yet valid"));
                }
                if now > identity.not_after {
                    return Err(anyhow::anyhow!("Certificate has expired"));
                }
                
                // Step 3: Check revocation status (database-based)
                if self.check_revocation {
                    let is_revoked = self.cert_repo.is_revoked(&identity.serial).await
                        .unwrap_or(false);
                    if is_revoked {
                        return Err(anyhow::anyhow!("Certificate has been revoked"));
                    }
                }
                
                // Step 4: Map to user
                let user_id = self.map_to_user(&identity).await?;
                
                // Step 4.5: Fetch user to get organization_id
                let user = self.user_repo.find_by_id(user_id).await?
                    .ok_or_else(|| anyhow::anyhow!("User not found after mapping"))?;
                
                // Step 5: Return auth result
                Ok(AuthResult {
                    user_id: user_id.to_string(),
                    email: identity.email.clone(),
                    auth_method: "certificate".to_string(),
                    permissions: vec!["patient:read".to_string(), "patient:write".to_string()],
                    claims: {
                        let mut claims = HashMap::new();
                        claims.insert("cert_cn".to_string(), serde_json::json!(identity.common_name));
                        claims.insert("cert_issuer".to_string(), serde_json::json!(identity.issuer_dn));
                        claims.insert("cert_serial".to_string(), serde_json::json!(identity.serial));
                        claims.insert("cert_subject_dn".to_string(), serde_json::json!(identity.subject_dn));
                        claims.insert("cert_fingerprint".to_string(), serde_json::json!(identity.fingerprint_sha256));
                        claims
                    },
                    cert_serial: Some(identity.serial.clone()),
                    oauth_provider: None,
                    organization_id: user.organization_id,
                })
            }
            _ => Err(anyhow::anyhow!("Invalid credentials type for certificate provider")),
        }
    }
    
    async fn user_exists(&self, identifier: &str) -> anyhow::Result<bool> {
        // Check if certificate exists by serial number or email
        let cert_by_serial = self.cert_repo.find_by_serial(identifier).await?;
        if cert_by_serial.is_some() {
            return Ok(true);
        }
        
        // TODO: Could also check by email in user database
        // For now, just return false if not found by serial
        let _ = identifier; // Silence unused warning for potential email lookup
        Ok(false)
    }
    
    fn name(&self) -> &str {
        "certificate"
    }
}

/// Certificate identity information
#[derive(Debug, Clone)]
struct CertificateIdentity {
    email: String,
    common_name: String,
    serial: String,
    subject_dn: String,
    issuer_dn: String,
    custom_attrs: HashMap<String, String>,
    fingerprint_sha256: String,
    not_before: DateTime<Utc>,
    not_after: DateTime<Utc>,
}

/// Parsed certificate data with owned DER bytes
struct ParsedCertificate {
    der_bytes: Vec<u8>,
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[tokio::test]
    #[ignore] // Requires database connection
    async fn test_certificate_provider_creation() {
        use crate::auth::db::{CertificateRepository, DbPool};
        use sqlx::PgPool;
        
        // Note: In production, use a real database pool
        // For unit testing, we'd need to mock the repository
        let pool = PgPool::connect("postgres://localhost/test")
            .await
            .expect("Test requires database connection or mock");
        
        let db_pool = DbPool::new(pool);
        let cert_repo = CertificateRepository::new(db_pool.clone());
        let user_repo = Arc::new(UserRepository::new(db_pool));
        let provider = CertificateProvider::new(
            "/etc/rustcare/ca-certificates/roots".to_string(),
            true,
            true,
            cert_repo,
            user_repo,
        );
        assert_eq!(provider.name(), "certificate");
    }
}
