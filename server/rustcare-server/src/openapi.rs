use utoipa::OpenApi;
use utoipa_swagger_ui::SwaggerUi;
use axum::Router;
use crate::server::RustCareServer;

/// Main OpenAPI documentation structure
#[derive(OpenApi)]
#[openapi(
    paths(
        // Health endpoints
        crate::handlers::health::health_check,
        crate::handlers::health::version_info,
        crate::handlers::health::system_status,
        
        // Authentication endpoints
        crate::handlers::auth::login,
    ),
    components(
        schemas(
            // Health schemas
            crate::handlers::health::HealthResponse,
            crate::handlers::health::VersionResponse,
            crate::handlers::health::StatusResponse,
            crate::handlers::health::ServiceStatus,
            
            // Authentication schemas
            crate::handlers::auth::AuthRequest,
            crate::handlers::auth::AuthResponse,
            crate::handlers::auth::OAuthRequest,
            crate::handlers::auth::TokenValidationRequest,
            crate::handlers::auth::TokenValidationResponse,
            
            // Workflow schemas
            crate::handlers::workflow::WorkflowDefinition,
            crate::handlers::workflow::WorkflowExecutionRequest,
            crate::handlers::workflow::WorkflowListResponse,
            crate::handlers::workflow::WorkflowSummary,
            crate::handlers::workflow::ExecutionStatusResponse,
        )
    ),
    tags(
        (name = "health", description = "System health and status endpoints"),
        (name = "authentication", description = "User authentication and authorization"),
        (name = "workflow", description = "Healthcare workflow management"),
        (name = "audit", description = "HIPAA compliance and audit logging"),
    ),
    info(
        title = "RustCare Engine API",
        version = "1.0.0",
        description = "HIPAA-compliant healthcare platform API providing secure patient data management, workflow automation, and audit logging.",
        contact(
            name = "RustCare Team",
            email = "api@rustcare.dev",
            url = "https://rustcare.dev"
        ),
        license(
            name = "MIT OR Apache-2.0",
            url = "https://github.com/Open-Hims-HQ/rustcare-engine/blob/main/LICENSE"
        ),
    ),
    servers(
        (url = "https://api.openhims.health", description = "Local HTTPS development (custom domain)"),
        (url = "http://localhost:8081", description = "Local direct HTTP server"),
        (url = "https://api.rustcare.dev", description = "Production server"),
        (url = "https://staging-api.rustcare.dev", description = "Staging server"),
    ),
    external_docs(
        description = "Find more info about RustCare Engine",
        url = "https://docs.rustcare.dev"
    ),
)]
pub struct ApiDoc;

/// Healthcare-specific API documentation
#[derive(OpenApi)]
#[openapi(
    info(
        title = "RustCare Healthcare API",
        version = "1.0.0",
        description = "Healthcare-specific endpoints for patient management, medical records, and clinical workflows.",
    ),
    tags(
        (name = "patients", description = "Patient management and records"),
        (name = "providers", description = "Healthcare provider management"),
        (name = "appointments", description = "Appointment scheduling and management"),
        (name = "medical-records", description = "Electronic health records (EHR)"),
        (name = "vitals", description = "Patient vital signs monitoring"),
        (name = "prescriptions", description = "Medication and prescription management"),
        (name = "billing", description = "Healthcare billing and insurance"),
    ),
)]
pub struct HealthcareApiDoc;

/// Admin and operations API documentation
#[derive(OpenApi)]
#[openapi(
    info(
        title = "RustCare Admin API",
        version = "1.0.0",
        description = "Administrative endpoints for system management, user administration, and compliance reporting.",
    ),
    tags(
        (name = "admin", description = "System administration"),
        (name = "users", description = "User management and roles"),
        (name = "compliance", description = "HIPAA compliance and reporting"),
        (name = "audit", description = "Audit logs and security monitoring"),
        (name = "analytics", description = "Healthcare analytics and reporting"),
        (name = "plugins", description = "Plugin management and runtime"),
    ),
)]
pub struct AdminApiDoc;

/// Create OpenAPI documentation routes
pub fn create_docs_routes() -> Router<RustCareServer> {
    Router::new()
        .merge(SwaggerUi::new("/docs").url("/api-docs/openapi.json", ApiDoc::openapi()))
        .merge(SwaggerUi::new("/docs/healthcare").url("/api-docs/healthcare.json", HealthcareApiDoc::openapi()))
        .merge(SwaggerUi::new("/docs/admin").url("/api-docs/admin.json", AdminApiDoc::openapi()))
}

/// Postman collection configuration
pub fn generate_postman_collection() -> serde_json::Value {
    serde_json::json!({
        "info": {
            "name": "RustCare Engine API",
            "description": "Complete API collection for RustCare Engine - HIPAA-compliant healthcare platform",
            "schema": "https://schema.getpostman.com/json/collection/v2.1.0/collection.json",
            "version": "1.0.0"
        },
        "variable": [
            {
                "key": "baseUrl",
                "value": "http://localhost:8081",
                "type": "string"
            },
            {
                "key": "authToken",
                "value": "",
                "type": "string"
            }
        ],
        "auth": {
            "type": "bearer",
            "bearer": [
                {
                    "key": "token",
                    "value": "{{authToken}}",
                    "type": "string"
                }
            ]
        },
        "item": [
            {
                "name": "Health & Status",
                "item": [
                    {
                        "name": "Health Check",
                        "request": {
                            "method": "GET",
                            "header": [],
                            "url": {
                                "raw": "{{baseUrl}}/health",
                                "host": ["{{baseUrl}}"],
                                "path": ["health"]
                            },
                            "description": "Check system health status"
                        }
                    },
                    {
                        "name": "Version Info",
                        "request": {
                            "method": "GET",
                            "header": [],
                            "url": {
                                "raw": "{{baseUrl}}/version",
                                "host": ["{{baseUrl}}"],
                                "path": ["version"]
                            },
                            "description": "Get API version information"
                        }
                    },
                    {
                        "name": "System Status",
                        "request": {
                            "method": "GET",
                            "header": [],
                            "url": {
                                "raw": "{{baseUrl}}/status",
                                "host": ["{{baseUrl}}"],
                                "path": ["status"]
                            },
                            "description": "Get detailed system status"
                        }
                    }
                ]
            },
            {
                "name": "Authentication",
                "item": [
                    {
                        "name": "Login",
                        "event": [
                            {
                                "listen": "test",
                                "script": {
                                    "type": "text/javascript",
                                    "exec": [
                                        "if (pm.response.code === 200) {",
                                        "    const response = pm.response.json();",
                                        "    pm.collectionVariables.set('authToken', response.access_token);",
                                        "}"
                                    ]
                                }
                            }
                        ],
                        "request": {
                            "method": "POST",
                            "header": [
                                {
                                    "key": "Content-Type",
                                    "value": "application/json"
                                }
                            ],
                            "body": {
                                "mode": "raw",
                                "raw": "{\n  \"username\": \"{{username}}\",\n  \"password\": \"{{password}}\",\n  \"remember_me\": false\n}"
                            },
                            "url": {
                                "raw": "{{baseUrl}}/api/v1/auth/login",
                                "host": ["{{baseUrl}}"],
                                "path": ["api", "v1", "auth", "login"]
                            },
                            "description": "Authenticate user and receive access token"
                        }
                    },
                    {
                        "name": "Logout",
                        "request": {
                            "method": "POST",
                            "header": [
                                {
                                    "key": "Authorization",
                                    "value": "Bearer {{authToken}}"
                                }
                            ],
                            "url": {
                                "raw": "{{baseUrl}}/api/v1/auth/logout",
                                "host": ["{{baseUrl}}"],
                                "path": ["api", "v1", "auth", "logout"]
                            },
                            "description": "Logout and invalidate access token"
                        }
                    },
                    {
                        "name": "Validate Token",
                        "request": {
                            "method": "POST",
                            "header": [
                                {
                                    "key": "Content-Type",
                                    "value": "application/json"
                                }
                            ],
                            "body": {
                                "mode": "raw",
                                "raw": "{\n  \"token\": \"{{authToken}}\"\n}"
                            },
                            "url": {
                                "raw": "{{baseUrl}}/api/v1/auth/token/validate",
                                "host": ["{{baseUrl}}"],
                                "path": ["api", "v1", "auth", "token", "validate"]
                            },
                            "description": "Validate access token"
                        }
                    }
                ]
            },
            {
                "name": "Workflow Management",
                "item": [
                    {
                        "name": "List Workflows",
                        "request": {
                            "method": "GET",
                            "header": [
                                {
                                    "key": "Authorization",
                                    "value": "Bearer {{authToken}}"
                                }
                            ],
                            "url": {
                                "raw": "{{baseUrl}}/api/v1/workflow/workflows",
                                "host": ["{{baseUrl}}"],
                                "path": ["api", "v1", "workflow", "workflows"]
                            },
                            "description": "Get list of available workflows"
                        }
                    },
                    {
                        "name": "Get Workflow Details",
                        "request": {
                            "method": "GET",
                            "header": [
                                {
                                    "key": "Authorization",
                                    "value": "Bearer {{authToken}}"
                                }
                            ],
                            "url": {
                                "raw": "{{baseUrl}}/api/v1/workflow/workflows/{{workflowId}}",
                                "host": ["{{baseUrl}}"],
                                "path": ["api", "v1", "workflow", "workflows", "{{workflowId}}"]
                            },
                            "description": "Get detailed workflow information"
                        }
                    },
                    {
                        "name": "Execute Workflow",
                        "request": {
                            "method": "POST",
                            "header": [
                                {
                                    "key": "Authorization",
                                    "value": "Bearer {{authToken}}"
                                },
                                {
                                    "key": "Content-Type",
                                    "value": "application/json"
                                }
                            ],
                            "body": {
                                "mode": "raw",
                                "raw": "{\n  \"workflow_id\": \"patient-admission\",\n  \"parameters\": {\n    \"patient_id\": \"{{patientId}}\",\n    \"priority\": \"high\",\n    \"department\": \"emergency\"\n  },\n  \"execute_async\": true\n}"
                            },
                            "url": {
                                "raw": "{{baseUrl}}/api/v1/workflow/workflows/execute",
                                "host": ["{{baseUrl}}"],
                                "path": ["api", "v1", "workflow", "workflows", "execute"]
                            },
                            "description": "Execute a healthcare workflow"
                        }
                    },
                    {
                        "name": "Get Execution Status",
                        "request": {
                            "method": "GET",
                            "header": [
                                {
                                    "key": "Authorization",
                                    "value": "Bearer {{authToken}}"
                                }
                            ],
                            "url": {
                                "raw": "{{baseUrl}}/api/v1/workflow/executions/{{executionId}}/status",
                                "host": ["{{baseUrl}}"],
                                "path": ["api", "v1", "workflow", "executions", "{{executionId}}", "status"]
                            },
                            "description": "Check workflow execution status"
                        }
                    },
                    {
                        "name": "Cancel Execution",
                        "request": {
                            "method": "DELETE",
                            "header": [
                                {
                                    "key": "Authorization",
                                    "value": "Bearer {{authToken}}"
                                }
                            ],
                            "url": {
                                "raw": "{{baseUrl}}/api/v1/workflow/executions/{{executionId}}/cancel",
                                "host": ["{{baseUrl}}"],
                                "path": ["api", "v1", "workflow", "executions", "{{executionId}}", "cancel"]
                            },
                            "description": "Cancel workflow execution"
                        }
                    }
                ]
            },
            {
                "name": "Healthcare (Future)",
                "item": [
                    {
                        "name": "Patient Management",
                        "item": [
                            {
                                "name": "List Patients",
                                "request": {
                                    "method": "GET",
                                    "header": [
                                        {
                                            "key": "Authorization",
                                            "value": "Bearer {{authToken}}"
                                        },
                                        {
                                            "key": "X-HIPAA-Consent",
                                            "value": "true"
                                        }
                                    ],
                                    "url": {
                                        "raw": "{{baseUrl}}/api/v1/patients?limit=50&offset=0",
                                        "host": ["{{baseUrl}}"],
                                        "path": ["api", "v1", "patients"]
                                    },
                                    "description": "Get paginated list of patients (requires HIPAA consent)"
                                }
                            },
                            {
                                "name": "Create Patient",
                                "request": {
                                    "method": "POST",
                                    "header": [
                                        {
                                            "key": "Authorization",
                                            "value": "Bearer {{authToken}}"
                                        },
                                        {
                                            "key": "Content-Type",
                                            "value": "application/json"
                                        },
                                        {
                                            "key": "X-HIPAA-Consent",
                                            "value": "true"
                                        }
                                    ],
                                    "body": {
                                        "mode": "raw",
                                        "raw": "{\n  \"first_name\": \"John\",\n  \"last_name\": \"Doe\",\n  \"date_of_birth\": \"1980-01-01\",\n  \"gender\": \"M\",\n  \"phone\": \"555-0123\",\n  \"email\": \"john.doe@example.com\",\n  \"address\": {\n    \"street\": \"123 Main St\",\n    \"city\": \"Anytown\",\n    \"state\": \"CA\",\n    \"zip\": \"12345\"\n  },\n  \"emergency_contact\": {\n    \"name\": \"Jane Doe\",\n    \"relationship\": \"spouse\",\n    \"phone\": \"555-0124\"\n  }\n}"
                                    },
                                    "url": {
                                        "raw": "{{baseUrl}}/api/v1/patients",
                                        "host": ["{{baseUrl}}"],
                                        "path": ["api", "v1", "patients"]
                                    },
                                    "description": "Create new patient record"
                                }
                            }
                        ]
                    },
                    {
                        "name": "Appointments",
                        "item": [
                            {
                                "name": "Schedule Appointment",
                                "request": {
                                    "method": "POST",
                                    "header": [
                                        {
                                            "key": "Authorization",
                                            "value": "Bearer {{authToken}}"
                                        },
                                        {
                                            "key": "Content-Type",
                                            "value": "application/json"
                                        }
                                    ],
                                    "body": {
                                        "mode": "raw",
                                        "raw": "{\n  \"patient_id\": \"{{patientId}}\",\n  \"provider_id\": \"{{providerId}}\",\n  \"appointment_type\": \"consultation\",\n  \"scheduled_time\": \"2024-01-15T10:00:00Z\",\n  \"duration_minutes\": 30,\n  \"reason\": \"Annual checkup\",\n  \"notes\": \"Patient requested morning appointment\"\n}"
                                    },
                                    "url": {
                                        "raw": "{{baseUrl}}/api/v1/appointments",
                                        "host": ["{{baseUrl}}"],
                                        "path": ["api", "v1", "appointments"]
                                    },
                                    "description": "Schedule a new appointment"
                                }
                            }
                        ]
                    }
                ]
            }
        ]
    })
}