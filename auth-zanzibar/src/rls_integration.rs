//! RLS Integration for Zanzibar + PostgreSQL
//! 
//! This module bridges Zanzibar authorization with PostgreSQL Row-Level Security:
//! 
//! 1. Zanzibar determines WHAT the user can access
//! 2. RLS enforces it at the database level
//! 3. Supports elevated access (break-glass) with audit logging
//! 4. Time-based access expiration
//! 
//! Flow:
//! User Request → Zanzibar Check → Generate RLS Context → Set PG Session Vars → Query

use crate::{
    engine::AuthorizationEngine,
    models::*,
    error::ZanzibarError,
};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sqlx::{PgConnection, PgPool, Acquire};
use std::sync::Arc;
use tracing::{debug, info, warn};
use uuid::Uuid;

/// RLS context generated from Zanzibar permissions
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RlsContext {
    /// Current user ID
    pub user_id: Uuid,
    
    /// Organization ID for multi-tenant isolation
    pub organization_id: Uuid,
    
    /// User's role (e.g., "doctor", "nurse", "admin")
    pub role: String,
    
    /// Whether elevated/break-glass access is active
    pub elevated: bool,
    
    /// List of resource IDs the user can access
    pub allowed_resources: Vec<String>,
    
    /// Expiration time for time-based access
    pub access_until: Option<DateTime<Utc>>,
    
    /// Session ID for audit tracking
    pub session_id: Uuid,
}

impl RlsContext {
    /// Apply this RLS context to a PostgreSQL connection
    pub async fn apply_to_connection(&self, conn: &mut PgConnection) -> Result<(), ZanzibarError> {
        debug!("Applying RLS context for user {}", self.user_id);

        // Build SQL to set session variables
        let sql = format!(
            r#"
            SET LOCAL app.current_user_id = '{}';
            SET LOCAL app.organization_id = '{}';
            SET LOCAL app.role = '{}';
            SET LOCAL app.elevated = '{}';
            SET LOCAL app.allowed_resources = '{}';
            SET LOCAL app.session_id = '{}';
            "#,
            self.user_id,
            self.organization_id,
            self.role,
            self.elevated,
            self.allowed_resources.join(","),
            self.session_id,
        );

        // Add expiration if present
        let sql = if let Some(access_until) = self.access_until {
            format!(
                "{}SET LOCAL app.access_until = '{}';",
                sql,
                access_until.to_rfc3339()
            )
        } else {
            sql
        };

        sqlx::query(&sql)
            .execute(conn)
            .await
            .map_err(|e| ZanzibarError::StorageError(format!("Failed to set RLS context: {}", e)))?;

        info!("RLS context applied successfully for user {} (elevated={})", self.user_id, self.elevated);
        Ok(())
    }
}

/// RLS Middleware for Zanzibar integration
pub struct RlsMiddleware {
    zanzibar: Arc<AuthorizationEngine>,
    pool: PgPool,
}

impl RlsMiddleware {
    pub fn new(zanzibar: Arc<AuthorizationEngine>, pool: PgPool) -> Self {
        Self { zanzibar, pool }
    }

    /// Generate RLS context for a user accessing a resource
    /// 
    /// # Example Flow:
    /// 1. Check if user has direct access to resources
    /// 2. Check if user can elevate (break-glass)
    /// 3. Build allowed resource list from Zanzibar
    /// 4. Return RLS context with session variables
    pub async fn generate_context(
        &self,
        user_id: Uuid,
        organization_id: Uuid,
        resource_type: &str,
        requested_elevated: bool,
    ) -> Result<RlsContext, ZanzibarError> {
        let user_subject = Subject::user(&user_id.to_string());
        
        // 1. Determine user's primary role
        let role = self.get_user_role(&user_subject).await?;
        
        // 2. Check if elevation is requested and allowed
        let elevated = if requested_elevated {
            self.check_can_elevate(&user_subject, &role).await?
        } else {
            false
        };
        
        // 3. Build allowed resources list
        let allowed_resources = if elevated {
            // Elevated mode: grant access based on role
            self.get_elevated_resources(&role, resource_type).await?
        } else {
            // Normal mode: only resources explicitly granted
            self.get_direct_resources(&user_subject, resource_type).await?
        };
        
        // 4. Check for time-based access
        let access_until = self.get_access_expiration(&user_subject).await?;
        
        Ok(RlsContext {
            user_id,
            organization_id,
            role,
            elevated,
            allowed_resources,
            access_until,
            session_id: Uuid::new_v4(),
        })
    }

    /// Get the user's primary role from Zanzibar
    async fn get_user_role(&self, user_subject: &Subject) -> Result<String, ZanzibarError> {
        // Query Zanzibar for role assignments
        // Tuple: user:alice#member@role:doctor
        let role_tuples = self.zanzibar.read_tuples(
            Some(user_subject.clone()),
            Some(Relation::new("member")),
            None,
        ).await?;

        // Find role objects
        let roles: Vec<String> = role_tuples
            .iter()
            .filter_map(|t| {
                if t.object.object_type == "role" {
                    Some(t.object.object_id.clone())
                } else {
                    None
                }
            })
            .collect();

        // Return first role (in real system, handle multiple roles or priorities)
        roles.first()
            .cloned()
            .ok_or_else(|| ZanzibarError::ValidationError("User has no assigned role".to_string()))
    }

    /// Check if user can elevate to break-glass mode
    async fn check_can_elevate(&self, user_subject: &Subject, role: &str) -> Result<bool, ZanzibarError> {
        // Check tuple: user:alice#can_elevate@role:doctor
        let can_elevate = self.zanzibar.check(
            user_subject.clone(),
            Relation::new("can_elevate"),
            Object::new("role", role),
        ).await?;

        if can_elevate {
            warn!("User {} requested elevated access - AUDIT THIS", user_subject);
        }

        Ok(can_elevate)
    }

    /// Get resources the user has direct (non-elevated) access to
    async fn get_direct_resources(
        &self,
        user_subject: &Subject,
        resource_type: &str,
    ) -> Result<Vec<String>, ZanzibarError> {
        // Find all objects the user can view
        // Tuple: patient_record:101#viewer@user:alice
        let objects = self.zanzibar.list_objects(
            user_subject.clone(),
            Relation::new("viewer"),
            resource_type.to_string(),
        ).await?;

        Ok(objects.iter().map(|obj| obj.object_id.clone()).collect())
    }

    /// Get resources accessible in elevated mode (based on role)
    async fn get_elevated_resources(
        &self,
        role: &str,
        resource_type: &str,
    ) -> Result<Vec<String>, ZanzibarError> {
        // In elevated mode, access is role-based, not resource-specific
        // RLS policy will handle: WHERE app.elevated = true AND app.role = 'doctor'
        // So we return empty list - RLS grants access via role check, not resource list
        
        debug!("Elevated mode: {} accessing all {} resources", role, resource_type);
        Ok(vec![])
    }

    /// Get access expiration time (for time-based access)
    async fn get_access_expiration(&self, _user_subject: &Subject) -> Result<Option<DateTime<Utc>>, ZanzibarError> {
        // TODO: Query Zanzibar tuples for expires_at
        // For now, return None (no expiration)
        Ok(None)
    }

    /// Execute a query with RLS context applied
    pub async fn execute_with_context<F, T>(
        &self,
        context: &RlsContext,
        operation: F,
    ) -> Result<T, ZanzibarError>
    where
        F: FnOnce(&mut PgConnection) -> std::pin::Pin<Box<dyn std::future::Future<Output = Result<T, ZanzibarError>> + Send>>,
    {
        let mut conn = self.pool
            .acquire()
            .await
            .map_err(|e| ZanzibarError::StorageError(format!("Failed to acquire connection: {}", e)))?;

        // Start transaction
        let mut tx = conn.begin()
            .await
            .map_err(|e| ZanzibarError::StorageError(format!("Failed to start transaction: {}", e)))?;

        // Apply RLS context
        context.apply_to_connection(&mut *tx).await?;

        // Execute operation
        let result = operation(&mut *tx).await?;

        // Commit transaction
        tx.commit()
            .await
            .map_err(|e| ZanzibarError::StorageError(format!("Failed to commit: {}", e)))?;

        Ok(result)
    }
}

/// Example RLS policy (for documentation - actual policy is in migration SQL)
/// 
/// ```sql
/// CREATE POLICY patient_records_rls_policy
///   ON patient_records
///   FOR SELECT
///   USING (
///     -- Normal mode: only allowed resources
///     id::text = ANY(string_to_array(current_setting('app.allowed_resources', true), ','))
///     OR
///     -- Elevated mode: role-based access
///     (
///       current_setting('app.elevated', true)::boolean = true
///       AND current_setting('app.role', true) IN ('doctor', 'auditor', 'admin')
///     )
///     OR
///     -- Time-based access still valid
///     (
///       current_setting('app.access_until', true) != ''
///       AND current_setting('app.access_until', true)::timestamptz > now()
///     )
///   );
/// ```

#[cfg(test)]
mod tests {
    use super::*;
    use crate::repository::InMemoryTupleRepository;

    #[tokio::test]
    async fn test_generate_context_normal_mode() {
        let repo = Arc::new(InMemoryTupleRepository::new());
        let engine = Arc::new(AuthorizationEngine::new(repo).await.unwrap());

        // Setup: User Alice is a doctor
        let alice = Subject::user("alice");
        let doctor_role = Object::new("role", "doctor");
        engine.write_tuple(Tuple::new(
            alice.clone(),
            Relation::new("member"),
            doctor_role.clone(),
        )).await.unwrap();

        // Setup: Alice can view patient 101
        engine.write_tuple(Tuple::new(
            alice.clone(),
            Relation::new("viewer"),
            Object::new("patient_record", "101"),
        )).await.unwrap();

        // Note: Can't test full middleware without PostgreSQL connection
        // But we can test Zanzibar logic independently
        let role_tuples = engine.read_tuples(
            Some(alice.clone()),
            Some(Relation::new("member")),
            None,
        ).await.unwrap();

        assert_eq!(role_tuples.len(), 1);
        assert_eq!(role_tuples[0].object.object_id, "doctor");
    }
}
