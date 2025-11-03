# AuthContext with Built-in Security ✅

## Overview

**AuthContext now automatically includes all security features!** No code changes needed in existing handlers.

## What's Automatic ✅

When you use `AuthContext` in any handler, you automatically get:

1. ✅ **JWT Authentication** - Token validation and user extraction
2. ✅ **Request Context** - Request ID, origin, headers, remote address
3. ✅ **Rate Limiting** - Automatic check (100 requests/minute per user)
4. ✅ **CSRF Protection** - Automatic validation for POST/PUT/DELETE
5. ✅ **Same-Site Validation** - Origin/Referer validation
6. ✅ **Zanzibar Authorization** - Permission checking methods

## Usage (No Changes Needed!)

### Existing Handlers Work As-Is ✅

```rust
pub async fn list_pharmacies(
    auth: AuthContext,  // ✅ Everything is automatic!
    // ...
) -> Result<...> {
    // ✅ Rate limiting: Already checked
    // ✅ CSRF: Already checked (if POST/PUT/DELETE)
    // ✅ Same-site: Already validated
    
    // Access user info
    let user_id = auth.user_id;
    let org_id = auth.organization_id;
    
    // Access request info (NEW!)
    let request_id = auth.request_id();  // ✅ Request ID
    let is_same_site = auth.is_same_site();  // ✅ Same-site check
    
    // Check remaining rate limit
    let remaining = auth.rate_limit_remaining().await;  // ✅ Rate limit remaining
    
    // Check permissions
    auth.require_permission("pharmacy", None, "read").await?;  // ✅ Zanzibar
    
    Ok(Json(api_success(data)))
}
```

## New Features Available

### 1. Request Context Access

```rust
// Request ID for tracing
tracing::info!(request_id = %auth.request_id(), "Processing request");

// Check same-site validation
if !auth.is_same_site() {
    tracing::warn!("Cross-site request detected");
}

// Access full request context
let origin = &auth.request.origin;
let user_agent = &auth.request.user_agent;
let remote_addr = &auth.request.remote_addr;
```

### 2. Rate Limiting

```rust
// Check remaining requests in current window
let remaining = auth.rate_limit_remaining().await;
if remaining < 10 {
    tracing::warn!("Rate limit getting low: {} remaining", remaining);
}

// Rate limit is automatically checked when AuthContext is extracted
// If exceeded, request fails with ApiError::RateLimit
```

### 3. CSRF Protection

```rust
// CSRF is automatically checked for POST/PUT/PATCH/DELETE
// No code needed - happens automatically!

// For clients, include CSRF token:
// X-CSRF-Token: <token>
// Or include Origin header matching ALLOWED_ORIGINS
```

### 4. Permission Checks

```rust
// Check if user has permission
let can_edit = auth.check_permission("patient", Some(patient_id), "edit").await?;
if !can_edit {
    return Err(ApiError::authorization("Cannot edit patient"));
}

// Or require permission (throws error if not granted)
auth.require_permission("patient", Some(patient_id), "view").await?;
```

## Complete Example

```rust
use crate::middleware::AuthContext;
use crate::error::{ApiError, ApiResponse, api_success};

pub async fn create_patient(
    State(server): State<RustCareServer>,
    auth: AuthContext,  // ✅ All security automatic!
    Json(req): Json<CreatePatientRequest>,
) -> Result<Json<ApiResponse<Patient>>, ApiError> {
    // ✅ Rate limiting: Already checked
    // ✅ CSRF: Already checked (POST request)
    // ✅ Same-site: Already validated
    
    // Log with request ID
    tracing::info!(
        request_id = %auth.request_id(),
        user_id = %auth.user_id,
        "Creating patient"
    );
    
    // Check permission
    auth.require_permission("patient", None, "create").await?;
    
    // Check rate limit remaining
    let remaining = auth.rate_limit_remaining().await;
    tracing::debug!("Rate limit remaining: {}", remaining);
    
    // Create patient with organization scoping
    let patient = sqlx::query_as::<_, Patient>(
        "INSERT INTO patients (name, organization_id, created_by) 
         VALUES ($1, $2, $3) 
         RETURNING *"
    )
    .bind(req.name)
    .bind(auth.organization_id)  // ✅ Automatic org scoping
    .bind(auth.user_id)  // ✅ Automatic user tracking
    .fetch_one(&server.db_pool)
    .await?;
    
    Ok(Json(api_success(patient)))
}
```

## Migration: Zero Changes Needed! ✅

**All existing handlers work without any changes!**

The only difference is you now have **additional features available**:

### Before (Still Works)
```rust
pub async fn handler(auth: AuthContext) -> Result<...> {
    let user_id = auth.user_id;  // ✅ Works
    let org_id = auth.organization_id;  // ✅ Works
}
```

### After (New Features Available)
```rust
pub async fn handler(auth: AuthContext) -> Result<...> {
    let user_id = auth.user_id;  // ✅ Still works
    let org_id = auth.organization_id;  // ✅ Still works
    
    // ✅ NEW: Request tracking
    let request_id = auth.request_id();
    
    // ✅ NEW: Rate limit info
    let remaining = auth.rate_limit_remaining().await;
    
    // ✅ NEW: Same-site check
    if !auth.is_same_site() {
        // Handle cross-site request
    }
    
    // ✅ NEW: Permission checks
    auth.require_permission("resource", Some(id), "action").await?;
}
```

## Configuration

Security is configured in `lib.rs`:

```rust
let security_config = SecurityConfig {
    rate_limit: Some(RateLimitConfig {
        max_requests: 100,      // 100 requests
        window_seconds: 60,     // per minute
        by_user: true,           // per user
    }),
    csrf: Some(CsrfValidator::new()),
    strict_same_site: false,     // Set true to reject cross-site
};
```

## Backward Compatibility ✅

- ✅ All existing handlers using `AuthContext` work without changes
- ✅ All existing code accessing `auth.user_id`, `auth.organization_id` works
- ✅ Serialization still works (request and rate_limiter are skipped)
- ✅ Tests work without changes

## What's New

| Feature | Before | After |
|---------|--------|-------|
| Authentication | ✅ | ✅ |
| Request ID | ❌ | ✅ `auth.request_id()` |
| Rate Limiting | ❌ | ✅ Automatic |
| CSRF Protection | ❌ | ✅ Automatic |
| Same-Site Check | ❌ | ✅ `auth.is_same_site()` |
| Permission Checks | ✅ Manual | ✅ `auth.require_permission()` |

## Summary

**Zero migration needed!** All security features are now built into `AuthContext` and work automatically. Existing handlers continue to work, and you can optionally use the new features (request tracking, rate limit info, etc.) when needed.

