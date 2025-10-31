-- Create Pharmacy and Vendor Management Tables
-- Comprehensive inventory and external provider management

CREATE EXTENSION IF NOT EXISTS "uuid-ossp";

-- ============================================================================
-- PHARMACY MODULE
-- ============================================================================

-- Table: pharmacies
CREATE TABLE IF NOT EXISTS pharmacies (
    id UUID PRIMARY KEY DEFAULT uuid_generate_v4(),
    organization_id UUID NOT NULL REFERENCES organizations(id) ON DELETE CASCADE,
    
    -- Pharmacy Information
    name VARCHAR(255) NOT NULL,
    code VARCHAR(50) NOT NULL UNIQUE,
    address TEXT NOT NULL,
    city VARCHAR(100) NOT NULL,
    state VARCHAR(50) NOT NULL,
    postal_code VARCHAR(20) NOT NULL,
    country VARCHAR(100) DEFAULT 'US',
    
    -- Contact
    phone VARCHAR(50),
    email VARCHAR(255),
    fax VARCHAR(50),
    
    -- Licensing
    license_number VARCHAR(100),
    license_authority VARCHAR(255),
    license_expiry DATE,
    dea_number VARCHAR(50), -- Drug Enforcement Administration
    
    -- Operations
    hours_of_operation JSONB, -- Opening hours by day
    is_internal BOOLEAN DEFAULT true, -- Internal vs external pharmacy
    is_active BOOLEAN DEFAULT true,
    
    -- Metadata
    settings JSONB DEFAULT '{}',
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    
    UNIQUE(organization_id, code)
);

CREATE INDEX idx_pharmacies_org ON pharmacies(organization_id);
CREATE INDEX idx_pharmacies_active ON pharmacies(is_active) WHERE is_active = true;
CREATE INDEX idx_pharmacies_internal ON pharmacies(is_internal);

-- Table: medications
CREATE TABLE IF NOT EXISTS medications (
    id UUID PRIMARY KEY DEFAULT uuid_generate_v4(),
    organization_id UUID NOT NULL REFERENCES organizations(id) ON DELETE CASCADE,
    
    -- Medication Information
    name VARCHAR(500) NOT NULL,
    generic_name VARCHAR(500),
    medication_code VARCHAR(100), -- NDC, RxNorm, etc.
    medication_type VARCHAR(50) NOT NULL CHECK (medication_type IN (
        'drug', 'supplement', 'vaccine', 'medical_device', 'supply'
    )),
    
    -- Classification
    drug_class VARCHAR(200),
    therapeutic_category VARCHAR(200),
    
    -- Drug Details (JSONB for flexibility)
    active_ingredients JSONB DEFAULT '{}',
    strength VARCHAR(100),
    dosage_form VARCHAR(100), -- tablet, capsule, liquid, etc.
    route_of_administration VARCHAR(100), -- oral, IV, topical, etc.
    
    -- Regulations
    prescription_required BOOLEAN DEFAULT true,
    controlled_substance_schedule VARCHAR(10), -- I, II, III, IV, V
    dea_classification VARCHAR(50),
    
    -- Safety
    contraindications JSONB DEFAULT '[]',
    side_effects JSONB DEFAULT '[]',
    drug_interactions JSONB DEFAULT '[]',
    pregnancy_category VARCHAR(10), -- A, B, C, D, X
    pediatric_safe BOOLEAN,
    geriatric_safe BOOLEAN,
    
    -- Commercial
    manufacturer VARCHAR(255),
    brand_name VARCHAR(255),
    
    -- Metadata
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE INDEX idx_medications_org ON medications(organization_id);
CREATE INDEX idx_medications_code ON medications(medication_code);
CREATE INDEX idx_medications_type ON medications(medication_type);
CREATE INDEX idx_medications_name ON medications USING GIN(to_tsvector('english', name));

-- Table: pharmacy_inventory
CREATE TABLE IF NOT EXISTS pharmacy_inventory (
    id UUID PRIMARY KEY DEFAULT uuid_generate_v4(),
    pharmacy_id UUID NOT NULL REFERENCES pharmacies(id) ON DELETE CASCADE,
    medication_id UUID NOT NULL REFERENCES medications(id) ON DELETE CASCADE,
    
    -- Stock Information
    quantity_on_hand INTEGER NOT NULL DEFAULT 0 CHECK (quantity_on_hand >= 0),
    quantity_reserved INTEGER NOT NULL DEFAULT 0 CHECK (quantity_reserved >= 0),
    quantity_available INTEGER GENERATED ALWAYS AS (quantity_on_hand - quantity_reserved) STORED,
    
    -- Location
    location VARCHAR(200), -- Shelf location, zone, etc.
    lot_number VARCHAR(100),
    
    -- Expiry & Batches
    expiry_date DATE,
    date_received DATE NOT NULL,
    
    -- Pricing
    unit_cost DECIMAL(10, 2),
    unit_price DECIMAL(10, 2),
    currency VARCHAR(3) DEFAULT 'USD',
    
    -- Reorder Management
    reorder_level INTEGER DEFAULT 10,
    reorder_quantity INTEGER,
    last_reorder_date DATE,
    
    -- Status
    status VARCHAR(50) NOT NULL DEFAULT 'active' CHECK (status IN (
        'active', 'low_stock', 'out_of_stock', 'expired', 'quarantined', 'discontinued'
    )),
    
    -- Audit
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    
    UNIQUE(pharmacy_id, medication_id, lot_number)
);

CREATE INDEX idx_inventory_pharmacy ON pharmacy_inventory(pharmacy_id);
CREATE INDEX idx_inventory_medication ON pharmacy_inventory(medication_id);
CREATE INDEX idx_inventory_status ON pharmacy_inventory(status);
CREATE INDEX idx_inventory_expiry ON pharmacy_inventory(expiry_date);
CREATE INDEX idx_inventory_low_stock ON pharmacy_inventory(quantity_available) WHERE status = 'low_stock';

-- Table: prescriptions
CREATE TABLE IF NOT EXISTS prescriptions (
    id UUID PRIMARY KEY DEFAULT uuid_generate_v4(),
    organization_id UUID NOT NULL REFERENCES organizations(id) ON DELETE CASCADE,
    patient_id UUID NOT NULL, -- Will reference patients table
    provider_id UUID NOT NULL REFERENCES healthcare_providers(id) ON DELETE CASCADE,
    pharmacy_id UUID REFERENCES pharmacies(id),
    medication_id UUID NOT NULL REFERENCES medications(id) ON DELETE CASCADE,
    
    -- Prescription Details
    dosage VARCHAR(100) NOT NULL, -- e.g., "20mg", "500mg"
    quantity INTEGER NOT NULL,
    days_supply INTEGER,
    frequency VARCHAR(100) NOT NULL, -- e.g., "twice daily", "as needed"
    route_of_administration VARCHAR(100),
    duration_days INTEGER,
    
    -- Instructions
    instructions TEXT,
    patient_instructions TEXT,
    sig_code VARCHAR(20), -- Prescription abbreviation
    
    -- Status
    status VARCHAR(50) NOT NULL DEFAULT 'pending' CHECK (status IN (
        'pending', 'approved', 'dispersed', 'partially_dispersed', 'cancelled', 'expired'
    )),
    
    -- Dates
    prescribed_date TIMESTAMP NOT NULL DEFAULT NOW(),
    start_date TIMESTAMP,
    end_date TIMESTAMP,
    date_dispersed TIMESTAMP,
    
    -- Refills
    refills_remaining INTEGER DEFAULT 0,
    max_refills INTEGER DEFAULT 0,
    
    -- Cost
    insurance_covered BOOLEAN,
    copay_amount DECIMAL(10, 2),
    total_cost DECIMAL(10, 2),
    
    -- Regulatory
    electronic_signature TEXT,
    paper_prescription BOOLEAN DEFAULT false,
    dea_required BOOLEAN DEFAULT false,
    
    -- Audit
    created_by UUID NOT NULL REFERENCES users(id),
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE INDEX idx_prescriptions_patient ON prescriptions(patient_id);
CREATE INDEX idx_prescriptions_provider ON prescriptions(provider_id);
CREATE INDEX idx_prescriptions_pharmacy ON prescriptions(pharmacy_id);
CREATE INDEX idx_prescriptions_status ON prescriptions(status);
CREATE INDEX idx_prescriptions_date ON prescriptions(prescribed_date DESC);

-- ============================================================================
-- VENDOR / EXTERNAL PROVIDER MODULE
-- ============================================================================

-- Table: vendor_types
CREATE TABLE IF NOT EXISTS vendor_types (
    id UUID PRIMARY KEY DEFAULT uuid_generate_v4(),
    code VARCHAR(50) NOT NULL UNIQUE,
    name VARCHAR(255) NOT NULL,
    description TEXT,
    category VARCHAR(100), -- equipment, supplies, services, IT, facilities
    is_active BOOLEAN DEFAULT true,
    metadata JSONB DEFAULT '{}'
);

CREATE INDEX idx_vendor_types_category ON vendor_types(category);

-- Table: vendors
CREATE TABLE IF NOT EXISTS vendors (
    id UUID PRIMARY KEY DEFAULT uuid_generate_v4(),
    organization_id UUID NOT NULL REFERENCES organizations(id) ON DELETE CASCADE,
    vendor_type_id UUID NOT NULL REFERENCES vendor_types(id) ON DELETE CASCADE,
    
    -- Vendor Information
    name VARCHAR(255) NOT NULL,
    code VARCHAR(50) NOT NULL,
    tax_id VARCHAR(50),
    vat_number VARCHAR(50),
    
    -- Contact Information
    address TEXT NOT NULL,
    city VARCHAR(100) NOT NULL,
    state VARCHAR(50) NOT NULL,
    postal_code VARCHAR(20) NOT NULL,
    country VARCHAR(100) DEFAULT 'US',
    
    phone VARCHAR(50),
    email VARCHAR(255),
    website VARCHAR(500),
    contact_person VARCHAR(255),
    contact_phone VARCHAR(50),
    contact_email VARCHAR(255),
    
    -- Business Details
    legal_entity_type VARCHAR(50), -- LLC, Corp, Partnership, etc.
    established_date DATE,
    registration_number VARCHAR(100),
    
    -- Licensing & Compliance
    licenses JSONB DEFAULT '[]', -- Array of licenses
    certifications JSONB DEFAULT '[]', -- Certifications (ISO, etc.)
    insurance JSONB DEFAULT '{}',
    
    -- Financial
    payment_terms VARCHAR(100), -- Net 30, etc.
    credit_limit DECIMAL(12, 2),
    currency VARCHAR(3) DEFAULT 'USD',
    
    -- Quality & Performance
    quality_rating DECIMAL(3, 2), -- 0.00 to 5.00
    performance_score INTEGER, -- 0 to 100
    on_time_delivery_rate DECIMAL(5, 2),
    
    -- Status
    is_preferred_vendor BOOLEAN DEFAULT false,
    is_active BOOLEAN DEFAULT true,
    contract_start_date DATE,
    contract_end_date DATE,
    
    -- Metadata
    notes TEXT,
    tags TEXT[],
    metadata JSONB DEFAULT '{}',
    
    -- Audit
    created_by UUID REFERENCES users(id),
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    
    UNIQUE(organization_id, code)
);

CREATE INDEX idx_vendors_org ON vendors(organization_id);
CREATE INDEX idx_vendors_type ON vendors(vendor_type_id);
CREATE INDEX idx_vendors_active ON vendors(is_active) WHERE is_active = true;
CREATE INDEX idx_vendors_preferred ON vendors(is_preferred_vendor) WHERE is_preferred_vendor = true;

-- Table: vendor_inventory
CREATE TABLE IF NOT EXISTS vendor_inventory (
    id UUID PRIMARY KEY DEFAULT uuid_generate_v4(),
    vendor_id UUID NOT NULL REFERENCES vendors(id) ON DELETE CASCADE,
    
    -- Item Information
    item_code VARCHAR(100) NOT NULL,
    item_name VARCHAR(500) NOT NULL,
    description TEXT,
    item_category VARCHAR(100), -- equipment, supplies, consumables, etc.
    unit_of_measure VARCHAR(50), -- each, box, case, liter, kg, etc.
    
    -- Pricing
    unit_price DECIMAL(10, 2) NOT NULL,
    bulk_price DECIMAL(10, 2),
    minimum_order_quantity INTEGER,
    currency VARCHAR(3) DEFAULT 'USD',
    
    -- Stock from Vendor
    in_stock BOOLEAN DEFAULT true,
    lead_time_days INTEGER, -- Days to deliver
    stock_quantity INTEGER,
    
    -- Product Details
    manufacturer VARCHAR(255),
    brand VARCHAR(255),
    model VARCHAR(255),
    specifications JSONB DEFAULT '{}',
    
    -- Compliance & Safety
    regulatory_approvals JSONB DEFAULT '[]',
    certifications JSONB DEFAULT '[]',
    safety_data_sheet_url TEXT,
    
    -- Status
    is_active BOOLEAN DEFAULT true,
    discontinued_date DATE,
    
    -- Metadata
    images JSONB DEFAULT '[]', -- Array of image URLs
    metadata JSONB DEFAULT '{}',
    
    -- Audit
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    
    UNIQUE(vendor_id, item_code)
);

CREATE INDEX idx_vendor_inventory_vendor ON vendor_inventory(vendor_id);
CREATE INDEX idx_vendor_inventory_category ON vendor_inventory(item_category);
CREATE INDEX idx_vendor_inventory_active ON vendor_inventory(is_active) WHERE is_active = true;
CREATE INDEX idx_vendor_inventory_in_stock ON vendor_inventory(in_stock) WHERE in_stock = true;

-- Table: service_types (Reusable service catalog)
CREATE TABLE IF NOT EXISTS service_types (
    id UUID PRIMARY KEY DEFAULT uuid_generate_v4(),
    organization_id UUID REFERENCES organizations(id) ON DELETE SET NULL,
    
    -- Service Type Information
    code VARCHAR(100) NOT NULL UNIQUE,
    name VARCHAR(500) NOT NULL,
    description TEXT,
    category VARCHAR(100) NOT NULL, -- consultation, testing, procedure, diagnostic, treatment
    
    -- Service Details
    service_classification VARCHAR(100), -- primary_care, specialty, emergency, routine, etc.
    typical_duration_hours DECIMAL(8, 2),
    typical_duration_days INTEGER,
    
    -- Requirements
    requires_licensure BOOLEAN DEFAULT true,
    required_qualifications JSONB DEFAULT '[]', -- Array of required licenses/certifications
    equipment_required JSONB DEFAULT '[]', -- Equipment needed
    facility_requirements JSONB DEFAULT '{}', -- Facility level, type, etc.
    pre_authorization_required BOOLEAN DEFAULT false,
    
    -- Regulatory & Compliance
    cpt_code VARCHAR(10), -- Current Procedural Terminology
    icd_10_codes JSONB DEFAULT '[]', -- Applicable diagnosis codes
    hcpcs_code VARCHAR(10), -- Healthcare Common Procedure Coding System
    insurance_coverage_typical BOOLEAN DEFAULT true,
    
    -- Clinical
    urgency_level VARCHAR(50), -- routine, urgent, emergency
    complexity_level VARCHAR(50), -- simple, moderate, complex, highly_complex
    risk_level VARCHAR(50), -- low, moderate, high
    
    -- Metadata
    tags TEXT[],
    metadata JSONB DEFAULT '{}',
    is_active BOOLEAN DEFAULT true,
    
    -- Audit
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE INDEX idx_service_types_code ON service_types(code);
CREATE INDEX idx_service_types_category ON service_types(category);
CREATE INDEX idx_service_types_active ON service_types(is_active) WHERE is_active = true;

-- Table: vendor_services (Links vendors to service types)
CREATE TABLE IF NOT EXISTS vendor_services (
    id UUID PRIMARY KEY DEFAULT uuid_generate_v4(),
    vendor_id UUID NOT NULL REFERENCES vendors(id) ON DELETE CASCADE,
    service_type_id UUID NOT NULL REFERENCES service_types(id) ON DELETE CASCADE,
    
    -- Vendor-specific customization
    vendor_service_code VARCHAR(100), -- Vendor's own code for this service
    custom_name VARCHAR(500), -- Override service type name if needed
    custom_description TEXT,
    
    -- Vendor-specific pricing
    pricing_model VARCHAR(50) NOT NULL, -- fixed, hourly, per_unit, contract
    base_price DECIMAL(12, 2),
    hourly_rate DECIMAL(10, 2),
    per_unit_price DECIMAL(10, 2),
    min_quantity INTEGER DEFAULT 1,
    currency VARCHAR(3) DEFAULT 'USD',
    
    -- Vendor-specific availability
    is_available BOOLEAN DEFAULT true,
    availability_hours JSONB DEFAULT '{}', -- Hours of availability
    requires_appointment BOOLEAN DEFAULT false,
    booking_window_days INTEGER DEFAULT 1,
    turnaround_time VARCHAR(100), -- Vendor's turnaround time
    
    -- Vendor-specific requirements
    additional_certifications_required JSONB DEFAULT '[]',
    additional_equipment_required JSONB DEFAULT '[]',
    special_instructions TEXT,
    
    -- Quality guarantees
    quality_guarantee TEXT,
    satisfaction_guarantee TEXT,
    rework_policy TEXT,
    
    -- Vendor performance
    average_rating DECIMAL(3, 2), -- 0.00 to 5.00
    total_completions INTEGER DEFAULT 0,
    
    -- Status
    is_active BOOLEAN DEFAULT true,
    
    -- Metadata
    metadata JSONB DEFAULT '{}',
    
    -- Audit
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    
    UNIQUE(vendor_id, service_type_id)
);

CREATE INDEX idx_vendor_services_vendor ON vendor_services(vendor_id);
CREATE INDEX idx_vendor_services_type ON vendor_services(service_type_id);
CREATE INDEX idx_vendor_services_active ON vendor_services(is_active) WHERE is_active = true;

-- Table: provider_service_types (Links healthcare providers to service types they offer)
CREATE TABLE IF NOT EXISTS provider_service_types (
    id UUID PRIMARY KEY DEFAULT uuid_generate_v4(),
    provider_id UUID NOT NULL REFERENCES healthcare_providers(id) ON DELETE CASCADE,
    service_type_id UUID NOT NULL REFERENCES service_types(id) ON DELETE CASCADE,
    
    -- Provider-specific customization
    provider_service_code VARCHAR(100), -- Provider's own code
    custom_instructions TEXT,
    
    -- Provider-specific pricing (if applicable)
    provider_price DECIMAL(10, 2),
    accepts_insurance BOOLEAN DEFAULT true,
    self_pay_price DECIMAL(10, 2),
    
    -- Availability & Capacity
    is_active BOOLEAN DEFAULT true,
    typical_availability_hours JSONB DEFAULT '{}',
    max_daily_capacity INTEGER,
    current_bookings INTEGER DEFAULT 0,
    
    -- Performance metrics
    average_duration_hours DECIMAL(6, 2), -- Actual vs typical
    patient_satisfaction_score DECIMAL(3, 2),
    completion_rate DECIMAL(5, 2),
    
    -- Requirements (may differ from base service type)
    specific_certifications JSONB DEFAULT '[]',
    specific_equipment JSONB DEFAULT '[]',
    
    -- Status
    proficiency_level VARCHAR(50), -- beginner, intermediate, expert, master
    is_preferred BOOLEAN DEFAULT false, -- Preferred provider for this service
    
    -- Metadata
    notes TEXT,
    metadata JSONB DEFAULT '{}',
    
    -- Audit
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    
    UNIQUE(provider_id, service_type_id)
);

CREATE INDEX idx_provider_services_provider ON provider_service_types(provider_id);
CREATE INDEX idx_provider_services_type ON provider_service_types(service_type_id);
CREATE INDEX idx_provider_services_active ON provider_service_types(is_active) WHERE is_active = true;
CREATE INDEX idx_provider_services_preferred ON provider_service_types(is_preferred) WHERE is_preferred = true;

-- Table: purchase_orders
CREATE TABLE IF NOT EXISTS purchase_orders (
    id UUID PRIMARY KEY DEFAULT uuid_generate_v4(),
    organization_id UUID NOT NULL REFERENCES organizations(id) ON DELETE CASCADE,
    vendor_id UUID NOT NULL REFERENCES vendors(id) ON DELETE CASCADE,
    
    -- Order Information
    po_number VARCHAR(100) NOT NULL UNIQUE,
    order_date DATE NOT NULL DEFAULT CURRENT_DATE,
    expected_delivery_date DATE,
    actual_delivery_date DATE,
    
    -- Financial
    subtotal DECIMAL(12, 2) NOT NULL,
    tax_amount DECIMAL(12, 2),
    shipping_cost DECIMAL(10, 2),
    discount_amount DECIMAL(10, 2),
    total_amount DECIMAL(12, 2) NOT NULL,
    currency VARCHAR(3) DEFAULT 'USD',
    
    -- Status
    status VARCHAR(50) NOT NULL DEFAULT 'draft' CHECK (status IN (
        'draft', 'submitted', 'approved', 'ordered', 'partially_received', 
        'received', 'cancelled', 'closed'
    )),
    
    -- Approval
    requested_by UUID NOT NULL REFERENCES users(id),
    approved_by UUID REFERENCES users(id),
    approval_date TIMESTAMP,
    
    -- Delivery
    shipping_address TEXT,
    tracking_number VARCHAR(200),
    received_by UUID REFERENCES users(id),
    
    -- Notes
    notes TEXT,
    internal_notes TEXT,
    
    -- Audit
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE INDEX idx_purchase_orders_org ON purchase_orders(organization_id);
CREATE INDEX idx_purchase_orders_vendor ON purchase_orders(vendor_id);
CREATE INDEX idx_purchase_orders_status ON purchase_orders(status);
CREATE INDEX idx_purchase_orders_po_number ON purchase_orders(po_number);
CREATE INDEX idx_purchase_orders_date ON purchase_orders(order_date DESC);

-- Table: purchase_order_items
CREATE TABLE IF NOT EXISTS purchase_order_items (
    id UUID PRIMARY KEY DEFAULT uuid_generate_v4(),
    purchase_order_id UUID NOT NULL REFERENCES purchase_orders(id) ON DELETE CASCADE,
    vendor_inventory_id UUID REFERENCES vendor_inventory(id),
    
    -- Item Details
    item_code VARCHAR(100) NOT NULL,
    item_name VARCHAR(500) NOT NULL,
    description TEXT,
    quantity INTEGER NOT NULL CHECK (quantity > 0),
    unit_price DECIMAL(10, 2) NOT NULL,
    line_total DECIMAL(12, 2) GENERATED ALWAYS AS (quantity * unit_price) STORED,
    
    -- Status
    quantity_received INTEGER DEFAULT 0 CHECK (quantity_received >= 0),
    status VARCHAR(50) NOT NULL DEFAULT 'pending' CHECK (status IN (
        'pending', 'ordered', 'partially_received', 'received', 'cancelled'
    )),
    
    -- Notes
    notes TEXT,
    
    -- Audit
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE INDEX idx_po_items_po ON purchase_order_items(purchase_order_id);
CREATE INDEX idx_po_items_status ON purchase_order_items(status);

-- ============================================================================
-- VENDOR CONTRACTS
-- ============================================================================

-- Table: vendor_contracts
CREATE TABLE IF NOT EXISTS vendor_contracts (
    id UUID PRIMARY KEY DEFAULT uuid_generate_v4(),
    organization_id UUID NOT NULL REFERENCES organizations(id) ON DELETE CASCADE,
    vendor_id UUID NOT NULL REFERENCES vendors(id) ON DELETE CASCADE,
    
    -- Contract Information
    contract_number VARCHAR(100) NOT NULL UNIQUE,
    contract_type VARCHAR(100), -- service, supply, lease, etc.
    
    -- Dates
    start_date DATE NOT NULL,
    end_date DATE,
    auto_renew BOOLEAN DEFAULT false,
    renewal_term_months INTEGER,
    
    -- Financial
    total_contract_value DECIMAL(12, 2),
    payment_schedule VARCHAR(100),
    currency VARCHAR(3) DEFAULT 'USD',
    
    -- Terms & Conditions
    terms_conditions TEXT,
    sla_requirements JSONB DEFAULT '{}', -- Service Level Agreements
    penalties JSONB DEFAULT '{}',
    incentives JSONB DEFAULT '{}',
    
    -- Status
    status VARCHAR(50) NOT NULL DEFAULT 'active' CHECK (status IN (
        'draft', 'pending_approval', 'active', 'expired', 'terminated', 'cancelled'
    )),
    
    -- Documents
    contract_document_url TEXT,
    amendments JSONB DEFAULT '[]',
    
    -- Audit
    created_by UUID NOT NULL REFERENCES users(id),
    approved_by UUID REFERENCES users(id),
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE INDEX idx_contracts_org ON vendor_contracts(organization_id);
CREATE INDEX idx_contracts_vendor ON vendor_contracts(vendor_id);
CREATE INDEX idx_contracts_status ON vendor_contracts(status);
CREATE INDEX idx_contracts_dates ON vendor_contracts(start_date, end_date);

-- ============================================================================
-- TRIGGERS
-- ============================================================================

-- Update updated_at timestamp
CREATE TRIGGER update_pharmacies_updated_at
    BEFORE UPDATE ON pharmacies
    FOR EACH ROW
    EXECUTE FUNCTION update_updated_at_column();

CREATE TRIGGER update_medications_updated_at
    BEFORE UPDATE ON medications
    FOR EACH ROW
    EXECUTE FUNCTION update_updated_at_column();

CREATE TRIGGER update_inventory_updated_at
    BEFORE UPDATE ON pharmacy_inventory
    FOR EACH ROW
    EXECUTE FUNCTION update_updated_at_column();

CREATE TRIGGER update_prescriptions_updated_at
    BEFORE UPDATE ON prescriptions
    FOR EACH ROW
    EXECUTE FUNCTION update_updated_at_column();

CREATE TRIGGER update_vendors_updated_at
    BEFORE UPDATE ON vendors
    FOR EACH ROW
    EXECUTE FUNCTION update_updated_at_column();

CREATE TRIGGER update_vendor_inventory_updated_at
    BEFORE UPDATE ON vendor_inventory
    FOR EACH ROW
    EXECUTE FUNCTION update_updated_at_column();

CREATE TRIGGER update_vendor_services_updated_at
    BEFORE UPDATE ON vendor_services
    FOR EACH ROW
    EXECUTE FUNCTION update_updated_at_column();

CREATE TRIGGER update_purchase_orders_updated_at
    BEFORE UPDATE ON purchase_orders
    FOR EACH ROW
    EXECUTE FUNCTION update_updated_at_column();

CREATE TRIGGER update_vendor_contracts_updated_at
    BEFORE UPDATE ON vendor_contracts
    FOR EACH ROW
    EXECUTE FUNCTION update_updated_at_column();

-- ============================================================================
-- COMMENTS
-- ============================================================================

COMMENT ON TABLE pharmacies IS 'Internal and external pharmacy management';
COMMENT ON TABLE medications IS 'Medication catalog with safety information';
COMMENT ON TABLE pharmacy_inventory IS 'Real-time pharmacy stock tracking';
COMMENT ON TABLE prescriptions IS 'Digital prescriptions with e-signature support';
COMMENT ON TABLE vendors IS 'External vendor and service provider management';
COMMENT ON TABLE vendor_inventory IS 'Available products and equipment from vendors';
COMMENT ON TABLE vendor_services IS 'Service catalog from external providers';
COMMENT ON TABLE purchase_orders IS 'Procurement and ordering system';
COMMENT ON TABLE vendor_contracts IS 'Vendor contract and SLA management';

