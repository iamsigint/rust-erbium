// src/bridges/chains/bitcoin/adapter.rs
use super::types::*;
use bitcoin::{Block, Network, Address};
use bitcoincore_rpc::{Client, RpcApi, Auth};
use std::collections::HashMap;
use std::str::FromStr;

pub struct BitcoinAdapter {
    rpc_client: Client,
    network: Network,
    watched_addresses: HashMap<String, WatchInfo>,
    last_processed_block: u64,
}

impl BitcoinAdapter {
    pub fn new(config: BitcoinConfig) -> Result<Self, BitcoinError> {
        let rpc_url = format!("http://{}:{}", config.rpc_host, config.rpc_port);
        let auth = Auth::UserPass(config.rpc_user, config.rpc_pass);
        
        let rpc_client = Client::new(&rpc_url, auth)
            .map_err(|e| BitcoinError::RpcConnection(e.to_string()))?;
        
        // Verify connection
        let _block_count = rpc_client.get_block_count()
            .map_err(|e| BitcoinError::RpcConnection(e.to_string()))?;
        
        Ok(Self {
            rpc_client,
            network: config.network,
            watched_addresses: HashMap::new(),
            last_processed_block: 0,
        })
    }
    
    pub fn watch_address(
        &mut self,
        address: String,
        confirmations_required: u32,
        callback_url: Option<String>,
    ) -> Result<(), BitcoinError> {
        // Validate Bitcoin address
        Address::from_str(&address)
            .map_err(|e| BitcoinError::InvalidAddress(e.to_string()))?;
        
        let watch_info = WatchInfo {
            address: address.clone(),
            confirmations_required,
            callback_url,
        };
        
        self.watched_addresses.insert(address.clone(), watch_info);
        log::info!("Now watching Bitcoin address: {}", address);
        
        Ok(())
    }
    
    pub fn unwatch_address(&mut self, address: &str) -> Result<(), BitcoinError> {
        self.watched_addresses.remove(address)
            .ok_or_else(|| BitcoinError::AddressNotWatched(address.to_string()))?;
        
        log::info!("Stopped watching Bitcoin address: {}", address);
        Ok(())
    }
    
    pub fn process_new_blocks(&mut self) -> Result<Vec<BitcoinTransaction>, BitcoinError> {
        let current_height = self.rpc_client.get_block_count()?;
        
        if current_height <= self.last_processed_block {
            return Ok(Vec::new());
        }
        
        let mut new_transactions = Vec::new();
        
        // Process blocks from last processed to current
        for height in (self.last_processed_block + 1)..=current_height {
            let block_hash = self.rpc_client.get_block_hash(height)?;
            let block = self.rpc_client.get_block(&block_hash)?;
            
            let block_transactions = self.process_block_transactions(&block, height)?;
            new_transactions.extend(block_transactions);
        }
        
        self.last_processed_block = current_height;
        
        log::debug!("Processed {} new blocks, found {} transactions", 
                   current_height - self.last_processed_block, 
                   new_transactions.len());
        
        Ok(new_transactions)
    }
    
    pub fn get_transaction_proof(
        &self,
        txid: &str,
        block_height: u64,
    ) -> Result<BitcoinTransactionProof, BitcoinError> {
        let txid_parsed = bitcoin::Txid::from_str(txid)
            .map_err(|e| BitcoinError::InvalidTxid(e.to_string()))?;
        
        // Get transaction
        let block_hash = self.rpc_client.get_block_hash(block_height)?;
        let tx = self.rpc_client.get_raw_transaction(&txid_parsed, Some(&block_hash))
            .map_err(|e| BitcoinError::TransactionNotFound(e.to_string()))?;
        
        // Get block header
        let block_hash = self.rpc_client.get_block_hash(block_height)?;
        let block_header = self.rpc_client.get_block_header(&block_hash)?;
        
        // Get merkle proof
        let merkle_proof = self.generate_merkle_proof(&txid_parsed, block_height)?;
        
        let confirmations = (self.rpc_client.get_block_count()? - block_height) as u32;
        
        Ok(BitcoinTransactionProof {
            txid: txid.to_string(),
            transaction: tx,
            block_header: bitcoin::consensus::serialize(&block_header),
            merkle_proof,
            confirmations,
        })
    }
    
    pub fn broadcast_transaction(
        &self,
        signed_transaction: &str,
    ) -> Result<String, BitcoinError> {
        let tx_hex = signed_transaction;
        let txid = self.rpc_client.send_raw_transaction(tx_hex)?;
        
        log::info!("Broadcasted Bitcoin transaction: {}", txid);
        
        Ok(txid.to_string())
    }
    
    pub fn get_balance(&self, _address: &str) -> Result<u64, BitcoinError> {
        // This would typically use RPC to get address balance
        // For now, placeholder implementation
        Ok(0)
    }
    
    pub fn get_block_count(&self) -> Result<u64, BitcoinError> {
        self.rpc_client.get_block_count()
            .map_err(|e| BitcoinError::RpcConnection(e.to_string()))
    }
    
    // Private methods
    fn process_block_transactions(
        &self,
        block: &Block,
        _height: u64,
    ) -> Result<Vec<BitcoinTransaction>, BitcoinError> {
        let mut relevant_transactions = Vec::new();
        
        for tx in &block.txdata {
            for (vout, output) in tx.output.iter().enumerate() {
                if let Ok(addr) = Address::from_script(&output.script_pubkey, self.network) {
                    let address_str = addr.to_string();
                    
                    if self.watched_addresses.contains_key(&address_str) {
                        let btc_tx = BitcoinTransaction {
                    txid: tx.compute_txid().to_string(),
                            amount: output.value.to_sat(),
                            sender: self.extract_sender(tx)?,
                            recipient: address_str.clone(),
                            confirmations: 1, // Just mined
                            block_hash: Some(block.block_hash().to_string()),
                            vout: vout as u32,
                        };
                        
                        relevant_transactions.push(btc_tx);
                    }
                }
            }
        }
        
        Ok(relevant_transactions)
    }
    
    fn extract_sender(&self, tx: &bitcoin::Transaction) -> Result<String, BitcoinError> {
        // Extract sender from transaction inputs
        // This is simplified - in production would need to trace inputs
        if let Some(_input) = tx.input.first() {
            // Would need to fetch previous transaction to get the sender
            Ok("unknown".to_string())
        } else {
            Ok("coinbase".to_string()) // Coinbase transaction
        }
    }
    
    fn generate_merkle_proof(
        &self,
        txid: &bitcoin::Txid,
        block_height: u64,
    ) -> Result<Vec<u8>, BitcoinError> {
        // Simplified merkle proof generation
        // In production, this would generate a proper merkle proof
        let mut proof_data = Vec::new();
        proof_data.extend_from_slice(&txid[..]);
        proof_data.extend_from_slice(&block_height.to_be_bytes());
        
        Ok(proof_data)
    }
}