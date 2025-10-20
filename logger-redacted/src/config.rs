// Logger configuration
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LoggerConfig {
    pub redaction_enabled: bool,
    pub compliance_logging: bool,
    pub log_level: String,
}

impl Default for LoggerConfig {
    fn default() -> Self {
        Self {
            redaction_enabled: true,
            compliance_logging: true,
            log_level: "info".to_string(),
        }
    }
}