// src/network/p2p_network.rs

use crate::core::{Block, Transaction};
use crate::core::chain::PersistentBlockchain;
use crate::utils::error::{Result, BlockchainError};
use crate::network::dht::{DHT, DHTConfig};
use crate::network::discovery::PeerDiscovery;
use std::collections::{HashMap, HashSet};
use std::sync::Arc;
use tokio::sync::{RwLock, mpsc};
use tokio::time::{self, Duration};
use serde::{Deserialize, Serialize};
use std::fs;
use toml;

/// Bootstrap node configuration from TOML file
#[derive(Debug, Deserialize)]
struct BootstrapConfig {
    bootstrap: BootstrapSection,
    config: BootstrapBehavior,
}

#[derive(Debug, Deserialize)]
struct BootstrapSection {
    development_nodes: Vec<String>,
    mainnet_nodes: Vec<String>,
    testnet_nodes: Vec<String>,
}

#[derive(Debug, Deserialize)]
struct BootstrapBehavior {
    use_development_nodes: bool,
    allow_hardcoded_fallback: bool, // TODO: Currently unused - will be used for DHT fallback
    bootstrap_timeout_seconds: u64, // TODO: Currently unused - will be used for bootstrap timeouts
    max_bootstrap_attempts: usize, // TODO: Currently unused - will be used for bootstrap retries
}

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
    // DHT specific configuration
    pub dht_k: usize,       // Kademlia K parameter (bucket size)
    pub dht_alpha: usize,   // Kademlia alpha parameter (parallelism)
    pub dht_refresh_interval: Duration,
    // Bootstrap peers are now loaded from config/bootstrap_nodes.toml
}

impl Default for P2PConfig {
    fn default() -> Self {
        Self {
            listen_address: "/ip4/0.0.0.0/tcp/0".to_string(),
            max_peers: 50,
            ping_interval: Duration::from_secs(30),
            sync_interval: Duration::from_secs(10),
            block_announce_interval: Duration::from_millis(100),
            // DHT defaults (standard Kademlia)
            dht_k: 20,
            dht_alpha: 3,
            dht_refresh_interval: Duration::from_secs(3600), // 1 hour
        }
    }
}

/// P2P Network manager with DHT integration
pub struct P2PNetwork {
    config: P2PConfig,
    blockchain: Arc<RwLock<PersistentBlockchain>>,
    peers: Arc<RwLock<HashMap<String, PeerInfo>>>,
    known_blocks: Arc<RwLock<HashSet<String>>>, // Block hashes we know about
    known_txs: Arc<RwLock<HashSet<String>>>, // Transaction hashes we know about
    message_sender: mpsc::UnboundedSender<NetworkMessage>,
    message_receiver: Arc<RwLock<mpsc::UnboundedReceiver<NetworkMessage>>>,
    // DHT integration for decentralized peer discovery
    dht: Arc<RwLock<DHT>>,           // Kademlia DHT instance
    peer_discovery: Arc<RwLock<PeerDiscovery>>, // Traditional peer management
}

impl P2PNetwork {
    /// Create new P2P network with DHT integration
    pub fn new_with_dht(
        config: P2PConfig,
        blockchain: Arc<RwLock<PersistentBlockchain>>,
    ) -> Result<Self> {
        let (tx, rx) = mpsc::unbounded_channel();

        // Create DHT instance with configuration
        let dht_config = DHTConfig {
            k: config.dht_k,
            alpha: config.dht_alpha,
            id_bits: 160,
            refresh_interval: config.dht_refresh_interval,
            republish_interval: Duration::from_secs(86400),
            expire_interval: Duration::from_secs(86400),
        };

        let dht = DHT::new(dht_config);
        let peer_discovery = PeerDiscovery::new(vec![]); // Initialize empty, will populate

        Ok(Self {
            config,
            blockchain,
            peers: Arc::new(RwLock::new(HashMap::new())),
            known_blocks: Arc::new(RwLock::new(HashSet::new())),
            known_txs: Arc::new(RwLock::new(HashSet::new())),
            message_sender: tx,
            message_receiver: Arc::new(RwLock::new(rx)),
            dht: Arc::new(RwLock::new(dht)),
            peer_discovery: Arc::new(RwLock::new(peer_discovery)),
        })
    }

    /// Create new P2P network (legacy constructor)
    pub fn new(
        config: P2PConfig,
        blockchain: Arc<RwLock<PersistentBlockchain>>,
    ) -> Self {
        // For now, forward to new_with_dht but ignore errors
        // In a real implementation, this might create a stub DHT
        match Self::new_with_dht(config, blockchain) {
            Ok(network) => network,
            Err(_) => panic!("Failed to create P2P network with DHT"),
        }
    }

    /// Start the P2P network
    pub async fn start(&self) -> Result<()> {
        log::info!("Starting P2P network on {}", self.config.listen_address);

        // Initialize DHT with bootstrap nodes (Customer Requirement: DHT integration)
        self.initialize_dht_bootstrap().await?;

        // Initialize known blocks and transactions from blockchain
        self.initialize_known_data().await?;

        // Start background tasks
        self.start_background_tasks();

        log::info!("P2P network started successfully with DHT integration");
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
        let _message_receiver = Arc::clone(&self.message_receiver);
        let ping_interval = self.config.ping_interval;
        let dht = Arc::clone(&self.dht);
        let dht_refresh_interval = self.config.dht_refresh_interval;

        // DHT background tasks
        self.start_dht_operations(dht.clone(), dht_refresh_interval);

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
        let message_receiver_clone = Arc::clone(&self.message_receiver);
        tokio::spawn(async move {
            let mut receiver = message_receiver_clone.write().await;
            while let Some(message) = receiver.recv().await {
                // Process message
                // In a real implementation, this would route to appropriate handlers
                log::debug!("Processing network message: {:?}", message);
            }
        });
    }

    /// Start DHT background operations
    fn start_dht_operations(&self, dht: Arc<RwLock<DHT>>, refresh_interval: Duration) {
        // Periodic DHT operations
        let dht_clone = Arc::clone(&dht);
        let dht_lookup_interval = Duration::from_secs(900); // 15 minutes
        tokio::spawn(async move {
            let mut lookup_interval = time::interval(dht_lookup_interval);
            loop {
                lookup_interval.tick().await;

                // Perform periodic DHT operations
                if let Err(e) = Self::perform_dht_maintenance(dht_clone.clone()).await {
                    log::error!("DHT maintenance failed: {}", e);
                }
            }
        });

        // DHT bucket refresh
        let dht_clone = Arc::clone(&dht);
        tokio::spawn(async move {
            let mut refresh_interval_timer = time::interval(refresh_interval);
            loop {
                refresh_interval_timer.tick().await;

                // Refresh k-buckets
                if let Err(e) = Self::refresh_dht_buckets(dht_clone.clone()).await {
                    log::error!("DHT bucket refresh failed: {}", e);
                }
            }
        });

        // Presence announcement
        let dht_clone = Arc::clone(&dht);
        tokio::spawn(async move {
            let mut announce_interval = time::interval(Duration::from_secs(300)); // 5 minutes
            loop {
                announce_interval.tick().await;

                // Announce our presence to the network
                if let Err(e) = Self::announce_presence(dht_clone.clone()).await {
                    log::error!("Presence announcement failed: {}", e);
                }
            }
        });
    }

    /// Perform periodic DHT maintenance operations
    async fn perform_dht_maintenance(dht: Arc<RwLock<DHT>>) -> Result<()> {
        let mut dht_guard = dht.write().await;

        // Find a random node to lookup (for keeping routing table fresh)
        let random_target = crate::network::dht::NodeId::random();

        if let Err(e) = dht_guard.start_lookup(random_target) {
            log::warn!("Failed to start random DHT lookup: {}", e);
        } else {
            log::debug!("Started random DHT lookup for routing table maintenance");
        }

        // Check for completed lookups and process results
        Self::process_completed_lookups(&mut *dht_guard).await?;

        Ok(())
    }

    /// Process completed DHT lookups and connect discovered peers
    async fn process_completed_lookups(dht: &mut DHT) -> Result<()> {
        // Clean up old pending queries
        let mut completed_queries = Vec::new();
        let pending_queries = dht.pending_queries.clone();

        for (target, _query) in pending_queries.iter() {
            if dht.is_lookup_complete(target) {
                if let Some(results) = dht.complete_lookup(target) {
                    completed_queries.push(results);
                }
            }
        }

        // Process discovered peers
        for contacts in completed_queries {
            for contact in contacts {
                if contact.is_alive() {
                    // Add discovered peer to our knowledge
                    dht.insert_contact(contact.clone());

                    // TODO: Attempt P2P connection to discovered peer
                    // In a full implementation, this would establish new P2P connections
                    log::info!("Discovered new peer via DHT: {} at {}",
                              contact.node_id.to_hex(),
                              contact.address);
                }
            }
        }

        Ok(())
    }

    /// Refresh DHT k-buckets
    async fn refresh_dht_buckets(dht: Arc<RwLock<DHT>>) -> Result<()> {
        let mut dht_guard = dht.write().await;

        // Get nodes that need to be pinged
        let nodes_to_ping = dht_guard.refresh_buckets();

        log::debug!("Refreshing {} DHT contacts", nodes_to_ping.len());

        // In a full implementation, we would ping these nodes
        // For now, just mark them as seen (simplified)
        for node_id in nodes_to_ping {
            dht_guard.update_contact_last_seen(&node_id);
        }

        Ok(())
    }

    /// Announce our presence to the network
    async fn announce_presence(dht: Arc<RwLock<DHT>>) -> Result<()> {
        let dht_guard = dht.read().await;
        let our_node_id = *dht_guard.node_id();

        // In a real implementation, this would broadcast to nearby nodes
        // For now, just log our presence
        log::debug!("Announcing presence as node {}", our_node_id.to_hex());

        Ok(())
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

    /// Initialize DHT with bootstrap peers (from TOML config file)
    async fn initialize_dht_bootstrap(&self) -> Result<()> {
        let mut dht = self.dht.write().await;
        let mut successful_bootstrap = 0;

        // Load bootstrap configuration from TOML file
        let bootstrap_peers = match self.load_bootstrap_from_config().await {
            Ok(peers) => {
                log::info!("Loaded {} bootstrap peers from config file", peers.len());
                peers
            }
            Err(e) => {
                log::warn!("Failed to load bootstrap config: {}", e);
                // Return empty vector if we can't load any bootstrap nodes
                vec![]
            }
        };

        log::info!("Initializing DHT with {} bootstrap peers...", bootstrap_peers.len());

        for bootstrap_addr in &bootstrap_peers {
            match self.parse_and_add_bootstrap_node(&mut dht, bootstrap_addr).await {
                Ok(_) => {
                    successful_bootstrap += 1;
                    log::debug!("Added bootstrap node: {}", bootstrap_addr);
                }
                Err(e) => {
                    log::warn!("Failed to add bootstrap node {}: {}", bootstrap_addr, e);
                }
            }
        }

        if successful_bootstrap == 0 {
            log::warn!("No bootstrap nodes could be initialized - network may not function properly");
            return Err(BlockchainError::Network("No bootstrap nodes available".to_string()));
        }

        log::info!("DHT initialized with {}/{} bootstrap nodes successfully",
                  successful_bootstrap, bootstrap_peers.len());
        Ok(())
    }

    /// Load bootstrap nodes from TOML configuration file
    async fn load_bootstrap_from_config(&self) -> Result<Vec<String>> {
        let config_path = "config/bootstrap_nodes.toml";

        // Read the TOML file
        let config_content = fs::read_to_string(config_path)
            .map_err(|e| BlockchainError::Network(format!("Failed to read bootstrap config: {}", e)))?;

        // Parse TOML
        let bootstrap_config: BootstrapConfig = toml::from_str(&config_content)
            .map_err(|e| BlockchainError::Network(format!("Failed to parse bootstrap config: {}", e)))?;

        let mut bootstrap_peers = Vec::new();

        // Add mainnet nodes if we have them
        if !bootstrap_config.bootstrap.mainnet_nodes.is_empty() {
            bootstrap_peers.extend(bootstrap_config.bootstrap.mainnet_nodes);
        }
        // Add testnet nodes if we have them
        else if !bootstrap_config.bootstrap.testnet_nodes.is_empty() {
            bootstrap_peers.extend(bootstrap_config.bootstrap.testnet_nodes);
        }
        // Fall back to development nodes if configured to use them
        else if bootstrap_config.config.use_development_nodes && !bootstrap_config.bootstrap.development_nodes.is_empty() {
            bootstrap_peers.extend(bootstrap_config.bootstrap.development_nodes);
        }

        if bootstrap_peers.is_empty() {
            return Err(BlockchainError::Network("No bootstrap nodes configured".to_string()));
        }

        Ok(bootstrap_peers)
    }

    /// Parse bootstrap node address and add to DHT
    async fn parse_and_add_bootstrap_node(&self, dht: &mut DHT, addr_str: &str) -> Result<()> {
        // Parse bootstrap address in format: /ip4/IP/tcp/PORT/p2p/PEER_ID
        let parts: Vec<&str> = addr_str.split('/').collect();
        if parts.len() < 8 || parts[1] != "ip4" || parts[3] != "tcp" {
            return Err(BlockchainError::Network(format!("Invalid bootstrap address format: {}", addr_str)));
        }

        let ip = parts[2];
        let port_str = parts[4];
        let peer_id_str = parts[6];

        // Parse IP, port, and create socket address
        use std::net::{IpAddr, SocketAddr};
        let ip_addr: IpAddr = ip.parse()
            .map_err(|_| BlockchainError::Network("Invalid IP address".to_string()))?;
        let port: u16 = port_str.parse()
            .map_err(|_| BlockchainError::Network("Invalid port".to_string()))?;

        let socket_addr = SocketAddr::new(ip_addr, port);

        // Parse PeerId from string (simplified - would need proper PeerId parsing)
        use crate::network::dht::NodeId;
        let node_id = NodeId::from_hash(peer_id_str.as_bytes()); // Hash string to NodeId

        // Create contact and add to DHT
        use crate::network::dht::Contact;
        let contact = Contact::new(node_id, socket_addr);
        dht.add_bootstrap_node(contact)?;

        Ok(())
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
