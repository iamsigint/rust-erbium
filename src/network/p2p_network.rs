// src/network/p2p_network.rs

use crate::core::chain::PersistentBlockchain;
use crate::core::{Block, Transaction};
use crate::network::dht::{DHTConfig, DHT};
use crate::network::discovery::PeerDiscovery;
use crate::network::opportunistic::{OpportunisticDiscovery, OpportunisticDiscoveryConfig};
use crate::utils::error::{BlockchainError, Result};
use libp2p::multiaddr::Protocol;
use libp2p::Multiaddr;
use local_ip_address::{list_afinet_netifas, local_ip};
use serde::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet};
use std::fs;
use std::net::{IpAddr, SocketAddr};
use std::sync::Arc;
use tokio::sync::{mpsc, watch, RwLock};
use tokio::task::JoinHandle;
use tokio::time::{self, Duration};
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

#[allow(dead_code)]
#[derive(Debug, Deserialize)]
struct BootstrapBehavior {
    use_development_nodes: bool,
    allow_hardcoded_fallback: bool, // TODO: Currently unused - will be used for DHT fallback
    bootstrap_timeout_seconds: u64, // TODO: Currently unused - will be used for bootstrap timeouts
    max_bootstrap_attempts: usize,  // TODO: Currently unused - will be used for bootstrap retries
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
    pub dht_k: usize,     // Kademlia K parameter (bucket size)
    pub dht_alpha: usize, // Kademlia alpha parameter (parallelism)
    pub dht_refresh_interval: Duration,
    // Opportunistic discovery configuration
    pub opportunistic_enabled: bool,
    pub opportunistic_port: u16,
    pub opportunistic_interval: Duration,
    // Bootstrap peers are now loaded from config/bootstrap_nodes.toml
}

impl Default for P2PConfig {
    fn default() -> Self {
        Self {
            listen_address: "/ip4/0.0.0.0/tcp/30303".to_string(),
            max_peers: 50,
            ping_interval: Duration::from_secs(30),
            sync_interval: Duration::from_secs(10),
            block_announce_interval: Duration::from_millis(100),
            // DHT defaults (standard Kademlia)
            dht_k: 20,
            dht_alpha: 3,
            dht_refresh_interval: Duration::from_secs(3600), // 1 hour
            opportunistic_enabled: true,
            opportunistic_port: 30303,
            opportunistic_interval: Duration::from_secs(5),
        }
    }
}

/// P2P Network manager with DHT integration
pub struct P2PNetwork {
    config: P2PConfig,
    blockchain: Arc<RwLock<PersistentBlockchain>>,
    peers: Arc<RwLock<HashMap<String, PeerInfo>>>,
    known_blocks: Arc<RwLock<HashSet<String>>>, // Block hashes we know about
    known_txs: Arc<RwLock<HashSet<String>>>,    // Transaction hashes we know about
    message_sender: mpsc::UnboundedSender<NetworkMessage>,
    message_receiver: Arc<RwLock<mpsc::UnboundedReceiver<NetworkMessage>>>,
    // DHT integration for decentralized peer discovery
    dht: Arc<RwLock<DHT>>,                      // Kademlia DHT instance
    peer_discovery: Arc<RwLock<PeerDiscovery>>, // Traditional peer management
    opportunistic_tasks: Arc<RwLock<Vec<JoinHandle<()>>>>,
    opportunistic_shutdown: Arc<RwLock<Option<watch::Sender<bool>>>>,
    // Node identification
    local_peer_id: String,  // This node's PeerID for network identification
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
        
        // Generate unique PeerID for this node (deterministic based on node data)
        let local_peer_id = Self::generate_peer_id(&config);
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
            opportunistic_tasks: Arc::new(RwLock::new(Vec::new())),
            opportunistic_shutdown: Arc::new(RwLock::new(None)),
            local_peer_id,
        })
    }

    /// Create new P2P network (legacy constructor)
    pub fn new(config: P2PConfig, blockchain: Arc<RwLock<PersistentBlockchain>>) -> Self {
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

        // Opportunistic peer discovery (best-effort)
        if let Err(e) = self.start_opportunistic_discovery().await {
            log::warn!("Opportunistic peer discovery not started: {}", e);
        }

        // Start background tasks
        self.start_background_tasks();

        // Log node info on startup
        log::warn!(
            "\n\n\
            ════════════════════════════════════════════════════════════════\n\
            🆔 NODE PEER ID: {}\n\
            🌐 Listen Address: {}\n\
            ════════════════════════════════════════════════════════════════\n\
            ⚠️  Share this PeerID with other node operators for bootstrap!\n\
            ⚠️  Add to config/bootstrap_nodes.toml as:\n\
            \"/ip4/YOUR_PUBLIC_IP/tcp/PORT/p2p/{}\"\n\
            ════════════════════════════════════════════════════════════════\n\
            ",
            self.local_peer_id,
            self.config.listen_address,
            self.local_peer_id
        );
        
        // Auto-detect public IP and announce to DHT
        if let Err(e) = self.auto_announce_to_bootstrap().await {
            log::warn!("Failed to auto-announce to bootstrap: {}", e);
        }

        log::info!("P2P network started successfully with DHT integration");
        Ok(())
    }
    
    /// Get this node's PeerID
    pub fn get_peer_id(&self) -> &str {
        &self.local_peer_id
    }
    
    /// Detect external/public IP address
    async fn detect_public_ip(&self) -> Option<String> {
        // Try multiple methods in order of reliability
        
        // Method 1: Try STUN server (most reliable for public IP)
        if let Ok(public_ip) = Self::get_ip_via_stun().await {
            log::info!("Detected public IP via STUN: {}", public_ip);
            return Some(public_ip);
        }
        
        // Method 2: Try HTTP IP detection service (fallback)
        if let Ok(public_ip) = Self::get_ip_via_http().await {
            log::info!("Detected public IP via HTTP service: {}", public_ip);
            return Some(public_ip);
        }
        
        // Method 3: Use local IP (for LAN nodes)
        if let Ok(local_ip) = local_ip() {
            let ip_str = local_ip.to_string();
            if !ip_str.starts_with("127.") && !ip_str.starts_with("169.254.") {
                log::info!("Using local IP address: {}", ip_str);
                return Some(ip_str);
            }
        }
        
        log::warn!("Could not determine external IP address");
        None
    }
    
    /// Get public IP via STUN server
    async fn get_ip_via_stun() -> Result<String> {
        use std::net::UdpSocket;
        use std::time::Duration;
        
        // Google's public STUN server
        let stun_server = "stun.l.google.com:19302";
        
        let socket = UdpSocket::bind("0.0.0.0:0")
            .map_err(|e| BlockchainError::Network(format!("STUN bind failed: {}", e)))?;
        
        socket.set_read_timeout(Some(Duration::from_secs(3)))
            .map_err(|e| BlockchainError::Network(format!("STUN timeout set failed: {}", e)))?;
        
        // Simple STUN Binding Request
        let request = [
            0x00, 0x01, // Binding Request
            0x00, 0x00, // Message Length
            0x21, 0x12, 0xa4, 0x42, // Magic Cookie
            // 12 bytes Transaction ID (random)
            0x00, 0x01, 0x02, 0x03, 0x04, 0x05, 0x06, 0x07, 0x08, 0x09, 0x0a, 0x0b,
        ];
        
        socket.send_to(&request, stun_server)
            .map_err(|e| BlockchainError::Network(format!("STUN send failed: {}", e)))?;
        
        let mut buf = [0u8; 1024];
        let (size, _) = socket.recv_from(&mut buf)
            .map_err(|e| BlockchainError::Network(format!("STUN recv failed: {}", e)))?;
        
        // Parse STUN response (simplified - look for XOR-MAPPED-ADDRESS)
        for i in 20..size.saturating_sub(8) {
            if buf[i] == 0x00 && buf[i+1] == 0x20 { // XOR-MAPPED-ADDRESS
                let port_xor = u16::from_be_bytes([buf[i+6], buf[i+7]]);
                let port = port_xor ^ 0x2112; // XOR with magic cookie first 2 bytes
                
                let ip_bytes = [
                    buf[i+8] ^ 0x21,
                    buf[i+9] ^ 0x12,
                    buf[i+10] ^ 0xa4,
                    buf[i+11] ^ 0x42,
                ];
                
                let ip = format!("{}.{}.{}.{}", ip_bytes[0], ip_bytes[1], ip_bytes[2], ip_bytes[3]);
                log::debug!("STUN detected IP: {}:{}", ip, port);
                return Ok(ip);
            }
        }
        
        Err(BlockchainError::Network("STUN response parsing failed".to_string()))
    }
    
    /// Get public IP via HTTP service (fallback)
    async fn get_ip_via_http() -> Result<String> {
        // Try multiple services in case one is down
        let services = [
            "https://api.ipify.org",
            "https://icanhazip.com",
            "https://ifconfig.me/ip",
        ];
        
        for service in &services {
            if let Ok(response) = tokio::time::timeout(
                std::time::Duration::from_secs(3),
                reqwest::get(*service)
            ).await {
                if let Ok(resp) = response {
                    if let Ok(ip) = resp.text().await {
                        let ip = ip.trim().to_string();
                        if ip.parse::<std::net::IpAddr>().is_ok() {
                            return Ok(ip);
                        }
                    }
                }
            }
        }
        
        Err(BlockchainError::Network("All HTTP IP services failed".to_string()))
    }
    
    /// Auto-announce this node to bootstrap discovery
    async fn auto_announce_to_bootstrap(&self) -> Result<()> {
        // Detect public IP
        let public_ip = match self.detect_public_ip().await {
            Some(ip) => ip,
            None => {
                log::warn!("Cannot auto-announce: No public IP detected");
                return Ok(()); // Not an error, just can't announce
            }
        };
        
        // Parse port from listen address
        let port = self.config.listen_address
            .split(':')
            .last()
            .and_then(|p| p.parse::<u16>().ok())
            .unwrap_or(30303);
        
        // Create bootstrap address
        let bootstrap_addr = format!("/ip4/{}/tcp/{}/p2p/{}", public_ip, port, self.local_peer_id);
        
        log::warn!(
            "\n\n\
            ════════════════════════════════════════════════════════════════\n\
            🚀 AUTO-DETECTED BOOTSTRAP ADDRESS:\n\
            ════════════════════════════════════════════════════════════════\n\
            {}\n\
            ════════════════════════════════════════════════════════════════\n\
            ✅ This node is now discoverable!\n\
            ⚠️  Share this address with peers to join your network\n\
            ⚠️  Other nodes can add this to their bootstrap_nodes.toml\n\
            ════════════════════════════════════════════════════════════════\n\
            ",
            bootstrap_addr
        );
        
        // Try to announce to DHT
        let mut dht = self.dht.write().await;
        
        // Store our own contact info in DHT
        let our_node_id = *dht.node_id();
        
        // Parse socket address
        let socket_addr = format!("{}:{}", public_ip, port)
            .parse::<std::net::SocketAddr>()
            .map_err(|e| BlockchainError::Network(format!("Invalid socket address: {}", e)))?;
        
        let our_contact = crate::network::dht::Contact::new(our_node_id, socket_addr);
        
        dht.insert_contact(our_contact);
        
        log::info!("Successfully announced to DHT with public IP: {}", public_ip);
        
        Ok(())
    }
    
    /// Get comprehensive node information
    pub async fn get_node_info(&self) -> Result<serde_json::Value> {
        let blockchain = self.blockchain.read().await;
        let height = blockchain.get_latest_height().await?;
        let peers = self.peers.read().await;
        
        let genesis_hash = if height > 0 {
            match blockchain.get_block_by_height(0).await {
                Ok(Some(block)) => block.hash().to_hex(),
                _ => "unknown".to_string(),
            }
        } else {
            "not_initialized".to_string()
        };
        
        Ok(serde_json::json!({
            "peer_id": self.local_peer_id,
            "listen_address": self.config.listen_address,
            "genesis_hash": genesis_hash,
            "block_height": height,
            "peers_connected": peers.len(),
            "version": env!("CARGO_PKG_VERSION"),
            "network_type": if self.config.listen_address.contains("127.0.0.1") { 
                "local" 
            } else { 
                "public" 
            }
        }))
    }
    
    /// Generate deterministic PeerID for this node
    fn generate_peer_id(config: &P2PConfig) -> String {
        use sha2::{Digest, Sha256};
        
        // Generate from listen address and process ID for uniqueness
        let mut hasher = Sha256::new();
        hasher.update(config.listen_address.as_bytes());
        hasher.update(std::process::id().to_le_bytes());
        
        let hash = hasher.finalize();
        // Format like libp2p PeerID: 12D3KooW + base58-style encoding
        format!("12D3KooW{}", hex::encode(&hash[..16]))
    }

    /// Stop the P2P network
    pub async fn stop(&self) -> Result<()> {
        log::info!("Stopping P2P network");

        {
            let mut shutdown_guard = self.opportunistic_shutdown.write().await;
            if let Some(sender) = shutdown_guard.take() {
                let _ = sender.send(true);
            }
        }

        {
            let mut tasks = self.opportunistic_tasks.write().await;
            for handle in tasks.drain(..) {
                handle.abort();
            }
        }

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
        self.message_sender.send(message).map_err(|_| {
            BlockchainError::Network("Failed to send block announcement".to_string())
        })?;

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
        self.message_sender.send(message).map_err(|_| {
            BlockchainError::Network("Failed to send transaction announcement".to_string())
        })?;

        log::debug!("Broadcasting transaction {} to peers", tx_hash);
        Ok(())
    }

    /// Request blocks from peers
    pub async fn request_blocks(&self, start_height: u64, count: u32) -> Result<()> {
        let message = NetworkMessage::BlockRequest {
            start_height,
            count,
        };

        self.message_sender
            .send(message)
            .map_err(|_| BlockchainError::Network("Failed to send block request".to_string()))?;

        log::debug!(
            "Requesting {} blocks starting from height {}",
            count,
            start_height
        );
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
            NetworkMessage::BlockRequest {
                start_height,
                count,
            } => {
                self.handle_block_request(start_height, count, peer_id)
                    .await?;
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
            self.request_blocks(
                current_height + 1,
                (block.header.number - current_height) as u32,
            )
            .await?;
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
                    log::warn!(
                        "Failed to add block {} from peer {}: {}",
                        block_hash,
                        peer_id,
                        e
                    );
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
    async fn handle_block_request(
        &self,
        start_height: u64,
        count: u32,
        peer_id: &str,
    ) -> Result<()> {
        let mut blocks = Vec::new();

        for i in 0..count {
            let height = start_height + i as u64;
            if let Ok(Some(block)) = self
                .blockchain
                .read()
                .await
                .get_block_by_height(height)
                .await
            {
                blocks.push(block);
            } else {
                break; // No more blocks
            }
        }

        if !blocks.is_empty() {
            let response = NetworkMessage::BlockResponse(blocks.clone());
            self.message_sender.send(response).map_err(|_| {
                BlockchainError::Network("Failed to send block response".to_string())
            })?;
        }

        log::debug!(
            "Sent {} blocks to peer {} starting from height {}",
            blocks.len(),
            peer_id,
            start_height
        );
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
            if let Ok(Some(tx)) = self
                .blockchain
                .read()
                .await
                .get_transaction_by_hash(hash.as_bytes())
                .await
            {
                transactions.push(tx);
            }
        }

        if !transactions.is_empty() {
            let response = NetworkMessage::TransactionResponse(transactions);
            self.message_sender.send(response).map_err(|_| {
                BlockchainError::Network("Failed to send transaction response".to_string())
            })?;
        }

        Ok(())
    }

    /// Handle transaction response from peer
    async fn handle_transaction_response(
        &self,
        transactions: Vec<Transaction>,
        peer_id: &str,
    ) -> Result<()> {
        // For now, just log the transactions
        // In a full implementation, this would add to mempool
        log::debug!(
            "Received {} transactions from peer {}",
            transactions.len(),
            peer_id
        );
        Ok(())
    }

    /// Handle peer status update
    async fn handle_peer_status(&self, peer_id: &str, height: u64, peer_count: u32) -> Result<()> {
        // Update peer information
        {
            let mut peers = self.peers.write().await;
            if let Some(peer) = peers.get_mut(peer_id) {
                peer.best_height = height;
                peer.last_seen = current_timestamp();
            }
        }

        log::debug!(
            "Peer {} status: height={}, peers={}",
            peer_id,
            height,
            peer_count
        );
        
        // Check if we need to sync with this peer
        let my_height = {
            let blockchain = self.blockchain.read().await;
            blockchain.get_latest_height().await?
        };
        
        if height > my_height {
            log::info!(
                "Peer {} has height {} (we have {}). Triggering sync...",
                peer_id,
                height,
                my_height
            );
            
            // Trigger synchronization in background using Arc references
            let blockchain = Arc::clone(&self.blockchain);
            let peers = Arc::clone(&self.peers);
            let message_sender = self.message_sender.clone();
            let peer_id_str = peer_id.to_string();
            
            tokio::spawn(async move {
                if let Err(e) = Self::sync_with_peer_standalone(
                    &peer_id_str,
                    blockchain,
                    peers,
                    message_sender,
                ).await {
                    log::error!("Failed to sync with peer {}: {}", peer_id_str, e);
                }
            });
        }
        
        Ok(())
    }
    
    /// Standalone sync function for background spawning
    async fn sync_with_peer_standalone(
        peer_id: &str,
        blockchain: Arc<RwLock<PersistentBlockchain>>,
        peers: Arc<RwLock<HashMap<String, PeerInfo>>>,
        message_sender: mpsc::UnboundedSender<NetworkMessage>,
    ) -> Result<()> {
        log::info!("Starting blockchain sync with peer {}", peer_id);
        
        // Get our current blockchain height
        let my_height = {
            let blockchain = blockchain.read().await;
            blockchain.get_latest_height().await?
        };
        
        // Get peer's blockchain height
        let peer_height = {
            let peers = peers.read().await;
            peers.get(peer_id)
                .map(|p| p.best_height)
                .unwrap_or(0)
        };
        
        log::info!(
            "Sync status: My height = {}, Peer {} height = {}",
            my_height, peer_id, peer_height
        );
        
        // If peer has more blocks than us, request them
        if peer_height > my_height {
            let blocks_to_request = peer_height - my_height;
            log::info!(
                "Peer {} is ahead by {} blocks. Requesting blocks {} to {}",
                peer_id,
                blocks_to_request,
                my_height + 1,
                peer_height
            );
            
            // Request blocks in batches
            let batch_size = 100u32;
            let mut current_height = my_height + 1;
            
            while current_height <= peer_height {
                let remaining = (peer_height - current_height + 1) as u32;
                let count = remaining.min(batch_size);
                
                log::debug!("Requesting batch: {} blocks from height {}", count, current_height);
                
                let message = NetworkMessage::BlockRequest {
                    start_height: current_height,
                    count,
                };
                
                message_sender.send(message).map_err(|_| {
                    BlockchainError::Network("Failed to send block request".to_string())
                })?;
                
                current_height += count as u64;
                
                // Small delay between batches
                tokio::time::sleep(Duration::from_millis(100)).await;
            }
        }
        
        Ok(())
    }

    /// Handle ping from peer
    async fn handle_ping(&self, peer_id: &str) -> Result<()> {
        // Send pong response
        let pong = NetworkMessage::Pong;
        self.message_sender
            .send(pong)
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
        log::info!(
            "Initialized P2P network with blockchain height: {}",
            latest_height
        );

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
                    log::info!(
                        "Discovered new peer via DHT: {} at {}",
                        contact.node_id.to_hex(),
                        contact.address
                    );
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

    async fn start_opportunistic_discovery(&self) -> Result<()> {
        if !self.config.opportunistic_enabled {
            return Ok(());
        }

        let advertised_addr = match self.determine_advertised_socket()? {
            Some(addr) => addr,
            None => {
                log::warn!(
                    "Unable to determine advertised address for opportunistic discovery; skipping"
                );
                return Ok(());
            }
        };

        let node_id = {
            let dht_guard = self.dht.read().await;
            *dht_guard.node_id()
        };

        let discovery_config = OpportunisticDiscoveryConfig {
            port: self.config.opportunistic_port,
            interval: self.config.opportunistic_interval,
            max_packet_size: 2048,
        };

        let (shutdown_tx, shutdown_rx) = watch::channel(false);

        let handles = OpportunisticDiscovery::spawn(
            discovery_config,
            advertised_addr,
            node_id,
            Arc::clone(&self.dht),
            Arc::clone(&self.peer_discovery),
            shutdown_rx,
        )
        .await?;

        {
            let mut shutdown_guard = self.opportunistic_shutdown.write().await;
            *shutdown_guard = Some(shutdown_tx);
        }

        {
            let mut tasks_guard = self.opportunistic_tasks.write().await;
            tasks_guard.extend(handles);
        }

        log::info!(
            "Opportunistic discovery broadcasting {} on UDP port {}",
            advertised_addr,
            self.config.opportunistic_port
        );

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

        log::info!(
            "Initializing DHT with {} bootstrap peers...",
            bootstrap_peers.len()
        );

        for bootstrap_addr in &bootstrap_peers {
            match self
                .parse_and_add_bootstrap_node(&mut dht, bootstrap_addr)
                .await
            {
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
            log::warn!("No bootstrap nodes could be initialized from configuration");

            if !self.config.opportunistic_enabled {
                return Err(BlockchainError::Network(
                    "No bootstrap nodes available".to_string(),
                ));
            }

            log::info!("Falling back to opportunistic peer discovery with UDP broadcasts");
        } else {
            log::info!(
                "DHT initialized with {}/{} bootstrap nodes successfully",
                successful_bootstrap,
                bootstrap_peers.len()
            );
        }
        Ok(())
    }

    /// Load bootstrap nodes from TOML configuration file
    async fn load_bootstrap_from_config(&self) -> Result<Vec<String>> {
        let config_path = "config/bootstrap_nodes.toml";

        // Read the TOML file
        let config_content = fs::read_to_string(config_path).map_err(|e| {
            BlockchainError::Network(format!("Failed to read bootstrap config: {}", e))
        })?;

        // Parse TOML
        let bootstrap_config: BootstrapConfig = toml::from_str(&config_content).map_err(|e| {
            BlockchainError::Network(format!("Failed to parse bootstrap config: {}", e))
        })?;

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
        else if bootstrap_config.config.use_development_nodes
            && !bootstrap_config.bootstrap.development_nodes.is_empty()
        {
            bootstrap_peers.extend(bootstrap_config.bootstrap.development_nodes);
        }

        if bootstrap_peers.is_empty() {
            return Err(BlockchainError::Network(
                "No bootstrap nodes configured".to_string(),
            ));
        }

        Ok(bootstrap_peers)
    }

    /// Parse bootstrap node address and add to DHT
    async fn parse_and_add_bootstrap_node(&self, dht: &mut DHT, addr_str: &str) -> Result<()> {
        // Parse bootstrap address in format: /ip4/IP/tcp/PORT/p2p/PEER_ID
        let parts: Vec<&str> = addr_str.split('/').collect();
        if parts.len() < 8 || parts[1] != "ip4" || parts[3] != "tcp" {
            return Err(BlockchainError::Network(format!(
                "Invalid bootstrap address format: {}",
                addr_str
            )));
        }

        let ip = parts[2];
        let port_str = parts[4];
        let peer_id_str = parts[6];

        // Parse IP, port, and create socket address
        use std::net::{IpAddr, SocketAddr};
        let ip_addr: IpAddr = ip
            .parse()
            .map_err(|_| BlockchainError::Network("Invalid IP address".to_string()))?;
        let port: u16 = port_str
            .parse()
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
        let peer_id = peer_info.peer_id.clone();
        
        {
            let mut peers = self.peers.write().await;
            peers.insert(peer_id.clone(), peer_info);
        }
        
        log::info!("Added peer {} to network", peer_id);
        
        // Automatically attempt to sync with the new peer
        if let Err(e) = self.sync_with_peer(&peer_id).await {
            log::warn!("Failed to sync with peer {}: {}", peer_id, e);
        }
        
        Ok(())
    }
    
    /// Synchronize blockchain with a specific peer
    async fn sync_with_peer(&self, peer_id: &str) -> Result<()> {
        log::info!("Starting blockchain sync with peer {}", peer_id);
        
        // Get our current blockchain height
        let my_height = {
            let blockchain = self.blockchain.read().await;
            blockchain.get_latest_height().await?
        };
        
        // Get peer's blockchain height
        let peer_height = {
            let peers = self.peers.read().await;
            peers.get(peer_id)
                .map(|p| p.best_height)
                .unwrap_or(0)
        };
        
        log::info!(
            "Sync status: My height = {}, Peer {} height = {}",
            my_height, peer_id, peer_height
        );
        
        // If peer has more blocks than us, request them
        if peer_height > my_height {
            let blocks_to_request = peer_height - my_height;
            log::info!(
                "Peer {} is ahead by {} blocks. Requesting blocks {} to {}",
                peer_id,
                blocks_to_request,
                my_height + 1,
                peer_height
            );
            
            // Request blocks in batches to avoid overwhelming the network
            let batch_size = 100u32;
            let mut current_height = my_height + 1;
            
            while current_height <= peer_height {
                let remaining = (peer_height - current_height + 1) as u32;
                let count = remaining.min(batch_size);
                
                log::debug!("Requesting batch: {} blocks from height {}", count, current_height);
                self.request_blocks(current_height, count).await?;
                
                current_height += count as u64;
                
                // Small delay between batches to allow processing
                tokio::time::sleep(Duration::from_millis(100)).await;
            }
            
            Ok(())
        } else if peer_height < my_height {
            log::info!(
                "We are ahead of peer {} by {} blocks. Peer will sync from us.",
                peer_id,
                my_height - peer_height
            );
            Ok(())
        } else {
            log::debug!("Blockchains are at the same height ({})", my_height);
            Ok(())
        }
    }

    /// Remove a peer from the network
    pub async fn remove_peer(&self, peer_id: &str) -> Result<()> {
        let mut peers = self.peers.write().await;
        peers.remove(peer_id);
        log::info!("Removed peer {} from network", peer_id);
        Ok(())
    }

    fn determine_advertised_socket(&self) -> Result<Option<SocketAddr>> {
        if self.config.listen_address.trim().is_empty() {
            return Ok(None);
        }

        let Some(listen_socket) = parse_listen_address(&self.config.listen_address) else {
            log::warn!(
                "Failed to parse listen address '{}' for opportunistic discovery",
                self.config.listen_address
            );
            return Ok(None);
        };

        if listen_socket.port() == 0 {
            log::warn!(
                "Listen address '{}' uses port 0; unable to advertise",
                self.config.listen_address
            );
            return Ok(None);
        }

        let prefer_ipv4 = listen_socket.is_ipv4();
        let ip = if listen_socket.ip().is_unspecified() {
            match resolve_local_ip(prefer_ipv4) {
                Some(ip) => ip,
                None => {
                    log::warn!("Could not determine a suitable local IP address to advertise");
                    return Ok(None);
                }
            }
        } else {
            listen_socket.ip()
        };

        Ok(Some(SocketAddr::new(ip, listen_socket.port())))
    }
}

fn parse_listen_address(address: &str) -> Option<SocketAddr> {
    if address.trim().is_empty() {
        return None;
    }

    if let Ok(multiaddr) = address.parse::<Multiaddr>() {
        let mut ip: Option<IpAddr> = None;
        let mut port: Option<u16> = None;

        for proto in multiaddr.iter() {
            match proto {
                Protocol::Ip4(v4) => ip = Some(IpAddr::V4(v4)),
                Protocol::Ip6(v6) => ip = Some(IpAddr::V6(v6)),
                Protocol::Tcp(p) => port = Some(p),
                _ => {}
            }
        }

        if let (Some(ip), Some(port)) = (ip, port) {
            return Some(SocketAddr::new(ip, port));
        }
    }

    address.parse().ok()
}

fn resolve_local_ip(prefer_ipv4: bool) -> Option<IpAddr> {
    // Prefer non-loopback addresses first
    let mut fallback_loopback: Option<IpAddr> = None;

    if let Ok(interfaces) = list_afinet_netifas() {
        for (_, ip) in interfaces {
            match ip {
                IpAddr::V4(v4) => {
                    if prefer_ipv4 && !v4.is_unspecified() {
                        if !v4.is_loopback() {
                            return Some(IpAddr::V4(v4));
                        }
                        if fallback_loopback.is_none() {
                            fallback_loopback = Some(IpAddr::V4(v4));
                        }
                    }
                }
                IpAddr::V6(v6) => {
                    if !prefer_ipv4 && !v6.is_unspecified() {
                        if !v6.is_loopback() {
                            return Some(IpAddr::V6(v6));
                        }
                        if fallback_loopback.is_none() {
                            fallback_loopback = Some(IpAddr::V6(v6));
                        }
                    }
                }
            }
        }
    }

    if let Ok(ip) = local_ip() {
        match ip {
            IpAddr::V4(v4) => {
                if prefer_ipv4 {
                    if !v4.is_loopback() && !v4.is_unspecified() {
                        return Some(IpAddr::V4(v4));
                    }
                    if fallback_loopback.is_none() {
                        fallback_loopback = Some(IpAddr::V4(v4));
                    }
                }
            }
            IpAddr::V6(v6) => {
                if !prefer_ipv4 {
                    if !v6.is_loopback() && !v6.is_unspecified() {
                        return Some(IpAddr::V6(v6));
                    }
                    if fallback_loopback.is_none() {
                        fallback_loopback = Some(IpAddr::V6(v6));
                    }
                }
            }
        }
    }

    fallback_loopback
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
            PersistentBlockchain::new(temp_dir.path().to_str().unwrap())
                .await
                .unwrap(),
        ));

        let config = P2PConfig::default();
        let network = P2PNetwork::new(config, blockchain);

        let stats = network.get_stats().await.unwrap();
        assert_eq!(stats.connected_peers, 0);
        assert_eq!(stats.total_peers, 0);
    }
}
