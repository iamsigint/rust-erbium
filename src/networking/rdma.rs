//! RDMA (Remote Direct Memory Access) networking for ultra-low latency communication
//!
//! This module provides RDMA-based networking capabilities for high-performance
//! node-to-node communication in distributed blockchain systems.

use crate::utils::error::{Result, BlockchainError};
use serde::{Serialize, Deserialize};
use std::collections::{HashMap, VecDeque};
use std::sync::Arc;
use tokio::sync::{RwLock, mpsc};
use std::net::SocketAddr;
use std::time::{SystemTime, UNIX_EPOCH, Duration};

/// RDMA configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RdmaConfig {
    pub enable_rdma: bool,
    pub max_connections: usize,
    pub buffer_size_mb: usize,
    pub max_message_size: usize,
    pub connection_timeout_ms: u64,
    pub heartbeat_interval_ms: u64,
    pub enable_zero_copy: bool,
    pub enable_async_operations: bool,
}

impl Default for RdmaConfig {
    fn default() -> Self {
        Self {
            enable_rdma: true,
            max_connections: 1000,
            buffer_size_mb: 256, // 256MB per connection
            max_message_size: 64 * 1024 * 1024, // 64MB
            connection_timeout_ms: 5000,
            heartbeat_interval_ms: 1000,
            enable_zero_copy: true,
            enable_async_operations: true,
        }
    }
}

/// RDMA connection information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RdmaConnection {
    pub peer_addr: SocketAddr,
    pub connection_id: u64,
    pub state: ConnectionState,
    pub last_heartbeat: u64,
    pub bytes_sent: u64,
    pub bytes_received: u64,
    pub latency_us: u64,
    pub queue_depth: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum ConnectionState {
    Connecting,
    Connected,
    Disconnected,
    Failed,
}

/// RDMA message types
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum RdmaMessage {
    /// Blockchain transaction data
    Transaction {
        data: Vec<u8>,
        priority: MessagePriority,
        requires_ack: bool,
    },
    /// Consensus messages
    Consensus {
        block_height: u64,
        data: Vec<u8>,
        priority: MessagePriority,
    },
    /// State synchronization
    StateSync {
        shard_id: u32,
        data: Vec<u8>,
        checksum: [u8; 32],
    },
    /// Heartbeat for connection monitoring
    Heartbeat {
        timestamp: u64,
        node_id: String,
    },
    /// Acknowledgment
    Ack {
        message_id: u64,
        success: bool,
    },
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum MessagePriority {
    Low,
    Normal,
    High,
    Critical,
}

/// RDMA networking manager
pub struct RdmaManager {
    config: RdmaConfig,
    connections: Arc<RwLock<HashMap<SocketAddr, RdmaConnection>>>,
    message_queue: Arc<RwLock<VecDeque<QueuedMessage>>>,
    stats: Arc<RwLock<RdmaStats>>,
    message_sender: mpsc::UnboundedSender<RdmaMessage>,
    message_receiver: Arc<RwLock<mpsc::UnboundedReceiver<RdmaMessage>>>,
}

#[derive(Debug, Clone)]
struct QueuedMessage {
    message: RdmaMessage,
    destination: SocketAddr,
    enqueue_time: u64,
    retry_count: u32,
}

#[derive(Debug, Clone, Default)]
pub struct RdmaStats {
    pub total_connections: usize,
    pub active_connections: usize,
    pub messages_sent: u64,
    pub messages_received: u64,
    pub bytes_sent: u64,
    pub bytes_received: u64,
    pub average_latency_us: u64,
    pub failed_connections: u64,
    pub retransmissions: u64,
}

impl RdmaManager {
    /// Create a new RDMA manager
    pub fn new(config: RdmaConfig) -> Self {
        let (message_sender, message_receiver) = mpsc::unbounded_channel();

        Self {
            config,
            connections: Arc::new(RwLock::new(HashMap::new())),
            message_queue: Arc::new(RwLock::new(VecDeque::new())),
            stats: Arc::new(RwLock::new(RdmaStats::default())),
            message_sender,
            message_receiver: Arc::new(RwLock::new(message_receiver)),
        }
    }

    /// Establish RDMA connection to peer
    pub async fn connect(&self, peer_addr: SocketAddr) -> Result<u64> {
        if !self.config.enable_rdma {
            return Err(BlockchainError::Network("RDMA is disabled".to_string()));
        }

        let mut connections = self.connections.write().await;

        if connections.len() >= self.config.max_connections {
            return Err(BlockchainError::Network("Maximum connections reached".to_string()));
        }

        if connections.contains_key(&peer_addr) {
            return Err(BlockchainError::Network("Connection already exists".to_string()));
        }

        // Simulate RDMA connection establishment
        // In production, this would use RDMA verbs (ibv_* functions)
        let connection_id = generate_connection_id();

        let connection = RdmaConnection {
            peer_addr,
            connection_id,
            state: ConnectionState::Connecting,
            last_heartbeat: current_timestamp(),
            bytes_sent: 0,
            bytes_received: 0,
            latency_us: 0,
            queue_depth: 0,
        };

        connections.insert(peer_addr, connection);

        // Simulate connection establishment time
        tokio::time::sleep(Duration::from_millis(10)).await;

        // Update connection state
        if let Some(conn) = connections.get_mut(&peer_addr) {
            conn.state = ConnectionState::Connected;
        }

        let mut stats = self.stats.write().await;
        stats.total_connections += 1;
        stats.active_connections += 1;

        log::info!("Established RDMA connection to {} (ID: {})", peer_addr, connection_id);
        Ok(connection_id)
    }

    /// Send message via RDMA
    pub async fn send_message(&self, peer_addr: SocketAddr, message: RdmaMessage) -> Result<()> {
        let connections = self.connections.read().await;

        let connection = connections.get(&peer_addr)
            .ok_or_else(|| BlockchainError::Network("Connection not found".to_string()))?;

        if connection.state != ConnectionState::Connected {
            return Err(BlockchainError::Network("Connection not ready".to_string()));
        }

        drop(connections); // Release read lock

        // Check message size
        let message_size = self.estimate_message_size(&message);
        if message_size > self.config.max_message_size {
            return Err(BlockchainError::Network("Message too large".to_string()));
        }

        // Send via RDMA (simulated)
        let start_time = std::time::Instant::now();
        let success = self.send_via_rdma(peer_addr, &message).await?;
        let latency = start_time.elapsed().as_micros() as u64;

        if success {
            // Update statistics
            let mut stats = self.stats.write().await;
            stats.messages_sent += 1;
            stats.bytes_sent += message_size as u64;

            let mut connections = self.connections.write().await;
            if let Some(conn) = connections.get_mut(&peer_addr) {
                conn.bytes_sent += message_size as u64;
                conn.latency_us = (conn.latency_us + latency) / 2; // Rolling average
            }
        } else {
            let mut stats = self.stats.write().await;
            stats.failed_connections += 1;
        }

        Ok(())
    }

    /// Receive messages (called by message processing loop)
    pub async fn receive_messages(&self) -> Result<Vec<(SocketAddr, RdmaMessage)>> {
        let mut messages = Vec::new();

        // In production, this would be event-driven from RDMA completion queues
        // For simulation, we'll check for queued messages

        let mut message_queue = self.message_queue.write().await;
        while let Some(queued) = message_queue.pop_front() {
            messages.push((queued.destination, queued.message));
        }

        // Update stats
        if !messages.is_empty() {
            let mut stats = self.stats.write().await;
            stats.messages_received += messages.len() as u64;
            for (_, message) in &messages {
                stats.bytes_received += self.estimate_message_size(message) as u64;
            }
        }

        Ok(messages)
    }

    /// Send high-priority transaction via RDMA
    pub async fn send_transaction(&self, peer_addr: SocketAddr, transaction_data: Vec<u8>) -> Result<()> {
        let message = RdmaMessage::Transaction {
            data: transaction_data,
            priority: MessagePriority::High,
            requires_ack: true,
        };

        self.send_message(peer_addr, message).await
    }

    /// Send consensus message with critical priority
    pub async fn send_consensus_message(&self, peer_addr: SocketAddr, block_height: u64, data: Vec<u8>) -> Result<()> {
        let message = RdmaMessage::Consensus {
            block_height,
            data,
            priority: MessagePriority::Critical,
        };

        self.send_message(peer_addr, message).await
    }

    /// Send state synchronization data
    pub async fn send_state_sync(&self, peer_addr: SocketAddr, shard_id: u32, data: Vec<u8>) -> Result<()> {
        let checksum = blake3::hash(&data);
        let message = RdmaMessage::StateSync {
            shard_id,
            data,
            checksum: *checksum.as_bytes(),
        };

        self.send_message(peer_addr, message).await
    }

    /// Broadcast message to multiple peers
    pub async fn broadcast_message(&self, peers: &[SocketAddr], message: RdmaMessage) -> Result<()> {
        // Use RDMA multicast when available, otherwise send individually
        if self.supports_multicast() {
            self.send_multicast(peers, &message).await?;
        } else {
            for &peer in peers {
                if let Err(e) = self.send_message(peer, message.clone()).await {
                    log::warn!("Failed to send to {}: {:?}", peer, e);
                }
            }
        }

        Ok(())
    }

    /// Get connection statistics
    pub async fn get_connection_stats(&self, peer_addr: SocketAddr) -> Result<RdmaConnection> {
        let connections = self.connections.read().await;
        connections.get(&peer_addr)
            .cloned()
            .ok_or_else(|| BlockchainError::Network("Connection not found".to_string()))
    }

    /// Get overall RDMA statistics
    pub async fn get_stats(&self) -> Result<RdmaStats> {
        let stats = self.stats.read().await;
        Ok(stats.clone())
    }

    /// Close connection
    pub async fn close_connection(&self, peer_addr: SocketAddr) -> Result<()> {
        let mut connections = self.connections.write().await;

        if let Some(mut connection) = connections.remove(&peer_addr) {
            connection.state = ConnectionState::Disconnected;

            let mut stats = self.stats.write().await;
            stats.active_connections = stats.active_connections.saturating_sub(1);

            log::info!("Closed RDMA connection to {}", peer_addr);
        }

        Ok(())
    }

    /// Perform maintenance operations
    pub async fn perform_maintenance(&self) -> Result<()> {
        let mut connections = self.connections.write().await;
        let current_time = current_timestamp();

        // Check for stale connections
        let stale_connections: Vec<SocketAddr> = connections.iter()
            .filter(|(_, conn)| {
                current_time - conn.last_heartbeat > self.config.connection_timeout_ms
            })
            .map(|(addr, _)| *addr)
            .collect();

        // Remove stale connections
        for addr in stale_connections {
            if let Some(conn) = connections.get_mut(&addr) {
                conn.state = ConnectionState::Failed;
            }

            let mut stats = self.stats.write().await;
            stats.failed_connections += 1;
            stats.active_connections = stats.active_connections.saturating_sub(1);
        }

        // Send heartbeats to active connections
        for (addr, conn) in connections.iter_mut() {
            if conn.state == ConnectionState::Connected &&
               current_time - conn.last_heartbeat >= self.config.heartbeat_interval_ms {
                let heartbeat = RdmaMessage::Heartbeat {
                    timestamp: current_time,
                    node_id: "local_node".to_string(),
                };

                // Queue heartbeat (in production, would send directly)
                let queued = QueuedMessage {
                    message: heartbeat,
                    destination: *addr,
                    enqueue_time: current_time,
                    retry_count: 0,
                };

                let mut message_queue = self.message_queue.write().await;
                message_queue.push_back(queued);

                conn.last_heartbeat = current_time;
            }
        }

        Ok(())
    }

    // Internal methods
    async fn send_via_rdma(&self, peer_addr: SocketAddr, message: &RdmaMessage) -> Result<bool> {
        // Simulate RDMA send operation
        // In production, this would use ibv_post_send with RDMA work requests

        // Simulate network latency (much lower than TCP)
        let latency_us = 50 + (rand::random::<u64>() % 100); // 50-150 microseconds
        tokio::time::sleep(Duration::from_micros(latency_us)).await;

        // Simulate success (in production, would check completion status)
        Ok(true)
    }

    async fn send_multicast(&self, peers: &[SocketAddr], message: &RdmaMessage) -> Result<()> {
        // Simulate RDMA multicast
        // In production, this would use RDMA multicast groups
        tokio::time::sleep(Duration::from_micros(100)).await;

        let mut stats = self.stats.write().await;
        stats.messages_sent += peers.len() as u64;
        stats.bytes_sent += (self.estimate_message_size(message) * peers.len()) as u64;

        Ok(())
    }

    fn supports_multicast(&self) -> bool {
        // In production, check RDMA hardware capabilities
        true // Assume supported for simulation
    }

    fn estimate_message_size(&self, message: &RdmaMessage) -> usize {
        // Rough estimation of serialized message size
        match message {
            RdmaMessage::Transaction { data, .. } => 100 + data.len(),
            RdmaMessage::Consensus { data, .. } => 50 + data.len(),
            RdmaMessage::StateSync { data, .. } => 100 + data.len(),
            RdmaMessage::Heartbeat { .. } => 50,
            RdmaMessage::Ack { .. } => 20,
        }
    }
}

fn generate_connection_id() -> u64 {
    use std::collections::hash_map::DefaultHasher;
    use std::hash::{Hash, Hasher};

    let mut hasher = DefaultHasher::new();
    current_timestamp().hash(&mut hasher);
    rand::random::<u64>().hash(&mut hasher);
    hasher.finish()
}

fn current_timestamp() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_millis() as u64
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::net::{IpAddr, Ipv4Addr};

    #[tokio::test]
    async fn test_rdma_manager_creation() {
        let config = RdmaConfig::default();
        let manager = RdmaManager::new(config);

        assert!(manager.config.enable_rdma);
        assert_eq!(manager.config.max_connections, 1000);
    }

    #[tokio::test]
    async fn test_rdma_connection() {
        let config = RdmaConfig::default();
        let manager = RdmaManager::new(config);

        let peer_addr = SocketAddr::new(IpAddr::V4(Ipv4Addr::new(127, 0, 0, 1)), 8333);
        let connection_id = manager.connect(peer_addr).await.unwrap();

        assert!(connection_id > 0);

        let stats = manager.get_stats().await.unwrap();
        assert_eq!(stats.total_connections, 1);
        assert_eq!(stats.active_connections, 1);
    }

    #[tokio::test]
    async fn test_rdma_message_send() {
        let config = RdmaConfig::default();
        let manager = RdmaManager::new(config);

        let peer_addr = SocketAddr::new(IpAddr::V4(Ipv4Addr::new(127, 0, 0, 1)), 8333);
        manager.connect(peer_addr).await.unwrap();

        let message = RdmaMessage::Transaction {
            data: vec![1, 2, 3, 4],
            priority: MessagePriority::Normal,
            requires_ack: false,
        };

        manager.send_message(peer_addr, message).await.unwrap();

        let stats = manager.get_stats().await.unwrap();
        assert_eq!(stats.messages_sent, 1);
        assert!(stats.bytes_sent > 0);
    }

    #[tokio::test]
    async fn test_rdma_transaction_send() {
        let config = RdmaConfig::default();
        let manager = RdmaManager::new(config);

        let peer_addr = SocketAddr::new(IpAddr::V4(Ipv4Addr::new(127, 0, 0, 1)), 8333);
        manager.connect(peer_addr).await.unwrap();

        let tx_data = vec![0; 1024]; // 1KB transaction
        manager.send_transaction(peer_addr, tx_data).await.unwrap();

        let conn_stats = manager.get_connection_stats(peer_addr).await.unwrap();
        assert!(conn_stats.bytes_sent > 0);
    }

    #[tokio::test]
    async fn test_rdma_maintenance() {
        let config = RdmaConfig::default();
        let manager = RdmaManager::new(config);

        let peer_addr = SocketAddr::new(IpAddr::V4(Ipv4Addr::new(127, 0, 0, 1)), 8333);
        manager.connect(peer_addr).await.unwrap();

        manager.perform_maintenance().await.unwrap();

        // Connection should still be active
        let conn_stats = manager.get_connection_stats(peer_addr).await.unwrap();
        assert_eq!(conn_stats.state, ConnectionState::Connected);
    }

    #[tokio::test]
    async fn test_connection_close() {
        let config = RdmaConfig::default();
        let manager = RdmaManager::new(config);

        let peer_addr = SocketAddr::new(IpAddr::V4(Ipv4Addr::new(127, 0, 0, 1)), 8333);
        manager.connect(peer_addr).await.unwrap();

        manager.close_connection(peer_addr).await.unwrap();

        let stats = manager.get_stats().await.unwrap();
        assert_eq!(stats.active_connections, 0);
    }
}
