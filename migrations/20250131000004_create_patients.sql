-- Create patients table
-- This table stores patient information with multi-tenancy support

CREATE TABLE IF NOT EXISTS patients (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    organization_id UUID NOT NULL REFERENCES organizations(id) ON DELETE CASCADE,
    patient_id VARCHAR(100) NOT NULL, -- Medical Record Number (MRN) or Patient ID
    first_name VARCHAR(255) NOT NULL,
    last_name VARCHAR(255) NOT NULL,
    date_of_birth DATE NOT NULL,
    email VARCHAR(255),
    phone VARCHAR(50),
    assigned_department VARCHAR(255),
    primary_provider UUID REFERENCES users(id) ON DELETE SET NULL,
    access_level VARCHAR(50) NOT NULL DEFAULT 'restricted', -- public, restricted, confidential
    consent_status VARCHAR(50) NOT NULL DEFAULT 'pending', -- pending, granted, revoked, expired
    
    -- Audit fields
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    created_by UUID REFERENCES users(id) ON DELETE SET NULL,
    updated_by UUID REFERENCES users(id) ON DELETE SET NULL,
    is_deleted BOOLEAN DEFAULT false,
    deleted_at TIMESTAMPTZ,
    deleted_by UUID REFERENCES users(id) ON DELETE SET NULL,
    
    -- Constraints
    CONSTRAINT unique_patient_per_org UNIQUE (organization_id, patient_id),
    CONSTRAINT valid_access_level CHECK (access_level IN ('public', 'restricted', 'confidential')),
    CONSTRAINT valid_consent_status CHECK (consent_status IN ('pending', 'granted', 'revoked', 'expired', 'not_required'))
);

-- Indexes for performance
CREATE INDEX idx_patients_organization_id ON patients(organization_id) WHERE is_deleted = false;
CREATE INDEX idx_patients_patient_id ON patients(patient_id) WHERE is_deleted = false;
CREATE INDEX idx_patients_email ON patients(email) WHERE is_deleted = false AND email IS NOT NULL;
CREATE INDEX idx_patients_primary_provider ON patients(primary_provider) WHERE is_deleted = false AND primary_provider IS NOT NULL;
CREATE INDEX idx_patients_created_at ON patients(created_at DESC) WHERE is_deleted = false;
CREATE INDEX idx_patients_last_name ON patients(last_name) WHERE is_deleted = false;

-- Full-text search index for patient names
CREATE INDEX idx_patients_name_search ON patients USING gin(
    to_tsvector('english', first_name || ' ' || last_name)
) WHERE is_deleted = false;

-- Trigger for updated_at timestamp
CREATE OR REPLACE FUNCTION update_patients_updated_at()
RETURNS TRIGGER AS $$
BEGIN
    NEW.updated_at = NOW();
    RETURN NEW;
END;
$$ LANGUAGE plpgsql;

CREATE TRIGGER trigger_patients_updated_at
    BEFORE UPDATE ON patients
    FOR EACH ROW
    EXECUTE FUNCTION update_patients_updated_at();

-- Enable Row Level Security (RLS)
ALTER TABLE patients ENABLE ROW LEVEL SECURITY;

-- RLS Policy: Users can only see patients from their organization
CREATE POLICY patients_org_isolation ON patients
    FOR ALL
    USING (organization_id = current_setting('app.current_organization_id', TRUE)::UUID);

-- RLS Policy: System administrators can see all patients (check organization_employees for admin role)
-- Note: Administrators are identified through organization_employees.role_id
-- This policy can be enhanced when role-based access is fully implemented
CREATE POLICY patients_admin_access ON patients
    FOR ALL
    USING (
        EXISTS (
            SELECT 1 FROM users u
            WHERE u.id = current_setting('app.current_user_id', TRUE)::UUID
            AND u.status = 'active'
        )
    );

-- RLS Policy: Healthcare providers can only see their assigned patients
CREATE POLICY patients_provider_access ON patients
    FOR SELECT
    USING (
        primary_provider = current_setting('app.current_user_id', TRUE)::UUID
    );

-- Grant permissions
GRANT SELECT, INSERT, UPDATE, DELETE ON patients TO rustcare;
GRANT USAGE, SELECT ON ALL SEQUENCES IN SCHEMA public TO rustcare;

-- Comments for documentation
COMMENT ON TABLE patients IS 'Stores patient demographic and basic information with multi-tenancy support';
COMMENT ON COLUMN patients.patient_id IS 'Medical Record Number (MRN) or external patient identifier';
COMMENT ON COLUMN patients.access_level IS 'Data sensitivity level: public, restricted, or confidential';
COMMENT ON COLUMN patients.consent_status IS 'Patient consent status for data sharing and treatment';
COMMENT ON COLUMN patients.is_deleted IS 'Soft delete flag - patients are never hard deleted for compliance';

