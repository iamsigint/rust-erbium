use super::types::{Address, Hash};
use super::transaction::Transaction;
use crate::utils::error::{Result, BlockchainError};
use crate::storage::cache::{SharedCache, create_shared_cache_with_memory_limit};
use crate::storage::database::Database;
use std::collections::HashMap;
use std::time::{SystemTime, UNIX_EPOCH};
use std::sync::Arc;
use tokio::sync::RwLock;

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct Account {
    pub balance: u64,
    pub nonce: u64,
    pub code_hash: Option<Hash>, // For smart contracts
    pub storage_root: Hash, // Merkle root of storage
    pub last_accessed: u64, // For cache optimization
}

#[derive(Clone, serde::Serialize, serde::Deserialize)]
pub struct State {
    accounts: HashMap<Address, Account>,
    total_supply: u64,
    // Note: database and cache fields are not serialized
    #[serde(skip)]
    database: Option<Arc<RwLock<Database>>>,
    // Performance optimizations
    #[serde(skip)]
    account_cache: SharedCache, // Cache for frequently accessed accounts
    #[serde(skip)]
    hot_accounts: HashMap<Address, u64>, // Track access frequency
    cache_hits: u64,
    cache_misses: u64,
}

impl State {
    pub fn new() -> Self {
        let mut s = Self {
            accounts: HashMap::new(),
            total_supply: 1_000_000_000 * 100_000_000, // 1B ERB with 8 decimals
            database: None,
            account_cache: create_shared_cache_with_memory_limit(10_000, 50), // 10k entries, 50MB
            hot_accounts: HashMap::new(),
            cache_hits: 0,
            cache_misses: 0,
        };
        // Faucet for dev: pre-fund sender used in examples
        let faucet_addr = super::types::Address::new_unchecked("0x0000000000000000000000000000000000000001".to_string());
        s.accounts.insert(faucet_addr.clone(), Account {
            balance: 10_000 * 100_000_000, // 10,000 ERB
            nonce: 0,
            code_hash: None,
            storage_root: Hash::new(b"empty"),
            last_accessed: current_timestamp(),
        });
        s
    }

    /// Create a new state with database persistence
    pub async fn new_with_database(database_path: &str) -> Result<Self> {
        let database = Database::new(database_path)?;
        let database = Arc::new(RwLock::new(database));

        let mut s = Self {
            accounts: HashMap::new(),
            total_supply: 1_000_000_000 * 100_000_000, // 1B ERB with 8 decimals
            database: Some(database),
            account_cache: create_shared_cache_with_memory_limit(10_000, 50), // 10k entries, 50MB
            hot_accounts: HashMap::new(),
            cache_hits: 0,
            cache_misses: 0,
        };

        // Load existing accounts from database
        s.load_accounts_from_database().await?;

        // Faucet for dev: pre-fund sender used in examples (only if not exists)
        let faucet_addr = super::types::Address::new_unchecked("0x0000000000000000000000000000000000000001".to_string());
        if !s.accounts.contains_key(&faucet_addr) {
            s.accounts.insert(faucet_addr.clone(), Account {
                balance: 10_000 * 100_000_000, // 10,000 ERB
                nonce: 0,
                code_hash: None,
                storage_root: Hash::new(b"empty"),
                last_accessed: current_timestamp(),
            });
            s.save_account_to_database(&faucet_addr).await?;
        }

        Ok(s)
    }

    /// Set database for persistence
    pub fn set_database(&mut self, database: Arc<RwLock<Database>>) {
        self.database = Some(database);
    }

    /// Load all accounts from database
    async fn load_accounts_from_database(&mut self) -> Result<()> {
        if let Some(database) = &self.database {
            let mut db = database.write().await;

            // Load total supply
            if let Ok(Some(total_supply_bytes)) = db.get(b"total_supply") {
                if total_supply_bytes.len() == 8 {
                    self.total_supply = u64::from_be_bytes(total_supply_bytes.try_into().unwrap());
                }
            }

            // Try to load all accounts with prefix "account:"
            // If iteration is not supported (unordered columns), skip loading existing accounts
            match db.iterate_with_prefix(b"account:") {
                Ok(account_entries) => {
                    for (key, value) in account_entries {
                        if key.starts_with(b"account:") && value.len() >= 8 {
                            // Extract address from key (skip "account:" prefix)
                            let address_hex = &key[8..];
                            if let Ok(address_str) = std::str::from_utf8(address_hex) {
                                if let Ok(address) = Address::new(address_str.to_string()) {
                                    // Deserialize account data
                                    if value.len() >= 8 {
                                        let balance = u64::from_be_bytes(value[0..8].try_into().unwrap());
                                        let nonce = if value.len() >= 16 {
                                            u64::from_be_bytes(value[8..16].try_into().unwrap())
                                        } else { 0 };

                                        let account = Account {
                                            balance,
                                            nonce,
                                            code_hash: None, // TODO: implement code hash storage
                                            storage_root: Hash::new(b"empty"), // TODO: implement storage root
                                            last_accessed: current_timestamp(),
                                        };

                                        self.accounts.insert(address, account);
                                    }
                                }
                            }
                        }
                    }
                    log::info!("Loaded {} accounts from database", self.accounts.len());
                }
                Err(e) => {
                    log::warn!("Could not load existing accounts from database (iteration not supported): {}. Starting with fresh state.", e);
                    // Continue without loading existing accounts
                    // The database will still work for new operations
                }
            }
        }

        Ok(())
    }

    /// Save account to database
    async fn save_account_to_database(&self, address: &Address) -> Result<()> {
        if let Some(database) = &self.database {
            if let Some(account) = self.accounts.get(address) {
                let mut db = database.write().await;

                // Save account data
                let key = format!("account:{}", address.as_str());
                let mut value = Vec::new();
                value.extend_from_slice(&account.balance.to_be_bytes());
                value.extend_from_slice(&account.nonce.to_be_bytes());

                db.put(key.as_bytes(), &value)?;

                // Save total supply
                db.put(b"total_supply", &self.total_supply.to_be_bytes())?;
            }
        }

        Ok(())
    }

    /// Flush all accounts to database
    pub async fn flush_to_database(&self) -> Result<()> {
        if let Some(database) = &self.database {
            let mut db = database.write().await;

            // Save total supply
            db.put(b"total_supply", &self.total_supply.to_be_bytes())?;

            // Save all accounts
            for (address, account) in &self.accounts {
                let key = format!("account:{}", address.as_str());
                let mut value = Vec::new();
                value.extend_from_slice(&account.balance.to_be_bytes());
                value.extend_from_slice(&account.nonce.to_be_bytes());

                db.put(key.as_bytes(), &value)?;
            }

            // Flush database
            db.flush()?;

            log::debug!("Flushed {} accounts to database", self.accounts.len());
        }

        Ok(())
    }
    
    pub fn apply_transaction(&mut self, transaction: &Transaction) -> Result<()> {
        match transaction.transaction_type {
            crate::core::transaction::TransactionType::ConfidentialTransfer => {
                self.apply_confidential_transaction(transaction)
            }
            crate::core::transaction::TransactionType::Stake => {
                self.apply_stake_transaction(transaction)
            }
            crate::core::transaction::TransactionType::Unstake => {
                self.apply_unstake_transaction(transaction)
            }
            _ => {
                self.apply_regular_transaction(transaction)
            }
        }
    }

    fn apply_regular_transaction(&mut self, transaction: &Transaction) -> Result<()> {
        // Special handling for genesis transactions (from address 0x0)
        if transaction.from.as_str() == "0x0000000000000000000000000000000000000000" {
            // Genesis transactions create tokens from nothing
            log::info!("Applying genesis transaction: creating {} tokens for {}", transaction.amount, transaction.to);

            // Update recipient account (create if doesn't exist)
            let recipient_account = self.get_account_mut(&transaction.to)?;
            recipient_account.balance += transaction.amount;

            // Increase total supply for genesis allocations
            self.total_supply += transaction.amount;

            // No fee burning for genesis transactions
            return Ok(());
        }

        let sender_account = self.get_account_mut(&transaction.from)?;

        // Check nonce
        if transaction.nonce != sender_account.nonce {
            return Err(BlockchainError::InvalidTransaction("Invalid nonce".to_string()));
        }

        // Check balance
        if sender_account.balance < transaction.amount + transaction.fee {
            return Err(BlockchainError::InvalidTransaction("Insufficient balance".to_string()));
        }

        // Update sender account
        sender_account.balance -= transaction.amount + transaction.fee;
        sender_account.nonce += 1;

        // Update recipient account
        let recipient_account = self.get_account_mut(&transaction.to)?;
        recipient_account.balance += transaction.amount;

        // Burn transaction fee (deflationary mechanism)
        self.total_supply -= transaction.fee;

        Ok(())
    }

    fn apply_confidential_transaction(&mut self, transaction: &Transaction) -> Result<()> {
        // For confidential transactions, we only update nonce and burn fee
        // The actual balance changes are hidden via commitments
        let sender_account = self.get_account_mut(&transaction.from)?;

        // Check nonce
        if transaction.nonce != sender_account.nonce {
            return Err(BlockchainError::InvalidTransaction("Invalid nonce".to_string()));
        }

        // For confidential transfers, the public amount should be 0
        // All value transfer is done via commitments
        if transaction.amount != 0 {
            return Err(BlockchainError::InvalidTransaction(
                "Confidential transaction should have zero public amount".to_string()
            ));
        }

        // Check that sender has enough balance for the fee
        if sender_account.balance < transaction.fee {
            return Err(BlockchainError::InvalidTransaction("Insufficient balance for fee".to_string()));
        }

        // Update sender nonce and deduct fee
        sender_account.nonce += 1;
        sender_account.balance -= transaction.fee;

        // Burn transaction fee
        self.total_supply -= transaction.fee;

        log::debug!("Applied confidential transaction: nonce={}, fee={}", transaction.nonce, transaction.fee);

        Ok(())
    }

    fn apply_stake_transaction(&mut self, transaction: &Transaction) -> Result<()> {
        let sender_account = self.get_account_mut(&transaction.from)?;

        // Check nonce
        if transaction.nonce != sender_account.nonce {
            return Err(BlockchainError::InvalidTransaction("Invalid nonce".to_string()));
        }

        // Check balance
        if sender_account.balance < transaction.amount + transaction.fee {
            return Err(BlockchainError::InvalidTransaction("Insufficient balance".to_string()));
        }

        // Update sender account
        sender_account.balance -= transaction.amount + transaction.fee;
        sender_account.nonce += 1;

        // Burn transaction fee
        self.total_supply -= transaction.fee;

        // Note: Actual staking logic is handled by consensus layer
        // In a full implementation, we would call consensus.add_stake_from_transaction()
        log::info!("Applied stake transaction: {} staked {}", transaction.from.as_str(), transaction.amount);

        Ok(())
    }

    fn apply_unstake_transaction(&mut self, transaction: &Transaction) -> Result<()> {
        let sender_account = self.get_account_mut(&transaction.from)?;

        // Check nonce
        if transaction.nonce != sender_account.nonce {
            return Err(BlockchainError::InvalidTransaction("Invalid nonce".to_string()));
        }

        // Check balance (unstaking fee)
        if sender_account.balance < transaction.fee {
            return Err(BlockchainError::InvalidTransaction("Insufficient balance for unstaking fee".to_string()));
        }

        // Update sender account
        sender_account.balance -= transaction.fee;
        sender_account.nonce += 1;

        // Burn transaction fee
        self.total_supply -= transaction.fee;

        // Note: Actual unstaking logic is handled by consensus layer
        log::info!("Applied unstake transaction: {} unstaked {}", transaction.from.as_str(), transaction.amount);

        Ok(())
    }
    
    pub fn get_balance(&self, address: &Address) -> Result<u64> {
        let account = self.accounts.get(address)
            .ok_or_else(|| BlockchainError::InvalidTransaction("Account not found".to_string()))?;
        Ok(account.balance)
    }
    
    pub fn get_nonce(&self, address: &Address) -> Result<u64> {
        let account = self.accounts.get(address)
            .ok_or_else(|| BlockchainError::InvalidTransaction("Account not found".to_string()))?;
        Ok(account.nonce)
    }
    
    fn get_account_mut(&mut self, address: &Address) -> Result<&mut Account> {
        if !self.accounts.contains_key(address) {
            self.accounts.insert(address.clone(), Account {
                balance: 0,
                nonce: 0,
                code_hash: None,
                storage_root: Hash::new(b"empty"),
                last_accessed: current_timestamp(),
            });
        }

        self.accounts.get_mut(address)
            .ok_or_else(|| BlockchainError::InvalidTransaction("Account not found".to_string()))
    }
    
    pub fn get_total_supply(&self) -> u64 {
        self.total_supply
    }
    
    /// Verifica se uma conta existe no estado
    pub fn account_exists(&self, address: &Address) -> Result<bool> {
        Ok(self.accounts.contains_key(address))
    }

    /// Get cache performance statistics
    pub fn get_cache_stats(&self) -> Result<CachePerformanceStats> {
        let cache_stats = self.account_cache.read()
            .map_err(|_| BlockchainError::Storage("Cache lock error".to_string()))?
            .get_stats();

        Ok(CachePerformanceStats {
            cache_stats,
            total_accounts: self.accounts.len(),
            cache_hits: self.cache_hits,
            cache_misses: self.cache_misses,
        })
    }

    /// Optimize cache by cleaning up old entries and updating hot accounts
    pub fn optimize_cache(&mut self) {
        // Clean up old cache entries (older than 1 hour)
        if let Ok(mut cache) = self.account_cache.write() {
            let cleaned = cache.cleanup_old_entries(3600); // 1 hour
            if cleaned > 0 {
                log::debug!("Cleaned {} old cache entries", cleaned);
            }
        }

        // Update hot accounts tracking
        self.update_hot_accounts();
    }

    fn update_hot_accounts(&mut self) {
        // Track accounts accessed in the last 5 minutes
        let recent_threshold = current_timestamp() - 300;

        self.hot_accounts.clear();
        for (address, account) in &self.accounts {
            if account.last_accessed > recent_threshold {
                *self.hot_accounts.entry(address.clone()).or_insert(0) += 1;
            }
        }
    }
}

fn current_timestamp() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_secs()
}

#[derive(Debug, Clone)]
pub struct CachePerformanceStats {
    pub cache_stats: crate::storage::cache::CacheStats,
    pub total_accounts: usize,
    pub cache_hits: u64,
    pub cache_misses: u64,
}
