//! Centralized audit logging service
//!
//! This module provides a unified audit logging service to replace ad-hoc
//! audit logging code across handlers, ensuring consistent audit trails
//! and compliance with HIPAA requirements.

use sqlx::PgPool;
use uuid::Uuid;
use chrono::Utc;
use serde_json::Value as JsonValue;
use crate::error::ApiError;
use crate::middleware::AuthContext;

/// Centralized audit logging service
///
/// Provides methods to log audit events for various entity types,
/// automatically capturing organization and user context from authentication.
pub struct AuditService {
    db_pool: PgPool,
}

impl AuditService {
    /// Create a new audit service instance
    pub fn new(db_pool: PgPool) -> Self {
        Self { db_pool }
    }

    /// Log an audit action for any entity type
    ///
    /// # Arguments
    ///
    /// * `auth` - Authentication context containing user and organization info
    /// * `entity_type` - Type of entity being audited (e.g., "notification", "medical_record")
    /// * `entity_id` - UUID of the entity being audited
    /// * `action` - Action being performed (e.g., "create", "update", "delete", "view")
    /// * `details` - Optional JSON details about the action
    /// * `ip_address` - Optional IP address of the requester
    /// * `user_agent` - Optional user agent string
    ///
    /// # Example
    ///
    /// ```rust
    /// audit_service.log_action(
    ///     &auth,
    ///     "notification",
    ///     notification_id,
    ///     "create",
    ///     Some(serde_json::json!({"type": "email", "recipient": "user@example.com"})),
    ///     Some("192.168.1.1".to_string()),
    ///     Some("Mozilla/5.0...".to_string()),
    /// ).await?;
    /// ```
    pub async fn log_action(
        &self,
        auth: &AuthContext,
        entity_type: &str,
        entity_id: Uuid,
        action: &str,
        details: Option<JsonValue>,
        ip_address: Option<String>,
        user_agent: Option<String>,
    ) -> Result<(), ApiError> {
        // Determine audit table based on entity type
        let table_name = match entity_type {
            "notification" => "notification_audit_logs",
            "medical_record" => "medical_record_audit_logs",
            "prescription" => "prescription_audit_logs",
            "appointment" => "appointment_audit_logs",
            "patient" => "patient_audit_logs",
            "organization" => "organization_audit_logs",
            "user" => "user_audit_logs",
            "device" => "device_audit_logs",
            "secret" => "secret_audit_logs",
            "key" => "key_audit_logs",
            _ => "general_audit_logs",
        };

        sqlx::query(&format!(
            r#"
            INSERT INTO {} (
                entity_type, entity_id, organization_id, user_id,
                action, action_details, ip_address, user_agent, created_at
            ) VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9)
            "#,
            table_name
        ))
        .bind(entity_type)
        .bind(entity_id)
        .bind(auth.organization_id)
        .bind(auth.user_id)
        .bind(action)
        .bind(details)
        .bind(ip_address)
        .bind(user_agent)
        .bind(Utc::now())
        .execute(&self.db_pool)
        .await
        .map_err(|e| ApiError::internal(format!("Failed to log audit event: {}", e)))?;

        Ok(())
    }

    /// Log a notification-specific audit action
    ///
    /// Convenience method for notification audit logging with standardized parameters.
    pub async fn log_notification_action(
        &self,
        auth: &AuthContext,
        notification_id: Uuid,
        action: &str,
        action_details: Option<JsonValue>,
    ) -> Result<(), ApiError> {
        self.log_action(
            auth,
            "notification",
            notification_id,
            action,
            action_details,
            None,
            None,
        )
        .await
    }

    /// Log a medical record audit action
    ///
    /// Convenience method for medical record audit logging.
    pub async fn log_medical_record_action(
        &self,
        auth: &AuthContext,
        record_id: Uuid,
        action: &str,
        action_details: Option<JsonValue>,
    ) -> Result<(), ApiError> {
        self.log_action(
            auth,
            "medical_record",
            record_id,
            action,
            action_details,
            None,
            None,
        )
        .await
    }

    /// Log a patient audit action
    ///
    /// Convenience method for patient audit logging.
    pub async fn log_patient_action(
        &self,
        auth: &AuthContext,
        patient_id: Uuid,
        action: &str,
        action_details: Option<JsonValue>,
    ) -> Result<(), ApiError> {
        self.log_action(
            auth,
            "patient",
            patient_id,
            action,
            action_details,
            None,
            None,
        )
        .await
    }

    /// Log an appointment audit action
    ///
    /// Convenience method for appointment audit logging.
    pub async fn log_appointment_action(
        &self,
        auth: &AuthContext,
        appointment_id: Uuid,
        action: &str,
        action_details: Option<JsonValue>,
    ) -> Result<(), ApiError> {
        self.log_action(
            auth,
            "appointment",
            appointment_id,
            action,
            action_details,
            None,
            None,
        )
        .await
    }

    /// Log a general audit action (fallback for entities without specific tables)
    ///
    /// Uses the `general_audit_logs` table for entities that don't have
    /// dedicated audit tables.
    pub async fn log_general_action(
        &self,
        auth: &AuthContext,
        entity_type: &str,
        entity_id: Uuid,
        action: &str,
        details: Option<JsonValue>,
    ) -> Result<(), ApiError> {
        self.log_action(
            auth,
            entity_type,
            entity_id,
            action,
            details,
            None,
            None,
        )
        .await
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // Note: These tests would require a test database setup
    // For now, we verify the structure compiles correctly

    #[test]
    fn test_audit_service_structure() {
        // Verify that AuditService can be instantiated (would need mock pool in real tests)
        // This is a compile-time check
        let _service_type = std::marker::PhantomData::<AuditService>;
    }
}

