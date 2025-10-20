// Audit entry types and structures
use serde::{Serialize, Deserialize};
use uuid::Uuid;
use chrono::{DateTime, Utc};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AuditEntry {
    pub id: Uuid,
    pub timestamp: DateTime<Utc>,
    pub event_type: String,
    pub subject: String,
    pub action: String,
    pub data: serde_json::Value,
}