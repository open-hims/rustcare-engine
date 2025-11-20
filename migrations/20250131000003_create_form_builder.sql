-- Dynamic Form Builder System
-- PostgreSQL Migration: Create form definitions and submissions tables
-- Version: 003
-- Description: Support for dynamic form definitions and submissions across all modules

-- =============================================================================
-- FORM DEFINITIONS TABLE
-- =============================================================================
-- Stores form definitions that can be used across any module
CREATE TABLE IF NOT EXISTS form_definitions (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    organization_id UUID NOT NULL REFERENCES organizations(id) ON DELETE CASCADE,
    
    -- Form identification
    form_name VARCHAR(255) NOT NULL,
    form_slug VARCHAR(100) NOT NULL,
    display_name VARCHAR(255) NOT NULL,
    description TEXT,
    
    -- Module association
    module_name VARCHAR(100) NOT NULL, -- 'healthcare', 'pharmacy', 'billing', 'compliance', etc.
    entity_type VARCHAR(100), -- 'patient', 'appointment', 'prescription', 'claim', etc.
    
    -- Form configuration
    form_schema JSONB NOT NULL, -- Complete form field definitions
    form_layout JSONB, -- Layout configuration (columns, sections, etc.)
    validation_rules JSONB, -- Cross-field validation rules
    submission_handler VARCHAR(255), -- Handler function/endpoint for submissions
    
    -- Form settings
    is_active BOOLEAN NOT NULL DEFAULT TRUE,
    is_template BOOLEAN NOT NULL DEFAULT FALSE, -- Can be used as template
    allow_multiple_submissions BOOLEAN NOT NULL DEFAULT TRUE,
    require_approval BOOLEAN NOT NULL DEFAULT FALSE,
    
    -- Access control
    requires_permission VARCHAR(255),
    required_roles TEXT[],
    allowed_roles TEXT[],
    
    -- Versioning
    version INTEGER NOT NULL DEFAULT 1,
    parent_form_id UUID REFERENCES form_definitions(id), -- For versioning
    
    -- Metadata
    tags TEXT[],
    category VARCHAR(100),
    icon VARCHAR(100),
    
    -- Audit timestamps
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    created_by UUID REFERENCES users(id),
    deleted_at TIMESTAMPTZ, -- Soft delete
    
    -- Constraints
    CONSTRAINT form_slug_unique_per_org UNIQUE (organization_id, form_slug, deleted_at),
    CONSTRAINT form_name_not_empty CHECK (form_name != ''),
    CONSTRAINT form_slug_format CHECK (form_slug ~ '^[a-z0-9-]+$')
);

-- Indexes for form definitions
CREATE INDEX idx_form_definitions_org ON form_definitions(organization_id) WHERE deleted_at IS NULL;
CREATE INDEX idx_form_definitions_module ON form_definitions(module_name, organization_id) WHERE deleted_at IS NULL;
CREATE INDEX idx_form_definitions_entity ON form_definitions(entity_type, organization_id) WHERE deleted_at IS NULL;
CREATE INDEX idx_form_definitions_active ON form_definitions(is_active, organization_id) WHERE deleted_at IS NULL;
CREATE INDEX idx_form_definitions_template ON form_definitions(is_template, organization_id) WHERE deleted_at IS NULL;
CREATE INDEX idx_form_definitions_slug ON form_definitions(form_slug, organization_id) WHERE deleted_at IS NULL;

-- =============================================================================
-- FORM SUBMISSIONS TABLE
-- =============================================================================
-- Stores form submissions with data and metadata
CREATE TABLE IF NOT EXISTS form_submissions (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    organization_id UUID NOT NULL REFERENCES organizations(id) ON DELETE CASCADE,
    form_definition_id UUID NOT NULL REFERENCES form_definitions(id) ON DELETE RESTRICT,
    
    -- Submission data
    submission_data JSONB NOT NULL, -- Actual form data
    submission_status VARCHAR(50) NOT NULL DEFAULT 'draft' CHECK (submission_status IN ('draft', 'submitted', 'approved', 'rejected', 'cancelled')),
    
    -- Entity association (optional - links to specific entity)
    entity_type VARCHAR(100), -- 'patient', 'appointment', etc.
    entity_id UUID, -- ID of the associated entity
    
    -- Approval workflow
    submitted_by UUID REFERENCES users(id),
    submitted_at TIMESTAMPTZ,
    approved_by UUID REFERENCES users(id),
    approved_at TIMESTAMPTZ,
    rejected_by UUID REFERENCES users(id),
    rejected_at TIMESTAMPTZ,
    rejection_reason TEXT,
    
    -- Version tracking
    form_version INTEGER NOT NULL, -- Version of form definition used
    
    -- Metadata
    ip_address INET,
    user_agent TEXT,
    notes TEXT,
    
    -- Audit timestamps
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    deleted_at TIMESTAMPTZ, -- Soft delete for compliance
    deleted_by UUID REFERENCES users(id),
    
    -- Constraints
    CONSTRAINT entity_association CHECK (
        (entity_type IS NULL AND entity_id IS NULL) OR
        (entity_type IS NOT NULL AND entity_id IS NOT NULL)
    )
);

-- Indexes for form submissions
CREATE INDEX idx_form_submissions_org ON form_submissions(organization_id) WHERE deleted_at IS NULL;
CREATE INDEX idx_form_submissions_form ON form_submissions(form_definition_id, organization_id) WHERE deleted_at IS NULL;
CREATE INDEX idx_form_submissions_status ON form_submissions(submission_status, organization_id) WHERE deleted_at IS NULL;
CREATE INDEX idx_form_submissions_entity ON form_submissions(entity_type, entity_id, organization_id) WHERE deleted_at IS NULL;
CREATE INDEX idx_form_submissions_submitted_by ON form_submissions(submitted_by, organization_id) WHERE deleted_at IS NULL;
CREATE INDEX idx_form_submissions_created_at ON form_submissions(created_at DESC) WHERE deleted_at IS NULL;

-- =============================================================================
-- FORM FIELD VALIDATIONS TABLE
-- =============================================================================
-- Stores reusable validation rules for form fields
CREATE TABLE IF NOT EXISTS form_field_validations (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    organization_id UUID NOT NULL REFERENCES organizations(id) ON DELETE CASCADE,
    
    -- Validation rule identification
    rule_name VARCHAR(255) NOT NULL,
    rule_type VARCHAR(100) NOT NULL, -- 'regex', 'range', 'custom', etc.
    
    -- Validation configuration
    rule_config JSONB NOT NULL, -- Rule-specific configuration
    error_message TEXT NOT NULL,
    
    -- Applicability
    field_types TEXT[], -- Field types this rule applies to
    modules TEXT[], -- Modules this rule applies to
    
    -- Metadata
    description TEXT,
    is_active BOOLEAN NOT NULL DEFAULT TRUE,
    
    -- Audit timestamps
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    created_by UUID REFERENCES users(id),
    deleted_at TIMESTAMPTZ,
    
    -- Constraints
    CONSTRAINT rule_name_unique_per_org UNIQUE (organization_id, rule_name, deleted_at)
);

-- Indexes for form field validations
CREATE INDEX idx_form_validations_org ON form_field_validations(organization_id) WHERE deleted_at IS NULL;
CREATE INDEX idx_form_validations_type ON form_field_validations(rule_type, organization_id) WHERE deleted_at IS NULL;
CREATE INDEX idx_form_validations_active ON form_field_validations(is_active, organization_id) WHERE deleted_at IS NULL;

-- =============================================================================
-- COMMENTS
-- =============================================================================
COMMENT ON TABLE form_definitions IS 'Dynamic form definitions that can be used across any module';
COMMENT ON COLUMN form_definitions.form_schema IS 'JSON schema defining form fields, types, validation, etc.';
COMMENT ON COLUMN form_definitions.form_layout IS 'Layout configuration for form rendering (sections, columns, etc.)';
COMMENT ON COLUMN form_definitions.module_name IS 'Module this form belongs to (healthcare, pharmacy, billing, etc.)';
COMMENT ON COLUMN form_definitions.entity_type IS 'Entity type this form is associated with (patient, appointment, etc.)';

COMMENT ON TABLE form_submissions IS 'Form submission data with workflow support';
COMMENT ON COLUMN form_submissions.submission_data IS 'Actual form data submitted by user';
COMMENT ON COLUMN form_submissions.entity_id IS 'Optional link to specific entity (e.g., patient_id, appointment_id)';

COMMENT ON TABLE form_field_validations IS 'Reusable validation rules for form fields';

-- =============================================================================
-- RLS POLICIES
-- =============================================================================
-- Enable RLS on form tables
ALTER TABLE form_definitions ENABLE ROW LEVEL SECURITY;
ALTER TABLE form_submissions ENABLE ROW LEVEL SECURITY;
ALTER TABLE form_field_validations ENABLE ROW LEVEL SECURITY;

-- Form definitions RLS policies
CREATE POLICY form_definitions_org_isolation ON form_definitions
    FOR ALL
    USING (organization_id = current_setting('app.current_organization_id', true)::UUID);

-- Form submissions RLS policies
CREATE POLICY form_submissions_org_isolation ON form_submissions
    FOR ALL
    USING (organization_id = current_setting('app.current_organization_id', true)::UUID);

-- Form validations RLS policies
CREATE POLICY form_validations_org_isolation ON form_field_validations
    FOR ALL
    USING (organization_id = current_setting('app.current_organization_id', true)::UUID);

