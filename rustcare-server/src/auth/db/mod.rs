/// Database repository layer for authentication
/// 
/// Provides typed query interfaces for all authentication tables using database-layer

pub mod user_repository;
pub mod credential_repository;
pub mod oauth_repository;
pub mod certificate_repository;
pub mod refresh_token_repository;
pub mod session_repository;
pub mod jwt_key_repository;
pub mod audit_repository;
pub mod permission_repository;
pub mod rate_limit_repository;
pub mod organization_repository;

pub use user_repository::UserRepository;
pub use credential_repository::CredentialRepository;
pub use oauth_repository::OAuthRepository;
pub use certificate_repository::CertificateRepository;
pub use refresh_token_repository::RefreshTokenRepository;
pub use session_repository::SessionRepository;
pub use jwt_key_repository::JwtKeyRepository;
pub use audit_repository::AuditRepository;
pub use permission_repository::PermissionRepository;
pub use rate_limit_repository::RateLimitRepository;
pub use organization_repository::OrganizationRepository;

// Re-export database layer components
pub use database_layer::{
    DatabasePool,
    RlsContext,
    DatabaseError,
    DatabaseResult,
    AuditLogger,
};

use sqlx::{PgPool, Error as SqlxError};
use std::sync::Arc;

/// Database connection pool wrapper (legacy compatibility)
/// Use DatabasePool from database-layer for new code
#[derive(Clone)]
pub struct DbPool {
    pool: Arc<PgPool>,
    db_pool: Option<DatabasePool>,
}

impl DbPool {
    pub fn new(pool: PgPool) -> Self {
        Self {
            pool: Arc::new(pool),
            db_pool: None,
        }
    }

    /// Create from DatabasePool (preferred)
    pub async fn from_database_pool(database_url: &str) -> Result<Self, DatabaseError> {
        let db_pool = DatabasePool::new(database_url)
            .await?
            .with_rls(true)
            .with_audit(true)
            .with_encryption(false);  // Enable when encryption is fully implemented

        let pool = db_pool.pool().clone();

        Ok(Self {
            pool: Arc::new(pool),
            db_pool: Some(db_pool),
        })
    }
    
    pub fn get(&self) -> &PgPool {
        &self.pool
    }

    /// Get the database layer pool (if available)
    pub fn database_pool(&self) -> Option<&DatabasePool> {
        self.db_pool.as_ref()
    }
}

/// Combined repository providing access to all auth tables
/// 
/// Supports RLS context for multi-tenant isolation and audit logging for HIPAA compliance
pub struct AuthRepository {
    pub users: UserRepository,
    pub credentials: CredentialRepository,
    pub oauth: OAuthRepository,
    pub certificates: CertificateRepository,
    pub refresh_tokens: RefreshTokenRepository,
    pub sessions: SessionRepository,
    pub jwt_keys: JwtKeyRepository,
    pub audit: AuditRepository,
    pub permissions: PermissionRepository,
    pub rate_limits: RateLimitRepository,
    pub organizations: OrganizationRepository,
    
    /// Optional RLS context for automatic tenant isolation
    rls_context: Option<RlsContext>,
    /// Optional audit logger for HIPAA compliance
    audit_logger: Option<Arc<AuditLogger>>,
}

impl AuthRepository {
    pub fn new(pool: PgPool) -> Self {
        let db_pool = DbPool::new(pool.clone());
        
        // Create audit logger
        let audit_logger = Arc::new(AuditLogger::new(pool.clone()));
        
        Self {
            users: UserRepository::new(db_pool.clone())
                .with_audit_logger(audit_logger.clone()),
            credentials: CredentialRepository::new(db_pool.clone())
                .with_audit_logger(audit_logger.clone()),
            oauth: OAuthRepository::new(db_pool.clone())
                .with_audit_logger(audit_logger.clone()),
            certificates: CertificateRepository::new(db_pool.clone())
                .with_audit_logger(audit_logger.clone()),
            refresh_tokens: RefreshTokenRepository::new(db_pool.clone())
                .with_audit_logger(audit_logger.clone()),
            sessions: SessionRepository::new(pool.clone())
                .with_audit_logger(audit_logger.clone()),
            jwt_keys: JwtKeyRepository::new(db_pool.clone())
                .with_audit_logger(audit_logger.clone()),
            audit: AuditRepository::new(db_pool.clone()),
            permissions: PermissionRepository::new(pool.clone())
                .with_audit_logger(audit_logger.clone()),
            rate_limits: RateLimitRepository::new(db_pool)
                .with_audit_logger(audit_logger.clone()),
            organizations: OrganizationRepository::new(Arc::new(pool))
                .with_audit_logger(audit_logger.clone()),
            rls_context: None,
            audit_logger: Some(audit_logger),
        }
    }

    /// Create from DatabasePool with RLS support
    pub async fn from_database_pool(database_url: &str) -> Result<Self, DatabaseError> {
        let db_pool = DbPool::from_database_pool(database_url).await?;
        let pool = db_pool.get().clone();

        // Create audit logger
        let audit_logger = Arc::new(AuditLogger::new(pool.clone()));

        Ok(Self {
            users: UserRepository::new(db_pool.clone())
                .with_audit_logger(audit_logger.clone()),
            credentials: CredentialRepository::new(db_pool.clone())
                .with_audit_logger(audit_logger.clone()),
            oauth: OAuthRepository::new(db_pool.clone())
                .with_audit_logger(audit_logger.clone()),
            certificates: CertificateRepository::new(db_pool.clone())
                .with_audit_logger(audit_logger.clone()),
            refresh_tokens: RefreshTokenRepository::new(db_pool.clone())
                .with_audit_logger(audit_logger.clone()),
            sessions: SessionRepository::new(pool.clone())
                .with_audit_logger(audit_logger.clone()),
            jwt_keys: JwtKeyRepository::new(db_pool.clone())
                .with_audit_logger(audit_logger.clone()),
            audit: AuditRepository::new(db_pool.clone()),
            permissions: PermissionRepository::new(pool.clone())
                .with_audit_logger(audit_logger.clone()),
            rate_limits: RateLimitRepository::new(db_pool)
                .with_audit_logger(audit_logger.clone()),
            organizations: OrganizationRepository::new(Arc::new(pool))
                .with_audit_logger(audit_logger.clone()),
            rls_context: None,
            audit_logger: Some(audit_logger),
        })
    }

    /// Set RLS context for multi-tenant operations
    pub fn with_rls_context(mut self, context: RlsContext) -> Self {
        self.rls_context = Some(context.clone());
        
        // Propagate RLS context to all repositories
        self.users = self.users.with_rls_context(context.clone());
        self.credentials = self.credentials.with_rls_context(context.clone());
        self.oauth = self.oauth.with_rls_context(context.clone());
        self.certificates = self.certificates.with_rls_context(context.clone());
        self.refresh_tokens = self.refresh_tokens.with_rls_context(context.clone());
        self.sessions = self.sessions.with_rls_context(context.clone());
        self.jwt_keys = self.jwt_keys.with_rls_context(context.clone());
        self.audit = self.audit.with_rls_context(context.clone());
        self.permissions = self.permissions.with_rls_context(context.clone());
        self.rate_limits = self.rate_limits.with_rls_context(context.clone());
        self.organizations = self.organizations.with_rls_context(context);
        
        self
    }

    /// Get the current RLS context
    pub fn rls_context(&self) -> Option<&RlsContext> {
        self.rls_context.as_ref()
    }

    /// Get the audit logger
    pub fn audit_logger(&self) -> Option<&Arc<AuditLogger>> {
        self.audit_logger.as_ref()
    }
}

/// Common database operations result type
pub type DbResult<T> = Result<T, SqlxError>;
