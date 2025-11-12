// src/bridges/chains/cosmos/adapter.rs
use super::types::*;
use tendermint_rpc::{HttpClient, Client};
use cosmrs::tendermint::block::Height;
use std::collections::HashMap;

/// Cosmos blockchain adapter for bridge operations
pub struct CosmosAdapter {
    rpc_client: HttpClient,
    _grpc_client: Option<tonic::transport::Channel>,
    _chain_id: String,
    last_processed_block: u64,
    watched_addresses: HashMap<String, WatchInfo>,
}

impl CosmosAdapter {
    /// Create a new Cosmos adapter instance
    pub async fn new(config: CosmosConfig) -> Result<Self, CosmosError> {
        let rpc_client = HttpClient::new(config.rpc_endpoint.as_str())
            .map_err(|e| CosmosError::Connection(e.to_string()))?;
        
        // Verify RPC connection
        let _status = rpc_client.status()
            .await
            .map_err(|e| CosmosError::Connection(e.to_string()))?;
        
        // Initialize gRPC client if endpoint provided
        let grpc_client = if let Some(grpc_endpoint) = config.grpc_endpoint {
            Some(tonic::transport::Channel::from_shared(grpc_endpoint)
                .map_err(|e| CosmosError::GrpcError(e.to_string()))?
                .connect()
                .await
                .map_err(|e| CosmosError::GrpcError(e.to_string()))?)
        } else {
            None
        };
        
        Ok(Self {
            rpc_client,
            _grpc_client: grpc_client,
            _chain_id: config.chain_id,
            last_processed_block: 0,
            watched_addresses: HashMap::new(),
        })
    }
    
    /// Start monitoring a Cosmos address for specific message types
    pub fn watch_address(
        &mut self,
        address: String,
        message_types: Vec<String>,
    ) -> Result<(), CosmosError> {
        let watch_info = WatchInfo {
            address: address.clone(),
            message_types,
        };
        
        self.watched_addresses.insert(address.clone(), watch_info);
        log::info!("Now watching Cosmos address: {}", address);
        
        Ok(())
    }
    
    /// Stop monitoring a Cosmos address
    pub fn unwatch_address(&mut self, address: &str) -> Result<(), CosmosError> {
        self.watched_addresses.remove(address)
            .ok_or_else(|| CosmosError::InvalidAddress(format!("Address not being watched: {}", address)))?;
        
        log::info!("Stopped watching Cosmos address: {}", address);
        Ok(())
    }
    
    /// Process new blocks and return relevant transactions
    pub async fn process_new_blocks(&mut self) -> Result<Vec<CosmosTransaction>, CosmosError> {
        let current_height = self.get_current_block_height().await?;
        
        if current_height <= self.last_processed_block {
            return Ok(Vec::new());
        }
        
        let mut new_transactions = Vec::new();
        
        // Process blocks from last processed to current
        for height in (self.last_processed_block + 1)..=current_height {
            if let Some(block) = self.get_block_at_height(height).await? {
                let transactions = self.process_block_transactions(&block, height).await?;
                new_transactions.extend(transactions);
            }
        }
        
        self.last_processed_block = current_height;
        
        log::debug!("Processed {} new Cosmos blocks, found {} transactions", 
                   current_height - self.last_processed_block, 
                   new_transactions.len());
        
        Ok(new_transactions)
    }
    
    /// Get transaction by hash
    pub async fn get_transaction(
        &self,
        hash: &str,
    ) -> Result<Option<CosmosTransaction>, CosmosError> {
        let tx_response = self.rpc_client.tx(hash.parse().unwrap(), false)
            .await
            .map_err(|e| CosmosError::RpcError(e.to_string()))?;
        Ok(Some(self.parse_transaction_response(tx_response)?))
    }
    
    /// Broadcast a transaction to the Cosmos network
    pub async fn broadcast_transaction(
        &self,
        tx_bytes: Vec<u8>,
        mode: BroadcastMode,
    ) -> Result<BroadcastTxResponse, CosmosError> {
        match mode {
            BroadcastMode::Sync => {
                let response = self.rpc_client.broadcast_tx_sync(tx_bytes.clone()).await
                    .map_err(|e| CosmosError::BroadcastError(e.to_string()))?;
                Ok(BroadcastTxResponse {
                    hash: response.hash.to_string(),
                    height: 0,
                    code: response.code.value(),
                    raw_log: response.log,
                })
            },
            BroadcastMode::Async => {
                let response = self.rpc_client.broadcast_tx_async(tx_bytes.clone()).await
                    .map_err(|e| CosmosError::BroadcastError(e.to_string()))?;
                Ok(BroadcastTxResponse {
                    hash: response.hash.to_string(),
                    height: 0,
                    code: response.code.value(),
                    raw_log: response.log,
                })
            },
            BroadcastMode::Block => {
                let response = self.rpc_client.broadcast_tx_commit(tx_bytes.clone()).await
                    .map_err(|e| CosmosError::BroadcastError(e.to_string()))?;
                Ok(BroadcastTxResponse {
                    hash: response.hash.to_string(),
                    height: response.height.value(),
                    code: response.tx_result.code.value(),
                    raw_log: response.tx_result.log,
                })
            },
        }
    }
    
    /// Query balance for a specific address and denomination
    pub async fn query_balance(
        &self,
        _address: &str,
        denom: &str,
    ) -> Result<CosmosCoin, CosmosError> {
        // This would use the bank module query via gRPC
        // Placeholder implementation
        Ok(CosmosCoin {
            denom: denom.to_string(),
            amount: "0".to_string(),
        })
    }
    
    /// Query smart contract state
    pub async fn query_contract_state(
        &self,
        _contract_address: &str,
        _query_msg: Vec<u8>,
    ) -> Result<Vec<u8>, CosmosError> {
        // Query Wasm contract state via gRPC
        // Placeholder implementation
        Ok(Vec::new())
    }
    
    /// Get the current block height
    pub async fn get_current_block_height(&self) -> Result<u64, CosmosError> {
        let status = self.rpc_client.status()
            .await
            .map_err(|e| CosmosError::RpcError(e.to_string()))?;
        
        Ok(status.sync_info.latest_block_height.value())
    }
    
    // Private methods
    
    /// Get block at specific height
    async fn get_block_at_height(
        &self,
        height: u64,
    ) -> Result<Option<tendermint::Block>, CosmosError> {
        let block = self.rpc_client.block(Height::from(height as u32))
            .await
            .map_err(|e| CosmosError::RpcError(e.to_string()))?;
        
        Ok(Some(block.block))
    }
    
    /// Process transactions in a block
    async fn process_block_transactions(
        &self,
        block: &tendermint::Block,
        height: u64,
    ) -> Result<Vec<CosmosTransaction>, CosmosError> {
        let mut relevant_transactions = Vec::new();
        
        for tx in &block.data {
            if let Some(cosmos_tx) = self.parse_transaction(tx, height).await? {
                if self.is_relevant_transaction(&cosmos_tx) {
                    relevant_transactions.push(cosmos_tx);
                }
            }
        }
        
        Ok(relevant_transactions)
    }
    
    /// Parse raw transaction bytes into CosmosTransaction
    async fn parse_transaction(
        &self,
        tx_bytes: &[u8],
        height: u64,
    ) -> Result<Option<CosmosTransaction>, CosmosError> {
        // Parse Cosmos transaction from bytes using protobuf decoding
        // This would properly decode the transaction using cosmrs or similar
        
        // Placeholder implementation - in production, this would:
        // 1. Decode the protobuf transaction
        // 2. Extract messages, fees, memo
        // 3. Parse message contents
        
        let tx = CosmosTransaction {
            hash: format!("0x{}", hex::encode(&tx_bytes[..32])), // Simplified hash
            height,
            messages: Vec::new(), // Would parse actual messages
            fee: CosmosFee {
                amount: Vec::new(),
                gas_limit: 0,
            },
            memo: "".to_string(),
            success: true,
        };
        
        Ok(Some(tx))
    }
    
    /// Parse transaction from RPC response
    fn parse_transaction_response(
        &self,
        tx_response: tendermint_rpc::endpoint::tx::Response,
    ) -> Result<CosmosTransaction, CosmosError> {
        // Parse transaction from RPC response
        // Placeholder implementation
        
        Ok(CosmosTransaction {
            hash: tx_response.hash.to_string(),
            height: tx_response.height.value(),
            messages: Vec::new(),
            fee: CosmosFee {
                amount: Vec::new(),
                gas_limit: 0,
            },
            memo: "".to_string(),
            success: tx_response.tx_result.code.is_ok(),
        })
    }
    
    /// Check if transaction is relevant for our bridge monitoring
    fn is_relevant_transaction(&self, transaction: &CosmosTransaction) -> bool {
        // Check if transaction contains relevant messages for watched addresses
        for message in &transaction.messages {
            for watch_info in self.watched_addresses.values() {
                if watch_info.message_types.contains(&message.type_url) {
                    return true;
                }
            }
        }
        
        false
    }
}

// Implement From for BroadcastMode conversion
// Removed obsolete conversion: tendermint_rpc::client::sync::BroadcastTxMethod no longer exists in current tendermint-rpc
