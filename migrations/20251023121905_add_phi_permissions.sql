-- Migration: Add PHI (Protected Health Information) Access Permissions
-- Description: Implements HIPAA-compliant field-level access control
-- Author: RustCare Team
-- Date: 2025-10-21

-- =============================================================================
-- CREATE PERMISSIONS TABLE (if not exists)
-- =============================================================================

CREATE TABLE IF NOT EXISTS permissions (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    name VARCHAR(255) UNIQUE NOT NULL,
    resource VARCHAR(100) NOT NULL,
    action VARCHAR(50) NOT NULL,
    description TEXT,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    
    CONSTRAINT unique_permission UNIQUE (name)
);

CREATE INDEX IF NOT EXISTS idx_permissions_name ON permissions(name);
CREATE INDEX IF NOT EXISTS idx_permissions_resource ON permissions(resource);

-- =============================================================================
-- CREATE ROLES TABLE (if not exists)
-- =============================================================================

CREATE TABLE IF NOT EXISTS roles (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    name VARCHAR(100) UNIQUE NOT NULL,
    description TEXT,
    organization_id UUID REFERENCES organizations(id) ON DELETE CASCADE,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    
    CONSTRAINT unique_role_per_org UNIQUE (name, organization_id)
);

CREATE INDEX IF NOT EXISTS idx_roles_name ON roles(name);
CREATE INDEX IF NOT EXISTS idx_roles_organization_id ON roles(organization_id) WHERE organization_id IS NOT NULL;

-- Enable RLS on roles table
DO $$ 
BEGIN
    IF EXISTS (SELECT 1 FROM pg_tables WHERE tablename = 'roles') THEN
        ALTER TABLE roles ENABLE ROW LEVEL SECURITY;
        
        -- Roles: Access global roles (NULL) OR organization-specific roles
        EXECUTE 'CREATE POLICY roles_org_isolation ON roles
            USING (
                organization_id IS NULL OR 
                organization_id = current_setting(''app.organization_id'', true)::UUID
            )';
        
        EXECUTE 'CREATE POLICY roles_org_isolation_insert ON roles
            FOR INSERT
            WITH CHECK (
                organization_id IS NULL OR 
                organization_id = current_setting(''app.organization_id'', true)::UUID
            )';
    END IF;
END $$;

-- =============================================================================
-- CREATE ROLE_PERMISSIONS TABLE (if not exists)
-- =============================================================================

CREATE TABLE IF NOT EXISTS role_permissions (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    role_id UUID NOT NULL REFERENCES roles(id) ON DELETE CASCADE,
    permission_id UUID NOT NULL REFERENCES permissions(id) ON DELETE CASCADE,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    
    CONSTRAINT unique_role_permission UNIQUE (role_id, permission_id)
);

CREATE INDEX IF NOT EXISTS idx_role_permissions_role_id ON role_permissions(role_id);
CREATE INDEX IF NOT EXISTS idx_role_permissions_permission_id ON role_permissions(permission_id);

-- =============================================================================
-- ZANZIBAR TUPLE PERSISTENCE
-- =============================================================================
-- Authorization tuples for Google Zanzibar-style access control
-- See: docs/ZANZIBAR_INTEGRATION_STRATEGY.md

CREATE TABLE IF NOT EXISTS zanzibar_tuples (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    organization_id UUID REFERENCES organizations(id) ON DELETE CASCADE,
    
    -- Subject (who)
    subject_namespace VARCHAR(50) NOT NULL,
    subject_type VARCHAR(50) NOT NULL,
    subject_id VARCHAR(255) NOT NULL,
    subject_relation VARCHAR(50), -- For userset subjects
    
    -- Relation (permission)
    relation_name VARCHAR(50) NOT NULL,
    
    -- Object (resource)
    object_namespace VARCHAR(50) NOT NULL,
    object_type VARCHAR(50) NOT NULL,
    object_id VARCHAR(255) NOT NULL,
    
    -- Metadata
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    created_by UUID REFERENCES users(id),
    expires_at TIMESTAMPTZ, -- Optional expiration
    
    -- Uniqueness constraint
    CONSTRAINT unique_tuple UNIQUE (
        organization_id, 
        subject_namespace, subject_type, subject_id, subject_relation,
        relation_name,
        object_namespace, object_type, object_id
    )
);

-- Indexes for fast authorization checks
-- Note: Cannot use NOW() in index predicate as it's not IMMUTABLE
-- Queries will need to filter expires_at at runtime
CREATE INDEX IF NOT EXISTS idx_zanzibar_check ON zanzibar_tuples(
    organization_id, object_type, object_id, relation_name, subject_type, subject_id
);

-- Index for expansion (find all subjects with relation to object)
CREATE INDEX IF NOT EXISTS idx_zanzibar_expand ON zanzibar_tuples(
    organization_id, object_type, object_id, relation_name
);

-- Index for reverse lookup (find all objects a subject has relation to)
CREATE INDEX IF NOT EXISTS idx_zanzibar_reverse ON zanzibar_tuples(
    organization_id, subject_type, subject_id, relation_name
);

-- Partial index for non-expired tuples (covers most queries)
CREATE INDEX IF NOT EXISTS idx_zanzibar_active ON zanzibar_tuples(
    organization_id, object_type, object_id, relation_name
) WHERE expires_at IS NULL;

-- Enable RLS on zanzibar_tuples
DO $$ 
BEGIN
    IF EXISTS (SELECT 1 FROM pg_tables WHERE tablename = 'zanzibar_tuples') THEN
        ALTER TABLE zanzibar_tuples ENABLE ROW LEVEL SECURITY;
        
        EXECUTE 'CREATE POLICY zanzibar_tuples_org_isolation ON zanzibar_tuples
            USING (
                organization_id IS NULL OR 
                organization_id = current_setting(''app.organization_id'', true)::UUID
            )';
        
        EXECUTE 'CREATE POLICY zanzibar_tuples_org_isolation_insert ON zanzibar_tuples
            FOR INSERT
            WITH CHECK (
                organization_id IS NULL OR 
                organization_id = current_setting(''app.organization_id'', true)::UUID
            )';
    END IF;
END $$;

-- =============================================================================
-- NOTES ON DYNAMIC PERMISSIONS
-- =============================================================================
-- 
-- NO SEED DATA: All roles and permissions are created dynamically through the
-- application API. This allows:
--
-- 1. Organizations to define their own roles and permissions
-- 2. Support for international healthcare models (not US-specific)
-- 3. Compliance with varying state/country regulations
-- 4. Integration with auth-zanzibar for fine-grained access control
--
-- See documentation:
-- - docs/ZANZIBAR_INTEGRATION_STRATEGY.md
-- - MIGRATION_HARDCODED_DATA_REPORT.md
--
-- To create roles and permissions, use the Admin API:
--   POST /api/admin/roles
--   POST /api/admin/permissions
--   POST /api/admin/roles/{id}/permissions
--
-- =============================================================================
