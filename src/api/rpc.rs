// src/api/rpc.rs

use crate::core::{Address, Block, Blockchain, Transaction};
use crate::node::config::NodeConfig;
use jsonrpc_core::{IoHandler, Result, Value};
use jsonrpc_derive::rpc;
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::RwLock;
use tokio::time::timeout;

#[rpc]
pub trait BlockchainRPC {
    #[rpc(name = "eth_blockNumber")]
    fn eth_block_number(&self) -> Result<String>;

    #[rpc(name = "eth_getBlockByHash")]
    fn eth_get_block_by_hash(&self, hash: String, full_tx: bool) -> Result<Option<Value>>;

    #[rpc(name = "eth_getBlockByNumber")]
    fn eth_get_block_by_number(&self, number: String, full_tx: bool) -> Result<Option<Value>>;

    #[rpc(name = "eth_getTransactionByHash")]
    fn eth_get_transaction_by_hash(&self, hash: String) -> Result<Option<Value>>;

    #[rpc(name = "eth_sendTransaction")]
    fn eth_send_transaction(&self, tx_params: Value) -> Result<String>;

    #[rpc(name = "eth_getBalance")]
    fn eth_get_balance(&self, address: String, block: Option<String>) -> Result<String>;

    #[rpc(name = "net_version")]
    fn net_version(&self) -> Result<String>;

    #[rpc(name = "web3_clientVersion")]
    fn web3_client_version(&self) -> Result<String>;

    // Custom informational endpoint
    #[rpc(name = "erb_chainInfo")]
    fn erb_chain_info(&self) -> Result<Value>;
}

pub struct BlockchainRPCImpl {
    blockchain: Arc<RwLock<Blockchain>>,
    config: NodeConfig,
}

impl BlockchainRPCImpl {
    pub fn new(blockchain: Arc<RwLock<Blockchain>>, config: NodeConfig) -> Self {
        Self { blockchain, config }
    }

    /// Helper to acquire read lock with timeout
    #[allow(dead_code)]
    async fn read_blockchain_with_timeout(&self) -> Result<tokio::sync::RwLockReadGuard<'_, Blockchain>> {
        timeout(Duration::from_secs(5), self.blockchain.read())
            .await
            .map_err(|_| jsonrpc_core::Error::internal_error())
    }

    /// Helper to acquire write lock with timeout
    #[allow(dead_code)]
    async fn write_blockchain_with_timeout(&self) -> Result<tokio::sync::RwLockWriteGuard<'_, Blockchain>> {
        timeout(Duration::from_secs(5), self.blockchain.write())
            .await
            .map_err(|_| jsonrpc_core::Error::internal_error())
    }
}

impl BlockchainRPC for BlockchainRPCImpl {
    fn eth_block_number(&self) -> Result<String> {
        let blockchain = self
            .blockchain
            .try_read()
            .map_err(|e| jsonrpc_core::Error::invalid_params(e.to_string()))?;
        let height = blockchain.get_block_height();
        // Height is count; latest block number is height-1, clamp at 0
        let block_number = height.saturating_sub(1) as u64;
        Ok(format!("0x{:x}", block_number))
    }

    fn eth_get_block_by_hash(&self, hash: String, full_tx: bool) -> Result<Option<Value>> {
        let blockchain = self
            .blockchain
            .try_read()
            .map_err(|e| jsonrpc_core::Error::invalid_params(e.to_string()))?;

        // Use Hash::from_hex() which automatically normalizes "0x" prefix
        let block_hash = crate::core::types::Hash::from_hex(&hash)
            .map_err(|e| jsonrpc_core::Error::invalid_params(format!("Invalid hash: {}", e)))?;

        if let Some(block) = blockchain.get_block_by_hash(&block_hash) {
            Ok(Some(serialize_block(block, full_tx)))
        } else {
            Ok(None)
        }
    }

    fn eth_get_block_by_number(&self, number: String, full_tx: bool) -> Result<Option<Value>> {
        let blockchain = self
            .blockchain
            .try_read()
            .map_err(|e| jsonrpc_core::Error::invalid_params(e.to_string()))?;

        let block_number = parse_block_number(&number)?;

        if let Some(block) = blockchain.get_block_by_height(block_number as usize) {
            Ok(Some(serialize_block(block, full_tx)))
        } else {
            Ok(None)
        }
    }

    fn eth_get_transaction_by_hash(&self, hash: String) -> Result<Option<Value>> {
        let blockchain = self
            .blockchain
            .try_read()
            .map_err(|e| jsonrpc_core::Error::invalid_params(e.to_string()))?;

        // Use Hash::from_hex() which automatically normalizes "0x" prefix
        let tx_hash = crate::core::types::Hash::from_hex(&hash)
            .map_err(|e| jsonrpc_core::Error::invalid_params(format!("Invalid hash: {}", e)))?;

        if let Some((tx, block_height, tx_index)) = blockchain.get_transaction_by_hash(&tx_hash) {
            // Get block info for context
            if let Some(block) = blockchain.get_block_by_height(block_height) {
                let mut serialized_tx = serialize_transaction(tx);
                if let Some(obj) = serialized_tx.as_object_mut() {
                    obj.insert(
                        "blockHash".to_string(),
                        serde_json::Value::String(format!(
                            "0x{}",
                            hex::encode(block.hash().as_bytes())
                        )),
                    );
                    obj.insert(
                        "blockNumber".to_string(),
                        serde_json::Value::String(format!("0x{:x}", block.header.number)),
                    );
                    obj.insert(
                        "transactionIndex".to_string(),
                        serde_json::Value::String(format!("0x{:x}", tx_index)),
                    );
                }
                Ok(Some(serialized_tx))
            } else {
                Ok(Some(serialize_transaction(tx)))
            }
        } else {
            Ok(None)
        }
    }

    fn eth_send_transaction(&self, tx_params: Value) -> Result<String> {
        // Parse transaction parameters
        let from_str = tx_params
            .get("from")
            .and_then(|v| v.as_str())
            .ok_or_else(|| jsonrpc_core::Error::invalid_params("Missing 'from' field"))?;

        let to_str = tx_params.get("to").and_then(|v| v.as_str());

        let value = tx_params
            .get("value")
            .and_then(|v| v.as_str())
            .and_then(|s| parse_hex_number(s).ok())
            .unwrap_or(0);

        let from = Address::new(from_str.to_string()).map_err(|e| {
            jsonrpc_core::Error::invalid_params(format!("Invalid 'from' address: {}", e))
        })?;

        // Handle `to` address
        let to = match to_str {
            Some(s) => Address::new(s.to_string()).map_err(|e| {
                jsonrpc_core::Error::invalid_params(format!("Invalid 'to' address: {}", e))
            })?,
            // Use a default zero address if 'to' is not provided
            None => Address::new("0x0000000000000000000000000000000000000000".to_string())
                .map_err(|e| {
                    jsonrpc_core::Error::invalid_params(format!("Invalid default address: {}", e))
                })?,
        };

        // Acquire state to fetch current nonce
        let mut blockchain = self
            .blockchain
            .try_write()
            .map_err(|e| jsonrpc_core::Error::invalid_params(e.to_string()))?;
        let current_nonce = blockchain.state.get_nonce(&from).unwrap_or(0);

        // Create transaction using new_transfer with correct nonce
        let transaction = Transaction::new_transfer(
            from,
            to,
            value,
            1, // fee placeholder
            current_nonce,
        );

        // Dev shortcut: apply directly if enabled
        if self.config.dev_apply_on_send {
            if let Err(e) = blockchain.state.apply_transaction(&transaction) {
                return Err(jsonrpc_core::Error::invalid_params(format!(
                    "Transaction application failed: {}",
                    e
                )));
            }
        } else {
            log::warn!("eth_send_transaction is not fully implemented (mempool)");
        }
        Ok(format!("0x{}", hex::encode(transaction.hash())))
    }

    fn eth_get_balance(&self, address: String, _block: Option<String>) -> Result<String> {
        let blockchain = self
            .blockchain
            .try_read()
            .map_err(|e| jsonrpc_core::Error::invalid_params(e.to_string()))?;

        let addr = Address::new(address)
            .map_err(|e| jsonrpc_core::Error::invalid_params(format!("Invalid address: {}", e)))?;
        let balance = blockchain.state.get_balance(&addr).unwrap_or(0);
        Ok(format!("0x{:x}", balance))
    }

    fn net_version(&self) -> Result<String> {
        Ok(format!("{}", self.config.chain_id))
    }

    fn web3_client_version(&self) -> Result<String> {
        Ok("ErbiumBlockchain/0.1.0".to_string())
    }

    fn erb_chain_info(&self) -> Result<Value> {
        Ok(serde_json::json!({
            "name": self.config.asset_name,
            "symbol": self.config.asset_symbol,
            "decimals": self.config.asset_decimals,
            "chainId": format!("0x{:x}", self.config.chain_id),
            "gasPrice": format!("0x{:x}", self.config.gas_price_wei),
            "client": "ErbiumBlockchain/0.1.0"
        }))
    }
}

pub struct RpcServer {
    handler: IoHandler,
    port: u16,
    config: NodeConfig,
}

impl RpcServer {
    pub fn new(
        port: u16,
        config: NodeConfig,
    ) -> std::result::Result<Self, Box<dyn std::error::Error>> {
        let mut handler = IoHandler::new();
        let cfg = config.clone();

        // Add basic methods (chain parameters)
        handler.add_method("eth_chainId", move |_params| {
            let cfg = cfg.clone();
            async move { Ok(Value::String(format!("0x{:x}", cfg.chain_id))) }
        });

        let cfg2 = config.clone();
        handler.add_method("eth_gasPrice", move |_params| {
            let cfg2 = cfg2.clone();
            async move { Ok(Value::String(format!("0x{:x}", cfg2.gas_price_wei))) }
        });

        handler.add_method(
            "eth_estimateGas",
            |_params: jsonrpc_core::Params| async move {
                Ok(Value::String("0x5208".to_string())) // 21000 gas
            },
        );

        Ok(Self {
            handler,
            port,
            config,
        })
    }

    pub fn with_blockchain(mut self, blockchain: Arc<RwLock<Blockchain>>) -> Self {
        let rpc_impl = BlockchainRPCImpl::new(blockchain, self.config.clone());
        self.handler.extend_with(rpc_impl.to_delegate());
        self
    }

    pub async fn start(self) -> std::result::Result<(), Box<dyn std::error::Error>> {
        use jsonrpc_http_server::{AccessControlAllowOrigin, DomainsValidation, ServerBuilder};

        let server = ServerBuilder::new(self.handler)
            .cors(DomainsValidation::AllowOnly(vec![
                AccessControlAllowOrigin::Any,
            ]))
            .start_http(&format!("127.0.0.1:{}", self.port).parse()?)?;

        server.wait();
        Ok(())
    }
}

pub fn create_rpc_handler(blockchain: Arc<RwLock<Blockchain>>, config: NodeConfig) -> IoHandler {
    let rpc_impl = BlockchainRPCImpl::new(blockchain, config.clone());
    let mut io = IoHandler::new();

    io.extend_with(rpc_impl.to_delegate());

    // Add additional methods
    let cfg = config.clone();
    io.add_method("eth_chainId", move |_params| {
        let cfg = cfg.clone();
        async move { Ok(Value::String(format!("0x{:x}", cfg.chain_id))) }
    });

    let cfg2 = config.clone();
    io.add_method("eth_gasPrice", move |_params| {
        let cfg2 = cfg2.clone();
        async move { Ok(Value::String(format!("0x{:x}", cfg2.gas_price_wei))) }
    });

    io.add_method(
        "eth_estimateGas",
        |_params: jsonrpc_core::Params| async move { Ok(Value::String("0x5208".to_string())) },
    );

    let cfg3 = config.clone();
    io.add_method("erb_chainInfo", move |_params| {
        let cfg3 = cfg3.clone();
        async move {
            Ok(serde_json::json!({
                "name": cfg3.asset_name,
                "symbol": cfg3.asset_symbol,
                "decimals": cfg3.asset_decimals,
                "chainId": format!("0x{:x}", cfg3.chain_id),
                "gasPrice": format!("0x{:x}", cfg3.gas_price_wei),
                "client": "ErbiumBlockchain/0.1.0"
            }))
        }
    });

    io
}

// Helper functions
fn parse_block_number(number: &str) -> jsonrpc_core::Result<u64> {
    if number == "latest" {
        Ok(0) // Placeholder for latest block
    } else if number.starts_with("0x") {
        u64::from_str_radix(number.trim_start_matches("0x"), 16)
            .map_err(|e| jsonrpc_core::Error::invalid_params(e.to_string()))
    } else {
        number
            .parse::<u64>()
            .map_err(|e| jsonrpc_core::Error::invalid_params(e.to_string()))
    }
}

fn parse_hex_number(hex_str: &str) -> jsonrpc_core::Result<u64> {
    u64::from_str_radix(hex_str.trim_start_matches("0x"), 16)
        .map_err(|e| jsonrpc_core::Error::invalid_params(e.to_string()))
}

// Serialize a block to JSON-RPC format
fn serialize_block(block: &Block, full_tx: bool) -> Value {
    let mut transactions = Vec::new();

    if full_tx {
        for tx in &block.transactions {
            transactions.push(serialize_transaction(tx));
        }
    } else {
        for tx in &block.transactions {
            transactions.push(Value::String(format!(
                "0x{}",
                hex::encode(tx.hash().as_bytes())
            )));
        }
    }

    serde_json::json!({
        "number": format!("0x{:x}", block.header.number),
        "hash": format!("0x{}", hex::encode(block.hash().as_bytes())),
        "parentHash": format!("0x{}", hex::encode(block.header.previous_hash.as_bytes())),
        "nonce": format!("0x{:x}", block.header.nonce.value()),
        "sha3Uncles": "0x1dcc4de8dec75d7aab85b567b6ccd41ad312451b948a7413f0a142fd40d49347", // Placeholder
        "logsBloom": "0x00000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000", // Placeholder
        "transactionsRoot": format!("0x{}", hex::encode(block.header.merkle_root.as_bytes())),
        "stateRoot": "0x56e81f171bcc55a6ff8345e692c0f86e5b48e01b996cadc001622fb5e363b421", // Placeholder
        "receiptsRoot": "0x56e81f171bcc55a6ff8345e692c0f86e5b48e01b996cadc001622fb5e363b421", // Placeholder
        "miner": block.header.validator,
        "difficulty": format!("0x{:x}", block.header.difficulty),
        "totalDifficulty": format!("0x{:x}", block.header.difficulty), // Simplified
        "extraData": "0x", // Placeholder
        "size": format!("0x{:x}", bincode::serialize(block).unwrap_or_default().len()),
        "gasLimit": "0x6691b7", // Placeholder
        "gasUsed": "0x0", // Placeholder
        "timestamp": format!("0x{:x}", block.header.timestamp),
        "transactions": transactions,
        "uncles": [] // No uncles in this blockchain
    })
}

// Serialize a transaction to JSON-RPC format
fn serialize_transaction(tx: &Transaction) -> Value {
    let transaction_type = match tx.transaction_type {
        crate::core::transaction::TransactionType::Transfer => "0x0",
        crate::core::transaction::TransactionType::ConfidentialTransfer => "0x1",
        crate::core::transaction::TransactionType::ContractDeployment => "0x2",
        crate::core::transaction::TransactionType::ContractCall => "0x3",
        crate::core::transaction::TransactionType::Stake => "0x4",
        crate::core::transaction::TransactionType::Unstake => "0x5",
        crate::core::transaction::TransactionType::Vote => "0x6",
    };

    serde_json::json!({
        "hash": format!("0x{}", hex::encode(tx.hash().as_bytes())),
        "nonce": format!("0x{:x}", tx.nonce),
        "blockHash": null, // Would need block context
        "blockNumber": null, // Would need block context
        "transactionIndex": null, // Would need block context
        "from": tx.from.as_str(),
        "to": if tx.transaction_type == crate::core::transaction::TransactionType::ContractDeployment {
            None
        } else {
            Some(tx.to.as_str())
        },
        "value": format!("0x{:x}", tx.amount),
        "gasPrice": format!("0x{:x}", tx.fee), // Simplified
        "gas": "0x5208", // Placeholder
        "input": format!("0x{}", hex::encode(&tx.data)),
        "type": transaction_type,
        "v": "0x0", // Placeholder
        "r": "0x0", // Placeholder
        "s": "0x0"  // Placeholder
    })
}
