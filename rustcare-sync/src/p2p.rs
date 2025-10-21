//! Peer-to-Peer Local Network Synchronization
//!
//! Enables direct synchronization between devices on the same local network
//! without requiring internet connectivity or a central server.
//!
//! Features:
//! - mDNS service discovery for automatic peer detection
//! - WebSocket connections for real-time sync
//! - Automatic conflict resolution using CRDTs
//! - Mesh topology for multi-peer sync
//!
//! Use Cases:
//! - Clinic with multiple devices on same WiFi
//! - Offline rural clinics syncing when devices are nearby
//! - Emergency response scenarios without internet

use crate::error::{SyncError, SyncResult};
use crate::local_db::LocalDatabase;
use crate::sync_protocol::{SyncOperation, SyncStats};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::net::SocketAddr;
use std::sync::Arc;
use tokio::sync::RwLock;
use uuid::Uuid;

/// P2P sync configuration
#[derive(Debug, Clone)]
pub struct P2PConfig {
    /// Service name for mDNS discovery
    pub service_name: String,
    /// Service type (e.g., "_rustcare-sync._tcp")
    pub service_type: String,
    /// Port for WebSocket connections
    pub port: u16,
    /// Enable automatic peer discovery
    pub enable_discovery: bool,
    /// Sync interval in seconds
    pub sync_interval_secs: u64,
    /// Maximum number of concurrent peer connections
    pub max_peers: usize,
}

impl Default for P2PConfig {
    fn default() -> Self {
        Self {
            service_name: "RustCare Sync".to_string(),
            service_type: "_rustcare-sync._tcp.local.".to_string(),
            port: 9876,
            enable_discovery: true,
            sync_interval_secs: 30,
            max_peers: 10,
        }
    }
}

/// Peer information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PeerInfo {
    /// Peer node ID
    pub node_id: Uuid,
    /// Peer name/hostname
    pub name: String,
    /// Peer address
    pub address: SocketAddr,
    /// Last seen timestamp
    pub last_seen: chrono::DateTime<chrono::Utc>,
    /// Connection status
    pub status: PeerStatus,
}

/// Peer connection status
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum PeerStatus {
    Discovered,
    Connecting,
    Connected,
    Disconnected,
    Error,
}

/// P2P synchronization engine
pub struct P2PSync {
    local_db: Arc<LocalDatabase>,
    config: P2PConfig,
    peers: Arc<RwLock<HashMap<Uuid, PeerInfo>>>,
    node_id: Uuid,
}

impl P2PSync {
    /// Create a new P2P sync instance
    pub fn new(local_db: Arc<LocalDatabase>, config: P2PConfig) -> Self {
        let node_id = local_db.node_id();
        
        Self {
            local_db,
            config,
            peers: Arc::new(RwLock::new(HashMap::new())),
            node_id,
        }
    }
    
    /// Start P2P sync service
    ///
    /// This will:
    /// 1. Start mDNS service discovery
    /// 2. Advertise this node on the network
    /// 3. Listen for incoming connections
    /// 4. Periodically sync with discovered peers
    pub async fn start(&self) -> SyncResult<()> {
        tracing::info!(
            node_id = %self.node_id,
            port = self.config.port,
            "Starting P2P sync service"
        );
        
        if self.config.enable_discovery {
            // Start mDNS discovery
            #[cfg(feature = "p2p")]
            {
                self.start_mdns_discovery().await?;
            }
            #[cfg(not(feature = "p2p"))]
            {
                tracing::warn!("P2P feature not enabled, skipping mDNS discovery");
            }
        }
        
        // Start WebSocket server
        #[cfg(feature = "p2p")]
        {
            self.start_websocket_server().await?;
        }
        
        // Start periodic sync
        self.start_periodic_sync().await;
        
        Ok(())
    }
    
    /// Stop P2P sync service
    pub async fn stop(&self) -> SyncResult<()> {
        tracing::info!("Stopping P2P sync service");
        
        // Disconnect from all peers
        let mut peers = self.peers.write().await;
        for (peer_id, peer) in peers.iter_mut() {
            tracing::debug!(peer_id = %peer_id, "Disconnecting peer");
            peer.status = PeerStatus::Disconnected;
        }
        
        Ok(())
    }
    
    /// Get list of discovered peers
    pub async fn get_peers(&self) -> Vec<PeerInfo> {
        let peers = self.peers.read().await;
        peers.values().cloned().collect()
    }
    
    /// Manually add a peer by address
    pub async fn add_peer(&self, address: SocketAddr, name: String) -> SyncResult<()> {
        let peer = PeerInfo {
            node_id: Uuid::new_v4(), // Will be updated on connection
            name,
            address,
            last_seen: chrono::Utc::now(),
            status: PeerStatus::Discovered,
        };
        
        let mut peers = self.peers.write().await;
        peers.insert(peer.node_id, peer);
        
        tracing::info!(address = %address, "Manually added peer");
        Ok(())
    }
    
    /// Sync with a specific peer
    pub async fn sync_with_peer(&self, peer_id: Uuid) -> SyncResult<SyncStats> {
        let peers = self.peers.read().await;
        let peer = peers.get(&peer_id)
            .ok_or_else(|| SyncError::NotFound(format!("Peer {} not found", peer_id)))?;
        
        tracing::info!(
            peer_id = %peer_id,
            peer_address = %peer.address,
            "Syncing with peer"
        );
        
        // TODO: Implement actual WebSocket sync
        // For now, return empty stats
        Ok(SyncStats::default())
    }
    
    /// Sync with all connected peers
    pub async fn sync_all(&self) -> SyncResult<SyncStats> {
        let mut total_stats = SyncStats::default();
        
        let peers = self.peers.read().await;
        let connected_peers: Vec<_> = peers
            .iter()
            .filter(|(_, p)| p.status == PeerStatus::Connected)
            .map(|(id, _)| *id)
            .collect();
        
        drop(peers); // Release lock
        
        for peer_id in connected_peers {
            match self.sync_with_peer(peer_id).await {
                Ok(stats) => {
                    total_stats.pulled_operations += stats.pulled_operations;
                    total_stats.pushed_operations += stats.pushed_operations;
                    total_stats.conflicts_resolved += stats.conflicts_resolved;
                }
                Err(e) => {
                    tracing::warn!(peer_id = %peer_id, error = %e, "Failed to sync with peer");
                    total_stats.failed_operations += 1;
                }
            }
        }
        
        Ok(total_stats)
    }
    
    #[cfg(feature = "p2p")]
    /// Start mDNS service discovery
    async fn start_mdns_discovery(&self) -> SyncResult<()> {
        tracing::info!("Starting mDNS discovery");
        
        // TODO: Implement mDNS discovery using mdns-sd crate
        // 1. Create mDNS service
        // 2. Register this node
        // 3. Browse for peers
        // 4. Add discovered peers to peers map
        
        Ok(())
    }
    
    #[cfg(feature = "p2p")]
    /// Start WebSocket server for incoming connections
    async fn start_websocket_server(&self) -> SyncResult<()> {
        let addr = format!("0.0.0.0:{}", self.config.port);
        tracing::info!(address = %addr, "Starting WebSocket server");
        
        // TODO: Implement WebSocket server using tokio-tungstenite
        // 1. Create TCP listener
        // 2. Accept connections
        // 3. Handle sync protocol over WebSocket
        
        Ok(())
    }
    
    /// Start periodic sync with all peers
    async fn start_periodic_sync(&self) {
        let interval_secs = self.config.sync_interval_secs;
        let p2p_clone = Arc::new(self.clone_weak());
        
        tokio::spawn(async move {
            let mut interval = tokio::time::interval(
                tokio::time::Duration::from_secs(interval_secs)
            );
            
            loop {
                interval.tick().await;
                
                tracing::debug!("Running periodic P2P sync");
                
                if let Err(e) = p2p_clone.sync_all().await {
                    tracing::error!(error = %e, "Periodic sync failed");
                }
            }
        });
    }
    
    /// Create a weak clone for background tasks
    fn clone_weak(&self) -> Self {
        Self {
            local_db: Arc::clone(&self.local_db),
            config: self.config.clone(),
            peers: Arc::clone(&self.peers),
            node_id: self.node_id,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::local_db::LocalDbConfig;
    use tempfile::NamedTempFile;
    
    async fn create_test_db() -> Arc<LocalDatabase> {
        let temp_file = NamedTempFile::new().unwrap();
        let db_path = temp_file.path().to_str().unwrap().to_string();
        
        let config = LocalDbConfig {
            db_path,
            node_id: Uuid::new_v4(),
            max_connections: 5,
            enable_wal: true,
        };
        
        Arc::new(LocalDatabase::new(config).await.unwrap())
    }
    
    #[tokio::test]
    async fn test_p2p_creation() {
        let local_db = create_test_db().await;
        let config = P2PConfig::default();
        
        let p2p = P2PSync::new(local_db, config);
        assert!(!p2p.node_id.is_nil());
    }
    
    #[tokio::test]
    async fn test_add_peer() {
        let local_db = create_test_db().await;
        let config = P2PConfig::default();
        let p2p = P2PSync::new(local_db, config);
        
        let addr: SocketAddr = "192.168.1.100:9876".parse().unwrap();
        p2p.add_peer(addr, "Test Peer".to_string()).await.unwrap();
        
        let peers = p2p.get_peers().await;
        assert_eq!(peers.len(), 1);
        assert_eq!(peers[0].name, "Test Peer");
    }
    
    #[tokio::test]
    async fn test_get_empty_peers() {
        let local_db = create_test_db().await;
        let config = P2PConfig::default();
        let p2p = P2PSync::new(local_db, config);
        
        let peers = p2p.get_peers().await;
        assert_eq!(peers.len(), 0);
    }
    
    #[tokio::test]
    async fn test_sync_all_no_peers() {
        let local_db = create_test_db().await;
        let config = P2PConfig::default();
        let p2p = P2PSync::new(local_db, config);
        
        let stats = p2p.sync_all().await.unwrap();
        assert_eq!(stats.pulled_operations, 0);
        assert_eq!(stats.pushed_operations, 0);
    }
}
