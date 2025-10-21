/// Authentication module for RustCare Engine
/// 
/// Supports multiple authentication methods:
/// - Email/Password with Argon2id hashing
/// - OAuth2/OIDC SSO (Google, Azure AD, Okta)
/// - VPN CA Certificate authentication (mTLS)
/// - WebAuthn/FIDO2 (future)
/// 
/// Security features:
/// - Short-lived JWT access tokens (5 minutes)
/// - Long-lived refresh tokens with rotation (30 days)
/// - Server-side session management (Redis)
/// - Certificate binding and revocation checks
/// - Step-up authentication for sensitive operations
/// - Anomaly detection and rate limiting

pub mod config;
pub mod providers;
pub mod tokens;
pub mod session;
pub mod middleware;
pub mod certificate;
pub mod models;
pub mod db;

pub use config::{AuthConfig, AuthProvider, TokenConfig, SessionConfig};
pub use providers::{EmailPasswordProvider, OAuthProvider, CertificateProvider};
pub use tokens::{JwtService, RefreshTokenService, TokenClaims};
pub use session::{SessionManager, SessionData};
pub use middleware::{AuthService, AuthContext, RequirePermission, RequireRole, RequireAnyPermission, RequireAllPermissions, AuthError};
pub use models::*;
pub use db::AuthRepository;
