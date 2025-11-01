//! Transport layer for MCP (stdin/stdout or HTTP)

/// Transport type enumeration
#[derive(Debug, Clone)]
pub enum TransportType {
    /// STDIO transport (used by clients)
    Stdio,
    /// HTTP transport (for web clients)
    Http { port: u16 },
    /// WebSocket transport (for real-time clients)
    WebSocket { port: u16 },
}

/// Transport configuration
#[derive(Debug, Clone)]
pub struct TransportConfig {
    /// Transport type
    pub transport_type: TransportType,
    /// Enable TLS
    pub enable_tls: bool,
}

impl Default for TransportConfig {
    fn default() -> Self {
        Self {
            transport_type: TransportType::Stdio,
            enable_tls: false,
        }
    }
}

/// MCP Transport abstraction
#[async_trait::async_trait]
pub trait Transport: Send + Sync {
    /// Read a request from transport
    async fn read_request(&mut self) -> anyhow::Result<String>;
    
    /// Write a response to transport
    async fn write_response(&mut self, response: String) -> anyhow::Result<()>;
}

// TODO: Implement actual transport implementations
// - StdioTransport
// - HttpTransport
// - WebSocketTransport

