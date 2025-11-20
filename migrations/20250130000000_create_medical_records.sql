-- Create Medical Records and Healthcare Tables
-- HIPAA-compliant EMR system implementation

CREATE EXTENSION IF NOT EXISTS "uuid-ossp";
CREATE EXTENSION IF NOT EXISTS "pgcrypto"; -- For encryption functions

-- ============================================================================
-- MEDICAL RECORDS
-- ============================================================================

-- Table: medical_records
CREATE TABLE IF NOT EXISTS medical_records (
    id UUID PRIMARY KEY DEFAULT uuid_generate_v4(),
    organization_id UUID NOT NULL REFERENCES organizations(id) ON DELETE CASCADE,
    patient_id UUID NOT NULL, -- Will reference patients table when created
    provider_id UUID NOT NULL REFERENCES users(id),
    
    -- Record Metadata
    record_type VARCHAR(50) NOT NULL CHECK (record_type IN (
        'consultation', 'diagnostic', 'treatment', 'prescription', 
        'lab', 'imaging', 'vital_signs', 'procedure', 'emergency'
    )),
    title VARCHAR(500) NOT NULL,
    description TEXT,
    
    -- Clinical Data (JSONB for flexible structure)
    chief_complaint TEXT,
    diagnosis JSONB DEFAULT '{}', -- ICD-10 codes, severity, etc.
    treatments JSONB DEFAULT '{}',
    prescriptions JSONB DEFAULT '{}',
    test_results JSONB DEFAULT '{}',
    vital_signs JSONB DEFAULT '{}',
    
    -- Visit Information
    visit_date TIMESTAMP NOT NULL,
    visit_duration_minutes INTEGER,
    location VARCHAR(200), -- Department, room, etc.
    
    -- HIPAA Compliance Fields
    access_level VARCHAR(20) DEFAULT 'restricted' CHECK (access_level IN ('public', 'restricted', 'confidential')),
    phi_present BOOLEAN DEFAULT true,
    encryption_key_id UUID, -- Reference to encryption_keys table
    
    -- Audit Trail
    created_by UUID NOT NULL REFERENCES users(id),
    updated_by UUID REFERENCES users(id),
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    deleted_at TIMESTAMPTZ,
    
    -- Soft delete support
    is_deleted BOOLEAN DEFAULT false
);

CREATE INDEX idx_medical_records_patient ON medical_records(patient_id);
CREATE INDEX idx_medical_records_provider ON medical_records(provider_id);
CREATE INDEX idx_medical_records_org ON medical_records(organization_id);
CREATE INDEX idx_medical_records_visit_date ON medical_records(visit_date DESC);
CREATE INDEX idx_medical_records_type ON medical_records(record_type);
CREATE INDEX idx_medical_records_deleted ON medical_records(is_deleted) WHERE is_deleted = false;

-- GIN index for JSONB fields
CREATE INDEX idx_medical_records_diagnosis ON medical_records USING GIN(diagnosis);
CREATE INDEX idx_medical_records_treatments ON medical_records USING GIN(treatments);
CREATE INDEX idx_medical_records_prescriptions ON medical_records USING GIN(prescriptions);

-- ============================================================================
-- MEDICAL RECORD AUDIT LOG
-- ============================================================================

-- Table: medical_record_audit_log
CREATE TABLE IF NOT EXISTS medical_record_audit_log (
    id UUID PRIMARY KEY DEFAULT uuid_generate_v4(),
    medical_record_id UUID NOT NULL REFERENCES medical_records(id) ON DELETE CASCADE,
    
    -- Access Information
    accessed_by UUID NOT NULL REFERENCES users(id),
    access_type VARCHAR(50) NOT NULL CHECK (access_type IN (
        'view', 'create', 'update', 'delete', 'print', 'export', 'share'
    )),
    access_time TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    
    -- Context
    ip_address INET,
    user_agent TEXT,
    access_reason TEXT,
    emergency_access BOOLEAN DEFAULT false,
    
    -- Audit Data
    before_state JSONB DEFAULT '{}',
    after_state JSONB DEFAULT '{}',
    fields_accessed TEXT[], -- Which specific fields were accessed
    
    -- Result
    success BOOLEAN DEFAULT true,
    denied_reason TEXT
);

CREATE INDEX idx_audit_record_id ON medical_record_audit_log(medical_record_id);
CREATE INDEX idx_audit_user ON medical_record_audit_log(accessed_by);
CREATE INDEX idx_audit_time ON medical_record_audit_log(access_time DESC);
CREATE INDEX idx_audit_type ON medical_record_audit_log(access_type);

-- Retention policy: Keep audit logs for 7 years
-- Note: Indexing for retention cleanup should be handled differently as NOW() is not immutable
-- CREATE INDEX idx_audit_retention ON medical_record_audit_log(access_time);

-- ============================================================================
-- PROVIDERS (Doctors, Nurses, etc.)
-- ============================================================================

-- Table: healthcare_providers
CREATE TABLE IF NOT EXISTS healthcare_providers (
    id UUID PRIMARY KEY DEFAULT uuid_generate_v4(),
    user_id UUID NOT NULL UNIQUE REFERENCES users(id) ON DELETE CASCADE,
    organization_id UUID NOT NULL REFERENCES organizations(id) ON DELETE CASCADE,
    
    -- Professional Information
    license_number VARCHAR(100) NOT NULL,
    license_state VARCHAR(2) NOT NULL,
    license_expiry DATE NOT NULL,
    specialty VARCHAR(100),
    npi_number VARCHAR(10) UNIQUE, -- National Provider Identifier
    
    -- Department Assignment
    department VARCHAR(100),
    sub_department VARCHAR(100),
    
    -- Status
    is_active BOOLEAN DEFAULT true,
    start_date DATE NOT NULL,
    end_date DATE,
    
    -- Metadata
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE INDEX idx_providers_user ON healthcare_providers(user_id);
CREATE INDEX idx_providers_org ON healthcare_providers(organization_id);
CREATE INDEX idx_providers_active ON healthcare_providers(is_active) WHERE is_active = true;
CREATE UNIQUE INDEX idx_providers_license ON healthcare_providers(license_number, license_state);

-- Appointments moved to 20250131000000_create_appointments_visits.sql

-- ============================================================================
-- VITAL SIGNS
-- ============================================================================

-- Table: vital_signs
CREATE TABLE IF NOT EXISTS vital_signs (
    id UUID PRIMARY KEY DEFAULT uuid_generate_v4(),
    organization_id UUID NOT NULL REFERENCES organizations(id) ON DELETE CASCADE,
    patient_id UUID NOT NULL, -- Will reference patients table
    medical_record_id UUID REFERENCES medical_records(id) ON DELETE SET NULL,
    provider_id UUID NOT NULL REFERENCES healthcare_providers(id) ON DELETE CASCADE,
    
    -- Measurements
    systolic_bp INTEGER,
    diastolic_bp INTEGER,
    heart_rate INTEGER,
    temperature_celsius DECIMAL(4,2),
    weight_kg DECIMAL(5,2),
    height_cm DECIMAL(5,2),
    oxygen_saturation DECIMAL(5,2),
    respiratory_rate INTEGER,
    
    -- Context
    recorded_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    notes TEXT,
    
    -- Audit
    recorded_by UUID NOT NULL REFERENCES users(id),
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE INDEX idx_vitals_patient ON vital_signs(patient_id);
CREATE INDEX idx_vitals_recorded ON vital_signs(recorded_at DESC);
CREATE INDEX idx_vitals_provider ON vital_signs(provider_id);

-- ============================================================================
-- ROW LEVEL SECURITY POLICIES
-- ============================================================================

-- Enable RLS on all tables
ALTER TABLE medical_records ENABLE ROW LEVEL SECURITY;
ALTER TABLE medical_record_audit_log ENABLE ROW LEVEL SECURITY;
ALTER TABLE healthcare_providers ENABLE ROW LEVEL SECURITY;
-- ALTER TABLE appointments ENABLE ROW LEVEL SECURITY;
ALTER TABLE vital_signs ENABLE ROW LEVEL SECURITY;

-- Medical Records RLS Policies
CREATE POLICY medical_records_select_policy ON medical_records
    FOR SELECT
    USING (
        -- Only non-deleted records
        is_deleted = false
        AND
        -- Provider can view own records
        provider_id = current_setting('app.current_user_id')::UUID
        OR
        -- Doctors can view records in their organization
        EXISTS (
            SELECT 1 FROM healthcare_providers hp
            WHERE hp.user_id = current_setting('app.current_user_id')::UUID
            AND hp.organization_id = medical_records.organization_id
            AND hp.is_active = true
        )
        OR
        -- Emergency access (must be logged)
        current_setting('app.is_emergency_access', true)::BOOLEAN = true
    );

CREATE POLICY medical_records_insert_policy ON medical_records
    FOR INSERT
    WITH CHECK (
        -- Only active providers can create records
        EXISTS (
            SELECT 1 FROM healthcare_providers hp
            WHERE hp.user_id = current_setting('app.current_user_id')::UUID
            AND hp.is_active = true
            AND hp.organization_id = medical_records.organization_id
        )
    );

CREATE POLICY medical_records_update_policy ON medical_records
    FOR UPDATE
    USING (
        -- Only creator or supervising doctor can update
        created_by = current_setting('app.current_user_id')::UUID
        OR
        EXISTS (
            SELECT 1 FROM healthcare_providers hp
            WHERE hp.user_id = current_setting('app.current_user_id')::UUID
            AND hp.organization_id = medical_records.organization_id
            AND hp.specialty = 'Administrator'
            AND hp.is_active = true
        )
    );

-- ============================================================================
-- TRIGGERS
-- ============================================================================

-- Update updated_at timestamp
CREATE OR REPLACE FUNCTION update_updated_at_column()
RETURNS TRIGGER AS $$
BEGIN
    NEW.updated_at = NOW();
    RETURN NEW;
END;
$$ LANGUAGE plpgsql;

CREATE TRIGGER update_medical_records_updated_at
    BEFORE UPDATE ON medical_records
    FOR EACH ROW
    EXECUTE FUNCTION update_updated_at_column();

CREATE TRIGGER update_providers_updated_at
    BEFORE UPDATE ON healthcare_providers
    FOR EACH ROW
    EXECUTE FUNCTION update_updated_at_column();

-- CREATE TRIGGER update_appointments_updated_at
--     BEFORE UPDATE ON appointments
--     FOR EACH ROW
--     EXECUTE FUNCTION update_updated_at_column();

-- Automatic audit logging
CREATE OR REPLACE FUNCTION log_medical_record_changes()
RETURNS TRIGGER AS $$
BEGIN
    IF TG_OP = 'INSERT' THEN
        INSERT INTO medical_record_audit_log (
            medical_record_id, accessed_by, access_type,
            after_state, success
        ) VALUES (
            NEW.id, NEW.created_by, 'create',
            to_jsonb(NEW), true
        );
        RETURN NEW;
    ELSIF TG_OP = 'UPDATE' THEN
        INSERT INTO medical_record_audit_log (
            medical_record_id, accessed_by, access_type,
            before_state, after_state, success
        ) VALUES (
            NEW.id, NEW.updated_by, 'update',
            to_jsonb(OLD), to_jsonb(NEW), true
        );
        RETURN NEW;
    ELSIF TG_OP = 'DELETE' THEN
        INSERT INTO medical_record_audit_log (
            medical_record_id, accessed_by, access_type,
            before_state, success
        ) VALUES (
            OLD.id, current_setting('app.current_user_id')::UUID, 'delete',
            to_jsonb(OLD), true
        );
        RETURN OLD;
    END IF;
END;
$$ LANGUAGE plpgsql;

CREATE TRIGGER medical_record_audit_trigger
    AFTER INSERT OR UPDATE OR DELETE ON medical_records
    FOR EACH ROW
    EXECUTE FUNCTION log_medical_record_changes();

-- ============================================================================
-- COMMENTS
-- ============================================================================

COMMENT ON TABLE medical_records IS 'Electronic Medical Records with HIPAA-compliant security';
COMMENT ON TABLE medical_record_audit_log IS 'Complete audit trail for all medical record access';
COMMENT ON TABLE healthcare_providers IS 'Healthcare providers (doctors, nurses, etc.)';
-- COMMENT ON TABLE appointments IS 'Patient appointment scheduling';
COMMENT ON TABLE vital_signs IS 'Patient vital signs measurements';

