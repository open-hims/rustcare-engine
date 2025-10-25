use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sqlx::FromRow;
use uuid::Uuid;
use std::collections::HashMap;

// ============================================================================
// CONFIGURABLE DEVICE TYPES & ENUMS
// ============================================================================

/// Device type - fully configurable via database/config
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct DeviceType {
    pub code: String,
    pub name: String,
    pub category: String,
    pub description: Option<String>,
}

/// Device status - configurable workflow states
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct DeviceStatus {
    pub code: String,
    pub name: String,
    pub color: String, // For UI: red, green, yellow, etc.
    pub description: Option<String>,
}

/// Connection type - configurable communication methods
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ConnectionType {
    pub code: String,
    pub name: String,
    pub protocol: String,
    pub default_port: Option<u16>,
    pub requires_auth: bool,
    pub settings_schema: serde_json::Value, // JSON Schema for connection settings
}

/// Data format - configurable parsers/generators
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct DataFormat {
    pub code: String,
    pub name: String,
    pub version: Option<String>,
    pub mime_type: String,
    pub parser_plugin: String, // Plugin name to handle this format
    pub validator_plugin: Option<String>,
}

// ============================================================================
// DEVICE CONFIGURATION - FULLY DYNAMIC
// ============================================================================

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DeviceConfig {
    /// Connection settings - completely dynamic
    pub connection: ConnectionSettings,
    
    /// Protocol settings - varies by device
    pub protocol: ProtocolSettings,
    
    /// Authentication - configurable per device
    pub auth: Option<AuthSettings>,
    
    /// Advanced settings - plugin-specific
    pub advanced: Option<AdvancedSettings>,
    
    /// Custom fields - for plugin extensions
    pub custom: HashMap<String, serde_json::Value>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConnectionSettings {
    pub connection_type: String, // Reference to ConnectionType.code
    pub host: Option<String>,
    pub port: Option<u16>,
    pub path: Option<String>,
    pub serial_port: Option<String>,
    pub baud_rate: Option<u32>,
    pub data_bits: Option<u8>,
    pub stop_bits: Option<u8>,
    pub parity: Option<String>,
    pub flow_control: Option<String>,
    pub custom: HashMap<String, serde_json::Value>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProtocolSettings {
    pub format: String, // Reference to DataFormat.code
    pub version: Option<String>,
    pub encoding: Option<String>,
    pub delimiter: Option<String>,
    pub compression: Option<String>,
    pub custom: HashMap<String, serde_json::Value>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AuthSettings {
    pub auth_type: String, // basic, bearer, api_key, certificate, oauth2, etc.
    pub username: Option<String>,
    #[serde(skip_serializing)] // Never serialize passwords
    pub password: Option<String>,
    pub api_key: Option<String>,
    pub token: Option<String>,
    pub certificate_path: Option<String>,
    pub client_id: Option<String>,
    pub client_secret: Option<String>,
    pub custom: HashMap<String, serde_json::Value>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AdvancedSettings {
    pub timeout_ms: Option<u64>,
    pub retry_attempts: Option<u8>,
    pub retry_delay_ms: Option<u64>,
    pub polling_interval_ms: Option<u64>,
    pub buffer_size: Option<usize>,
    pub queue_size: Option<usize>,
    pub batch_size: Option<usize>,
    pub keep_alive: Option<bool>,
    pub custom: HashMap<String, serde_json::Value>,
}

// ============================================================================
// DEVICE MODEL - FLEXIBLE SCHEMA
// ============================================================================

#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct Device {
    pub id: Uuid,
    pub name: String,
    pub device_type: String, // Reference to DeviceType.code
    pub manufacturer: String,
    pub model: String,
    pub serial_number: String,
    
    // Location - configurable hierarchy
    pub location: serde_json::Value, // { department, building, floor, room, bed, etc. }
    
    // Status
    pub status: String, // Reference to DeviceStatus.code
    pub last_connected: Option<DateTime<Utc>>,
    pub last_data_received: Option<DateTime<Utc>>,
    pub last_error: Option<String>,
    
    // Configuration (stored as JSONB) - completely dynamic
    pub config: serde_json::Value,
    
    // Metadata - extensible
    pub metadata: serde_json::Value, // { firmware, calibration_date, maintenance_date, tags, etc. }
    
    // Timestamps
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    pub created_by: Option<Uuid>,
    pub updated_by: Option<Uuid>,
}

impl Device {
    pub fn new(
        name: String,
        device_type: String,
        manufacturer: String,
        model: String,
        serial_number: String,
        location: serde_json::Value,
        config: DeviceConfig,
    ) -> Self {
        Self {
            id: Uuid::new_v4(),
            name,
            device_type,
            manufacturer,
            model,
            serial_number,
            location,
            status: "disconnected".to_string(), // Default status
            last_connected: None,
            last_data_received: None,
            last_error: None,
            config: serde_json::to_value(config).unwrap(),
            metadata: serde_json::json!({}),
            created_at: Utc::now(),
            updated_at: Utc::now(),
            created_by: None,
            updated_by: None,
        }
    }
    
    pub fn get_config(&self) -> Result<DeviceConfig, serde_json::Error> {
        serde_json::from_value(self.config.clone())
    }
    
    pub fn update_status(&mut self, status: String) {
        self.status = status;
        self.updated_at = Utc::now();
    }
}

// ============================================================================
// DEVICE DATA - FLEXIBLE STORAGE
// ============================================================================

#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct DeviceData {
    pub id: Uuid,
    pub device_id: Uuid,
    pub timestamp: DateTime<Utc>,
    
    // Data classification
    pub data_type: String, // vital_signs, lab_result, image, etc.
    pub format: String, // Reference to DataFormat.code
    
    // Raw and parsed data
    pub raw_data: serde_json::Value,
    pub parsed_data: Option<serde_json::Value>,
    pub normalized_data: Option<serde_json::Value>, // FHIR format
    
    // Context
    pub patient_id: Option<Uuid>,
    pub encounter_id: Option<Uuid>,
    pub provider_id: Option<Uuid>,
    
    // Metadata
    pub metadata: serde_json::Value,
    
    pub created_at: DateTime<Utc>,
}

// ============================================================================
// DEVICE COMMAND - EXTENSIBLE
// ============================================================================

#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct DeviceCommand {
    pub id: Uuid,
    pub device_id: Uuid,
    pub command: String,
    pub parameters: serde_json::Value,
    pub metadata: serde_json::Value,
    
    pub created_at: DateTime<Utc>,
    pub executed_at: Option<DateTime<Utc>>,
    pub completed_at: Option<DateTime<Utc>>,
    
    pub status: String, // pending, executing, completed, failed
    pub response: Option<serde_json::Value>,
    pub error: Option<String>,
}

// ============================================================================
// DEVICE STATS - AGGREGATED
// ============================================================================

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DeviceStats {
    pub total: i64,
    pub by_type: HashMap<String, i64>,
    pub by_status: HashMap<String, i64>,
    pub by_location: HashMap<String, i64>,
}
