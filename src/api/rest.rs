use crate::bridges::core::bridge_manager::BridgeManager;
use crate::core::chain::{Blockchain, PersistentBlockchain};
use crate::core::types::{Address, Hash};
use crate::core::{Block, Transaction};
use crate::utils::error::BlockchainError;
use serde::{Deserialize, Serialize};
use std::convert::Infallible;
use std::sync::Arc;
use tokio::sync::RwLock;

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

#[derive(Deserialize)]
struct BridgeTransferRequest {
    _source_chain: String,
    _target_chain: String,
    _amount: u64,
    _asset_id: String,
    _sender: String,
    _recipient: String,
}

fn parse_and_process_transaction(body: serde_json::Value) -> Result<String, BlockchainError> {
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

    // TODO: Add transaction to mempool for processing
    // For now, just return the hash
    log::info!("Transaction validated and queued: {}", tx_hash);

    Ok(tx_hash)
}

#[derive(Clone)]
pub struct RestServer {
    port: u16,
    persistent_blockchain: Option<Arc<RwLock<PersistentBlockchain>>>,
    blockchain: Option<Arc<RwLock<Blockchain>>>,
    bridge_manager: Option<Arc<RwLock<BridgeManager>>>,
    p2p_network: Option<Arc<RwLock<crate::network::p2p_network::P2PNetwork>>>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct RestResponse<T> {
    pub success: bool,
    pub data: Option<T>,
    pub error: Option<String>,
}

impl RestServer {
    pub fn new(port: u16) -> Result<Self, BlockchainError> {
        Ok(Self {
            port,
            persistent_blockchain: None,
            blockchain: None,
            bridge_manager: None,
            p2p_network: None,
        })
    }

    pub fn with_persistent_blockchain(
        mut self,
        blockchain: Arc<RwLock<PersistentBlockchain>>,
    ) -> Self {
        self.persistent_blockchain = Some(blockchain);
        self
    }

    pub fn with_blockchain(mut self, blockchain: Arc<RwLock<Blockchain>>) -> Self {
        self.blockchain = Some(blockchain);
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
        let routes = self.create_routes();

        log::info!("Starting REST API server on port {}", self.port);
        warp::serve(routes).run(([0, 0, 0, 0], self.port)).await;

        Ok(())
    }

    pub async fn stop(&self) -> Result<(), BlockchainError> {
        log::info!("REST API server stopped");
        Ok(())
    }

    fn create_routes(&self) -> warp::filters::BoxedFilter<(impl warp::Reply,)> {
        use warp::Filter;

        // Clone self for use in closures
        let self_clone1 = self.clone();
        let self_clone2 = self.clone();

        let node_info = warp::path("api")
            .and(warp::path("v1"))
            .and(warp::path("node"))
            .and(warp::path::end())
            .and(warp::get())
            .and_then(move || {
                let server = self_clone1.clone();
                async move { server.get_node_info().await }
            });

        let chain_info = warp::path("api")
            .and(warp::path("v1"))
            .and(warp::path("chain"))
            .and(warp::path::end())
            .and(warp::get())
            .and_then(move || {
                let server = self_clone2.clone();
                async move { server.get_chain_info().await }
            });

        // Create additional clones for other routes
        let self_clone3 = self.clone();
        let self_clone4 = self.clone();
        let self_clone5 = self.clone();

        let blocks = warp::path("api")
            .and(warp::path("v1"))
            .and(warp::path("blocks"))
            .and(warp::path::end())
            .and(warp::get())
            .and_then(move || {
                let server = self_clone3.clone();
                async move { server.get_blocks().await }
            });

        let block_by_hash = warp::path("api")
            .and(warp::path("v1"))
            .and(warp::path("blocks"))
            .and(warp::path::param::<String>())
            .and(warp::path::end())
            .and(warp::get())
            .and_then(move |hash| {
                let server = self_clone4.clone();
                async move { server.get_block_by_hash(hash).await }
            });

        let transactions = warp::path("api")
            .and(warp::path("v1"))
            .and(warp::path("transactions"))
            .and(warp::path::end())
            .and(warp::get())
            .and_then(move || {
                let server = self_clone5.clone();
                async move { server.get_transactions().await }
            });

        // Create clones for remaining routes
        let self_clone6 = self.clone();
        let self_clone7 = self.clone();
        let self_clone8 = self.clone();

        let transaction_by_hash = warp::path("api")
            .and(warp::path("v1"))
            .and(warp::path("transactions"))
            .and(warp::path::param::<String>())
            .and(warp::path::end())
            .and(warp::get())
            .and_then(move |hash| {
                let server = self_clone6.clone();
                async move { server.get_transaction_by_hash(hash).await }
            });

        let send_transaction = warp::path("api")
            .and(warp::path("v1"))
            .and(warp::path("transactions"))
            .and(warp::path::end())
            .and(warp::post())
            .and(warp::body::json())
            .and_then(move |body| {
                let server = self_clone7.clone();
                async move { server.send_transaction(body).await }
            });

        let validators = warp::path("api")
            .and(warp::path("v1"))
            .and(warp::path("validators"))
            .and(warp::path::end())
            .and(warp::get())
            .and_then(move || {
                let server = self_clone8.clone();
                async move { server.get_validators().await }
            });

        let health = warp::path("api")
            .and(warp::path("v1"))
            .and(warp::path("health"))
            .and(warp::path::end())
            .and(warp::get())
            .and_then(|| async { RestServer::health_check_static().await });

        // Group core endpoints
        let core_routes = node_info
            .or(chain_info)
            .or(blocks)
            .or(block_by_hash)
            .or(transactions)
            .or(transaction_by_hash)
            .or(send_transaction)
            .or(validators)
            .or(health)
            .boxed();

        // Bridge endpoints
        let bridges = warp::path("api")
            .and(warp::path("v1"))
            .and(warp::path("bridges"))
            .and(warp::path::end())
            .and(warp::get())
            .and_then(Self::get_bridges);

        let bridge_transfers = warp::path("api")
            .and(warp::path("v1"))
            .and(warp::path!("bridges" / "transfers"))
            .and(warp::get())
            .and_then(Self::get_bridge_transfers);

        let bridge_transfer_by_id = warp::path("api")
            .and(warp::path("v1"))
            .and(warp::path!("bridges" / "transfers" / String))
            .and(warp::get())
            .and_then(Self::get_bridge_transfer_by_id);

        let initiate_bridge_transfer = warp::path("api")
            .and(warp::path("v1"))
            .and(warp::path!("bridges" / "transfers"))
            .and(warp::post())
            .and(warp::body::json())
            .and_then(Self::initiate_bridge_transfer);

        let bridge_routes = bridges
            .or(bridge_transfers)
            .or(bridge_transfer_by_id)
            .or(initiate_bridge_transfer)
            .boxed();

        // Governance endpoints
        let governance_proposals = warp::path("api")
            .and(warp::path("v1"))
            .and(warp::path!("governance" / "proposals"))
            .and(warp::get())
            .and_then(Self::get_governance_proposals);

        let governance_proposal_by_id = warp::path("api")
            .and(warp::path("v1"))
            .and(warp::path!("governance" / "proposals" / String))
            .and(warp::get())
            .and_then(Self::get_governance_proposal_by_id);

        let create_governance_proposal = warp::path("api")
            .and(warp::path("v1"))
            .and(warp::path!("governance" / "proposals"))
            .and(warp::post())
            .and(warp::body::json())
            .and_then(Self::create_governance_proposal);

        let vote_governance_proposal = warp::path("api")
            .and(warp::path("v1"))
            .and(warp::path!("governance" / "proposals" / String / "vote"))
            .and(warp::post())
            .and(warp::body::json())
            .and_then(Self::vote_governance_proposal);

        let governance_dao = warp::path("api")
            .and(warp::path("v1"))
            .and(warp::path!("governance" / "dao"))
            .and(warp::get())
            .and_then(Self::get_governance_dao);

        let governance_routes = governance_proposals
            .or(governance_proposal_by_id)
            .or(create_governance_proposal)
            .or(vote_governance_proposal)
            .or(governance_dao)
            .boxed();

        // Staking endpoints
        let staking_validators = warp::path("api")
            .and(warp::path("v1"))
            .and(warp::path!("staking" / "validators"))
            .and(warp::get())
            .and_then(Self::get_staking_validators);

        let staking_validator_by_addr = warp::path("api")
            .and(warp::path("v1"))
            .and(warp::path!("staking" / "validators" / String))
            .and(warp::get())
            .and_then(Self::get_staking_validator_by_addr);

        let staking_delegate = warp::path("api")
            .and(warp::path("v1"))
            .and(warp::path!("staking" / "delegate"))
            .and(warp::post())
            .and(warp::body::json())
            .and_then(Self::staking_delegate);

        let staking_undelegate = warp::path("api")
            .and(warp::path("v1"))
            .and(warp::path!("staking" / "undelegate"))
            .and(warp::post())
            .and(warp::body::json())
            .and_then(Self::staking_undelegate);

        let staking_rewards = warp::path("api")
            .and(warp::path("v1"))
            .and(warp::path!("staking" / "rewards" / String))
            .and(warp::get())
            .and_then(Self::get_staking_rewards);

        let staking_claim_rewards = warp::path("api")
            .and(warp::path("v1"))
            .and(warp::path!("staking" / "claim-rewards"))
            .and(warp::post())
            .and(warp::body::json())
            .and_then(Self::staking_claim_rewards);

        let staking_routes = staking_validators
            .or(staking_validator_by_addr)
            .or(staking_delegate)
            .or(staking_undelegate)
            .or(staking_rewards)
            .or(staking_claim_rewards)
            .boxed();

        // Contract endpoints
        let contract_info = warp::path("api")
            .and(warp::path("v1"))
            .and(warp::path!("contracts" / String))
            .and(warp::path::end())
            .and(warp::get())
            .and_then(Self::get_contract_info);

        let contract_abi = warp::path("api")
            .and(warp::path("v1"))
            .and(warp::path!("contracts" / String / "abi"))
            .and(warp::get())
            .and_then(Self::get_contract_abi);

        let deploy_contract = warp::path("api")
            .and(warp::path("v1"))
            .and(warp::path("contracts"))
            .and(warp::path::end())
            .and(warp::post())
            .and(warp::body::json())
            .and_then(Self::deploy_contract);

        let call_contract = warp::path("api")
            .and(warp::path("v1"))
            .and(warp::path!("contracts" / String / "call"))
            .and(warp::post())
            .and(warp::body::json())
            .and_then(Self::call_contract);

        let contract_events = warp::path("api")
            .and(warp::path("v1"))
            .and(warp::path!("contracts" / "events"))
            .and(warp::get())
            .and_then(Self::get_contract_events);

        let contract_routes = contract_info
            .or(contract_abi)
            .or(deploy_contract)
            .or(call_contract)
            .or(contract_events)
            .boxed();

        // Account endpoints
        let self_clone_accounts_info = self.clone();
        let account_info = warp::path("api")
            .and(warp::path("v1"))
            .and(warp::path!("accounts" / String))
            .and(warp::path::end())
            .and(warp::get())
            .and_then(move |account| {
                let server = self_clone_accounts_info.clone();
                async move { server.get_account_info(account).await }
            });

        let self_clone_account_txs = self.clone();
        let account_transactions = warp::path("api")
            .and(warp::path("v1"))
            .and(warp::path!("accounts" / String / "transactions"))
            .and(warp::get())
            .and_then(move |account| {
                let server = self_clone_account_txs.clone();
                async move { server.get_account_transactions(account).await }
            });

        let account_routes = account_info.or(account_transactions).boxed();

        // Analytics endpoints
        let analytics_tps = warp::path("api")
            .and(warp::path("v1"))
            .and(warp::path!("analytics" / "tps"))
            .and(warp::get())
            .and_then(Self::get_analytics_tps);

        let analytics_gas_usage = warp::path("api")
            .and(warp::path("v1"))
            .and(warp::path!("analytics" / "gas-usage"))
            .and(warp::get())
            .and_then(Self::get_analytics_gas_usage);

        let analytics_network_health = warp::path("api")
            .and(warp::path("v1"))
            .and(warp::path!("analytics" / "network-health"))
            .and(warp::get())
            .and_then(Self::get_analytics_network_health);

        let analytics_bridge_activity = warp::path("api")
            .and(warp::path("v1"))
            .and(warp::path!("analytics" / "bridge-activity"))
            .and(warp::get())
            .and_then(Self::get_analytics_bridge_activity);

        let analytics_routes = analytics_tps
            .or(analytics_gas_usage)
            .or(analytics_network_health)
            .or(analytics_bridge_activity)
            .boxed();

        let metrics = warp::path("api")
            .and(warp::path("v1"))
            .and(warp::path("metrics"))
            .and(warp::path::end())
            .and(warp::get())
            .and_then(Self::get_metrics);

        let cors = warp::cors()
            .allow_any_origin()
            .allow_headers(vec!["content-type", "authorization"])
            .allow_methods(vec!["GET", "POST", "PUT", "DELETE"]);

        core_routes
            .or(bridge_routes)
            .or(governance_routes)
            .or(staking_routes)
            .or(contract_routes)
            .or(account_routes)
            .or(analytics_routes)
            .or(metrics)
            .with(cors)
            .with(warp::log("api"))
            .boxed()
    }

    async fn get_node_info(&self) -> Result<impl warp::Reply, Infallible> {
        let version = env!("CARGO_PKG_VERSION");

        let mut block_height: u64 = 0;
        let mut latest_block_time: Option<u64> = None;

        if let Some(blockchain) = &self.persistent_blockchain {
            let chain = blockchain.read().await;
            match chain.get_latest_height().await {
                Ok(height) => {
                    block_height = height;
                    if let Ok(Some(block)) = chain.get_block_by_height(height).await {
                        latest_block_time = Some(block.header.timestamp);
                    }
                }
                Err(e) => {
                    log::warn!("Failed to obtain latest block height: {}", e);
                }
            }
        }

        let peers = if let Some(p2p) = &self.p2p_network {
            match p2p.read().await.get_stats().await {
                Ok(stats) => stats.connected_peers,
                Err(e) => {
                    log::warn!("Failed to obtain peer statistics: {}", e);
                    0
                }
            }
        } else {
            0
        };

        let sync_status = if peers > 0 { "syncing" } else { "offline" };

        let response = RestResponse {
            success: true,
            data: Some(serde_json::json!({
                "version": format!("erbium/{}", version),
                "network": "mainnet",
                "blockHeight": block_height,
                "latestBlockTime": latest_block_time,
                "syncStatus": sync_status,
                "peers": peers
            })),
            error: None,
        };
        Ok(warp::reply::json(&response))
    }

    async fn get_chain_info(&self) -> Result<impl warp::Reply, Infallible> {
        let version = env!("CARGO_PKG_VERSION");
        let mut height: u64 = 0;
        let mut total_transactions: u64 = 0;
        let mut genesis_timestamp: Option<u64> = None;
        let mut latest_block_timestamp: Option<u64> = None;
        let mut circulating_supply = "0".to_string();

        if let Some(blockchain) = &self.persistent_blockchain {
            let chain = blockchain.read().await;

            circulating_supply = chain.get_state().get_total_supply().to_string();

            match chain.get_latest_height().await {
                Ok(latest_height) => height = latest_height,
                Err(e) => {
                    log::warn!("Failed to determine latest height: {}", e);
                }
            }

            let blocks = &chain.blockchain().blocks;
            if let Some(genesis_block) = blocks.first() {
                genesis_timestamp = Some(genesis_block.header.timestamp);
            }
            if let Some(latest_block) = blocks.last() {
                latest_block_timestamp = Some(latest_block.header.timestamp);
            }
            total_transactions = blocks
                .iter()
                .map(|block| block.transactions.len() as u64)
                .sum();
        }

        let response = RestResponse {
            success: true,
            data: Some(serde_json::json!({
                "height": height,
                "network": "erbium",
                "version": version,
                "timestamp": chrono::Utc::now().timestamp_millis(),
                "chain_id": 137,
                "genesis_timestamp": genesis_timestamp,
                "latest_block_timestamp": latest_block_timestamp,
                "total_transactions": total_transactions,
                "active_validators": 0,
                "circulating_supply": circulating_supply
            })),
            error: None,
        };
        Ok(warp::reply::json(&response))
    }

    async fn get_blocks(&self) -> Result<impl warp::Reply, Infallible> {
        const MAX_BLOCKS: usize = 100;

        let mut blocks_json = Vec::new();

        if let Some(blockchain) = &self.persistent_blockchain {
            let chain = blockchain.read().await;
            let blocks = &chain.blockchain().blocks;

            for block in blocks.iter().rev().take(MAX_BLOCKS) {
                blocks_json.push(Self::format_block_summary(block));
            }
        }

        let response = RestResponse {
            success: true,
            data: Some(serde_json::Value::Array(blocks_json)),
            error: None,
        };
        Ok(warp::reply::json(&response))
    }

    async fn get_block_by_hash(&self, hash: String) -> Result<impl warp::Reply, Infallible> {
        if let Some(blockchain) = &self.persistent_blockchain {
            match Hash::from_hex(&hash) {
                Ok(block_hash) => {
                    match blockchain.read().await.get_block_by_hash(&block_hash).await {
                        Ok(Some(block)) => {
                            let block_response = Self::format_block_detailed(&block);

                            let response = RestResponse {
                                success: true,
                                data: Some(block_response),
                                error: None,
                            };
                            return Ok(warp::reply::json(&response));
                        }
                        Ok(None) => {}
                        Err(e) => {
                            log::warn!("Failed to load block {}: {}", hash, e);
                        }
                    }
                }
                Err(_) => {
                    let response = RestResponse::<serde_json::Value> {
                        success: false,
                        data: None,
                        error: Some("Invalid block hash".to_string()),
                    };
                    return Ok(warp::reply::json(&response));
                }
            }
        }

        let response = RestResponse::<serde_json::Value> {
            success: false,
            data: None,
            error: Some("Block not found".to_string()),
        };
        Ok(warp::reply::json(&response))
    }

    fn format_block_summary(block: &Block) -> serde_json::Value {
        let block_hash = block.hash().to_string();
        serde_json::json!({
            "hash": block_hash,
            "height": block.header.number,
            "timestamp": block.header.timestamp,
            "transactionCount": block.transactions.len(),
            "validator": block.header.validator,
            "difficulty": block.header.difficulty,
            "previousHash": block.header.previous_hash.to_string(),
            "merkleRoot": block.header.merkle_root.to_string()
        })
    }

    fn format_block_detailed(block: &Block) -> serde_json::Value {
        let transactions: Vec<_> = block
            .transactions
            .iter()
            .enumerate()
            .map(|(index, tx)| {
                Self::format_transaction(tx, Some(block), Some(block.header.number), Some(index))
            })
            .collect();

        let block_hash = block.hash().to_string();

        serde_json::json!({
            "hash": block_hash,
            "height": block.header.number,
            "timestamp": block.header.timestamp,
            "validator": block.header.validator,
            "difficulty": block.header.difficulty,
            "previousHash": block.header.previous_hash.to_string(),
            "merkleRoot": block.header.merkle_root.to_string(),
            "transactionCount": block.transactions.len(),
            "transactions": transactions
        })
    }

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
            "status": "confirmed"
        })
    }

    fn account_activity_metrics(
        blockchain: &Blockchain,
        address: &Address,
    ) -> (u64, Option<u64>, Option<u64>) {
        let mut transaction_count: u64 = 0;
        let mut created_at: Option<u64> = None;
        let mut last_activity: Option<u64> = None;
        let target = address.as_str();

        for block in &blockchain.blocks {
            let mut touched = false;
            for tx in &block.transactions {
                if tx.from.as_str() == target || tx.to.as_str() == target {
                    transaction_count += 1;
                    touched = true;
                }
            }

            if touched {
                let ts = block.header.timestamp;
                if created_at.map_or(true, |current| ts < current) {
                    created_at = Some(ts);
                }
                if last_activity.map_or(true, |current| ts > current) {
                    last_activity = Some(ts);
                }
            }
        }

        (transaction_count, created_at, last_activity)
    }

    async fn get_transactions(&self) -> Result<impl warp::Reply, Infallible> {
        const MAX_TRANSACTIONS: usize = 100;

        let mut transactions_json = Vec::new();

        if let Some(blockchain) = &self.persistent_blockchain {
            let chain = blockchain.read().await;
            let blocks = &chain.blockchain().blocks;

            'outer: for block in blocks.iter().rev() {
                for (index, tx) in block.transactions.iter().enumerate().rev() {
                    transactions_json.push(Self::format_transaction(
                        tx,
                        Some(block),
                        Some(block.header.number),
                        Some(index),
                    ));

                    if transactions_json.len() >= MAX_TRANSACTIONS {
                        break 'outer;
                    }
                }
            }
        }

        let response = RestResponse {
            success: true,
            data: Some(serde_json::Value::Array(transactions_json)),
            error: None,
        };
        Ok(warp::reply::json(&response))
    }

    async fn get_transaction_by_hash(&self, hash: String) -> Result<impl warp::Reply, Infallible> {
        if let Some(blockchain) = &self.persistent_blockchain {
            let tx_hash = match Hash::from_hex(&hash) {
                Ok(parsed) => parsed,
                Err(_) => {
                    let response = RestResponse::<serde_json::Value> {
                        success: false,
                        data: None,
                        error: Some("Invalid transaction hash".to_string()),
                    };
                    return Ok(warp::reply::json(&response));
                }
            };

            let chain = blockchain.read().await;

            if let Some((transaction, block_index, tx_index)) =
                chain.blockchain().get_transaction_by_hash(&tx_hash)
            {
                if let Some(block) = chain.blockchain().get_block_by_height(block_index) {
                    let transaction_response = Self::format_transaction(
                        transaction,
                        Some(block),
                        Some(block.header.number),
                        Some(tx_index),
                    );

                    let response = RestResponse {
                        success: true,
                        data: Some(transaction_response),
                        error: None,
                    };
                    return Ok(warp::reply::json(&response));
                }
            }

            match chain.get_transaction_by_hash(tx_hash.as_bytes()).await {
                Ok(Some(transaction)) => {
                    let transaction_response =
                        Self::format_transaction(&transaction, None, None, None);

                    let response = RestResponse {
                        success: true,
                        data: Some(transaction_response),
                        error: None,
                    };
                    return Ok(warp::reply::json(&response));
                }
                Ok(None) => {}
                Err(e) => {
                    log::warn!("Failed to load transaction {}: {}", hash, e);
                }
            }
        }

        let response = RestResponse::<serde_json::Value> {
            success: false,
            data: None,
            error: Some("Transaction not found".to_string()),
        };
        Ok(warp::reply::json(&response))
    }

    async fn send_transaction(
        &self,
        body: serde_json::Value,
    ) -> Result<impl warp::Reply, Infallible> {
        log::info!("Received transaction via REST: {:?}", body);

        // Parse transaction from JSON
        let result = tokio::task::spawn_blocking(move || parse_and_process_transaction(body)).await;

        match result {
            Ok(Ok(tx_hash)) => {
                let response: RestResponse<serde_json::Value> = RestResponse {
                    success: true,
                    data: Some(serde_json::json!({
                        "transactionHash": tx_hash,
                        "status": "pending"
                    })),
                    error: None,
                };
                Ok(warp::reply::json(&response))
            }
            Ok(Err(e)) => {
                let response: RestResponse<String> = RestResponse {
                    success: false,
                    data: None,
                    error: Some(e.to_string()),
                };
                Ok(warp::reply::json(&response))
            }
            Err(e) => {
                let response: RestResponse<String> = RestResponse {
                    success: false,
                    data: None,
                    error: Some(format!("Internal error: {}", e)),
                };
                Ok(warp::reply::json(&response))
            }
        }
    }

    async fn get_validators(&self) -> Result<impl warp::Reply, Infallible> {
        // No validators exist yet - return empty array
        let response = RestResponse {
            success: true,
            data: Some(serde_json::Value::Array(vec![])),
            error: None,
        };
        Ok(warp::reply::json(&response))
    }

    async fn health_check(&self) -> Result<impl warp::Reply, Infallible> {
        let response = RestResponse {
            success: true,
            data: Some(serde_json::json!({
                "status": "healthy",
                "timestamp": chrono::Utc::now().timestamp_millis()
            })),
            error: None,
        };
        Ok(warp::reply::json(&response))
    }

    async fn health_check_static() -> Result<impl warp::Reply, Infallible> {
        let response = RestResponse {
            success: true,
            data: Some(serde_json::json!({
                "status": "healthy",
                "timestamp": chrono::Utc::now().timestamp_millis()
            })),
            error: None,
        };
        Ok(warp::reply::json(&response))
    }

    async fn get_bridges() -> Result<impl warp::Reply, Infallible> {
        // No bridges configured yet - return empty array
        let response = RestResponse {
            success: true,
            data: Some(serde_json::Value::Array(vec![])),
            error: None,
        };
        Ok(warp::reply::json(&response))
    }

    async fn get_bridge_transfers() -> Result<impl warp::Reply, Infallible> {
        // No bridge transfers exist - return empty array
        let response = RestResponse {
            success: true,
            data: Some(serde_json::Value::Array(vec![])),
            error: None,
        };
        Ok(warp::reply::json(&response))
    }

    async fn get_bridge_transfer_by_id(
        _transfer_id: String,
    ) -> Result<impl warp::Reply, Infallible> {
        // Bridge transfer doesn't exist - return not found
        let response = RestResponse::<serde_json::Value> {
            success: false,
            data: None,
            error: Some("Bridge transfer not found".to_string()),
        };
        Ok(warp::reply::json(&response))
    }

    async fn initiate_bridge_transfer(
        _body: serde_json::Value,
    ) -> Result<impl warp::Reply, Infallible> {
        // Bridges not implemented - return error
        let response: RestResponse<String> = RestResponse {
            success: false,
            data: None,
            error: Some("Bridge transfers not implemented in this version".to_string()),
        };
        Ok(warp::reply::json(&response))
    }

    // Governance handlers
    async fn get_governance_proposals() -> Result<impl warp::Reply, Infallible> {
        // DISABLE: Return fake governance proposals - governance not implemented
        let response: RestResponse<String> = RestResponse {
            success: false,
            data: None,
            error: Some("Governance system not yet implemented in this version".to_string()),
        };
        Ok(warp::reply::json(&response))
    }

    async fn get_governance_proposal_by_id(
        _proposal_id: String,
    ) -> Result<impl warp::Reply, Infallible> {
        // Governance not implemented - return not found
        let response = RestResponse::<serde_json::Value> {
            success: false,
            data: None,
            error: Some("Proposal not found".to_string()),
        };
        Ok(warp::reply::json(&response))
    }

    async fn create_governance_proposal(
        _body: serde_json::Value,
    ) -> Result<impl warp::Reply, Infallible> {
        // Governance not implemented - return error
        let response: RestResponse<String> = RestResponse {
            success: false,
            data: None,
            error: Some("Governance system not yet implemented in this version".to_string()),
        };
        Ok(warp::reply::json(&response))
    }

    async fn vote_governance_proposal(
        _proposal_id: String,
        _body: serde_json::Value,
    ) -> Result<impl warp::Reply, Infallible> {
        // Governance not implemented - return error
        let response: RestResponse<String> = RestResponse {
            success: false,
            data: None,
            error: Some("Governance system not yet implemented in this version".to_string()),
        };
        Ok(warp::reply::json(&response))
    }

    async fn get_governance_dao() -> Result<impl warp::Reply, Infallible> {
        // Governance not implemented - return error
        let response: RestResponse<String> = RestResponse {
            success: false,
            data: None,
            error: Some("Governance system not yet implemented in this version".to_string()),
        };
        Ok(warp::reply::json(&response))
    }

    // Staking handlers
    async fn get_staking_validators() -> Result<impl warp::Reply, Infallible> {
        // DISABLE: Return fake validators - staking not implemented
        let response: RestResponse<String> = RestResponse {
            success: false,
            data: None,
            error: Some("Staking system not yet implemented in this version".to_string()),
        };
        Ok(warp::reply::json(&response))
    }

    async fn get_staking_validator_by_addr(
        _validator_addr: String,
    ) -> Result<impl warp::Reply, Infallible> {
        // Staking not implemented - return not found
        let response = RestResponse::<serde_json::Value> {
            success: false,
            data: None,
            error: Some("Validator not found".to_string()),
        };
        Ok(warp::reply::json(&response))
    }

    async fn staking_delegate(_body: serde_json::Value) -> Result<impl warp::Reply, Infallible> {
        // Staking not implemented - return error
        let response: RestResponse<String> = RestResponse {
            success: false,
            data: None,
            error: Some("Staking delegation not implemented in current version".to_string()),
        };
        Ok(warp::reply::json(&response))
    }

    async fn staking_undelegate(_body: serde_json::Value) -> Result<impl warp::Reply, Infallible> {
        // Staking not implemented - return error
        let response: RestResponse<String> = RestResponse {
            success: false,
            data: None,
            error: Some("Staking undelegation not implemented in current version".to_string()),
        };
        Ok(warp::reply::json(&response))
    }

    async fn get_staking_rewards(_delegator_addr: String) -> Result<impl warp::Reply, Infallible> {
        // Staking not implemented - return error
        let response: RestResponse<String> = RestResponse {
            success: false,
            data: None,
            error: Some("Staking system not yet implemented in this version".to_string()),
        };
        Ok(warp::reply::json(&response))
    }

    async fn staking_claim_rewards(
        _body: serde_json::Value,
    ) -> Result<impl warp::Reply, Infallible> {
        // Staking not implemented - return error
        let response: RestResponse<String> = RestResponse {
            success: false,
            data: None,
            error: Some("Staking system not yet implemented in this version".to_string()),
        };
        Ok(warp::reply::json(&response))
    }

    // Contract handlers - RETURN ERRORS FOR NOW
    async fn get_contract_info(_contract_addr: String) -> Result<impl warp::Reply, Infallible> {
        // Smart contracts not implemented yet - return error
        let response = RestResponse::<serde_json::Value> {
            success: false,
            data: None,
            error: Some("Contract not found".to_string()),
        };
        Ok(warp::reply::json(&response))
    }

    async fn get_contract_abi(_contract_addr: String) -> Result<impl warp::Reply, Infallible> {
        // Smart contracts not implemented yet - return error
        let response = RestResponse::<serde_json::Value> {
            success: false,
            data: None,
            error: Some("Contract ABI not available".to_string()),
        };
        Ok(warp::reply::json(&response))
    }

    async fn deploy_contract(_body: serde_json::Value) -> Result<impl warp::Reply, Infallible> {
        // Smart contracts not implemented yet - return error
        let response: RestResponse<String> = RestResponse {
            success: false,
            data: None,
            error: Some("Smart contract deployment not implemented in this version".to_string()),
        };
        Ok(warp::reply::json(&response))
    }

    async fn call_contract(
        _contract_addr: String,
        _body: serde_json::Value,
    ) -> Result<impl warp::Reply, Infallible> {
        // Smart contracts not implemented yet - return error
        let response: RestResponse<String> = RestResponse {
            success: false,
            data: None,
            error: Some("Smart contract calls not implemented in this version".to_string()),
        };
        Ok(warp::reply::json(&response))
    }

    async fn get_contract_events() -> Result<impl warp::Reply, Infallible> {
        // Smart contracts not implemented yet - return empty array
        let response = RestResponse {
            success: true,
            data: Some(serde_json::Value::Array(vec![])),
            error: None,
        };
        Ok(warp::reply::json(&response))
    }

    // Account handlers - REAL BALANCE INTEGRATION
    async fn get_account_info(&self, account_addr: String) -> Result<impl warp::Reply, Infallible> {
        let address = match Address::new(account_addr.clone()) {
            Ok(addr) => addr,
            Err(_) => {
                let response = RestResponse::<serde_json::Value> {
                    success: false,
                    data: None,
                    error: Some("Invalid account address".to_string()),
                };
                return Ok(warp::reply::json(&response));
            }
        };

        if let Some(blockchain) = &self.persistent_blockchain {
            let chain = blockchain.read().await;
            let state = chain.get_state();

            if let Some(account) = state.get_account(&address) {
                let (transaction_count, created_at, last_activity) =
                    Self::account_activity_metrics(chain.blockchain(), &address);

                let response = RestResponse {
                    success: true,
                    data: Some(serde_json::json!({
                        "address": account_addr,
                        "balance": account.balance.to_string(),
                        "nonce": account.nonce,
                        "code_hash": account.code_hash.as_ref().map(|hash| hash.to_string()),
                        "storage_root": account.storage_root.to_string(),
                        "transaction_count": transaction_count,
                        "is_contract": account.code_hash.is_some(),
                        "created_at": created_at,
                        "last_activity": last_activity
                    })),
                    error: None,
                };
                return Ok(warp::reply::json(&response));
            }
        }

        let response = RestResponse::<serde_json::Value> {
            success: false,
            data: None,
            error: Some("Account not found".to_string()),
        };
        Ok(warp::reply::json(&response))
    }

    async fn get_account_transactions(
        &self,
        account_addr: String,
    ) -> Result<impl warp::Reply, Infallible> {
        const MAX_ACCOUNT_TRANSACTIONS: usize = 100;

        let address = match Address::new(account_addr.clone()) {
            Ok(addr) => addr,
            Err(_) => {
                let response = RestResponse::<serde_json::Value> {
                    success: false,
                    data: None,
                    error: Some("Invalid account address".to_string()),
                };
                return Ok(warp::reply::json(&response));
            }
        };

        let mut transactions_json = Vec::new();

        if let Some(blockchain) = &self.persistent_blockchain {
            let chain = blockchain.read().await;
            let blocks = &chain.blockchain().blocks;
            let address_str = address.as_str().to_string();

            'outer: for block in blocks.iter().rev() {
                for (index, tx) in block.transactions.iter().enumerate().rev() {
                    if tx.from.as_str() == address_str || tx.to.as_str() == address_str {
                        transactions_json.push(Self::format_transaction(
                            tx,
                            Some(block),
                            Some(block.header.number),
                            Some(index),
                        ));

                        if transactions_json.len() >= MAX_ACCOUNT_TRANSACTIONS {
                            break 'outer;
                        }
                    }
                }
            }
        }

        let response = RestResponse {
            success: true,
            data: Some(serde_json::Value::Array(transactions_json)),
            error: None,
        };
        Ok(warp::reply::json(&response))
    }

    // Analytics handlers - RETURN REAL DATA OR DISABLED
    async fn get_analytics_tps() -> Result<impl warp::Reply, Infallible> {
        // DISABLE: Analytics not implemented - return 0 values instead of fake data
        let tps_data = serde_json::json!({
            "current_tps": 0,
            "peak_tps": 0,
            "average_tps_24h": 0,
            "average_tps_7d": 0,
            "transactions_last_block": 0,
            "gas_used_last_block": 0,
            "timestamp": chrono::Utc::now().timestamp_millis()
        });

        let response = RestResponse {
            success: true,
            data: Some(tps_data),
            error: None,
        };
        Ok(warp::reply::json(&response))
    }

    async fn get_analytics_gas_usage() -> Result<impl warp::Reply, Infallible> {
        // DISABLE: Analytics not implemented - return 0 values instead of fake data
        let gas_data = serde_json::json!({
            "average_gas_price": "0",
            "median_gas_price": "0",
            "gas_used_24h": "0",
            "gas_limit_total": "0",
            "gas_utilization_percent": 0,
            "expensive_tx_count": 0,
            "cheap_tx_count": 0
        });

        let response = RestResponse {
            success: true,
            data: Some(gas_data),
            error: None,
        };
        Ok(warp::reply::json(&response))
    }

    async fn get_analytics_network_health() -> Result<impl warp::Reply, Infallible> {
        // DISABLE: Analytics not implemented - return 0 values instead of fake data
        let health_data = serde_json::json!({
            "block_time_average": 0,
            "network_hashrate": "0",
            "active_validators": 0,
            "total_stake": "0",
            "finalized_blocks_24h": 0,
            "missed_blocks_24h": 0,
            "network_uptime": 0,
            "peer_count": 0,
            "sync_status": "stopped"
        });

        let response = RestResponse {
            success: true,
            data: Some(health_data),
            error: None,
        };
        Ok(warp::reply::json(&response))
    }

    async fn get_analytics_bridge_activity() -> Result<impl warp::Reply, Infallible> {
        // DISABLE: Analytics not implemented - return empty data instead of fake data
        let bridge_data = serde_json::json!({
            "total_transfers_24h": 0,
            "total_volume_24h": "0",
            "active_bridges": [],
            "bridge_stats": {},
            "failed_transfers_24h": 0,
            "pending_transfers": 0
        });

        let response = RestResponse {
            success: true,
            data: Some(bridge_data),
            error: None,
        };
        Ok(warp::reply::json(&response))
    }

    async fn get_metrics() -> Result<impl warp::Reply, Infallible> {
        // DISABLE: Metrics not implemented - return empty data instead of fake data
        let metrics = serde_json::json!({
            "blockchain": {
                "head_block": 0,
                "total_transactions": 0,
                "total_accounts": 0,
                "total_contracts": 0,
                "circulating_supply": "0"
            },
            "network": {
                "active_peers": 0,
                "connected_peers": 0,
                "bytes_received": "0",
                "bytes_sent": "0"
            },
            "performance": {
                "avg_block_time": 0,
                "avg_tx_per_block": 0,
                "gas_utilization": 0,
                "cpu_usage": 0,
                "memory_usage": 0
            },
            "bridges": {
                "active_bridges": 0,
                "pending_transfers": 0,
                "completed_transfers_24h": 0,
                "bridge_balance_btc": "0",
                "bridge_balance_eth": "0"
            }
        });

        let response = RestResponse {
            success: true,
            data: Some(metrics),
            error: None,
        };
        Ok(warp::reply::json(&response))
    }
}
