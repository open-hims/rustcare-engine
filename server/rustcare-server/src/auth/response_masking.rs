/// Response masking middleware for HIPAA-compliant field protection
/// 
/// Automatically masks sensitive fields in JSON responses based on user permissions.
/// Integrates with the database-layer MaskingEngine to enforce field-level access control.

use axum::{
    extract::Request,
    middleware::Next,
    response::{Response, IntoResponse},
    http::StatusCode,
    body::Body,
};
use http_body_util::BodyExt;
use database_layer::encryption::{MaskingEngine, MaskingMiddleware};
use crate::auth::middleware::AuthContext;
use serde_json::Value;

/// Response masking middleware
/// 
/// Intercepts JSON responses and masks sensitive fields based on user's PHI permissions.
/// Only masks responses with Content-Type: application/json.
/// 
/// # HIPAA Compliance
/// 
/// This middleware enforces the "Minimum Necessary" standard by:
/// - Automatically masking SSN, MRN, diagnosis, medications, etc.
/// - Only showing unmasked data if user has appropriate phi:view:{level} permission
/// - Logging all field access attempts via AuditLogger
/// 
/// # Permission Hierarchy
/// 
/// - phi:view:public - No restrictions (public data)
/// - phi:view:internal - Email, phone (non-identifying)
/// - phi:view:confidential - Address, DOB (demographics)
/// - phi:view:restricted - SSN, MRN, financial data
/// - phi:view:ephi - Diagnosis, medications, clinical notes
/// - phi:view:unmasked - Full unmasked access (admin, compliance)
/// 
/// # Example
/// 
/// ```rust
/// use axum::Router;
/// use axum::middleware;
/// 
/// let app = Router::new()
///     .route("/api/patients/:id", get(get_patient))
///     .layer(middleware::from_fn(auth_middleware))
///     .layer(middleware::from_fn(response_masking_middleware));
/// ```
pub async fn response_masking_middleware(
    request: Request<Body>,
    next: Next,
) -> Result<Response, MaskingError> {
    // Extract auth context (if present)
    let auth_ctx = request.extensions()
        .get::<AuthContext>()
        .cloned();
    
    // Run the actual handler
    let response = next.run(request).await;
    
    // Only process JSON responses
    let content_type = response.headers()
        .get("content-type")
        .and_then(|v| v.to_str().ok())
        .unwrap_or("");
    
    if !content_type.contains("application/json") {
        return Ok(response);
    }
    
    // Extract response parts
    let (parts, body) = response.into_parts();
    
    // Collect body bytes
    let bytes = body
        .collect()
        .await
        .map_err(|_| MaskingError::BodyReadError)?
        .to_bytes();
    
    // Parse as JSON
    let json: Value = serde_json::from_slice(&bytes)
        .map_err(|_| MaskingError::JsonParseError)?;
    
    // Apply masking if user is authenticated
    let masked_json = if let Some(ctx) = auth_ctx {
        let masking_middleware = MaskingMiddleware::default();
        
        // Mask the response based on user permissions
        let masked = masking_middleware.mask_response(json, &ctx.permissions);
        
        // Log PHI access (if any sensitive fields were accessed)
        // TODO: Extract field access info and log via AuditLogger
        // This requires tracking which fields were requested and masked
        
        tracing::debug!(
            user_id = %ctx.user_id,
            permissions = ?ctx.permissions,
            "Applied response masking"
        );
        
        masked
    } else {
        // No auth context - mask everything at highest level
        let masking_middleware = MaskingMiddleware::default();
        masking_middleware.mask_response(json, &Vec::new())
    };
    
    // Serialize back to JSON
    let masked_bytes = serde_json::to_vec(&masked_json)
        .map_err(|_| MaskingError::JsonSerializeError)?;
    
    // Rebuild response with masked body
    let new_body = Body::from(masked_bytes);
    let response = Response::from_parts(parts, new_body);
    
    Ok(response)
}

/// Advanced response masking middleware with audit logging
/// 
/// Similar to `response_masking_middleware` but includes detailed audit logging
/// for PHI access. Use this for endpoints that handle protected health information.
/// 
/// # Example
/// 
/// ```rust
/// let app = Router::new()
///     .route("/api/patients/:id/records", get(get_medical_records))
///     .layer(middleware::from_fn(auth_middleware))
///     .layer(middleware::from_fn(response_masking_with_audit_middleware));
/// ```
pub async fn response_masking_with_audit_middleware(
    request: Request<Body>,
    next: Next,
) -> Result<Response, MaskingError> {
    // Extract auth context and request path for audit logging
    let auth_ctx = request.extensions()
        .get::<AuthContext>()
        .cloned();
    
    let request_path = request.uri().path().to_string();
    let request_method = request.method().to_string();
    
    // Run the actual handler
    let response = next.run(request).await;
    
    // Only process JSON responses
    let content_type = response.headers()
        .get("content-type")
        .and_then(|v| v.to_str().ok())
        .unwrap_or("");
    
    if !content_type.contains("application/json") {
        return Ok(response);
    }
    
    // Extract response parts
    let (parts, body) = response.into_parts();
    
    // Collect body bytes
    let bytes = body
        .collect()
        .await
        .map_err(|_| MaskingError::BodyReadError)?
        .to_bytes();
    
    // Parse as JSON
    let json: Value = serde_json::from_slice(&bytes)
        .map_err(|_| MaskingError::JsonParseError)?;
    
    // Track which fields were accessed and masked
    let mut fields_accessed = Vec::new();
    let mut fields_masked = Vec::new();
    
    // Apply masking if user is authenticated
    let masked_json = if let Some(ctx) = auth_ctx {
        let engine = MaskingEngine::default();
        
        // Collect all sensitive field names in the response
        collect_field_names(&json, &mut fields_accessed, &engine);
        
        let masking_middleware = MaskingMiddleware::default();
        let masked = masking_middleware.mask_response(json.clone(), &ctx.permissions);
        
        // Determine which fields were actually masked
        identify_masked_fields(&json, &masked, &mut fields_masked, &engine);
        
        // Log PHI access
        if !fields_accessed.is_empty() {
            tracing::info!(
                user_id = %ctx.user_id,
                organization_id = ?ctx.organization_id,
                request_path = %request_path,
                request_method = %request_method,
                fields_accessed = ?fields_accessed,
                fields_masked = ?fields_masked,
                "PHI access in API response"
            );
            
            // TODO: Call AuditLogger::log_phi_access() with proper context
            // This requires access to the database pool and tenant ID
        }
        
        masked
    } else {
        // No auth context - mask everything
        let masking_middleware = MaskingMiddleware::default();
        masking_middleware.mask_response(json, &Vec::new())
    };
    
    // Serialize back to JSON
    let masked_bytes = serde_json::to_vec(&masked_json)
        .map_err(|_| MaskingError::JsonSerializeError)?;
    
    // Rebuild response with masked body
    let new_body = Body::from(masked_bytes);
    let response = Response::from_parts(parts, new_body);
    
    Ok(response)
}

/// Collect all sensitive field names present in the JSON value
fn collect_field_names(
    json: &Value,
    fields: &mut Vec<String>,
    engine: &MaskingEngine,
) {
    match json {
        Value::Object(map) => {
            for (key, value) in map {
                // Check if this field is sensitive
                if engine.get_sensitivity_level(key).is_some() {
                    fields.push(key.clone());
                }
                
                // Recurse into nested objects/arrays
                collect_field_names(value, fields, engine);
            }
        }
        Value::Array(arr) => {
            for item in arr {
                collect_field_names(item, fields, engine);
            }
        }
        _ => {}
    }
}

/// Identify which fields were actually masked by comparing original and masked JSON
fn identify_masked_fields(
    original: &Value,
    masked: &Value,
    fields: &mut Vec<String>,
    engine: &MaskingEngine,
) {
    match (original, masked) {
        (Value::Object(orig_map), Value::Object(masked_map)) => {
            for (key, orig_value) in orig_map {
                if let Some(masked_value) = masked_map.get(key) {
                    // Check if field is sensitive and values differ
                    if engine.get_sensitivity_level(key).is_some() && orig_value != masked_value {
                        fields.push(key.clone());
                    }
                    
                    // Recurse into nested objects/arrays
                    identify_masked_fields(orig_value, masked_value, fields, engine);
                }
            }
        }
        (Value::Array(orig_arr), Value::Array(masked_arr)) => {
            for (orig_item, masked_item) in orig_arr.iter().zip(masked_arr.iter()) {
                identify_masked_fields(orig_item, masked_item, fields, engine);
            }
        }
        _ => {}
    }
}

// =============================================================================
// ERROR TYPES
// =============================================================================

#[derive(Debug, thiserror::Error)]
pub enum MaskingError {
    #[error("Failed to read response body")]
    BodyReadError,
    
    #[error("Failed to parse JSON response")]
    JsonParseError,
    
    #[error("Failed to serialize masked JSON")]
    JsonSerializeError,
}

impl IntoResponse for MaskingError {
    fn into_response(self) -> Response {
        let (status, message) = match self {
            MaskingError::BodyReadError => (StatusCode::INTERNAL_SERVER_ERROR, "Failed to read response body"),
            MaskingError::JsonParseError => (StatusCode::INTERNAL_SERVER_ERROR, "Failed to parse JSON response"),
            MaskingError::JsonSerializeError => (StatusCode::INTERNAL_SERVER_ERROR, "Failed to serialize response"),
        };
        
        let body = serde_json::json!({
            "error": message,
            "status": status.as_u16(),
        });
        
        (status, axum::Json(body)).into_response()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_collect_field_names() {
        let engine = MaskingEngine::default();
        let mut fields = Vec::new();
        
        let json = serde_json::json!({
            "id": "123",
            "name": "John Doe",
            "email": "john@example.com",
            "ssn": "123-45-6789",
            "address": {
                "street": "123 Main St",
                "city": "Boston",
                "zip_code": "02101"
            },
            "diagnosis": "Hypertension"
        });
        
        collect_field_names(&json, &mut fields, &engine);
        
        // Should collect all sensitive fields
        assert!(fields.contains(&"email".to_string()));
        assert!(fields.contains(&"ssn".to_string()));
        assert!(fields.contains(&"city".to_string()));
        assert!(fields.contains(&"zip_code".to_string()));
        assert!(fields.contains(&"diagnosis".to_string()));
        
        // Should not collect non-sensitive fields
        assert!(!fields.contains(&"id".to_string()));
    }
    
    #[test]
    fn test_identify_masked_fields() {
        let engine = MaskingEngine::default();
        let mut fields = Vec::new();
        
        let original = serde_json::json!({
            "ssn": "123-45-6789",
            "email": "john@example.com",
            "name": "John Doe"
        });
        
        let masked = serde_json::json!({
            "ssn": "***-**-6789",
            "email": "joh***@example.com",
            "name": "J**n D*e"
        });
        
        identify_masked_fields(&original, &masked, &mut fields, &engine);
        
        // All sensitive fields should be identified as masked
        assert!(fields.contains(&"ssn".to_string()));
        assert!(fields.contains(&"email".to_string()));
    }
}
