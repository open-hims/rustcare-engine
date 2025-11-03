# Refactoring Implementation Guide

**Created**: 2025-01-30  
**Status**: In Progress - First 5 Refactorings Implemented

This guide provides step-by-step instructions for using the new refactored utilities.

---

## ✅ Completed Refactorings

### 1. Timestamp Standardization ✅

**Location**: `rustcare-server/src/utils/timestamps.rs`

**Usage Example**:
```rust
use crate::utils::timestamps::{ApiTimestamp, now_rfc3339, parse_rfc3339};

// In struct definitions:
#[derive(Debug, Serialize, Deserialize)]
pub struct MyStruct {
    pub id: Uuid,
    pub created_at: ApiTimestamp,  // Instead of String or DateTime<Utc>
    pub updated_at: ApiTimestamp,
}

// Creating timestamps:
let my_struct = MyStruct {
    id: Uuid::new_v4(),
    created_at: ApiTimestamp::now(),
    updated_at: ApiTimestamp::now(),
};

// Parsing from API:
let timestamp = ApiTimestamp::from_rfc3339("2024-01-15T10:30:00Z")?;
```

**Migration**:
- Replace all `String` timestamp fields with `ApiTimestamp`
- Replace all `DateTime<Utc>` fields (optional, ApiTimestamp wraps it)
- Use `ApiTimestamp::now()` instead of `Utc::now().to_rfc3339()`

---

### 2. Query Builder Utilities ✅

**Location**: `rustcare-server/src/utils/query_builder.rs`

**Usage Example**:
```rust
use crate::utils::query_builder::PaginatedQuery;

// Before (15+ lines):
let mut query_builder = sqlx::QueryBuilder::new(
    "SELECT * FROM medical_records WHERE is_deleted = false"
);
if let Some(patient_id) = params.patient_id {
    query_builder.push(" AND patient_id = ");
    query_builder.push_bind(patient_id);
}
if let Some(provider_id) = params.provider_id {
    query_builder.push(" AND provider_id = ");
    query_builder.push_bind(provider_id);
}
query_builder.push(" ORDER BY visit_date DESC");
let page = params.page.unwrap_or(1);
let page_size = params.page_size.unwrap_or(20);
let offset = (page - 1) * page_size;
query_builder.push(" LIMIT ").push_bind(page_size as i64);
query_builder.push(" OFFSET ").push_bind(offset as i64);

// After (5 lines):
let mut query = PaginatedQuery::new(
    "SELECT * FROM medical_records WHERE is_deleted = false"
);
query
    .filter_active()
    .filter_organization(auth.organization_id)
    .filter_eq("patient_id", params.patient_id)
    .filter_eq("provider_id", params.provider_id)
    .order_by("visit_date", "DESC")
    .paginate(params.page, params.page_size);

let records: Vec<MedicalRecord> = query.build().fetch_all(&server.db_pool).await?;
```

---

### 3. Authentication Context Extraction 🚧

**Location**: `rustcare-server/src/middleware/auth_context.rs`

**Current Status**: Infrastructure created, needs JWT validation integration

**Usage Example**:
```rust
use crate::middleware::AuthContext;

// Before:
pub async fn create_notification(
    State(app_state): State<RustCareServer>,
    Json(req): Json<CreateNotificationRequest>,
) -> Result<Json<ApiResponse<Notification>>, ApiError> {
    sqlx::query(/* ... */)
        .bind(Uuid::nil()) // TODO: Get from auth context
        .bind(Uuid::nil()) // TODO: Get from auth context
        .execute(&app_state.db_pool)
        .await?;
}

// After:
pub async fn create_notification(
    State(app_state): State<RustCareServer>,
    auth: AuthContext, // Automatically extracted!
    Json(req): Json<CreateNotificationRequest>,
) -> Result<Json<ApiResponse<Notification>>, ApiError> {
    sqlx::query(/* ... */)
        .bind(auth.organization_id) // Use actual context
        .bind(auth.user_id)
        .execute(&app_state.db_pool)
        .await?;
}
```

**Next Steps**: Integrate with `auth-gateway` module for actual JWT validation.

---

### 4. Pagination Standardization ✅

**Location**: `rustcare-server/src/types/pagination.rs`

**Usage Example**:
```rust
use crate::types::pagination::PaginationParams;

// In handler params:
#[derive(Debug, Deserialize, IntoParams)]
pub struct ListMedicalRecordsParams {
    pub patient_id: Option<Uuid>,
    #[serde(flatten)]
    pub pagination: PaginationParams, // Standard pagination
}

// In handler:
pub async fn list_medical_records(
    State(server): State<RustCareServer>,
    Query(params): Query<ListMedicalRecordsParams>,
) -> Result<Json<ApiResponse<Vec<MedicalRecord>>>, ApiError> {
    // ... query building ...
    let total_count = get_total_count().await?;
    let metadata = params.pagination.to_metadata(total_count);
    
    Ok(Json(api_success_with_meta(records, metadata)))
}
```

---

### 5. Generic CRUD Handler Traits 🚧

**Location**: `rustcare-server/src/handlers/common/crud.rs`

**Usage Example**:
```rust
use crate::handlers::common::crud::CrudHandler;

// Define your list params with pagination:
#[derive(Debug, Deserialize)]
pub struct ListPharmaciesParams {
    #[serde(flatten)]
    pub pagination: PaginationParams,
    pub is_active: Option<bool>,
}

// Implement the trait:
pub struct PharmacyHandler;

#[async_trait]
impl CrudHandler<Pharmacy, CreatePharmacyRequest, UpdatePharmacyRequest, ListPharmaciesParams> 
    for PharmacyHandler 
{
    fn table_name() -> &'static str {
        "pharmacies"
    }
    
    fn apply_filters(query: &mut PaginatedQuery, params: &ListPharmaciesParams) -> Result<(), ApiError> {
        query.filter_active();
        if let Some(is_active) = params.is_active {
            query.filter_eq("is_active", Some(is_active));
        }
        Ok(())
    }
    
    fn extract_page(params: &ListPharmaciesParams) -> Option<u32> {
        params.pagination.page
    }
    
    fn extract_page_size(params: &ListPharmaciesParams) -> Option<u32> {
        params.pagination.page_size
    }
    
    // Override create for custom logic
    async fn create(
        State(server): State<RustCareServer>,
        Json(req): Json<CreatePharmacyRequest>,
    ) -> Result<Json<ApiResponse<Pharmacy>>, ApiError> {
        // Custom create logic
        // ...
    }
}

// Use in routes:
// .route("/pharmacies", get(PharmacyHandler::list))
// .route("/pharmacies/:id", get(PharmacyHandler::get))
// .route("/pharmacies", post(PharmacyHandler::create))
```

---

## Migration Checklist

### Phase 1: Update Existing Handlers

- [ ] Update `handlers/pharmacy.rs` to use new utilities
- [ ] Update `handlers/vendors.rs` to use new utilities
- [ ] Update `handlers/healthcare.rs` to use PaginatedQuery
- [ ] Update `handlers/notifications.rs` to use AuthContext
- [ ] Update `handlers/organizations.rs` to use ApiTimestamp

### Phase 2: Standardize All Handlers

- [ ] All handlers use `PaginationParams`
- [ ] All handlers use `ApiTimestamp` for timestamps
- [ ] All handlers use `PaginatedQuery` for queries
- [ ] All handlers use `AuthContext` for auth
- [ ] All handlers return `Result<..., ApiError>`

---

## Next Steps

1. **Integrate JWT validation** in `auth_context.rs`
2. **Create example handler** using all utilities
3. **Migrate one handler** as proof of concept
4. **Update tests** for new utilities
5. **Document migration path** for remaining handlers

