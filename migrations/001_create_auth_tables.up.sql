-- Authentication System Database Schema
-- PostgreSQL Migration: Create Users and Authentication Tables
-- Version: 001
-- Description: Core authentication tables for email, OAuth, and certificate auth

-- =============================================================================
-- USERS TABLE
-- =============================================================================
-- Core user identity table - central to all authentication methods
CREATE TABLE IF NOT EXISTS users (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    
    -- User identification
    email VARCHAR(255) UNIQUE NOT NULL,
    email_verified BOOLEAN NOT NULL DEFAULT FALSE,
    email_verified_at TIMESTAMPTZ,
    
    -- User profile
    full_name VARCHAR(255),
    display_name VARCHAR(100),
    avatar_url TEXT,
    
    -- User status
    status VARCHAR(50) NOT NULL DEFAULT 'active' CHECK (status IN ('active', 'inactive', 'suspended', 'locked', 'pending_verification')),
    
    -- Account metadata
    locale VARCHAR(10) DEFAULT 'en-US',
    timezone VARCHAR(100) DEFAULT 'UTC',
    
    -- Security tracking
    last_login_at TIMESTAMPTZ,
    last_login_ip INET,
    last_login_method VARCHAR(50), -- 'email_password', 'oauth', 'certificate'
    failed_login_attempts INTEGER NOT NULL DEFAULT 0,
    locked_until TIMESTAMPTZ,
    
    -- Audit timestamps
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    deleted_at TIMESTAMPTZ, -- Soft delete for HIPAA compliance
    
    -- Indexes
    CONSTRAINT email_lowercase CHECK (email = LOWER(email))
);

-- Indexes for users table
CREATE INDEX idx_users_email ON users(email) WHERE deleted_at IS NULL;
CREATE INDEX idx_users_status ON users(status) WHERE deleted_at IS NULL;
CREATE INDEX idx_users_last_login ON users(last_login_at DESC) WHERE deleted_at IS NULL;

-- =============================================================================
-- USER CREDENTIALS TABLE (Email/Password Authentication)
-- =============================================================================
CREATE TABLE IF NOT EXISTS user_credentials (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    user_id UUID NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    
    -- Password authentication
    password_hash TEXT NOT NULL, -- Argon2id hash
    password_algorithm VARCHAR(50) NOT NULL DEFAULT 'argon2id',
    password_changed_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    password_expires_at TIMESTAMPTZ, -- Optional password expiration
    
    -- Multi-factor authentication
    mfa_enabled BOOLEAN NOT NULL DEFAULT FALSE,
    mfa_secret TEXT, -- TOTP secret (encrypted)
    mfa_backup_codes TEXT[], -- Encrypted backup codes
    mfa_enabled_at TIMESTAMPTZ,
    
    -- Security questions (optional)
    security_questions JSONB,
    
    -- Metadata
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    
    -- Constraints
    CONSTRAINT one_credential_per_user UNIQUE (user_id)
);

CREATE INDEX idx_user_credentials_user_id ON user_credentials(user_id);

-- =============================================================================
-- OAUTH ACCOUNTS TABLE (OAuth/OIDC SSO)
-- =============================================================================
CREATE TABLE IF NOT EXISTS oauth_accounts (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    user_id UUID NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    
    -- OAuth provider information
    provider VARCHAR(100) NOT NULL, -- 'google', 'azure', 'okta', 'github', etc.
    provider_account_id VARCHAR(255) NOT NULL, -- OAuth 'sub' claim
    provider_email VARCHAR(255),
    
    -- OAuth tokens (encrypted at application level)
    access_token TEXT,
    refresh_token TEXT,
    id_token TEXT,
    token_expires_at TIMESTAMPTZ,
    
    -- Provider-specific data
    provider_data JSONB, -- Store additional claims
    scopes TEXT[], -- Granted OAuth scopes
    
    -- Metadata
    first_login_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    last_login_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    
    -- Constraints
    CONSTRAINT unique_provider_account UNIQUE (provider, provider_account_id)
);

CREATE INDEX idx_oauth_accounts_user_id ON oauth_accounts(user_id);
CREATE INDEX idx_oauth_accounts_provider ON oauth_accounts(provider);
CREATE INDEX idx_oauth_accounts_provider_email ON oauth_accounts(provider_email);

-- =============================================================================
-- CLIENT CERTIFICATES TABLE (VPN CA / mTLS Authentication)
-- =============================================================================
CREATE TABLE IF NOT EXISTS client_certificates (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    user_id UUID NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    
    -- Certificate identification
    serial_number VARCHAR(100) NOT NULL UNIQUE, -- Hex serial number
    fingerprint_sha256 VARCHAR(64) NOT NULL, -- SHA-256 fingerprint
    
    -- Certificate details
    subject_dn TEXT NOT NULL, -- Distinguished Name
    issuer_dn TEXT NOT NULL,
    common_name VARCHAR(255),
    email_address VARCHAR(255),
    organization VARCHAR(255),
    organizational_unit VARCHAR(255),
    
    -- Certificate validity
    not_before TIMESTAMPTZ NOT NULL,
    not_after TIMESTAMPTZ NOT NULL,
    
    -- Certificate status
    status VARCHAR(50) NOT NULL DEFAULT 'active' CHECK (status IN ('active', 'revoked', 'expired', 'suspended')),
    revoked_at TIMESTAMPTZ,
    revocation_reason VARCHAR(100),
    
    -- Certificate data
    certificate_pem TEXT NOT NULL, -- Full PEM-encoded certificate
    public_key_pem TEXT,
    
    -- Usage tracking
    first_login_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    last_login_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    login_count INTEGER NOT NULL DEFAULT 0,
    
    -- Metadata
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    
    -- Constraints
    CONSTRAINT valid_dates CHECK (not_before < not_after)
);

CREATE INDEX idx_client_certs_user_id ON client_certificates(user_id);
CREATE INDEX idx_client_certs_serial ON client_certificates(serial_number);
CREATE INDEX idx_client_certs_fingerprint ON client_certificates(fingerprint_sha256);
CREATE INDEX idx_client_certs_status ON client_certificates(status);
CREATE INDEX idx_client_certs_expiry ON client_certificates(not_after);

-- =============================================================================
-- REFRESH TOKENS TABLE
-- =============================================================================
CREATE TABLE IF NOT EXISTS refresh_tokens (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    user_id UUID NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    
    -- Token identification
    token_hash VARCHAR(64) NOT NULL UNIQUE, -- SHA-256 hash of token
    token_family UUID NOT NULL, -- For token rotation tracking
    
    -- Token metadata
    device_name VARCHAR(255),
    device_fingerprint VARCHAR(255),
    user_agent TEXT,
    ip_address INET,
    
    -- Token validity
    issued_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    expires_at TIMESTAMPTZ NOT NULL,
    last_used_at TIMESTAMPTZ,
    
    -- Token status
    revoked BOOLEAN NOT NULL DEFAULT FALSE,
    revoked_at TIMESTAMPTZ,
    revocation_reason VARCHAR(100),
    
    -- Related authentication
    auth_method VARCHAR(50), -- 'email_password', 'oauth', 'certificate'
    cert_serial VARCHAR(100), -- If certificate auth
    
    -- Rotation tracking
    parent_token_id UUID REFERENCES refresh_tokens(id),
    replaced_by_token_id UUID REFERENCES refresh_tokens(id),
    
    -- Metadata
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    
    -- Constraints
    CONSTRAINT valid_expiry CHECK (expires_at > issued_at)
);

CREATE INDEX idx_refresh_tokens_user_id ON refresh_tokens(user_id) WHERE revoked = FALSE;
CREATE INDEX idx_refresh_tokens_hash ON refresh_tokens(token_hash) WHERE revoked = FALSE;
CREATE INDEX idx_refresh_tokens_family ON refresh_tokens(token_family);
CREATE INDEX idx_refresh_tokens_expires ON refresh_tokens(expires_at);
CREATE INDEX idx_refresh_tokens_device ON refresh_tokens(device_fingerprint, user_id);

-- =============================================================================
-- SESSIONS TABLE (Server-Side Session Store)
-- =============================================================================
-- Note: In production with Redis, this may be Redis-only
-- This table serves as backup/audit trail
CREATE TABLE IF NOT EXISTS sessions (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    user_id UUID NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    
    -- Session identification
    session_token VARCHAR(64) NOT NULL UNIQUE, -- SHA-256 hash
    
    -- Device information
    device_fingerprint VARCHAR(255),
    user_agent TEXT,
    ip_address INET,
    device_name VARCHAR(255),
    device_type VARCHAR(50), -- 'desktop', 'mobile', 'tablet'
    
    -- Session validity
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    last_activity_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    expires_at TIMESTAMPTZ NOT NULL,
    
    -- Authentication context
    auth_method VARCHAR(50) NOT NULL,
    cert_serial VARCHAR(100), -- If certificate auth
    oauth_provider VARCHAR(100), -- If OAuth auth
    
    -- Session metadata
    metadata JSONB, -- Store additional session data
    
    -- Session status
    active BOOLEAN NOT NULL DEFAULT TRUE,
    terminated_at TIMESTAMPTZ,
    termination_reason VARCHAR(100),
    
    -- Constraints
    CONSTRAINT valid_session_expiry CHECK (expires_at > created_at)
);

CREATE INDEX idx_sessions_user_id ON sessions(user_id) WHERE active = TRUE;
CREATE INDEX idx_sessions_token ON sessions(session_token) WHERE active = TRUE;
CREATE INDEX idx_sessions_last_activity ON sessions(last_activity_at DESC) WHERE active = TRUE;
CREATE INDEX idx_sessions_expires ON sessions(expires_at);
CREATE INDEX idx_sessions_device ON sessions(device_fingerprint, user_id);

-- =============================================================================
-- JWT SIGNING KEYS TABLE
-- =============================================================================
CREATE TABLE IF NOT EXISTS jwt_signing_keys (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    
    -- Key identification
    kid VARCHAR(100) NOT NULL UNIQUE, -- Key ID for JWKS
    algorithm VARCHAR(50) NOT NULL, -- 'RS256', 'RS384', 'RS512', 'EdDSA'
    
    -- Key material (encrypted at rest)
    private_key_pem TEXT NOT NULL, -- RSA or Ed25519 private key (encrypted)
    public_key_pem TEXT NOT NULL, -- Public key for JWKS endpoint
    
    -- Key status
    status VARCHAR(50) NOT NULL DEFAULT 'active' CHECK (status IN ('active', 'rotating', 'retired')),
    is_primary BOOLEAN NOT NULL DEFAULT FALSE, -- Current signing key
    
    -- Key validity
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    activated_at TIMESTAMPTZ,
    rotated_at TIMESTAMPTZ,
    retired_at TIMESTAMPTZ,
    expires_at TIMESTAMPTZ, -- When to stop accepting tokens signed with this key
    
    -- Usage tracking
    tokens_signed BIGINT NOT NULL DEFAULT 0,
    last_used_at TIMESTAMPTZ,
    
    -- Metadata
    key_size INTEGER, -- RSA key size (2048, 4096, etc.)
    rotation_reason VARCHAR(255)
);

-- Ensure only one primary key is active
CREATE UNIQUE INDEX idx_jwt_keys_one_primary ON jwt_signing_keys(is_primary) WHERE is_primary = TRUE AND status = 'active';

CREATE INDEX idx_jwt_keys_kid ON jwt_signing_keys(kid);
CREATE INDEX idx_jwt_keys_status ON jwt_signing_keys(status);
CREATE INDEX idx_jwt_keys_primary ON jwt_signing_keys(is_primary) WHERE is_primary = TRUE;

-- =============================================================================
-- AUTH AUDIT LOG TABLE (HIPAA Compliance)
-- =============================================================================
CREATE TABLE IF NOT EXISTS auth_audit_log (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    
    -- Who
    user_id UUID REFERENCES users(id) ON DELETE SET NULL,
    email VARCHAR(255), -- Preserve email even if user deleted
    
    -- What
    event_type VARCHAR(100) NOT NULL, -- 'login', 'logout', 'login_failed', 'token_refresh', etc.
    event_status VARCHAR(50) NOT NULL, -- 'success', 'failure', 'blocked'
    
    -- How
    auth_method VARCHAR(50), -- 'email_password', 'oauth', 'certificate'
    oauth_provider VARCHAR(100),
    cert_serial VARCHAR(100),
    
    -- When
    timestamp TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    
    -- Where
    ip_address INET,
    user_agent TEXT,
    device_fingerprint VARCHAR(255),
    geolocation JSONB, -- {country, region, city}
    
    -- Context
    session_id UUID,
    request_id VARCHAR(100), -- Trace ID for correlation
    endpoint VARCHAR(255), -- API endpoint accessed
    
    -- Details
    error_message TEXT,
    metadata JSONB, -- Additional event-specific data
    
    -- Security flags
    anomaly_detected BOOLEAN DEFAULT FALSE,
    risk_score INTEGER, -- 0-100
    blocked_reason VARCHAR(255)
);

-- Partitioning by timestamp for efficient queries (PostgreSQL 10+)
-- Note: Actual partitioning commands would be separate
CREATE INDEX idx_audit_log_user_id ON auth_audit_log(user_id, timestamp DESC);
CREATE INDEX idx_audit_log_timestamp ON auth_audit_log(timestamp DESC);
CREATE INDEX idx_audit_log_event_type ON auth_audit_log(event_type);
CREATE INDEX idx_audit_log_ip ON auth_audit_log(ip_address, timestamp DESC);
CREATE INDEX idx_audit_log_anomaly ON auth_audit_log(anomaly_detected) WHERE anomaly_detected = TRUE;

-- =============================================================================
-- USER PERMISSIONS TABLE
-- =============================================================================
CREATE TABLE IF NOT EXISTS user_permissions (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    user_id UUID NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    
    -- Permission definition
    permission VARCHAR(255) NOT NULL, -- 'patient:read', 'patient:write', 'admin:*'
    resource_type VARCHAR(100), -- 'patient', 'appointment', 'billing'
    resource_id UUID, -- Specific resource (optional)
    
    -- Permission metadata
    granted_by UUID REFERENCES users(id),
    granted_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    expires_at TIMESTAMPTZ, -- Optional expiration
    
    -- Metadata
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    
    -- Constraints
    CONSTRAINT unique_user_permission UNIQUE (user_id, permission, resource_type, resource_id)
);

CREATE INDEX idx_user_permissions_user_id ON user_permissions(user_id);
CREATE INDEX idx_user_permissions_permission ON user_permissions(permission);
CREATE INDEX idx_user_permissions_resource ON user_permissions(resource_type, resource_id);

-- =============================================================================
-- RATE LIMITING TABLE
-- =============================================================================
CREATE TABLE IF NOT EXISTS rate_limits (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    
    -- Rate limit key
    key_type VARCHAR(50) NOT NULL, -- 'ip', 'user', 'email'
    key_value VARCHAR(255) NOT NULL,
    
    -- Rate limit tracking
    endpoint VARCHAR(255),
    request_count INTEGER NOT NULL DEFAULT 1,
    window_start TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    window_end TIMESTAMPTZ NOT NULL,
    
    -- Lockout tracking
    locked_until TIMESTAMPTZ,
    
    -- Metadata
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    
    -- Constraints
    CONSTRAINT unique_rate_limit_key UNIQUE (key_type, key_value, endpoint, window_start)
);

CREATE INDEX idx_rate_limits_key ON rate_limits(key_type, key_value);
CREATE INDEX idx_rate_limits_window ON rate_limits(window_end);
CREATE INDEX idx_rate_limits_locked ON rate_limits(locked_until) WHERE locked_until IS NOT NULL;

-- =============================================================================
-- TRIGGERS FOR AUTOMATIC TIMESTAMPS
-- =============================================================================

-- Function to update updated_at timestamp
CREATE OR REPLACE FUNCTION update_updated_at_column()
RETURNS TRIGGER AS $$
BEGIN
    NEW.updated_at = NOW();
    RETURN NEW;
END;
$$ language 'plpgsql';

-- Apply to all tables with updated_at
CREATE TRIGGER update_users_updated_at BEFORE UPDATE ON users
    FOR EACH ROW EXECUTE FUNCTION update_updated_at_column();

CREATE TRIGGER update_user_credentials_updated_at BEFORE UPDATE ON user_credentials
    FOR EACH ROW EXECUTE FUNCTION update_updated_at_column();

CREATE TRIGGER update_oauth_accounts_updated_at BEFORE UPDATE ON oauth_accounts
    FOR EACH ROW EXECUTE FUNCTION update_updated_at_column();

CREATE TRIGGER update_client_certificates_updated_at BEFORE UPDATE ON client_certificates
    FOR EACH ROW EXECUTE FUNCTION update_updated_at_column();

CREATE TRIGGER update_rate_limits_updated_at BEFORE UPDATE ON rate_limits
    FOR EACH ROW EXECUTE FUNCTION update_updated_at_column();

-- =============================================================================
-- VIEWS FOR COMMON QUERIES
-- =============================================================================

-- Active users view (excludes soft-deleted)
CREATE OR REPLACE VIEW active_users AS
SELECT * FROM users
WHERE deleted_at IS NULL AND status = 'active';

-- Active sessions view
CREATE OR REPLACE VIEW active_sessions AS
SELECT 
    s.*,
    u.email,
    u.full_name,
    u.status as user_status
FROM sessions s
JOIN users u ON s.user_id = u.id
WHERE s.active = TRUE 
  AND s.expires_at > NOW()
  AND u.deleted_at IS NULL;

-- User authentication methods view
CREATE OR REPLACE VIEW user_auth_methods AS
SELECT 
    u.id as user_id,
    u.email,
    CASE WHEN uc.id IS NOT NULL THEN TRUE ELSE FALSE END as has_password,
    ARRAY_AGG(DISTINCT oa.provider) FILTER (WHERE oa.provider IS NOT NULL) as oauth_providers,
    COUNT(DISTINCT cc.id) FILTER (WHERE cc.status = 'active') as active_certificates
FROM users u
LEFT JOIN user_credentials uc ON u.id = uc.user_id
LEFT JOIN oauth_accounts oa ON u.id = oa.user_id
LEFT JOIN client_certificates cc ON u.id = cc.user_id
WHERE u.deleted_at IS NULL
GROUP BY u.id, u.email, uc.id;

-- =============================================================================
-- COMMENTS FOR DOCUMENTATION
-- =============================================================================

COMMENT ON TABLE users IS 'Core user identity table - central to all authentication methods';
COMMENT ON TABLE user_credentials IS 'Email/password credentials with Argon2id hashing and MFA';
COMMENT ON TABLE oauth_accounts IS 'OAuth/OIDC SSO provider accounts (Google, Azure AD, Okta)';
COMMENT ON TABLE client_certificates IS 'X.509 client certificates for mTLS authentication';
COMMENT ON TABLE refresh_tokens IS 'Long-lived refresh tokens with rotation support';
COMMENT ON TABLE sessions IS 'Server-side session store with device tracking';
COMMENT ON TABLE jwt_signing_keys IS 'RSA/Ed25519 keys for JWT token signing with rotation';
COMMENT ON TABLE auth_audit_log IS 'HIPAA-compliant audit log for all authentication events';
COMMENT ON TABLE user_permissions IS 'Fine-grained permission assignments';
COMMENT ON TABLE rate_limits IS 'Rate limiting and account lockout tracking';
