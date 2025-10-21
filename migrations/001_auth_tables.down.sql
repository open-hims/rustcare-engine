-- Rollback Migration for Authentication System
-- Version: 001
-- Description: Drop all authentication tables and related objects

-- =============================================================================
-- DROP VIEWS
-- =============================================================================
DROP VIEW IF EXISTS user_auth_methods CASCADE;
DROP VIEW IF EXISTS active_sessions CASCADE;
DROP VIEW IF EXISTS active_users CASCADE;

-- =============================================================================
-- DROP TRIGGERS
-- =============================================================================
DROP TRIGGER IF EXISTS update_rate_limits_updated_at ON rate_limits;
DROP TRIGGER IF EXISTS update_client_certificates_updated_at ON client_certificates;
DROP TRIGGER IF EXISTS update_oauth_accounts_updated_at ON oauth_accounts;
DROP TRIGGER IF EXISTS update_user_credentials_updated_at ON user_credentials;
DROP TRIGGER IF EXISTS update_users_updated_at ON users;

-- =============================================================================
-- DROP FUNCTIONS
-- =============================================================================
DROP FUNCTION IF EXISTS update_updated_at_column() CASCADE;

-- =============================================================================
-- DROP TABLES (in reverse dependency order)
-- =============================================================================
DROP TABLE IF EXISTS rate_limits CASCADE;
DROP TABLE IF EXISTS user_permissions CASCADE;
DROP TABLE IF EXISTS auth_audit_log CASCADE;
DROP TABLE IF EXISTS jwt_signing_keys CASCADE;
DROP TABLE IF EXISTS sessions CASCADE;
DROP TABLE IF EXISTS refresh_tokens CASCADE;
DROP TABLE IF EXISTS client_certificates CASCADE;
DROP TABLE IF EXISTS oauth_accounts CASCADE;
DROP TABLE IF EXISTS user_credentials CASCADE;
DROP TABLE IF EXISTS users CASCADE;
