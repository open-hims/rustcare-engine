// Database models
use serde::{Deserialize, Serialize};
use uuid::Uuid;
use chrono::{DateTime, NaiveDate, Utc};

/// Base trait for all database models
pub trait DatabaseModel {
    fn table_name() -> &'static str;
}

/// Example of a new model using embedded audit fields
/// This is the recommended approach for new models
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExampleModel {
    pub id: Uuid,
    pub name: String,
    pub description: Option<String>,
    pub is_active: bool,
    pub metadata: Option<serde_json::Value>,
    
    // Embedded audit fields
    #[serde(flatten)]
    pub audit: AuditFields,
}

impl ExampleModel {
    pub fn new(name: String, user_id: Option<Uuid>) -> Self {
        Self {
            id: Uuid::new_v4(),
            name,
            description: None,
            is_active: true,
            metadata: None,
            audit: AuditFields::new(user_id),
        }
    }
}

impl Auditable for ExampleModel {
    fn get_audit_fields(&self) -> &AuditFields {
        &self.audit
    }
    
    fn get_audit_fields_mut(&mut self) -> &mut AuditFields {
        &mut self.audit
    }
}

impl DatabaseModel for ExampleModel {
    fn table_name() -> &'static str {
        "example_models"
    }
}

/// Utility function to create audit fields for database operations
pub fn create_audit_metadata(
    operation: &str,
    old_values: Option<serde_json::Value>,
    new_values: Option<serde_json::Value>,
    additional_context: Option<serde_json::Value>,
) -> serde_json::Value {
    let mut metadata = serde_json::Map::new();
    metadata.insert("operation".to_string(), serde_json::Value::String(operation.to_string()));
    
    if let Some(old) = old_values {
        metadata.insert("old_values".to_string(), old);
    }
    
    if let Some(new) = new_values {
        metadata.insert("new_values".to_string(), new);
    }
    
    if let Some(context) = additional_context {
        metadata.insert("context".to_string(), context);
    }
    
    serde_json::Value::Object(metadata)
}

/// Audit trail information for database operations
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AuditInfo {
    pub user_id: Uuid,
    pub tenant_id: String,
    pub operation: String,
    pub table_name: String,
    pub record_id: Option<String>,
    pub timestamp: DateTime<Utc>,
    pub metadata: serde_json::Value,
}

/// Common audit fields that can be embedded in other structures
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AuditFields {
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    pub created_by: Option<Uuid>,
    pub updated_by: Option<Uuid>,
}

impl AuditFields {
    pub fn new(user_id: Option<Uuid>) -> Self {
        let now = Utc::now();
        Self {
            created_at: now,
            updated_at: now,
            created_by: user_id,
            updated_by: user_id,
        }
    }

    pub fn updated_now(&mut self, user_id: Option<Uuid>) {
        self.updated_at = Utc::now();
        self.updated_by = user_id;
    }
}

/// Base model with just ID and timestamps (legacy)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BaseModel {
    pub id: Uuid,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

impl BaseModel {
    pub fn new() -> Self {
        let now = Utc::now();
        Self {
            id: Uuid::new_v4(),
            created_at: now,
            updated_at: now,
        }
    }
}

/// Extended base model with full audit tracking
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AuditableBaseModel {
    pub id: Uuid,
    #[serde(flatten)]
    pub audit: AuditFields,
}

impl AuditableBaseModel {
    pub fn new(user_id: Option<Uuid>) -> Self {
        Self {
            id: Uuid::new_v4(),
            audit: AuditFields::new(user_id),
        }
    }

    pub fn update_audit(&mut self, user_id: Option<Uuid>) {
        self.audit.updated_now(user_id);
    }
}

/// Trait for models that support audit tracking
pub trait Auditable {
    fn get_audit_fields(&self) -> &AuditFields;
    fn get_audit_fields_mut(&mut self) -> &mut AuditFields;
    
    fn update_audit(&mut self, user_id: Option<Uuid>) {
        self.get_audit_fields_mut().updated_now(user_id);
    }
    
    fn created_by(&self) -> Option<Uuid> {
        self.get_audit_fields().created_by
    }
    
    fn updated_by(&self) -> Option<Uuid> {
        self.get_audit_fields().updated_by
    }
    
    fn created_at(&self) -> DateTime<Utc> {
        self.get_audit_fields().created_at
    }
    
    fn updated_at(&self) -> DateTime<Utc> {
        self.get_audit_fields().updated_at
    }
}

/// Implementation of Auditable for AuditableBaseModel
impl Auditable for AuditableBaseModel {
    fn get_audit_fields(&self) -> &AuditFields {
        &self.audit
    }
    
    fn get_audit_fields_mut(&mut self) -> &mut AuditFields {
        &mut self.audit
    }
}

/// Common timestamp fields
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Timestamps {
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

impl Timestamps {
    pub fn now() -> Self {
        let now = Utc::now();
        Self {
            created_at: now,
            updated_at: now,
        }
    }
}

// Geographic Models
#[derive(Debug, Clone, Serialize, Deserialize, sqlx::FromRow)]
pub struct GeographicRegion {
    pub id: Uuid,
    pub code: String,
    pub name: String,
    pub region_type: String,
    pub parent_region_id: Option<Uuid>,
    pub path: Option<String>, // ltree path
    pub level: i32,
    pub iso_country_code: Option<String>,
    pub iso_subdivision_code: Option<String>,
    pub timezone: Option<String>,
    pub coordinates: Option<String>, // Simplified from PostGIS point
    pub population: Option<i64>,
    pub area_sq_km: Option<f64>,
    pub metadata: Option<serde_json::Value>,
    pub is_active: bool,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize, sqlx::FromRow)]
pub struct PostalCodeRegionMapping {
    pub id: Uuid,
    pub region_id: Uuid,
    pub postal_code: String,
    pub postal_code_prefix: Option<String>,
    pub is_exact_match: bool,
    pub confidence_score: f64,
    pub validation_source: Option<String>,
    pub is_active: bool,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

/// Macro to generate audit field implementations for existing structs
macro_rules! impl_auditable {
    ($struct_name:ty) => {
        impl Auditable for $struct_name {
            fn get_audit_fields(&self) -> &AuditFields {
                // This is a workaround for structs that don't embed AuditFields
                // In practice, new structs should embed AuditFields directly
                panic!("get_audit_fields not supported for structs with individual audit fields. Use the individual methods instead.");
            }
            
            fn get_audit_fields_mut(&mut self) -> &mut AuditFields {
                panic!("get_audit_fields_mut not supported for structs with individual audit fields. Use update_audit() instead.");
            }
            
            fn update_audit(&mut self, user_id: Option<Uuid>) {
                self.updated_at = Utc::now();
                self.updated_by = user_id;
            }
            
            fn created_by(&self) -> Option<Uuid> {
                self.created_by
            }
            
            fn updated_by(&self) -> Option<Uuid> {
                self.updated_by
            }
            
            fn created_at(&self) -> DateTime<Utc> {
                self.created_at
            }
            
            fn updated_at(&self) -> DateTime<Utc> {
                self.updated_at
            }
        }
    };
}

#[derive(Debug, Clone, Serialize, Deserialize, sqlx::FromRow)]
pub struct ComplianceFramework {
    pub id: Uuid,
    pub organization_id: Uuid,
    pub name: String,
    pub code: String,
    pub version: String,
    pub description: Option<String>,
    pub authority: Option<String>,
    pub jurisdiction: Option<String>,
    pub effective_date: NaiveDate,
    pub review_date: Option<NaiveDate>,
    pub status: String,
    pub parent_framework_id: Option<Uuid>,
    pub metadata: Option<serde_json::Value>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    pub created_by: Option<Uuid>,
    pub updated_by: Option<Uuid>,
}

// Apply the Auditable trait to ComplianceFramework
impl_auditable!(ComplianceFramework);

#[derive(Debug, Clone, Serialize, Deserialize, sqlx::FromRow)]
pub struct ComplianceRule {
    pub id: Uuid,
    pub organization_id: Uuid,
    pub framework_id: Uuid,
    pub rule_code: String,
    pub title: String,
    pub description: Option<String>,
    pub category: Option<String>,
    pub severity: String,
    pub rule_type: String,
    pub applies_to_entity_types: Option<serde_json::Value>, // JSON array
    pub applies_to_roles: Option<serde_json::Value>, // JSON array
    pub applies_to_regions: Option<serde_json::Value>, // JSON array
    pub validation_logic: Option<serde_json::Value>,
    pub remediation_steps: Option<String>,
    pub documentation_requirements: Option<serde_json::Value>,
    pub is_automated: Option<bool>,
    pub automation_script: Option<String>,
    pub check_frequency_days: Option<i32>,
    pub last_checked_at: Option<DateTime<Utc>>,
    pub status: String,
    pub version: i32,
    pub effective_date: DateTime<Utc>,
    pub expiry_date: Option<DateTime<Utc>>,
    pub metadata: Option<serde_json::Value>,
    pub tags: Option<serde_json::Value>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    pub created_by: Option<Uuid>,
    pub updated_by: Option<Uuid>,
}

// Apply the Auditable trait to ComplianceRule
impl_auditable!(ComplianceRule);

#[derive(Debug, Clone, Serialize, Deserialize, sqlx::FromRow)]
pub struct ComplianceRegionMapping {
    pub id: Uuid,
    pub region_id: Uuid,
    pub framework_id: Uuid,
    pub assignment_reason: String,
    pub auto_assigned: bool,
    pub priority_level: i32,
    pub effective_date: DateTime<Utc>,
    pub expiry_date: Option<DateTime<Utc>>,
    pub is_active: bool,
    pub metadata: Option<serde_json::Value>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

// Implementation of DatabaseModel trait for geographic models
impl DatabaseModel for GeographicRegion {
    fn table_name() -> &'static str {
        "geographic_regions"
    }
}

impl DatabaseModel for PostalCodeRegionMapping {
    fn table_name() -> &'static str {
        "postal_code_region_mapping"
    }
}

impl DatabaseModel for ComplianceFramework {
    fn table_name() -> &'static str {
        "compliance_frameworks"
    }
}

impl DatabaseModel for ComplianceRule {
    fn table_name() -> &'static str {
        "compliance_rules"
    }
}

impl DatabaseModel for ComplianceRegionMapping {
    fn table_name() -> &'static str {
        "compliance_region_mapping"
    }
}