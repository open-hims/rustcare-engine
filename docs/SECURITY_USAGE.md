# Security Middleware Usage Guide

## Overview

The unified security middleware combines:
- **AuthContext** - JWT authentication and user context
- **RequestContext** - Request ID, origin, headers, same-site validation
- **Rate Limiting** - Per-user or per-IP rate limiting
- **CSRF Protection** - Token validation for state-changing operations
- **Zanzibar Authorization** - Fine-grained permission checks

## Basic Usage

### Option 1: Use SecurityContext in Handlers (Recommended)

```rust
use axum::{extract::{State, Method}, http::HeaderMap};
use crate::middleware::{AuthContext, RequestContext, SecurityContext};
use crate::server::RustCareServer;

pub async fn handler(
    auth: AuthContext,
    req_ctx: RequestContext,
    method: Method,
    headers: HeaderMap,
    State(server): State<RustCareServer>,
) -> Result<Json<ApiResponse<T>>, ApiError> {
    // Create security context with checks
    let security = SecurityContext::from_contexts_with_checks(
        auth,
        req_ctx,
        &method,
        &headers,
        &server.security_state, // Add this to RustCareServer
    ).await?;
    
    // Use security context for permission checks
    security.require_permission("patient", Some(patient_id), "view").await?;
    
    // Check rate limit remaining
    let remaining = security.rate_limit_remaining().await;
    tracing::info!(remaining = remaining, "Rate limit remaining");
    
    // Your handler logic here
    Ok(Json(api_success(data)))
}
```

### Option 2: Use Individual Contexts

```rust
use crate::middleware::{AuthContext, RequestContext};

pub async fn handler(
    auth: AuthContext,
    req_ctx: RequestContext,
) -> Result<Json<ApiResponse<T>>, ApiError> {
    // Check same-site validation
    if !req_ctx.is_same_site() {
        tracing::warn!("Same-site validation failed");
    }
    
    // Check permission using AuthContext
    auth.require_permission("patient", Some(patient_id), "view").await?;
    
    // Log with request ID
    tracing::info!(
        request_id = %req_ctx.request_id,
        user_id = %auth.user_id,
        "Processing request"
    );
    
    Ok(Json(api_success(data)))
}
```

## Rate Limiting

### Configuration

```rust
use crate::middleware::{RateLimitConfig, SecurityConfig};

let rate_limit_config = RateLimitConfig {
    max_requests: 100,        // 100 requests
    window_seconds: 60,        // per 60 seconds
    by_user: true,             // per user (false = per IP)
};

let security_config = SecurityConfig {
    rate_limit: Some(rate_limit_config),
    csrf: Some(CsrfValidator::new()),
    strict_same_site: false,
};
```

### Check Rate Limit in Handler

```rust
let security = SecurityContext::from_contexts_with_checks(...).await?;

// Check remaining requests
let remaining = security.rate_limit_remaining().await;
if remaining < 10 {
    tracing::warn!("Rate limit getting low");
}

// Rate limit check happens automatically in from_contexts_with_checks
// If exceeded, it returns ApiError::RateLimit
```

## CSRF Protection

### Configuration

```rust
use crate::middleware::CsrfValidator;

let csrf_validator = CsrfValidator {
    header_name: "X-CSRF-Token".to_string(),
    require_for_mutations: true, // Require for POST/PUT/PATCH/DELETE
};
```

### Client Usage

**For Web Applications:**
```javascript
// Include CSRF token in headers for state-changing requests
fetch('/api/v1/patients', {
    method: 'POST',
    headers: {
        'Authorization': 'Bearer ' + token,
        'X-CSRF-Token': csrfToken, // From cookie or meta tag
        'Content-Type': 'application/json',
    },
    body: JSON.stringify(data),
});
```

**For API Clients:**
```bash
# Include Origin header (CSRF check will pass if Origin matches ALLOWED_ORIGINS)
curl -X POST https://api.example.com/api/v1/patients \
  -H "Authorization: Bearer $TOKEN" \
  -H "Origin: https://example.com" \
  -H "Content-Type: application/json" \
  -d '{"name": "John Doe"}'
```

## Same-Site Validation

### Configuration

Set allowed origins via environment variable:
```bash
ALLOWED_ORIGINS=localhost,example.com,api.example.com
```

### Usage

```rust
let req_ctx = RequestContext::from_request_parts(...).await?;

if !req_ctx.is_same_site() {
    // Log warning (doesn't reject by default)
    tracing::warn!(
        origin = ?req_ctx.origin,
        "Same-site validation failed"
    );
}

// Or enforce strictly
if !req_ctx.is_same_site() {
    return Err(ApiError::authentication("Cross-site request rejected"));
}
```

## Zanzibar Permission Checks

### Basic Permission Check

```rust
let auth = AuthContext::from_request_parts(...).await?;

// Check if user has permission
let can_view = auth.check_permission("patient", Some(patient_id), "view").await?;
if !can_view {
    return Err(ApiError::authorization("Cannot view patient"));
}

// Or require permission (throws error if not granted)
auth.require_permission("patient", Some(patient_id), "edit").await?;
```

### Using SecurityContext

```rust
let security = SecurityContext::from_contexts_with_checks(...).await?;

// SecurityContext delegates to AuthContext
security.require_permission("patient", Some(patient_id), "view").await?;
security.check_permission("patient", Some(patient_id), "edit").await?;
```

## Setting Up Security State in Router

```rust
use crate::middleware::{SecurityConfig, SecurityState};

// Create security configuration
let security_config = SecurityConfig {
    rate_limit: Some(RateLimitConfig {
        max_requests: 100,
        window_seconds: 60,
        by_user: true,
    }),
    csrf: Some(CsrfValidator::new()),
    strict_same_site: false,
};

let security_state = SecurityState::new(security_config);

// Add to router as extension
let app = Router::new()
    .route("/api/v1/patients", get(list_patients))
    .layer(Extension(security_state)); // Makes it available in handlers
```

## Security Headers

The middleware automatically adds security headers:
- `X-Content-Type-Options: nosniff`
- `X-Frame-Options: DENY`
- `X-XSS-Protection: 1; mode=block`

Request ID is included in response headers automatically.

## Complete Example Handler

```rust
use axum::{extract::{State, Method, Path}, http::HeaderMap, Json};
use crate::middleware::{AuthContext, RequestContext, SecurityContext};
use crate::server::RustCareServer;
use crate::error::{ApiError, ApiResponse, api_success};

pub async fn get_patient(
    auth: AuthContext,
    req_ctx: RequestContext,
    method: Method,
    headers: HeaderMap,
    Path(patient_id): Path<Uuid>,
    State(server): State<RustCareServer>,
) -> Result<Json<ApiResponse<Patient>>, ApiError> {
    // Create security context with all checks
    let security = SecurityContext::from_contexts_with_checks(
        auth,
        req_ctx,
        &method,
        &headers,
        &server.security_state,
    ).await?;
    
    // Check permission
    security.require_permission("patient", Some(patient_id), "view").await?;
    
    // Log with request ID
    tracing::info!(
        request_id = %security.request.request_id,
        user_id = %security.auth.user_id,
        patient_id = %patient_id,
        "Fetching patient"
    );
    
    // Fetch patient (with organization scoping)
    let patient = sqlx::query_as::<_, Patient>(
        "SELECT * FROM patients WHERE id = $1 AND organization_id = $2"
    )
    .bind(patient_id)
    .bind(security.auth.organization_id)
    .fetch_optional(&server.db_pool)
    .await?
    .ok_or_else(|| ApiError::not_found("patient"))?;
    
    Ok(Json(api_success(patient)))
}
```

## Testing

### Mock SecurityContext

```rust
use crate::middleware::{AuthContext, RequestContext, SecurityContext};

let auth = AuthContext::new(user_id, org_id);
let req_ctx = RequestContext::new();
let security = SecurityContext::new(auth, req_ctx);

// Use in tests
```

### Disable Rate Limiting in Tests

```rust
let security_config = SecurityConfig {
    rate_limit: None, // Disable rate limiting
    csrf: None,        // Disable CSRF
    strict_same_site: false,
};
```

## Best Practices

1. **Always use SecurityContext for state-changing operations** (POST/PUT/DELETE)
2. **Check permissions before database queries** to avoid unnecessary work
3. **Log security events** using request_id for correlation
4. **Use rate limiting** to prevent abuse
5. **Validate same-site** for web applications
6. **Include request_id in error responses** for debugging

## Troubleshooting

### Rate Limit Errors
- Check rate limit configuration
- Verify user_id or IP is being extracted correctly
- Consider increasing limits for trusted clients

### CSRF Errors
- Ensure CSRF token is included in headers for mutations
- Or include Origin header matching ALLOWED_ORIGINS
- Check `require_for_mutations` setting

### Same-Site Validation Failures
- Verify ALLOWED_ORIGINS includes your domain
- Check Origin/Referer headers are being sent
- Consider disabling strict mode for API-only clients

### Permission Denied Errors
- Verify Zanzibar engine is initialized
- Check Zanzibar tuples exist for the user/resource
- Ensure permission names match schema

