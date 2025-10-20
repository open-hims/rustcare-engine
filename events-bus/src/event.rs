// Event types and structures
use serde::{Serialize, Deserialize};
use uuid::Uuid;
use chrono::{DateTime, Utc};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Event {
    pub id: Uuid,
    pub event_type: String,
    pub data: serde_json::Value,
    pub timestamp: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DomainEvent {
    pub event_type: String,
    pub data: serde_json::Value,
}

impl DomainEvent {
    pub fn new(event_type: &str, data: serde_json::Value) -> Self {
        Self {
            event_type: event_type.to_string(),
            data,
        }
    }
}