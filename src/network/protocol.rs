use serde::{Deserialize, Serialize};
use crate::core::{Block, Transaction};
use crate::core::types::Hash;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum NetworkMessage {
    // Block propagation
    NewBlock(Block),
    RequestBlock(Hash),
    ResponseBlock(Block),
    
    // Transaction propagation
    NewTransaction(Transaction),
    
    // Chain synchronization
    RequestBlocks(Vec<Hash>), // Request specific blocks
    RequestChainSync(u64), // Request blocks from specific height
    
    // Peer discovery
    PeerList(Vec<String>), // List of peer addresses
    
    // Consensus messages
    Vote(VoteMessage),
    
    // Bridge messages
    CrossChainMessage(CrossChainData),
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VoteMessage {
    pub block_hash: Hash,
    pub validator: String,
    pub signature: Vec<u8>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CrossChainData {
    pub source_chain: String,
    pub target_chain: String,
    pub message: Vec<u8>,
    pub proof: Vec<u8>,
}

impl NetworkMessage {
    pub fn serialize(&self) -> Result<Vec<u8>, Box<dyn std::error::Error>> {
        Ok(bincode::serialize(self)?)
    }
    
    pub fn deserialize(data: &[u8]) -> Result<Self, Box<dyn std::error::Error>> {
        Ok(bincode::deserialize(data)?)
    }
}
