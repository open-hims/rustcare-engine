-- Create organization_employees table
-- This table stores employee/staff information for organizations with multi-tenancy support

CREATE TABLE IF NOT EXISTS organization_employees (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    user_id UUID NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    organization_id UUID NOT NULL REFERENCES organizations(id) ON DELETE CASCADE,
    employee_id VARCHAR(100) NOT NULL, -- Employee number or staff ID
    first_name VARCHAR(255) NOT NULL,
    last_name VARCHAR(255) NOT NULL,
    email VARCHAR(255) NOT NULL,
    phone VARCHAR(50),
    department VARCHAR(255),
    position VARCHAR(255), -- Job title/position
    start_date DATE NOT NULL DEFAULT CURRENT_DATE,
    end_date DATE, -- NULL if currently employed
    is_active BOOLEAN NOT NULL DEFAULT true,
    
    -- Role and permissions (stored as JSON for flexibility)
    role_id UUID, -- Reference to roles table if it exists
    
    -- Audit fields
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    created_by UUID REFERENCES users(id) ON DELETE SET NULL,
    updated_by UUID REFERENCES users(id) ON DELETE SET NULL,
    is_deleted BOOLEAN DEFAULT false,
    deleted_at TIMESTAMPTZ,
    deleted_by UUID REFERENCES users(id) ON DELETE SET NULL,
    
    -- Constraints
    CONSTRAINT unique_employee_per_org UNIQUE (organization_id, employee_id),
    CONSTRAINT unique_user_per_org UNIQUE (organization_id, user_id),
    CONSTRAINT valid_employment_dates CHECK (end_date IS NULL OR end_date >= start_date)
);

-- Indexes for performance
CREATE INDEX idx_org_employees_organization_id ON organization_employees(organization_id) WHERE is_deleted = false;
CREATE INDEX idx_org_employees_user_id ON organization_employees(user_id) WHERE is_deleted = false;
CREATE INDEX idx_org_employees_employee_id ON organization_employees(employee_id) WHERE is_deleted = false;
CREATE INDEX idx_org_employees_email ON organization_employees(email) WHERE is_deleted = false;
CREATE INDEX idx_org_employees_department ON organization_employees(department) WHERE is_deleted = false AND department IS NOT NULL;
CREATE INDEX idx_org_employees_is_active ON organization_employees(is_active) WHERE is_deleted = false;
CREATE INDEX idx_org_employees_created_at ON organization_employees(created_at DESC) WHERE is_deleted = false;

-- Full-text search index for employee names
CREATE INDEX idx_org_employees_name_search ON organization_employees USING gin(
    to_tsvector('english', first_name || ' ' || last_name || ' ' || COALESCE(email, ''))
) WHERE is_deleted = false;

-- Trigger for updated_at timestamp
CREATE OR REPLACE FUNCTION update_organization_employees_updated_at()
RETURNS TRIGGER AS $$
BEGIN
    NEW.updated_at = NOW();
    RETURN NEW;
END;
$$ LANGUAGE plpgsql;

CREATE TRIGGER trigger_organization_employees_updated_at
    BEFORE UPDATE ON organization_employees
    FOR EACH ROW
    EXECUTE FUNCTION update_organization_employees_updated_at();

-- Enable Row Level Security (RLS)
ALTER TABLE organization_employees ENABLE ROW LEVEL SECURITY;

-- RLS Policy: Users can only see employees from their organization
CREATE POLICY org_employees_org_isolation ON organization_employees
    FOR ALL
    USING (organization_id = current_setting('app.current_organization_id', TRUE)::UUID);

-- RLS Policy: Active users can see all employees in their org
CREATE POLICY org_employees_user_access ON organization_employees
    FOR SELECT
    USING (
        EXISTS (
            SELECT 1 FROM users u
            WHERE u.id = current_setting('app.current_user_id', TRUE)::UUID
            AND u.organization_id = organization_employees.organization_id
            AND u.status = 'active'
        )
    );

-- RLS Policy: Employees can see their own record
CREATE POLICY org_employees_self_access ON organization_employees
    FOR SELECT
    USING (user_id = current_setting('app.current_user_id', TRUE)::UUID);

-- Grant permissions
GRANT SELECT, INSERT, UPDATE, DELETE ON organization_employees TO rustcare;
GRANT USAGE, SELECT ON ALL SEQUENCES IN SCHEMA public TO rustcare;

-- Comments for documentation
COMMENT ON TABLE organization_employees IS 'Stores employee/staff information for organizations with multi-tenancy support';
COMMENT ON COLUMN organization_employees.employee_id IS 'Employee number or staff identifier unique within organization';
COMMENT ON COLUMN organization_employees.user_id IS 'Reference to the user account for this employee';
COMMENT ON COLUMN organization_employees.is_active IS 'Whether the employee is currently active (not terminated/on leave)';
COMMENT ON COLUMN organization_employees.end_date IS 'Employment end date - NULL indicates currently employed';
COMMENT ON COLUMN organization_employees.is_deleted IS 'Soft delete flag - employees are never hard deleted for audit compliance';

