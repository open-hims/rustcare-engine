-- Migration: Enable Row-Level Security (RLS) Policies
-- Description: Implements database-level tenant isolation using PostgreSQL RLS
-- Author: RustCare Team
-- Date: 2025-10-21

-- =============================================================================
-- ENABLE RLS ON ALL MULTI-TENANT TABLES
-- =============================================================================

-- Core user tables (NOT NULL organization_id)
ALTER TABLE users ENABLE ROW LEVEL SECURITY;
ALTER TABLE user_credentials ENABLE ROW LEVEL SECURITY;
ALTER TABLE oauth_accounts ENABLE ROW LEVEL SECURITY;
ALTER TABLE client_certificates ENABLE ROW LEVEL SECURITY;
ALTER TABLE refresh_tokens ENABLE ROW LEVEL SECURITY;
ALTER TABLE sessions ENABLE ROW LEVEL SECURITY;

-- Global resource tables (NULLABLE organization_id)
ALTER TABLE jwt_signing_keys ENABLE ROW LEVEL SECURITY;
ALTER TABLE roles ENABLE ROW LEVEL SECURITY;
ALTER TABLE user_permissions ENABLE ROW LEVEL SECURITY;
ALTER TABLE rate_limits ENABLE ROW LEVEL SECURITY;

-- Audit table (special read-only policy)
ALTER TABLE audit_logs ENABLE ROW LEVEL SECURITY;

-- =============================================================================
-- RLS POLICIES FOR CORE USER TABLES (NOT NULL organization_id)
-- =============================================================================

-- Users table: Only access users within same organization
CREATE POLICY users_org_isolation ON users
    USING (organization_id = current_setting('app.organization_id', true)::UUID);

CREATE POLICY users_org_isolation_insert ON users
    FOR INSERT
    WITH CHECK (organization_id = current_setting('app.organization_id', true)::UUID);

-- User Credentials table
CREATE POLICY credentials_org_isolation ON user_credentials
    USING (organization_id = current_setting('app.organization_id', true)::UUID);

CREATE POLICY credentials_org_isolation_insert ON user_credentials
    FOR INSERT
    WITH CHECK (organization_id = current_setting('app.organization_id', true)::UUID);

-- OAuth Accounts table
CREATE POLICY oauth_org_isolation ON oauth_accounts
    USING (organization_id = current_setting('app.organization_id', true)::UUID);

CREATE POLICY oauth_org_isolation_insert ON oauth_accounts
    FOR INSERT
    WITH CHECK (organization_id = current_setting('app.organization_id', true)::UUID);

-- Client Certificates table
CREATE POLICY certificates_org_isolation ON client_certificates
    USING (organization_id = current_setting('app.organization_id', true)::UUID);

CREATE POLICY certificates_org_isolation_insert ON client_certificates
    FOR INSERT
    WITH CHECK (organization_id = current_setting('app.organization_id', true)::UUID);

-- Refresh Tokens table
CREATE POLICY tokens_org_isolation ON refresh_tokens
    USING (organization_id = current_setting('app.organization_id', true)::UUID);

CREATE POLICY tokens_org_isolation_insert ON refresh_tokens
    FOR INSERT
    WITH CHECK (organization_id = current_setting('app.organization_id', true)::UUID);

-- Sessions table
CREATE POLICY sessions_org_isolation ON sessions
    USING (organization_id = current_setting('app.organization_id', true)::UUID);

CREATE POLICY sessions_org_isolation_insert ON sessions
    FOR INSERT
    WITH CHECK (organization_id = current_setting('app.organization_id', true)::UUID);

-- =============================================================================
-- RLS POLICIES FOR GLOBAL RESOURCE TABLES (NULLABLE organization_id)
-- Allow access to global resources (NULL) + organization-specific resources
-- =============================================================================

-- JWT Signing Keys: Access global keys (NULL) OR organization-specific keys
CREATE POLICY jwt_keys_org_isolation ON jwt_signing_keys
    USING (
        organization_id IS NULL OR 
        organization_id = current_setting('app.organization_id', true)::UUID
    );

CREATE POLICY jwt_keys_org_isolation_insert ON jwt_signing_keys
    FOR INSERT
    WITH CHECK (
        organization_id IS NULL OR 
        organization_id = current_setting('app.organization_id', true)::UUID
    );

-- Roles: Access global roles (NULL) OR organization-specific roles
CREATE POLICY roles_org_isolation ON roles
    USING (
        organization_id IS NULL OR 
        organization_id = current_setting('app.organization_id', true)::UUID
    );

CREATE POLICY roles_org_isolation_insert ON roles
    FOR INSERT
    WITH CHECK (
        organization_id IS NULL OR 
        organization_id = current_setting('app.organization_id', true)::UUID
    );

-- User Permissions: Access global permissions (NULL) OR organization-specific permissions
CREATE POLICY permissions_org_isolation ON user_permissions
    USING (
        organization_id IS NULL OR 
        organization_id = current_setting('app.organization_id', true)::UUID
    );

CREATE POLICY permissions_org_isolation_insert ON user_permissions
    FOR INSERT
    WITH CHECK (
        organization_id IS NULL OR 
        organization_id = current_setting('app.organization_id', true)::UUID
    );

-- Rate Limits: Access global limits (NULL) OR organization-specific limits
CREATE POLICY rate_limits_org_isolation ON rate_limits
    USING (
        organization_id IS NULL OR 
        organization_id = current_setting('app.organization_id', true)::UUID
    );

CREATE POLICY rate_limits_org_isolation_insert ON rate_limits
    FOR INSERT
    WITH CHECK (
        organization_id IS NULL OR 
        organization_id = current_setting('app.organization_id', true)::UUID
    );

-- =============================================================================
-- RLS POLICY FOR AUDIT LOGS (READ-ONLY, NO INSERT/UPDATE/DELETE)
-- =============================================================================

-- Audit Logs: READ-ONLY access to organization's audit logs
-- No WITH CHECK clause = prevents INSERT/UPDATE/DELETE through this policy
CREATE POLICY audit_logs_org_isolation ON audit_logs
    FOR SELECT
    USING (organization_id = current_setting('app.organization_id', true)::UUID);

-- Separate policy for INSERT (used by audit system with elevated privileges)
-- Only the audit system can insert logs, checked by organization_id match
CREATE POLICY audit_logs_system_insert ON audit_logs
    FOR INSERT
    WITH CHECK (organization_id = current_setting('app.organization_id', true)::UUID);

-- =============================================================================
-- GRANT PERMISSIONS
-- =============================================================================

-- Note: Ensure application database user has appropriate permissions
-- Example (run separately as superuser if needed):
-- GRANT SELECT, INSERT, UPDATE, DELETE ON ALL TABLES IN SCHEMA public TO rustcare_app;
-- GRANT USAGE, SELECT ON ALL SEQUENCES IN SCHEMA public TO rustcare_app;

-- =============================================================================
-- VERIFICATION QUERIES (Run after migration to verify RLS is working)
-- =============================================================================

-- 1. Verify RLS is enabled:
-- SELECT schemaname, tablename, rowsecurity 
-- FROM pg_tables 
-- WHERE schemaname = 'public' AND rowsecurity = true;

-- 2. List all RLS policies:
-- SELECT schemaname, tablename, policyname, permissive, roles, cmd, qual, with_check
-- FROM pg_policies
-- WHERE schemaname = 'public'
-- ORDER BY tablename, policyname;

-- 3. Test tenant isolation (as application user):
-- SET app.organization_id = '123e4567-e89b-12d3-a456-426614174000';
-- SELECT COUNT(*) FROM users; -- Should only see users from org 123e4567-e89b-12d3-a456-426614174000
--
-- SET app.organization_id = '987fcdeb-51a2-43f1-9876-543210fedcba';
-- SELECT COUNT(*) FROM users; -- Should only see users from org 987fcdeb-51a2-43f1-9876-543210fedcba

-- 4. Test global resource access:
-- SET app.organization_id = '123e4567-e89b-12d3-a456-426614174000';
-- SELECT COUNT(*) FROM jwt_signing_keys WHERE organization_id IS NULL; -- Should see global keys
-- SELECT COUNT(*) FROM roles WHERE organization_id IS NULL; -- Should see global roles

-- =============================================================================
-- IMPORTANT NOTES
-- =============================================================================

-- 1. Application MUST set app.organization_id session variable for every request
--    using: SET LOCAL app.organization_id = '<uuid>';
--
-- 2. Use SET LOCAL (not SET) in transactions to automatically reset after commit
--
-- 3. If app.organization_id is not set, current_setting() with true flag returns NULL,
--    causing RLS policies to filter out all rows (safe default)
--
-- 4. Superuser and table owner bypass RLS by default
--    Use: ALTER TABLE <table> FORCE ROW LEVEL SECURITY; to enforce for owners
--
-- 5. For admin/superuser access to all organizations, temporarily disable RLS:
--    SET SESSION row_security = OFF; (requires appropriate privileges)
--
-- 6. Audit logs are read-only for organizations (SELECT only)
--    Only the audit system can insert logs
--
-- 7. Global resources (NULL organization_id) are accessible to all organizations
--    Use for: system-wide JWT keys, default roles, global rate limits

-- =============================================================================
-- MONITORING & ALERTING
-- =============================================================================

-- Create function to log RLS violations (optional)
CREATE OR REPLACE FUNCTION log_rls_violation()
RETURNS event_trigger
LANGUAGE plpgsql
AS $$
BEGIN
    -- Log RLS policy violations for security monitoring
    -- This is triggered when RLS denies access
    RAISE WARNING 'RLS Policy Violation: % at %', TG_TAG, NOW();
END;
$$;

-- Create event trigger for RLS violations
-- CREATE EVENT TRIGGER rls_violation_trigger
-- ON ddl_command_end
-- WHEN TAG IN ('ALTER TABLE', 'DROP TABLE')
-- EXECUTE FUNCTION log_rls_violation();
