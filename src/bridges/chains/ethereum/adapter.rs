// src/bridges/chains/ethereum/adapter.rs
use super::types::*;
use web3::{
    types::{Address, H256, U256, Transaction, Block, Log, TransactionReceipt, CallRequest, BlockId, BlockNumber},
    transports::Http,
    Web3,
};
use std::collections::HashMap;

/// Ethereum blockchain adapter for bridge operations
pub struct EthereumAdapter {
    web3: Web3<Http>,
    contract_watches: HashMap<Address, ContractWatchInfo>,
    last_processed_block: u64,
    _erc20_tokens: HashMap<Address, Erc20TokenInfo>,
}

impl EthereumAdapter {
    /// Create a new Ethereum adapter instance
    pub async fn new(config: EthereumConfig) -> Result<Self, EthereumError> {
        let transport = Http::new(&config.rpc_endpoint)
            .map_err(|e| EthereumError::Connection(e.to_string()))?;

        let web3 = Web3::new(transport);

        // Verify connection - simplified for now
        // Connection will be verified on first actual call

        Ok(Self {
            web3,
            contract_watches: HashMap::new(),
            last_processed_block: 0,
            _erc20_tokens: HashMap::new(),
        })
    }
    
    /// Start monitoring an Ethereum contract for specific events
    pub fn watch_contract(
        &mut self,
        contract_name: String,
        contract_address: Address,
        event_signatures: Vec<H256>,
    ) -> Result<(), EthereumError> {
        let watch_info = ContractWatchInfo {
            contract_name: contract_name.clone(),
            contract_address,
            event_signatures,
        };
        
        self.contract_watches.insert(contract_address, watch_info);
        log::info!("Now watching Ethereum contract: {} at {:?}", contract_name, contract_address);
        
        Ok(())
    }
    
    /// Stop monitoring an Ethereum contract
    pub fn unwatch_contract(&mut self, contract_address: Address) -> Result<(), EthereumError> {
        self.contract_watches.remove(&contract_address)
            .ok_or_else(|| EthereumError::InvalidAddress)?;
        
        log::info!("Stopped watching Ethereum contract: {:?}", contract_address);
        Ok(())
    }
    
    /// Process new blocks and return relevant transactions
    pub async fn process_new_blocks(&mut self) -> Result<Vec<EthereumTransaction>, EthereumError> {
        let current_block = self.web3.eth().block_number().await?.as_u64();
        
        if current_block <= self.last_processed_block {
            return Ok(Vec::new());
        }
        
        let mut new_transactions = Vec::new();
        
        // Process blocks from last processed to current
        for block_number in (self.last_processed_block + 1)..=current_block {
            if let Some(block) = self.get_block_with_transactions(block_number).await? {
                for transaction in block.transactions {
                    if self.is_relevant_transaction(&transaction) {
                        // Fetch transaction logs
                        let logs = self.get_transaction_logs(transaction.hash).await?;
                        
                        let eth_tx = EthereumTransaction {
                            hash: transaction.hash,
                            from: transaction.from.unwrap_or_default(),
                            to: transaction.to,
                            value: transaction.value,
                            input: transaction.input.0,
                            block_number,
                            block_hash: block.hash.unwrap(),
                            logs,
                        };
                        
                        new_transactions.push(eth_tx);
                    }
                }
            }
        }
        
        self.last_processed_block = current_block;
        
        log::debug!("Processed {} new Ethereum blocks, found {} transactions", 
                   current_block - self.last_processed_block, 
                   new_transactions.len());
        
        Ok(new_transactions)
    }
    
    /// Get transaction receipt by hash
    pub async fn get_transaction_receipt(
        &self,
        tx_hash: H256,
    ) -> Result<Option<TransactionReceipt>, EthereumError> {
        self.web3.eth()
            .transaction_receipt(tx_hash)
            .await
            .map_err(|e| EthereumError::RpcError(e.to_string()))
    }
    
    /// Get transaction proof for bridge verification
    pub async fn get_transaction_proof(
        &self,
        tx_hash: H256,
    ) -> Result<EthereumTransactionProof, EthereumError> {
        let tx = self.web3.eth()
            .transaction(web3::types::TransactionId::Hash(tx_hash))
            .await
            .map_err(|e| EthereumError::TransactionNotFound(e.to_string()))?
            .ok_or_else(|| EthereumError::TransactionNotFound("Transaction not found".to_string()))?;
        
        let receipt = self.get_transaction_receipt(tx_hash).await?
            .ok_or_else(|| EthereumError::TransactionNotFound("Receipt not found".to_string()))?;
        
        let block_hash = tx.block_hash
            .ok_or_else(|| EthereumError::BlockNotFound("Transaction not in block".to_string()))?;
        
        let block = self.web3.eth()
            .block(BlockId::Hash(block_hash))
            .await
            .map_err(|e| EthereumError::BlockNotFound(e.to_string()))?
            .ok_or_else(|| EthereumError::BlockNotFound("Block not found".to_string()))?;
        
        Ok(EthereumTransactionProof {
            transaction: tx,
            receipt,
            block: block.clone(),
            state_proof: Vec::new(), // Ethereum uses state proofs instead of merkle proofs
        })
    }
    
    /// Call a contract method without sending a transaction
    pub async fn call_contract(
        &self,
        call: ContractCall,
    ) -> Result<Vec<u8>, EthereumError> {
        let call_request = CallRequest {
            to: Some(call.to),
            data: Some(call.data.into()),
            value: Some(call.value),
            gas: call.gas_limit,
            ..Default::default()
        };
        
        let result = self.web3.eth()
            .call(call_request, None)
            .await
            .map_err(|e| EthereumError::ContractCall(e.to_string()))?;
        
        Ok(result.0)
    }
    
    /// Get current gas price from the network
    pub async fn get_gas_price(&self) -> Result<U256, EthereumError> {
        self.web3.eth()
            .gas_price()
            .await
            .map_err(|e| EthereumError::RpcError(e.to_string()))
    }
    
    /// Estimate gas for a transaction
    pub async fn estimate_gas(
        &self,
        params: &TransactionParams,
    ) -> Result<U256, EthereumError> {
        let call_request = CallRequest {
            to: Some(params.to),
            data: Some(params.data.clone().into()),
            value: Some(params.value),
            gas: Some(params.gas_limit),
            ..Default::default()
        };
        
        self.web3.eth()
            .estimate_gas(call_request, None)
            .await
            .map_err(|e| EthereumError::GasEstimationFailed(e.to_string()))
    }
    
    /// Get current block number
    pub async fn get_block_number(&self) -> Result<u64, EthereumError> {
        self.web3.eth()
            .block_number()
            .await
            .map(|n| n.as_u64())
            .map_err(|e| EthereumError::RpcError(e.to_string()))
    }
    
    /// Get balance of an address
    pub async fn get_balance(&self, address: Address) -> Result<U256, EthereumError> {
        self.web3.eth()
            .balance(address, None)
            .await
            .map_err(|e| EthereumError::RpcError(e.to_string()))
    }
    
    // Private methods
    
    /// Get block with transactions at specific block number
    async fn get_block_with_transactions(
        &self,
        block_number: u64,
    ) -> Result<Option<Block<Transaction>>, EthereumError> {
        self.web3.eth()
            .block_with_txs(BlockId::Number(BlockNumber::Number(block_number.into())))
            .await
            .map_err(|e| EthereumError::RpcError(e.to_string()))
    }
    
    /// Get transaction logs
    async fn get_transaction_logs(
        &self,
        tx_hash: H256,
    ) -> Result<Vec<Log>, EthereumError> {
        if let Some(receipt) = self.get_transaction_receipt(tx_hash).await? {
            Ok(receipt.logs)
        } else {
            Ok(Vec::new())
        }
    }
    
    /// Check if transaction is relevant for our bridge monitoring
    fn is_relevant_transaction(&self, transaction: &Transaction) -> bool {
        // Check if transaction interacts with watched contracts
        if let Some(to) = transaction.to {
            if self.contract_watches.contains_key(&to) {
                return true;
            }
        }
        
        // Check if it's a contract creation to a watched address
        if transaction.to.is_none() && !transaction.input.0.is_empty() {
            // Could check if the created contract is relevant
            // This would require more complex analysis
        }
        
        false
    }
}
