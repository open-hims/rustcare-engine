//! Audit logging for secrets access

use serde::{Deserialize, Serialize};
use tracing::{info, warn};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AuditEvent {
    pub timestamp: chrono::DateTime<chrono::Utc>,
    pub event_type: AuditEventType,
    pub secret_key: String,
    pub user: Option<String>,
    pub success: bool,
    pub error_message: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum AuditEventType {
    SecretAccessed,
    SecretCreated,
    SecretUpdated,
    SecretDeleted,
    SecretRotated,
    SecretExpired,
    AccessDenied,
}

pub struct AuditLogger {
    enabled: bool,
    log_all_access: bool,
}

impl AuditLogger {
    pub fn new(enabled: bool, log_all_access: bool) -> Self {
        Self {
            enabled,
            log_all_access,
        }
    }
    
    pub fn log_event(&self, event: AuditEvent) {
        if !self.enabled {
            return;
        }
        
        // Skip logging routine access events if not configured
        if !self.log_all_access && matches!(event.event_type, AuditEventType::SecretAccessed) {
            return;
        }
        
        if event.success {
            info!(
                event_type = ?event.event_type,
                secret_key = %event.secret_key,
                user = ?event.user,
                "Audit event"
            );
        } else {
            warn!(
                event_type = ?event.event_type,
                secret_key = %event.secret_key,
                user = ?event.user,
                error = ?event.error_message,
                "Audit event failed"
            );
        }
    }
    
    pub fn log_access(&self, secret_key: &str, user: Option<&str>) {
        if !self.enabled || !self.log_all_access {
            return;
        }
        
        let event = AuditEvent {
            timestamp: chrono::Utc::now(),
            event_type: AuditEventType::SecretAccessed,
            secret_key: secret_key.to_string(),
            user: user.map(|s| s.to_string()),
            success: true,
            error_message: None,
        };
        
        self.log_event(event);
    }
}
