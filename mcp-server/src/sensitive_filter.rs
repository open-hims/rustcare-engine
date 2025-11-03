//! Sensitive endpoint filter for MCP tools
//!
//! Defines which endpoints should be excluded from public LLM access
//! due to security, privacy, or compliance concerns.

use std::collections::HashSet;

/// Check if an endpoint path is sensitive and should be excluded
pub fn is_sensitive_endpoint(path: &str, method: &str) -> bool {
    let sensitive_patterns = get_sensitive_patterns();
    
    for pattern in sensitive_patterns {
        if path.contains(pattern) {
            return true;
        }
    }
    
    // Check method-specific sensitive operations
    match method {
        "DELETE" => {
            // Deletion operations are generally sensitive
            path.contains("/delete") || path.contains("/remove") || path.contains("/destroy")
        }
        "POST" => {
            // Creation of sensitive resources
            path.contains("/encrypt") 
            || path.contains("/decrypt")
            || path.contains("/rotate")
            || path.contains("/credentials")
        }
        _ => false,
    }
}

/// Get list of sensitive path patterns
fn get_sensitive_patterns() -> Vec<&'static str> {
    vec![
        // Authentication & Credentials
        "/auth/login",
        "/auth/logout",
        "/auth/token",
        "/credentials",
        "/password",
        
        // Secrets & Keys
        "/secrets",
        "/kms",
        "/keys",
        "/encrypt",
        "/decrypt",
        "/rotate",
        
        // Sensitive Healthcare Data (can be filtered by permission)
        // Note: Some endpoints like patient data should be available
        // but protected by Zanzibar permissions, not excluded entirely
        
        // Audit & Compliance (read-only, but sensitive)
        "/audit",
        "/compliance/assignment",
        
        // Admin Operations
        "/admin",
        "/system",
        "/config",
    ]
}

/// Check if a tool category is sensitive
pub fn is_sensitive_category(category: &str) -> bool {
    let sensitive_categories: HashSet<&str> = [
        "authentication",
        "secrets",
        "kms",
        "encryption",
        "admin",
        "system",
    ].iter().cloned().collect();
    
    sensitive_categories.contains(category)
}

/// Sensitive tool configuration
pub struct SensitiveConfig {
    /// Categories that are always sensitive
    pub sensitive_categories: HashSet<String>,
    
    /// Path patterns that are sensitive
    pub sensitive_paths: Vec<String>,
    
    /// Methods that are sensitive for certain paths
    pub sensitive_methods: HashSet<String>,
}

impl Default for SensitiveConfig {
    fn default() -> Self {
        Self {
            sensitive_categories: [
                "authentication",
                "secrets",
                "kms",
                "encryption",
            ].iter().map(|s| s.to_string()).collect(),
            
            sensitive_paths: vec![
                "/secrets".to_string(),
                "/kms".to_string(),
                "/auth".to_string(),
            ],
            
            sensitive_methods: ["DELETE", "POST"].iter().map(|s| s.to_string()).collect(),
        }
    }
}

impl SensitiveConfig {
    /// Check if a tool should be excluded based on config
    pub fn should_exclude(&self, category: &str, path: &str, method: &str) -> bool {
        // Check category
        if self.sensitive_categories.contains(category) {
            return true;
        }
        
        // Check path patterns
        for pattern in &self.sensitive_paths {
            if path.contains(pattern) {
                return true;
            }
        }
        
        // Check method for sensitive operations
        if self.sensitive_methods.contains(method) {
            if path.contains("/delete") || path.contains("/rotate") || path.contains("/encrypt") {
                return true;
            }
        }
        
        false
    }
}

