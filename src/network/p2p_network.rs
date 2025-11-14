// src/network/p2p_network.rs

use crate::core::{Block, Transaction};

use crate::core::chain::PersistentBlockchain;
use crate::utils::error::{Result, BlockchainError};
use std::collections::{HashMap, HashSet};
use std::sync::Arc;
use tokio::sync::{RwLock, mpsc};
use tokio::time::{self, Duration};
use serde::{Deserialize, Serialize};

/// Network message types
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum NetworkMessage {
    /// New block announcement
    NewBlock(Block),
    /// New transaction announcement
    NewTransaction(Transaction),
    /// Request for blocks
    BlockRequest { start_height: u64, count: u32 },
    /// Response with blocks
    BlockResponse(Vec<Block>),
    /// Request for transactions
    TransactionRequest(Vec<String>), // Transaction hashes
    /// Response with transactions
    TransactionResponse(Vec<Transaction>),
    /// Peer status update
    PeerStatus { height: u64, peers: u32 },
    /// Ping message
    Ping,
    /// Pong response
    Pong,
}

/// Peer information
#[derive(Debug, Clone)]
pub struct PeerInfo {
    pub peer_id: String,
    pub address: String,
    pub last_seen: u64,
    pub best_height: u64,
    pub connected: bool,
}

/// P2P Network configuration
#[derive(Debug, Clone)]
pub struct P2PConfig {
    pub listen_address: String,
    pub max_peers: usize,
    pub ping_interval: Duration,
    pub sync_interval: Duration,
    pub block_announce_interval: Duration,
}

impl Default for P2PConfig {
    fn default() -> Self {
        Self {
            listen_address: "/ip4/0.0.0.0/tcp/0".to_string(),
            max_peers: 50,
            ping_interval: Duration::from_secs(30),
            sync_interval: Duration::from_secs(10),
            block_announce_interval: Duration::from_millis(100),
        }
    }
}

/// P2P Network manager
pub struct P2PNetwork {
    config: P2PConfig,
    blockchain: Arc<RwLock<PersistentBlockchain>>,
    peers: Arc<RwLock<HashMap<String, PeerInfo>>>,
    known_blocks: Arc<RwLock<HashSet<String>>>, // Block hashes we know about
    known_txs: Arc<RwLock<HashSet<String>>>, // Transaction hashes we know about
    message_sender: mpsc::UnboundedSender<NetworkMessage>,
    message_receiver: Arc<RwLock<mpsc::UnboundedReceiver<NetworkMessage>>>,
}

impl P2PNetwork {
    /// Create new P2P network
    pub fn new(
        config: P2PConfig,
        blockchain: Arc<RwLock<PersistentBlockchain>>,
    ) -> Self {
        let (tx, rx) = mpsc::unbounded_channel();

        Self {
            config,
            blockchain,
            peers: Arc::new(RwLock::new(HashMap::new())),
            known_blocks: Arc::new(RwLock::new(HashSet::new())),
            known_txs: Arc::new(RwLock::new(HashSet::new())),
            message_sender: tx,
            message_receiver: Arc::new(RwLock::new(rx)),
        }
    }

    /// Start the P2P network
    pub async fn start(&self) -> Result<()> {
        log::info!("Starting P2P network on {}", self.config.listen_address);

        // Initialize known blocks and transactions from blockchain
        self.initialize_known_data().await?;

        // Start background tasks
        self.start_background_tasks();

        log::info!("P2P network started successfully");
        Ok(())
    }

    /// Stop the P2P network
    pub async fn stop(&self) -> Result<()> {
        log::info!("Stopping P2P network");
        // Close channels and cleanup
        Ok(())
    }

    /// Broadcast a new block to all peers
    pub async fn broadcast_block(&self, block: &Block) -> Result<()> {
        let block_hash = block.hash().to_hex();

        // Check if we already know about this block
        {
            let known_blocks = self.known_blocks.read().await;
            if known_blocks.contains(&block_hash) {
                return Ok(());
            }
        }

        // Add to known blocks
        {
            let mut known_blocks = self.known_blocks.write().await;
            known_blocks.insert(block_hash.clone());
        }

        // Create announcement message
        let message = NetworkMessage::NewBlock(block.clone());

        // Send to message channel for processing
        self.message_sender.send(message)
            .map_err(|_| BlockchainError::Network("Failed to send block announcement".to_string()))?;

        log::debug!("Broadcasting block {} to peers", block_hash);
        Ok(())
    }

    /// Broadcast a new transaction to all peers
    pub async fn broadcast_transaction(&self, transaction: &Transaction) -> Result<()> {
        let tx_hash = transaction.hash().to_hex();

        // Check if we already know about this transaction
        {
            let known_txs = self.known_txs.read().await;
            if known_txs.contains(&tx_hash) {
                return Ok(());
            }
        }

        // Add to known transactions
        {
            let mut known_txs = self.known_txs.write().await;
            known_txs.insert(tx_hash.clone());
        }

        // Create announcement message
        let message = NetworkMessage::NewTransaction(transaction.clone());

        // Send to message channel for processing
        self.message_sender.send(message)
            .map_err(|_| BlockchainError::Network("Failed to send transaction announcement".to_string()))?;

        log::debug!("Broadcasting transaction {} to peers", tx_hash);
        Ok(())
    }

    /// Request blocks from peers
    pub async fn request_blocks(&self, start_height: u64, count: u32) -> Result<()> {
        let message = NetworkMessage::BlockRequest { start_height, count };

        self.message_sender.send(message)
            .map_err(|_| BlockchainError::Network("Failed to send block request".to_string()))?;

        log::debug!("Requesting {} blocks starting from height {}", count, start_height);
        Ok(())
    }

    /// Handle incoming network message
    pub async fn handle_message(&self, message: NetworkMessage, peer_id: &str) -> Result<()> {
        match message {
            NetworkMessage::NewBlock(block) => {
                self.handle_new_block(block, peer_id).await?;
            }
            NetworkMessage::NewTransaction(tx) => {
                self.handle_new_transaction(tx, peer_id).await?;
            }
            NetworkMessage::BlockRequest { start_height, count } => {
                self.handle_block_request(start_height, count, peer_id).await?;
            }
            NetworkMessage::BlockResponse(blocks) => {
                self.handle_block_response(blocks, peer_id).await?;
            }
            NetworkMessage::TransactionRequest(hashes) => {
                self.handle_transaction_request(hashes, peer_id).await?;
            }
            NetworkMessage::TransactionResponse(txs) => {
                self.handle_transaction_response(txs, peer_id).await?;
            }
            NetworkMessage::PeerStatus { height, peers } => {
                self.handle_peer_status(peer_id, height, peers).await?;
            }
            NetworkMessage::Ping => {
                self.handle_ping(peer_id).await?;
            }
            NetworkMessage::Pong => {
                // Pong received, update peer last seen
                self.update_peer_last_seen(peer_id).await?;
            }
        }

        Ok(())
    }

    /// Handle new block from peer
    async fn handle_new_block(&self, block: Block, peer_id: &str) -> Result<()> {
        let block_hash = block.hash().to_hex();

        // Check if we already know about this block
        {
            let known_blocks = self.known_blocks.read().await;
            if known_blocks.contains(&block_hash) {
                return Ok(());
            }
        }

        // Add to known blocks
        {
            let mut known_blocks = self.known_blocks.write().await;
            known_blocks.insert(block_hash.clone());
        }

        // Try to add block to blockchain
        let blockchain = self.blockchain.read().await;
        let current_height = blockchain.get_latest_height().await?;

        if block.header.number > current_height + 1 {
            // We're missing blocks, request them
            self.request_blocks(current_height + 1, (block.header.number - current_height) as u32).await?;
        } else if block.header.number == current_height + 1 {
            // This is the next block, try to add it
            drop(blockchain);
            let mut blockchain_write = self.blockchain.write().await;
            match blockchain_write.add_block(block.clone()).await {
                Ok(_) => {
                    log::info!("Added new block {} from peer {}", block_hash, peer_id);
                    // Broadcast to other peers
                    self.broadcast_block(&block).await?;
                }
                Err(e) => {
                    log::warn!("Failed to add block {} from peer {}: {}", block_hash, peer_id, e);
                }
            }
        }

        Ok(())
    }

    /// Handle new transaction from peer
    async fn handle_new_transaction(&self, transaction: Transaction, peer_id: &str) -> Result<()> {
        let tx_hash = transaction.hash().to_hex();

        // Check if we already know about this transaction
        {
            let known_txs = self.known_txs.read().await;
            if known_txs.contains(&tx_hash) {
                return Ok(());
            }
        }

        // Add to known transactions
        {
            let mut known_txs = self.known_txs.write().await;
            known_txs.insert(tx_hash.clone());
        }

        // For now, just log the transaction
        // In a full implementation, this would add to mempool
        log::debug!("Received transaction {} from peer {}", tx_hash, peer_id);

        // Broadcast to other peers
        self.broadcast_transaction(&transaction).await?;

        Ok(())
    }

    /// Handle block request from peer
    async fn handle_block_request(&self, start_height: u64, count: u32, peer_id: &str) -> Result<()> {
        let mut blocks = Vec::new();

        for i in 0..count {
            let height = start_height + i as u64;
            if let Ok(Some(block)) = self.blockchain.read().await.get_block_by_height(height).await {
                blocks.push(block);
            } else {
                break; // No more blocks
            }
        }

        if !blocks.is_empty() {
            let response = NetworkMessage::BlockResponse(blocks.clone());
            self.message_sender.send(response)
                .map_err(|_| BlockchainError::Network("Failed to send block response".to_string()))?;
        }

        log::debug!("Sent {} blocks to peer {} starting from height {}", blocks.len(), peer_id, start_height);
        Ok(())
    }

    /// Handle block response from peer
    async fn handle_block_response(&self, blocks: Vec<Block>, peer_id: &str) -> Result<()> {
        for block in &blocks {
            // Try to add each block
            let mut blockchain = self.blockchain.write().await;
            if let Err(e) = blockchain.add_block(block.clone()).await {
                log::warn!("Failed to add block from peer {}: {}", peer_id, e);
                break; // Stop processing if one fails
            }
        }

        log::debug!("Processed {} blocks from peer {}", blocks.len(), peer_id);
        Ok(())
    }

    /// Handle transaction request from peer
    async fn handle_transaction_request(&self, hashes: Vec<String>, _peer_id: &str) -> Result<()> {
        let mut transactions = Vec::new();

        for hash in hashes {
            if let Ok(Some(tx)) = self.blockchain.read().await.get_transaction_by_hash(hash.as_bytes()).await {
                transactions.push(tx);
            }
        }

        if !transactions.is_empty() {
            let response = NetworkMessage::TransactionResponse(transactions);
            self.message_sender.send(response)
                .map_err(|_| BlockchainError::Network("Failed to send transaction response".to_string()))?;
        }

        Ok(())
    }

    /// Handle transaction response from peer
    async fn handle_transaction_response(&self, transactions: Vec<Transaction>, peer_id: &str) -> Result<()> {
        // For now, just log the transactions
        // In a full implementation, this would add to mempool
        log::debug!("Received {} transactions from peer {}", transactions.len(), peer_id);
        Ok(())
    }

    /// Handle peer status update
    async fn handle_peer_status(&self, peer_id: &str, height: u64, peer_count: u32) -> Result<()> {
        let mut peers = self.peers.write().await;
        if let Some(peer) = peers.get_mut(peer_id) {
            peer.best_height = height;
            peer.last_seen = current_timestamp();
        }

        log::debug!("Peer {} status: height={}, peers={}", peer_id, height, peer_count);
        Ok(())
    }

    /// Handle ping from peer
    async fn handle_ping(&self, peer_id: &str) -> Result<()> {
        // Send pong response
        let pong = NetworkMessage::Pong;
        self.message_sender.send(pong)
            .map_err(|_| BlockchainError::Network("Failed to send pong".to_string()))?;

        // Update peer last seen
        self.update_peer_last_seen(peer_id).await?;

        Ok(())
    }

    /// Update peer last seen timestamp
    async fn update_peer_last_seen(&self, peer_id: &str) -> Result<()> {
        let mut peers = self.peers.write().await;
        if let Some(peer) = peers.get_mut(peer_id) {
            peer.last_seen = current_timestamp();
        }
        Ok(())
    }

    /// Initialize known blocks and transactions from blockchain
    async fn initialize_known_data(&self) -> Result<()> {
        // Get latest height
        let latest_height = self.blockchain.read().await.get_latest_height().await?;

        // For now, we'll initialize with some basic data
        // In a full implementation, this would scan the blockchain
        log::info!("Initialized P2P network with blockchain height: {}", latest_height);

        Ok(())
    }

    /// Start background network tasks
    fn start_background_tasks(&self) {
        let peers = Arc::clone(&self.peers);
        let message_receiver = Arc::clone(&self.message_receiver);
        let ping_interval = self.config.ping_interval;

        // Ping task
        tokio::spawn(async move {
            let mut interval = time::interval(ping_interval);
            loop {
                interval.tick().await;

                // Send ping to all connected peers
                let peer_ids: Vec<String> = {
                    let peers_read = peers.read().await;
                    peers_read.keys().cloned().collect()
                };

                for peer_id in peer_ids {
                    // Send ping message
                    // In a real implementation, this would use the actual network transport
                    log::debug!("Pinging peer {}", peer_id);
                }
            }
        });

        // Message processing task
        let message_receiver_clone = Arc::clone(&message_receiver);
        tokio::spawn(async move {
            let mut receiver = message_receiver_clone.write().await;
            while let Some(message) = receiver.recv().await {
                // Process message
                // In a real implementation, this would route to appropriate handlers
                log::debug!("Processing network message: {:?}", message);
            }
        });
    }

    /// Get network statistics
    pub async fn get_stats(&self) -> Result<NetworkStats> {
        let peers = self.peers.read().await;
        let known_blocks = self.known_blocks.read().await;
        let known_txs = self.known_txs.read().await;

        Ok(NetworkStats {
            connected_peers: peers.values().filter(|p| p.connected).count(),
            total_peers: peers.len(),
            known_blocks: known_blocks.len(),
            known_transactions: known_txs.len(),
        })
    }

    /// Add a peer to the network
    pub async fn add_peer(&self, peer_info: PeerInfo) -> Result<()> {
        let mut peers = self.peers.write().await;
        peers.insert(peer_info.peer_id.clone(), peer_info);
        log::info!("Added peer to network");
        Ok(())
    }

    /// Remove a peer from the network
    pub async fn remove_peer(&self, peer_id: &str) -> Result<()> {
        let mut peers = self.peers.write().await;
        peers.remove(peer_id);
        log::info!("Removed peer {} from network", peer_id);
        Ok(())
    }
}

/// Network statistics
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NetworkStats {
    pub connected_peers: usize,
    pub total_peers: usize,
    pub known_blocks: usize,
    pub known_transactions: usize,
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
    use tempfile::TempDir;

    #[tokio::test]
    async fn test_p2p_network_creation() {
        let temp_dir = TempDir::new().unwrap();
        let blockchain = Arc::new(RwLock::new(
            PersistentBlockchain::new(temp_dir.path().to_str().unwrap()).await.unwrap()
        ));

        let config = P2PConfig::default();
        let network = P2PNetwork::new(config, blockchain);

        let stats = network.get_stats().await.unwrap();
        assert_eq!(stats.connected_peers, 0);
        assert_eq!(stats.total_peers, 0);
    }
}
