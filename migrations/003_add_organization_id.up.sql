-- Multi-Tenant Infrastructure: Add organization_id to all tables
-- PostgreSQL Migration: Add organization_id foreign keys
-- Version: 003
-- Description: Add organization_id column to all auth tables for tenant isolation

-- =============================================================================
-- ADD ORGANIZATION_ID TO USERS TABLE
-- =============================================================================
ALTER TABLE users 
    ADD COLUMN IF NOT EXISTS organization_id UUID REFERENCES organizations(id) ON DELETE CASCADE;

-- Set default to system organization for existing users
UPDATE users 
SET organization_id = '00000000-0000-0000-0000-000000000000'::UUID 
WHERE organization_id IS NULL;

-- Now make it NOT NULL
ALTER TABLE users 
    ALTER COLUMN organization_id SET NOT NULL;

-- Create index for RLS performance
CREATE INDEX idx_users_organization_id ON users(organization_id) WHERE deleted_at IS NULL;

-- =============================================================================
-- ADD ORGANIZATION_ID TO USER_CREDENTIALS TABLE
-- =============================================================================
ALTER TABLE user_credentials 
    ADD COLUMN IF NOT EXISTS organization_id UUID REFERENCES organizations(id) ON DELETE CASCADE;

-- Set organization_id from parent user
UPDATE user_credentials uc
SET organization_id = u.organization_id
FROM users u
WHERE uc.user_id = u.id AND uc.organization_id IS NULL;

ALTER TABLE user_credentials 
    ALTER COLUMN organization_id SET NOT NULL;

CREATE INDEX idx_user_credentials_organization_id ON user_credentials(organization_id);

-- =============================================================================
-- ADD ORGANIZATION_ID TO OAUTH_ACCOUNTS TABLE
-- =============================================================================
ALTER TABLE oauth_accounts 
    ADD COLUMN IF NOT EXISTS organization_id UUID REFERENCES organizations(id) ON DELETE CASCADE;

UPDATE oauth_accounts oa
SET organization_id = u.organization_id
FROM users u
WHERE oa.user_id = u.id AND oa.organization_id IS NULL;

ALTER TABLE oauth_accounts 
    ALTER COLUMN organization_id SET NOT NULL;

CREATE INDEX idx_oauth_accounts_organization_id ON oauth_accounts(organization_id);

-- =============================================================================
-- ADD ORGANIZATION_ID TO CLIENT_CERTIFICATES TABLE
-- =============================================================================
ALTER TABLE client_certificates 
    ADD COLUMN IF NOT EXISTS organization_id UUID REFERENCES organizations(id) ON DELETE CASCADE;

UPDATE client_certificates cc
SET organization_id = u.organization_id
FROM users u
WHERE cc.user_id = u.id AND cc.organization_id IS NULL;

ALTER TABLE client_certificates 
    ALTER COLUMN organization_id SET NOT NULL;

CREATE INDEX idx_client_certificates_organization_id ON client_certificates(organization_id);

-- =============================================================================
-- ADD ORGANIZATION_ID TO REFRESH_TOKENS TABLE
-- =============================================================================
ALTER TABLE refresh_tokens 
    ADD COLUMN IF NOT EXISTS organization_id UUID REFERENCES organizations(id) ON DELETE CASCADE;

UPDATE refresh_tokens rt
SET organization_id = u.organization_id
FROM users u
WHERE rt.user_id = u.id AND rt.organization_id IS NULL;

ALTER TABLE refresh_tokens 
    ALTER COLUMN organization_id SET NOT NULL;

CREATE INDEX idx_refresh_tokens_organization_id ON refresh_tokens(organization_id);

-- =============================================================================
-- ADD ORGANIZATION_ID TO SESSIONS TABLE
-- =============================================================================
ALTER TABLE sessions 
    ADD COLUMN IF NOT EXISTS organization_id UUID REFERENCES organizations(id) ON DELETE CASCADE;

UPDATE sessions s
SET organization_id = u.organization_id
FROM users u
WHERE s.user_id = u.id AND s.organization_id IS NULL;

ALTER TABLE sessions 
    ALTER COLUMN organization_id SET NOT NULL;

CREATE INDEX idx_sessions_organization_id ON sessions(organization_id);

-- =============================================================================
-- ADD ORGANIZATION_ID TO JWT_SIGNING_KEYS TABLE
-- =============================================================================
-- JWT keys can be global (NULL) or organization-specific
ALTER TABLE jwt_signing_keys 
    ADD COLUMN IF NOT EXISTS organization_id UUID REFERENCES organizations(id) ON DELETE CASCADE;

-- Leave as NULL for global keys (can be used across all organizations)
-- No NOT NULL constraint here - allows global keys

CREATE INDEX idx_jwt_signing_keys_organization_id ON jwt_signing_keys(organization_id) WHERE organization_id IS NOT NULL;

-- =============================================================================
-- ADD ORGANIZATION_ID TO ROLES TABLE (if exists)
-- =============================================================================
-- Roles can be global (NULL) or organization-specific
DO $$ 
BEGIN
    IF EXISTS (SELECT 1 FROM information_schema.tables WHERE table_name = 'roles') THEN
        ALTER TABLE roles 
            ADD COLUMN IF NOT EXISTS organization_id UUID REFERENCES organizations(id) ON DELETE CASCADE;
        
        CREATE INDEX IF NOT EXISTS idx_roles_organization_id ON roles(organization_id) WHERE organization_id IS NOT NULL;
    END IF;
END $$;

-- =============================================================================
-- ADD ORGANIZATION_ID TO USER_PERMISSIONS TABLE
-- =============================================================================
-- Permissions can be global (NULL) or organization-specific
ALTER TABLE user_permissions 
    ADD COLUMN IF NOT EXISTS organization_id UUID REFERENCES organizations(id) ON DELETE CASCADE;

UPDATE user_permissions up
SET organization_id = u.organization_id
FROM users u
WHERE up.user_id = u.id AND up.organization_id IS NULL;

-- Allow NULL for global permissions
CREATE INDEX idx_user_permissions_organization_id ON user_permissions(organization_id) WHERE organization_id IS NOT NULL;

-- =============================================================================
-- ADD ORGANIZATION_ID TO RATE_LIMITS TABLE
-- =============================================================================
ALTER TABLE rate_limits 
    ADD COLUMN IF NOT EXISTS organization_id UUID REFERENCES organizations(id) ON DELETE CASCADE;

-- Rate limits without organization_id are global
-- No migration needed - leave as NULL for existing records

CREATE INDEX idx_rate_limits_organization_id ON rate_limits(organization_id) WHERE organization_id IS NOT NULL;

-- =============================================================================
-- ADD ORGANIZATION_ID TO AUTH_AUDIT_LOG TABLE
-- =============================================================================
-- Rename tenant_id to organization_id for consistency (if exists)
DO $$ 
BEGIN
    -- Check if tenant_id column exists in auth_audit_log
    IF EXISTS (
        SELECT 1 
        FROM information_schema.columns 
        WHERE table_name = 'auth_audit_log' 
        AND column_name = 'tenant_id'
    ) THEN
        -- Rename tenant_id to organization_id
        ALTER TABLE auth_audit_log RENAME COLUMN tenant_id TO organization_id;
        
        -- Add foreign key if not exists
        IF NOT EXISTS (
            SELECT 1 
            FROM information_schema.table_constraints 
            WHERE constraint_name = 'auth_audit_log_organization_id_fkey'
        ) THEN
            ALTER TABLE auth_audit_log 
                ADD CONSTRAINT auth_audit_log_organization_id_fkey 
                FOREIGN KEY (organization_id) REFERENCES organizations(id) ON DELETE CASCADE;
        END IF;
    ELSE
        -- Add organization_id column if doesn't exist
        ALTER TABLE auth_audit_log 
            ADD COLUMN IF NOT EXISTS organization_id UUID REFERENCES organizations(id) ON DELETE CASCADE;
    END IF;
    
    -- Create index
    CREATE INDEX IF NOT EXISTS idx_auth_audit_log_organization_id ON auth_audit_log(organization_id);
END $$;

-- =============================================================================
-- COMMENTS
-- =============================================================================
COMMENT ON COLUMN users.organization_id IS 'Organization/tenant this user belongs to (required for RLS isolation)';
COMMENT ON COLUMN user_credentials.organization_id IS 'Organization/tenant for credential isolation';
COMMENT ON COLUMN oauth_accounts.organization_id IS 'Organization/tenant for OAuth account isolation';
COMMENT ON COLUMN client_certificates.organization_id IS 'Organization/tenant for certificate isolation';
COMMENT ON COLUMN refresh_tokens.organization_id IS 'Organization/tenant for token isolation';
COMMENT ON COLUMN sessions.organization_id IS 'Organization/tenant for session isolation';
COMMENT ON COLUMN jwt_signing_keys.organization_id IS 'Organization/tenant for key isolation (NULL = global key)';
COMMENT ON COLUMN user_permissions.organization_id IS 'Organization/tenant for permission isolation (NULL = global permission)';
COMMENT ON COLUMN rate_limits.organization_id IS 'Organization/tenant for rate limit isolation (NULL = global limit)';
COMMENT ON COLUMN auth_audit_log.organization_id IS 'Organization/tenant for audit log isolation';
