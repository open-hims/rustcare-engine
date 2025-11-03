use axum::{
    extract::{Path, State},
    Json,
};
use serde::{Deserialize, Serialize};
use uuid::Uuid;
use utoipa::{ToSchema, IntoParams};
use crate::server::RustCareServer;
use crate::middleware::AuthContext;
use crate::types::pagination::PaginationParams;
use crate::utils::query_builder::PaginatedQuery;

/// Organization with setup configuration
#[derive(Debug, Serialize, Deserialize, ToSchema)]
pub struct Organization {
    pub id: Uuid,
    pub name: String,
    pub slug: String,
    pub organization_type: String, // clinic, hospital, practice, etc.
    pub description: Option<String>,
    pub email: Option<String>,
    pub phone: Option<String>,
    pub website: Option<String>,
    pub address: Option<String>,
    pub city: Option<String>,
    pub state: Option<String>,
    pub postal_code: Option<String>,
    pub country: String,
    pub timezone: String,
    pub license_number: Option<String>,
    pub license_authority: Option<String>,
    pub license_valid_until: Option<String>,
    pub compliance_frameworks: Vec<Uuid>,
    pub geographic_regions: Vec<Uuid>,
    pub settings: serde_json::Value,
    pub is_active: bool,
    pub created_at: String,
    pub updated_at: String,
}

/// Organization setup wizard request
#[derive(Debug, Deserialize, ToSchema)]
pub struct OrganizationSetupRequest {
    pub name: String,
    pub organization_type: String,
    pub description: Option<String>,
    pub email: Option<String>,
    pub phone: Option<String>,
    pub website: Option<String>,
    pub address: Option<String>,
    pub city: Option<String>,
    pub state: Option<String>,
    pub postal_code: Option<String>,
    pub country: String,
    pub timezone: String,
    pub license_number: Option<String>,
    pub license_authority: Option<String>,
    pub license_valid_until: Option<String>,
    pub auto_assign_compliance: bool,
    pub settings: Option<serde_json::Value>,
}

/// Organization geographic presence
#[derive(Debug, Serialize, Deserialize, ToSchema)]
pub struct OrganizationRegion {
    pub id: Uuid,
    pub organization_id: Uuid,
    pub region_id: Uuid,
    pub presence_type: String, // headquarters, branch, service_area, etc.
    pub is_primary: bool,
    pub operational_since: Option<String>,
    pub status: String,
    pub license_number: Option<String>,
    pub license_authority: Option<String>,
    pub license_valid_from: Option<String>,
    pub license_valid_until: Option<String>,
    pub address: Option<String>,
    pub phone: Option<String>,
    pub email: Option<String>,
}

/// Role definition with Zanzibar integration
#[derive(Debug, Serialize, Deserialize, ToSchema)]
pub struct Role {
    pub id: Uuid,
    pub organization_id: Uuid,
    pub name: String,
    pub code: String,
    pub description: Option<String>,
    pub role_type: String, // system, department, custom
    pub permissions: Vec<String>,
    pub zanzibar_namespace: String,
    pub zanzibar_relations: Vec<String>,
    pub department_scope: Option<String>,
    pub resource_scope: Vec<String>,
    pub time_restrictions: Option<serde_json::Value>,
    pub approval_required: bool,
    pub is_active: bool,
    pub created_at: String,
    pub updated_at: String,
}

/// Employee with role assignments
#[derive(Debug, Serialize, Deserialize, ToSchema)]
pub struct Employee {
    pub id: Uuid,
    pub user_id: Uuid,
    pub organization_id: Uuid,
    pub employee_id: String,
    pub first_name: String,
    pub last_name: String,
    pub email: String,
    pub phone: Option<String>,
    pub department: Option<String>,
    pub position: Option<String>,
    pub roles: Vec<Uuid>,
    pub direct_permissions: Vec<String>,
    pub start_date: String,
    pub end_date: Option<String>,
    pub is_active: bool,
    pub last_login: Option<String>,
    pub zanzibar_subject_id: String,
}

/// Patient with access control
#[derive(Debug, Serialize, Deserialize, ToSchema)]
pub struct Patient {
    pub id: Uuid,
    pub organization_id: Uuid,
    pub patient_id: String,
    pub first_name: String,
    pub last_name: String,
    pub date_of_birth: String,
    pub email: Option<String>,
    pub phone: Option<String>,
    pub assigned_department: Option<String>,
    pub primary_provider: Option<Uuid>,
    pub access_level: String, // public, restricted, confidential
    pub consent_status: String,
    pub created_at: String,
    pub updated_at: String,
}

/// Create role request
#[derive(Debug, Deserialize, ToSchema)]
pub struct CreateRoleRequest {
    pub name: String,
    pub description: Option<String>,
    pub permissions: Vec<String>,
}

/// Role assignment request
#[derive(Debug, Deserialize, ToSchema)]
pub struct AssignRoleRequest {
    pub employee_id: Uuid,
    pub role_id: Uuid,
    pub department_scope: Option<String>,
    pub resource_scope: Vec<String>,
    pub valid_from: Option<String>,
    pub valid_until: Option<String>,
    pub approval_reason: Option<String>,
}

/// Zanzibar tuple for role assignment
#[derive(Debug, Serialize, Deserialize, ToSchema)]
pub struct ZanzibarTuple {
    pub subject_namespace: String,
    pub subject_type: String,
    pub subject_id: String,
    pub subject_relation: Option<String>,
    pub relation_name: String,
    pub object_namespace: String,
    pub object_type: String,
    pub object_id: String,
    pub expires_at: Option<String>,
}

#[derive(Debug, Deserialize, IntoParams)]
pub struct ListOrganizationsParams {
    pub is_active: Option<bool>,
    pub country: Option<String>,
    #[serde(flatten)]
    pub pagination: PaginationParams,
}

/// List organizations
#[utoipa::path(
    get,
    path = "/api/v1/organizations",
    responses(
        (status = 200, description = "Organizations retrieved successfully", body = Vec<Organization>),
        (status = 401, description = "Unauthorized"),
        (status = 500, description = "Internal server error")
    ),
    tag = "organizations",
    security(
        ("bearer_auth" = [])
    )
)]
pub async fn list_organizations(
    State(server): State<RustCareServer>,
    auth: AuthContext,
) -> Result<Json<crate::error::ApiResponse<Vec<Organization>>>, crate::error::ApiError> {
    use crate::error::{ApiError, api_success};
    
    let mut query_builder = PaginatedQuery::new(
        "SELECT \
            id, name, slug, description, contact_email, contact_phone, \
            website_url, address_line1, city, state_province, postal_code, country, \
            settings, subscription_tier, is_active, created_at, updated_at \
         FROM organizations \
         WHERE deleted_at IS NULL"
    );

    query_builder
        .filter_eq("is_active", Some(true))
        .order_by("created_at", "DESC")
        .paginate(None, None);

    let orgs = query_builder.build_query_as::<(Uuid, String, String, Option<String>, Option<String>, Option<String>, Option<String>, Option<String>, Option<String>, Option<String>, Option<String>, Option<String>, serde_json::Value, String, bool, chrono::DateTime<chrono::Utc>, chrono::DateTime<chrono::Utc>)>()
        .fetch_all(&server.db_pool)
        .await
        .map_err(|e| ApiError::internal(format!("Failed to fetch organizations: {}", e)))?;

    let mut organizations = Vec::new();
    for org in orgs {
        let (id, name, slug, description, contact_email, contact_phone, website_url, address_line1, city, state_province, postal_code, country, settings, _subscription_tier, is_active, created_at, updated_at) = org;
        let settings_obj = settings.as_object();
        let org_type = settings_obj
            .and_then(|s| s.get("organization_type"))
            .and_then(|v| v.as_str())
            .unwrap_or("clinic")
            .to_string();
        let timezone = settings_obj
            .and_then(|s| s.get("timezone"))
            .and_then(|v| v.as_str())
            .unwrap_or("UTC")
            .to_string();
        let license_number = settings_obj.and_then(|s| s.get("license_number")).and_then(|v| v.as_str()).map(|s| s.to_string());
        let license_authority = settings_obj.and_then(|s| s.get("license_authority")).and_then(|v| v.as_str()).map(|s| s.to_string());
        let license_valid_until = settings_obj.and_then(|s| s.get("license_valid_until")).and_then(|v| v.as_str()).map(|s| s.to_string());

        organizations.push(Organization {
            id,
            name,
            slug,
            organization_type: org_type,
            description,
            email: contact_email,
            phone: contact_phone,
            website: website_url,
            address: address_line1,
            city,
            state: state_province,
            postal_code,
            country: country.unwrap_or_else(|| "US".to_string()),
            timezone,
            license_number,
            license_authority,
            license_valid_until,
            compliance_frameworks: Vec::new(),
            geographic_regions: Vec::new(),
            settings,
            is_active,
            created_at: created_at.to_rfc3339(),
            updated_at: updated_at.to_rfc3339(),
        });
    }
    
    Ok(Json(api_success(organizations)))
}

/// Create organization with setup wizard
#[utoipa::path(
    post,
    path = "/api/v1/organizations",
    request_body = OrganizationSetupRequest,
    responses(
        (status = 201, description = "Organization created successfully", body = Organization),
        (status = 400, description = "Invalid request"),
        (status = 401, description = "Unauthorized"),
        (status = 500, description = "Internal server error")
    ),
    tag = "organizations",
    security(
        ("bearer_auth" = [])
    )
)]
pub async fn create_organization(
    State(server): State<RustCareServer>,
    Json(request): Json<OrganizationSetupRequest>,
) -> Result<Json<crate::error::ApiResponse<Organization>>, crate::error::ApiError> {
    use crate::error::{ApiError, api_success};
    
    // Validate required fields
    if request.name.trim().is_empty() {
        return Err(ApiError::validation("Organization name is required"));
    }
    
    // Generate slug from name
    let slug = request.name
        .to_lowercase()
        .trim()
        .replace(|c: char| !c.is_alphanumeric() && c != '-', "-")
        .replace("--", "-")
        .trim_matches('-')
        .to_string();
    
    // Validate slug uniqueness
    let existing_org = sqlx::query!(
        "SELECT id FROM organizations WHERE slug = $1 AND deleted_at IS NULL",
        slug
    )
    .fetch_optional(&server.db_pool)
    .await
    .map_err(|e| ApiError::internal(format!("Failed to check existing organization: {}", e)))?;
    
    if existing_org.is_some() {
        return Err(ApiError::conflict("An organization with this name already exists"));
    }
    
    let org_id = Uuid::new_v4();
    
    // Prepare settings with organization type
    let mut settings = request.settings.unwrap_or_else(|| serde_json::json!({}));
    if let Some(obj) = settings.as_object_mut() {
        obj.insert("organization_type".to_string(), serde_json::json!(request.organization_type));
        obj.insert("license_number".to_string(), serde_json::json!(request.license_number));
        obj.insert("license_authority".to_string(), serde_json::json!(request.license_authority));
        obj.insert("license_valid_until".to_string(), serde_json::json!(request.license_valid_until));
        obj.insert("timezone".to_string(), serde_json::json!(request.timezone));
    }
    
    // Create organization
    sqlx::query!(
        r#"
        INSERT INTO organizations (
            id, name, slug, description, contact_email, contact_phone, 
            website_url, address_line1, city, state_province, postal_code, country,
            settings, subscription_tier, is_active, is_verified
        )
        VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12, $13, 'free', true, false)
        "#,
        org_id,
        request.name,
        slug,
        request.description,
        request.email,
        request.phone,
        request.website,
        request.address,
        request.city,
        request.state,
        request.postal_code,
        request.country,
        settings
    )
    .execute(&server.db_pool)
    .await
    .map_err(|e| ApiError::internal(format!("Failed to create organization: {}", e)))?;
    
    // Auto-detect geographic regions based on country
    let mut geographic_regions = Vec::new();
    if request.auto_assign_compliance {
        // Query geographic regions matching the country
        let regions = sqlx::query!(
            r#"
            SELECT id
            FROM geographic_regions
            WHERE iso_country_code = $1 
              AND is_active = true
            ORDER BY level ASC
            LIMIT 5
            "#,
            request.country
        )
        .fetch_all(&server.db_pool)
        .await
        .unwrap_or_default();
        
        geographic_regions = regions.into_iter().map(|r| r.id).collect();
    }
    
    // Auto-assign compliance frameworks based on country jurisdiction
    let mut compliance_frameworks = Vec::new();
    if request.auto_assign_compliance {
        let frameworks = sqlx::query!(
            r#"
            SELECT cf.id, cf.effective_date
            FROM compliance_frameworks cf
            WHERE cf.jurisdiction = $1
              AND cf.status = 'active'
            ORDER BY cf.effective_date DESC
            LIMIT 10
            "#,
            request.country
        )
        .fetch_all(&server.db_pool)
        .await
        .unwrap_or_default();
        
        compliance_frameworks = frameworks.into_iter().map(|f| f.id).collect();
    }
    
    tracing::info!(
        org_id = %org_id,
        slug = %slug,
        regions = geographic_regions.len(),
        frameworks = compliance_frameworks.len(),
        "Successfully created organization with auto-assigned compliance"
    );
    
    let organization = Organization {
        id: org_id,
        name: request.name,
        slug,
        organization_type: request.organization_type,
        description: request.description,
        email: request.email,
        phone: request.phone,
        website: request.website,
        address: request.address,
        city: request.city,
        state: request.state,
        postal_code: request.postal_code,
        country: request.country,
        timezone: request.timezone,
        license_number: request.license_number,
        license_authority: request.license_authority,
        license_valid_until: request.license_valid_until,
        compliance_frameworks,
        geographic_regions,
        settings,
        is_active: true,
        created_at: chrono::Utc::now().to_rfc3339(),
        updated_at: chrono::Utc::now().to_rfc3339(),
    };

    Ok(Json(api_success(organization)))
}

/// List roles for organization
#[utoipa::path(
    get,
    path = "/api/v1/organizations/{org_id}/roles",
    params(
        ("org_id" = Uuid, Path, description = "Organization ID")
    ),
    responses(
        (status = 200, description = "Roles retrieved successfully", body = Vec<Role>),
        (status = 401, description = "Unauthorized"),
        (status = 500, description = "Internal server error")
    ),
    tag = "organizations",
    security(
        ("bearer_auth" = [])
    )
)]
pub async fn list_organization_roles(
    State(server): State<RustCareServer>,
    Path(org_id): Path<Uuid>,
) -> Result<Json<crate::error::ApiResponse<Vec<Role>>>, crate::error::ApiError> {
    use crate::error::{ApiError, api_success};
    
    // Query roles for the organization
    let roles_data = sqlx::query!(
        r#"
        SELECT id, name, description, organization_id, created_at
        FROM roles
        WHERE organization_id = $1
        ORDER BY name ASC
        "#,
        org_id
    )
    .fetch_all(&server.db_pool)
    .await
    .map_err(|e| ApiError::internal(format!("Failed to fetch roles: {}", e)))?;
    
    let mut roles = Vec::new();
    for role_data in roles_data {
        // Fetch permissions for this role
        let permissions = sqlx::query!(
            r#"
            SELECT p.name
            FROM permissions p
            JOIN role_permissions rp ON p.id = rp.permission_id
            WHERE rp.role_id = $1
            "#,
            role_data.id
        )
        .fetch_all(&server.db_pool)
        .await
        .map(|perms| perms.into_iter().map(|p| p.name).collect())
        .unwrap_or_default();
        
        let code = role_data.name.to_lowercase().replace(' ', "_");
        
        roles.push(Role {
            id: role_data.id,
            organization_id: role_data.organization_id.unwrap_or(org_id),
            name: role_data.name,
            code,
            description: role_data.description,
            role_type: "custom".to_string(),
            permissions,
            zanzibar_namespace: "role".to_string(),
            zanzibar_relations: vec!["member".to_string()],
            department_scope: None,
            resource_scope: Vec::new(),
            time_restrictions: None,
            approval_required: false,
            is_active: true,
            created_at: role_data.created_at.to_rfc3339(),
            updated_at: role_data.created_at.to_rfc3339(),
        });
    }
    
    Ok(Json(api_success(roles)))
}

/// Create role with Zanzibar integration
#[utoipa::path(
    post,
    path = "/api/v1/organizations/{org_id}/roles",
    params(
        ("org_id" = Uuid, Path, description = "Organization ID")
    ),
    request_body = CreateRoleRequest,
    responses(
        (status = 201, description = "Role created successfully", body = Role),
        (status = 400, description = "Invalid request"),
        (status = 401, description = "Unauthorized"),
        (status = 500, description = "Internal server error")
    ),
    tag = "organizations",
    security(
        ("bearer_auth" = [])
    )
)]
pub async fn create_organization_role(
    State(server): State<RustCareServer>,
    Path(org_id): Path<Uuid>,
    Json(request): Json<CreateRoleRequest>,
) -> Result<Json<crate::error::ApiResponse<Role>>, crate::error::ApiError> {
    use crate::error::{ApiError, api_success};
    
    // Validate role name
    if request.name.trim().is_empty() {
        return Err(ApiError::validation("Role name is required"));
    }
    
    // Check for duplicate role name in organization
    let existing_role = sqlx::query!(
        "SELECT id FROM roles WHERE name = $1 AND organization_id = $2",
        request.name,
        org_id
    )
    .fetch_optional(&server.db_pool)
    .await
    .map_err(|e| ApiError::internal(format!("Failed to check existing role: {}", e)))?;
    
    if existing_role.is_some() {
        return Err(ApiError::conflict("A role with this name already exists in the organization"));
    }
    
    let role_id = Uuid::new_v4();
    
    // Create role
    let role_record = sqlx::query!(
        r#"
        INSERT INTO roles (id, name, description, organization_id, created_at)
        VALUES ($1, $2, $3, $4, NOW())
        RETURNING id, name, description, organization_id, created_at
        "#,
        role_id,
        request.name,
        request.description,
        org_id
    )
    .fetch_one(&server.db_pool)
    .await
    .map_err(|e| ApiError::internal(format!("Failed to create role: {}", e)))?;
    
    // Create/fetch permissions and associate with role
    let mut created_permissions = Vec::new();
    for perm_name in &request.permissions {
        // Create permission if it doesn't exist
        let perm = sqlx::query!(
            r#"
            INSERT INTO permissions (name, resource, action, description, created_at)
            VALUES ($1, 'general', 'access', $2, NOW())
            ON CONFLICT (name) DO UPDATE SET name = EXCLUDED.name
            RETURNING id, name
            "#,
            perm_name,
            format!("Permission: {}", perm_name)
        )
        .fetch_one(&server.db_pool)
        .await
        .map_err(|e| ApiError::internal(format!("Failed to create permission: {}", e)))?;
        
        // Associate permission with role
        sqlx::query!(
            r#"
            INSERT INTO role_permissions (role_id, permission_id, created_at)
            VALUES ($1, $2, NOW())
            ON CONFLICT (role_id, permission_id) DO NOTHING
            "#,
            role_id,
            perm.id
        )
        .execute(&server.db_pool)
        .await
        .map_err(|e| ApiError::internal(format!("Failed to associate permission with role: {}", e)))?;
        
        created_permissions.push(perm.name);
    }
    
    tracing::info!(
        role_id = %role_id,
        role_name = %request.name,
        org_id = %org_id,
        permissions_count = created_permissions.len(),
        "Successfully created role with permissions"
    );
    
    let code = request.name.to_lowercase().replace(' ', "_");
    
    let role = Role {
        id: role_record.id,
        organization_id: role_record.organization_id.unwrap_or(org_id),
        name: role_record.name,
        code,
        description: role_record.description,
        role_type: "custom".to_string(),
        permissions: created_permissions,
        zanzibar_namespace: "role".to_string(),
        zanzibar_relations: vec!["member".to_string()],
        department_scope: None,
        resource_scope: Vec::new(),
        time_restrictions: None,
        approval_required: false,
        is_active: true,
        created_at: role_record.created_at.to_rfc3339(),
        updated_at: role_record.created_at.to_rfc3339(),
    };

    Ok(Json(api_success(role)))
}

/// Get role templates for healthcare organizations
#[utoipa::path(
    get,
    path = "/api/v1/role-templates",
    responses(
        (status = 200, description = "Role templates retrieved successfully", body = Vec<serde_json::Value>),
        (status = 500, description = "Internal server error")
    ),
    tag = "organizations"
)]
pub async fn get_role_templates(
    State(_server): State<RustCareServer>,
) -> Result<Json<crate::error::ApiResponse<Vec<serde_json::Value>>>, crate::error::ApiError> {
    use crate::error::api_success;
    
    let templates = vec![
        serde_json::json!({
            "name": "Physician",
            "description": "Licensed medical doctor with full patient access and prescription authority",
            "permissions": [
                "read_patient", "write_patient", "read_medical_history", "write_diagnosis",
                "prescribe_medication", "order_lab_tests", "view_lab_results", "schedule_appointments"
            ]
        }),
        serde_json::json!({
            "name": "Nurse",
            "description": "Registered nurse with patient care and documentation access",
            "permissions": [
                "read_patient", "write_patient_notes", "read_medical_history", "view_lab_results",
                "administer_medication", "record_vitals", "schedule_appointments"
            ]
        }),
        serde_json::json!({
            "name": "Medical Assistant",
            "description": "Medical assistant with limited patient interaction and administrative duties",
            "permissions": [
                "read_patient", "record_vitals", "schedule_appointments", "check_in_patient",
                "update_demographics", "view_basic_info"
            ]
        }),
        serde_json::json!({
            "name": "Receptionist",
            "description": "Front desk staff with scheduling and basic patient information access",
            "permissions": [
                "schedule_appointments", "check_in_patient", "read_basic_patient_info",
                "update_demographics", "manage_billing_info"
            ]
        }),
        serde_json::json!({
            "name": "Administrator",
            "description": "System administrator with full access to organization settings",
            "permissions": [
                "manage_users", "manage_roles", "manage_organization", "view_reports",
                "manage_compliance", "configure_system", "view_audit_logs"
            ]
        }),
        serde_json::json!({
            "name": "Pharmacist",
            "description": "Licensed pharmacist with prescription and medication management access",
            "permissions": [
                "read_patient", "view_prescriptions", "dispense_medication", "check_interactions",
                "view_medical_history", "update_prescription_status"
            ]
        }),
        serde_json::json!({
            "name": "Lab Technician",
            "description": "Laboratory staff with test order and result access",
            "permissions": [
                "read_patient", "view_lab_orders", "enter_lab_results", "view_test_history",
                "manage_specimens", "quality_control"
            ]
        }),
    ];
    
    Ok(Json(api_success(templates)))
}

/// List employees for organization
#[utoipa::path(
    get,
    path = "/api/v1/organizations/{org_id}/employees",
    params(
        ("org_id" = Uuid, Path, description = "Organization ID")
    ),
    responses(
        (status = 200, description = "Employees retrieved successfully", body = Vec<Employee>),
        (status = 401, description = "Unauthorized"),
        (status = 500, description = "Internal server error")
    ),
    tag = "organizations",
    security(
        ("bearer_auth" = [])
    )
)]
pub async fn list_organization_employees(
    State(server): State<RustCareServer>,
    Path(org_id): Path<Uuid>,
    auth: AuthContext,
) -> Result<Json<crate::error::ApiResponse<Vec<Employee>>>, crate::error::ApiError> {
    use crate::error::{ApiError, api_success};

    if auth.organization_id != org_id {
        return Err(ApiError::authorization("Access denied"));
    }

    let employees = sqlx::query_as!(
        Employee,
        r#"SELECT 
            id, user_id, organization_id, employee_id, first_name, last_name, email,
            phone, department, position, ARRAY[]::uuid[] as roles, ARRAY[]::text[] as direct_permissions,
            to_char(start_date, 'YYYY-MM-DD') as start_date, NULL::text as end_date, is_active,
            NULL::text as last_login, '' as zanzibar_subject_id
          FROM organization_employees
          WHERE organization_id = $1 AND (is_deleted = false OR is_deleted IS NULL)
          ORDER BY created_at DESC"#,
        org_id
    )
    .fetch_all(&server.db_pool)
    .await
    .map_err(|e| ApiError::internal(format!("Failed to fetch employees: {}", e)))?;

    Ok(Json(api_success(employees)))
}

/// Assign role to employee with Zanzibar tuple creation
#[utoipa::path(
    post,
    path = "/api/v1/organizations/{org_id}/employees/{employee_id}/roles",
    params(
        ("org_id" = Uuid, Path, description = "Organization ID"),
        ("employee_id" = Uuid, Path, description = "Employee ID")
    ),
    request_body = AssignRoleRequest,
    responses(
        (status = 201, description = "Role assigned successfully", body = Vec<ZanzibarTuple>),
        (status = 400, description = "Invalid request"),
        (status = 401, description = "Unauthorized"),
        (status = 404, description = "Employee or role not found"),
        (status = 500, description = "Internal server error")
    ),
    tag = "organizations",
    security(
        ("bearer_auth" = [])
    )
)]
pub async fn assign_employee_role(
    State(server): State<RustCareServer>,
    Path((org_id, employee_id)): Path<(Uuid, Uuid)>,
    Json(request): Json<AssignRoleRequest>,
) -> Result<Json<crate::error::ApiResponse<Vec<ZanzibarTuple>>>, crate::error::ApiError> {
    use crate::error::{ApiError, api_success};
    
    // Validate that role exists and belongs to the organization
    let role = sqlx::query!(
        "SELECT id, name FROM roles WHERE id = $1 AND organization_id = $2",
        request.role_id,
        org_id
    )
    .fetch_optional(&server.db_pool)
    .await
    .map_err(|e| ApiError::internal(format!("Failed to fetch role: {}", e)))?;
    
    if role.is_none() {
        return Err(ApiError::not_found("role"));
    }
    
    let role = role.unwrap();
    
    // Validate that user/employee exists
    let user = sqlx::query!(
        "SELECT id, email FROM users WHERE id = $1",
        request.employee_id
    )
    .fetch_optional(&server.db_pool)
    .await
    .map_err(|e| ApiError::internal(format!("Failed to fetch user: {}", e)))?;
    
    if user.is_none() {
        return Err(ApiError::not_found("employee"));
    }
    
    let user = user.unwrap();
    
    // Parse optional expiration date
    let expires_at = request.valid_until.as_ref()
        .map(|date_str| {
            chrono::DateTime::parse_from_rfc3339(date_str)
                .map(|dt| dt.with_timezone(&chrono::Utc))
                .map_err(|_| ApiError::validation("Invalid valid_until date format. Expected RFC3339."))
        })
        .transpose()?;
    
    // Create Zanzibar tuple for role membership
    // Pattern: user:USER_ID#member@role:ROLE_ID
    let tuple_id = Uuid::new_v4();
    sqlx::query!(
        r#"
        INSERT INTO zanzibar_tuples (
            id, organization_id,
            subject_namespace, subject_type, subject_id, subject_relation,
            relation_name,
            object_namespace, object_type, object_id,
            created_at, created_by, expires_at
        )
        VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, NOW(), $11, $12)
        ON CONFLICT (organization_id, subject_namespace, subject_type, subject_id, subject_relation, relation_name, object_namespace, object_type, object_id)
        DO UPDATE SET expires_at = EXCLUDED.expires_at
        RETURNING id
        "#,
        tuple_id,
        org_id,
        "user", // subject_namespace
        "user", // subject_type
        user.id.to_string(), // subject_id
        None::<String>, // subject_relation
        "member", // relation_name
        "role", // object_namespace
        "role", // object_type
        role.id.to_string(), // object_id
        None::<Uuid>, // created_by (TODO: get from auth context)
        expires_at
    )
    .fetch_one(&server.db_pool)
    .await
    .map_err(|e| ApiError::internal(format!("Failed to create Zanzibar tuple: {}", e)))?;
    
    tracing::info!(
        tuple_id = %tuple_id,
        user_id = %user.id,
        role_id = %role.id,
        org_id = %org_id,
        "Successfully assigned role to user via Zanzibar tuple"
    );
    
    // Return created tuple
    let tuples = vec![ZanzibarTuple {
        subject_namespace: "user".to_string(),
        subject_type: "user".to_string(),
        subject_id: user.id.to_string(),
        subject_relation: None,
        relation_name: "member".to_string(),
        object_namespace: "role".to_string(),
        object_type: "role".to_string(),
        object_id: role.id.to_string(),
        expires_at: expires_at.map(|dt| dt.to_rfc3339()),
    }];
    
    Ok(Json(api_success(tuples)))
}

/// List patients for organization
#[utoipa::path(
    get,
    path = "/api/v1/organizations/{org_id}/patients",
    params(
        ("org_id" = Uuid, Path, description = "Organization ID")
    ),
    responses(
        (status = 200, description = "Patients retrieved successfully", body = Vec<Patient>),
        (status = 401, description = "Unauthorized"),
        (status = 500, description = "Internal server error")
    ),
    tag = "organizations",
    security(
        ("bearer_auth" = [])
    )
)]
pub async fn list_organization_patients(
    State(server): State<RustCareServer>,
    Path(org_id): Path<Uuid>,
    auth: AuthContext,
) -> Result<Json<crate::error::ApiResponse<Vec<Patient>>>, crate::error::ApiError> {
    use crate::error::{ApiError, api_success};

    if auth.organization_id != org_id {
        return Err(ApiError::authorization("Access denied"));
    }

    let patients = sqlx::query_as!(
        Patient,
        r#"SELECT 
            id, organization_id, patient_id, first_name, last_name,
            to_char(date_of_birth, 'YYYY-MM-DD') as date_of_birth,
            email, phone, assigned_department, primary_provider, access_level,
            consent_status, to_char(created_at, 'YYYY-MM-DD"T"HH24:MI:SS"Z"') as created_at,
            to_char(updated_at, 'YYYY-MM-DD"T"HH24:MI:SS"Z"') as updated_at
          FROM patients
          WHERE organization_id = $1 AND (is_deleted = false OR is_deleted IS NULL)
          ORDER BY created_at DESC"#,
        org_id
    )
    .fetch_all(&server.db_pool)
    .await
    .map_err(|e| ApiError::internal(format!("Failed to fetch patients: {}", e)))?;

    Ok(Json(api_success(patients)))
}

/// Create patient with access control
#[utoipa::path(
    post,
    path = "/api/v1/organizations/{org_id}/patients",
    params(
        ("org_id" = Uuid, Path, description = "Organization ID")
    ),
    responses(
        (status = 201, description = "Patient created successfully", body = Patient),
        (status = 400, description = "Invalid request"),
        (status = 401, description = "Unauthorized"),
        (status = 500, description = "Internal server error")
    ),
    tag = "organizations",
    security(
        ("bearer_auth" = [])
    )
)]
pub async fn create_organization_patient(
    State(_server): State<RustCareServer>,
    Path(_org_id): Path<Uuid>,
    Json(_request): Json<serde_json::Value>, // TODO: Define proper request type
) -> Result<Json<crate::error::ApiResponse<Patient>>, crate::error::ApiError> {
    use crate::error::api_success;
    
    // TODO: Implement patient creation with Zanzibar access control setup
    let patient = Patient {
        id: Uuid::new_v4(),
        organization_id: Uuid::new_v4(),
        patient_id: "P001".to_string(),
        first_name: "John".to_string(),
        last_name: "Doe".to_string(),
        date_of_birth: "1990-01-01".to_string(),
        email: None,
        phone: None,
        assigned_department: Some("Cardiology".to_string()),
        primary_provider: None,
        access_level: "restricted".to_string(),
        consent_status: "active".to_string(),
        created_at: chrono::Utc::now().to_rfc3339(),
        updated_at: chrono::Utc::now().to_rfc3339(),
    };

    Ok(Json(api_success(patient)))
}