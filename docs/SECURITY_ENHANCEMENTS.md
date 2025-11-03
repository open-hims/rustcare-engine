# Security Enhancements - Request Context, Same-Site, Zanzibar & MCP Integration

**Date**: 2025-01-30  
**Status**: In Progress

## Overview

This document outlines the security enhancements being implemented to integrate:
1. **Request Context** - Request ID tracking and metadata
2. **Same-Site Security** - CSRF protection via Origin/Referer validation
3. **Zanzibar Authorization** - Fine-grained permission checks
4. **MCP Zanzibar Integration** - Model Context Protocol with authorization

---

## 1. Request Context Middleware ✅

### Implementation
- **File**: `rustcare-server/src/middleware/request_context.rs`
- **Status**: ✅ Complete

### Features
- **Request ID**: Extracts or generates unique request ID from `X-Request-ID` header
- **Origin/Referer Tracking**: Extracts origin and referer headers for same-site validation
- **Remote Address**: Extracts client IP from headers or connection info
- **Timestamp**: Records request timestamp
- **Same-Site Validation**: Validates Origin/Referer against allowed origins

### Usage
```rust
use crate::middleware::RequestContext;

pub async fn handler(
    req_ctx: RequestContext,
    // ... other params
) -> Result<Json<ApiResponse<T>>, ApiError> {
    // Access request metadata
    tracing::info!(
        request_id = %req_ctx.request_id,
        origin = ?req_ctx.origin,
        "Processing request"
    );
    
    // Check same-site validation
    if !req_ctx.is_same_site() {
        tracing::warn!("Same-site validation failed");
    }
    
    // ... handler logic
}
```

### Configuration
Set allowed origins via environment variable:
```bash
ALLOWED_ORIGINS=localhost,example.com,api.example.com
```

---

## 2. Enhanced AuthContext with Zanzibar ✅

### Implementation
- **File**: `rustcare-server/src/middleware/auth_context.rs`
- **Status**: ✅ Complete (needs Zanzibar engine integration)

### Features
- **Zanzibar Integration**: `ZanzibarCheck` trait for permission checks
- **Permission Checking**: `check_permission()` and `require_permission()` methods
- **Fallback**: Falls back to JWT permissions if Zanzibar engine unavailable

### Usage
```rust
use crate::middleware::AuthContext;

pub async fn handler(
    auth: AuthContext,
    // ... other params
) -> Result<Json<ApiResponse<T>>, ApiError> {
    // Check permission using Zanzibar
    auth.require_permission("patient", Some(patient_id), "view").await?;
    
    // Or check without error
    let can_edit = auth.check_permission("patient", Some(patient_id), "edit").await?;
    if !can_edit {
        return Err(ApiError::authorization("Cannot edit patient"));
    }
    
    // ... handler logic
}
```

### Zanzibar Engine Integration (TODO)
Currently, `zanzibar_engine` is `None`. To integrate:

1. **Create Zanzibar Engine Wrapper**:
```rust
use auth_zanzibar::{AuthorizationEngine, Subject, Object, Relation};

pub struct ZanzibarEngineWrapper {
    engine: Arc<AuthorizationEngine>,
}

#[async_trait]
impl ZanzibarCheck for ZanzibarEngineWrapper {
    async fn check_permission(
        &self,
        user_id: Uuid,
        resource_type: &str,
        resource_id: Option<Uuid>,
        permission: &str,
        organization_id: Uuid,
    ) -> Result<bool, String> {
        let subject = Subject::user(&user_id.to_string());
        let object = if let Some(id) = resource_id {
            Object::new(resource_type, &id.to_string())
        } else {
            Object::new(resource_type, &organization_id.to_string())
        };
        let relation = Relation::new(permission);
        
        self.engine.check(subject, relation, object, None).await
            .map_err(|e| e.to_string())
    }
}
```

2. **Initialize in Server State**:
```rust
// In server.rs
pub struct RustCareServer {
    // ... existing fields
    pub zanzibar_engine: Option<Arc<dyn ZanzibarCheck>>,
}
```

3. **Inject into AuthContext**:
```rust
// In auth_context.rs FromRequestParts implementation
// Get engine from State and attach to AuthContext
```

---

## 3. Same-Site Security ✅

### Implementation
- **File**: `rustcare-server/src/middleware/request_context.rs`
- **Status**: ✅ Complete

### How It Works
1. Extracts `Origin` header from request
2. Extracts `Referer` header if Origin is missing
3. Validates against `ALLOWED_ORIGINS` environment variable
4. Logs warnings for failed validations (doesn't reject by default)

### Configuration
```bash
# Comma-separated list of allowed origins
ALLOWED_ORIGINS=localhost,127.0.0.1,example.com,api.example.com
```

### Security Considerations
- **CSRF Protection**: Prevents cross-site request forgery
- **API Clients**: Requests without Origin/Referer are allowed (for API clients)
- **Strict Mode**: Can be enhanced to reject requests if needed

---

## 4. MCP Zanzibar Integration ⏳

### Current Status
- **MCP Server**: ✅ Exists (`mcp-server/`)
- **Zanzibar Trait**: ✅ Defined (`mcp-server/src/tools.rs`)
- **Integration**: ⏳ Needs completion

### Required Work

1. **Implement ZanzibarClient for MCP**:
```rust
// In mcp-server/src/tools.rs or new file
use auth_zanzibar::{AuthorizationEngine, Subject, Object, Relation};

pub struct McpZanzibarClient {
    engine: Arc<AuthorizationEngine>,
}

#[async_trait]
impl ZanzibarClient for McpZanzibarClient {
    async fn check_permission(
        &self,
        user_id: Uuid,
        resource_type: &str,
        resource_id: Option<Uuid>,
        permission: &str,
        organization_id: Uuid,
    ) -> Result<bool, String> {
        // Implementation similar to ZanzibarEngineWrapper above
    }
}
```

2. **Initialize in MCP Server**:
```rust
// In mcp-server/src/server.rs
pub struct McpServer {
    // ... existing fields
    zanzibar_client: Option<Arc<dyn ZanzibarClient>>,
}
```

3. **Use in Tool Execution**:
```rust
// In mcp-server/src/tools.rs execute method
async fn execute(
    &self,
    input: ToolInput,
    auth_context: &AuthContext,
    zanzibar_client: Option<&dyn ZanzibarClient>,
) -> McpResult<ToolResult> {
    // Check permission before executing tool
    if let Some(required_perm) = self.required_permission() {
        if let Some(zanzibar) = zanzibar_client {
            let has_permission = zanzibar.check_permission(
                auth_context.user_id,
                self.resource_type(),
                None,
                required_perm,
                auth_context.organization_id,
            ).await.map_err(|e| McpError::Permission(e))?;
            
            if !has_permission {
                return Err(McpError::Permission("Permission denied".to_string()));
            }
        }
    }
    
    // Execute tool
    // ...
}
```

---

## 5. Unified Security Middleware ⏳

### Planned Implementation
Create a middleware that combines:
- Request context extraction
- Authentication (JWT validation)
- Same-site validation
- Zanzibar authorization (optional)

### Structure
```rust
// rustcare-server/src/middleware/security.rs
pub struct SecurityContext {
    pub request: RequestContext,
    pub auth: AuthContext,
}

#[async_trait]
impl<S> FromRequestParts<S> for SecurityContext {
    // Combines RequestContext and AuthContext extraction
    // Performs same-site validation
    // Sets up Zanzibar engine if available
}
```

---

## Integration Checklist

### ✅ Completed
- [x] RequestContext middleware with same-site validation
- [x] Enhanced AuthContext with Zanzibar trait support
- [x] Permission checking methods in AuthContext
- [x] Request ID extraction and tracking

### ⏳ In Progress
- [ ] Zanzibar engine wrapper implementation
- [ ] Zanzibar engine initialization in server state
- [ ] Zanzibar engine injection into AuthContext
- [ ] MCP ZanzibarClient implementation
- [ ] MCP tool permission checks
- [ ] Unified security middleware

### 📋 Future Enhancements
- [ ] Strict same-site rejection mode
- [ ] Rate limiting per request context
- [ ] Request context caching
- [ ] Zanzibar permission caching
- [ ] Audit logging integration with request context

---

## Testing

### Unit Tests
- ✅ RequestContext creation and validation
- ✅ Same-site validation logic
- ✅ AuthContext permission checking (mock)

### Integration Tests Needed
- [ ] RequestContext extraction in handlers
- [ ] Zanzibar permission checks end-to-end
- [ ] MCP tool execution with Zanzibar
- [ ] Same-site validation in real requests

---

## Configuration

### Environment Variables
```bash
# Same-site validation
ALLOWED_ORIGINS=localhost,example.com

# JWT (existing)
JWT_SECRET=your-secret-key

# Zanzibar (future)
ZANZIBAR_ENABLED=true
ZANZIBAR_DATABASE_URL=postgresql://...
```

---

## Next Steps

1. **Complete Zanzibar Engine Integration**
   - Create `ZanzibarEngineWrapper`
   - Initialize in `RustCareServer`
   - Inject into `AuthContext` during extraction

2. **Complete MCP Integration**
   - Implement `McpZanzibarClient`
   - Integrate into MCP server
   - Add permission checks to tool execution

3. **Create Unified Middleware**
   - Combine RequestContext and AuthContext
   - Add security headers
   - Implement rate limiting

4. **Add Integration Tests**
   - Test end-to-end flows
   - Test permission checks
   - Test same-site validation

---

**Status**: Foundation complete, integration work remaining.

