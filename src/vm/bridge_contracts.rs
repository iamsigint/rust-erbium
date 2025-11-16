//! Bridge contract implementations for cross-chain interoperability

use crate::utils::error::{BlockchainError, Result};
use serde::{Deserialize, Serialize};

/// Bridge contract for cross-chain transfers
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BridgeContract {
    pub source_chain: String,
    pub target_chain: String,
    pub locked_tokens: u64,
    pub pending_transfers: Vec<PendingTransfer>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PendingTransfer {
    pub id: u64,
    pub sender: String,
    pub recipient: String,
    pub amount: u64,
    pub target_chain: String,
    pub status: TransferStatus,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum TransferStatus {
    Pending,
    Locked,
    Completed,
    Failed,
}

impl BridgeContract {
    /// Create a new bridge contract
    pub fn new(source_chain: String, target_chain: String) -> Self {
        Self {
            source_chain,
            target_chain,
            locked_tokens: 0,
            pending_transfers: Vec::new(),
        }
    }

    /// Initiate a cross-chain transfer
    pub fn initiate_transfer(
        &mut self,
        sender: String,
        recipient: String,
        amount: u64,
    ) -> Result<u64> {
        let transfer_id = self.pending_transfers.len() as u64 + 1;
        let transfer = PendingTransfer {
            id: transfer_id,
            sender,
            recipient,
            amount,
            target_chain: self.target_chain.clone(),
            status: TransferStatus::Pending,
        };

        self.pending_transfers.push(transfer);
        self.locked_tokens += amount;

        Ok(transfer_id)
    }

    /// Complete a transfer
    pub fn complete_transfer(&mut self, transfer_id: u64) -> Result<()> {
        if let Some(transfer) = self
            .pending_transfers
            .iter_mut()
            .find(|t| t.id == transfer_id)
        {
            if transfer.status == TransferStatus::Locked {
                transfer.status = TransferStatus::Completed;
                self.locked_tokens -= transfer.amount;
                Ok(())
            } else {
                Err(BlockchainError::Bridge("Transfer not locked".to_string()))
            }
        } else {
            Err(BlockchainError::Bridge("Transfer not found".to_string()))
        }
    }

    /// Lock tokens for transfer
    pub fn lock_tokens(&mut self, transfer_id: u64) -> Result<()> {
        if let Some(transfer) = self
            .pending_transfers
            .iter_mut()
            .find(|t| t.id == transfer_id)
        {
            if transfer.status == TransferStatus::Pending {
                transfer.status = TransferStatus::Locked;
                Ok(())
            } else {
                Err(BlockchainError::Bridge("Transfer not pending".to_string()))
            }
        } else {
            Err(BlockchainError::Bridge("Transfer not found".to_string()))
        }
    }
}
