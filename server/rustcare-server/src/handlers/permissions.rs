//! Permission Management Handlers
//!
//! Provides APIs for managing Zanzibar permissions:
//! - Resources (screens, APIs, modules)
//! - Groups (collection of permissions)
//! - Roles (predefined permission sets)
//! - User assignments

use axum::{
    extract::{Path, Query, State},
    http::StatusCode,
    Json,
};
use serde::{Deserialize, Serialize};
use utoipa::ToSchema;
use uuid::Uuid;
use crate::server::RustCareServer;
use crate::error::{ApiError, ApiResponse, api_success};

// ============================================================================
// Resource Management (Screens, APIs, Modules)
// ============================================================================

/// Resource type in the system
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "lowercase")]
pub enum ResourceType {
    /// UI Screen/Page
    Screen,
    /// API Endpoint
    Api,
    /// Feature Module
    Module,
    /// Data Entity
    Entity,
}

/// Resource definition (screen, API, module)
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct Resource {
    /// Unique resource ID
    pub id: Uuid,
    
    /// Resource type
    pub resource_type: ResourceType,
    
    /// Resource name
    #[schema(example = "patient-list")]
    pub name: String,
    
    /// Human-readable description
    #[schema(example = "Patient list screen")]
    pub description: String,
    
    /// Route/path for screens, endpoint for APIs
    #[schema(example = "/patients")]
    pub path: Option<String>,
    
    /// Available actions on this resource
    #[schema(example = json!(["read", "write", "delete"]))]
    pub actions: Vec<String>,
    
    /// Parent module (if applicable)
    pub parent_module: Option<Uuid>,
    
    /// Tags for categorization
    #[schema(example = json!(["patient-management", "phi"]))]
    pub tags: Vec<String>,
    
    /// Whether this resource contains PHI
    pub contains_phi: bool,
    
    /// Created timestamp
    pub created_at: String,
    
    /// Last modified timestamp
    pub updated_at: String,
}

/// Request to create a new resource
#[derive(Debug, Deserialize, ToSchema)]
pub struct CreateResourceRequest {
    pub resource_type: ResourceType,
    pub name: String,
    pub description: String,
    pub path: Option<String>,
    pub actions: Vec<String>,
    pub parent_module: Option<Uuid>,
    pub tags: Vec<String>,
    pub contains_phi: bool,
}

impl RequestValidation for CreateResourceRequest {
    fn validate(&self) -> Result<(), ApiError> {
        validate_required!(self.name, "Resource name is required");
        validate_required!(self.description, "Description is required");
        
        validate_length!(self.name, 1, 100, "Name must be between 1 and 100 characters");
        validate_length!(self.description, 1, 500, "Description must be between 1 and 500 characters");
        
        Ok(())
    }
}

/// List all resources
#[utoipa::path(
    get,
    path = "/api/permissions/resources",
    tag = "Permission Management",
    responses(
        (status = 200, description = "List of all resources", body = Vec<Resource>),
        (status = 401, description = "Unauthorized"),
    ),
    security(("bearer_auth" = []))
)]
pub async fn list_resources(
    State(server): State<RustCareServer>,
    Query(params): Query<std::collections::HashMap<String, String>>,
) -> Result<Json<Vec<Resource>>, StatusCode> {
    // TODO: Query database for resources
    // Filter by resource_type, tags, parent_module if provided in query params
    
    let resources = vec![
        Resource {
            id: Uuid::new_v4(),
            resource_type: ResourceType::Screen,
            name: "patient-list".to_string(),
            description: "Patient list screen".to_string(),
            path: Some("/patients".to_string()),
            actions: vec!["read".to_string()],
            parent_module: None,
            tags: vec!["patient-management".to_string()],
            contains_phi: true,
            created_at: chrono::Utc::now().to_rfc3339(),
            updated_at: chrono::Utc::now().to_rfc3339(),
        },
        Resource {
            id: Uuid::new_v4(),
            resource_type: ResourceType::Api,
            name: "patient-api".to_string(),
            description: "Patient CRUD API".to_string(),
            path: Some("/api/patients".to_string()),
            actions: vec!["read".to_string(), "write".to_string(), "delete".to_string()],
            parent_module: None,
            tags: vec!["patient-management".to_string(), "api".to_string()],
            contains_phi: true,
            created_at: chrono::Utc::now().to_rfc3339(),
            updated_at: chrono::Utc::now().to_rfc3339(),
        },
        Resource {
            id: Uuid::new_v4(),
            resource_type: ResourceType::Module,
            name: "patient-management".to_string(),
            description: "Patient management module".to_string(),
            path: None,
            actions: vec!["access".to_string()],
            parent_module: None,
            tags: vec!["core".to_string()],
            contains_phi: true,
            created_at: chrono::Utc::now().to_rfc3339(),
            updated_at: chrono::Utc::now().to_rfc3339(),
        },
    ];
    
    Ok(Json(resources))
}

/// Create a new resource
#[utoipa::path(
    post,
    path = "/api/permissions/resources",
    tag = "Permission Management",
    request_body = CreateResourceRequest,
    responses(
        (status = 201, description = "Resource created", body = Resource),
        (status = 400, description = "Invalid request"),
        (status = 401, description = "Unauthorized"),
    ),
    security(("bearer_auth" = []))
)]
pub async fn create_resource(
    State(server): State<RustCareServer>,
    Json(request): Json<CreateResourceRequest>,
    auth: AuthContext,
) -> Result<(StatusCode, Json<ApiResponse<Resource>>), ApiError> {
    // Validate request
    request.validate()?;
    // TODO: Validate and insert into database
    // TODO: Register in Zanzibar as a namespace/resource type
    
    let resource = Resource {
        id: Uuid::new_v4(),
        resource_type: request.resource_type,
        name: request.name,
        description: request.description,
        path: request.path,
        actions: request.actions,
        parent_module: request.parent_module,
        tags: request.tags,
        contains_phi: request.contains_phi,
        created_at: chrono::Utc::now().to_rfc3339(),
        updated_at: chrono::Utc::now().to_rfc3339(),
    };
    
    // Log the creation using AuditService
    let audit_service = AuditService::new(server.db_pool.clone());
    let _ = audit_service.log_general_action(
        &auth,
        "resource",
        resource.id,
        "created",
        Some(serde_json::json!({"name": request.name, "type": format!("{:?}", request.resource_type)})),
    ).await;
    
    Ok((StatusCode::CREATED, Json(api_success(resource))))
}

// ============================================================================
// Permission Groups
// ============================================================================

/// Permission group (collection of permissions)
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct PermissionGroup {
    /// Unique group ID
    pub id: Uuid,
    
    /// Group name
    #[schema(example = "patient-viewers")]
    pub name: String,
    
    /// Description
    #[schema(example = "Can view patient records")]
    pub description: String,
    
    /// List of permissions in this group
    /// Format: "resource:action" (e.g., "patient:read")
    #[schema(example = json!(["patient:read", "appointment:read"]))]
    pub permissions: Vec<String>,
    
    /// Members (user IDs)
    pub members: Vec<Uuid>,
    
    /// Created timestamp
    pub created_at: String,
    
    /// Last modified timestamp
    pub updated_at: String,
}

/// Request to create a permission group
#[derive(Debug, Deserialize, ToSchema)]
pub struct CreateGroupRequest {
    pub name: String,
    pub description: String,
    pub permissions: Vec<String>,
}

impl RequestValidation for CreateGroupRequest {
    fn validate(&self) -> Result<(), ApiError> {
        validate_required!(self.name, "Group name is required");
        validate_required!(self.description, "Description is required");
        
        validate_length!(self.name, 1, 100, "Name must be between 1 and 100 characters");
        validate_length!(self.description, 1, 500, "Description must be between 1 and 500 characters");
        
        Ok(())
    }
}

/// List all permission groups
#[utoipa::path(
    get,
    path = "/api/permissions/groups",
    tag = "Permission Management",
    responses(
        (status = 200, description = "List of permission groups", body = Vec<PermissionGroup>),
        (status = 401, description = "Unauthorized"),
    ),
    security(("bearer_auth" = []))
)]
pub async fn list_groups(
    State(server): State<RustCareServer>,
) -> Result<Json<Vec<PermissionGroup>>, StatusCode> {
    // TODO: Query database for groups
    
    let groups = vec![
        PermissionGroup {
            id: Uuid::new_v4(),
            name: "patient-viewers".to_string(),
            description: "Can view patient records".to_string(),
            permissions: vec!["patient:read".to_string(), "appointment:read".to_string()],
            members: vec![],
            created_at: chrono::Utc::now().to_rfc3339(),
            updated_at: chrono::Utc::now().to_rfc3339(),
        },
        PermissionGroup {
            id: Uuid::new_v4(),
            name: "patient-editors".to_string(),
            description: "Can edit patient records".to_string(),
            permissions: vec![
                "patient:read".to_string(),
                "patient:write".to_string(),
                "appointment:read".to_string(),
                "appointment:write".to_string(),
            ],
            members: vec![],
            created_at: chrono::Utc::now().to_rfc3339(),
            updated_at: chrono::Utc::now().to_rfc3339(),
        },
    ];
    
    Ok(Json(groups))
}

/// Create a new permission group
#[utoipa::path(
    post,
    path = "/api/permissions/groups",
    tag = "Permission Management",
    request_body = CreateGroupRequest,
    responses(
        (status = 201, description = "Group created", body = PermissionGroup),
        (status = 400, description = "Invalid request"),
        (status = 401, description = "Unauthorized"),
    ),
    security(("bearer_auth" = []))
)]
pub async fn create_group(
    State(server): State<RustCareServer>,
    Json(request): Json<CreateGroupRequest>,
    auth: AuthContext,
) -> Result<(StatusCode, Json<ApiResponse<PermissionGroup>>), ApiError> {
    // Validate request
    request.validate()?;
    
    // TODO: Validate and insert into database
    
    let group = PermissionGroup {
        id: Uuid::new_v4(),
        name: request.name,
        description: request.description,
        permissions: request.permissions,
        members: vec![],
        created_at: chrono::Utc::now().to_rfc3339(),
        updated_at: chrono::Utc::now().to_rfc3339(),
    };
    
    // Log the creation using AuditService
    let audit_service = AuditService::new(server.db_pool.clone());
    let _ = audit_service.log_general_action(
        &auth,
        "permission_group",
        group.id,
        "created",
        Some(serde_json::json!({"name": request.name})),
    ).await;
    
    Ok((StatusCode::CREATED, Json(api_success(group))))
}

// ============================================================================
// Roles
// ============================================================================

/// Role (predefined permission set)
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct Role {
    /// Unique role ID
    pub id: Uuid,
    
    /// Role name
    #[schema(example = "doctor")]
    pub name: String,
    
    /// Description
    #[schema(example = "Medical doctor with full patient access")]
    pub description: String,
    
    /// Permission groups included in this role
    pub groups: Vec<Uuid>,
    
    /// Direct permissions (not from groups)
    #[schema(example = json!(["phi:ssn:read", "phi:diagnosis:read"]))]
    pub direct_permissions: Vec<String>,
    
    /// Whether this is a system role (cannot be deleted)
    pub is_system_role: bool,
    
    /// Members (user IDs)
    pub members: Vec<Uuid>,
    
    /// Created timestamp
    pub created_at: String,
    
    /// Last modified timestamp
    pub updated_at: String,
}

/// Request to create a role
#[derive(Debug, Deserialize, ToSchema)]
pub struct CreateRoleRequest {
    pub name: String,
    pub description: String,
    pub groups: Vec<Uuid>,
    pub direct_permissions: Vec<String>,
}

/// List all roles
#[utoipa::path(
    get,
    path = "/api/permissions/roles",
    tag = "Permission Management",
    responses(
        (status = 200, description = "List of roles", body = Vec<Role>),
        (status = 401, description = "Unauthorized"),
    ),
    security(("bearer_auth" = []))
)]
pub async fn list_roles(
    State(server): State<RustCareServer>,
) -> Result<Json<Vec<Role>>, StatusCode> {
    // TODO: Query database for roles
    
    let roles = vec![
        Role {
            id: Uuid::new_v4(),
            name: "doctor".to_string(),
            description: "Medical doctor with full patient access".to_string(),
            groups: vec![],
            direct_permissions: vec![
                "patient:read".to_string(),
                "patient:write".to_string(),
                "phi:ssn:read".to_string(),
                "phi:diagnosis:read".to_string(),
                "appointment:read".to_string(),
                "appointment:write".to_string(),
            ],
            is_system_role: true,
            members: vec![],
            created_at: chrono::Utc::now().to_rfc3339(),
            updated_at: chrono::Utc::now().to_rfc3339(),
        },
        Role {
            id: Uuid::new_v4(),
            name: "nurse".to_string(),
            description: "Nurse with limited patient access".to_string(),
            groups: vec![],
            direct_permissions: vec![
                "patient:read".to_string(),
                "appointment:read".to_string(),
                "phi:dob:read".to_string(),
            ],
            is_system_role: true,
            members: vec![],
            created_at: chrono::Utc::now().to_rfc3339(),
            updated_at: chrono::Utc::now().to_rfc3339(),
        },
        Role {
            id: Uuid::new_v4(),
            name: "admin".to_string(),
            description: "System administrator".to_string(),
            groups: vec![],
            direct_permissions: vec!["admin:read".to_string(), "admin:write".to_string()],
            is_system_role: true,
            members: vec![],
            created_at: chrono::Utc::now().to_rfc3339(),
            updated_at: chrono::Utc::now().to_rfc3339(),
        },
    ];
    
    Ok(Json(roles))
}

/// Create a new role
#[utoipa::path(
    post,
    path = "/api/permissions/roles",
    tag = "Permission Management",
    request_body = CreateRoleRequest,
    responses(
        (status = 201, description = "Role created", body = Role),
        (status = 400, description = "Invalid request"),
        (status = 401, description = "Unauthorized"),
    ),
    security(("bearer_auth" = []))
)]
pub async fn create_role(
    State(server): State<RustCareServer>,
    Json(request): Json<CreateRoleRequest>,
    auth: AuthContext,
) -> Result<(StatusCode, Json<ApiResponse<Role>>), ApiError> {
    // Validate request
    request.validate()?;
    
    // TODO: Validate and insert into database
    
    let role = Role {
        id: Uuid::new_v4(),
        name: request.name,
        description: request.description,
        groups: request.groups,
        direct_permissions: request.direct_permissions,
        is_system_role: false,
        members: vec![],
        created_at: chrono::Utc::now().to_rfc3339(),
        updated_at: chrono::Utc::now().to_rfc3339(),
    };
    
    // Log the creation using AuditService
    let audit_service = AuditService::new(server.db_pool.clone());
    let _ = audit_service.log_general_action(
        &auth,
        "role",
        role.id,
        "created",
        Some(serde_json::json!({"name": request.name})),
    ).await;
    
    Ok((StatusCode::CREATED, Json(api_success(role))))
}

// ============================================================================
// User Permission Assignment
// ============================================================================

/// User permission assignment
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct UserPermissions {
    /// User ID
    pub user_id: Uuid,
    
    /// Assigned roles
    pub roles: Vec<Uuid>,
    
    /// Assigned groups
    pub groups: Vec<Uuid>,
    
    /// Direct permissions (individual)
    #[schema(example = json!(["patient:read", "appointment:write"]))]
    pub direct_permissions: Vec<String>,
    
    /// Computed effective permissions (flattened from roles + groups + direct)
    #[schema(example = json!(["patient:read", "patient:write", "appointment:read"]))]
    pub effective_permissions: Vec<String>,
    
    /// Last updated timestamp
    pub updated_at: String,
}

/// Request to assign permissions to a user
#[derive(Debug, Deserialize, ToSchema)]
pub struct AssignPermissionsRequest {
    pub user_id: Uuid,
    pub roles: Option<Vec<Uuid>>,
    pub groups: Option<Vec<Uuid>>,
    pub direct_permissions: Option<Vec<String>>,
}

/// Get user's permissions
#[utoipa::path(
    get,
    path = "/api/permissions/users/{user_id}",
    tag = "Permission Management",
    params(
        ("user_id" = Uuid, Path, description = "User ID")
    ),
    responses(
        (status = 200, description = "User permissions", body = UserPermissions),
        (status = 404, description = "User not found"),
        (status = 401, description = "Unauthorized"),
    ),
    security(("bearer_auth" = []))
)]
pub async fn get_user_permissions(
    State(server): State<RustCareServer>,
    Path(user_id): Path<Uuid>,
) -> Result<Json<UserPermissions>, StatusCode> {
    // TODO: Query database and compute effective permissions
    
    let permissions = UserPermissions {
        user_id,
        roles: vec![],
        groups: vec![],
        direct_permissions: vec!["patient:read".to_string()],
        effective_permissions: vec![
            "patient:read".to_string(),
            "appointment:read".to_string(),
        ],
        updated_at: chrono::Utc::now().to_rfc3339(),
    };
    
    Ok(Json(permissions))
}

/// Assign permissions to a user
#[utoipa::path(
    put,
    path = "/api/permissions/users/{user_id}",
    tag = "Permission Management",
    params(
        ("user_id" = Uuid, Path, description = "User ID")
    ),
    request_body = AssignPermissionsRequest,
    responses(
        (status = 200, description = "Permissions assigned", body = UserPermissions),
        (status = 400, description = "Invalid request"),
        (status = 401, description = "Unauthorized"),
    ),
    security(("bearer_auth" = []))
)]
pub async fn assign_user_permissions(
    State(server): State<RustCareServer>,
    Path(user_id): Path<Uuid>,
    Json(request): Json<AssignPermissionsRequest>,
) -> Result<Json<UserPermissions>, StatusCode> {
    // TODO: Validate and update Zanzibar tuples
    // TODO: Compute effective permissions
    // TODO: Log audit event
    
    let permissions = UserPermissions {
        user_id,
        roles: request.roles.unwrap_or_default(),
        groups: request.groups.unwrap_or_default(),
        direct_permissions: request.direct_permissions.unwrap_or_default(),
        effective_permissions: vec![],
        updated_at: chrono::Utc::now().to_rfc3339(),
    };
    
    Ok(Json(permissions))
}

/// Check if user has specific permission
#[derive(Debug, Deserialize, ToSchema)]
pub struct CheckPermissionRequest {
    pub user_id: Uuid,
    pub resource: String,
    pub action: String,
    pub resource_id: Option<String>,
}

#[derive(Debug, Serialize, ToSchema)]
pub struct CheckPermissionResponse {
    pub allowed: bool,
    pub reason: Option<String>,
}

/// Check permission (for Remix loaders)
#[utoipa::path(
    post,
    path = "/api/auth/check",
    tag = "Permission Management",
    request_body = CheckPermissionRequest,
    responses(
        (status = 200, description = "Permission check result", body = CheckPermissionResponse),
        (status = 400, description = "Invalid request"),
        (status = 401, description = "Unauthorized"),
    ),
    security(("bearer_auth" = []))
)]
pub async fn check_permission(
    State(server): State<RustCareServer>,
    Json(request): Json<CheckPermissionRequest>,
) -> Result<Json<CheckPermissionResponse>, StatusCode> {
    // TODO: Query Zanzibar for permission check
    // TODO: Log audit event (both allow and deny)
    
    // Placeholder logic
    let allowed = true;
    
    Ok(Json(CheckPermissionResponse {
        allowed,
        reason: if !allowed {
            Some("User does not have required permission".to_string())
        } else {
            None
        },
    }))
}
