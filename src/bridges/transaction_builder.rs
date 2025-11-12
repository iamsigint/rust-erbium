//! Transaction builder for cross-chain operations

use crate::utils::error::Result;
use serde::{Serialize, Deserialize};

/// Transaction builder for cross-chain operations
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TransactionBuilder {
    pub source_chain: String,
    pub target_chain: String,
    pub transaction_type: TransactionType,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum TransactionType {
    Transfer,
    Swap,
    Bridge,
    ContractCall,
}

impl TransactionBuilder {
    /// Create a new transaction builder
    pub fn new(source_chain: String, target_chain: String, transaction_type: TransactionType) -> Self {
        Self {
            source_chain,
            target_chain,
            transaction_type,
        }
    }

    /// Build a cross-chain transaction
    pub fn build_transaction(&self, params: TransactionParams) -> Result<Vec<u8>> {
        // Mock transaction building
        let mut tx_data = Vec::new();
        tx_data.extend_from_slice(params.sender.as_bytes());
        tx_data.extend_from_slice(params.recipient.as_bytes());
        tx_data.extend_from_slice(&params.amount.to_le_bytes());
        Ok(tx_data)
    }
}

/// Transaction parameters
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TransactionParams {
    pub sender: String,
    pub recipient: String,
    pub amount: u64,
    pub data: Vec<u8>,
}
