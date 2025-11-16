// src/core/mempool.rs

use crate::core::{Address, Hash, State, Transaction};
use crate::utils::error::{BlockchainError, Result};
use std::collections::{BTreeMap, HashMap, HashSet};
use std::sync::Arc;
use std::time::{SystemTime, UNIX_EPOCH};
use tokio::sync::RwLock;
use tokio::time::{self, Duration};

/// Mempool configuration
#[derive(Debug, Clone)]
pub struct MempoolConfig {
    pub max_size: usize,               // Maximum number of transactions
    pub max_size_bytes: usize,         // Maximum size in bytes
    pub min_fee_per_byte: u64,         // Minimum fee per byte
    pub max_transaction_age: Duration, // Maximum age before expiration
    pub cleanup_interval: Duration,    // Cleanup interval
    pub max_replacements: usize,       // Max replacements per account
}

impl Default for MempoolConfig {
    fn default() -> Self {
        Self {
            max_size: 50000,                                        // 50k transactions
            max_size_bytes: 100 * 1024 * 1024,                      // 100MB
            min_fee_per_byte: 1,                                    // 1 unit per byte minimum
            max_transaction_age: Duration::from_secs(24 * 60 * 60), // 24 hours
            cleanup_interval: Duration::from_secs(60),              // 1 minute
            max_replacements: 5, // Max 5 replacements per account
        }
    }
}

/// Transaction entry in mempool
#[derive(Debug, Clone)]
pub struct MempoolEntry {
    pub transaction: Transaction,
    pub received_at: u64,
    pub fee_per_byte: u64,
    pub size_bytes: usize,
    pub dependencies: HashSet<Hash>, // Transaction hashes this depends on
    pub dependents: HashSet<Hash>,   // Transactions that depend on this
}

impl MempoolEntry {
    pub fn new(transaction: Transaction) -> Self {
        let size_bytes = bincode::serialize(&transaction).unwrap_or_default().len();
        let fee_per_byte = if size_bytes > 0 {
            transaction.fee / size_bytes as u64
        } else {
            0
        };

        Self {
            transaction,
            received_at: current_timestamp(),
            fee_per_byte,
            size_bytes,
            dependencies: HashSet::new(),
            dependents: HashSet::new(),
        }
    }

    /// Check if transaction has expired
    pub fn is_expired(&self, max_age: Duration) -> bool {
        let age = Duration::from_secs(current_timestamp() - self.received_at);
        age > max_age
    }

    /// Get transaction hash
    pub fn hash(&self) -> Hash {
        self.transaction.hash()
    }
}

/// Mempool statistics
#[derive(Debug, Clone)]
pub struct MempoolStats {
    pub total_transactions: usize,
    pub total_size_bytes: usize,
    pub average_fee_per_byte: u64,
    pub min_fee_per_byte: u64,
    pub max_fee_per_byte: u64,
    pub pending_by_account: HashMap<Address, usize>,
}

/// Transaction pool for pending transactions
pub struct Mempool {
    config: MempoolConfig,
    transactions: HashMap<Hash, MempoolEntry>,
    by_fee: BTreeMap<(u64, Hash), Hash>, // (fee_per_byte, hash) -> hash for ordering
    by_account: HashMap<Address, HashSet<Hash>>, // Account -> transaction hashes
    state: Arc<RwLock<State>>,
    total_size_bytes: usize,
}

impl Mempool {
    /// Create new mempool
    pub fn new(config: MempoolConfig, state: Arc<RwLock<State>>) -> Self {
        let mempool = Self {
            config,
            transactions: HashMap::new(),
            by_fee: BTreeMap::new(),
            by_account: HashMap::new(),
            state,
            total_size_bytes: 0,
        };

        // Start cleanup task
        let mempool_clone = mempool.clone();
        tokio::spawn(async move {
            let mut interval = time::interval(mempool_clone.config.cleanup_interval);
            loop {
                interval.tick().await;
                if let Err(e) = mempool_clone.cleanup_expired().await {
                    log::error!("Mempool cleanup error: {}", e);
                }
            }
        });

        mempool
    }

    /// Add transaction to mempool
    pub async fn add_transaction(&mut self, transaction: Transaction) -> Result<()> {
        let tx_hash = transaction.hash();

        // Check if already exists
        if self.transactions.contains_key(&tx_hash) {
            return Err(BlockchainError::InvalidTransaction(
                "Transaction already in mempool".to_string(),
            ));
        }

        // Validate transaction
        self.validate_transaction(&transaction).await?;

        // Check size limits
        let entry = MempoolEntry::new(transaction);
        if self.total_size_bytes + entry.size_bytes > self.config.max_size_bytes {
            return Err(BlockchainError::InvalidTransaction(
                "Mempool full".to_string(),
            ));
        }

        if self.transactions.len() >= self.config.max_size {
            // Remove lowest fee transaction
            self.remove_lowest_fee_transaction()?;
        }

        // Check account limits
        let account_key = entry.transaction.from.clone();
        let should_remove_oldest = {
            let account_txs = self
                .by_account
                .entry(account_key.clone())
                .or_insert(HashSet::new());
            if account_txs.len() >= self.config.max_replacements {
                // Find oldest transaction from this account
                account_txs
                    .iter()
                    .min_by_key(|hash| {
                        self.transactions
                            .get(hash)
                            .map(|e| e.received_at)
                            .unwrap_or(u64::MAX)
                    })
                    .cloned()
            } else {
                None
            }
        };

        if let Some(oldest_hash) = should_remove_oldest {
            self.remove_transaction(&oldest_hash)?;
        }

        // Add transaction
        self.transactions.insert(tx_hash, entry.clone());
        self.by_fee.insert((entry.fee_per_byte, tx_hash), tx_hash);
        self.by_account
            .entry(account_key)
            .or_insert(HashSet::new())
            .insert(tx_hash);
        self.total_size_bytes += entry.size_bytes;

        log::debug!("Added transaction {} to mempool", tx_hash.to_hex());
        Ok(())
    }

    /// Remove transaction from mempool
    pub fn remove_transaction(&mut self, tx_hash: &Hash) -> Result<()> {
        if let Some(entry) = self.transactions.remove(tx_hash) {
            self.by_fee.remove(&(entry.fee_per_byte, *tx_hash));
            self.total_size_bytes -= entry.size_bytes;

            // Remove from account tracking
            if let Some(account_txs) = self.by_account.get_mut(&entry.transaction.from) {
                account_txs.remove(tx_hash);
                if account_txs.is_empty() {
                    self.by_account.remove(&entry.transaction.from);
                }
            }

            // Remove dependencies
            for dep_hash in &entry.dependencies {
                if let Some(dep_entry) = self.transactions.get_mut(dep_hash) {
                    dep_entry.dependents.remove(tx_hash);
                }
            }

            // Remove dependents
            for dep_hash in &entry.dependents {
                if let Some(dep_entry) = self.transactions.get_mut(dep_hash) {
                    dep_entry.dependencies.remove(tx_hash);
                }
            }

            log::debug!("Removed transaction {} from mempool", tx_hash.to_hex());
        }

        Ok(())
    }

    /// Get transaction by hash
    pub fn get_transaction(&self, tx_hash: &Hash) -> Option<&Transaction> {
        self.transactions
            .get(tx_hash)
            .map(|entry| &entry.transaction)
    }

    /// Check if transaction exists in mempool
    pub fn contains_transaction(&self, tx_hash: &Hash) -> bool {
        self.transactions.contains_key(tx_hash)
    }

    /// Get highest fee transactions for block creation
    pub fn get_highest_fee_transactions(
        &self,
        max_count: usize,
        max_size: usize,
    ) -> Vec<Transaction> {
        let mut result = Vec::new();
        let mut current_size = 0;

        // Get transactions ordered by fee (highest first)
        for (_, tx_hash) in self.by_fee.iter().rev() {
            if let Some(entry) = self.transactions.get(tx_hash) {
                if result.len() >= max_count || current_size + entry.size_bytes > max_size {
                    break;
                }

                result.push(entry.transaction.clone());
                current_size += entry.size_bytes;
            }
        }

        result
    }

    /// Get transactions for a specific account
    pub fn get_account_transactions(&self, account: &Address) -> Vec<&Transaction> {
        self.by_account
            .get(account)
            .map(|tx_hashes| {
                tx_hashes
                    .iter()
                    .filter_map(|hash| self.transactions.get(hash))
                    .map(|entry| &entry.transaction)
                    .collect()
            })
            .unwrap_or_default()
    }

    /// Get mempool statistics
    pub fn get_stats(&self) -> MempoolStats {
        let mut fee_per_byte_values: Vec<u64> = self
            .transactions
            .values()
            .map(|entry| entry.fee_per_byte)
            .collect();

        let (min_fee, max_fee, avg_fee) = if fee_per_byte_values.is_empty() {
            (0, 0, 0)
        } else {
            fee_per_byte_values.sort();
            let min = fee_per_byte_values[0];
            let max = fee_per_byte_values[fee_per_byte_values.len() - 1];
            let sum: u64 = fee_per_byte_values.iter().sum();
            let avg = sum / fee_per_byte_values.len() as u64;
            (min, max, avg)
        };

        let pending_by_account = self
            .by_account
            .iter()
            .map(|(account, txs)| (account.clone(), txs.len()))
            .collect();

        MempoolStats {
            total_transactions: self.transactions.len(),
            total_size_bytes: self.total_size_bytes,
            average_fee_per_byte: avg_fee,
            min_fee_per_byte: min_fee,
            max_fee_per_byte: max_fee,
            pending_by_account,
        }
    }

    /// Clear all transactions
    pub fn clear(&mut self) {
        self.transactions.clear();
        self.by_fee.clear();
        self.by_account.clear();
        self.total_size_bytes = 0;
        log::info!("Mempool cleared");
    }

    /// Validate transaction before adding to mempool
    async fn validate_transaction(&self, transaction: &Transaction) -> Result<()> {
        // Basic structure validation
        crate::core::validate_transaction_structure(transaction)?;

        // Check minimum fee
        let tx_size = bincode::serialize(transaction).unwrap_or_default().len();
        let fee_per_byte = if tx_size > 0 {
            transaction.fee / tx_size as u64
        } else {
            0
        };

        if fee_per_byte < self.config.min_fee_per_byte {
            return Err(BlockchainError::InvalidTransaction(format!(
                "Fee too low: {} per byte, minimum: {}",
                fee_per_byte, self.config.min_fee_per_byte
            )));
        }

        // Check against current state
        let state = self.state.read().await;

        // Check nonce
        let expected_nonce = state
            .get_nonce(&transaction.from)
            .map_err(|_| BlockchainError::InvalidTransaction("Account not found".to_string()))?;

        if transaction.nonce < expected_nonce {
            return Err(BlockchainError::InvalidTransaction(
                "Nonce too low".to_string(),
            ));
        }

        // Check balance (including pending transactions)
        let mut total_pending = 0u64;
        if let Some(account_txs) = self.by_account.get(&transaction.from) {
            for tx_hash in account_txs {
                if let Some(entry) = self.transactions.get(tx_hash) {
                    if entry.transaction.nonce == transaction.nonce {
                        return Err(BlockchainError::InvalidTransaction(
                            "Nonce already used in mempool".to_string(),
                        ));
                    }
                    total_pending += entry.transaction.amount + entry.transaction.fee;
                }
            }
        }

        let balance = state
            .get_balance(&transaction.from)
            .map_err(|_| BlockchainError::InvalidTransaction("Account not found".to_string()))?;

        if balance < transaction.amount + transaction.fee + total_pending {
            return Err(BlockchainError::InvalidTransaction(
                "Insufficient balance".to_string(),
            ));
        }

        Ok(())
    }

    /// Remove lowest fee transaction when mempool is full
    fn remove_lowest_fee_transaction(&mut self) -> Result<()> {
        if let Some((_, tx_hash)) = self.by_fee.iter().next() {
            let hash_to_remove = *tx_hash; // Copy the hash
            self.remove_transaction(&hash_to_remove)?;
        }
        Ok(())
    }

    /// Clean up expired transactions
    async fn cleanup_expired(&self) -> Result<()> {
        let mut to_remove = Vec::new();

        for (tx_hash, entry) in &self.transactions {
            if entry.is_expired(self.config.max_transaction_age) {
                to_remove.push(*tx_hash);
            }
        }

        // Note: In a real implementation, we'd need mutable access here
        // For now, this is just a placeholder for the cleanup logic
        if !to_remove.is_empty() {
            log::debug!("Would remove {} expired transactions", to_remove.len());
        }

        Ok(())
    }

    /// Get all pending transactions
    pub fn get_all_transactions(&self) -> Vec<&Transaction> {
        self.transactions
            .values()
            .map(|entry| &entry.transaction)
            .collect()
    }

    /// Get transaction count
    pub fn size(&self) -> usize {
        self.transactions.len()
    }

    /// Check if mempool is empty
    pub fn is_empty(&self) -> bool {
        self.transactions.is_empty()
    }
}

impl Clone for Mempool {
    fn clone(&self) -> Self {
        // Note: This is a simplified clone that doesn't copy the cleanup task
        Self {
            config: self.config.clone(),
            transactions: self.transactions.clone(),
            by_fee: self.by_fee.clone(),
            by_account: self.by_account.clone(),
            state: Arc::clone(&self.state),
            total_size_bytes: self.total_size_bytes,
        }
    }
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
    use std::sync::Arc;
    use tokio::sync::RwLock;

    #[tokio::test]
    async fn test_mempool_creation() {
        let state = Arc::new(RwLock::new(State::new()));
        let config = MempoolConfig::default();
        let mempool = Mempool::new(config, state);

        assert_eq!(mempool.size(), 0);
        assert!(mempool.is_empty());
    }

    #[tokio::test]
    async fn test_mempool_stats() {
        let state = Arc::new(RwLock::new(State::new()));
        let config = MempoolConfig::default();
        let mempool = Mempool::new(config, state);

        let stats = mempool.get_stats();
        assert_eq!(stats.total_transactions, 0);
        assert_eq!(stats.total_size_bytes, 0);
    }

    #[tokio::test]
    async fn test_mempool_clear() {
        let state = Arc::new(RwLock::new(State::new()));
        let config = MempoolConfig::default();
        let mut mempool = Mempool::new(config, state);

        mempool.clear();
        assert_eq!(mempool.size(), 0);
    }
}
