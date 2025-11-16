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

/// Inner mempool data structure
struct MempoolInner {
    config: MempoolConfig,
    transactions: HashMap<Hash, MempoolEntry>,
    by_fee: BTreeMap<(u64, Hash), Hash>, // (fee_per_byte, hash) -> hash for ordering
    by_account: HashMap<Address, HashSet<Hash>>, // Account -> transaction hashes
    total_size_bytes: usize,
}

/// Transaction pool for pending transactions
/// Uses Arc<RwLock<>> internally for safe concurrent access
pub struct Mempool {
    inner: Arc<RwLock<MempoolInner>>,
    state: Arc<RwLock<State>>,
}

impl Mempool {
    /// Create new mempool
    pub fn new(config: MempoolConfig, state: Arc<RwLock<State>>) -> Self {
        let cleanup_interval = config.cleanup_interval;
        let inner = Arc::new(RwLock::new(MempoolInner {
            config,
            transactions: HashMap::new(),
            by_fee: BTreeMap::new(),
            by_account: HashMap::new(),
            total_size_bytes: 0,
        }));

        let mempool = Self {
            inner: Arc::clone(&inner),
            state,
        };

        // Start cleanup task with proper Arc sharing (no Clone trait needed)
        let inner_clone = Arc::clone(&inner);
        let state_clone = Arc::clone(&mempool.state);
        tokio::spawn(async move {
            let mut interval = time::interval(cleanup_interval);
            loop {
                interval.tick().await;
                let mut inner_guard = inner_clone.write().await;
                if let Err(e) = Self::cleanup_expired_internal(&mut inner_guard, &state_clone).await {
                    log::error!("Mempool cleanup error: {}", e);
                }
            }
        });

        mempool
    }

    /// Add transaction to mempool
    pub async fn add_transaction(&self, transaction: Transaction) -> Result<()> {
        let tx_hash = transaction.hash();

        // Validate transaction
        self.validate_transaction(&transaction).await?;

        let mut inner = self.inner.write().await;
        
        // Check if already exists
        if inner.transactions.contains_key(&tx_hash) {
            return Err(BlockchainError::InvalidTransaction(
                "Transaction already in mempool".to_string(),
            ));
        }

        // Check size limits
        let entry = MempoolEntry::new(transaction);
        if inner.total_size_bytes + entry.size_bytes > inner.config.max_size_bytes {
            return Err(BlockchainError::InvalidTransaction(
                "Mempool full".to_string(),
            ));
        }

        if inner.transactions.len() >= inner.config.max_size {
            // Remove lowest fee transaction
            Self::remove_lowest_fee_transaction_internal(&mut inner)?;
        }

        // Check account limits
        let account_key = entry.transaction.from.clone();
        
        // First, check if we need to remove oldest transaction
        let should_remove_oldest = {
            if let Some(account_txs) = inner.by_account.get(&account_key) {
                if account_txs.len() >= inner.config.max_replacements {
                    // Find oldest transaction from this account
                    account_txs
                        .iter()
                        .min_by_key(|hash| {
                            inner.transactions
                                .get(hash)
                                .map(|e| e.received_at)
                                .unwrap_or(u64::MAX)
                        })
                        .cloned()
                } else {
                    None
                }
            } else {
                None
            }
        };

        if let Some(oldest_hash) = should_remove_oldest {
            Self::remove_transaction_internal(&mut inner, &oldest_hash)?;
        }

        // Add transaction
        inner.transactions.insert(tx_hash, entry.clone());
        inner.by_fee.insert((entry.fee_per_byte, tx_hash), tx_hash);
        inner.by_account
            .entry(account_key)
            .or_insert_with(HashSet::new)
            .insert(tx_hash);
        inner.total_size_bytes += entry.size_bytes;

        log::debug!("Added transaction {} to mempool", tx_hash.to_hex());
        Ok(())
    }

    /// Remove transaction from mempool
    pub async fn remove_transaction(&self, tx_hash: &Hash) -> Result<()> {
        let mut inner = self.inner.write().await;
        Self::remove_transaction_internal(&mut inner, tx_hash)
    }

    /// Get transaction by hash
    pub async fn get_transaction(&self, tx_hash: &Hash) -> Option<Transaction> {
        let inner = self.inner.read().await;
        inner.transactions
            .get(tx_hash)
            .map(|entry| entry.transaction.clone())
    }

    /// Check if transaction exists in mempool
    pub async fn contains_transaction(&self, tx_hash: &Hash) -> bool {
        let inner = self.inner.read().await;
        inner.transactions.contains_key(tx_hash)
    }

    /// Get highest fee transactions for block creation
    pub async fn get_highest_fee_transactions(
        &self,
        max_count: usize,
        max_size: usize,
    ) -> Vec<Transaction> {
        let inner = self.inner.read().await;
        let mut result = Vec::new();
        let mut current_size = 0;

        // Get transactions ordered by fee (highest first)
        for (_, tx_hash) in inner.by_fee.iter().rev() {
            if let Some(entry) = inner.transactions.get(tx_hash) {
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
    pub async fn get_account_transactions(&self, account: &Address) -> Vec<Transaction> {
        let inner = self.inner.read().await;
        inner.by_account
            .get(account)
            .map(|tx_hashes| {
                tx_hashes
                    .iter()
                    .filter_map(|hash| inner.transactions.get(hash))
                    .map(|entry| entry.transaction.clone())
                    .collect()
            })
            .unwrap_or_default()
    }

    /// Get mempool statistics
    pub async fn get_stats(&self) -> MempoolStats {
        let inner = self.inner.read().await;
        let mut fee_per_byte_values: Vec<u64> = inner
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

        let pending_by_account = inner
            .by_account
            .iter()
            .map(|(account, txs)| (account.clone(), txs.len()))
            .collect();

        MempoolStats {
            total_transactions: inner.transactions.len(),
            total_size_bytes: inner.total_size_bytes,
            average_fee_per_byte: avg_fee,
            min_fee_per_byte: min_fee,
            max_fee_per_byte: max_fee,
            pending_by_account,
        }
    }

    /// Clear all transactions
    pub async fn clear(&self) {
        let mut inner = self.inner.write().await;
        inner.transactions.clear();
        inner.by_fee.clear();
        inner.by_account.clear();
        inner.total_size_bytes = 0;
        log::info!("Mempool cleared");
    }

    /// Validate transaction before adding to mempool
    async fn validate_transaction(&self, transaction: &Transaction) -> Result<()> {
        // Basic structure validation
        crate::core::validate_transaction_structure(transaction)?;

        let inner = self.inner.read().await;

        // Check minimum fee
        let tx_size = bincode::serialize(transaction).unwrap_or_default().len();
        let fee_per_byte = if tx_size > 0 {
            transaction.fee / tx_size as u64
        } else {
            0
        };

        if fee_per_byte < inner.config.min_fee_per_byte {
            return Err(BlockchainError::InvalidTransaction(format!(
                "Fee too low: {} per byte, minimum: {}",
                fee_per_byte, inner.config.min_fee_per_byte
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
        if let Some(account_txs) = inner.by_account.get(&transaction.from) {
            for tx_hash in account_txs {
                if let Some(entry) = inner.transactions.get(tx_hash) {
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

    /// Get all pending transactions
    pub async fn get_all_transactions(&self) -> Vec<Transaction> {
        let inner = self.inner.read().await;
        inner.transactions
            .values()
            .map(|entry| entry.transaction.clone())
            .collect()
    }

    /// Get transaction count
    pub async fn size(&self) -> usize {
        let inner = self.inner.read().await;
        inner.transactions.len()
    }

    /// Check if mempool is empty
    pub async fn is_empty(&self) -> bool {
        let inner = self.inner.read().await;
        inner.transactions.is_empty()
    }
}

// Clone removed to prevent race conditions. Use Arc<RwLock<>> pattern instead.

impl Mempool {
    /// Internal method for removing transaction (requires lock already held)
    fn remove_transaction_internal(inner: &mut MempoolInner, tx_hash: &Hash) -> Result<()> {
        if let Some(entry) = inner.transactions.remove(tx_hash) {
            inner.by_fee.remove(&(entry.fee_per_byte, *tx_hash));
            inner.total_size_bytes -= entry.size_bytes;

            // Remove from account tracking
            if let Some(account_txs) = inner.by_account.get_mut(&entry.transaction.from) {
                account_txs.remove(tx_hash);
                if account_txs.is_empty() {
                    inner.by_account.remove(&entry.transaction.from);
                }
            }

            // Remove dependencies
            for dep_hash in &entry.dependencies {
                if let Some(dep_entry) = inner.transactions.get_mut(dep_hash) {
                    dep_entry.dependents.remove(tx_hash);
                }
            }

            // Remove dependents
            for dep_hash in &entry.dependents {
                if let Some(dep_entry) = inner.transactions.get_mut(dep_hash) {
                    dep_entry.dependencies.remove(tx_hash);
                }
            }

            log::debug!("Removed transaction {} from mempool", tx_hash.to_hex());
        }
        Ok(())
    }

    /// Internal method for removing lowest fee transaction
    fn remove_lowest_fee_transaction_internal(inner: &mut MempoolInner) -> Result<()> {
        if let Some((_, tx_hash)) = inner.by_fee.iter().next() {
            let tx_hash = *tx_hash;
            Self::remove_transaction_internal(inner, &tx_hash)?;
        }
        Ok(())
    }

    /// Internal cleanup method
    async fn cleanup_expired_internal(inner: &mut MempoolInner, _state: &Arc<RwLock<State>>) -> Result<()> {
        let mut to_remove = Vec::new();
        
        for (hash, entry) in &inner.transactions {
            if entry.is_expired(inner.config.max_transaction_age) {
                to_remove.push(*hash);
            }
        }

        for hash in to_remove {
            Self::remove_transaction_internal(inner, &hash)?;
            log::debug!("Removed expired transaction {} from mempool", hash.to_hex());
        }

        Ok(())
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

        assert_eq!(mempool.size().await, 0);
        assert!(mempool.is_empty().await);
    }

    #[tokio::test]
    async fn test_mempool_stats() {
        let state = Arc::new(RwLock::new(State::new()));
        let config = MempoolConfig::default();
        let mempool = Mempool::new(config, state);

        let stats = mempool.get_stats().await;
        assert_eq!(stats.total_transactions, 0);
        assert_eq!(stats.total_size_bytes, 0);
    }

    #[tokio::test]
    async fn test_mempool_clear() {
        let state = Arc::new(RwLock::new(State::new()));
        let config = MempoolConfig::default();
        let mempool = Mempool::new(config, state);

        mempool.clear().await;
        assert_eq!(mempool.size().await, 0);
    }
}
