-- Rollback Multi-Tenant Infrastructure: Organizations Table
-- PostgreSQL Migration Rollback: Drop organizations table
-- Version: 002_rollback

-- Drop indexes
DROP INDEX IF EXISTS idx_organizations_created_at;
DROP INDEX IF EXISTS idx_organizations_subscription_tier;
DROP INDEX IF EXISTS idx_organizations_is_active;
DROP INDEX IF EXISTS idx_organizations_domain;
DROP INDEX IF EXISTS idx_organizations_slug;

-- Drop table
DROP TABLE IF EXISTS organizations;
