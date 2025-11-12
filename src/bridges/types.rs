//! Common types for bridge operations

use serde::{Serialize, Deserialize};

/// Bridge message
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BridgeMessage {
    pub id: u64,
    pub source_chain: String,
    pub target_chain: String,
    pub payload: Vec<u8>,
    pub timestamp: u64,
}

/// Bridge transaction
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BridgeTransaction {
    pub id: String,
    pub from_chain: String,
    pub to_chain: String,
    pub amount: u64,
    pub sender: String,
    pub recipient: String,
    pub status: TransactionStatus,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum TransactionStatus {
    Pending,
    Confirmed,
    Failed,
}
