-- Multi-Tenant Infrastructure: Organizations Table
-- PostgreSQL Migration: Create organizations table
-- Version: 002
-- Description: Foundation for multi-tenant architecture with organization-level isolation

-- =============================================================================
-- ORGANIZATIONS TABLE
-- =============================================================================
-- Core organization/tenant table for multi-tenant isolation
CREATE TABLE IF NOT EXISTS organizations (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    
    -- Organization identification
    name VARCHAR(255) NOT NULL,
    slug VARCHAR(100) UNIQUE NOT NULL,
    domain VARCHAR(255), -- Primary domain for the organization
    
    -- Organization details
    description TEXT,
    logo_url TEXT,
    website_url TEXT,
    
    -- Subscription and limits
    subscription_tier VARCHAR(50) NOT NULL DEFAULT 'free' CHECK (subscription_tier IN ('free', 'starter', 'professional', 'enterprise', 'custom')),
    max_users INTEGER NOT NULL DEFAULT 10,
    max_storage_gb INTEGER NOT NULL DEFAULT 5,
    
    -- Organization status
    is_active BOOLEAN NOT NULL DEFAULT TRUE,
    is_verified BOOLEAN NOT NULL DEFAULT FALSE,
    verified_at TIMESTAMPTZ,
    
    -- Settings (JSONB for flexible configuration)
    settings JSONB DEFAULT '{}',
    -- Example settings:
    -- {
    --   "branding": {"primary_color": "#0066cc", "logo_url": "..."},
    --   "security": {"require_mfa": true, "session_timeout_minutes": 30},
    --   "features": {"enable_audit_logs": true, "enable_api_access": true},
    --   "compliance": {"hipaa_enabled": true, "data_retention_days": 2555}
    -- }
    
    -- Contact information
    contact_email VARCHAR(255),
    contact_phone VARCHAR(50),
    billing_email VARCHAR(255),
    
    -- Address (optional)
    address_line1 VARCHAR(255),
    address_line2 VARCHAR(255),
    city VARCHAR(100),
    state_province VARCHAR(100),
    postal_code VARCHAR(20),
    country VARCHAR(100),
    
    -- Audit timestamps
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    deleted_at TIMESTAMPTZ, -- Soft delete for compliance
    
    -- Constraints
    CONSTRAINT slug_lowercase CHECK (slug = LOWER(slug)),
    CONSTRAINT slug_format CHECK (slug ~ '^[a-z0-9-]+$')
);

-- Indexes for organizations table
CREATE INDEX idx_organizations_slug ON organizations(slug) WHERE deleted_at IS NULL;
CREATE INDEX idx_organizations_domain ON organizations(domain) WHERE deleted_at IS NULL AND domain IS NOT NULL;
CREATE INDEX idx_organizations_is_active ON organizations(is_active) WHERE deleted_at IS NULL;
CREATE INDEX idx_organizations_subscription_tier ON organizations(subscription_tier) WHERE deleted_at IS NULL;
CREATE INDEX idx_organizations_created_at ON organizations(created_at DESC) WHERE deleted_at IS NULL;

-- Comment on table
COMMENT ON TABLE organizations IS 'Core organizations/tenants table for multi-tenant architecture with RLS isolation';
COMMENT ON COLUMN organizations.slug IS 'URL-safe unique identifier for the organization (e.g., acme-corp)';
COMMENT ON COLUMN organizations.subscription_tier IS 'Subscription level determining feature access and limits';
COMMENT ON COLUMN organizations.settings IS 'Flexible JSON configuration for organization-specific settings';
COMMENT ON COLUMN organizations.max_users IS 'Maximum number of users allowed for this organization';
COMMENT ON COLUMN organizations.max_storage_gb IS 'Maximum storage capacity in GB for this organization';

-- Create a default system organization (for global/system-level data)
INSERT INTO organizations (
    id,
    name,
    slug,
    subscription_tier,
    max_users,
    max_storage_gb,
    is_active,
    is_verified,
    verified_at,
    settings
) VALUES (
    '00000000-0000-0000-0000-000000000000'::UUID,
    'System',
    'system',
    'custom',
    999999,
    999999,
    TRUE,
    TRUE,
    NOW(),
    '{"system": true, "description": "System organization for global resources"}'::JSONB
) ON CONFLICT (id) DO NOTHING;
