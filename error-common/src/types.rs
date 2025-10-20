use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fmt;
use thiserror::Error;
use uuid::Uuid;
use chrono::{DateTime, Utc};

/// Main error type for RustCare Engine
#[derive(Error, Debug, Serialize, Deserialize)]
pub struct RustCareError {
    /// Error type category
    pub error_type: ErrorType,
    /// Structured error code
    pub code: ErrorCode,
    /// Human-readable error message (sanitized)
    pub message: String,
    /// Additional context (sanitized)
    pub context: HashMap<String, String>,
    /// Unique error instance ID for tracing
    pub error_id: Uuid,
    /// Timestamp when error occurred
    pub timestamp: DateTime<Utc>,
    /// Whether this error contains sensitive data
    pub is_sensitive: bool,
    /// Stack trace (only in debug mode)
    #[serde(skip)]
    pub source: Option<Box<dyn std::error::Error + Send + Sync>>,
}

/// Error type categories
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum ErrorType {
    /// Input validation errors
    Validation,
    /// Authentication errors
    Authentication,
    /// Authorization/permission errors
    Authorization,
    /// Database-related errors
    Database,
    /// Network/communication errors
    Network,
    /// Business logic violations
    BusinessLogic,
    /// System/infrastructure errors
    System,
    /// External service errors
    External,
    /// Compliance/regulatory errors
    Compliance,
    /// Configuration errors
    Configuration,
    /// Rate limiting errors
    RateLimit,
    /// Resource not found errors
    NotFound,
    /// Conflict/concurrency errors
    Conflict,
    /// Timeout errors
    Timeout,
}

/// Structured error codes
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct ErrorCode {
    /// Major category (e.g., "AUTH", "DB", "VALIDATION")
    pub category: String,
    /// Specific error code within category
    pub code: u32,
    /// Human-readable description
    pub description: String,
}

impl ErrorCode {
    // Validation errors (1000-1999)
    pub const INVALID_INPUT: ErrorCode = ErrorCode {
        category: String::new(), // Will be set to "VALIDATION" at runtime
        code: 1001,
        description: String::new(), // Will be set at runtime
    };
    
    pub const MISSING_REQUIRED_FIELD: ErrorCode = ErrorCode {
        category: String::new(),
        code: 1002,
        description: String::new(),
    };
    
    pub const INVALID_FORMAT: ErrorCode = ErrorCode {
        category: String::new(),
        code: 1003,
        description: String::new(),
    };
    
    // Authentication errors (2000-2999)
    pub const INVALID_CREDENTIALS: ErrorCode = ErrorCode {
        category: String::new(),
        code: 2001,
        description: String::new(),
    };
    
    pub const TOKEN_EXPIRED: ErrorCode = ErrorCode {
        category: String::new(),
        code: 2002,
        description: String::new(),
    };
    
    pub const SESSION_INVALID: ErrorCode = ErrorCode {
        category: String::new(),
        code: 2003,
        description: String::new(),
    };
    
    // Authorization errors (3000-3999)
    pub const ACCESS_DENIED: ErrorCode = ErrorCode {
        category: String::new(),
        code: 3001,
        description: String::new(),
    };
    
    pub const INSUFFICIENT_PERMISSIONS: ErrorCode = ErrorCode {
        category: String::new(),
        code: 3002,
        description: String::new(),
    };
    
    // Database errors (4000-4999)
    pub const CONNECTION_FAILED: ErrorCode = ErrorCode {
        category: String::new(),
        code: 4001,
        description: String::new(),
    };
    
    pub const QUERY_FAILED: ErrorCode = ErrorCode {
        category: String::new(),
        code: 4002,
        description: String::new(),
    };
    
    pub const CONSTRAINT_VIOLATION: ErrorCode = ErrorCode {
        category: String::new(),
        code: 4003,
        description: String::new(),
    };
    
    // Business logic errors (5000-5999)
    pub const BUSINESS_RULE_VIOLATION: ErrorCode = ErrorCode {
        category: String::new(),
        code: 5001,
        description: String::new(),
    };
    
    pub const WORKFLOW_ERROR: ErrorCode = ErrorCode {
        category: String::new(),
        code: 5002,
        description: String::new(),
    };
    
    // System errors (6000-6999)
    pub const INTERNAL_ERROR: ErrorCode = ErrorCode {
        category: String::new(),
        code: 6001,
        description: String::new(),
    };
    
    pub const SERVICE_UNAVAILABLE: ErrorCode = ErrorCode {
        category: String::new(),
        code: 6002,
        description: String::new(),
    };
}

impl RustCareError {
    /// Create a new validation error
    pub fn validation() -> ErrorBuilder {
        ErrorBuilder::new(ErrorType::Validation)
    }
    
    /// Create a new authentication error
    pub fn authentication() -> ErrorBuilder {
        ErrorBuilder::new(ErrorType::Authentication)
    }
    
    /// Create a new authorization error
    pub fn authorization() -> ErrorBuilder {
        ErrorBuilder::new(ErrorType::Authorization)
    }
    
    /// Create a new database error
    pub fn database() -> ErrorBuilder {
        ErrorBuilder::new(ErrorType::Database)
    }
    
    /// Create a new system error
    pub fn system() -> ErrorBuilder {
        ErrorBuilder::new(ErrorType::System)
    }
    
    /// Create a new business logic error
    pub fn business_logic() -> ErrorBuilder {
        ErrorBuilder::new(ErrorType::BusinessLogic)
    }
    
    /// Check if error is retryable
    pub fn is_retryable(&self) -> bool {
        matches!(
            self.error_type,
            ErrorType::Network | ErrorType::System | ErrorType::External | ErrorType::Timeout
        )
    }
    
    /// Check if error should be reported to external monitoring
    pub fn should_report(&self) -> bool {
        matches!(
            self.error_type,
            ErrorType::System | ErrorType::Database | ErrorType::External
        )
    }
    
    /// Get sanitized error for client response
    pub fn sanitized(&self) -> RustCareError {
        let mut sanitized = RustCareError {
            error_type: self.error_type.clone(),
            code: self.code.clone(),
            message: self.message.clone(),
            context: self.context.clone(),
            error_id: self.error_id,
            timestamp: self.timestamp,
            is_sensitive: self.is_sensitive,
            source: None, // Don't clone the source error
        };
        
        if self.is_sensitive {
            sanitized.message = "An error occurred. Please contact support.".to_string();
            sanitized.context.clear();
        }
        
        sanitized
    }
}

impl fmt::Display for RustCareError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "[{}:{}] {} (ID: {})",
            self.code.category, self.code.code, self.message, self.error_id
        )
    }
}

impl fmt::Display for ErrorType {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ErrorType::Validation => write!(f, "Validation"),
            ErrorType::Authentication => write!(f, "Authentication"),
            ErrorType::Authorization => write!(f, "Authorization"),
            ErrorType::Database => write!(f, "Database"),
            ErrorType::Network => write!(f, "Network"),
            ErrorType::BusinessLogic => write!(f, "BusinessLogic"),
            ErrorType::System => write!(f, "System"),
            ErrorType::External => write!(f, "External"),
            ErrorType::Compliance => write!(f, "Compliance"),
            ErrorType::Configuration => write!(f, "Configuration"),
            ErrorType::RateLimit => write!(f, "RateLimit"),
            ErrorType::NotFound => write!(f, "NotFound"),
            ErrorType::Conflict => write!(f, "Conflict"),
            ErrorType::Timeout => write!(f, "Timeout"),
        }
    }
}

/// Builder pattern for constructing RustCareError instances
pub struct ErrorBuilder {
    error_type: ErrorType,
    code: Option<ErrorCode>,
    message: Option<String>,
    context: HashMap<String, String>,
    is_sensitive: bool,
    source: Option<Box<dyn std::error::Error + Send + Sync>>,
}

impl ErrorBuilder {
    pub fn new(error_type: ErrorType) -> Self {
        Self {
            error_type,
            code: None,
            message: None,
            context: HashMap::new(),
            is_sensitive: false,
            source: None,
        }
    }
    
    pub fn with_code(mut self, code: ErrorCode) -> Self {
        self.code = Some(code);
        self
    }
    
    pub fn with_message<S: Into<String>>(mut self, message: S) -> Self {
        self.message = Some(message.into());
        self
    }
    
    pub fn with_context<K: Into<String>, V: Into<String>>(mut self, key: K, value: V) -> Self {
        self.context.insert(key.into(), value.into());
        self
    }
    
    pub fn sensitive(mut self) -> Self {
        self.is_sensitive = true;
        self
    }
    
    pub fn with_source<E: std::error::Error + Send + Sync + 'static>(mut self, source: E) -> Self {
        self.source = Some(Box::new(source));
        self
    }
    
    pub fn build(self) -> RustCareError {
        let error_type = self.error_type.clone();
        let code = self.code.unwrap_or_else(|| Self::default_code_for_type(&error_type));
        
        RustCareError {
            error_type,
            code,
            message: self.message.unwrap_or_else(|| "An error occurred".to_string()),
            context: self.context,
            error_id: Uuid::new_v4(),
            timestamp: Utc::now(),
            is_sensitive: self.is_sensitive,
            source: self.source,
        }
    }
    
    fn default_code_for_type(error_type: &ErrorType) -> ErrorCode {
        match error_type {
            ErrorType::Validation => ErrorCode::INVALID_INPUT,
            ErrorType::Authentication => ErrorCode::INVALID_CREDENTIALS,
            ErrorType::Authorization => ErrorCode::ACCESS_DENIED,
            ErrorType::Database => ErrorCode::CONNECTION_FAILED,
            ErrorType::System => ErrorCode::INTERNAL_ERROR,
            ErrorType::BusinessLogic => ErrorCode::BUSINESS_RULE_VIOLATION,
            _ => ErrorCode::INTERNAL_ERROR,
        }
    }
}

/// Result type alias for RustCare operations
pub type RustCareResult<T> = Result<T, RustCareError>;