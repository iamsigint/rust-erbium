use crate::bridges::core::bridge_manager::BridgeManager;
use crate::core::chain::{Blockchain, PersistentBlockchain};
use crate::core::mempool::{Mempool, MempoolConfig};
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
    blockchain: Option<Arc<RwLock<Blockchain>>>,
    mempool: Option<Arc<RwLock<Mempool>>>,
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
            mempool: None,
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

        Ok(())
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
        let transactions = {
            let mut mp = mempool.write().await;
            mp.get_highest_fee_transactions(10, 1024 * 1024)
        };

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
        let mut mp = mempool.write().await;
        for tx in &transactions {
            let _ = mp.remove_transaction(&tx.hash());
        }

        log::info!("📦 Block {} mined with {} transactions", latest_height + 1, transactions.len());

        Ok(())
    }

    /// Real mempool integration - NO simulation of instant mining
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
