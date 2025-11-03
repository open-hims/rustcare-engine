//! Model Context Protocol (MCP) Server for RustCare Engine
//! 
//! Provides MCP-compliant server implementation that exposes RustCare capabilities
//! to AI agents and clients, enabling natural language access to healthcare functions.
//!
//! # MCP Server Capabilities
//!
//! - **Patient Management**: Query patient records, demographics, visits
//! - **Clinical Data**: Access medical records, medications, allergies
//! - **Appointments**: Schedule, cancel, reschedule appointments
//! - **Voice Dictation**: Start/stop voice transcription sessions
//! - **Notifications**: Send alerts, read notifications
//! - **Pharmacy**: Manage prescriptions and inventory
//! - **Analytics**: Generate reports, query metrics
//!
//! # Architecture
//!
//! The MCP server acts as a bridge between:
//! - AI agents/clients (via JSON-RPC over stdio/HTTP)
//! - RustCare plugin runtime
//! - Healthcare services (EMR, pharmacy, etc.)
//!
//! # Example Client Usage
//!
//! ```bash
//! # Start MCP server
//! cargo run --bin mcp-server
//!
//! # Connect from AI client
//! mcp-client connect rustcare://localhost:8080/mcp
//!
//! # Natural language request
//! "Show me the patient with ID 12345"
//! # MCP translates to: invoke_tool("get_patient", {"patient_id": "12345"})
//! ```

pub mod server;
pub mod protocol;
pub mod tools;
pub mod capabilities;
pub mod transport;
pub mod error;
pub mod tool_wrapper;
pub mod sensitive_filter;
pub mod render;

pub use server::*;
pub use protocol::*;
pub use tools::*;
pub use capabilities::*;
pub use tool_wrapper::*;
pub use sensitive_filter::*;
pub use render::*;
pub use error::{McpError as Error, McpResult as Result};

/// MCP Server for RustCare
pub struct McpServer {
    server: server::Server,
}

impl McpServer {
    /// Create a new MCP server instance
    pub fn new() -> Self {
        Self {
            server: server::Server::new(),
        }
    }

    /// Start the MCP server
    pub async fn start(&self) -> anyhow::Result<()> {
        self.server.start().await
    }

    /// Stop the MCP server
    pub async fn stop(&self) -> anyhow::Result<()> {
        self.server.stop().await
    }
}

impl Default for McpServer {
    fn default() -> Self {
        Self::new()
    }
}

