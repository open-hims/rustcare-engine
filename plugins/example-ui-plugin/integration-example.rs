//! Integration example showing how to use the WASM plugin
//!
//! This demonstrates how to:
//! 1. Load the WASM plugin
//! 2. Register UI components from the plugin
//! 3. Execute plugin functions

use plugin_runtime_core::{
    api::{PluginInfo, ApiInput, ApiOutput, ExecutionContext, PluginConfig},
    lifecycle::LifecycleManager,
};
use std::collections::HashMap;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Initialize plugin runtime
    let config = plugin_runtime_core::lifecycle::LifecycleConfig {
        max_concurrent_plugins: 10,
        lifecycle_timeout_seconds: 30,
        auto_cleanup_enabled: true,
        cleanup_interval_seconds: 300,
    };
    let runtime = LifecycleManager::new(config);

    // Create plugin info
    let plugin_info = PluginInfo {
        id: uuid::Uuid::parse_str("550e8400-e29b-41d4-a716-446655440000")?,
        name: "example-ui-plugin".to_string(),
        version: "0.1.0".to_string(),
        description: "Example WASM plugin".to_string(),
        author: "RustCare Team".to_string(),
        api_version: "1.0.0".to_string(),
    };

    // TODO: Load actual WASM module
    // For now, this is a placeholder showing the integration pattern
    println!("Plugin integration example:");
    println!("1. Load WASM module from file");
    println!("2. Create plugin instance");
    println!("3. Install plugin");
    println!("4. Register UI components");
    println!("5. Execute plugin functions");

    // Example: Register UI components
    // In production, this would be called from the plugin's register_ui_components function
    println!("\nUI Components to register:");
    println!("- PatientDashboardWidget (widget)");
    println!("- HealthMetricsChart (component)");
    println!("- QuickActionButton (button)");

    // Example: Execute plugin function
    println!("\nExample: Calculate BMI");
    let bmi_input = serde_json::json!({
        "weight_kg": 70.0,
        "height_m": 1.75
    });

    println!("Input: {:?}", bmi_input);
    println!("Expected output: BMI = 22.86, Category = Normal");

    Ok(())
}

