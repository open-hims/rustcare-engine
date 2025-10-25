use async_trait::async_trait;
use uuid::Uuid;
use crate::error::{DeviceError, Result};
use serde::{Deserialize, Serialize};
use chrono::{DateTime, Utc};

// ============================================================================
// DEVICE PLUGIN TRAIT
// ============================================================================

/// Core trait that all device plugins must implement
#[async_trait]
pub trait DevicePlugin: Send + Sync {
    /// Plugin name
    fn name(&self) -> &str;
    
    /// Supported device types (as string codes)
    fn supported_types(&self) -> Vec<String>;
    
    /// Connect to the device
    async fn connect(&self, device_id: &str, config: serde_json::Value) -> Result<()>;
    
    /// Disconnect from the device
    async fn disconnect(&self, device_id: &str) -> Result<()>;
    
    /// Check device status
    async fn status(&self, device_id: &str) -> Result<String>;
    
    /// Read data from device
    async fn read(&self, device_id: &str) -> Result<serde_json::Value>;
    
    /// Write command to device
    async fn write(&self, device_id: &str, command: serde_json::Value) -> Result<serde_json::Value>;
    
    /// Validate device configuration
    fn validate_config(&self, config: &serde_json::Value) -> Result<ValidationResult>;
    
    /// Test connection without actually connecting
    async fn test_connection(&self, config: &serde_json::Value) -> Result<bool>;
}

// ============================================================================
// FORMAT PLUGIN TRAIT
// ============================================================================

/// Trait for parsing and generating healthcare data formats
#[async_trait]
pub trait FormatPlugin: Send + Sync {
    /// Plugin name
    fn name(&self) -> &str;
    
    /// Supported data formats (as string codes)
    fn supported_formats(&self) -> Vec<String>;
    
    /// Parse raw data into structured format
    fn parse(&self, raw: &[u8], format: &str) -> Result<serde_json::Value>;
    
    /// Generate format from structured data
    fn generate(&self, data: &serde_json::Value, format: &str) -> Result<Vec<u8>>;
    
    /// Validate format compliance
    fn validate(&self, data: &[u8], format: &str) -> Result<ValidationResult>;
    
    /// Auto-detect format
    fn detect_format(&self, data: &[u8]) -> Option<String>;
    
    /// Normalize to standard format (FHIR R4)
    fn normalize(&self, data: &serde_json::Value, from_format: &str) -> Result<serde_json::Value>;
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ValidationResult {
    pub valid: bool,
    pub errors: Vec<String>,
    pub warnings: Vec<String>,
}

// ============================================================================
// STORAGE PLUGIN TRAIT
// ============================================================================

#[async_trait]
pub trait StoragePlugin: Send + Sync {
    /// Plugin name
    fn name(&self) -> &str;
    
    /// Save device data
    async fn save(&self, data: &serde_json::Value) -> Result<Uuid>;
    
    /// Retrieve device data
    async fn retrieve(&self, id: &Uuid) -> Result<serde_json::Value>;
    
    /// Query device data
    async fn query(&self, device_id: &Uuid, limit: Option<i64>) -> Result<Vec<serde_json::Value>>;
    
    /// Delete device data
    async fn delete(&self, id: &Uuid) -> Result<()>;
    
    /// Bulk save
    async fn bulk_save(&self, data: Vec<serde_json::Value>) -> Result<Vec<Uuid>>;
}

// ============================================================================
// TRANSFER PLUGIN TRAIT
// ============================================================================

#[async_trait]
pub trait TransferPlugin: Send + Sync {
    /// Plugin name
    fn name(&self) -> &str;
    
    /// Supported protocols
    fn protocol(&self) -> TransferProtocol;
    
    /// Send data to destination
    async fn send(&self, data: &[u8], destination: &Destination) -> Result<TransferReceipt>;
    
    /// Receive data from source
    async fn receive(&self, source: &Source) -> Result<Vec<u8>>;
    
    /// Check transfer status
    async fn status(&self, receipt: &TransferReceipt) -> Result<TransferStatus>;
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum TransferProtocol {
    Http,
    Mllp,
    Sftp,
    Websocket,
    Kafka,
    IheXds,
    Ndhm,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Destination {
    pub protocol: TransferProtocol,
    pub endpoint: String,
    pub credentials: Option<serde_json::Value>,
    pub metadata: Option<serde_json::Value>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Source {
    pub protocol: TransferProtocol,
    pub endpoint: String,
    pub credentials: Option<serde_json::Value>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TransferReceipt {
    pub id: Uuid,
    pub timestamp: DateTime<Utc>,
    pub destination: String,
    pub size: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum TransferStatus {
    Pending,
    InProgress,
    Completed,
    Failed(String),
}

