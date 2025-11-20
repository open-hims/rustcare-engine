# Example UI Plugin

A WASM plugin example that demonstrates:
- Registering UI components with RustCare
- Providing custom health calculation functions
- Integrating with the plugin runtime

## Features

### UI Components Registered
1. **PatientDashboardWidget** - Custom dashboard widget
2. **HealthMetricsChart** - Health metrics visualization
3. **QuickActionButton** - Quick action button

### Functions Provided
1. **calculate_bmi** - Calculate Body Mass Index
2. **format_health_data** - Format health metrics for display
3. **validate_patient_data** - Validate patient data

## Building

```bash
# Install wasm32-wasi target
rustup target add wasm32-wasi

# Build the plugin
cd plugins/example-ui-plugin
cargo build --target wasm32-wasi --release

# The output will be in:
# target/wasm32-wasi/release/example_ui_plugin.wasm
```

## Installation

1. Copy the built WASM file to the plugin directory:
```bash
cp target/wasm32-wasi/release/example_ui_plugin.wasm /path/to/rustcare/plugins/
```

2. Install via API:
```bash
curl -X POST http://localhost:8080/api/v1/plugins \
  -H "Content-Type: application/json" \
  -H "Authorization: Bearer YOUR_TOKEN" \
  -d '{
    "name": "example-ui-plugin",
    "version": "0.1.0",
    "description": "Example WASM plugin",
    "author": "RustCare Team",
    "api_version": "1.0.0",
    "entry_point": "example_ui_plugin.wasm",
    "plugin_type": "wasm"
  }'
```

3. Load the plugin:
```bash
curl -X POST http://localhost:8080/api/v1/plugins/{plugin_id}/load \
  -H "Authorization: Bearer YOUR_TOKEN"
```

## Usage

### Execute BMI Calculation
```bash
curl -X POST http://localhost:8080/api/v1/plugins/{plugin_id}/execute \
  -H "Content-Type: application/json" \
  -H "Authorization: Bearer YOUR_TOKEN" \
  -d '{
    "function_name": "calculate_bmi",
    "input_data": {
      "weight_kg": 70.0,
      "height_m": 1.75
    }
  }'
```

### Format Health Data
```bash
curl -X POST http://localhost:8080/api/v1/plugins/{plugin_id}/execute \
  -H "Content-Type: application/json" \
  -H "Authorization: Bearer YOUR_TOKEN" \
  -d '{
    "function_name": "format_health_data",
    "input_data": {
      "systolic": 120,
      "diastolic": 80,
      "heart_rate": 72,
      "temperature": 36.5
    }
  }'
```

## Integration with UI

After installation, the plugin automatically registers UI components that can be used in the frontend. The UI components will appear in the component registry and can be dynamically loaded.

