// src/core/block.rs
use serde::{Deserialize, Serialize};
use crate::core::types::{Hash, Timestamp, Nonce};
use crate::core::transaction::Transaction;
use crate::utils::error::Result;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BlockHeader {
    pub number: u64, // Added block number (height)
    pub version: u32,
    pub previous_hash: Hash,
    pub merkle_root: Hash,
    pub timestamp: Timestamp,
    pub difficulty: u64,
    pub nonce: Nonce,
    pub validator: String, // Validator's address
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Block {
    pub header: BlockHeader,
    pub transactions: Vec<Transaction>,
    pub signature: Vec<u8>, // Dilithium signature
}

impl Block {
    pub fn new(
        number: u64, // Added block number
        previous_hash: Hash,
        transactions: Vec<Transaction>,
        validator: String,
        difficulty: u64,
    ) -> Self {
        let merkle_root = Self::calculate_merkle_root(&transactions);
        let timestamp = chrono::Utc::now().timestamp_millis() as u64;
        
        let header = BlockHeader {
            number, // Set block number
            version: 1,
            previous_hash,
            merkle_root,
            timestamp,
            difficulty,
            nonce: Nonce(0),
            validator,
        };
        
        Block {
            header,
            transactions,
            signature: Vec::new(),
        }
    }
    
    pub fn calculate_merkle_root(transactions: &[Transaction]) -> Hash {
        if transactions.is_empty() {
            return Hash::new(b"empty");
        }
        
        let mut hashes: Vec<Hash> = transactions
            .iter()
            .map(|tx| tx.hash())
            .collect();
            
        while hashes.len() > 1 {
            let mut next_level = Vec::new();
            
            for chunk in hashes.chunks(2) {
                let combined = if chunk.len() == 2 {
                    let mut combined = chunk[0].as_bytes().to_vec();
                    combined.extend_from_slice(chunk[1].as_bytes());
                    Hash::new(&combined)
                } else {
                    chunk[0]
                };
                next_level.push(combined);
            }
            
            hashes = next_level;
        }
        
        hashes[0]
    }
    
    pub fn hash(&self) -> Hash {
        let header_data = bincode::serialize(&self.header).unwrap();
        Hash::new(&header_data)
    }
    
    // Corrected return type from Result<(), BlockchainError> to Result<()>
    pub fn sign(&mut self, private_key: &[u8]) -> Result<()> {
        use crate::crypto::dilithium::Dilithium;
        let block_hash = self.hash();
        self.signature = Dilithium::sign(private_key, block_hash.as_bytes())?;
        Ok(())
    }
    
    // Corrected return type from Result<bool, BlockchainError> to Result<bool>
    pub fn verify_signature(&self, public_key: &[u8]) -> Result<bool> {
        // Use the static Dilithium struct for verification, not a Keypair instance
        use crate::crypto::dilithium::Dilithium;
        
        let block_hash = self.hash();
        // Call the static verify function
        Dilithium::verify(public_key, block_hash.as_bytes(), &self.signature)
    }
}