// src/core/mod.rs

pub mod block;
pub mod chain;
pub mod dex;
pub mod layer2;
pub mod mempool;
pub mod precompiles;
pub mod state;
pub mod transaction;
pub mod types;
pub mod units;
pub mod vm;

// Re-export commonly used types and functions
pub use block::Block;
pub use transaction::{Transaction, TransactionType};
pub use state::State;
pub use chain::Blockchain;
pub use types::{Hash, Address, Timestamp, Difficulty, Nonce};

use crate::utils::error::{Result, BlockchainError};

/// Core blockchain configuration
#[derive(Debug, Clone)]
pub struct BlockchainConfig {
    pub block_time: u64,         // Target block time in seconds
    pub max_block_size: usize,     // Maximum block size in bytes
    pub max_transactions_per_block: usize,
    pub genesis_timestamp: u64,    // Genesis block timestamp
    pub chain_id: u64,             // Network chain ID
}

impl Default for BlockchainConfig {
    fn default() -> Self {
        Self {
            block_time: 30,       // 30 seconds per block
            max_block_size: 8 * 1024 * 1024, // 8MB
            max_transactions_per_block: 10000,
            genesis_timestamp: 1635724800000, // Example timestamp
            chain_id: 137,        // Erbium mainnet chain ID
        }
    }
}

/// Blockchain status information
#[derive(Debug, Clone)]
pub struct BlockchainStatus {
    pub block_height: u64,
    pub latest_block_hash: Hash,
    pub total_transactions: u64,
    pub total_validators: usize,
    pub current_difficulty: u64,
    pub sync_status: SyncStatus,
    pub network_hashrate: f64,
}

#[derive(Debug, Clone)]
pub enum SyncStatus {
    Synced,
    Syncing(u64), // Current block height being synced
    Behind(u64),  // Blocks behind
}

/// Utility functions for core blockchain operations
pub fn validate_address(address: &Address) -> bool {
    // Basic address validation
    let addr_str = address.as_str();
    if addr_str.starts_with("0x") && addr_str.len() == 42 {
        // Ethereum-style address
        hex::decode(&addr_str[2..]).is_ok()
    } else {
        // Other address formats could be supported here
        false
    }
}

/// Calculate transaction fee based on gas and gas price
pub fn calculate_transaction_fee(gas_used: u64, gas_price: u64) -> u64 {
    gas_used.saturating_mul(gas_price)
}

/// Validate transaction basic structure
pub fn validate_transaction_structure(transaction: &Transaction) -> Result<()> {
    if transaction.fee == 0 {
        return Err(BlockchainError::InvalidTransaction("Zero fee not allowed".to_string()));
    }
    
    if transaction.timestamp > chrono::Utc::now().timestamp_millis() as u64 + 300000 {
        // 5 minutes in future maximum
        return Err(BlockchainError::InvalidTransaction("Timestamp too far in future".to_string()));
    }
    
    if !validate_address(&transaction.from) {
        return Err(BlockchainError::InvalidTransaction("Invalid sender address".to_string()));
    }
    
    if !validate_address(&transaction.to) {
        return Err(BlockchainError::InvalidTransaction("Invalid recipient address".to_string()));
    }
    
    Ok(())
}

/// Calculate block reward based on block height and configuration
pub fn calculate_block_reward(block_height: u64, config: &BlockchainConfig) -> u64 {
    // Simple block reward calculation
    // In real implementation, this would follow the emission schedule
    let base_reward = 10_000_000; // 10 ERB base reward
    
    // Halving every 4 years (assuming 30s blocks)
    let halving_interval = 4 * 365 * 24 * 60 * 60 / config.block_time;
    let halvings = block_height / halving_interval;
    
    base_reward / 2u64.pow(halvings as u32)
}

/// Calculate next block difficulty based on current difficulty and block times
pub fn calculate_next_difficulty(
    current_difficulty: u64,
    recent_block_times: &[u64],
    target_block_time: u64,
) -> u64 {
    if recent_block_times.len() < 2 {
        return current_difficulty;
    }
    
    let total_time: u64 = recent_block_times.windows(2)
        .map(|window| window[1] - window[0])
        .sum();
    
    let average_block_time = total_time / (recent_block_times.len() - 1) as u64;
    
    if average_block_time < target_block_time {
        // Blocks are coming too fast, increase difficulty
        current_difficulty * target_block_time / average_block_time
    } else {
        // Blocks are coming too slow, decrease difficulty
        current_difficulty * average_block_time / target_block_time
    }
}

/// Merkle tree utilities
pub struct MerkleTree;

impl MerkleTree {
    /// Calculate Merkle root from a list of hashes
    pub fn calculate_root(hashes: &[Hash]) -> Hash {
        if hashes.is_empty() {
            return Hash::new(b"empty");
        }
        
        if hashes.len() == 1 {
            return hashes[0];
        }
        
        let mut current_level = hashes.to_vec();
        
        while current_level.len() > 1 {
            let mut next_level = Vec::new();
            
            for chunk in current_level.chunks(2) {
                if chunk.len() == 2 {
                    // CORRIGIDO: Use chain e collect em vez de concat
                    let combined: Vec<u8> = chunk[0].as_bytes().iter()
                        .chain(chunk[1].as_bytes().iter())
                        .cloned()
                        .collect();
                    next_level.push(Hash::new(&combined));
                } else {
                    next_level.push(chunk[0]);
                }
            }
            
            current_level = next_level;
        }
        
        current_level[0]
    }
    
    /// Generate Merkle proof for a specific leaf
    pub fn generate_proof(hashes: &[Hash], leaf_index: usize) -> Option<Vec<Hash>> {
        if leaf_index >= hashes.len() {
            return None;
        }
        
        let mut proof = Vec::new();
        let mut current_index = leaf_index;
        let mut current_level = hashes.to_vec();
        
        while current_level.len() > 1 {
            if current_index.is_multiple_of(2) {
                // Current is left child, right sibling exists
                if current_index + 1 < current_level.len() {
                    proof.push(current_level[current_index + 1]);
                }
            } else {
                // Current is right child, left sibling exists
                proof.push(current_level[current_index - 1]);
            }

            // Move to next level
            current_index /= 2;
            current_level = Self::build_next_level(&current_level);
        }
        
        Some(proof)
    }
    
    /// Verify Merkle proof
    pub fn verify_proof(leaf: &Hash, proof: &[Hash], root: &Hash, index: usize) -> bool {
        let mut computed_hash = *leaf;
        let mut current_index = index;
        
        for proof_hash in proof {
            if current_index.is_multiple_of(2) {
                // Current is left child
                // CORRIGIDO: Use chain e collect em vez de concat
                let combined: Vec<u8> = computed_hash.as_bytes().iter()
                    .chain(proof_hash.as_bytes().iter())
                    .cloned()
                    .collect();
                computed_hash = Hash::new(&combined);
            } else {
                // Current is right child
                // CORRIGIDO: Use chain e collect em vez de concat
                let combined: Vec<u8> = proof_hash.as_bytes().iter()
                    .chain(computed_hash.as_bytes().iter())
                    .cloned()
                    .collect();
                computed_hash = Hash::new(&combined);
            }
            current_index /= 2;
        }
        
        &computed_hash == root
    }
    
    fn build_next_level(level: &[Hash]) -> Vec<Hash> {
        let mut next_level = Vec::new();
        
        for chunk in level.chunks(2) {
            if chunk.len() == 2 {
                // CORRIGIDO: Use chain e collect em vez de concat
                let combined: Vec<u8> = chunk[0].as_bytes().iter()
                    .chain(chunk[1].as_bytes().iter())
                    .cloned()
                    .collect();
                next_level.push(Hash::new(&combined));
            } else {
                next_level.push(chunk[0]);
            }
        }
        
        next_level
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::core::types::AddressError;

    #[test]
    fn test_merkle_tree() {
        let hashes = vec![
            Hash::new(b"hash1"),
            Hash::new(b"hash2"),
            Hash::new(b"hash3"),
        ];
        
        let root = MerkleTree::calculate_root(&hashes);
        assert_ne!(root, Hash::new(b"empty"));
        
        let proof = MerkleTree::generate_proof(&hashes, 0).unwrap();
        assert!(!proof.is_empty());
        
        let is_valid = MerkleTree::verify_proof(&hashes[0], &proof, &root, 0);
        assert!(is_valid);
    }
    
    #[test]
    fn test_block_reward_calculation() {
        let config = BlockchainConfig::default();
        let reward = calculate_block_reward(0, &config);
        assert_eq!(reward, 10_000_000);
    }
    
    #[test]
    fn test_address_validation() -> std::result::Result<(), AddressError> {
        // Address::new returns a Result, so we test the result
        let valid_address = Address::new("0x0000000000000000000000000000000000000000".to_string())?;
        let invalid_address_result = Address::new("invalid".to_string());
        
        assert!(validate_address(&valid_address));
        assert!(invalid_address_result.is_err());
        
        // Test the validation logic for an address that parses but might be "invalid"
        // In this case, our validation just checks parsing, so an invalid_address that parses
        // is not possible with the current Address::new implementation.
        // We can test the logic directly:
        let invalid_format_address = Address::new_unchecked("invalid".to_string());
        assert!(!validate_address(&invalid_format_address));
        
        Ok(())
    }
}
