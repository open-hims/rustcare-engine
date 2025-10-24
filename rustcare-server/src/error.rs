use axum::{
    http::StatusCode,
    response::{IntoResponse, Json, Response},
};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use thiserror::Error;
use tracing::error;
use uuid::Uuid;

/// Standard API error response structure
#[derive(Debug, Serialize, Deserialize)]
pub struct ApiErrorResponse {
    /// Unique error ID for tracking
    pub error_id: String,
    /// Error type/code
    pub error_type: String,
    /// Human-readable error message
    pub message: String,
    /// Detailed error description for debugging
    #[serde(skip_serializing_if = "Option::is_none")]
    pub details: Option<String>,
    /// Field-specific validation errors
    #[serde(skip_serializing_if = "Option::is_none")]
    pub field_errors: Option<HashMap<String, Vec<String>>>,
    /// Timestamp when error occurred
    pub timestamp: chrono::DateTime<chrono::Utc>,
    /// Request ID for correlation
    #[serde(skip_serializing_if = "Option::is_none")]
    pub request_id: Option<String>,
    /// Suggested actions for resolving the error
    #[serde(skip_serializing_if = "Option::is_none")]
    pub suggestions: Option<Vec<String>>,
}

/// Standard API success response wrapper
#[derive(Debug, Serialize, Deserialize)]
pub struct ApiResponse<T> {
    pub success: bool,
    pub data: T,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub metadata: Option<ResponseMetadata>,
}

/// Response metadata for pagination, etc.
#[derive(Debug, Serialize, Deserialize)]
pub struct ResponseMetadata {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub pagination: Option<PaginationInfo>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub total_count: Option<i64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub request_id: Option<String>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct PaginationInfo {
    pub page: i32,
    pub page_size: i32,
    pub total_pages: i32,
    pub has_next: bool,
    pub has_previous: bool,
}

/// Main API error enum
#[derive(Error, Debug)]
pub enum ApiError {
    #[error("Validation error: {message}")]
    Validation {
        message: String,
        field_errors: Option<HashMap<String, Vec<String>>>,
    },

    #[error("Authentication error: {message}")]
    Authentication { message: String },

    #[error("Authorization error: {message}")]
    Authorization { message: String },

    #[error("Resource not found: {resource_type}")]
    NotFound { resource_type: String },

    #[error("Resource conflict: {message}")]
    Conflict { message: String },

    #[error("Rate limit exceeded: {message}")]
    RateLimit { message: String },

    #[error("Database error: {0}")]
    Database(#[from] database_layer::DatabaseError),

    #[error("Internal server error: {message}")]
    Internal { message: String },

    #[error("Service unavailable: {message}")]
    ServiceUnavailable { message: String },

    #[error("Bad request: {message}")]
    BadRequest { message: String },

    #[error("Unprocessable entity: {message}")]
    UnprocessableEntity { message: String },

    #[error("Network error: {message}")]
    Network { message: String },

    #[error("Configuration error: {message}")]
    Configuration { message: String },
}

impl ApiError {
    /// Create a validation error with field-specific errors
    pub fn validation_with_fields(
        message: impl Into<String>,
        field_errors: HashMap<String, Vec<String>>,
    ) -> Self {
        Self::Validation {
            message: message.into(),
            field_errors: Some(field_errors),
        }
    }

    /// Create a simple validation error
    pub fn validation(message: impl Into<String>) -> Self {
        Self::Validation {
            message: message.into(),
            field_errors: None,
        }
    }

    /// Create an authentication error
    pub fn authentication(message: impl Into<String>) -> Self {
        Self::Authentication {
            message: message.into(),
        }
    }

    /// Create an authorization error
    pub fn authorization(message: impl Into<String>) -> Self {
        Self::Authorization {
            message: message.into(),
        }
    }

    /// Create a not found error
    pub fn not_found(resource_type: impl Into<String>) -> Self {
        Self::NotFound {
            resource_type: resource_type.into(),
        }
    }

    /// Create a conflict error
    pub fn conflict(message: impl Into<String>) -> Self {
        Self::Conflict {
            message: message.into(),
        }
    }

    /// Create an internal error
    pub fn internal(message: impl Into<String>) -> Self {
        Self::Internal {
            message: message.into(),
        }
    }

    /// Create a bad request error
    pub fn bad_request(message: impl Into<String>) -> Self {
        Self::BadRequest {
            message: message.into(),
        }
    }

    /// Get the HTTP status code for this error
    pub fn status_code(&self) -> StatusCode {
        match self {
            ApiError::Validation { .. } => StatusCode::BAD_REQUEST,
            ApiError::Authentication { .. } => StatusCode::UNAUTHORIZED,
            ApiError::Authorization { .. } => StatusCode::FORBIDDEN,
            ApiError::NotFound { .. } => StatusCode::NOT_FOUND,
            ApiError::Conflict { .. } => StatusCode::CONFLICT,
            ApiError::RateLimit { .. } => StatusCode::TOO_MANY_REQUESTS,
            ApiError::Database(db_err) => match db_err {
                database_layer::DatabaseError::RlsPolicyViolation => StatusCode::FORBIDDEN,
                database_layer::DatabaseError::QueryFailed(_) => StatusCode::BAD_REQUEST,
                database_layer::DatabaseError::ConnectionFailed(_) => StatusCode::SERVICE_UNAVAILABLE,
                _ => StatusCode::INTERNAL_SERVER_ERROR,
            },
            ApiError::Internal { .. } => StatusCode::INTERNAL_SERVER_ERROR,
            ApiError::ServiceUnavailable { .. } => StatusCode::SERVICE_UNAVAILABLE,
            ApiError::BadRequest { .. } => StatusCode::BAD_REQUEST,
            ApiError::UnprocessableEntity { .. } => StatusCode::UNPROCESSABLE_ENTITY,
            ApiError::Network { .. } => StatusCode::BAD_GATEWAY,
            ApiError::Configuration { .. } => StatusCode::INTERNAL_SERVER_ERROR,
        }
    }

    /// Get the error type string
    pub fn error_type(&self) -> &'static str {
        match self {
            ApiError::Validation { .. } => "validation_error",
            ApiError::Authentication { .. } => "authentication_error",
            ApiError::Authorization { .. } => "authorization_error",
            ApiError::NotFound { .. } => "not_found",
            ApiError::Conflict { .. } => "conflict",
            ApiError::RateLimit { .. } => "rate_limit_exceeded",
            ApiError::Database(_) => "database_error",
            ApiError::Internal { .. } => "internal_error",
            ApiError::ServiceUnavailable { .. } => "service_unavailable",
            ApiError::BadRequest { .. } => "bad_request",
            ApiError::UnprocessableEntity { .. } => "unprocessable_entity",
            ApiError::Network { .. } => "network_error",
            ApiError::Configuration { .. } => "configuration_error",
        }
    }

    /// Get suggested actions for resolving the error
    pub fn suggestions(&self) -> Option<Vec<String>> {
        match self {
            ApiError::Validation { .. } => Some(vec![
                "Check the request payload for invalid fields".to_string(),
                "Ensure all required fields are provided".to_string(),
                "Verify data types and formats match the API specification".to_string(),
            ]),
            ApiError::Authentication { .. } => Some(vec![
                "Verify your authentication credentials".to_string(),
                "Check if your token has expired".to_string(),
                "Ensure you're using the correct authentication method".to_string(),
            ]),
            ApiError::Authorization { .. } => Some(vec![
                "Verify you have the required permissions".to_string(),
                "Contact your administrator for access".to_string(),
                "Check if your role allows this operation".to_string(),
            ]),
            ApiError::NotFound { .. } => Some(vec![
                "Verify the resource ID is correct".to_string(),
                "Check if the resource exists".to_string(),
                "Ensure you have access to view this resource".to_string(),
            ]),
            ApiError::Database(db_err) => match db_err {
                database_layer::DatabaseError::RlsPolicyViolation => Some(vec![
                    "Check your organization access permissions".to_string(),
                    "Verify you're accessing resources within your scope".to_string(),
                ]),
                database_layer::DatabaseError::ConnectionFailed(_) => Some(vec![
                    "Try again in a few moments".to_string(),
                    "Contact support if the issue persists".to_string(),
                ]),
                _ => Some(vec![
                    "Verify your request data is valid".to_string(),
                    "Contact support if the issue persists".to_string(),
                ]),
            },
            ApiError::RateLimit { .. } => Some(vec![
                "Wait before making additional requests".to_string(),
                "Consider implementing exponential backoff".to_string(),
                "Contact support to increase your rate limits".to_string(),
            ]),
            _ => None,
        }
    }

    /// Pretty format database errors for better user experience
    pub fn format_database_error(db_error: &database_layer::DatabaseError) -> String {
        match db_error {
            database_layer::DatabaseError::ConnectionFailed(msg) => {
                format!("Unable to connect to the database. {}", msg)
            }
            database_layer::DatabaseError::QueryFailed(msg) => {
                if msg.contains("duplicate key") {
                    "A record with these details already exists.".to_string()
                } else if msg.contains("foreign key") {
                    "Referenced record does not exist or has been deleted.".to_string()
                } else if msg.contains("check constraint") {
                    "The provided data does not meet validation requirements.".to_string()
                } else if msg.contains("not null") {
                    "Required field is missing or empty.".to_string()
                } else {
                    format!("Database operation failed: {}", msg)
                }
            }
            database_layer::DatabaseError::RlsPolicyViolation => {
                "Access denied: You don't have permission to perform this operation.".to_string()
            }
            database_layer::DatabaseError::EncryptionError(msg) => {
                format!("Data encryption error: {}", msg)
            }
            database_layer::DatabaseError::ConfigurationError(msg) => {
                format!("Database configuration error: {}", msg)
            }
            database_layer::DatabaseError::SqlxError(sqlx_err) => {
                match sqlx_err {
                    sqlx::Error::RowNotFound => "Requested record not found.".to_string(),
                    sqlx::Error::ColumnNotFound(col) => {
                        format!("Database schema error: Column '{}' not found.", col)
                    }
                    sqlx::Error::TypeNotFound { type_name } => {
                        format!("Database type error: '{}' type not supported.", type_name)
                    }
                    _ => "Database operation failed. Please try again.".to_string(),
                }
            }
            _ => "An unexpected database error occurred.".to_string(),
        }
    }
}

impl IntoResponse for ApiError {
    fn into_response(self) -> Response {
        let error_id = Uuid::new_v4().to_string();
        let status_code = self.status_code();
        
        // Log the error with correlation ID
        error!(
            error_id = %error_id,
            error_type = %self.error_type(),
            status_code = %status_code.as_u16(),
            error = %self,
            "API error occurred"
        );

        let field_errors = match &self {
            ApiError::Validation { field_errors, .. } => field_errors.clone(),
            _ => None,
        };

        let message = match &self {
            ApiError::Database(db_err) => ApiError::format_database_error(db_err),
            _ => self.to_string(),
        };

        let error_response = ApiErrorResponse {
            error_id,
            error_type: self.error_type().to_string(),
            message,
            details: None, // Don't expose internal details in production
            field_errors,
            timestamp: chrono::Utc::now(),
            request_id: None, // TODO: Extract from request context
            suggestions: self.suggestions(),
        };

        (status_code, Json(error_response)).into_response()
    }
}

/// Helper trait for converting results to API responses
pub trait IntoApiResponse<T> {
    fn into_api_response(self) -> Result<ApiResponse<T>, ApiError>;
    fn into_api_response_with_meta(
        self,
        metadata: ResponseMetadata,
    ) -> Result<ApiResponse<T>, ApiError>;
}

impl<T, E> IntoApiResponse<T> for Result<T, E>
where
    E: Into<ApiError>,
{
    fn into_api_response(self) -> Result<ApiResponse<T>, ApiError> {
        match self {
            Ok(data) => Ok(ApiResponse {
                success: true,
                data,
                metadata: None,
            }),
            Err(e) => Err(e.into()),
        }
    }

    fn into_api_response_with_meta(
        self,
        metadata: ResponseMetadata,
    ) -> Result<ApiResponse<T>, ApiError> {
        match self {
            Ok(data) => Ok(ApiResponse {
                success: true,
                data,
                metadata: Some(metadata),
            }),
            Err(e) => Err(e.into()),
        }
    }
}

/// Helper function to create successful API responses
pub fn api_success<T>(data: T) -> ApiResponse<T> {
    ApiResponse {
        success: true,
        data,
        metadata: None,
    }
}

/// Helper function to create successful API responses with metadata
pub fn api_success_with_meta<T>(data: T, metadata: ResponseMetadata) -> ApiResponse<T> {
    ApiResponse {
        success: true,
        data,
        metadata: Some(metadata),
    }
}

/// Helper function to create paginated responses
pub fn api_paginated<T>(
    data: T,
    page: i32,
    page_size: i32,
    total_count: i64,
) -> ApiResponse<T> {
    let total_pages = ((total_count as f64) / (page_size as f64)).ceil() as i32;
    
    let pagination = PaginationInfo {
        page,
        page_size,
        total_pages,
        has_next: page < total_pages,
        has_previous: page > 1,
    };

    let metadata = ResponseMetadata {
        pagination: Some(pagination),
        total_count: Some(total_count),
        request_id: None,
    };

    api_success_with_meta(data, metadata)
}

/// Convert SQLx errors to API errors
impl From<sqlx::Error> for ApiError {
    fn from(sqlx_error: sqlx::Error) -> Self {
        let db_error = database_layer::DatabaseError::SqlxError(sqlx_error);
        ApiError::Database(db_error)
    }
}

/// Convert anyhow errors to API errors
impl From<anyhow::Error> for ApiError {
    fn from(error: anyhow::Error) -> Self {
        ApiError::Internal {
            message: error.to_string(),
        }
    }
}

/// Convert serde JSON errors to API errors
impl From<serde_json::Error> for ApiError {
    fn from(error: serde_json::Error) -> Self {
        ApiError::BadRequest {
            message: format!("Invalid JSON: {}", error),
        }
    }
}

/// Type alias for API results
pub type ApiResult<T> = Result<T, ApiError>;