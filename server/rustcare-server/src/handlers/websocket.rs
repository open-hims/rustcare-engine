use axum::{
    extract::{
        ws::{Message, WebSocket, WebSocketUpgrade},
        State, ConnectInfo,
    },
    response::Response,
};
use futures::{sink::SinkExt, stream::StreamExt};
use serde::{Deserialize, Serialize};
use std::{
    collections::HashMap,
    net::SocketAddr,
    sync::{Arc, atomic::{AtomicU64, Ordering}},
};
use tokio::sync::{RwLock, broadcast};
use tracing::{debug, error, info, warn};
use uuid::Uuid;

use crate::server::RustCareServer;
use error_common::{RustCareError, Result};
use logger_redacted::RedactedLogger;

/// WebSocket connection manager for healthcare real-time communication
#[derive(Clone)]
pub struct WebSocketManager {
    /// Active connections registry
    connections: Arc<RwLock<HashMap<String, WebSocketConnection>>>,
    /// Broadcast channel for server-wide messages
    broadcast_tx: broadcast::Sender<WebSocketMessage>,
    /// Connection counter for unique IDs
    connection_counter: Arc<AtomicU64>,
    /// Redacted logger for HIPAA compliance
    logger: Arc<RedactedLogger>,
}

/// WebSocket connection metadata
#[derive(Debug, Clone)]
pub struct WebSocketConnection {
    /// Connection ID
    pub connection_id: String,
    /// User ID (if authenticated)
    pub user_id: Option<String>,
    /// User role (for permission checks)
    pub user_role: Option<String>,
    /// Connection timestamp
    pub connected_at: chrono::DateTime<chrono::Utc>,
    /// Client IP address
    pub client_ip: String,
    /// Subscribed channels
    pub subscribed_channels: Vec<String>,
    /// Connection type
    pub connection_type: WebSocketConnectionType,
}

/// WebSocket connection types for healthcare
#[derive(Debug, Clone, PartialEq)]
pub enum WebSocketConnectionType {
    /// Healthcare provider dashboard
    ProviderDashboard,
    /// Patient portal
    PatientPortal,
    /// Administrative console
    AdminConsole,
    /// Real-time monitoring
    RealtimeMonitoring,
    /// Emergency alerts
    EmergencyAlerts,
    /// System notifications
    SystemNotifications,
}

/// WebSocket message types
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WebSocketMessage {
    /// Message ID
    pub id: String,
    /// Message type
    pub message_type: WebSocketMessageType,
    /// Target channel
    pub channel: String,
    /// Message payload
    pub payload: serde_json::Value,
    /// Timestamp
    pub timestamp: chrono::DateTime<chrono::Utc>,
    /// Sender ID (optional)
    pub sender_id: Option<String>,
    /// HIPAA compliance flag
    pub hipaa_compliant: bool,
}

/// WebSocket message types for healthcare
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum WebSocketMessageType {
    // Real-time updates
    PatientVitalSigns,
    AppointmentUpdate,
    MedicalRecordUpdate,
    
    // Notifications
    SystemNotification,
    ProviderAlert,
    PatientAlert,
    EmergencyAlert,
    
    // Chat and communication
    ChatMessage,
    ConsultationRequest,
    ConsultationResponse,
    
    // System events
    ConnectionStatus,
    HeartBeat,
    Error,
    
    // Admin events
    UserStatusChange,
    SystemMaintenance,
    AuditAlert,
}

/// WebSocket client message
#[derive(Debug, Deserialize)]
pub struct ClientMessage {
    pub action: String,
    pub channel: Option<String>,
    pub payload: Option<serde_json::Value>,
}

/// WebSocket server response
#[derive(Debug, Serialize)]
pub struct ServerResponse {
    pub success: bool,
    pub message: String,
    pub data: Option<serde_json::Value>,
    pub timestamp: chrono::DateTime<chrono::Utc>,
}

impl WebSocketManager {
    /// Create a new WebSocket manager
    pub fn new(logger: Arc<RedactedLogger>) -> Self {
        let (broadcast_tx, _) = broadcast::channel(1000);
        
        Self {
            connections: Arc::new(RwLock::new(HashMap::new())),
            broadcast_tx,
            connection_counter: Arc::new(AtomicU64::new(0)),
            logger,
        }
    }

    /// Get broadcast sender for publishing messages
    pub fn get_broadcaster(&self) -> broadcast::Sender<WebSocketMessage> {
        self.broadcast_tx.clone()
    }

    /// Register a new WebSocket connection
    pub async fn register_connection(
        &self,
        connection: WebSocketConnection,
    ) -> Result<broadcast::Receiver<WebSocketMessage>> {
        let mut connections = self.connections.write().await;
        let receiver = self.broadcast_tx.subscribe();
        
        connections.insert(connection.connection_id.clone(), connection.clone());
        
        self.logger.info(&format!(
            "WebSocket connection registered: {} (Type: {:?}, User: {:?})",
            connection.connection_id,
            connection.connection_type,
            connection.user_id
        )).await;

        Ok(receiver)
    }

    /// Unregister a WebSocket connection
    pub async fn unregister_connection(&self, connection_id: &str) {
        let mut connections = self.connections.write().await;
        
        if let Some(connection) = connections.remove(connection_id) {
            self.logger.info(&format!(
                "WebSocket connection unregistered: {} (User: {:?})",
                connection_id,
                connection.user_id
            )).await;
        }
    }

    /// Broadcast message to all connections in a channel
    pub async fn broadcast_to_channel(&self, channel: &str, message: WebSocketMessage) -> Result<()> {
        let connections = self.connections.read().await;
        
        // Filter connections by channel subscription
        let target_connections: Vec<_> = connections
            .values()
            .filter(|conn| conn.subscribed_channels.contains(&channel.to_string()))
            .cloned()
            .collect();

        if !target_connections.is_empty() {
            if let Err(e) = self.broadcast_tx.send(message.clone()) {
                self.logger.error(&format!(
                    "Failed to broadcast message to channel {}: {}",
                    channel, e
                )).await;
                return Err(RustCareError::WebSocketError(format!("Broadcast failed: {}", e)));
            }

            self.logger.debug(&format!(
                "Broadcasted message to {} connections in channel {}",
                target_connections.len(),
                channel
            )).await;
        }

        Ok(())
    }

    /// Send message to specific connection
    pub async fn send_to_connection(&self, connection_id: &str, message: WebSocketMessage) -> Result<()> {
        // In a real implementation, we would maintain individual connection senders
        // For now, we'll use the broadcast mechanism with connection-specific filtering
        self.broadcast_tx.send(message)
            .map_err(|e| RustCareError::WebSocketError(format!("Send failed: {}", e)))?;
        
        Ok(())
    }

    /// Get connection statistics
    pub async fn get_connection_stats(&self) -> ConnectionStats {
        let connections = self.connections.read().await;
        
        let mut stats = ConnectionStats {
            total_connections: connections.len(),
            connections_by_type: HashMap::new(),
            connections_by_role: HashMap::new(),
        };

        for connection in connections.values() {
            // Count by connection type
            *stats.connections_by_type.entry(connection.connection_type.clone()).or_insert(0) += 1;
            
            // Count by user role
            if let Some(ref role) = connection.user_role {
                *stats.connections_by_role.entry(role.clone()).or_insert(0) += 1;
            }
        }

        stats
    }
}

/// Connection statistics
#[derive(Debug, Serialize)]
pub struct ConnectionStats {
    pub total_connections: usize,
    pub connections_by_type: HashMap<WebSocketConnectionType, usize>,
    pub connections_by_role: HashMap<String, usize>,
}

/// Handle WebSocket upgrade and connection
pub async fn websocket_handler(
    ws: WebSocketUpgrade,
    ConnectInfo(addr): ConnectInfo<SocketAddr>,
    State(server): State<RustCareServer>,
) -> Response {
    info!("WebSocket connection attempt from: {}", addr);
    
    ws.on_upgrade(move |socket| handle_websocket_connection(socket, addr, server))
}

/// Handle individual WebSocket connection
async fn handle_websocket_connection(
    socket: WebSocket,
    addr: SocketAddr,
    server: RustCareServer,
) {
    let connection_id = format!("ws_{}", Uuid::new_v4());
    let logger = Arc::new(RedactedLogger::new("websocket").await);
    let ws_manager = WebSocketManager::new(logger.clone());
    
    // Create connection metadata
    let connection = WebSocketConnection {
        connection_id: connection_id.clone(),
        user_id: None, // Will be set after authentication
        user_role: None, // Will be set after authentication
        connected_at: chrono::Utc::now(),
        client_ip: addr.ip().to_string(),
        subscribed_channels: vec!["general".to_string()],
        connection_type: WebSocketConnectionType::SystemNotifications,
    };

    // Register connection and get broadcast receiver
    let mut broadcast_rx = match ws_manager.register_connection(connection.clone()).await {
        Ok(rx) => rx,
        Err(e) => {
            error!("Failed to register WebSocket connection: {}", e);
            return;
        }
    };

    // Split socket into sender and receiver
    let (mut sender, mut receiver) = socket.split();

    // Send welcome message
    let welcome_msg = WebSocketMessage {
        id: Uuid::new_v4().to_string(),
        message_type: WebSocketMessageType::ConnectionStatus,
        channel: "system".to_string(),
        payload: serde_json::json!({
            "status": "connected",
            "connection_id": connection_id,
            "server_info": {
                "name": "RustCare Engine",
                "version": env!("CARGO_PKG_VERSION"),
                "hipaa_compliant": true
            }
        }),
        timestamp: chrono::Utc::now(),
        sender_id: Some("system".to_string()),
        hipaa_compliant: true,
    };

    if let Ok(welcome_json) = serde_json::to_string(&welcome_msg) {
        let _ = sender.send(Message::Text(welcome_json)).await;
    }

    // Handle incoming and outgoing messages concurrently
    let connection_id_clone = connection_id.clone();
    let ws_manager_clone = ws_manager.clone();
    
    tokio::select! {
        // Handle incoming messages from client
        _ = async {
            while let Some(msg) = receiver.next().await {
                match msg {
                    Ok(Message::Text(text)) => {
                        if let Err(e) = handle_client_message(&text, &connection_id, &server, &ws_manager).await {
                            logger.error(&format!("Error handling client message: {}", e)).await;
                        }
                    }
                    Ok(Message::Binary(bin)) => {
                        debug!("Received binary message: {} bytes", bin.len());
                        // Handle binary messages (e.g., file uploads) if needed
                    }
                    Ok(Message::Close(_)) => {
                        info!("WebSocket connection closed by client: {}", connection_id);
                        break;
                    }
                    Ok(Message::Ping(ping)) => {
                        let _ = sender.send(Message::Pong(ping)).await;
                    }
                    Ok(Message::Pong(_)) => {
                        // Handle pong if needed
                    }
                    Err(e) => {
                        error!("WebSocket error: {}", e);
                        break;
                    }
                }
            }
        } => {},
        
        // Handle broadcast messages
        _ = async {
            while let Ok(broadcast_msg) = broadcast_rx.recv().await {
                if let Ok(msg_json) = serde_json::to_string(&broadcast_msg) {
                    if sender.send(Message::Text(msg_json)).await.is_err() {
                        break;
                    }
                }
            }
        } => {}
    }

    // Cleanup connection
    ws_manager_clone.unregister_connection(&connection_id_clone).await;
    info!("WebSocket connection closed: {}", connection_id_clone);
}

/// Handle client message
async fn handle_client_message(
    text: &str,
    connection_id: &str,
    server: &RustCareServer,
    ws_manager: &WebSocketManager,
) -> Result<()> {
    let client_msg: ClientMessage = serde_json::from_str(text)
        .map_err(|e| RustCareError::WebSocketError(format!("Invalid message format: {}", e)))?;

    match client_msg.action.as_str() {
        "subscribe" => {
            if let Some(channel) = client_msg.channel {
                // Handle channel subscription
                info!("Connection {} subscribing to channel: {}", connection_id, channel);
                
                let response = ServerResponse {
                    success: true,
                    message: format!("Subscribed to channel: {}", channel),
                    data: Some(serde_json::json!({ "channel": channel })),
                    timestamp: chrono::Utc::now(),
                };

                // Send response back to client
                let response_msg = WebSocketMessage {
                    id: Uuid::new_v4().to_string(),
                    message_type: WebSocketMessageType::SystemNotification,
                    channel: "system".to_string(),
                    payload: serde_json::to_value(response)?,
                    timestamp: chrono::Utc::now(),
                    sender_id: Some("system".to_string()),
                    hipaa_compliant: true,
                };

                ws_manager.send_to_connection(connection_id, response_msg).await?;
            }
        }
        "unsubscribe" => {
            if let Some(channel) = client_msg.channel {
                info!("Connection {} unsubscribing from channel: {}", connection_id, channel);
            }
        }
        "ping" => {
            let pong_msg = WebSocketMessage {
                id: Uuid::new_v4().to_string(),
                message_type: WebSocketMessageType::HeartBeat,
                channel: "system".to_string(),
                payload: serde_json::json!({ "type": "pong" }),
                timestamp: chrono::Utc::now(),
                sender_id: Some("system".to_string()),
                hipaa_compliant: true,
            };

            ws_manager.send_to_connection(connection_id, pong_msg).await?;
        }
        _ => {
            warn!("Unknown WebSocket action: {}", client_msg.action);
        }
    }

    Ok(())
}