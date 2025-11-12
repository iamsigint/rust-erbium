// src/bridges/light_clients/bitcoin_client.rs
use bitcoin::block::Header as BlockHeader;
use bitcoin::TxMerkleNode;
use bitcoin::hashes::{Hash, sha256d};
use super::errors::LightClientError;
use std::collections::HashMap;
use std::sync::{Arc, RwLock};

/// Bitcoin light client for SPV (Simplified Payment Verification)
pub struct BitcoinLightClient {
    headers: Arc<RwLock<HashMap<u64, BlockHeader>>>,
    best_height: Arc<RwLock<u64>>,
}

impl BitcoinLightClient {
    /// Create a new Bitcoin light client
    pub fn new() -> Self {
        Self {
            headers: Arc::new(RwLock::new(HashMap::new())),
            best_height: Arc::new(RwLock::new(0)),
        }
    }

    /// Add a new block header to the light client
    pub fn add_header(&self, header: BlockHeader, height: u64) -> Result<(), LightClientError> {
        let mut headers = self.headers.write().unwrap();
        let mut best_height = self.best_height.write().unwrap();

        // Verify header links to previous
        if height > 0 {
            if let Some(prev_header) = headers.get(&(height - 1)) {
                if header.prev_blockhash != prev_header.block_hash() {
                    return Err(LightClientError::InvalidHeader("Header doesn't link to previous".to_string()));
                }
            }
        }

        // Verify proof of work
        if !self.verify_pow(&header) {
            return Err(LightClientError::InvalidProofOfWork);
        }

        headers.insert(height, header);
        if height > *best_height {
            *best_height = height;
        }

        log::debug!("Added Bitcoin header at height: {}", height);
        Ok(())
    }

    /// Verify transaction inclusion using Merkle proof
    pub fn verify_transaction_inclusion(
        &self,
        txid: &sha256d::Hash,
        block_height: u64,
        merkle_proof: &MerkleProof,
    ) -> Result<bool, LightClientError> {
        let headers = self.headers.read().unwrap();
        let header = headers.get(&block_height)
            .ok_or_else(|| LightClientError::HeaderNotFound(block_height))?;

        // Verify Merkle root matches
        let calculated_root = self.calculate_merkle_root(txid, merkle_proof)?;
        if calculated_root != header.merkle_root {
            return Ok(false);
        }

        Ok(true)
    }

    /// Get current best block height
    pub fn get_best_height(&self) -> u64 {
        *self.best_height.read().unwrap()
    }

    /// Get header at specific height
    pub fn get_header(&self, height: u64) -> Option<BlockHeader> {
        self.headers.read().unwrap().get(&height).cloned()
    }

    /// Verify proof of work for a header
    fn verify_pow(&self, header: &BlockHeader) -> bool {
        let target = header.target();
        
        // Verify the block hash meets the target difficulty
        header.validate_pow(target).is_ok()
    }

    /// Calculate Merkle root from transaction and proof
    fn calculate_merkle_root(
        &self,
        txid: &sha256d::Hash,
        proof: &MerkleProof,
    ) -> Result<TxMerkleNode, LightClientError> {
        let mut current_hash = *txid;

        for (hash, is_right) in &proof.hashes {
            let mut data = Vec::new();
            
            if *is_right {
                // Current hash is left, provided hash is right
                data.extend_from_slice(current_hash.as_byte_array());
                data.extend_from_slice(hash.as_byte_array());
            } else {
                // Provided hash is left, current hash is right
                data.extend_from_slice(hash.as_byte_array());
                data.extend_from_slice(current_hash.as_byte_array());
            }

            current_hash = sha256d::Hash::hash(&data);
        }

        Ok(TxMerkleNode::from_raw_hash(current_hash))
    }
}

/// Merkle proof for Bitcoin transaction inclusion
#[derive(Debug, Clone)]
pub struct MerkleProof {
    pub hashes: Vec<(sha256d::Hash, bool)>, // (hash, is_right)
    pub tx_index: u32,
    pub total_txs: u32,
}

impl Default for BitcoinLightClient {
    fn default() -> Self {
        Self::new()
    }
}