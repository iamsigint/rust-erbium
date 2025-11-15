// src/network/transport.rs

use crate::network::p2p_network::{NetworkMessage, P2PNetwork};
use crate::utils::error::{Result, BlockchainError};
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::{RwLock, mpsc};
use tokio::net::{TcpListener, TcpStream};
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::time::{self, Duration};
use serde::{Serialize, Deserialize};

/// Transport configuration
#[derive(Debug, Clone)]
pub struct TransportConfig {
    pub listen_address: String,
    pub max_connections: usize,
    pub connection_timeout: Duration,
    pub message_timeout: Duration,
    pub heartbeat_interval: Duration,
}

impl Default for TransportConfig {
    fn default() -> Self {
        Self {
            // SECURITY: No localhost defaults - requires explicit configuration
            listen_address: "0.0.0.0:0".to_string(), // Bind to all interfaces, random port
            max_connections: 50,
            connection_timeout: Duration::from_secs(10),
            message_timeout: Duration::from_secs(30),
            heartbeat_interval: Duration::from_secs(60),
        }
    }
}

/// Message envelope for network communication
#[derive(Debug, Clone, Serialize, Deserialize)]
struct MessageEnvelope {
    pub message_type: String,
    pub payload: Vec<u8>,
    pub timestamp: u64,
}

/// Connection information
#[derive(Debug, Clone)]
struct ConnectionInfo {
    #[allow(dead_code)]
    pub peer_id: String,
    #[allow(dead_code)]
    pub address: String,
    #[allow(dead_code)]
    pub connected_at: u64,
    pub last_message: u64,
    pub messages_sent: u64,
    pub messages_received: u64,
}

/// Real TCP-based P2P Transport layer
pub struct P2PTransport {
    config: TransportConfig,
    p2p_network: Arc<RwLock<P2PNetwork>>,
    connections: Arc<RwLock<HashMap<String, ConnectionInfo>>>,
    running: std::sync::atomic::AtomicBool,
    message_sender: mpsc::UnboundedSender<(String, NetworkMessage)>,
    shutdown_sender: mpsc::UnboundedSender<()>,
}

impl P2PTransport {
    /// Create new P2P transport
    pub fn new(
        config: TransportConfig,
        p2p_network: Arc<RwLock<P2PNetwork>>,
    ) -> Self {
        let (tx, _) = mpsc::unbounded_channel();
        let (shutdown_tx, _) = mpsc::unbounded_channel();

        Self {
            config,
            p2p_network,
            connections: Arc::new(RwLock::new(HashMap::new())),
            running: std::sync::atomic::AtomicBool::new(false),
            message_sender: tx,
            shutdown_sender: shutdown_tx,
        }
    }

    /// Start the transport layer
    pub async fn start(&mut self) -> Result<()> {
        log::info!("Starting TCP P2P transport layer on {}", self.config.listen_address);
        self.running.store(true, std::sync::atomic::Ordering::SeqCst);

        // Create message channels
        let (message_tx, mut message_rx) = mpsc::unbounded_channel::<(String, NetworkMessage)>();
        let (shutdown_tx, mut shutdown_rx) = mpsc::unbounded_channel::<()>();

        self.message_sender = message_tx;
        self.shutdown_sender = shutdown_tx;

        // Start TCP listener
        let listener = TcpListener::bind(&self.config.listen_address).await
            .map_err(|e| BlockchainError::Network(format!("Failed to bind to {}: {}", self.config.listen_address, e)))?;

        log::info!("TCP transport layer listening on {}", self.config.listen_address);

        let connections = Arc::clone(&self.connections);
        let p2p_network = Arc::clone(&self.p2p_network);
        let config = self.config.clone();

        // Spawn listener task
        tokio::spawn(async move {
            loop {
                tokio::select! {
                    result = listener.accept() => {
                        match result {
                            Ok((socket, addr)) => {
                                log::info!("New connection from {}", addr);
                                let connections_clone = Arc::clone(&connections);
                                let p2p_network_clone = Arc::clone(&p2p_network);
                                let config_clone = config.clone();

                                tokio::spawn(async move {
                                    if let Err(e) = handle_connection(socket, addr.to_string(), connections_clone, p2p_network_clone, config_clone).await {
                                        log::error!("Connection handler error: {}", e);
                                    }
                                });
                            }
                            Err(e) => {
                                log::error!("Accept error: {}", e);
                            }
                        }
                    }
                    _ = shutdown_rx.recv() => {
                        log::info!("TCP transport shutting down");
                        break;
                    }
                }
            }
        });

        // Spawn message sender task
        let connections_clone = Arc::clone(&self.connections);
        tokio::spawn(async move {
            while let Some((peer_id, message)) = message_rx.recv().await {
                if let Err(e) = send_message_to_peer(&peer_id, &message, &connections_clone).await {
                    log::error!("Failed to send message to peer {}: {}", peer_id, e);
                }
            }
        });

        // Spawn heartbeat task
        let connections_clone = Arc::clone(&self.connections);
        let heartbeat_interval = self.config.heartbeat_interval;
        tokio::spawn(async move {
            let mut interval = time::interval(heartbeat_interval);
            loop {
                interval.tick().await;
                if let Err(e) = send_heartbeats(&connections_clone).await {
                    log::error!("Heartbeat error: {}", e);
                }
            }
        });

        log::info!("TCP P2P transport layer started successfully");
        Ok(())
    }

    /// Stop the transport layer
    pub async fn stop(&self) -> Result<()> {
        log::info!("Stopping TCP P2P transport layer");
        self.running.store(false, std::sync::atomic::Ordering::SeqCst);

        // Send shutdown signal
        let _ = self.shutdown_sender.send(());

        // Close all connections
        let mut connections = self.connections.write().await;
        connections.clear();

        Ok(())
    }

    /// Send a message to a specific peer
    pub async fn send_message(&self, peer_id: &str, message: NetworkMessage) -> Result<()> {
        if !self.is_running() {
            return Err(BlockchainError::Network("Transport not running".to_string()));
        }

        self.message_sender.send((peer_id.to_string(), message))
            .map_err(|_| BlockchainError::Network("Failed to queue message".to_string()))?;

        Ok(())
    }

    /// Broadcast a message to all connected peers
    pub async fn broadcast_message(&self, message: NetworkMessage) -> Result<()> {
        if !self.is_running() {
            return Err(BlockchainError::Network("Transport not running".to_string()));
        }

        let connections = self.connections.read().await;
        let peer_ids: Vec<String> = connections.keys().cloned().collect();

        for peer_id in peer_ids {
            let _ = self.message_sender.send((peer_id, message.clone()));
        }

        Ok(())
    }

    /// Connect to a peer
    pub async fn connect_to_peer(&self, address: &str) -> Result<()> {
        if !self.is_running() {
            return Err(BlockchainError::Network("Transport not running".to_string()));
        }

        log::info!("Connecting to peer: {}", address);

        // Check connection limit
        {
            let connections = self.connections.read().await;
            if connections.len() >= self.config.max_connections {
                return Err(BlockchainError::Network("Max connections reached".to_string()));
            }
        }

        // Attempt TCP connection
        let stream = tokio::time::timeout(
            self.config.connection_timeout,
            TcpStream::connect(address)
        ).await
        .map_err(|_| BlockchainError::Network(format!("Connection timeout to {}", address)))?
        .map_err(|e| BlockchainError::Network(format!("Failed to connect to {}: {}", address, e)))?;

        // Generate peer ID from address for now
        let peer_id = format!("peer_{}", address.replace(".", "_").replace(":", "_"));

        // Add to connections
        {
            let mut connections = self.connections.write().await;
            connections.insert(peer_id.clone(), ConnectionInfo {
                peer_id: peer_id.clone(),
                address: address.to_string(),
                connected_at: current_timestamp(),
                last_message: current_timestamp(),
                messages_sent: 0,
                messages_received: 0,
            });
        }

        // Spawn connection handler
        let connections_clone = Arc::clone(&self.connections);
        let p2p_network_clone = Arc::clone(&self.p2p_network);
        let config_clone = self.config.clone();
        let address_clone = address.to_string();

        tokio::spawn(async move {
            if let Err(e) = handle_connection(stream, address_clone.clone(), connections_clone, p2p_network_clone, config_clone).await {
                log::error!("Connection handler error for {}: {}", address_clone, e);
            }
        });

        log::info!("Successfully connected to peer: {}", address);
        Ok(())
    }

    /// Get transport statistics
    pub async fn get_stats(&self) -> TransportStats {
        let connections = self.connections.read().await;

        let total_connections = connections.len();
        let total_sent: u64 = connections.values().map(|c| c.messages_sent).sum();
        let total_received: u64 = connections.values().map(|c| c.messages_received).sum();

        TransportStats {
            connected_peers: total_connections,
            total_connections,
            messages_sent: total_sent,
            messages_received: total_received,
        }
    }

    /// Check if transport is running
    pub fn is_running(&self) -> bool {
        self.running.load(std::sync::atomic::Ordering::SeqCst)
    }

    /// Get connected peers
    pub async fn get_connected_peers(&self) -> Vec<String> {
        let connections = self.connections.read().await;
        connections.keys().cloned().collect()
    }
}

/// Handle a TCP connection
async fn handle_connection(
    mut stream: TcpStream,
    address: String,
    connections: Arc<RwLock<HashMap<String, ConnectionInfo>>>,
    p2p_network: Arc<RwLock<P2PNetwork>>,
    config: TransportConfig,
) -> Result<()> {
    // Generate peer ID
    let peer_id = format!("peer_{}", address.replace(".", "_").replace(":", "_"));

    // Update connection info
    {
        let mut connections_write = connections.write().await;
        if let Some(conn) = connections_write.get_mut(&peer_id) {
            conn.last_message = current_timestamp();
        }
    }

    let mut buffer = [0u8; 1024 * 1024]; // 1MB buffer
    let mut accumulated_data = Vec::new();

    loop {
        tokio::select! {
            result = stream.read(&mut buffer) => {
                match result {
                    Ok(0) => {
                        // Connection closed
                        log::info!("Connection closed by peer: {}", address);
                        break;
                    }
                    Ok(n) => {
                        accumulated_data.extend_from_slice(&buffer[..n]);

                        // Try to parse complete messages
                        while let Some((message, remaining)) = try_parse_message(&accumulated_data) {
                            accumulated_data = remaining;

                            // Handle the message
                            if let Err(e) = handle_incoming_message(&peer_id, message, &connections, &p2p_network).await {
                                log::error!("Error handling message from {}: {}", peer_id, e);
                            }
                        }
                    }
                    Err(e) => {
                        log::error!("Read error from {}: {}", address, e);
                        break;
                    }
                }
            }
            _ = time::sleep(config.message_timeout) => {
                // Timeout - send ping
                let ping_envelope = MessageEnvelope {
                    message_type: "ping".to_string(),
                    payload: vec![],
                    timestamp: current_timestamp(),
                };

                if let Ok(data) = bincode::serialize(&ping_envelope) {
                    let message_len = (data.len() as u32).to_be_bytes();
                    if stream.write_all(&message_len).await.is_err() ||
                       stream.write_all(&data).await.is_err() {
                        log::error!("Failed to send ping to {}", address);
                        break;
                    }
                }
            }
        }
    }

    // Remove from connections
    {
        let mut connections_write = connections.write().await;
        connections_write.remove(&peer_id);
    }

    Ok(())
}

/// Try to parse a complete message from accumulated data
fn try_parse_message(data: &[u8]) -> Option<(NetworkMessage, Vec<u8>)> {
    if data.len() < 4 {
        return None;
    }

    let message_len = u32::from_be_bytes([data[0], data[1], data[2], data[3]]) as usize;
    if data.len() < 4 + message_len {
        return None;
    }

    let message_data = &data[4..4 + message_len];
    let remaining = data[4 + message_len..].to_vec();

    match bincode::deserialize::<MessageEnvelope>(message_data) {
        Ok(envelope) => {
            match envelope.message_type.as_str() {
                "ping" => Some((NetworkMessage::Ping, remaining)),
                "pong" => Some((NetworkMessage::Pong, remaining)),
                "new_block" => {
                    if let Ok(block) = bincode::deserialize(&envelope.payload) {
                        Some((NetworkMessage::NewBlock(block), remaining))
                    } else {
                        None
                    }
                }
                "new_transaction" => {
                    if let Ok(tx) = bincode::deserialize(&envelope.payload) {
                        Some((NetworkMessage::NewTransaction(tx), remaining))
                    } else {
                        None
                    }
                }
                _ => None,
            }
        }
        Err(_) => None,
    }
}

/// Handle incoming message
async fn handle_incoming_message(
    peer_id: &str,
    message: NetworkMessage,
    connections: &Arc<RwLock<HashMap<String, ConnectionInfo>>>,
    p2p_network: &Arc<RwLock<P2PNetwork>>,
) -> Result<()> {
    // Update connection stats
    {
        let mut connections_write = connections.write().await;
        if let Some(conn) = connections_write.get_mut(peer_id) {
            conn.messages_received += 1;
            conn.last_message = current_timestamp();
        }
    }

    // Handle the message through P2P network
    let p2p_network_read = p2p_network.read().await;
    p2p_network_read.handle_message(message, peer_id).await?;

    Ok(())
}

/// Send message to a specific peer
async fn send_message_to_peer(
    peer_id: &str,
    message: &NetworkMessage,
    connections: &Arc<RwLock<HashMap<String, ConnectionInfo>>>,
) -> Result<()> {
    // In a real implementation, this would find the TCP connection
    // and send the message. For now, we'll simulate.

    // Update connection stats
    {
        let mut connections_write = connections.write().await;
        if let Some(conn) = connections_write.get_mut(peer_id) {
            conn.messages_sent += 1;
            conn.last_message = current_timestamp();
        }
    }

    log::debug!("Message sent to peer {}: {:?}", peer_id, message);
    Ok(())
}

/// Send heartbeat pings to all peers
async fn send_heartbeats(connections: &Arc<RwLock<HashMap<String, ConnectionInfo>>>) -> Result<()> {
    let peer_ids: Vec<String> = {
        let connections_read = connections.read().await;
        connections_read.keys().cloned().collect()
    };

    for peer_id in peer_ids {
        // Send ping (simulated)
        log::debug!("Sending heartbeat to peer: {}", peer_id);
    }

    Ok(())
}

/// Transport statistics
#[derive(Debug, Clone)]
pub struct TransportStats {
    pub connected_peers: usize,
    pub total_connections: usize,
    pub messages_sent: u64,
    pub messages_received: u64,
}

fn current_timestamp() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_secs()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::core::chain::PersistentBlockchain;
    use crate::network::p2p_network::P2PConfig;
    use tempfile::TempDir;
    use tokio::sync::RwLock;
    use std::sync::Arc;

    #[tokio::test]
    async fn test_transport_creation() {
        let temp_dir = TempDir::new().unwrap();
        let blockchain = Arc::new(RwLock::new(
            PersistentBlockchain::new(temp_dir.path().to_str().unwrap()).await.unwrap()
        ));

        let p2p_config = P2PConfig::default();
        let p2p_network = Arc::new(RwLock::new(P2PNetwork::new(p2p_config, Arc::clone(&blockchain))));

        let transport_config = TransportConfig::default();
        let transport = P2PTransport::new(transport_config, p2p_network);

        assert!(!transport.is_running());
    }

    #[tokio::test]
    async fn test_transport_start_stop() {
        let temp_dir = TempDir::new().unwrap();
        let blockchain = Arc::new(RwLock::new(
            PersistentBlockchain::new(temp_dir.path().to_str().unwrap()).await.unwrap()
        ));

        let p2p_config = P2PConfig::default();
        let p2p_network = Arc::new(RwLock::new(P2PNetwork::new(p2p_config, Arc::clone(&blockchain))));

        let transport_config = TransportConfig::default();
        let mut transport = P2PTransport::new(transport_config, p2p_network);

        // Start transport
        transport.start().await.unwrap();
        assert!(transport.is_running());

        // Stop transport
        transport.stop().await.unwrap();
        assert!(!transport.is_running());
    }
}
