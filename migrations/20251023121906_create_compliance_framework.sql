-- Create Compliance Framework Tables (separate from geographic)
-- This migration creates compliance frameworks, rules, and entity compliance tracking

-- Ensure required extensions are enabled
CREATE EXTENSION IF NOT EXISTS "uuid-ossp";

-- ============================================================================
-- COMPLIANCE FRAMEWORKS
-- ============================================================================

-- Table: compliance_frameworks
CREATE TABLE IF NOT EXISTS compliance_frameworks (
    id UUID PRIMARY KEY DEFAULT uuid_generate_v4(),
    organization_id UUID NOT NULL REFERENCES organizations(id) ON DELETE CASCADE,
    name VARCHAR(255) NOT NULL,
    code VARCHAR(50) NOT NULL,
    version VARCHAR(20) NOT NULL DEFAULT '1.0',
    description TEXT,
    authority VARCHAR(255), -- Regulatory authority (e.g., "HHS", "EU Commission")
    jurisdiction VARCHAR(100), -- Geographic jurisdiction reference (will link to geographic table)
    effective_date DATE NOT NULL,
    review_date DATE,
    status VARCHAR(50) NOT NULL DEFAULT 'active' CHECK (status IN ('draft', 'active', 'deprecated', 'superseded')),
    parent_framework_id UUID REFERENCES compliance_frameworks(id),
    metadata JSONB DEFAULT '{}',
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    created_by UUID REFERENCES users(id),
    updated_by UUID REFERENCES users(id),
    UNIQUE(organization_id, code, version)
);

CREATE INDEX idx_frameworks_org ON compliance_frameworks(organization_id);
CREATE INDEX idx_frameworks_status ON compliance_frameworks(status);
CREATE INDEX idx_frameworks_jurisdiction ON compliance_frameworks(jurisdiction);
CREATE INDEX idx_frameworks_effective_date ON compliance_frameworks(effective_date);

-- ============================================================================
-- COMPLIANCE RULES
-- ============================================================================

-- Table: compliance_rules
CREATE TABLE IF NOT EXISTS compliance_rules (
    id UUID PRIMARY KEY DEFAULT uuid_generate_v4(),
    organization_id UUID NOT NULL REFERENCES organizations(id) ON DELETE CASCADE,
    framework_id UUID NOT NULL REFERENCES compliance_frameworks(id) ON DELETE CASCADE,
    rule_code VARCHAR(100) NOT NULL,
    title VARCHAR(500) NOT NULL,
    description TEXT,
    category VARCHAR(100), -- e.g., "data_protection", "patient_safety", "documentation"
    severity VARCHAR(50) NOT NULL DEFAULT 'medium' CHECK (severity IN ('low', 'medium', 'high', 'critical')),
    rule_type VARCHAR(50) NOT NULL CHECK (rule_type IN ('mandatory', 'recommended', 'best_practice')),
    
    -- Rule definition
    applies_to_entity_types TEXT[], -- e.g., ['patient_record', 'lab_report', 'medication']
    applies_to_roles TEXT[], -- Role-based application
    applies_to_regions TEXT[], -- Geographic regions (references to geographic table)
    
    -- Implementation details
    validation_logic JSONB, -- Structured validation rules
    remediation_steps TEXT,
    documentation_requirements TEXT[],
    
    -- Compliance tracking
    is_automated BOOLEAN DEFAULT FALSE,
    automation_script TEXT,
    check_frequency_days INTEGER,
    last_checked_at TIMESTAMPTZ,
    
    -- Status and versioning
    status VARCHAR(50) NOT NULL DEFAULT 'active' CHECK (status IN ('draft', 'active', 'deprecated')),
    version INTEGER NOT NULL DEFAULT 1,
    effective_date DATE NOT NULL,
    expiry_date DATE,
    
    -- Metadata
    metadata JSONB DEFAULT '{}',
    tags TEXT[],
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    created_by UUID REFERENCES users(id),
    updated_by UUID REFERENCES users(id),
    
    UNIQUE(organization_id, framework_id, rule_code)
);

CREATE INDEX idx_rules_org ON compliance_rules(organization_id);
CREATE INDEX idx_rules_framework ON compliance_rules(framework_id);
CREATE INDEX idx_rules_category ON compliance_rules(category);
CREATE INDEX idx_rules_severity ON compliance_rules(severity);
CREATE INDEX idx_rules_status ON compliance_rules(status);
CREATE INDEX idx_rules_entity_types ON compliance_rules USING GIN(applies_to_entity_types);
CREATE INDEX idx_rules_roles ON compliance_rules USING GIN(applies_to_roles);

-- ============================================================================
-- ENTITY COMPLIANCE TRACKING
-- ============================================================================

-- Table: entity_compliance
CREATE TABLE IF NOT EXISTS entity_compliance (
    id UUID PRIMARY KEY DEFAULT uuid_generate_v4(),
    organization_id UUID NOT NULL REFERENCES organizations(id) ON DELETE CASCADE,
    
    -- Entity identification
    entity_type VARCHAR(100) NOT NULL, -- e.g., 'patient_record', 'lab_report'
    entity_id UUID NOT NULL,
    
    -- Compliance rule
    rule_id UUID NOT NULL REFERENCES compliance_rules(id) ON DELETE CASCADE,
    framework_id UUID NOT NULL REFERENCES compliance_frameworks(id) ON DELETE CASCADE,
    
    -- Compliance status
    compliance_status VARCHAR(50) NOT NULL CHECK (compliance_status IN ('compliant', 'non_compliant', 'partial', 'not_applicable', 'pending')),
    compliance_score DECIMAL(5,2) CHECK (compliance_score BETWEEN 0 AND 100),
    
    -- Assessment details
    assessed_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    assessed_by UUID REFERENCES users(id),
    assessment_method VARCHAR(100), -- 'automated', 'manual', 'hybrid'
    
    -- Findings
    findings JSONB DEFAULT '{}',
    violations TEXT[],
    evidence_documents UUID[], -- References to document storage
    
    -- Remediation
    remediation_required BOOLEAN DEFAULT FALSE,
    remediation_status VARCHAR(50) CHECK (remediation_status IN ('not_required', 'pending', 'in_progress', 'completed', 'escalated')),
    remediation_due_date DATE,
    remediation_completed_at TIMESTAMPTZ,
    remediation_notes TEXT,
    
    -- Risk assessment
    risk_level VARCHAR(50) CHECK (risk_level IN ('low', 'medium', 'high', 'critical')),
    risk_score DECIMAL(5,2) CHECK (risk_score BETWEEN 0 AND 100),
    
    -- Metadata
    metadata JSONB DEFAULT '{}',
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    
    UNIQUE(organization_id, entity_type, entity_id, rule_id)
);

CREATE INDEX idx_entity_compliance_org ON entity_compliance(organization_id);
CREATE INDEX idx_entity_compliance_entity ON entity_compliance(entity_type, entity_id);
CREATE INDEX idx_entity_compliance_rule ON entity_compliance(rule_id);
CREATE INDEX idx_entity_compliance_framework ON entity_compliance(framework_id);
CREATE INDEX idx_entity_compliance_status ON entity_compliance(compliance_status);
CREATE INDEX idx_entity_compliance_risk ON entity_compliance(risk_level);

-- ============================================================================
-- COMPLIANCE AUDIT LOG
-- ============================================================================

-- Table: compliance_audit_log
CREATE TABLE IF NOT EXISTS compliance_audit_log (
    id UUID PRIMARY KEY DEFAULT uuid_generate_v4(),
    organization_id UUID NOT NULL REFERENCES organizations(id) ON DELETE CASCADE,
    
    -- Audit details
    audit_type VARCHAR(100) NOT NULL, -- 'rule_check', 'manual_review', 'exception', 'remediation'
    entity_type VARCHAR(100),
    entity_id UUID,
    rule_id UUID REFERENCES compliance_rules(id),
    framework_id UUID REFERENCES compliance_frameworks(id),
    
    -- Actor
    performed_by UUID REFERENCES users(id),
    performed_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    
    -- Details
    action VARCHAR(255) NOT NULL,
    previous_state JSONB,
    new_state JSONB,
    findings JSONB,
    notes TEXT,
    
    -- Context
    ip_address INET,
    user_agent TEXT,
    session_id UUID,
    
    metadata JSONB DEFAULT '{}',
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE INDEX idx_compliance_audit_org ON compliance_audit_log(organization_id);
CREATE INDEX idx_compliance_audit_entity ON compliance_audit_log(entity_type, entity_id);
CREATE INDEX idx_compliance_audit_user ON compliance_audit_log(performed_by);
CREATE INDEX idx_compliance_audit_time ON compliance_audit_log(performed_at);
CREATE INDEX idx_compliance_audit_type ON compliance_audit_log(audit_type);

-- ============================================================================
-- RLS POLICIES
-- ============================================================================

-- Enable RLS
ALTER TABLE compliance_frameworks ENABLE ROW LEVEL SECURITY;
ALTER TABLE compliance_rules ENABLE ROW LEVEL SECURITY;
ALTER TABLE entity_compliance ENABLE ROW LEVEL SECURITY;
ALTER TABLE compliance_audit_log ENABLE ROW LEVEL SECURITY;

-- Compliance frameworks policies
CREATE POLICY compliance_frameworks_org_isolation ON compliance_frameworks
    FOR ALL USING (organization_id = current_setting('app.current_organization_id', TRUE)::UUID);

-- Compliance rules policies
CREATE POLICY compliance_rules_org_isolation ON compliance_rules
    FOR ALL USING (organization_id = current_setting('app.current_organization_id', TRUE)::UUID);

-- Entity compliance policies
CREATE POLICY entity_compliance_org_isolation ON entity_compliance
    FOR ALL USING (organization_id = current_setting('app.current_organization_id', TRUE)::UUID);

-- Audit log policies
CREATE POLICY compliance_audit_org_isolation ON compliance_audit_log
    FOR ALL USING (organization_id = current_setting('app.current_organization_id', TRUE)::UUID);

-- ============================================================================
-- TRIGGERS
-- ============================================================================

-- Updated_at triggers
CREATE TRIGGER update_compliance_frameworks_updated_at
    BEFORE UPDATE ON compliance_frameworks
    FOR EACH ROW
    EXECUTE FUNCTION update_updated_at_column();

CREATE TRIGGER update_compliance_rules_updated_at
    BEFORE UPDATE ON compliance_rules
    FOR EACH ROW
    EXECUTE FUNCTION update_updated_at_column();

CREATE TRIGGER update_entity_compliance_updated_at
    BEFORE UPDATE ON entity_compliance
    FOR EACH ROW
    EXECUTE FUNCTION update_updated_at_column();

-- ============================================================================
-- COMMENTS
-- ============================================================================

COMMENT ON TABLE compliance_frameworks IS 'Compliance frameworks (HIPAA, GDPR, etc.) applicable to the organization';
COMMENT ON TABLE compliance_rules IS 'Specific compliance rules within each framework';
COMMENT ON TABLE entity_compliance IS 'Compliance tracking for individual entities (patients, records, etc.)';
COMMENT ON TABLE compliance_audit_log IS 'Audit trail of all compliance-related activities';
