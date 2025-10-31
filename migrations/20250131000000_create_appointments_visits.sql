-- Create Appointments, Visits, and Clinical Orders
-- Essential EMR workflow components

CREATE EXTENSION IF NOT EXISTS "uuid-ossp";

-- ============================================================================
-- APPOINTMENT SCHEDULING
-- ============================================================================

-- Table: appointments
CREATE TABLE IF NOT EXISTS appointments (
    id UUID PRIMARY KEY DEFAULT uuid_generate_v4(),
    organization_id UUID NOT NULL REFERENCES organizations(id) ON DELETE CASCADE,
    
    -- Core Information
    patient_id UUID NOT NULL, -- Will reference patients table
    provider_id UUID NOT NULL REFERENCES users(id),
    service_type_id UUID REFERENCES service_types(id),
    
    -- Scheduling
    appointment_type VARCHAR(50) NOT NULL CHECK (appointment_type IN (
        'consultation', 'follow_up', 'procedure', 'emergency', 
        'routine', 'urgent', 'walk_in', 'telemedicine'
    )),
    appointment_date TIMESTAMPTZ NOT NULL,
    duration_minutes INTEGER DEFAULT 30,
    
    -- Status Management
    status VARCHAR(50) NOT NULL DEFAULT 'scheduled' CHECK (status IN (
        'scheduled', 'confirmed', 'check_in', 'in_progress', 
        'completed', 'cancelled', 'no_show', 'rescheduled'
    )),
    
    -- Details
    reason_for_visit TEXT,
    special_instructions TEXT,
    
    -- Booking Information
    booked_by UUID REFERENCES users(id),
    booking_method VARCHAR(50) CHECK (booking_method IN (
        'online', 'phone', 'walk_in', 'staff', 'auto_reschedule'
    )),
    
    -- Cancellation
    cancelled_at TIMESTAMPTZ,
    cancelled_by UUID REFERENCES users(id),
    cancellation_reason TEXT,
    
    -- Reminders
    reminder_sent BOOLEAN DEFAULT false,
    reminder_sent_at TIMESTAMPTZ,
    
    -- Metadata
    location VARCHAR(200),
    room VARCHAR(100),
    metadata JSONB DEFAULT '{}',
    
    -- Audit
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE INDEX idx_appointments_patient ON appointments(patient_id);
CREATE INDEX idx_appointments_provider ON appointments(provider_id);
CREATE INDEX idx_appointments_date ON appointments(appointment_date);
CREATE INDEX idx_appointments_status ON appointments(status) WHERE status != 'completed';
CREATE INDEX idx_appointments_service ON appointments(service_type_id);

-- Table: provider_availability
CREATE TABLE IF NOT EXISTS provider_availability (
    id UUID PRIMARY KEY DEFAULT uuid_generate_v4(),
    provider_id UUID NOT NULL REFERENCES users(id),
    organization_id UUID NOT NULL REFERENCES organizations(id),
    
    -- Day and Time
    day_of_week INTEGER NOT NULL CHECK (day_of_week BETWEEN 0 AND 6), -- 0=Sun, 6=Sat
    start_time TIME NOT NULL,
    end_time TIME NOT NULL,
    
    -- Availability Type
    availability_type VARCHAR(50) DEFAULT 'regular' CHECK (availability_type IN (
        'regular', 'override', 'blocked', 'on_call'
    )),
    
    -- Exceptions
    effective_from DATE,
    effective_until DATE,
    is_exception BOOLEAN DEFAULT false,
    
    -- Blocks and Slots
    slot_duration_minutes INTEGER DEFAULT 30,
    buffer_time_minutes INTEGER DEFAULT 5,
    
    -- Status
    is_active BOOLEAN DEFAULT true,
    
    -- Metadata
    location VARCHAR(200),
    notes TEXT,
    
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    
    CHECK (end_time > start_time)
);

CREATE INDEX idx_provider_availability_provider ON provider_availability(provider_id, day_of_week, is_active);

-- ============================================================================
-- VISITS/ENCOUNTERS
-- ============================================================================

-- Table: patient_visits
CREATE TABLE IF NOT EXISTS patient_visits (
    id UUID PRIMARY KEY DEFAULT uuid_generate_v4(),
    organization_id UUID NOT NULL REFERENCES organizations(id),
    
    -- Core Information
    patient_id UUID NOT NULL,
    appointment_id UUID REFERENCES appointments(id),
    provider_id UUID NOT NULL REFERENCES users(id),
    
    -- Visit Details
    visit_type VARCHAR(50) NOT NULL CHECK (visit_type IN (
        'initial', 'follow_up', 'emergency', 'urgent_care', 
        'telemedicine', 'procedure', 'consultation'
    )),
    
    visit_date TIMESTAMPTZ NOT NULL,
    check_in_time TIMESTAMPTZ,
    seen_by_provider_time TIMESTAMPTZ,
    completion_time TIMESTAMPTZ,
    
    -- Status
    status VARCHAR(50) NOT NULL DEFAULT 'scheduled' CHECK (status IN (
        'scheduled', 'checked_in', 'in_progress', 'completed', 
        'cancelled', 'no_show', 'left_without_seen'
    )),
    
    -- Clinical
    chief_complaint TEXT,
    visit_duration_minutes INTEGER,
    
    -- Location
    location VARCHAR(200),
    department VARCHAR(100),
    room VARCHAR(100),
    
    -- Billing
    visit_billed BOOLEAN DEFAULT false,
    billing_status VARCHAR(50),
    
    -- Notes
    triage_notes TEXT,
    discharge_instructions TEXT,
    
    -- Metadata
    metadata JSONB DEFAULT '{}',
    
    -- Audit
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE INDEX idx_visits_patient ON patient_visits(patient_id);
CREATE INDEX idx_visits_provider ON patient_visits(provider_id);
CREATE INDEX idx_visits_date ON patient_visits(visit_date DESC);
CREATE INDEX idx_visits_appointment ON patient_visits(appointment_id);
CREATE INDEX idx_visits_status ON patient_visits(status) WHERE status != 'completed';

-- ============================================================================
-- CLINICAL ORDERS
-- ============================================================================

-- Table: clinical_orders
CREATE TABLE IF NOT EXISTS clinical_orders (
    id UUID PRIMARY KEY DEFAULT uuid_generate_v4(),
    organization_id UUID NOT NULL REFERENCES organizations(id),
    
    -- Core Information
    patient_id UUID NOT NULL,
    visit_id UUID REFERENCES patient_visits(id),
    provider_id UUID NOT NULL REFERENCES users(id),
    
    -- Order Details
    order_type VARCHAR(50) NOT NULL CHECK (order_type IN (
        'lab', 'radiology', 'procedure', 'medication', 
        'consultation', 'therapy', 'equipment'
    )),
    
    order_code VARCHAR(100), -- CPT, LOINC, or custom code
    order_name VARCHAR(500) NOT NULL,
    order_description TEXT,
    
    -- Service/Item Reference
    service_type_id UUID REFERENCES service_types(id),
    item_id UUID, -- For medications, equipment, etc.
    
    -- Priority and Status
    priority VARCHAR(50) DEFAULT 'routine' CHECK (priority IN (
        'stat', 'urgent', 'routine', 'timing_critical'
    )),
    
    status VARCHAR(50) NOT NULL DEFAULT 'ordered' CHECK (status IN (
        'ordered', 'sent', 'received', 'in_progress', 
        'completed', 'cancelled', 'rejected'
    )),
    
    -- Instructions
    special_instructions TEXT,
    clinical_notes TEXT,
    
    -- Timing
    order_date TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    requested_date TIMESTAMPTZ,
    due_date TIMESTAMPTZ,
    completed_date TIMESTAMPTZ,
    
    -- Results and Outcomes
    results JSONB DEFAULT '{}',
    interpretation TEXT,
    follow_up_required BOOLEAN DEFAULT false,
    
    -- Authorization
    requires_auth BOOLEAN DEFAULT false,
    auth_status VARCHAR(50),
    auth_number VARCHAR(100),
    
    -- Metadata
    metadata JSONB DEFAULT '{}',
    
    -- Audit
    created_by UUID NOT NULL REFERENCES users(id),
    updated_by UUID REFERENCES users(id),
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE INDEX idx_orders_patient ON clinical_orders(patient_id);
CREATE INDEX idx_orders_visit ON clinical_orders(visit_id);
CREATE INDEX idx_orders_provider ON clinical_orders(provider_id);
CREATE INDEX idx_orders_type ON clinical_orders(order_type);
CREATE INDEX idx_orders_status ON clinical_orders(status) WHERE status IN ('ordered', 'sent', 'in_progress');
CREATE INDEX idx_orders_service ON clinical_orders(service_type_id);

-- Table: order_results
CREATE TABLE IF NOT EXISTS order_results (
    id UUID PRIMARY KEY DEFAULT uuid_generate_v4(),
    order_id UUID NOT NULL REFERENCES clinical_orders(id),
    
    -- Result Data
    result_type VARCHAR(50) NOT NULL CHECK (result_type IN (
        'lab_result', 'imaging_result', 'procedure_result', 
        'interpretation', 'note', 'attachment'
    )),
    
    result_data JSONB NOT NULL DEFAULT '{}',
    result_text TEXT,
    
    -- Status
    is_final BOOLEAN DEFAULT false,
    is_abnormal BOOLEAN DEFAULT false,
    
    -- Professional Information
    performed_by UUID REFERENCES users(id),
    reviewed_by UUID REFERENCES users(id),
    reviewed_at TIMESTAMPTZ,
    
    -- Timing
    result_date TIMESTAMPTZ NOT NULL,
    
    -- Attachments
    attachment_urls TEXT[],
    
    -- Metadata
    metadata JSONB DEFAULT '{}',
    
    -- Audit
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE INDEX idx_order_results_order ON order_results(order_id);
CREATE INDEX idx_order_results_abnormal ON order_results(is_abnormal) WHERE is_abnormal = true;

-- ============================================================================
-- ROW LEVEL SECURITY
-- ============================================================================

ALTER TABLE appointments ENABLE ROW LEVEL SECURITY;
ALTER TABLE provider_availability ENABLE ROW LEVEL SECURITY;
ALTER TABLE patient_visits ENABLE ROW LEVEL SECURITY;
ALTER TABLE clinical_orders ENABLE ROW LEVEL SECURITY;
ALTER TABLE order_results ENABLE ROW LEVEL SECURITY;

-- Basic RLS policies (can be enhanced with Zanzibar)
CREATE POLICY appointments_org_policy ON appointments
    FOR ALL
    USING (organization_id = current_setting('app.current_org_id')::UUID);

CREATE POLICY provider_availability_policy ON provider_availability
    FOR ALL
    USING (organization_id = current_setting('app.current_org_id')::UUID);

CREATE POLICY visits_org_policy ON patient_visits
    FOR ALL
    USING (organization_id = current_setting('app.current_org_id')::UUID);

CREATE POLICY orders_org_policy ON clinical_orders
    FOR ALL
    USING (organization_id = current_setting('app.current_org_id')::UUID);

CREATE POLICY order_results_policy ON order_results
    FOR ALL
    USING (
        EXISTS (
            SELECT 1 FROM clinical_orders co 
            WHERE co.id = order_results.order_id 
            AND co.organization_id = current_setting('app.current_org_id')::UUID
        )
    );

-- ============================================================================
-- AUDIT TRIGGERS
-- ============================================================================

CREATE OR REPLACE FUNCTION update_updated_at_column()
RETURNS TRIGGER AS $$
BEGIN
    NEW.updated_at = NOW();
    RETURN NEW;
END;
$$ language 'plpgsql';

CREATE TRIGGER update_appointments_updated_at BEFORE UPDATE ON appointments
    FOR EACH ROW EXECUTE FUNCTION update_updated_at_column();

CREATE TRIGGER update_visits_updated_at BEFORE UPDATE ON patient_visits
    FOR EACH ROW EXECUTE FUNCTION update_updated_at_column();

CREATE TRIGGER update_orders_updated_at BEFORE UPDATE ON clinical_orders
    FOR EACH ROW EXECUTE FUNCTION update_updated_at_column();

CREATE TRIGGER update_order_results_updated_at BEFORE UPDATE ON order_results
    FOR EACH ROW EXECUTE FUNCTION update_updated_at_column();

