use crate::{
    Device, DeviceData, DeviceCommand, DeviceConfig, DeviceError, Result,
    DeviceRepository, PluginRegistry,
};
use std::sync::Arc;
use tokio::sync::RwLock;
use uuid::Uuid;
use std::collections::HashMap;

/// Device manager - business logic layer
pub struct DeviceManager {
    repository: Arc<DeviceRepository>,
    registry: Arc<PluginRegistry>,
    active_connections: Arc<RwLock<HashMap<Uuid, ConnectionState>>>,
}

#[derive(Debug, Clone)]
struct ConnectionState {
    device_id: Uuid,
    status: String,
    connected_at: chrono::DateTime<chrono::Utc>,
}

impl DeviceManager {
    pub fn new(repository: Arc<DeviceRepository>, registry: Arc<PluginRegistry>) -> Self {
        Self {
            repository,
            registry,
            active_connections: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    // ========================================================================
    // DEVICE MANAGEMENT
    // ========================================================================

    pub async fn register_device(
        &self,
        name: String,
        device_type: String,
        manufacturer: String,
        model: String,
        serial_number: String,
        location: serde_json::Value,
        config: DeviceConfig,
        user_id: Option<Uuid>,
    ) -> Result<Device> {
        // Validate device plugin exists for this type
        self.registry.get_device_plugin(&device_type).await?;

        let mut device = Device::new(
            name,
            device_type,
            manufacturer,
            model,
            serial_number,
            location,
            config,
        );
        device.created_by = user_id;
        device.updated_by = user_id;

        self.repository.create_device(&device).await
    }

    pub async fn get_device(&self, id: Uuid) -> Result<Device> {
        self.repository.get_device(id).await
    }

    pub async fn list_devices(
        &self,
        device_type: Option<String>,
        status: Option<String>,
        location_filter: Option<serde_json::Value>,
        page: i64,
        page_size: i64,
    ) -> Result<Vec<Device>> {
        let offset = (page - 1) * page_size;
        self.repository
            .list_devices(device_type, status, location_filter, page_size, offset)
            .await
    }

    pub async fn update_device(
        &self,
        id: Uuid,
        device: Device,
    ) -> Result<Device> {
        self.repository.update_device(id, &device).await
    }

    pub async fn delete_device(&self, id: Uuid) -> Result<()> {
        // Disconnect if connected
        if self.active_connections.read().await.contains_key(&id) {
            self.disconnect_device(id).await?;
        }

        self.repository.delete_device(id).await
    }

    // ========================================================================
    // CONNECTION MANAGEMENT
    // ========================================================================

    pub async fn connect_device(&self, id: Uuid) -> Result<()> {
        let device = self.repository.get_device(id).await?;
        
        // Check if already connected
        if self.active_connections.read().await.contains_key(&id) {
            return Err(DeviceError::Busy(format!("Device {} already connected", id)));
        }

        // Get plugin for this device type
        let plugin = self.registry.get_device_plugin(&device.device_type).await?;

        // Update status to connecting
        self.repository.update_device_status(id, "connecting".to_string(), None).await?;

        // Get device config
        let config = device.get_config()?;
        let config_json = serde_json::to_value(config)?;

        // Attempt connection
        match plugin.connect(&id.to_string(), config_json).await {
            Ok(_) => {
                // Store connection state
                let mut connections = self.active_connections.write().await;
                connections.insert(id, ConnectionState {
                    device_id: id,
                    status: "connected".to_string(),
                    connected_at: chrono::Utc::now(),
                });

                // Update device status
                self.repository.update_device_status(id, "connected".to_string(), None).await?;
                Ok(())
            }
            Err(e) => {
                let error_msg = e.to_string();
                self.repository.update_device_status(id, "error".to_string(), Some(error_msg.clone())).await?;
                Err(DeviceError::ConnectionError(error_msg))
            }
        }
    }

    pub async fn disconnect_device(&self, id: Uuid) -> Result<()> {
        let device = self.repository.get_device(id).await?;
        let plugin = self.registry.get_device_plugin(&device.device_type).await?;

        // Disconnect
        plugin.disconnect(&id.to_string()).await?;

        // Remove connection state
        self.active_connections.write().await.remove(&id);

        // Update status
        self.repository.update_device_status(id, "disconnected".to_string(), None).await?;

        Ok(())
    }

    pub async fn is_connected(&self, id: Uuid) -> bool {
        self.active_connections.read().await.contains_key(&id)
    }

    // ========================================================================
    // DATA OPERATIONS
    // ========================================================================

    pub async fn read_device_data(&self, id: Uuid) -> Result<DeviceData> {
        let device = self.repository.get_device(id).await?;
        
        if !self.is_connected(id).await {
            return Err(DeviceError::ConnectionError("Device not connected".to_string()));
        }

        let plugin = self.registry.get_device_plugin(&device.device_type).await?;
        
        // Read from device
        let raw_data = plugin.read(&id.to_string()).await?;
        
        // Create device data record
        let device_data = DeviceData {
            id: Uuid::new_v4(),
            device_id: id,
            timestamp: chrono::Utc::now(),
            data_type: "reading".to_string(),
            format: device.get_config()?.protocol.format.clone(),
            raw_data,
            parsed_data: None,
            normalized_data: None,
            patient_id: None,
            encounter_id: None,
            provider_id: None,
            metadata: serde_json::json!({}),
            created_at: chrono::Utc::now(),
        };

        // Save to database
        self.repository.save_device_data(&device_data).await
    }

    pub async fn get_device_data_history(
        &self,
        device_id: Uuid,
        start_time: Option<chrono::DateTime<chrono::Utc>>,
        end_time: Option<chrono::DateTime<chrono::Utc>>,
        data_type: Option<String>,
        limit: i64,
    ) -> Result<Vec<DeviceData>> {
        self.repository
            .get_device_data(device_id, start_time, end_time, data_type, limit)
            .await
    }

    // ========================================================================
    // COMMAND EXECUTION
    // ========================================================================

    pub async fn send_command(
        &self,
        device_id: Uuid,
        command: String,
        parameters: serde_json::Value,
    ) -> Result<DeviceCommand> {
        let device = self.repository.get_device(device_id).await?;
        
        if !self.is_connected(device_id).await {
            return Err(DeviceError::ConnectionError("Device not connected".to_string()));
        }

        // Create command record
        let mut device_command = DeviceCommand {
            id: Uuid::new_v4(),
            device_id,
            command: command.clone(),
            parameters: parameters.clone(),
            metadata: serde_json::json!({}),
            created_at: chrono::Utc::now(),
            executed_at: None,
            completed_at: None,
            status: "pending".to_string(),
            response: None,
            error: None,
        };

        // Save command
        device_command = self.repository.save_device_command(&device_command).await?;

        // Execute via plugin
        let plugin = self.registry.get_device_plugin(&device.device_type).await?;
        
        // Update to executing
        device_command = self.repository
            .update_command_status(device_command.id, "executing".to_string(), None, None)
            .await?;

        // Execute command
        match plugin.write(&device_id.to_string(), parameters).await {
            Ok(response) => {
                self.repository
                    .update_command_status(
                        device_command.id,
                        "completed".to_string(),
                        Some(response),
                        None,
                    )
                    .await
            }
            Err(e) => {
                self.repository
                    .update_command_status(
                        device_command.id,
                        "failed".to_string(),
                        None,
                        Some(e.to_string()),
                    )
                    .await
            }
        }
    }

    pub async fn get_device_commands(
        &self,
        device_id: Uuid,
        status: Option<String>,
        limit: i64,
    ) -> Result<Vec<DeviceCommand>> {
        self.repository.get_device_commands(device_id, status, limit).await
    }
}
