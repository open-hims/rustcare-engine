//! Plugin runtime error types
//! 
//! Comprehensive error handling for the plugin runtime system
//! with detailed error context and recovery information.

use std::fmt;
use thiserror::Error;
use uuid::Uuid;

/// Main plugin runtime error type
#[derive(Error, Debug)]
pub enum PluginRuntimeError {
    /// Plugin not found
    #[error("Plugin not found: {0}")]
    PluginNotFound(Uuid),
    
    /// Invalid plugin state for operation
    #[error("Invalid plugin state: {0}")]
    InvalidState(String),
    
    /// Plugin loading failed
    #[error("Plugin loading failed: {0}")]
    LoadingFailed(String),
    
    /// Plugin execution failed
    #[error("Plugin execution failed: {0}")]
    ExecutionFailed(String),
    
    /// Invalid plugin operation
    #[error("Invalid plugin operation: {0}")]
    InvalidOperation(String),
    
    /// Resource limit exceeded
    #[error("Resource limit exceeded: {0}")]
    ResourceLimitExceeded(String),
    
    /// Security violation
    #[error("Security violation: {0}")]
    SecurityViolation(String),
    
    /// Invalid plugin module
    #[error("Invalid plugin module: {0}")]
    InvalidModule(String),
    
    /// Invalid plugin manifest
    #[error("Invalid plugin manifest: {0}")]
    InvalidManifest(String),
    
    /// Unsupported plugin type
    #[error("Unsupported plugin type")]
    UnsupportedPluginType,
    
    /// Plugin timeout
    #[error("Plugin operation timed out: {0}")]
    Timeout(String),
    
    /// Configuration error
    #[error("Configuration error: {0}")]
    ConfigurationError(String),
    
    /// Runtime initialization failed
    #[error("Runtime initialization failed: {0}")]
    InitializationFailed(String),
    
    /// Plugin communication error
    #[error("Plugin communication error: {0}")]
    CommunicationError(String),
    
    /// Serialization error
    #[error("Serialization error: {0}")]
    SerializationError(String),
    
    /// I/O error
    #[error("I/O error: {0}")]
    IoError(#[from] std::io::Error),
    
    /// JSON error
    #[error("JSON error: {0}")]
    JsonError(#[from] serde_json::Error),
    
    /// Generic plugin error with context
    #[error("Plugin error in {context}: {message}")]
    PluginError {
        /// Error context
        context: String,
        /// Error message
        message: String,
        /// Error source
        source: Option<Box<dyn std::error::Error + Send + Sync>>,
    },
}

/// Plugin runtime result type
pub type PluginResult<T> = Result<T, PluginRuntimeError>;

/// Error context for plugin operations
#[derive(Debug, Clone)]
pub struct ErrorContext {
    /// Operation being performed
    pub operation: String,
    /// Plugin ID (if applicable)
    pub plugin_id: Option<Uuid>,
    /// Additional context data
    pub context_data: std::collections::HashMap<String, String>,
    /// Timestamp when error occurred
    pub timestamp: chrono::DateTime<chrono::Utc>,
}

impl ErrorContext {
    /// Create a new error context
    pub fn new(operation: impl Into<String>) -> Self {
        Self {
            operation: operation.into(),
            plugin_id: None,
            context_data: std::collections::HashMap::new(),
            timestamp: chrono::Utc::now(),
        }
    }
    
    /// Add plugin ID to context
    pub fn with_plugin_id(mut self, plugin_id: Uuid) -> Self {
        self.plugin_id = Some(plugin_id);
        self
    }
    
    /// Add context data
    pub fn with_context(mut self, key: impl Into<String>, value: impl Into<String>) -> Self {
        self.context_data.insert(key.into(), value.into());
        self
    }
}

/// Error recovery information
#[derive(Debug, Clone)]
pub struct ErrorRecovery {
    /// Whether the error is recoverable
    pub is_recoverable: bool,
    /// Suggested recovery actions
    pub recovery_actions: Vec<RecoveryAction>,
    /// Retry policy
    pub retry_policy: Option<RetryPolicy>,
}

/// Recovery action types
#[derive(Debug, Clone)]
pub enum RecoveryAction {
    /// Restart the plugin
    RestartPlugin,
    /// Reload plugin configuration
    ReloadConfiguration,
    /// Clear plugin cache
    ClearCache,
    /// Reset plugin state
    ResetState,
    /// Increase resource limits
    IncreaseResourceLimits,
    /// Update security policy
    UpdateSecurityPolicy,
    /// Custom recovery action
    Custom(String),
}

/// Retry policy configuration
#[derive(Debug, Clone)]
pub struct RetryPolicy {
    /// Maximum retry attempts
    pub max_attempts: u32,
    /// Base delay between retries (milliseconds)
    pub base_delay_ms: u64,
    /// Maximum delay between retries (milliseconds)
    pub max_delay_ms: u64,
    /// Backoff strategy
    pub backoff_strategy: BackoffStrategy,
}

/// Backoff strategy for retries
#[derive(Debug, Clone)]
pub enum BackoffStrategy {
    /// Fixed delay
    Fixed,
    /// Linear increase
    Linear,
    /// Exponential backoff
    Exponential,
    /// Exponential with jitter
    ExponentialJitter,
}

impl PluginRuntimeError {
    /// Create a new plugin error with context
    pub fn with_context(
        message: impl Into<String>,
        context: impl Into<String>,
    ) -> Self {
        Self::PluginError {
            context: context.into(),
            message: message.into(),
            source: None,
        }
    }
    
    /// Create a plugin error with source
    pub fn with_source(
        message: impl Into<String>,
        context: impl Into<String>,
        source: Box<dyn std::error::Error + Send + Sync>,
    ) -> Self {
        Self::PluginError {
            context: context.into(),
            message: message.into(),
            source: Some(source),
        }
    }
    
    /// Get error recovery information
    pub fn recovery_info(&self) -> ErrorRecovery {
        match self {
            Self::PluginNotFound(_) => ErrorRecovery {
                is_recoverable: false,
                recovery_actions: vec![],
                retry_policy: None,
            },
            Self::InvalidState(_) => ErrorRecovery {
                is_recoverable: true,
                recovery_actions: vec![RecoveryAction::ResetState, RecoveryAction::RestartPlugin],
                retry_policy: Some(RetryPolicy {
                    max_attempts: 3,
                    base_delay_ms: 1000,
                    max_delay_ms: 5000,
                    backoff_strategy: BackoffStrategy::Linear,
                }),
            },
            Self::LoadingFailed(_) => ErrorRecovery {
                is_recoverable: true,
                recovery_actions: vec![RecoveryAction::ReloadConfiguration],
                retry_policy: Some(RetryPolicy {
                    max_attempts: 2,
                    base_delay_ms: 2000,
                    max_delay_ms: 10000,
                    backoff_strategy: BackoffStrategy::Fixed,
                }),
            },
            Self::ExecutionFailed(_) => ErrorRecovery {
                is_recoverable: true,
                recovery_actions: vec![RecoveryAction::RestartPlugin, RecoveryAction::ClearCache],
                retry_policy: Some(RetryPolicy {
                    max_attempts: 3,
                    base_delay_ms: 1000,
                    max_delay_ms: 8000,
                    backoff_strategy: BackoffStrategy::Exponential,
                }),
            },
            Self::ResourceLimitExceeded(_) => ErrorRecovery {
                is_recoverable: true,
                recovery_actions: vec![RecoveryAction::IncreaseResourceLimits],
                retry_policy: None,
            },
            Self::SecurityViolation(_) => ErrorRecovery {
                is_recoverable: false,
                recovery_actions: vec![RecoveryAction::UpdateSecurityPolicy],
                retry_policy: None,
            },
            Self::Timeout(_) => ErrorRecovery {
                is_recoverable: true,
                recovery_actions: vec![RecoveryAction::RestartPlugin],
                retry_policy: Some(RetryPolicy {
                    max_attempts: 2,
                    base_delay_ms: 5000,
                    max_delay_ms: 15000,
                    backoff_strategy: BackoffStrategy::Fixed,
                }),
            },
            _ => ErrorRecovery {
                is_recoverable: true,
                recovery_actions: vec![RecoveryAction::RestartPlugin],
                retry_policy: Some(RetryPolicy {
                    max_attempts: 1,
                    base_delay_ms: 1000,
                    max_delay_ms: 1000,
                    backoff_strategy: BackoffStrategy::Fixed,
                }),
            },
        }
    }
    
    /// Check if error is considered critical
    pub fn is_critical(&self) -> bool {
        matches!(
            self,
            Self::SecurityViolation(_) | Self::InvalidModule(_) | Self::InitializationFailed(_)
        )
    }
    
    /// Get error category for logging and metrics
    pub fn category(&self) -> ErrorCategory {
        match self {
            Self::PluginNotFound(_) => ErrorCategory::NotFound,
            Self::InvalidState(_) => ErrorCategory::State,
            Self::LoadingFailed(_) => ErrorCategory::Loading,
            Self::ExecutionFailed(_) => ErrorCategory::Execution,
            Self::InvalidOperation(_) => ErrorCategory::Operation,
            Self::ResourceLimitExceeded(_) => ErrorCategory::Resource,
            Self::SecurityViolation(_) => ErrorCategory::Security,
            Self::InvalidModule(_) => ErrorCategory::Module,
            Self::InvalidManifest(_) => ErrorCategory::Configuration,
            Self::UnsupportedPluginType => ErrorCategory::Configuration,
            Self::Timeout(_) => ErrorCategory::Timeout,
            Self::ConfigurationError(_) => ErrorCategory::Configuration,
            Self::InitializationFailed(_) => ErrorCategory::Initialization,
            Self::CommunicationError(_) => ErrorCategory::Communication,
            Self::SerializationError(_) => ErrorCategory::Serialization,
            Self::IoError(_) => ErrorCategory::Io,
            Self::JsonError(_) => ErrorCategory::Serialization,
            Self::PluginError { .. } => ErrorCategory::Plugin,
        }
    }
}

/// Error category enumeration
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ErrorCategory {
    /// Not found errors
    NotFound,
    /// State management errors
    State,
    /// Loading errors
    Loading,
    /// Execution errors
    Execution,
    /// Operation errors
    Operation,
    /// Resource errors
    Resource,
    /// Security errors
    Security,
    /// Module errors
    Module,
    /// Timeout errors
    Timeout,
    /// Configuration errors
    Configuration,
    /// Initialization errors
    Initialization,
    /// Communication errors
    Communication,
    /// Serialization errors
    Serialization,
    /// I/O errors
    Io,
    /// Generic plugin errors
    Plugin,
}

impl fmt::Display for ErrorCategory {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::NotFound => write!(f, "not_found"),
            Self::State => write!(f, "state"),
            Self::Loading => write!(f, "loading"),
            Self::Execution => write!(f, "execution"),
            Self::Operation => write!(f, "operation"),
            Self::Resource => write!(f, "resource"),
            Self::Security => write!(f, "security"),
            Self::Module => write!(f, "module"),
            Self::Timeout => write!(f, "timeout"),
            Self::Configuration => write!(f, "configuration"),
            Self::Initialization => write!(f, "initialization"),
            Self::Communication => write!(f, "communication"),
            Self::Serialization => write!(f, "serialization"),
            Self::Io => write!(f, "io"),
            Self::Plugin => write!(f, "plugin"),
        }
    }
}