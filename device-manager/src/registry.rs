use crate::{DevicePlugin, FormatPlugin, StoragePlugin, TransferPlugin, DeviceError, Result};
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;

/// Plugin registry for managing all plugins
pub struct PluginRegistry {
    device_plugins: Arc<RwLock<HashMap<String, Arc<dyn DevicePlugin>>>>,
    format_plugins: Arc<RwLock<HashMap<String, Arc<dyn FormatPlugin>>>>,
    storage_plugins: Arc<RwLock<HashMap<String, Arc<dyn StoragePlugin>>>>,
    transfer_plugins: Arc<RwLock<HashMap<String, Arc<dyn TransferPlugin>>>>,
}

impl PluginRegistry {
    pub fn new() -> Self {
        Self {
            device_plugins: Arc::new(RwLock::new(HashMap::new())),
            format_plugins: Arc::new(RwLock::new(HashMap::new())),
            storage_plugins: Arc::new(RwLock::new(HashMap::new())),
            transfer_plugins: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    // ========================================================================
    // DEVICE PLUGINS
    // ========================================================================

    pub async fn register_device_plugin(
        &self,
        device_type: String,
        plugin: Arc<dyn DevicePlugin>,
    ) -> Result<()> {
        let mut plugins = self.device_plugins.write().await;
        plugins.insert(device_type, plugin);
        Ok(())
    }

    pub async fn get_device_plugin(&self, device_type: &str) -> Result<Arc<dyn DevicePlugin>> {
        let plugins = self.device_plugins.read().await;
        plugins
            .get(device_type)
            .cloned()
            .ok_or_else(|| DeviceError::NotFound(format!("No plugin found for device type: {}", device_type)))
    }

    pub async fn list_device_plugins(&self) -> Vec<String> {
        self.device_plugins.read().await.keys().cloned().collect()
    }

    // ========================================================================
    // FORMAT PLUGINS
    // ========================================================================

    pub async fn register_format_plugin(
        &self,
        format_code: String,
        plugin: Arc<dyn FormatPlugin>,
    ) -> Result<()> {
        let mut plugins = self.format_plugins.write().await;
        plugins.insert(format_code, plugin);
        Ok(())
    }

    pub async fn get_format_plugin(&self, format_code: &str) -> Result<Arc<dyn FormatPlugin>> {
        let plugins = self.format_plugins.read().await;
        plugins
            .get(format_code)
            .cloned()
            .ok_or_else(|| DeviceError::NotFound(format!("No plugin found for format: {}", format_code)))
    }

    pub async fn list_format_plugins(&self) -> Vec<String> {
        self.format_plugins.read().await.keys().cloned().collect()
    }

    // ========================================================================
    // STORAGE PLUGINS
    // ========================================================================

    pub async fn register_storage_plugin(
        &self,
        storage_type: String,
        plugin: Arc<dyn StoragePlugin>,
    ) -> Result<()> {
        let mut plugins = self.storage_plugins.write().await;
        plugins.insert(storage_type, plugin);
        Ok(())
    }

    pub async fn get_storage_plugin(&self, storage_type: &str) -> Result<Arc<dyn StoragePlugin>> {
        let plugins = self.storage_plugins.read().await;
        plugins
            .get(storage_type)
            .cloned()
            .ok_or_else(|| DeviceError::NotFound(format!("No plugin found for storage: {}", storage_type)))
    }

    pub async fn list_storage_plugins(&self) -> Vec<String> {
        self.storage_plugins.read().await.keys().cloned().collect()
    }

    // ========================================================================
    // TRANSFER PLUGINS
    // ========================================================================

    pub async fn register_transfer_plugin(
        &self,
        protocol: String,
        plugin: Arc<dyn TransferPlugin>,
    ) -> Result<()> {
        let mut plugins = self.transfer_plugins.write().await;
        plugins.insert(protocol, plugin);
        Ok(())
    }

    pub async fn get_transfer_plugin(&self, protocol: &str) -> Result<Arc<dyn TransferPlugin>> {
        let plugins = self.transfer_plugins.read().await;
        plugins
            .get(protocol)
            .cloned()
            .ok_or_else(|| DeviceError::NotFound(format!("No plugin found for protocol: {}", protocol)))
    }

    pub async fn list_transfer_plugins(&self) -> Vec<String> {
        self.transfer_plugins.read().await.keys().cloned().collect()
    }
}

impl Default for PluginRegistry {
    fn default() -> Self {
        Self::new()
    }
}
