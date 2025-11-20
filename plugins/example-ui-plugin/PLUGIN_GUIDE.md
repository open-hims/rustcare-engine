# WASM Plugin Integration Guide

This guide explains how to create and integrate WASM plugins with UI components in RustCare.

## Architecture Overview

```
┌─────────────────┐
│  WASM Plugin    │
│  (example-ui)   │
└────────┬────────┘
         │
         │ Registers UI Components
         ▼
┌─────────────────┐
│  Plugin Runtime │
│  (Lifecycle)    │
└────────┬────────┘
         │
         │ Manages Plugin State
         ▼
┌─────────────────┐
│  Plugin Handler │
│  (API)          │
└────────┬────────┘
         │
         │ Exposes via REST API
         ▼
┌─────────────────┐
│  UI Components  │
│  Registry       │
└────────┬────────┘
         │
         │ Available to Frontend
         ▼
┌─────────────────┐
│  React/Remix UI │
│  (Frontend)     │
└─────────────────┘
```

## Plugin Structure

### 1. WASM Plugin Code (`src/lib.rs`)

The plugin exports functions that:
- Register UI components
- Provide business logic functions
- Handle health checks

Key functions:
- `init()` - Initialize plugin and return metadata
- `register_ui_components()` - Return JSON array of UI components
- `execute(function_name, input_data)` - Execute plugin functions
- `health_check()` - Plugin health status

### 2. Plugin Manifest (`manifest.toml`)

Describes:
- Plugin metadata (id, name, version, etc.)
- Required permissions
- UI components to register
- Exposed functions
- Security settings

### 3. Build Process

```bash
# Install WASM target
rustup target add wasm32-wasi

# Build plugin
cargo build --target wasm32-wasi --release

# Output: target/wasm32-wasi/release/example_ui_plugin.wasm
```

## Integration Flow

### Step 1: Install Plugin

```bash
POST /api/v1/plugins
{
  "name": "example-ui-plugin",
  "version": "0.1.0",
  "description": "Example WASM plugin",
  "author": "RustCare Team",
  "api_version": "1.0.0",
  "entry_point": "example_ui_plugin.wasm",
  "plugin_type": "wasm"
}
```

### Step 2: Load Plugin

```bash
POST /api/v1/plugins/{plugin_id}/load
```

### Step 3: Register UI Components

When the plugin loads, it can register UI components:

```bash
POST /api/v1/ui/components/register
{
  "component_name": "PatientDashboardWidget",
  "component_path": "/plugins/example-ui-plugin/PatientDashboardWidget",
  "component_type": "widget",
  "display_name": "Patient Dashboard Widget",
  "category": "dashboard",
  "icon": "Users"
}
```

### Step 4: Execute Plugin Functions

```bash
POST /api/v1/plugins/{plugin_id}/execute
{
  "function_name": "calculate_bmi",
  "input_data": {
    "weight_kg": 70.0,
    "height_m": 1.75
  }
}
```

## UI Component Usage

Once registered, UI components can be used in React/Remix:

```tsx
import { PatientDashboardWidget } from '/plugins/example-ui-plugin/PatientDashboardWidget';

export default function Dashboard() {
  return (
    <div>
      <PatientDashboardWidget />
    </div>
  );
}
```

## Plugin Functions

### calculate_bmi
Calculates Body Mass Index from weight and height.

**Input:**
```json
{
  "weight_kg": 70.0,
  "height_m": 1.75
}
```

**Output:**
```json
{
  "success": true,
  "message": "BMI calculated successfully",
  "data": {
    "bmi": 22.86,
    "category": "Normal",
    "weight_kg": 70.0,
    "height_m": 1.75
  }
}
```

### format_health_data
Formats health metrics for display with status indicators.

**Input:**
```json
{
  "systolic": 120,
  "diastolic": 80,
  "heart_rate": 72,
  "temperature": 36.5
}
```

**Output:**
```json
{
  "success": true,
  "data": {
    "blood_pressure": "120/80 mmHg",
    "bp_category": "Normal",
    "heart_rate": "72 bpm",
    "hr_status": "Normal",
    "temperature": "36.5°C",
    "temp_status": "Normal"
  }
}
```

### validate_patient_data
Validates patient data and returns errors/warnings.

**Input:**
```json
{
  "age": 35,
  "weight_kg": 70.0,
  "height_cm": 175.0,
  "email": "patient@example.com"
}
```

**Output:**
```json
{
  "success": true,
  "message": "Patient data is valid",
  "data": {
    "valid": true,
    "errors": [],
    "warnings": []
  }
}
```

## Security Considerations

- Plugins run in WASM sandbox for isolation
- Signature verification required for production
- Resource limits enforced (memory, CPU time)
- Permission-based access control
- Audit logging for all plugin operations

## Development Tips

1. **Testing**: Test plugin functions independently before integration
2. **Error Handling**: Always handle plugin execution errors gracefully
3. **UI Components**: Keep UI components simple and reusable
4. **Documentation**: Document all plugin functions and UI components
5. **Versioning**: Use semantic versioning for plugin updates

## Next Steps

1. Implement actual WASM loading in plugin runtime
2. Add plugin hot-reloading support
3. Create plugin marketplace integration
4. Add plugin dependency management
5. Implement plugin update mechanism

