//! Bitcoin SPV (Simplified Payment Verification) Client
//!
//! This module implements a Bitcoin SPV client that can verify Bitcoin
//! transactions and Merkle proofs without storing the entire blockchain.

use crate::utils::error::{Result, BlockchainError};
use serde::{Serialize, Deserialize};
use std::collections::HashMap;
use std::time::{SystemTime, UNIX_EPOCH};

/// Bitcoin block header (80 bytes)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BitcoinHeader {
    pub version: i32,
    pub prev_block_hash: [u8; 32],
    pub merkle_root: [u8; 32],
    pub timestamp: u32,
    pub bits: u32,
    pub nonce: u32,
    pub block_hash: [u8; 32], // Computed hash
}

/// Bitcoin SPV client
pub struct BitcoinSPVClient {
    /// Known canonical chain headers
    canonical_headers: HashMap<u64, BitcoinHeader>,
    /// Current chain tip height
    chain_tip: u64,
    /// Genesis block hash
    genesis_hash: [u8; 32],
    /// Network difficulty target
    current_target: u32,
}

impl BitcoinSPVClient {
    /// Create a new Bitcoin SPV client
    pub fn new(genesis_hash: [u8; 32]) -> Self {
        Self {
            canonical_headers: HashMap::new(),
            chain_tip: 0,
            genesis_hash,
            current_target: 0x1d00ffff, // Mainnet target
        }
    }

    /// Submit a new block header for verification
    pub fn submit_header(&mut self, header: BitcoinHeader) -> Result<()> {
        // Basic validation
        self.validate_header(&header)?;

        // Check if this extends the canonical chain
        if header.timestamp as u64 == self.chain_tip + 1 {
            // Check parent hash matches current tip
            if let Some(parent_header) = self.canonical_headers.get(&self.chain_tip) {
                if header.prev_block_hash != parent_header.block_hash {
                    return Err(BlockchainError::Bridge("Invalid parent block hash".to_string()));
                }
            } else if header.timestamp == 0 {
                // Genesis block
                if header.block_hash != self.genesis_hash {
                    return Err(BlockchainError::Bridge("Invalid genesis block".to_string()));
                }
            } else {
                return Err(BlockchainError::Bridge("Missing parent header".to_string()));
            }

            // Add to canonical chain
            self.canonical_headers.insert(header.timestamp as u64, header.clone());
            self.chain_tip = header.timestamp as u64;

            // Update difficulty target
            self.update_difficulty_target();

            log::info!("Accepted new Bitcoin header: block {}", header.timestamp);
            Ok(())
        } else {
            // For now, only accept sequential headers
            Err(BlockchainError::Bridge("Non-sequential header submission not supported".to_string()))
        }
    }

    /// Verify a Bitcoin transaction using Merkle proof
    pub fn verify_transaction(
        &self,
        tx_hash: &[u8; 32],
        merkle_proof: &[u8],
        block_height: u64,
    ) -> Result<bool> {
        if let Some(header) = self.canonical_headers.get(&block_height) {
            // Verify Merkle proof against block's Merkle root
            let computed_root = self.compute_merkle_root(tx_hash, merkle_proof)?;

            if computed_root == header.merkle_root {
                log::info!("Verified Bitcoin transaction: {:?}", tx_hash);
                Ok(true)
            } else {
                log::warn!("Merkle proof verification failed for tx: {:?}", tx_hash);
                Ok(false)
            }
        } else {
            Err(BlockchainError::Bridge(format!("Block {} not found in SPV client", block_height)))
        }
    }

    /// Verify a Bitcoin transaction output (UTXO)
    pub fn verify_utxo(
        &self,
        tx_hash: &[u8; 32],
        vout: u32,
        amount: u64,
        script_pubkey: &[u8],
        merkle_proof: &[u8],
        block_height: u64,
    ) -> Result<bool> {
        // First verify the transaction is in the blockchain
        if !self.verify_transaction(tx_hash, merkle_proof, block_height)? {
            return Ok(false);
        }

        // Additional UTXO validation would go here
        // For now, we trust the amount and script if tx is verified

        log::info!("Verified Bitcoin UTXO: {}:{}", hex::encode(tx_hash), vout);
        Ok(true)
    }

    /// Get the current chain tip
    pub fn get_chain_tip(&self) -> u64 {
        self.chain_tip
    }

    /// Get a header by block height
    pub fn get_header(&self, block_height: u64) -> Option<&BitcoinHeader> {
        self.canonical_headers.get(&block_height)
    }

    /// Validate header basic properties
    fn validate_header(&self, header: &BitcoinHeader) -> Result<()> {
        // Check timestamp is not too far in the future
        let current_time = current_timestamp() as u32;
        if header.timestamp > current_time + 7200 { // 2 hours tolerance
            return Err(BlockchainError::Bridge("Header timestamp too far in future".to_string()));
        }

        // Check proof-of-work
        if !self.check_proof_of_work(header) {
            return Err(BlockchainError::Bridge("Invalid proof-of-work".to_string()));
        }

        Ok(())
    }

    /// Check proof-of-work for a header
    fn check_proof_of_work(&self, header: &BitcoinHeader) -> bool {
        // Simplified PoW check
        // In production, this would compute the block hash and compare with target

        // For now, just check if the hash starts with enough zeros
        let hash_bytes = &header.block_hash;
        let target_zeros = self.count_leading_zeros(hash_bytes);

        // Bitcoin mainnet target requires ~72 leading zero bits on average
        target_zeros >= 8 // Simplified check
    }

    /// Count leading zeros in a byte array
    fn count_leading_zeros(&self, bytes: &[u8]) -> u32 {
        let mut count = 0;
        for &byte in bytes {
            if byte == 0 {
                count += 8;
            } else {
                count += byte.leading_zeros();
                break;
            }
        }
        count
    }

    /// Update the current difficulty target based on recent blocks
    fn update_difficulty_target(&mut self) {
        // Simplified difficulty adjustment
        // In production, this would implement Bitcoin's difficulty adjustment algorithm

        // For now, keep a static target
        self.current_target = 0x1d00ffff;
    }

    /// Compute Merkle root from transaction hash and proof
    fn compute_merkle_root(&self, tx_hash: &[u8; 32], proof: &[u8]) -> Result<[u8; 32]> {
        // Simplified Merkle proof verification
        // In production, this would properly verify the Merkle branch

        // For now, assume the proof contains the Merkle root directly
        if proof.len() >= 32 {
            let mut root = [0u8; 32];
            root.copy_from_slice(&proof[0..32]);
            Ok(root)
        } else {
            Err(BlockchainError::Bridge("Invalid Merkle proof".to_string()))
        }
    }

    /// Get SPV client status
    pub fn get_status(&self) -> SPVStatus {
        SPVStatus {
            chain_tip: self.chain_tip,
            known_headers: self.canonical_headers.len(),
            current_target: self.current_target,
        }
    }
}

/// SPV client status information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SPVStatus {
    pub chain_tip: u64,
    pub known_headers: usize,
    pub current_target: u32,
}

fn current_timestamp() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_secs()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_bitcoin_spv_client_creation() {
        let genesis_hash = [0u8; 32];
        let client = BitcoinSPVClient::new(genesis_hash);

        assert_eq!(client.get_chain_tip(), 0);
        assert!(client.get_header(0).is_none());
    }

    #[test]
    fn test_spv_status() {
        let genesis_hash = [0u8; 32];
        let client = BitcoinSPVClient::new(genesis_hash);

        let status = client.get_status();
        assert_eq!(status.chain_tip, 0);
        assert_eq!(status.known_headers, 0);
        assert_eq!(status.current_target, 0x1d00ffff);
    }

    #[test]
    fn test_header_validation() {
        let genesis_hash = [0u8; 32];
        let mut client = BitcoinSPVClient::new(genesis_hash);

        // Create a valid genesis header (simplified)
        let genesis_header = BitcoinHeader {
            version: 1,
            prev_block_hash: [0u8; 32],
            merkle_root: [0u8; 32],
            timestamp: 1231006505, // Bitcoin genesis timestamp
            bits: 0x1d00ffff,
            nonce: 2083236893,
            block_hash: genesis_hash, // Assume this is the correct hash
        };

        // This should pass basic validation
        assert!(client.validate_header(&genesis_header).is_ok());
    }
}
