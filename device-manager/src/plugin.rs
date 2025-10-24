use async_trait::async_trait;
use uuid::Uuid;
use crate::types::*;
use crate::error::DeviceError;

// ============================================================================
// DEVICE PLUGIN TRAIT
// ============================================================================

/// Core trait that all device plugins must implement
#[async_trait]
pub trait DevicePlugin: Send + Sync {
    /// Plugin name
    fn name(&self) -> &str;
    
    /// Supported device types
    fn supported_types(&self) -> Vec<DeviceType>;
    
    /// Connect to the device
    async fn connect(&mut self, config: &DeviceConfig) -> Result<(), DeviceError>;
    
    /// Disconnect from the device
    async fn disconnect(&mut self) -> Result<(), DeviceError>;
    
    /// Check device status
    fn status(&self) -> DeviceStatus;
    
    /// Read data from device
    async fn read(&mut self) -> Result<DeviceData, DeviceError>;
    
    /// Write command to device
    async fn write(&mut self, command: &DeviceCommand) -> Result<serde_json::Value, DeviceError>;
    
    /// Subscribe to real-time data stream
    async fn subscribe<F>(&mut self, callback: F) -> Result<(), DeviceError>
    where
        F: Fn(DeviceData) -> () + Send + Sync + 'static;
    
    /// Validate device configuration
    fn validate_config(&self, config: &DeviceConfig) -> Result<(), DeviceError>;
    
    /// Test connection without actually connecting
    async fn test_connection(&self, config: &DeviceConfig) -> Result<bool, DeviceError>;
}

// ============================================================================
// FORMAT PLUGIN TRAIT
// ============================================================================

/// Trait for parsing and generating healthcare data formats
#[async_trait]
pub trait FormatPlugin: Send + Sync {
    /// Plugin name
    fn name(&self) -> &str;
    
    /// Supported data formats
    fn supported_formats(&self) -> Vec<DataFormat>;
    
    /// Parse raw data into structured format
    fn parse(&self, raw: &[u8], format: DataFormat) -> Result<serde_json::Value, DeviceError>;
    
    /// Generate format from structured data
    fn generate(&self, data: &serde_json::Value, format: DataFormat) -> Result<Vec<u8>, DeviceError>;
    
    /// Validate format compliance
    fn validate(&self, data: &[u8], format: DataFormat) -> Result<ValidationResult, DeviceError>;
    
    /// Auto-detect format
    fn detect_format(&self, data: &[u8]) -> Option<DataFormat>;
    
    /// Normalize to standard format (FHIR R4)
    fn normalize(&self, data: &serde_json::Value, from_format: DataFormat) -> Result<serde_json::Value, DeviceError>;
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
    async fn save(&self, data: &DeviceData) -> Result<Uuid, DeviceError>;
    
    /// Retrieve device data
    async fn retrieve(&self, id: &Uuid) -> Result<DeviceData, DeviceError>;
    
    /// Query device data
    async fn query(&self, device_id: &Uuid, limit: Option<i64>) -> Result<Vec<DeviceData>, DeviceError>;
    
    /// Delete device data
    async fn delete(&self, id: &Uuid) -> Result<(), DeviceError>;
    
    /// Bulk save
    async fn bulk_save(&self, data: Vec<DeviceData>) -> Result<Vec<Uuid>, DeviceError>;
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
    async fn send(&self, data: &[u8], destination: &Destination) -> Result<TransferReceipt, DeviceError>;
    
    /// Receive data from source
    async fn receive(&self, source: &Source) -> Result<Vec<u8>, DeviceError>;
    
    /// Check transfer status
    async fn status(&self, receipt: &TransferReceipt) -> Result<TransferStatus, DeviceError>;
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

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
