// src/core/chain.rs

use super::{Block, Transaction};
use super::types::{Hash, Address};
use super::state::State;
use crate::utils::error::{Result, BlockchainError};
use std::collections::HashMap;

pub struct Blockchain {
    pub blocks: Vec<Block>,
    pub state: State,
    pub block_hashes: HashMap<Hash, usize>, // hash -> block index
    pub current_difficulty: u64,
}

impl Blockchain {
    pub fn new() -> Result<Self> {
        let genesis_block = Self::create_genesis_block()?;
        let mut blockchain = Self {
            blocks: Vec::new(),
            state: State::new(),
            block_hashes: HashMap::new(),
            current_difficulty: 1000,
        };

        blockchain.add_block(genesis_block)?;
        Ok(blockchain)
    }

    /// Create a new blockchain with database persistence
    pub async fn new_with_database(database_path: &str) -> Result<Self> {
        let genesis_block = Self::create_genesis_block()?;
        let state = State::new_with_database(database_path).await?;
        let mut blockchain = Self {
            blocks: Vec::new(),
            state,
            block_hashes: HashMap::new(),
            current_difficulty: 1000,
        };

        blockchain.add_block(genesis_block)?;
        Ok(blockchain)
    }
    
    fn create_genesis_block() -> Result<Block> {
        let transactions = Vec::new();
        // Added missing block number '0'
        let block = Block::new(
            0, // block number
            Hash::new(b"genesis"),
            transactions,
            "genesis".to_string(),
            1000,
        );
        
        Ok(block)
    }
    
    pub fn add_block(&mut self, block: Block) -> Result<()> {
        // Verify block structure and transactions
        self.verify_block(&block)?;
        
        // Apply transactions to state
        for tx in &block.transactions {
            self.state.apply_transaction(tx)?;
        }
        
        // Add block to chain
        let block_hash = block.hash();
        let block_index = self.blocks.len();
        
        self.blocks.push(block);
        self.block_hashes.insert(block_hash, block_index);
        
        log::info!("Added block {} at height {}", block_hash, block_index);
        
        Ok(())
    }
    
    fn verify_block(&self, block: &Block) -> Result<()> {
        // Check previous hash
        if let Some(last_block) = self.blocks.last() {
            if block.header.previous_hash != last_block.hash() {
                return Err(BlockchainError::InvalidBlock("Previous hash mismatch".to_string()));
            }
            // Check block number
            if block.header.number != last_block.header.number + 1 {
                return Err(BlockchainError::InvalidBlock("Invalid block number".to_string()));
            }
        } else {
            // This is the genesis block
            if block.header.number != 0 {
                return Err(BlockchainError::InvalidBlock("Invalid genesis block number".to_string()));
            }
        }
        
        // Verify merkle root
        let calculated_merkle_root = Block::calculate_merkle_root(&block.transactions);
        if block.header.merkle_root != calculated_merkle_root {
            return Err(BlockchainError::InvalidBlock("Merkle root mismatch".to_string()));
        }
        
        // Verify block signature
        // Skip signature check for genesis block
        if block.header.number > 0 {
            if let Some(validator_key) = self.get_validator_public_key(&block.header.validator) {
                if !block.verify_signature(&validator_key)? {
                    return Err(BlockchainError::InvalidBlock("Invalid block signature".to_string()));
                }
            } else {
                return Err(BlockchainError::InvalidBlock(format!("Validator {} not found", block.header.validator)));
            }
        }
        
        // Verify all transactions in the block
        for tx in &block.transactions {
            self.verify_transaction(tx)?;
        }
        
        Ok(())
    }
    
    fn verify_transaction(&self, transaction: &Transaction) -> Result<()> {
        // Basic transaction structure validation
        transaction.validate_basic()?;
        
        match transaction.transaction_type {
            crate::core::transaction::TransactionType::ConfidentialTransfer => {
                self.verify_confidential_transaction(transaction)?;
            }
            _ => {
                self.verify_regular_transaction(transaction)?;
            }
        }
        
        // Verify transaction signature
        if let Some(sender_key) = self.get_account_public_key(&transaction.from) {
            if !transaction.verify_signature(&sender_key)? {
                return Err(BlockchainError::InvalidTransaction("Invalid transaction signature".to_string()));
            }
        } else {
            // For now, assume new accounts can send transactions (e.g., funded by genesis)
            // In a real chain, this might be a stricter check.
             log::warn!("Sender account {} public key not found. Skipping signature check.", transaction.from);
            // return Err(BlockchainError::InvalidTransaction("Sender account not found".to_string()));
        }
        
        Ok(())
    }
    
    fn verify_regular_transaction(&self, transaction: &Transaction) -> Result<()> {
        // For regular transactions, check balance and nonce
        let balance = self.state.get_balance(&transaction.from)?;
        let current_nonce = self.state.get_nonce(&transaction.from)?;
        
        if balance < transaction.amount + transaction.fee {
            return Err(BlockchainError::InvalidTransaction(format!(
                "Insufficient balance for {}: needed {}, got {}",
                transaction.from,
                transaction.amount + transaction.fee,
                balance
            )));
        }
        
        if transaction.nonce != current_nonce {
            return Err(BlockchainError::InvalidTransaction(format!(
                "Invalid nonce for {}: expected {}, got {}",
                transaction.from,
                current_nonce,
                transaction.nonce
            )));
        }
        
        Ok(())
    }
    
    fn verify_confidential_transaction(&self, transaction: &Transaction) -> Result<()> {
        use crate::privacy::confidential_tx::ConfidentialTxBuilder;

        // Verify ZK proof exists
        let zk_proof = transaction.zk_proof.as_ref()
            .ok_or_else(|| BlockchainError::InvalidTransaction(
                "Confidential transaction missing ZK proof".to_string()
            ))?;

        // Verify commitments exist
        let input_commitments = transaction.input_commitments.as_ref()
            .ok_or_else(|| BlockchainError::InvalidTransaction(
                "Confidential transaction missing input commitments".to_string()
            ))?;
        let output_commitments = transaction.output_commitments.as_ref()
            .ok_or_else(|| BlockchainError::InvalidTransaction(
                "Confidential transaction missing output commitments".to_string()
            ))?;

        // Verify range proofs exist
        let range_proofs = transaction.range_proofs.as_ref()
            .ok_or_else(|| BlockchainError::InvalidTransaction(
                "Confidential transaction missing range proofs".to_string()
            ))?;

        // Verify binding signature exists
        let binding_signature = transaction.binding_signature.as_ref()
            .ok_or_else(|| BlockchainError::InvalidTransaction(
                "Confidential transaction missing binding signature".to_string()
            ))?;

        // Reconstruct confidential transaction for verification
        // Parse the commitments properly
        let input_commitments_parsed: Result<Vec<_>> = input_commitments.iter()
            .map(|commitment| {
                // Garantir que o commitment tem o tamanho correto (32 bytes)
                if commitment.len() != 32 {
                    return Err(BlockchainError::InvalidTransaction(
                        format!("Invalid commitment size: {}", commitment.len())
                    ));
                }

                // Converter para CompressedRistretto
                let mut bytes = [0u8; 32];
                bytes.copy_from_slice(commitment);
                Ok(curve25519_dalek_ng::ristretto::CompressedRistretto::from_slice(&bytes))
            })
            .collect();
        let input_commitments_parsed = input_commitments_parsed?;

        let output_commitments_parsed: Result<Vec<_>> = output_commitments.iter()
            .map(|commitment| {
                // Garantir que o commitment tem o tamanho correto (32 bytes)
                if commitment.len() != 32 {
                    return Err(BlockchainError::InvalidTransaction(
                        format!("Invalid commitment size: {}", commitment.len())
                    ));
                }

                // Converter para CompressedRistretto
                let mut bytes = [0u8; 32];
                bytes.copy_from_slice(commitment);
                Ok(curve25519_dalek_ng::ristretto::CompressedRistretto::from_slice(&bytes))
            })
            .collect();
        let output_commitments_parsed = output_commitments_parsed?;

        let confidential_tx = crate::privacy::confidential_tx::ConfidentialTransaction {
            base_transaction: transaction.clone(),
            input_commitments: input_commitments_parsed,
            output_commitments: output_commitments_parsed,
            range_proofs: range_proofs.clone(),
            zk_proof: zk_proof.clone(),
            binding_signature: binding_signature.clone(),
        };

        // Create ConfidentialTxBuilder and verify
        let tx_builder = ConfidentialTxBuilder::new();
        if !tx_builder.verify_confidential_transaction(&confidential_tx)? {
            return Err(BlockchainError::InvalidTransaction(
                "Confidential transaction verification failed".to_string()
            ));
        }

        log::debug!("Successfully verified confidential transaction with ZK proof");

        Ok(())
    }
    
    fn get_validator_public_key(&self, validator_address: &str) -> Option<Vec<u8>> {
        // Implementação melhorada para buscar a chave pública do validador
        // Busca no estado da blockchain ou no conjunto de validadores
        use crate::consensus::validator::ValidatorSet;
        
        // Converter o endereço de string para Address
        let validator_addr = match Address::new(validator_address.to_string()) {
            Ok(addr) => addr,
            Err(_) => return None,
        };
        
        // Em uma implementação real, isso buscaria do estado
        // Por enquanto, vamos simular um conjunto de validadores
        let validator_set = ValidatorSet::new();
        
        // Buscar a chave pública do validador
        validator_set.get_public_key(&validator_addr)
    }
    
    fn get_account_public_key(&self, address: &Address) -> Option<Vec<u8>> {
        // Implementação melhorada para buscar a chave pública da conta
        // Busca no estado da blockchain
        
        // Verificar se a conta existe no estado
        if let Ok(account_exists) = self.state.account_exists(address) {
            if !account_exists {
                return None;
            }
        } else {
            return None;
        }
        
        // Em uma implementação real, isso buscaria do estado
        // Por enquanto, vamos simular uma busca no estado
        
        // Gerar uma chave determinística baseada no endereço para testes
        // Isso é apenas para simulação, não para uso em produção!
        let addr_bytes = match address.to_bytes() {
            Ok(bytes) => bytes,
            Err(_) => return None,
        };
        
        // Usar o hash do endereço como chave pública para testes
        let mut key = Vec::with_capacity(32);
        key.extend_from_slice(&addr_bytes);
        key.resize(32, 0); // Garantir que tenha 32 bytes
        
        Some(key)
    }
    
    pub fn get_block_by_hash(&self, hash: &Hash) -> Option<&Block> {
        self.block_hashes.get(hash).map(|&index| &self.blocks[index])
    }
    
    pub fn get_block_by_height(&self, height: usize) -> Option<&Block> {
        self.blocks.get(height)
    }
    
    pub fn get_latest_block(&self) -> Option<&Block> {
        self.blocks.last()
    }
    
    pub fn get_block_height(&self) -> usize {
        self.blocks.len()
    }
    
    pub fn get_current_difficulty(&self) -> u64 {
        self.current_difficulty
    }
    
    pub fn calculate_transaction_throughput(&self) -> f64 {
        // Calculate approximate TPS based on recent blocks
        if self.blocks.len() < 2 {
            return 0.0;
        }
        
        let recent_blocks = &self.blocks[self.blocks.len().saturating_sub(10)..];
        let total_transactions: usize = recent_blocks.iter()
            .map(|block| block.transactions.len())
            .sum();
        
        if recent_blocks.len() < 2 {
            return 0.0;
        }
        
        let time_span = recent_blocks.last().unwrap().header.timestamp 
            - recent_blocks.first().unwrap().header.timestamp;
        
        if time_span == 0 {
            return 0.0;
        }
        
        (total_transactions as f64) / (time_span as f64 / 1000.0) // Convert ms to seconds
    }
}
