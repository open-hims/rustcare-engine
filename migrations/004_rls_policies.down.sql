-- Rollback Migration: Disable Row-Level Security (RLS) Policies
-- Description: Removes all RLS policies and disables RLS on tables
-- Author: RustCare Team
-- Date: 2025-10-21

-- =============================================================================
-- DROP RLS POLICIES
-- =============================================================================

-- Core user tables
DROP POLICY IF EXISTS users_org_isolation ON users;
DROP POLICY IF EXISTS users_org_isolation_insert ON users;

DROP POLICY IF EXISTS credentials_org_isolation ON user_credentials;
DROP POLICY IF EXISTS credentials_org_isolation_insert ON user_credentials;

DROP POLICY IF EXISTS oauth_org_isolation ON oauth_accounts;
DROP POLICY IF EXISTS oauth_org_isolation_insert ON oauth_accounts;

DROP POLICY IF EXISTS certificates_org_isolation ON client_certificates;
DROP POLICY IF EXISTS certificates_org_isolation_insert ON client_certificates;

DROP POLICY IF EXISTS tokens_org_isolation ON refresh_tokens;
DROP POLICY IF EXISTS tokens_org_isolation_insert ON refresh_tokens;

DROP POLICY IF EXISTS sessions_org_isolation ON sessions;
DROP POLICY IF EXISTS sessions_org_isolation_insert ON sessions;

-- Global resource tables
DROP POLICY IF EXISTS jwt_keys_org_isolation ON jwt_signing_keys;
DROP POLICY IF EXISTS jwt_keys_org_isolation_insert ON jwt_signing_keys;

DROP POLICY IF EXISTS roles_org_isolation ON roles;
DROP POLICY IF EXISTS roles_org_isolation_insert ON roles;

DROP POLICY IF EXISTS permissions_org_isolation ON user_permissions;
DROP POLICY IF EXISTS permissions_org_isolation_insert ON user_permissions;

DROP POLICY IF EXISTS rate_limits_org_isolation ON rate_limits;
DROP POLICY IF EXISTS rate_limits_org_isolation_insert ON rate_limits;

-- Audit logs
DROP POLICY IF EXISTS audit_logs_org_isolation ON audit_logs;
DROP POLICY IF EXISTS audit_logs_system_insert ON audit_logs;

-- =============================================================================
-- DISABLE RLS ON ALL TABLES
-- =============================================================================

ALTER TABLE users DISABLE ROW LEVEL SECURITY;
ALTER TABLE user_credentials DISABLE ROW LEVEL SECURITY;
ALTER TABLE oauth_accounts DISABLE ROW LEVEL SECURITY;
ALTER TABLE client_certificates DISABLE ROW LEVEL SECURITY;
ALTER TABLE refresh_tokens DISABLE ROW LEVEL SECURITY;
ALTER TABLE sessions DISABLE ROW LEVEL SECURITY;
ALTER TABLE jwt_signing_keys DISABLE ROW LEVEL SECURITY;
ALTER TABLE roles DISABLE ROW LEVEL SECURITY;
ALTER TABLE user_permissions DISABLE ROW LEVEL SECURITY;
ALTER TABLE rate_limits DISABLE ROW LEVEL SECURITY;
ALTER TABLE audit_logs DISABLE ROW LEVEL SECURITY;

-- =============================================================================
-- DROP MONITORING FUNCTIONS (if created)
-- =============================================================================

-- DROP EVENT TRIGGER IF EXISTS rls_violation_trigger;
-- DROP FUNCTION IF EXISTS log_rls_violation();

-- =============================================================================
-- VERIFICATION
-- =============================================================================

-- Verify RLS is disabled:
-- SELECT schemaname, tablename, rowsecurity 
-- FROM pg_tables 
-- WHERE schemaname = 'public';
-- All should show rowsecurity = false
