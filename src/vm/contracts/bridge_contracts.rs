use crate::core::types::{Address, Hash};
use crate::utils::error::{Result, BlockchainError};
use crate::vm::storage::ContractStorage;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BridgeTransfer {
    pub id: u64,
    pub from_chain: String,
    pub to_chain: String,
    pub sender: Address,
    pub recipient: Address,
    pub amount: u64,
    pub token_address: Address,
    pub status: BridgeTransferStatus,
    pub created_at: u64,
    pub completed_at: Option<u64>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum BridgeTransferStatus {
    Pending,
    Completed,
    Failed,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BridgeContract {
    pub address: Address,
    pub supported_chains: Vec<String>,
    pub fee_collector: Address,
    pub bridge_fee: u64, // Fee percentage (basis points)
    storage: ContractStorage,
    transfers: HashMap<u64, BridgeTransfer>,
    next_transfer_id: u64,
    chain_nonces: HashMap<String, u64>, // Nonce per chain for ordering
}

impl BridgeContract {
    pub fn new(
        address: Address,
        supported_chains: Vec<String>,
        fee_collector: Address,
        bridge_fee: u64,
    ) -> Result<Self> {
        let mut contract = Self {
            address,
            supported_chains,
            fee_collector,
            bridge_fee,
            storage: ContractStorage::new()?,
            transfers: HashMap::new(),
            next_transfer_id: 1,
            chain_nonces: HashMap::new(),
        };
        
        contract.storage.initialize_contract(&contract.address)?;
        Ok(contract)
    }
    
    pub fn initiate_transfer(
        &mut self,
        from_chain: String,
        to_chain: String,
        sender: Address,
        recipient: Address,
        amount: u64,
        token_address: Address,
    ) -> Result<u64> {
        // Validate supported chains
        if !self.supported_chains.contains(&from_chain) || !self.supported_chains.contains(&to_chain) {
            return Err(BlockchainError::VM("Unsupported chain".to_string()));
        }
        
        // Calculate bridge fee
        let fee = (amount * self.bridge_fee) / 10000; // basis points
        let net_amount = amount - fee;
        
        let transfer_id = self.next_transfer_id;
        let current_time = chrono::Utc::now().timestamp_millis() as u64;
        
        let transfer = BridgeTransfer {
            id: transfer_id,
            from_chain: from_chain.clone(),
            to_chain: to_chain.clone(),
            sender: sender.clone(),
            recipient: recipient.clone(),
            amount: net_amount,
            token_address,
            status: BridgeTransferStatus::Pending,
            created_at: current_time,
            completed_at: None,
        };
        
        self.transfers.insert(transfer_id, transfer);
        self.next_transfer_id += 1;
        
        // Update chain nonce
        let chain_nonce = self.chain_nonces.entry(to_chain.clone()).or_insert(0);
        *chain_nonce += 1;
        
        self.store_transfer(transfer_id)?;
        self.store_chain_nonce(&to_chain)?;
        self.emit_transfer_initiated_event(transfer_id, &sender, &recipient, net_amount)?;
        
        Ok(transfer_id)
    }
    
    pub fn complete_transfer(
        &mut self,
        transfer_id: u64,
        proof: Vec<u8>,
        verifier: &Address,
    ) -> Result<bool> {
        // Clone transfer data first to avoid borrow issues
        let transfer_clone = self.transfers.get(&transfer_id)
            .ok_or_else(|| BlockchainError::VM("Transfer not found".to_string()))?
            .clone();
        
        // Check if transfer is still pending
        if transfer_clone.status != BridgeTransferStatus::Pending {
            return Err(BlockchainError::VM("Transfer already processed".to_string()));
        }
        
        // Verify cross-chain proof (simplified)
        if !self.verify_cross_chain_proof(&transfer_clone, &proof, verifier)? {
            return Err(BlockchainError::VM("Invalid cross-chain proof".to_string()));
        }
        
        // Get mutable reference after verification
        let transfer = self.transfers.get_mut(&transfer_id).unwrap();
        
        // Mark transfer as completed
        transfer.status = BridgeTransferStatus::Completed;
        transfer.completed_at = Some(chrono::Utc::now().timestamp_millis() as u64);
        
        self.store_transfer(transfer_id)?;
        self.emit_transfer_completed_event(transfer_id)?;
        
        Ok(true)
    }
    
    pub fn register_chain(&mut self, chain_id: String, admin: &Address) -> Result<bool> {
        // In real implementation, this would check admin permissions
        if self.supported_chains.contains(&chain_id) {
            return Err(BlockchainError::VM("Chain already registered".to_string()));
        }
        
        self.supported_chains.push(chain_id.clone());
        self.store_supported_chains()?;
        self.emit_chain_registered_event(&chain_id, admin)?;
        
        Ok(true)
    }
    
    pub fn update_bridge_fee(&mut self, new_fee: u64, admin: &Address) -> Result<bool> {
        // In real implementation, this would check admin permissions
        if new_fee > 1000 { // Max 10% fee
            return Err(BlockchainError::VM("Bridge fee too high".to_string()));
        }
        
        self.bridge_fee = new_fee;
        self.store_bridge_fee()?;
        self.emit_bridge_fee_updated_event(new_fee, admin)?;
        
        Ok(true)
    }
    
    pub fn get_transfer(&self, transfer_id: u64) -> Result<Option<BridgeTransfer>> {
        Ok(self.transfers.get(&transfer_id).cloned())
    }
    
    pub fn get_chain_nonce(&self, chain_id: &str) -> u64 {
        self.chain_nonces.get(chain_id).copied().unwrap_or(0)
    }
    
    pub fn get_supported_chains(&self) -> &Vec<String> {
        &self.supported_chains
    }
    
    fn verify_cross_chain_proof(
        &self,
        transfer: &BridgeTransfer,
        proof: &[u8],
        verifier: &Address,
    ) -> Result<bool> {
        // In real implementation, this would verify a ZK proof or cryptographic signature
        // that proves the transfer occurred on the source chain
        
        // Simplified verification for demo purposes
        log::debug!("Verifying cross-chain proof for transfer {} by verifier {}", 
                   transfer.id, verifier.as_str());
        
        // Check proof length and basic structure
        if proof.len() < 64 {
            return Ok(false);
        }
        
        // Placeholder verification logic
        Ok(true)
    }
    
    fn store_transfer(&mut self, transfer_id: u64) -> Result<()> {
        let transfer = self.transfers.get(&transfer_id)
            .ok_or_else(|| BlockchainError::VM("Transfer not found".to_string()))?;
        
        let transfer_data = bincode::serialize(transfer)
            .map_err(|e| BlockchainError::Serialization(e.to_string()))?;
        
        let key = Self::get_transfer_key(transfer_id);
        self.storage.store(&self.address, key, transfer_data)?;
        
        Ok(())
    }
    
    fn store_chain_nonce(&mut self, chain_id: &str) -> Result<()> {
        let nonce = self.get_chain_nonce(chain_id);
        let nonce_data = nonce.to_be_bytes().to_vec();
        
        let key = Self::get_chain_nonce_key(chain_id);
        self.storage.store(&self.address, key, nonce_data)?;
        
        Ok(())
    }
    
    fn store_supported_chains(&mut self) -> Result<()> {
        let chains_data = bincode::serialize(&self.supported_chains)
            .map_err(|e| BlockchainError::Serialization(e.to_string()))?;
        
        let key = b"supported_chains".to_vec();
        self.storage.store(&self.address, key, chains_data)?;
        
        Ok(())
    }
    
    fn store_bridge_fee(&mut self) -> Result<()> {
        let fee_data = self.bridge_fee.to_be_bytes().to_vec();
        let key = b"bridge_fee".to_vec();
        self.storage.store(&self.address, key, fee_data)?;
        
        Ok(())
    }
    
    fn emit_transfer_initiated_event(
        &self,
        transfer_id: u64,
        sender: &Address,
        recipient: &Address,
        amount: u64,
    ) -> Result<()> {
        log::debug!("Bridge Transfer Initiated: {} from {} to {} for {}", 
                   transfer_id, sender.as_str(), recipient.as_str(), amount);
        Ok(())
    }
    
    fn emit_transfer_completed_event(&self, transfer_id: u64) -> Result<()> {
        log::debug!("Bridge Transfer Completed: {}", transfer_id);
        Ok(())
    }
    
    fn emit_chain_registered_event(&self, chain_id: &str, admin: &Address) -> Result<()> {
        log::debug!("Bridge Chain Registered: {} by {}", chain_id, admin.as_str());
        Ok(())
    }
    
    fn emit_bridge_fee_updated_event(&self, new_fee: u64, admin: &Address) -> Result<()> {
        log::debug!("Bridge Fee Updated: {} by {}", new_fee, admin.as_str());
        Ok(())
    }
    
    fn get_transfer_key(transfer_id: u64) -> Vec<u8> {
        let mut key = b"transfer_".to_vec();
        key.extend_from_slice(&transfer_id.to_be_bytes());
        key
    }
    
    fn get_chain_nonce_key(chain_id: &str) -> Vec<u8> {
        let mut key = b"nonce_".to_vec();
        key.extend_from_slice(chain_id.as_bytes());
        key
    }
    
    pub fn get_storage_root(&self) -> Result<Hash> {
        self.storage.calculate_storage_root(&self.address)
    }
}

impl crate::vm::SmartContract for BridgeContract {
    fn execute(&mut self, function: &str, args: &[u8]) -> Result<Vec<u8>> {
        match function {
            "initiateTransfer" => {
                if args.len() < 80 {
                    return Err(BlockchainError::VM("Invalid initiateTransfer arguments".to_string()));
                }
                
                let from_chain_len = args[0] as usize;
                let to_chain_len = args[1] as usize;
                
                if args.len() < 2 + from_chain_len + to_chain_len + 60 {
                    return Err(BlockchainError::VM("Invalid chain data".to_string()));
                }
                
                let from_chain = String::from_utf8_lossy(&args[2..2 + from_chain_len]).to_string();
                let to_chain = String::from_utf8_lossy(&args[2 + from_chain_len..2 + from_chain_len + to_chain_len]).to_string();
                let sender = Address::new(String::from_utf8_lossy(&args[2 + from_chain_len + to_chain_len..2 + from_chain_len + to_chain_len + 20]).to_string())?;
                let recipient = Address::new(String::from_utf8_lossy(&args[2 + from_chain_len + to_chain_len + 20..2 + from_chain_len + to_chain_len + 40]).to_string())?;
                let amount = u64::from_be_bytes(args[2 + from_chain_len + to_chain_len + 40..2 + from_chain_len + to_chain_len + 48].try_into().unwrap());
                let token_address = Address::new(String::from_utf8_lossy(&args[2 + from_chain_len + to_chain_len + 48..2 + from_chain_len + to_chain_len + 68]).to_string())?;
                
                let transfer_id = self.initiate_transfer(from_chain, to_chain, sender, recipient, amount, token_address)?;
                Ok(transfer_id.to_be_bytes().to_vec())
            }
            "completeTransfer" => {
                if args.len() < 29 {
                    return Err(BlockchainError::VM("Invalid completeTransfer arguments".to_string()));
                }
                let transfer_id = u64::from_be_bytes(args[0..8].try_into().unwrap());
                let proof_len = args[8] as usize;
                
                if args.len() < 9 + proof_len + 20 {
                    return Err(BlockchainError::VM("Invalid proof data".to_string()));
                }
                
                let proof = args[9..9 + proof_len].to_vec();
                let verifier = Address::new(String::from_utf8_lossy(&args[9 + proof_len..9 + proof_len + 20]).to_string())?;
                
                let success = self.complete_transfer(transfer_id, proof, &verifier)?;
                Ok(vec![if success { 1 } else { 0 }])
            }
            "getTransfer" => {
                if args.len() < 8 {
                    return Err(BlockchainError::VM("Invalid getTransfer arguments".to_string()));
                }
                let transfer_id = u64::from_be_bytes(args[0..8].try_into().unwrap());
                
                if let Some(transfer) = self.get_transfer(transfer_id)? {
                    let transfer_data = bincode::serialize(&transfer)
                        .map_err(|e| BlockchainError::Serialization(e.to_string()))?;
                    Ok(transfer_data)
                } else {
                    Ok(vec![]) // Empty response for non-existent transfer
                }
            }
            _ => Err(BlockchainError::VM(format!("Unknown function: {}", function))),
        }
    }
    
    fn get_storage_root(&self) -> Result<Hash> {
        self.get_storage_root()
    }
    
    fn get_address(&self) -> &Address {
        &self.address
    }
}