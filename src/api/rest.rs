use crate::bridges::core::bridge_manager::BridgeManager;
use crate::core::chain::{Blockchain, PersistentBlockchain};
use crate::core::mempool::Mempool;
use crate::core::types::Address;
use crate::core::{Block, Transaction};
use crate::utils::error::BlockchainError;
use crate::utils::{RateLimiter, RateLimitConfig};
use serde::{Deserialize, Serialize};
use std::net::SocketAddr;
use std::sync::Arc;
use tokio::sync::RwLock;
use warp::http::StatusCode;
use warp::Filter;

// Custom error for warp rejections
#[derive(Debug)]
struct ApiError {
    message: String,
}

impl ApiError {
    fn new(message: &str) -> Self {
        Self {
            message: message.to_string(),
        }
    }
}

impl warp::reject::Reject for ApiError {}

#[allow(dead_code)]
#[derive(Deserialize)]
struct TransactionRequest {
    transaction_type: String,
    from: String,
    to: String,
    amount: Option<u64>,
    fee: u64,
    nonce: u64,
    _data: Option<String>,
    zk_proof: Option<String>,
    input_commitments: Option<Vec<String>>,
    output_commitments: Option<Vec<String>>,
    range_proofs: Option<Vec<String>>,
    binding_signature: Option<String>,
}

#[allow(dead_code)]
#[derive(Deserialize)]
struct BridgeTransferRequest {
    _source_chain: String,
    _target_chain: String,
    _amount: u64,
    _asset_id: String,
    _sender: String,
    _recipient: String,
}

#[allow(dead_code)]
fn parse_and_process_transaction(body: serde_json::Value) -> Result<(Transaction, String), BlockchainError> {
    let req: TransactionRequest = serde_json::from_value(body)
        .map_err(|e| BlockchainError::Serialization(format!("Invalid JSON: {}", e)))?;

    // Parse addresses
    let from = Address::new(req.from)
        .map_err(|_| BlockchainError::InvalidTransaction("Invalid from address".to_string()))?;
    let to = Address::new(req.to)
        .map_err(|_| BlockchainError::InvalidTransaction("Invalid to address".to_string()))?;

    // Create transaction based on type
    let transaction = match req.transaction_type.as_str() {
        "transfer" => crate::core::transaction::Transaction::new_transfer(
            from,
            to,
            req.amount.unwrap_or(0),
            req.fee,
            req.nonce,
        ),
        "confidential_transfer" => {
            // Parse confidential transaction data
            let zk_proof = hex::decode(req.zk_proof.unwrap_or_default())
                .map_err(|_| BlockchainError::InvalidTransaction("Invalid ZK proof".to_string()))?;
            let input_commitments = req
                .input_commitments
                .unwrap_or_default()
                .iter()
                .map(hex::decode)
                .collect::<Result<Vec<_>, _>>()
                .map_err(|_| {
                    BlockchainError::InvalidTransaction("Invalid input commitments".to_string())
                })?;
            let output_commitments = req
                .output_commitments
                .unwrap_or_default()
                .iter()
                .map(hex::decode)
                .collect::<Result<Vec<_>, _>>()
                .map_err(|_| {
                    BlockchainError::InvalidTransaction("Invalid output commitments".to_string())
                })?;
            let range_proofs = req
                .range_proofs
                .unwrap_or_default()
                .iter()
                .map(hex::decode)
                .collect::<Result<Vec<_>, _>>()
                .map_err(|_| {
                    BlockchainError::InvalidTransaction("Invalid range proofs".to_string())
                })?;
            let binding_signature = hex::decode(req.binding_signature.unwrap_or_default())
                .map_err(|_| {
                    BlockchainError::InvalidTransaction("Invalid binding signature".to_string())
                })?;

            crate::core::transaction::Transaction::new_confidential_transfer(
                from,
                to,
                req.amount.unwrap_or(0),
                req.fee,
                req.nonce,
                zk_proof,
                input_commitments,
                output_commitments,
                range_proofs,
                binding_signature,
            )
        }
        "stake" => crate::core::transaction::Transaction::new_stake(
            from,
            req.amount.unwrap_or(0),
            req.fee,
            req.nonce,
        ),
        "unstake" => crate::core::transaction::Transaction::new_unstake(
            from,
            req.amount.unwrap_or(0),
            req.fee,
            req.nonce,
        ),
        _ => {
            return Err(BlockchainError::InvalidTransaction(format!(
                "Unknown transaction type: {}",
                req.transaction_type
            )));
        }
    };

    // Validate transaction
    transaction.validate_basic()?;

    // Generate transaction hash
    let tx_hash = transaction.hash().to_string();

    log::info!("Transaction validated: {}", tx_hash);

    Ok((transaction, tx_hash))
}

// Simplified RestServer with mempool integration
#[derive(Clone)]
pub struct RestServer {
    port: u16,
    persistent_blockchain: Option<Arc<RwLock<PersistentBlockchain>>>,
    #[allow(dead_code)]
    blockchain: Option<Arc<RwLock<Blockchain>>>,
    mempool: Option<Arc<RwLock<Mempool>>>,
    bridge_manager: Option<Arc<RwLock<BridgeManager>>>,
    p2p_network: Option<Arc<RwLock<crate::network::p2p_network::P2PNetwork>>>,
    #[allow(dead_code)]
    rate_limiter: Arc<RateLimiter>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct RestResponse<T> {
    pub success: bool,
    pub data: Option<T>,
    pub error: Option<String>,
}

impl RestServer {
    pub fn new(port: u16) -> Result<Self, BlockchainError> {
        // Create rate limiter with default config
        let rate_limit_config = RateLimitConfig {
            max_requests: 100, // 100 requests per minute per IP
            window: std::time::Duration::from_secs(60),
            enabled: true,
        };
        let rate_limiter = Arc::new(RateLimiter::new(rate_limit_config));

        Ok(Self {
            port,
            persistent_blockchain: None,
            blockchain: None,
            mempool: None,
            bridge_manager: None,
            p2p_network: None,
            rate_limiter,
        })
    }

    /// Create with custom rate limit configuration
    pub fn with_rate_limit_config(port: u16, rate_limit_config: RateLimitConfig) -> Result<Self, BlockchainError> {
        let rate_limiter = Arc::new(RateLimiter::new(rate_limit_config));

        Ok(Self {
            port,
            persistent_blockchain: None,
            blockchain: None,
            mempool: None,
            bridge_manager: None,
            p2p_network: None,
            rate_limiter,
        })
    }

    /// Check rate limit for incoming request
    #[allow(dead_code)]
    async fn check_rate_limit(&self, addr: Option<SocketAddr>) -> Result<(), (StatusCode, String)> {
        if let Some(socket_addr) = addr {
            match self.rate_limiter.check_ip(socket_addr.ip()).await {
                Ok(()) => Ok(()),
                Err(retry_after) => {
                    log::warn!("Rate limit exceeded for IP: {}", socket_addr.ip());
                    Err((
                        StatusCode::TOO_MANY_REQUESTS,
                        format!(
                            "{{\"error\":\"Rate limit exceeded. Retry after {} seconds\",\"retry_after\":{}}}",
                            retry_after, retry_after
                        ),
                    ))
                }
            }
        } else {
            Ok(()) // No address means local/test request
        }
    }

    pub fn with_persistent_blockchain(
        mut self,
        blockchain: Arc<RwLock<PersistentBlockchain>>,
    ) -> Self {
        self.persistent_blockchain = Some(blockchain);
        self
    }

    pub fn with_mempool(mut self, mempool: Arc<RwLock<Mempool>>) -> Self {
        self.mempool = Some(mempool);
        self
    }

    pub fn with_bridge_manager(mut self, bridge_manager: Arc<RwLock<BridgeManager>>) -> Self {
        self.bridge_manager = Some(bridge_manager);
        self
    }

    pub fn with_p2p_network(
        mut self,
        p2p_network: Arc<RwLock<crate::network::p2p_network::P2PNetwork>>,
    ) -> Self {
        self.p2p_network = Some(p2p_network);
        self
    }

    pub async fn start(&self) -> Result<(), BlockchainError> {
        log::info!("Starting REST API server on port {}", self.port);

        // Start background mining task for development/testing
        let self_clone = self.clone();
        tokio::spawn(async move {
            self_clone.start_background_mining().await;
        });

        // Build routes
        let routes = self.build_routes();

        // Start the HTTP server
        let addr: SocketAddr = ([127, 0, 0, 1], self.port)
            .try_into()
            .map_err(|e| BlockchainError::Network(format!("Invalid address: {}", e)))?;

        log::info!("🌐 REST API listening on http://{}", addr);
        log::info!("📡 Available endpoints:");
        log::info!("   GET  /api/v1/health");
        log::info!("   GET  /api/v1/accounts/<address>");
        log::info!("   GET  /api/v1/blocks/<height_or_hash>");
        log::info!("   GET  /api/v1/blocks/latest");
        log::info!("   GET  /api/v1/transactions/<hash>");
        log::info!("   POST /api/v1/transactions");
        log::info!("   GET  /api/v1/node/info");
        log::info!("   GET  /api/v1/node/peers");

        warp::serve(routes).run(addr).await;

        Ok(())
    }

    fn build_routes(
        &self,
    ) -> impl warp::Filter<Extract = impl warp::Reply, Error = warp::Rejection> + Clone {
        let health = warp::path!("api" / "v1" / "health")
            .and(warp::get())
            .map(|| warp::reply::json(&serde_json::json!({"status": "healthy"})));

        let account = {
            let blockchain = self.persistent_blockchain.clone();
            warp::path!("api" / "v1" / "accounts" / String)
                .and(warp::get())
                .and_then(move |address: String| {
                    let blockchain = blockchain.clone();
                    async move {
                        Self::get_account_handler(blockchain, address).await
                    }
                })
        };

        let block = {
            let blockchain = self.persistent_blockchain.clone();
            warp::path!("api" / "v1" / "blocks" / String)
                .and(warp::get())
                .and_then(move |height_or_hash: String| {
                    let blockchain = blockchain.clone();
                    async move {
                        Self::get_block_handler(blockchain, height_or_hash).await
                    }
                })
        };

        let latest_block = {
            let blockchain = self.persistent_blockchain.clone();
            warp::path!("api" / "v1" / "blocks" / "latest")
                .and(warp::get())
                .and_then(move || {
                    let blockchain = blockchain.clone();
                    async move {
                        Self::get_latest_block_handler(blockchain).await
                    }
                })
        };

        let transaction = {
            let blockchain = self.persistent_blockchain.clone();
            warp::path!("api" / "v1" / "transactions" / String)
                .and(warp::get())
                .and_then(move |hash: String| {
                    let blockchain = blockchain.clone();
                    async move {
                        Self::get_transaction_handler(blockchain, hash).await
                    }
                })
        };

        let node_info = {
            let p2p = self.p2p_network.clone();
            warp::path!("api" / "v1" / "node" / "info")
                .and(warp::get())
                .and_then(move || {
                    let p2p = p2p.clone();
                    async move {
                        Self::get_node_info_handler(p2p).await
                    }
                })
        };

        let node_peers = {
            let p2p = self.p2p_network.clone();
            warp::path!("api" / "v1" / "node" / "peers")
                .and(warp::get())
                .and_then(move || {
                    let p2p = p2p.clone();
                    async move {
                        Self::get_node_peers_handler(p2p).await
                    }
                })
        };

        let send_transaction = {
            let mempool = self.mempool.clone();
            warp::path!("api" / "v1" / "transactions")
                .and(warp::post())
                .and(warp::body::json())
                .and_then(move |body: serde_json::Value| {
                    let mempool = mempool.clone();
                    async move {
                        Self::send_transaction_handler(mempool, body).await
                    }
                })
        };

        health
            .or(account)
            .or(block)
            .or(latest_block)
            .or(transaction)
            .or(node_info)
            .or(node_peers)
            .or(send_transaction)
    }

    async fn get_account_handler(
        blockchain: Option<Arc<RwLock<PersistentBlockchain>>>,
        address: String,
    ) -> Result<impl warp::Reply, warp::Rejection> {
        let blockchain = blockchain.ok_or_else(|| {
            warp::reject::custom(ApiError::new("Blockchain not available"))
        })?;

        let addr = Address::new(address.clone()).map_err(|_| {
            warp::reject::custom(ApiError::new("Invalid address"))
        })?;

        let chain = blockchain.read().await;
        let state = chain.get_state();
        let balance = state.get_balance(&addr).unwrap_or(0);

        Ok(warp::reply::json(&serde_json::json!({
            "address": address,
            "balance": balance,
        })))
    }

    async fn get_block_handler(
        blockchain: Option<Arc<RwLock<PersistentBlockchain>>>,
        height_or_hash: String,
    ) -> Result<impl warp::Reply, warp::Rejection> {
        let blockchain = blockchain.ok_or_else(|| {
            warp::reject::custom(ApiError::new("Blockchain not available"))
        })?;

        let chain = blockchain.read().await;

        let block = if let Ok(height) = height_or_hash.parse::<u64>() {
            chain.get_block_by_height(height).await.ok().flatten()
        } else {
            // Try as hash
            if let Ok(hash_bytes) = hex::decode(&height_or_hash) {
                if hash_bytes.len() == 32 {
                    let mut array = [0u8; 32];
                    array.copy_from_slice(&hash_bytes);
                    let hash = crate::core::types::Hash::from_bytes(array);
                    chain.get_block_by_hash(&hash).await.ok().flatten()
                } else {
                    None
                }
            } else {
                None
            }
        };

        if let Some(block) = block {
            Ok(warp::reply::json(&serde_json::json!({
                "height": block.header.number,
                "hash": block.hash().to_hex(),
                "previous_hash": block.header.previous_hash.to_hex(),
                "timestamp": block.header.timestamp,
                "validator": block.header.validator,
                "transactions": block.transactions.len(),
            })))
        } else {
            Err(warp::reject::custom(ApiError::new("Block not found")))
        }
    }

    async fn get_latest_block_handler(
        blockchain: Option<Arc<RwLock<PersistentBlockchain>>>,
    ) -> Result<impl warp::Reply, warp::Rejection> {
        let blockchain = blockchain.ok_or_else(|| {
            warp::reject::custom(ApiError::new("Blockchain not available"))
        })?;

        let chain = blockchain.read().await;
        let height = chain.get_latest_height().await.map_err(|_| {
            warp::reject::custom(ApiError::new("Failed to get latest height"))
        })?;

        if let Ok(Some(block)) = chain.get_block_by_height(height).await {
            Ok(warp::reply::json(&serde_json::json!({
                "height": block.header.number,
                "hash": block.hash().to_hex(),
                "previous_hash": block.header.previous_hash.to_hex(),
                "timestamp": block.header.timestamp,
                "validator": block.header.validator,
                "transactions": block.transactions.len(),
            })))
        } else {
            Err(warp::reject::custom(ApiError::new("Latest block not found")))
        }
    }

    async fn get_transaction_handler(
        blockchain: Option<Arc<RwLock<PersistentBlockchain>>>,
        hash: String,
    ) -> Result<impl warp::Reply, warp::Rejection> {
        let blockchain = blockchain.ok_or_else(|| {
            warp::reject::custom(ApiError::new("Blockchain not available"))
        })?;

        let hash_bytes = hex::decode(&hash).map_err(|_| {
            warp::reject::custom(ApiError::new("Invalid transaction hash"))
        })?;

        let chain = blockchain.read().await;
        if let Ok(Some(tx)) = chain.get_transaction_by_hash(&hash_bytes).await {
            Ok(warp::reply::json(&serde_json::json!({
                "hash": tx.hash().to_hex(),
                "from": tx.from.to_string(),
                "to": tx.to.to_string(),
                "amount": tx.amount,
                "fee": tx.fee,
                "nonce": tx.nonce,
                "timestamp": tx.timestamp,
            })))
        } else {
            Err(warp::reject::custom(ApiError::new("Transaction not found")))
        }
    }

    async fn get_node_info_handler(
        p2p: Option<Arc<RwLock<crate::network::p2p_network::P2PNetwork>>>,
    ) -> Result<impl warp::Reply, warp::Rejection> {
        let p2p = p2p.ok_or_else(|| {
            warp::reject::custom(ApiError::new("P2P network not available"))
        })?;

        let network = p2p.read().await;
        let info = network.get_node_info().await.map_err(|_| {
            warp::reject::custom(ApiError::new("Failed to get node info"))
        })?;

        Ok(warp::reply::json(&info))
    }

    async fn get_node_peers_handler(
        p2p: Option<Arc<RwLock<crate::network::p2p_network::P2PNetwork>>>,
    ) -> Result<impl warp::Reply, warp::Rejection> {
        let p2p = p2p.ok_or_else(|| {
            warp::reject::custom(ApiError::new("P2P network not available"))
        })?;

        let network = p2p.read().await;
        let stats = network.get_stats().await.map_err(|_| {
            warp::reject::custom(ApiError::new("Failed to get network stats"))
        })?;

        Ok(warp::reply::json(&stats))
    }

    async fn send_transaction_handler(
        mempool: Option<Arc<RwLock<Mempool>>>,
        body: serde_json::Value,
    ) -> Result<impl warp::Reply, warp::Rejection> {
        log::info!("Received transaction via REST: {:?}", body);

        let mempool = mempool.ok_or_else(|| {
            warp::reject::custom(ApiError::new("Mempool not available"))
        })?;

        // Parse transaction from JSON
        let result = tokio::task::spawn_blocking(move || parse_and_process_transaction(body)).await;

        match result {
            Ok(Ok((transaction, tx_hash))) => {
                // Add transaction to mempool
                mempool.write().await.add_transaction(transaction).await.map_err(|e| {
                    warp::reject::custom(ApiError::new(&format!("Failed to add transaction: {}", e)))
                })?;

                log::info!("Transaction {} added to mempool", tx_hash);

                Ok(warp::reply::json(&serde_json::json!({
                    "success": true,
                    "transactionHash": tx_hash,
                    "status": "pending"
                })))
            }
            Ok(Err(e)) => {
                Ok(warp::reply::json(&serde_json::json!({
                    "success": false,
                    "error": e.to_string()
                })))
            }
            Err(e) => {
                Ok(warp::reply::json(&serde_json::json!({
                    "success": false,
                    "error": format!("Internal error: {}", e)
                })))
            }
        }
    }

    /// Background mining task - simulates validator mining for development
    async fn start_background_mining(&self) {
        log::info!("Starting background mining task (development mode)");

        let mut interval = tokio::time::interval(tokio::time::Duration::from_secs(5)); // Mine every 5 seconds

        loop {
            interval.tick().await;

            if let Err(e) = self.process_pending_transactions().await {
                log::warn!("Background mining error: {}", e);
            }
        }
    }

    /// Process transactions from mempool and create blocks
    async fn process_pending_transactions(&self) -> Result<(), BlockchainError> {
        // Only mine if we have blockchain and mempool configured
        let (blockchain, mempool) = match (&self.persistent_blockchain, &self.mempool) {
            (Some(bc), Some(mp)) => (bc, mp),
            _ => return Ok(()), // No-op if not configured
        };

        // Get transactions from mempool (limit to 10 per block for development)
        let mp_guard = mempool.read().await;
        let transactions = mp_guard.get_highest_fee_transactions(10, 1024 * 1024).await;
        drop(mp_guard);

        if transactions.is_empty() {
            return Ok(());
        }

        // Create new block
        let mut chain = blockchain.write().await;
        let latest_height = chain.get_latest_height().await.unwrap_or(0);
        let latest_hash = if latest_height > 0 {
            chain.get_block_by_height(latest_height).await
                .ok().flatten()
                .map(|b| b.hash())
                .unwrap_or_else(|| crate::core::types::Hash::zero())
        } else {
            crate::core::types::Hash::zero()
        };

        let new_block = crate::core::Block::new(
            latest_height + 1,
            latest_hash,
            transactions.clone(),
            "dev-validator".to_string(), // Development validator
            1000, // difficulty
        );

        // Add block to chain (this updates balances and confirms transactions)
        chain.add_block(new_block).await?;

        // Remove mined transactions from mempool
        let mp_guard = mempool.read().await;
        for tx in &transactions {
            let _ = mp_guard.remove_transaction(&tx.hash()).await;
        }

        log::info!("📦 Block {} mined with {} transactions", latest_height + 1, transactions.len());

        Ok(())
    }

    /// Real mempool integration - NO simulation of instant mining
    #[allow(dead_code)]
    async fn send_transaction(&self, body: serde_json::Value) -> Result<RestResponse<serde_json::Value>, BlockchainError> {
        log::info!("Received transaction via REST: {:?}", body);

        // Parse transaction from JSON
        let result = tokio::task::spawn_blocking(move || parse_and_process_transaction(body)).await;

        match result {
            Ok(Ok((transaction, tx_hash))) => {
                // Add transaction to mempool - this is the REAL flow
                if let Some(mempool) = &self.mempool {
                    mempool.write().await.add_transaction(transaction).await?;
                    log::info!("Transaction {} added to mempool", tx_hash);
                } else {
                    return Err(BlockchainError::InvalidTransaction("No mempool configured".to_string()));
                }

                // Return pending status - transaction will be mined naturally by validators
                Ok(RestResponse {
                    success: true,
                    data: Some(serde_json::json!({
                        "transactionHash": tx_hash,
                        "status": "pending"
                    })),
                    error: None,
                })
            }
            Ok(Err(e)) => Ok(RestResponse {
                success: false,
                data: None,
                error: Some(e.to_string()),
            }),
            Err(e) => Ok(RestResponse {
                success: false,
                data: None,
                error: Some(format!("Internal error: {}", e)),
            }),
        }
    }

    #[allow(dead_code)]
    fn format_transaction(
        tx: &Transaction,
        block: Option<&Block>,
        block_height: Option<u64>,
        tx_index: Option<usize>,
    ) -> serde_json::Value {
        serde_json::json!({
            "hash": tx.hash().to_string(),
            "type": format!("{:?}", tx.transaction_type),
            "from": tx.from.to_string(),
            "to": tx.to.to_string(),
            "amount": tx.amount,
            "fee": tx.fee,
            "nonce": tx.nonce,
            "timestamp": tx.timestamp,
            "blockHash": block.map(|b| b.hash().to_string()),
            "blockHeight": block_height,
            "index": tx_index,
            "status": if block.is_some() { "confirmed" } else { "pending" }
        })
    }
}
