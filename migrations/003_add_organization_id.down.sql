-- Rollback Multi-Tenant Infrastructure: Remove organization_id from all tables
-- PostgreSQL Migration Rollback: Remove organization_id foreign keys
-- Version: 003_rollback

-- Drop indexes first
DROP INDEX IF EXISTS idx_audit_logs_organization_id;
DROP INDEX IF EXISTS idx_rate_limits_organization_id;
DROP INDEX IF EXISTS idx_user_permissions_organization_id;
DROP INDEX IF EXISTS idx_roles_organization_id;
DROP INDEX IF EXISTS idx_jwt_signing_keys_organization_id;
DROP INDEX IF EXISTS idx_sessions_organization_id;
DROP INDEX IF EXISTS idx_refresh_tokens_organization_id;
DROP INDEX IF EXISTS idx_client_certificates_organization_id;
DROP INDEX IF EXISTS idx_oauth_accounts_organization_id;
DROP INDEX IF EXISTS idx_user_credentials_organization_id;
DROP INDEX IF EXISTS idx_users_organization_id;

-- Drop foreign key constraints and columns
ALTER TABLE audit_logs DROP CONSTRAINT IF EXISTS audit_logs_organization_id_fkey;
ALTER TABLE audit_logs DROP COLUMN IF EXISTS organization_id;
ALTER TABLE audit_logs ADD COLUMN IF NOT EXISTS tenant_id VARCHAR(255); -- Restore old column name

ALTER TABLE rate_limits DROP COLUMN IF EXISTS organization_id;
ALTER TABLE user_permissions DROP COLUMN IF EXISTS organization_id;

DO $$ 
BEGIN
    IF EXISTS (SELECT 1 FROM information_schema.tables WHERE table_name = 'roles') THEN
        ALTER TABLE roles DROP COLUMN IF EXISTS organization_id;
    END IF;
END $$;

ALTER TABLE jwt_signing_keys DROP COLUMN IF EXISTS organization_id;
ALTER TABLE sessions DROP COLUMN IF EXISTS organization_id;
ALTER TABLE refresh_tokens DROP COLUMN IF EXISTS organization_id;
ALTER TABLE client_certificates DROP COLUMN IF EXISTS organization_id;
ALTER TABLE oauth_accounts DROP COLUMN IF EXISTS organization_id;
ALTER TABLE user_credentials DROP COLUMN IF EXISTS organization_id;
ALTER TABLE users DROP COLUMN IF EXISTS organization_id;
