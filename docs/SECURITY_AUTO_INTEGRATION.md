# Automatic Security Integration Guide

## Overview

The security middleware is now **automatically integrated** into all API routes. Here's how it works:

## Automatic Integration ✅

### 1. Router Setup (Already Done)

The security state is automatically added to all routes in `lib.rs`:

```rust
pub fn create_app(server: RustCareServer) -> Router {
    let security_state = SecurityState::new(SecurityConfig::default());
    
    routes::create_routes()
        .layer(Extension(security_state)) // ✅ Automatically available
        .with_state(server)
}
```

### 2. Automatic Extraction

Handlers can now use **three levels** of security extraction:

#### Level 1: Full Security Context (Recommended) ✅

```rust
use crate::middleware::SecureContext;

pub async fn handler(
    security: SecureContext,  // ✅ Automatically extracts AuthContext + RequestContext + performs checks
) -> Result<Json<ApiResponse<T>>, ApiError> {
    // Rate limiting ✅ Already checked
    // CSRF protection ✅ Already checked  
    // Same-site validation ✅ Already checked
    
    // Check permissions
    security.require_permission("patient", Some(patient_id), "view").await?;
    
    // Access auth context
    let user_id = security.auth.user_id;
    let org_id = security.auth.organization_id;
    
    // Access request context
    let request_id = &security.request.request_id;
    
    Ok(Json(api_success(data)))
}
```

**What's automatic:**
- ✅ Rate limiting check (fails if exceeded)
- ✅ CSRF token validation (for POST/PUT/DELETE)
- ✅ Same-site validation
- ✅ Request ID tracking
- ✅ Auth context extraction

#### Level 2: Individual Contexts (Current Handlers)

Existing handlers using `AuthContext` **already work** - they get authentication automatically:

```rust
use crate::middleware::AuthContext;

pub async fn handler(
    auth: AuthContext,  // ✅ Still works - authentication is automatic
) -> Result<Json<ApiResponse<T>>, ApiError> {
    // Auth is already extracted
    let user_id = auth.user_id;
    
    // But rate limiting and CSRF are NOT checked
    // For those, use SecureContext instead
}
```

#### Level 3: Request Context Only (Public Endpoints)

For public endpoints that don't need authentication:

```rust
use crate::middleware::ReqContext;

pub async fn handler(
    req_ctx: ReqContext,  // ✅ Only request context (no auth)
) -> Result<Json<ApiResponse<T>>, ApiError> {
    let request_id = &req_ctx.request_id;
    // No auth, but request tracking is automatic
}
```

## Migration Path

### Option A: Gradual Migration (Recommended)

Keep existing handlers using `AuthContext`, and update to `SecureContext` when you need:
- Rate limiting
- CSRF protection
- Same-site validation

**Example:**
```rust
// Before (still works)
pub async fn list_pharmacies(
    auth: AuthContext,
    // ...
) -> Result<...> {
    // Auth is automatic ✅
}

// After (adds rate limiting + CSRF)
pub async fn create_pharmacy(
    security: SecureContext,  // ✅ Upgraded
    // ...
) -> Result<...> {
    // Rate limiting ✅ Automatic
    // CSRF ✅ Automatic
    // Auth ✅ Automatic
    security.require_permission("pharmacy", None, "create").await?;
}
```

### Option B: Update All Handlers

Update all handlers to use `SecureContext` for consistent security:

```rust
// Find and replace in handlers:
// auth: AuthContext  →  security: SecureContext
// auth.user_id       →  security.auth.user_id
// auth.organization_id → security.auth.organization_id
```

## What's Automatic vs Manual

### ✅ Automatic (No Code Changes Needed)

1. **AuthContext extraction** - All handlers using `AuthContext` work automatically
2. **RequestContext extraction** - Available if handler uses `RequestContext` or `SecureContext`
3. **Security state** - Available in router extensions
4. **JWT validation** - Automatic when using `AuthContext` or `SecureContext`

### ⚠️ Requires Handler Update

1. **Rate limiting** - Only active if handler uses `SecureContext`
2. **CSRF protection** - Only active if handler uses `SecureContext`
3. **Same-site validation** - Only enforced if handler uses `SecureContext`
4. **Zanzibar checks** - Manual call: `security.require_permission(...)`

## Example: Updating a Handler

### Before:
```rust
pub async fn create_patient(
    State(server): State<RustCareServer>,
    auth: AuthContext,
    Json(req): Json<CreatePatientRequest>,
) -> Result<Json<ApiResponse<Patient>>, ApiError> {
    // No rate limiting
    // No CSRF check
    // Manual permission check needed
    
    sqlx::query("INSERT INTO patients ...")
        .bind(auth.organization_id)
        .execute(&server.db_pool)
        .await?;
    
    Ok(Json(api_success(patient)))
}
```

### After (Automatic Security):
```rust
pub async fn create_patient(
    State(server): State<RustCareServer>,
    security: SecureContext,  // ✅ Changed from AuthContext
    Json(req): Json<CreatePatientRequest>,
) -> Result<Json<ApiResponse<Patient>>, ApiError> {
    // ✅ Rate limiting: Automatic
    // ✅ CSRF check: Automatic
    // ✅ Same-site validation: Automatic
    
    // Check permission
    security.require_permission("patient", None, "create").await?;
    
    sqlx::query("INSERT INTO patients ...")
        .bind(security.auth.organization_id)  // ✅ Changed from auth.organization_id
        .execute(&server.db_pool)
        .await?;
    
    // Log with request ID
    tracing::info!(
        request_id = %security.request.request_id,
        "Patient created"
    );
    
    Ok(Json(api_success(patient)))
}
```

## Configuration

Security is configured in `lib.rs`:

```rust
let security_config = SecurityConfig {
    rate_limit: Some(RateLimitConfig {
        max_requests: 100,      // 100 requests
        window_seconds: 60,      // per minute
        by_user: true,           // per user (false = per IP)
    }),
    csrf: Some(CsrfValidator::new()),
    strict_same_site: false,    // Set true to reject cross-site requests
};
```

## Testing

### Existing Tests ✅

Handlers using `AuthContext` continue to work without changes.

### New Tests

For handlers using `SecureContext`, provide `SecurityState` in test setup:

```rust
#[tokio::test]
async fn test_handler_with_security() {
    let security_state = SecurityState::new(SecurityConfig::default());
    // ... test setup
}
```

## Summary

| Feature | AuthContext | SecureContext |
|---------|-------------|---------------|
| Authentication | ✅ Automatic | ✅ Automatic |
| Request Tracking | ❌ Manual | ✅ Automatic |
| Rate Limiting | ❌ No | ✅ Automatic |
| CSRF Protection | ❌ No | ✅ Automatic |
| Same-Site Check | ❌ No | ✅ Automatic |
| Zanzibar Checks | ✅ Manual | ✅ Manual |

**Recommendation:** Use `SecureContext` for all state-changing operations (POST/PUT/DELETE) and `AuthContext` for read-only operations (GET) if you don't need rate limiting.

