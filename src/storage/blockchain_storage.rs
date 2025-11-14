// src/storage/blockchain_storage.rs

use crate::core::{Block, Transaction, State};
use crate::storage::database::{Database, DatabaseConfig};
use crate::utils::error::{Result, BlockchainError};
use std::sync::Arc;
use tokio::sync::RwLock;

/// Storage layer for blockchain data persistence
pub struct BlockchainStorage {
    database: Arc<RwLock<Database>>,
}

impl BlockchainStorage {
    /// Create new blockchain storage
    pub async fn new(data_dir: &str) -> Result<Self> {
        let config = DatabaseConfig {
            path: format!("{}/blockchain", data_dir),
            sync_wal: true,
            sync_data: true,
            stats: true,
            columns: 4, // 0: blocks, 1: transactions, 2: state, 3: metadata
            ordered_columns: vec![0, 1, 2], // Ordered columns for iteration
            encryption_enabled: false, // Can be enabled later
        };

        let database = Database::with_config(config)?;
        Ok(Self {
            database: Arc::new(RwLock::new(database)),
        })
    }

    /// Store a block
    pub async fn store_block(&self, block: &Block) -> Result<()> {
        let mut db = self.database.write().await;

        // Serialize block
        let block_data = bincode::serialize(block)
            .map_err(|e| BlockchainError::Serialization(format!("Failed to serialize block: {}", e)))?;

        // Store block by hash
        let block_key = format!("block:{}", block.hash().to_hex()).into_bytes();
        db.put(&block_key, &block_data)?;

        // Store block by height for ordered access
        let height_key = format!("height:{:010}", block.header.number).into_bytes();
        db.put(&height_key, block.hash().as_bytes())?;

        // Update latest block height
        let latest_key = b"metadata:latest_height";
        let height_bytes = block.header.number.to_be_bytes();
        db.put(latest_key, &height_bytes)?;

        log::debug!("Stored block {} at height {}", block.hash().to_hex(), block.header.number);
        Ok(())
    }

    /// Load a block by hash
    pub async fn load_block(&self, block_hash: &[u8]) -> Result<Option<Block>> {
        let mut db = self.database.write().await;

        let block_key = format!("block:{}", hex::encode(block_hash)).into_bytes();

        if let Some(block_data) = db.get(&block_key)? {
            let block: Block = bincode::deserialize(&block_data)
                .map_err(|e| BlockchainError::Serialization(format!("Failed to deserialize block: {}", e)))?;
            Ok(Some(block))
        } else {
            Ok(None)
        }
    }

    /// Load a block by height
    pub async fn load_block_by_height(&self, height: u64) -> Result<Option<Block>> {
        let mut db = self.database.write().await;

        let height_key = format!("height:{:010}", height).into_bytes();

        if let Some(hash_bytes) = db.get(&height_key)? {
            self.load_block(&hash_bytes).await
        } else {
            Ok(None)
        }
    }

    /// Get latest block height
    pub async fn get_latest_height(&self) -> Result<u64> {
        let mut db = self.database.write().await;

        let latest_key = b"metadata:latest_height";
        if let Some(height_bytes) = db.get(latest_key)? {
            if height_bytes.len() == 8 {
                let height = u64::from_be_bytes(height_bytes.try_into().unwrap());
                Ok(height)
            } else {
                Ok(0)
            }
        } else {
            Ok(0)
        }
    }

    /// Store blockchain state
    pub async fn store_state(&self, state: &State) -> Result<()> {
        let mut db = self.database.write().await;

        // Serialize state (simplified - in production would need proper state serialization)
        let state_data = bincode::serialize(state)
            .map_err(|e| BlockchainError::Serialization(format!("Failed to serialize state: {}", e)))?;

        let state_key = b"state:latest";
        db.put(state_key, &state_data)?;

        log::debug!("Stored blockchain state");
        Ok(())
    }

    /// Load blockchain state
    pub async fn load_state(&self) -> Result<Option<State>> {
        let mut db = self.database.write().await;

        let state_key = b"state:latest";
        if let Some(state_data) = db.get(state_key)? {
            let state: State = bincode::deserialize(&state_data)
                .map_err(|e| BlockchainError::Serialization(format!("Failed to deserialize state: {}", e)))?;
            Ok(Some(state))
        } else {
            Ok(None)
        }
    }

    /// Store transaction (for indexing)
    pub async fn store_transaction(&self, tx: &Transaction, block_height: u64, tx_index: usize) -> Result<()> {
        let mut db = self.database.write().await;

        // Store transaction by hash
        let tx_data = bincode::serialize(tx)
            .map_err(|e| BlockchainError::Serialization(format!("Failed to serialize transaction: {}", e)))?;

        let tx_key = format!("tx:{}", tx.hash().to_hex()).into_bytes();
        db.put(&tx_key, &tx_data)?;

        // Store transaction metadata
        let metadata = serde_json::json!({
            "block_height": block_height,
            "tx_index": tx_index,
            "hash": tx.hash().to_hex()
        });

        let metadata_key = format!("tx_meta:{}", tx.hash().to_hex()).into_bytes();
        let metadata_bytes = serde_json::to_vec(&metadata)
            .map_err(|e| BlockchainError::Serialization(format!("Failed to serialize tx metadata: {}", e)))?;

        db.put(&metadata_key, &metadata_bytes)?;

        log::debug!("Stored transaction {}", tx.hash().to_hex());
        Ok(())
    }

    /// Load transaction by hash
    pub async fn load_transaction(&self, tx_hash: &[u8]) -> Result<Option<Transaction>> {
        let mut db = self.database.write().await;

        let tx_key = format!("tx:{}", hex::encode(tx_hash)).into_bytes();

        if let Some(tx_data) = db.get(&tx_key)? {
            let tx: Transaction = bincode::deserialize(&tx_data)
                .map_err(|e| BlockchainError::Serialization(format!("Failed to deserialize transaction: {}", e)))?;
            Ok(Some(tx))
        } else {
            Ok(None)
        }
    }

    /// Get transaction metadata
    pub async fn get_transaction_metadata(&self, tx_hash: &[u8]) -> Result<Option<serde_json::Value>> {
        let mut db = self.database.write().await;

        let metadata_key = format!("tx_meta:{}", hex::encode(tx_hash)).into_bytes();

        if let Some(metadata_bytes) = db.get(&metadata_key)? {
            let metadata: serde_json::Value = serde_json::from_slice(&metadata_bytes)
                .map_err(|e| BlockchainError::Serialization(format!("Failed to deserialize tx metadata: {}", e)))?;
            Ok(Some(metadata))
        } else {
            Ok(None)
        }
    }

    /// Store genesis configuration
    pub async fn store_genesis_config(&self, config: &serde_json::Value) -> Result<()> {
        let mut db = self.database.write().await;

        let config_bytes = serde_json::to_vec(config)
            .map_err(|e| BlockchainError::Serialization(format!("Failed to serialize genesis config: {}", e)))?;

        let config_key = b"metadata:genesis_config";
        db.put(config_key, &config_bytes)?;

        log::info!("Stored genesis configuration");
        Ok(())
    }

    /// Load genesis configuration
    pub async fn load_genesis_config(&self) -> Result<Option<serde_json::Value>> {
        let mut db = self.database.write().await;

        let config_key = b"metadata:genesis_config";
        if let Some(config_bytes) = db.get(config_key)? {
            let config: serde_json::Value = serde_json::from_slice(&config_bytes)
                .map_err(|e| BlockchainError::Serialization(format!("Failed to deserialize genesis config: {}", e)))?;
            Ok(Some(config))
        } else {
            Ok(None)
        }
    }

    /// Get database statistics
    pub async fn get_stats(&self) -> Result<serde_json::Value> {
        let db = self.database.read().await;
        let stats = db.get_stats()?;

        Ok(serde_json::json!({
            "total_keys": stats.total_keys,
            "total_size": stats.total_size,
            "cache_hits": stats.cache_hits,
            "cache_misses": stats.cache_misses,
            "buffer_stats": db.get_buffer_stats()
        }))
    }

    /// Flush all pending writes
    pub async fn flush(&self) -> Result<()> {
        let mut db = self.database.write().await;
        db.flush()?;
        Ok(())
    }

    /// Optimize database
    pub async fn optimize(&self) -> Result<()> {
        let mut db = self.database.write().await;
        db.optimize()?;
        Ok(())
    }

    /// Create backup
    pub async fn create_backup(&self, backup_path: &str) -> Result<()> {
        let db = self.database.read().await;
        db.create_backup(backup_path)?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    #[tokio::test]
    async fn test_blockchain_storage() {
        let temp_dir = TempDir::new().unwrap();
        let storage = BlockchainStorage::new(temp_dir.path().to_str().unwrap()).await.unwrap();

        // Test genesis config storage
        let genesis_config = serde_json::json!({
            "chain_id": 137,
            "initial_supply": "1000000000"
        });

        storage.store_genesis_config(&genesis_config).await.unwrap();
        let loaded_config = storage.load_genesis_config().await.unwrap().unwrap();
        assert_eq!(loaded_config, genesis_config);

        // Test stats
        let stats = storage.get_stats().await.unwrap();
        assert!(stats.is_object());
    }
}
