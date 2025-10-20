use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Error context information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ErrorContext {
    pub request_id: Option<String>,
    pub user_id: Option<String>,
    pub session_id: Option<String>,
    pub trace_id: Option<String>,
    pub additional: HashMap<String, String>,
}

impl ErrorContext {
    pub fn new() -> Self {
        Self {
            request_id: None,
            user_id: None,
            session_id: None,
            trace_id: None,
            additional: HashMap::new(),
        }
    }
    
    pub fn with_request_id(mut self, request_id: String) -> Self {
        self.request_id = Some(request_id);
        self
    }
    
    pub fn with_user_id(mut self, user_id: String) -> Self {
        self.user_id = Some(user_id);
        self
    }
    
    pub fn with_trace_id(mut self, trace_id: String) -> Self {
        self.trace_id = Some(trace_id);
        self
    }
    
    pub fn add_context<K: Into<String>, V: Into<String>>(mut self, key: K, value: V) -> Self {
        self.additional.insert(key.into(), value.into());
        self
    }
}