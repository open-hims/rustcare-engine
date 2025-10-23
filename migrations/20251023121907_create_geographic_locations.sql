-- Create Geographic Locations and Taxonomy
-- This migration creates geographic hierarchy and location-based organization structure

-- Ensure required extensions are enabled
CREATE EXTENSION IF NOT EXISTS "ltree";
CREATE EXTENSION IF NOT EXISTS "uuid-ossp";

-- ============================================================================
-- GEOGRAPHIC REGIONS
-- ============================================================================

-- Table: geographic_regions
CREATE TABLE IF NOT EXISTS geographic_regions (
    id UUID PRIMARY KEY DEFAULT uuid_generate_v4(),
    code VARCHAR(50) NOT NULL UNIQUE, -- ISO codes or custom codes (e.g., "US-CA", "EU-DE-BE")
    name VARCHAR(255) NOT NULL,
    region_type VARCHAR(50) NOT NULL CHECK (region_type IN ('country', 'state', 'province', 'city', 'district', 'zone', 'custom')),
    
    -- Hierarchy
    parent_region_id UUID REFERENCES geographic_regions(id) ON DELETE SET NULL,
    path LTREE, -- Materialized path for efficient hierarchical queries
    level INTEGER NOT NULL DEFAULT 0,
    
    -- Geographic data
    iso_country_code CHAR(2), -- ISO 3166-1 alpha-2
    iso_subdivision_code VARCHAR(10), -- ISO 3166-2
    timezone VARCHAR(50),
    coordinates POINT, -- Geographic coordinates (longitude, latitude)
    
    -- Metadata
    population BIGINT,
    area_sq_km DECIMAL(15,2),
    metadata JSONB DEFAULT '{}',
    
    -- Status
    is_active BOOLEAN DEFAULT TRUE,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    
    UNIQUE(code)
);

CREATE INDEX idx_regions_parent ON geographic_regions(parent_region_id);
CREATE INDEX idx_regions_path ON geographic_regions USING GIST(path);
CREATE INDEX idx_regions_type ON geographic_regions(region_type);
CREATE INDEX idx_regions_country ON geographic_regions(iso_country_code);
CREATE INDEX idx_regions_active ON geographic_regions(is_active);

-- ============================================================================
-- ORGANIZATION GEOGRAPHIC PRESENCE
-- ============================================================================

-- Table: organization_regions
CREATE TABLE IF NOT EXISTS organization_regions (
    id UUID PRIMARY KEY DEFAULT uuid_generate_v4(),
    organization_id UUID NOT NULL REFERENCES organizations(id) ON DELETE CASCADE,
    region_id UUID NOT NULL REFERENCES geographic_regions(id) ON DELETE CASCADE,
    
    -- Presence details
    presence_type VARCHAR(50) NOT NULL CHECK (presence_type IN ('headquarters', 'branch', 'service_area', 'licensed', 'registered')),
    is_primary BOOLEAN DEFAULT FALSE,
    
    -- Operational details
    operational_since DATE,
    operational_until DATE,
    status VARCHAR(50) DEFAULT 'active' CHECK (status IN ('active', 'inactive', 'planned', 'closed')),
    
    -- Licensing
    license_number VARCHAR(100),
    license_authority VARCHAR(255),
    license_valid_from DATE,
    license_valid_until DATE,
    
    -- Contact details for this location
    address TEXT,
    phone VARCHAR(50),
    email VARCHAR(255),
    
    -- Metadata
    metadata JSONB DEFAULT '{}',
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    
    UNIQUE(organization_id, region_id, presence_type)
);

CREATE INDEX idx_org_regions_org ON organization_regions(organization_id);
CREATE INDEX idx_org_regions_region ON organization_regions(region_id);
CREATE INDEX idx_org_regions_type ON organization_regions(presence_type);
CREATE INDEX idx_org_regions_status ON organization_regions(status);
CREATE INDEX idx_org_regions_primary ON organization_regions(is_primary) WHERE is_primary = TRUE;

-- ============================================================================
-- COMPLIANCE-GEOGRAPHIC MAPPING
-- ============================================================================

-- Table: compliance_region_mapping
CREATE TABLE IF NOT EXISTS compliance_region_mapping (
    id UUID PRIMARY KEY DEFAULT uuid_generate_v4(),
    organization_id UUID NOT NULL REFERENCES organizations(id) ON DELETE CASCADE,
    
    -- Links compliance framework to geographic regions
    framework_id UUID NOT NULL REFERENCES compliance_frameworks(id) ON DELETE CASCADE,
    region_id UUID NOT NULL REFERENCES geographic_regions(id) ON DELETE CASCADE,
    
    -- Applicability
    is_mandatory BOOLEAN DEFAULT TRUE,
    effective_date DATE NOT NULL,
    expiry_date DATE,
    
    -- Override rules specific to this region
    regional_rules JSONB DEFAULT '{}',
    exemptions TEXT[],
    special_provisions TEXT,
    
    -- Status
    status VARCHAR(50) DEFAULT 'active' CHECK (status IN ('active', 'inactive', 'pending', 'superseded')),
    
    metadata JSONB DEFAULT '{}',
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    
    UNIQUE(organization_id, framework_id, region_id)
);

CREATE INDEX idx_compliance_region_org ON compliance_region_mapping(organization_id);
CREATE INDEX idx_compliance_region_framework ON compliance_region_mapping(framework_id);
CREATE INDEX idx_compliance_region_region ON compliance_region_mapping(region_id);
CREATE INDEX idx_compliance_region_status ON compliance_region_mapping(status);

-- ============================================================================
-- RULE-REGION MAPPING
-- ============================================================================

-- Table: rule_region_applicability
CREATE TABLE IF NOT EXISTS rule_region_applicability (
    id UUID PRIMARY KEY DEFAULT uuid_generate_v4(),
    organization_id UUID NOT NULL REFERENCES organizations(id) ON DELETE CASCADE,
    
    -- Links specific rules to regions
    rule_id UUID NOT NULL REFERENCES compliance_rules(id) ON DELETE CASCADE,
    region_id UUID NOT NULL REFERENCES geographic_regions(id) ON DELETE CASCADE,
    
    -- Applicability details
    is_applicable BOOLEAN DEFAULT TRUE,
    override_severity VARCHAR(50) CHECK (override_severity IN ('low', 'medium', 'high', 'critical')),
    regional_variations JSONB DEFAULT '{}',
    
    -- Regional customization
    local_authority VARCHAR(255),
    local_requirements TEXT,
    local_exemptions TEXT[],
    
    effective_date DATE NOT NULL,
    expiry_date DATE,
    
    metadata JSONB DEFAULT '{}',
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    
    UNIQUE(organization_id, rule_id, region_id)
);

CREATE INDEX idx_rule_region_org ON rule_region_applicability(organization_id);
CREATE INDEX idx_rule_region_rule ON rule_region_applicability(rule_id);
CREATE INDEX idx_rule_region_region ON rule_region_applicability(region_id);

-- ============================================================================
-- RLS POLICIES
-- ============================================================================

ALTER TABLE geographic_regions ENABLE ROW LEVEL SECURITY;
ALTER TABLE organization_regions ENABLE ROW LEVEL SECURITY;
ALTER TABLE compliance_region_mapping ENABLE ROW LEVEL SECURITY;
ALTER TABLE rule_region_applicability ENABLE ROW LEVEL SECURITY;

-- Geographic regions - global read, admin write
CREATE POLICY geographic_regions_read_all ON geographic_regions
    FOR SELECT USING (TRUE);

CREATE POLICY geographic_regions_manage ON geographic_regions
    FOR ALL USING (
        current_setting('app.current_user_role', TRUE) IN ('super_admin', 'system')
    );

-- Organization regions - org isolation
CREATE POLICY organization_regions_org_isolation ON organization_regions
    FOR ALL USING (organization_id = current_setting('app.current_organization_id', TRUE)::UUID);

-- Compliance-region mapping - org isolation
CREATE POLICY compliance_region_mapping_org_isolation ON compliance_region_mapping
    FOR ALL USING (organization_id = current_setting('app.current_organization_id', TRUE)::UUID);

-- Rule-region mapping - org isolation
CREATE POLICY rule_region_applicability_org_isolation ON rule_region_applicability
    FOR ALL USING (organization_id = current_setting('app.current_organization_id', TRUE)::UUID);

-- ============================================================================
-- TRIGGERS
-- ============================================================================

-- Function to update geographic region path
CREATE OR REPLACE FUNCTION update_geographic_region_path()
RETURNS TRIGGER AS $$
BEGIN
    IF NEW.parent_region_id IS NULL THEN
        NEW.path = NEW.id::text::ltree;
        NEW.level = 0;
    ELSE
        SELECT path || NEW.id::text::ltree, level + 1
        INTO NEW.path, NEW.level
        FROM geographic_regions
        WHERE id = NEW.parent_region_id;
    END IF;
    RETURN NEW;
END;
$$ LANGUAGE plpgsql;

CREATE TRIGGER trigger_update_geographic_region_path
    BEFORE INSERT OR UPDATE OF parent_region_id ON geographic_regions
    FOR EACH ROW
    EXECUTE FUNCTION update_geographic_region_path();

-- Updated_at triggers
CREATE TRIGGER update_geographic_regions_updated_at
    BEFORE UPDATE ON geographic_regions
    FOR EACH ROW
    EXECUTE FUNCTION update_updated_at_column();

CREATE TRIGGER update_organization_regions_updated_at
    BEFORE UPDATE ON organization_regions
    FOR EACH ROW
    EXECUTE FUNCTION update_updated_at_column();

CREATE TRIGGER update_compliance_region_mapping_updated_at
    BEFORE UPDATE ON compliance_region_mapping
    FOR EACH ROW
    EXECUTE FUNCTION update_updated_at_column();

CREATE TRIGGER update_rule_region_applicability_updated_at
    BEFORE UPDATE ON rule_region_applicability
    FOR EACH ROW
    EXECUTE FUNCTION update_updated_at_column();

-- ============================================================================
-- COMMENTS
-- ============================================================================

COMMENT ON TABLE geographic_regions IS 'Hierarchical geographic taxonomy (countries, states, cities, etc.)';
COMMENT ON TABLE organization_regions IS 'Geographic presence and licensing information for organizations';
COMMENT ON TABLE compliance_region_mapping IS 'Maps compliance frameworks to geographic regions';
COMMENT ON TABLE rule_region_applicability IS 'Regional variations and applicability of compliance rules';
