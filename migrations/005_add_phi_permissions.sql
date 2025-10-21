    -- Migration: Add PHI (Protected Health Information) Access Permissions
-- Description: Implements HIPAA-compliant field-level access control
-- Author: RustCare Team
-- Date: 2025-10-21

-- =============================================================================
-- CREATE PHI ACCESS PERMISSIONS
-- =============================================================================

-- These permissions control access to sensitive healthcare data based on
-- HIPAA sensitivity levels defined in the masking engine

-- Public level - No restrictions (non-sensitive public data)
INSERT INTO permissions (id, name, resource, action, description, created_at)
VALUES (
    gen_random_uuid(),
    'phi:view:public',
    'phi',
    'view',
    'View public non-sensitive data (no PHI)',
    NOW()
) ON CONFLICT (name) DO NOTHING;

-- Internal level - Basic employee access (contact info, non-identifying)
INSERT INTO permissions (id, name, resource, action, description, created_at)
VALUES (
    gen_random_uuid(),
    'phi:view:internal',
    'phi',
    'view',
    'View internal data (email, phone - non-identifying contact info)',
    NOW()
) ON CONFLICT (name) DO NOTHING;

-- Confidential level - Restricted employee access (demographics)
INSERT INTO permissions (id, name, resource, action, description, created_at)
VALUES (
    gen_random_uuid(),
    'phi:view:confidential',
    'phi',
    'view',
    'View confidential data (address, date of birth, demographics)',
    NOW()
) ON CONFLICT (name) DO NOTHING;

-- Restricted level - Highly restricted access (identifiers, financial)
INSERT INTO permissions (id, name, resource, action, description, created_at)
VALUES (
    gen_random_uuid(),
    'phi:view:restricted',
    'phi',
    'view',
    'View restricted identifiers (SSN, MRN, insurance numbers, financial data)',
    NOW()
) ON CONFLICT (name) DO NOTHING;

-- ePHI level - Protected Health Information (clinical data)
INSERT INTO permissions (id, name, resource, action, description, created_at)
VALUES (
    gen_random_uuid(),
    'phi:view:ephi',
    'phi',
    'view',
    'View Protected Health Information (diagnosis, medications, lab results, treatment notes)',
    NOW()
) ON CONFLICT (name) DO NOTHING;

-- Unmasked level - Full unmasked access (compliance, security, admin)
INSERT INTO permissions (id, name, resource, action, description, created_at)
VALUES (
    gen_random_uuid(),
    'phi:view:unmasked',
    'phi',
    'view',
    'View all data unmasked (for compliance officers, security team, system administrators)',
    NOW()
) ON CONFLICT (name) DO NOTHING;

-- =============================================================================
-- CREATE DEFAULT ROLES IF NOT EXISTS
-- =============================================================================

-- Doctor role (full clinical access)
INSERT INTO roles (id, name, description, created_at)
VALUES (
    gen_random_uuid(),
    'doctor',
    'Physician with full clinical access to patient records',
    NOW()
) ON CONFLICT (name) DO NOTHING;

-- Nurse role (clinical access, limited to assigned patients)
INSERT INTO roles (id, name, description, created_at)
VALUES (
    gen_random_uuid(),
    'nurse',
    'Nurse with clinical access to assigned patients',
    NOW()
) ON CONFLICT (name) DO NOTHING;

-- Receptionist role (administrative access, no clinical data)
INSERT INTO roles (id, name, description, created_at)
VALUES (
    gen_random_uuid(),
    'receptionist',
    'Front desk staff with administrative access only',
    NOW()
) ON CONFLICT (name) DO NOTHING;

-- Medical Records role (access to historical records)
INSERT INTO roles (id, name, description, created_at)
VALUES (
    gen_random_uuid(),
    'medical_records',
    'Medical records staff with access to patient documentation',
    NOW()
) ON CONFLICT (name) DO NOTHING;

-- Billing role (financial data access)
INSERT INTO roles (id, name, description, created_at)
VALUES (
    gen_random_uuid(),
    'billing',
    'Billing staff with access to financial and insurance data',
    NOW()
) ON CONFLICT (name) DO NOTHING;

-- Admin role (system administration)
INSERT INTO roles (id, name, description, created_at)
VALUES (
    gen_random_uuid(),
    'admin',
    'System administrator with full access',
    NOW()
) ON CONFLICT (name) DO NOTHING;

-- Compliance Officer role (audit and compliance)
INSERT INTO roles (id, name, description, created_at)
VALUES (
    gen_random_uuid(),
    'compliance_officer',
    'Compliance officer with unmasked access for auditing',
    NOW()
) ON CONFLICT (name) DO NOTHING;

-- =============================================================================
-- ASSIGN PERMISSIONS TO ROLES
-- =============================================================================

-- Doctor: Full clinical access (internal + confidential + restricted + ePHI)
INSERT INTO role_permissions (role_id, permission_id, created_at)
SELECT r.id, p.id, NOW()
FROM roles r, permissions p
WHERE r.name = 'doctor' AND p.name IN (
    'phi:view:public',
    'phi:view:internal',
    'phi:view:confidential',
    'phi:view:restricted',
    'phi:view:ephi'
)
ON CONFLICT DO NOTHING;

-- Nurse: Clinical access (internal + confidential + ePHI, no restricted identifiers)
INSERT INTO role_permissions (role_id, permission_id, created_at)
SELECT r.id, p.id, NOW()
FROM roles r, permissions p
WHERE r.name = 'nurse' AND p.name IN (
    'phi:view:public',
    'phi:view:internal',
    'phi:view:confidential',
    'phi:view:ephi'
)
ON CONFLICT DO NOTHING;

-- Receptionist: Administrative access only (public + internal, no clinical data)
INSERT INTO role_permissions (role_id, permission_id, created_at)
SELECT r.id, p.id, NOW()
FROM roles r, permissions p
WHERE r.name = 'receptionist' AND p.name IN (
    'phi:view:public',
    'phi:view:internal'
)
ON CONFLICT DO NOTHING;

-- Medical Records: Documentation access (public + internal + confidential + ePHI)
INSERT INTO role_permissions (role_id, permission_id, created_at)
SELECT r.id, p.id, NOW()
FROM roles r, permissions p
WHERE r.name = 'medical_records' AND p.name IN (
    'phi:view:public',
    'phi:view:internal',
    'phi:view:confidential',
    'phi:view:ephi'
)
ON CONFLICT DO NOTHING;

-- Billing: Financial access (public + internal + restricted for insurance/billing)
INSERT INTO role_permissions (role_id, permission_id, created_at)
SELECT r.id, p.id, NOW()
FROM roles r, permissions p
WHERE r.name = 'billing' AND p.name IN (
    'phi:view:public',
    'phi:view:internal',
    'phi:view:restricted'
)
ON CONFLICT DO NOTHING;

-- Admin: Full system access (all levels including unmasked)
INSERT INTO role_permissions (role_id, permission_id, created_at)
SELECT r.id, p.id, NOW()
FROM roles r, permissions p
WHERE r.name = 'admin' AND p.name IN (
    'phi:view:public',
    'phi:view:internal',
    'phi:view:confidential',
    'phi:view:restricted',
    'phi:view:ephi',
    'phi:view:unmasked'
)
ON CONFLICT DO NOTHING;

-- Compliance Officer: Unmasked access for auditing and compliance
INSERT INTO role_permissions (role_id, permission_id, created_at)
SELECT r.id, p.id, NOW()
FROM roles r, permissions p
WHERE r.name = 'compliance_officer' AND p.name IN (
    'phi:view:public',
    'phi:view:internal',
    'phi:view:confidential',
    'phi:view:restricted',
    'phi:view:ephi',
    'phi:view:unmasked'
)
ON CONFLICT DO NOTHING;

-- =============================================================================
-- CREATE INDEXES FOR PERFORMANCE
-- =============================================================================

-- Index for faster permission lookups by name
CREATE INDEX IF NOT EXISTS idx_permissions_name ON permissions(name);
CREATE INDEX IF NOT EXISTS idx_permissions_resource ON permissions(resource);

-- Index for role-permission joins
CREATE INDEX IF NOT EXISTS idx_role_permissions_role_id ON role_permissions(role_id);
CREATE INDEX IF NOT EXISTS idx_role_permissions_permission_id ON role_permissions(permission_id);

-- =============================================================================
-- VERIFICATION QUERIES
-- =============================================================================

-- View all PHI permissions
-- SELECT * FROM permissions WHERE resource = 'phi' ORDER BY name;

-- View role assignments
-- SELECT r.name as role, p.name as permission
-- FROM roles r
-- JOIN role_permissions rp ON r.id = rp.role_id
-- JOIN permissions p ON rp.permission_id = p.id
-- WHERE p.resource = 'phi'
-- ORDER BY r.name, p.name;

-- Check doctor permissions
-- SELECT p.name FROM permissions p
-- JOIN role_permissions rp ON p.id = rp.permission_id
-- JOIN roles r ON rp.role_id = r.id
-- WHERE r.name = 'doctor' AND p.resource = 'phi';

-- =============================================================================
-- USAGE EXAMPLE IN APPLICATION
-- =============================================================================

-- In your application, retrieve user permissions like this:
--
-- SELECT ARRAY_AGG(p.name) as permissions
-- FROM users u
-- JOIN user_roles ur ON u.id = ur.user_id
-- JOIN roles r ON ur.role_id = r.id
-- JOIN role_permissions rp ON r.id = rp.role_id
-- JOIN permissions p ON rp.permission_id = p.id
-- WHERE u.id = $1
-- GROUP BY u.id;
--
-- Then use MaskingEngine::can_view_unmasked() to check permissions:
-- if engine.can_view_unmasked("diagnosis", &user_permissions) {
--     // Show unmasked value
-- } else {
--     // Show "[REDACTED]"
-- }

-- =============================================================================
-- HIPAA COMPLIANCE NOTES
-- =============================================================================

-- This permission structure supports HIPAA's "Minimum Necessary" standard:
-- - Each role only gets the minimum access needed for their job function
-- - Receptionists cannot view clinical data (diagnosis, medications)
-- - Nurses cannot view SSN/financial data unless specifically granted
-- - All access to ePHI is logged via AuditLogger::log_phi_access()
-- - Unmasked access is restricted to compliance and administrative staff

-- HIPAA Safe Harbor compliance:
-- - 18 HIPAA identifiers are classified as Restricted or ePHI
-- - Automatic masking applied unless user has appropriate permission
-- - All access attempts are audited with field-level granularity
