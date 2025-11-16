// src/network/sync.rs

use crate::core::{Block, Blockchain};
use crate::network::p2p::P2PNode;
use crate::utils::error::{BlockchainError, Result};
use std::sync::Arc;
use tokio::sync::Mutex;

/// Blockchain synchronization status
#[derive(Debug, Clone, PartialEq)]
pub enum SyncStatus {
    /// Node is fully synchronized with the network
    Synced,
    /// Node is currently syncing, showing the current block height
    Syncing(u64),
    /// Node is behind, showing the number of missing blocks
    Behind(u64),
    /// Node is initializing
    Initializing,
}

/// Blockchain synchronization manager
pub struct SyncManager {
    blockchain: Arc<Mutex<Blockchain>>,
    _p2p_node: Arc<Mutex<P2PNode>>,
    status: SyncStatus,
    highest_seen_block: u64,
}

impl SyncManager {
    /// Creates a new synchronization manager
    pub fn new(blockchain: Arc<Mutex<Blockchain>>, p2p_node: Arc<Mutex<P2PNode>>) -> Self {
        Self {
            blockchain,
            _p2p_node: p2p_node,
            status: SyncStatus::Initializing,
            highest_seen_block: 0,
        }
    }

    /// Starts the blockchain synchronization process
    pub async fn start_sync(&mut self) -> Result<()> {
        log::info!("Starting blockchain synchronization");

        // Update status to syncing
        self.status = SyncStatus::Syncing(0);

        // Request the latest block from the network
        self.request_latest_block().await?;

        // Start the main synchronization loop
        self.sync_loop().await
    }

    /// Requests the latest block from the network
    async fn request_latest_block(&mut self) -> Result<()> {
        log::debug!("Requesting the latest block from the network");

        // In a real implementation, this would send a message to the network
        // Here, we only simulate receiving a response

        // Update the highest seen block
        let blockchain = self.blockchain.lock().await;
        let current_height = blockchain.get_block_height() as u64;
        drop(blockchain);

        // Simulate that we saw a higher block on the network
        self.highest_seen_block = current_height + 10;

        Ok(())
    }

    /// Main synchronization loop
    async fn sync_loop(&mut self) -> Result<()> {
        log::info!("Starting synchronization loop");

        loop {
            // Check the current blockchain state
            let blockchain = self.blockchain.lock().await;
            let current_height = blockchain.get_block_height() as u64;
            drop(blockchain);

            if current_height >= self.highest_seen_block {
                // We are synchronized
                self.status = SyncStatus::Synced;
                log::info!("Blockchain synced at height {}", current_height);
                break;
            } else {
                // Still behind, need to continue syncing
                let blocks_behind = self.highest_seen_block - current_height;
                self.status = SyncStatus::Behind(blocks_behind);

                log::info!(
                    "Syncing: current_height={}, target_height={}, blocks_remaining={}",
                    current_height,
                    self.highest_seen_block,
                    blocks_behind
                );

                // Request the next batch of blocks
                self.request_blocks(current_height + 1, 50).await?;

                // Simulate a delay to avoid overloading the system
                tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;
            }
        }

        Ok(())
    }

    /// Requests a batch of blocks from the network
    async fn request_blocks(&self, start_height: u64, count: u64) -> Result<()> {
        log::debug!(
            "Requesting blocks {} to {}",
            start_height,
            start_height + count - 1
        );

        // In a real implementation, this would send a network message
        // and process the received blocks

        // Simulate processing received blocks
        self.process_received_blocks(start_height, count).await
    }

    /// Processes blocks received from the network
    async fn process_received_blocks(&self, start_height: u64, count: u64) -> Result<()> {
        log::debug!(
            "Processing {} blocks starting from height {}",
            count,
            start_height
        );

        // In a real implementation, this would validate and add blocks to the blockchain
        // Here, we only simulate the process

        // Simulate adding blocks to the blockchain
        let mut blockchain = self.blockchain.lock().await;

        for i in 0..count {
            let height = start_height + i;

            // Stop if we reached the highest seen block
            if height > self.highest_seen_block {
                break;
            }

            // Simulate block creation
            if let Some(last_block) = blockchain.get_latest_block() {
                let transactions = Vec::new(); // No transactions in this simulation

                // Create a new block based on the last one
                let mut new_block = Block::new(
                    height,
                    last_block.hash(),
                    transactions,
                    "sync_validator".to_string(),
                    blockchain.get_current_difficulty(),
                );

                // Simulate block signing (would be properly done in a real implementation)
                let dummy_key = vec![0u8; 32];
                new_block.sign(&dummy_key)?;

                // Add the new block to the blockchain
                blockchain.add_block(new_block)?;

                log::debug!("Added synced block at height {}", height);
            } else {
                return Err(BlockchainError::InvalidBlock(
                    "Failed to retrieve latest block".to_string(),
                ));
            }
        }

        Ok(())
    }

    /// Returns the current synchronization status
    pub fn get_status(&self) -> SyncStatus {
        self.status.clone()
    }

    /// Checks if the node is fully synchronized
    pub fn is_synced(&self) -> bool {
        self.status == SyncStatus::Synced
    }
}
