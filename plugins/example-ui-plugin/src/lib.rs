//! Example UI Plugin - WASM Plugin that registers UI components
//!
//! This plugin demonstrates how to create a WASM plugin that:
//! - Registers UI components with the RustCare system
//! - Provides custom functionality via WASM
//! - Integrates with the plugin runtime
//!
//! Note: This uses WASM32-WASI target
//! Functions return JSON strings that can be called from the plugin runtime

use serde::{Deserialize, Serialize};

/// Plugin metadata
#[derive(Debug, Serialize, Deserialize)]
pub struct PluginMetadata {
    pub id: String,
    pub name: String,
    pub version: String,
    pub description: String,
    pub author: String,
}

/// UI Component registration data
#[derive(Debug, Serialize, Deserialize)]
pub struct UIComponent {
    pub component_name: String,
    pub component_path: String,
    pub component_type: String,
    pub display_name: String,
    pub description: String,
    pub route_path: Option<String>,
    pub category: String,
    pub icon: Option<String>,
    pub tags: Vec<String>,
}

/// Plugin execution result
#[derive(Debug, Serialize, Deserialize)]
pub struct PluginResult {
    pub success: bool,
    pub message: String,
    pub data: serde_json::Value,
}

/// Initialize the plugin
/// Returns JSON string with plugin metadata
#[no_mangle]
pub extern "C" fn plugin_init() -> i32 {
    // Return success (actual metadata would be retrieved via another function)
    0
}

/// Get plugin metadata as JSON string
/// Returns a pointer to a JSON string (caller manages memory)
#[no_mangle]
pub extern "C" fn plugin_get_metadata() -> *const u8 {
    let metadata = PluginMetadata {
        id: "example-ui-plugin".to_string(),
        name: "Example UI Plugin".to_string(),
        version: "0.1.0".to_string(),
        description: "Example WASM plugin that registers UI components".to_string(),
        author: "RustCare Team".to_string(),
    };
    
    let json = serde_json::to_string(&metadata).unwrap_or_default();
    let boxed = json.into_boxed_str();
    Box::into_raw(boxed) as *const u8
}

/// Register UI components
/// Returns JSON array of UI components to register
#[no_mangle]
pub extern "C" fn plugin_register_ui_components() -> *const u8 {
    let components = vec![
        UIComponent {
            component_name: "PatientDashboardWidget".to_string(),
            component_path: "/plugins/example-ui-plugin/PatientDashboardWidget".to_string(),
            component_type: "widget".to_string(),
            display_name: "Patient Dashboard Widget".to_string(),
            description: "Custom patient dashboard widget plugin".to_string(),
            route_path: Some("/dashboard/patient-widget".to_string()),
            category: "dashboard".to_string(),
            icon: Some("Users".to_string()),
            tags: vec!["patient".to_string(), "dashboard".to_string(), "widget".to_string()],
        },
        UIComponent {
            component_name: "HealthMetricsChart".to_string(),
            component_path: "/plugins/example-ui-plugin/HealthMetricsChart".to_string(),
            component_type: "component".to_string(),
            display_name: "Health Metrics Chart".to_string(),
            description: "Interactive health metrics visualization".to_string(),
            route_path: None,
            category: "analytics".to_string(),
            icon: Some("Activity".to_string()),
            tags: vec!["health".to_string(), "metrics".to_string(), "chart".to_string()],
        },
        UIComponent {
            component_name: "QuickActionButton".to_string(),
            component_path: "/plugins/example-ui-plugin/QuickActionButton".to_string(),
            component_type: "button".to_string(),
            display_name: "Quick Action".to_string(),
            description: "Quick action button for common tasks".to_string(),
            route_path: None,
            category: "actions".to_string(),
            icon: Some("Zap".to_string()),
            tags: vec!["action".to_string(), "quick".to_string()],
        },
    ];
    
    let json = serde_json::to_string(&components).unwrap_or_default();
    let boxed = json.into_boxed_str();
    Box::into_raw(boxed) as *const u8
}

/// Execute plugin function
/// This is called by the plugin runtime when executing the plugin
/// Parameters are passed as JSON strings
#[no_mangle]
pub extern "C" fn plugin_execute(function_name_ptr: *const u8, function_name_len: usize, input_data_ptr: *const u8, input_data_len: usize) -> *const u8 {
    // Convert C strings to Rust strings
    let function_name = unsafe {
        std::str::from_utf8(std::slice::from_raw_parts(function_name_ptr, function_name_len))
            .unwrap_or_default()
    };
    let input_data = unsafe {
        std::str::from_utf8(std::slice::from_raw_parts(input_data_ptr, input_data_len))
            .unwrap_or_default()
    };
    
    let result = execute_internal(function_name, input_data);
    
    // Return result as C string (caller must free)
    let boxed = result.into_boxed_str();
    Box::into_raw(boxed) as *const u8
}

/// Internal execute function
fn execute_internal(function_name: &str, input_data: &str) -> String {
    match function_name {
        "calculate_bmi" => calculate_bmi(input_data),
        "format_health_data" => format_health_data(input_data),
        "validate_patient_data" => validate_patient_data(input_data),
        _ => {
            let result = PluginResult {
                success: false,
                message: format!("Unknown function: {}", function_name),
                data: serde_json::json!({}),
            };
            serde_json::to_string(&result).unwrap_or_default()
        }
    }
}

/// Calculate BMI (Body Mass Index)
fn calculate_bmi(input: &str) -> String {
    #[derive(Deserialize)]
    struct BMIInput {
        weight_kg: f64,
        height_m: f64,
    }
    
    match serde_json::from_str::<BMIInput>(input) {
        Ok(data) => {
            let bmi = data.weight_kg / (data.height_m * data.height_m);
            let category = match bmi {
                b if b < 18.5 => "Underweight",
                b if b < 25.0 => "Normal",
                b if b < 30.0 => "Overweight",
                _ => "Obese",
            };
            
            let result = PluginResult {
                success: true,
                message: "BMI calculated successfully".to_string(),
                data: serde_json::json!({
                    "bmi": bmi,
                    "category": category,
                    "weight_kg": data.weight_kg,
                    "height_m": data.height_m,
                }),
            };
            serde_json::to_string(&result).unwrap_or_default()
        }
        Err(e) => {
            let result = PluginResult {
                success: false,
                message: format!("Invalid input: {}", e),
                data: serde_json::json!({}),
            };
            serde_json::to_string(&result).unwrap_or_default()
        }
    }
}

/// Format health data for display
fn format_health_data(input: &str) -> String {
    #[derive(Deserialize)]
    struct HealthData {
        systolic: Option<u32>,
        diastolic: Option<u32>,
        heart_rate: Option<u32>,
        temperature: Option<f64>,
    }
    
    match serde_json::from_str::<HealthData>(input) {
        Ok(data) => {
            let mut formatted = serde_json::Map::new();
            
            if let Some(s) = data.systolic {
                if let Some(d) = data.diastolic {
                    formatted.insert("blood_pressure".to_string(), 
                        serde_json::json!(format!("{}/{} mmHg", s, d)));
                    
                    // Add category
                    let bp_category = match (s, d) {
                        (s, d) if s < 120 && d < 80 => "Normal",
                        (s, d) if s < 130 && d < 80 => "Elevated",
                        (s, d) if s < 140 || d < 90 => "High Stage 1",
                        _ => "High Stage 2",
                    };
                    formatted.insert("bp_category".to_string(), serde_json::json!(bp_category));
                }
            }
            
            if let Some(hr) = data.heart_rate {
                formatted.insert("heart_rate".to_string(), 
                    serde_json::json!(format!("{} bpm", hr)));
                
                let hr_status = match hr {
                    hr if hr < 60 => "Bradycardia",
                    hr if hr <= 100 => "Normal",
                    _ => "Tachycardia",
                };
                formatted.insert("hr_status".to_string(), serde_json::json!(hr_status));
            }
            
            if let Some(temp) = data.temperature {
                formatted.insert("temperature".to_string(), 
                    serde_json::json!(format!("{:.1}°C", temp)));
                
                let temp_status = match temp {
                    t if t < 36.1 => "Hypothermia",
                    t if t <= 37.2 => "Normal",
                    t if t <= 38.0 => "Low-grade fever",
                    _ => "Fever",
                };
                formatted.insert("temp_status".to_string(), serde_json::json!(temp_status));
            }
            
            let result = PluginResult {
                success: true,
                message: "Health data formatted successfully".to_string(),
                data: serde_json::Value::Object(formatted),
            };
            serde_json::to_string(&result).unwrap_or_default()
        }
        Err(e) => {
            let result = PluginResult {
                success: false,
                message: format!("Invalid input: {}", e),
                data: serde_json::json!({}),
            };
            serde_json::to_string(&result).unwrap_or_default()
        }
    }
}

/// Validate patient data
fn validate_patient_data(input: &str) -> String {
    #[derive(Deserialize)]
    struct PatientData {
        age: Option<u32>,
        weight_kg: Option<f64>,
        height_cm: Option<f64>,
        email: Option<String>,
    }
    
    match serde_json::from_str::<PatientData>(input) {
        Ok(data) => {
            let mut errors = Vec::new();
            let mut warnings = Vec::new();
            
            if let Some(age) = data.age {
                if age > 150 {
                    errors.push("Age seems unrealistic (>150 years)");
                } else if age > 120 {
                    warnings.push("Age is unusually high");
                }
            }
            
            if let Some(weight) = data.weight_kg {
                if weight <= 0.0 {
                    errors.push("Weight must be positive");
                } else if weight > 500.0 {
                    errors.push("Weight seems unrealistic (>500 kg)");
                } else if weight > 200.0 {
                    warnings.push("Weight is unusually high");
                }
            }
            
            if let Some(height) = data.height_cm {
                if height <= 0.0 {
                    errors.push("Height must be positive");
                } else if height > 300.0 {
                    errors.push("Height seems unrealistic (>300 cm)");
                } else if height > 250.0 {
                    warnings.push("Height is unusually high");
                }
            }
            
            if let Some(ref email) = data.email {
                if !email.contains('@') {
                    errors.push("Invalid email format");
                }
            }
            
            let result = PluginResult {
                success: errors.is_empty(),
                message: if errors.is_empty() {
                    "Patient data is valid".to_string()
                } else {
                    format!("Validation failed: {} error(s)", errors.len())
                },
                data: serde_json::json!({
                    "valid": errors.is_empty(),
                    "errors": errors,
                    "warnings": warnings,
                }),
            };
            serde_json::to_string(&result).unwrap_or_default()
        }
        Err(e) => {
            let result = PluginResult {
                success: false,
                message: format!("Invalid input: {}", e),
                data: serde_json::json!({}),
            };
            serde_json::to_string(&result).unwrap_or_default()
        }
    }
}

/// Health check function
#[no_mangle]
pub extern "C" fn plugin_health_check() -> *const u8 {
    let result = PluginResult {
        success: true,
        message: "Plugin is healthy".to_string(),
        data: serde_json::json!({
            "status": "healthy",
            "version": "0.1.0",
            "functions": ["calculate_bmi", "format_health_data", "validate_patient_data"],
        }),
    };
    let json = serde_json::to_string(&result).unwrap_or_default();
    let boxed = json.into_boxed_str();
    Box::into_raw(boxed) as *const u8
}
