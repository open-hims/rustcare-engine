//! Plugin security and permission management
//! 
//! Provides security controls and permission validation for
//! plugin execution in healthcare environments.

use std::collections::{HashMap, HashSet};
use serde::{Deserialize, Serialize};
use uuid::Uuid;
use chrono::{Datelike, Timelike};

/// Security manager for plugin permissions
pub struct SecurityManager {
    /// Security policies registry
    policies: HashMap<Uuid, SecurityPolicy>,
    /// Global security configuration
    config: SecurityConfig,
}

/// Security policy for a plugin
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SecurityPolicy {
    /// Policy ID
    pub id: Uuid,
    /// Plugin ID this policy applies to
    pub plugin_id: Uuid,
    /// Granted permissions
    pub permissions: HashSet<Permission>,
    /// Security constraints
    pub constraints: Vec<SecurityConstraint>,
    /// Trust level
    pub trust_level: TrustLevel,
    /// Policy metadata
    pub metadata: PolicyMetadata,
}

/// Security configuration
#[derive(Debug, Clone)]
pub struct SecurityConfig {
    /// Enforce strict HIPAA compliance
    pub hipaa_compliance_mode: bool,
    /// Default trust level for new plugins
    pub default_trust_level: TrustLevel,
    /// Enable signature verification
    pub signature_verification: bool,
    /// Audit all security events
    pub audit_security_events: bool,
}

/// Plugin permission types
#[derive(Debug, Clone, Hash, Eq, PartialEq, Serialize, Deserialize)]
pub enum Permission {
    /// Read patient data
    ReadPatientData {
        /// Allowed data types
        data_types: Vec<String>,
        /// Access level
        level: AccessLevel,
    },
    /// Write patient data
    WritePatientData {
        /// Allowed data types
        data_types: Vec<String>,
        /// Access level
        level: AccessLevel,
    },
    /// Access to audit logs
    AuditLogAccess {
        /// Read level
        level: AccessLevel,
    },
    /// Network access
    NetworkAccess {
        /// Allowed hosts
        allowed_hosts: Vec<String>,
        /// Allowed ports
        allowed_ports: Vec<u16>,
    },
    /// File system access
    FileSystemAccess {
        /// Allowed paths
        allowed_paths: Vec<String>,
        /// Access type
        access_type: FileAccessType,
    },
    /// Database access
    DatabaseAccess {
        /// Database types
        database_types: Vec<String>,
        /// Access level
        level: AccessLevel,
    },
    /// Email/notification sending
    NotificationSend {
        /// Notification types
        types: Vec<String>,
    },
    /// Integration API access
    IntegrationApiAccess {
        /// API endpoints
        endpoints: Vec<String>,
        /// Methods allowed
        methods: Vec<String>,
    },
}

/// Access level enumeration
#[derive(Debug, Clone, Hash, Eq, PartialEq, Serialize, Deserialize)]
pub enum AccessLevel {
    /// No access
    None,
    /// Read-only access
    ReadOnly,
    /// Read-write access
    ReadWrite,
    /// Administrative access
    Admin,
}

/// File access type
#[derive(Debug, Clone, Hash, Eq, PartialEq, Serialize, Deserialize)]
pub enum FileAccessType {
    /// Read-only access
    ReadOnly,
    /// Write-only access
    WriteOnly,
    /// Read-write access
    ReadWrite,
    /// Execute access
    Execute,
}

/// Security constraint
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum SecurityConstraint {
    /// Time-based access restriction
    TimeRestriction {
        /// Allowed hours (24-hour format)
        allowed_hours: Vec<u8>,
        /// Allowed days of week (0=Sunday)
        allowed_days: Vec<u8>,
    },
    /// IP address restriction
    IpRestriction {
        /// Allowed IP ranges
        allowed_ips: Vec<String>,
    },
    /// Rate limiting
    RateLimit {
        /// Max requests per time window
        max_requests: u32,
        /// Time window in seconds
        window_seconds: u64,
    },
    /// Data volume limit
    DataVolumeLimit {
        /// Max bytes per operation
        max_bytes_per_operation: usize,
        /// Max bytes per day
        max_bytes_per_day: usize,
    },
}

/// Trust level for plugins
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub enum TrustLevel {
    /// Untrusted - minimal permissions
    Untrusted,
    /// Basic trust - limited permissions
    Basic,
    /// Verified - standard permissions
    Verified,
    /// Trusted - extended permissions
    Trusted,
    /// FullyTrusted - all permissions
    FullyTrusted,
}

/// Policy metadata
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PolicyMetadata {
    /// Policy name
    pub name: String,
    /// Policy description
    pub description: String,
    /// Created timestamp
    pub created_at: chrono::DateTime<chrono::Utc>,
    /// Updated timestamp
    pub updated_at: chrono::DateTime<chrono::Utc>,
    /// Policy version
    pub version: String,
    /// Created by user
    pub created_by: String,
}

impl SecurityManager {
    /// Create a new security manager
    pub fn new(config: SecurityConfig) -> Self {
        Self {
            policies: HashMap::new(),
            config,
        }
    }
    
    /// Register a security policy for a plugin
    pub fn register_policy(&mut self, policy: SecurityPolicy) -> Result<(), crate::error::PluginRuntimeError> {
        // Validate policy
        self.validate_policy(&policy)?;
        
        self.policies.insert(policy.id, policy);
        Ok(())
    }
    
    /// Check if a plugin has permission for an operation
    pub fn check_permission(
        &self,
        plugin_id: Uuid,
        permission: &Permission,
    ) -> Result<bool, crate::error::PluginRuntimeError> {
        // Find policy for plugin
        let policy = self.policies
            .values()
            .find(|p| p.plugin_id == plugin_id)
            .ok_or_else(|| crate::error::PluginRuntimeError::SecurityViolation(
                "No security policy found for plugin".to_string()
            ))?;
        
        // Check if permission is granted
        let has_permission = policy.permissions.contains(permission);
        
        // Validate constraints if permission is granted
        if has_permission {
            self.validate_constraints(policy, permission)?;
        }
        
        Ok(has_permission)
    }
    
    /// Validate security constraints
    fn validate_constraints(
        &self,
        policy: &SecurityPolicy,
        _permission: &Permission,
    ) -> Result<(), crate::error::PluginRuntimeError> {
        let now = chrono::Utc::now();
        
        for constraint in &policy.constraints {
            match constraint {
                SecurityConstraint::TimeRestriction { allowed_hours, allowed_days } => {
                    let current_hour = now.hour() as u8;
                    let current_day = now.weekday().num_days_from_sunday() as u8;
                    
                    if !allowed_hours.contains(&current_hour) || !allowed_days.contains(&current_day) {
                        return Err(crate::error::PluginRuntimeError::SecurityViolation(
                            "Access not allowed at current time".to_string(),
                        ));
                    }
                }
                SecurityConstraint::RateLimit { max_requests: _, window_seconds: _ } => {
                    // Rate limiting validation would require state tracking
                    // Implementation omitted for brevity
                }
                _ => {
                    // Other constraint validations
                }
            }
        }
        
        Ok(())
    }
    
    /// Validate a security policy
    fn validate_policy(&self, policy: &SecurityPolicy) -> Result<(), crate::error::PluginRuntimeError> {
        // HIPAA compliance checks
        if self.config.hipaa_compliance_mode {
            // Check for overly permissive patient data access
            for permission in &policy.permissions {
                match permission {
                    Permission::ReadPatientData { level: AccessLevel::Admin, .. } |
                    Permission::WritePatientData { level: AccessLevel::Admin, .. } => {
                        if policy.trust_level < TrustLevel::Trusted {
                            return Err(crate::error::PluginRuntimeError::SecurityViolation(
                                "Admin-level patient data access requires trusted plugin".to_string(),
                            ));
                        }
                    }
                    _ => {}
                }
            }
        }
        
        Ok(())
    }
    
    /// Create a default security policy for a plugin
    pub fn create_default_policy(&self, plugin_id: Uuid, plugin_name: String) -> SecurityPolicy {
        SecurityPolicy {
            id: Uuid::new_v4(),
            plugin_id,
            permissions: HashSet::new(), // No permissions by default
            constraints: vec![
                SecurityConstraint::RateLimit {
                    max_requests: 100,
                    window_seconds: 60,
                },
            ],
            trust_level: self.config.default_trust_level.clone(),
            metadata: PolicyMetadata {
                name: format!("Default policy for {}", plugin_name),
                description: "Auto-generated default security policy".to_string(),
                created_at: chrono::Utc::now(),
                updated_at: chrono::Utc::now(),
                version: "1.0.0".to_string(),
                created_by: "system".to_string(),
            },
        }
    }
}

impl Default for SecurityConfig {
    fn default() -> Self {
        Self {
            hipaa_compliance_mode: true,
            default_trust_level: TrustLevel::Untrusted,
            signature_verification: true,
            audit_security_events: true,
        }
    }
}