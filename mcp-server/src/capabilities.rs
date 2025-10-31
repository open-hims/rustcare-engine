//! MCP Capabilities registry
use crate::protocol::{Capability, CapabilityType};
use std::collections::HashMap;

/// Registry of MCP capabilities
pub struct CapabilitiesRegistry {
    capabilities: HashMap<String, Capability>,
}

impl CapabilitiesRegistry {
    /// Create a new capabilities registry
    pub fn new() -> Self {
        let mut registry = Self {
            capabilities: HashMap::new(),
        };
        
        // Register built-in RustCare capabilities
        registry.register_default_capabilities();
        
        registry
    }

    /// Register default RustCare capabilities
    fn register_default_capabilities(&mut self) {
        self.register(Capability {
            name: "patient_management".to_string(),
            description: "Query and manage patient records".to_string(),
            capability_type: CapabilityType::Tool,
        });
        
        self.register(Capability {
            name: "clinical_data".to_string(),
            description: "Access medical records, medications, allergies".to_string(),
            capability_type: CapabilityType::Tool,
        });
        
        self.register(Capability {
            name: "appointments".to_string(),
            description: "Schedule and manage appointments".to_string(),
            capability_type: CapabilityType::Tool,
        });
        
        self.register(Capability {
            name: "voice_dictation".to_string(),
            description: "Voice transcription and dictation".to_string(),
            capability_type: CapabilityType::Tool,
        });
        
        self.register(Capability {
            name: "notifications".to_string(),
            description: "Send and manage notifications".to_string(),
            capability_type: CapabilityType::Tool,
        });
        
        self.register(Capability {
            name: "pharmacy".to_string(),
            description: "Pharmacy and prescription management".to_string(),
            capability_type: CapabilityType::Tool,
        });
        
        self.register(Capability {
            name: "analytics".to_string(),
            description: "Generate reports and query metrics".to_string(),
            capability_type: CapabilityType::Tool,
        });
    }

    /// Register a new capability
    pub fn register(&mut self, capability: Capability) {
        self.capabilities.insert(capability.name.clone(), capability);
    }

    /// List all capabilities
    pub fn list(&self) -> Vec<&Capability> {
        self.capabilities.values().collect()
    }

    /// Get a specific capability
    pub fn get(&self, name: &str) -> Option<&Capability> {
        self.capabilities.get(name)
    }
}

impl Default for CapabilitiesRegistry {
    fn default() -> Self {
        Self::new()
    }
}

