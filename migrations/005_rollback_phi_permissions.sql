-- Migration Rollback: Remove PHI Access Permissions
-- Description: Removes HIPAA-compliant field-level access control
-- Author: RustCare Team
-- Date: 2025-10-21

-- =============================================================================
-- REMOVE ROLE-PERMISSION ASSIGNMENTS
-- =============================================================================

-- Delete all PHI permission assignments
DELETE FROM role_permissions
WHERE permission_id IN (
    SELECT id FROM permissions WHERE resource = 'phi'
);

-- =============================================================================
-- REMOVE PHI PERMISSIONS
-- =============================================================================

DELETE FROM permissions WHERE name = 'phi:view:unmasked';
DELETE FROM permissions WHERE name = 'phi:view:ephi';
DELETE FROM permissions WHERE name = 'phi:view:restricted';
DELETE FROM permissions WHERE name = 'phi:view:confidential';
DELETE FROM permissions WHERE name = 'phi:view:internal';
DELETE FROM permissions WHERE name = 'phi:view:public';

-- =============================================================================
-- OPTIONALLY REMOVE ROLES (uncomment if needed)
-- =============================================================================

-- Note: Only uncomment these if you want to completely remove the roles
-- This is usually NOT recommended as roles may have other permissions

-- DELETE FROM user_roles WHERE role_id IN (
--     SELECT id FROM roles WHERE name IN (
--         'doctor', 'nurse', 'receptionist', 'medical_records',
--         'billing', 'admin', 'compliance_officer'
--     )
-- );

-- DELETE FROM roles WHERE name IN (
--     'doctor', 'nurse', 'receptionist', 'medical_records',
--     'billing', 'admin', 'compliance_officer'
-- );

-- =============================================================================
-- DROP INDEXES
-- =============================================================================

DROP INDEX IF EXISTS idx_permissions_name;
DROP INDEX IF EXISTS idx_permissions_resource;
DROP INDEX IF EXISTS idx_role_permissions_role_id;
DROP INDEX IF EXISTS idx_role_permissions_permission_id;

-- =============================================================================
-- VERIFICATION
-- =============================================================================

-- Verify all PHI permissions are removed
-- SELECT COUNT(*) FROM permissions WHERE resource = 'phi';
-- Expected: 0
