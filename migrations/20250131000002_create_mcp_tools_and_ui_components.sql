-- Migration: Create MCP Tools and UI Components Registry
-- Purpose: Store discovered MCP tools and UI components with permissions for auto-discovery

-- MCP Tools Registry
CREATE TABLE IF NOT EXISTS mcp_tools (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    organization_id UUID REFERENCES organizations(id) ON DELETE CASCADE,
    
    -- Tool identification
    tool_name VARCHAR(255) NOT NULL,
    handler_function VARCHAR(255) NOT NULL, -- e.g., "pharmacy::list_pharmacies"
    handler_file VARCHAR(500) NOT NULL,     -- e.g., "handlers/pharmacy.rs"
    
    -- Tool metadata
    description TEXT,
    category VARCHAR(100) NOT NULL,
    response_type VARCHAR(255), -- e.g., "Vec<Pharmacy>", "Patient"
    render_type VARCHAR(50),    -- e.g., "table", "markdown", "json"
    
    -- Security
    requires_permission VARCHAR(255), -- Zanzibar permission string
    sensitive BOOLEAN DEFAULT false,
    
    -- Input/Output schemas (JSON Schema)
    input_schema JSONB,
    output_schema JSONB,
    
    -- Registration metadata
    auto_discovered BOOLEAN DEFAULT true,
    registered_at TIMESTAMPTZ DEFAULT NOW(),
    registered_by UUID REFERENCES users(id),
    
    -- Status
    is_active BOOLEAN DEFAULT true,
    is_deleted BOOLEAN DEFAULT false,
    deleted_at TIMESTAMPTZ,
    
    created_at TIMESTAMPTZ DEFAULT NOW(),
    updated_at TIMESTAMPTZ DEFAULT NOW(),
    
    UNIQUE(organization_id, tool_name)
);

CREATE INDEX idx_mcp_tools_category ON mcp_tools(category);
CREATE INDEX idx_mcp_tools_active ON mcp_tools(organization_id, is_active, is_deleted) WHERE is_active = true AND is_deleted = false;
CREATE INDEX idx_mcp_tools_permission ON mcp_tools(requires_permission) WHERE requires_permission IS NOT NULL;

-- UI Components Registry
CREATE TABLE IF NOT EXISTS ui_components (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    organization_id UUID REFERENCES organizations(id) ON DELETE CASCADE,
    
    -- Component identification
    component_name VARCHAR(255) NOT NULL,
    component_path VARCHAR(500) NOT NULL, -- e.g., "src/components/PharmacyList.tsx"
    component_type VARCHAR(50) NOT NULL,  -- "page", "component", "button", "form", "modal", etc.
    
    -- Component metadata
    display_name VARCHAR(255),
    description TEXT,
    route_path VARCHAR(500), -- For pages/routes
    parent_component_id UUID REFERENCES ui_components(id) ON DELETE CASCADE, -- For nested components
    
    -- Permissions and access
    requires_permission VARCHAR(255), -- Zanzibar permission string
    required_roles TEXT[], -- Array of role names
    sensitive BOOLEAN DEFAULT false,
    
    -- UI metadata
    icon VARCHAR(100),
    category VARCHAR(100), -- e.g., "pharmacy", "healthcare", "admin"
    tags TEXT[], -- For search/filtering
    
    -- Component structure (for buttons, forms, etc.)
    component_props JSONB, -- Component props schema
    actions JSONB, -- Array of actions/buttons within component
    
    -- Registration metadata
    auto_discovered BOOLEAN DEFAULT true,
    registered_at TIMESTAMPTZ DEFAULT NOW(),
    registered_by UUID REFERENCES users(id),
    
    -- Status
    is_active BOOLEAN DEFAULT true,
    is_deleted BOOLEAN DEFAULT false,
    deleted_at TIMESTAMPTZ,
    
    created_at TIMESTAMPTZ DEFAULT NOW(),
    updated_at TIMESTAMPTZ DEFAULT NOW(),
    
    UNIQUE(organization_id, component_path)
);

CREATE INDEX idx_ui_components_type ON ui_components(component_type);
CREATE INDEX idx_ui_components_category ON ui_components(category);
CREATE INDEX idx_ui_components_parent ON ui_components(parent_component_id) WHERE parent_component_id IS NOT NULL;
CREATE INDEX idx_ui_components_active ON ui_components(organization_id, is_active, is_deleted) WHERE is_active = true AND is_deleted = false;
CREATE INDEX idx_ui_components_permission ON ui_components(requires_permission) WHERE requires_permission IS NOT NULL;
CREATE INDEX idx_ui_components_route ON ui_components(route_path) WHERE route_path IS NOT NULL;

-- Component Actions (buttons, form actions, etc.)
CREATE TABLE IF NOT EXISTS ui_component_actions (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    component_id UUID NOT NULL REFERENCES ui_components(id) ON DELETE CASCADE,
    
    -- Action identification
    action_name VARCHAR(255) NOT NULL,
    action_type VARCHAR(50) NOT NULL, -- "button", "link", "form_submit", "api_call", etc.
    
    -- Action metadata
    display_label VARCHAR(255),
    description TEXT,
    icon VARCHAR(100),
    
    -- Permissions
    requires_permission VARCHAR(255),
    required_roles TEXT[],
    sensitive BOOLEAN DEFAULT false,
    
    -- Action configuration
    action_config JSONB, -- e.g., { "api_endpoint": "/api/v1/pharmacy/pharmacies", "method": "POST" }
    handler_function VARCHAR(255), -- Backend handler function name
    
    -- UI properties
    variant VARCHAR(50), -- "primary", "secondary", "danger", etc.
    size VARCHAR(50),    -- "sm", "md", "lg"
    disabled_condition TEXT, -- Expression for when action is disabled
    
    -- Ordering
    display_order INTEGER DEFAULT 0,
    
    -- Status
    is_active BOOLEAN DEFAULT true,
    is_deleted BOOLEAN DEFAULT false,
    deleted_at TIMESTAMPTZ,
    
    created_at TIMESTAMPTZ DEFAULT NOW(),
    updated_at TIMESTAMPTZ DEFAULT NOW(),
    
    UNIQUE(component_id, action_name)
);

CREATE INDEX idx_component_actions_component ON ui_component_actions(component_id, is_active, is_deleted) WHERE is_active = true AND is_deleted = false;
CREATE INDEX idx_component_actions_permission ON ui_component_actions(requires_permission) WHERE requires_permission IS NOT NULL;

-- RLS Policies
ALTER TABLE mcp_tools ENABLE ROW LEVEL SECURITY;
ALTER TABLE ui_components ENABLE ROW LEVEL SECURITY;
ALTER TABLE ui_component_actions ENABLE ROW LEVEL SECURITY;

-- RLS for mcp_tools
CREATE POLICY mcp_tools_org_isolation ON mcp_tools
    USING (organization_id = current_setting('app.current_organization_id', true)::UUID);

CREATE POLICY mcp_tools_select ON mcp_tools
    FOR SELECT
    USING (
        organization_id = current_setting('app.current_organization_id', true)::UUID
        AND is_active = true
        AND is_deleted = false
    );

-- RLS for ui_components
CREATE POLICY ui_components_org_isolation ON ui_components
    USING (organization_id = current_setting('app.current_organization_id', true)::UUID);

CREATE POLICY ui_components_select ON ui_components
    FOR SELECT
    USING (
        organization_id = current_setting('app.current_organization_id', true)::UUID
        AND is_active = true
        AND is_deleted = false
    );

-- RLS for ui_component_actions
CREATE POLICY ui_component_actions_org_isolation ON ui_component_actions
    USING (
        component_id IN (
            SELECT id FROM ui_components 
            WHERE organization_id = current_setting('app.current_organization_id', true)::UUID
        )
    );

CREATE POLICY ui_component_actions_select ON ui_component_actions
    FOR SELECT
    USING (
        component_id IN (
            SELECT id FROM ui_components 
            WHERE organization_id = current_setting('app.current_organization_id', true)::UUID
            AND is_active = true
            AND is_deleted = false
        )
        AND is_active = true
        AND is_deleted = false
    );

-- Comments
COMMENT ON TABLE mcp_tools IS 'Registry of MCP tools discovered from handler functions with #[mcp_tool] decorator';
COMMENT ON TABLE ui_components IS 'Registry of UI components discovered from React/TypeScript files with decorators';
COMMENT ON TABLE ui_component_actions IS 'Registry of actions (buttons, links, etc.) within UI components';

