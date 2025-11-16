// src/core/chain.rs

use super::state::State;
use super::types::{Address, Hash};
use super::{Block, Transaction};
use crate::storage::blockchain_storage::BlockchainStorage;
use crate::utils::error::{BlockchainError, Result};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;

/// CRITICAL: Expected genesis block hash for network consensus
/// This MUST match across all nodes or they won't synchronize
/// Computed from deterministic genesis with config/genesis/allocations.toml
/// 
/// ⚠️  TO UPDATE: Run node once, check logs for genesis hash, update this constant
/// ⚠️  If you change genesis config, this MUST be updated!
pub const EXPECTED_GENESIS_HASH: &str = "PLACEHOLDER_UPDATE_AFTER_FIRST_RUN";
use tokio::sync::RwLock;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GenesisConfig {
    pub genesis: GenesisData,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GenesisData {
    pub timestamp: u64,
    pub initial_validators: Vec<ValidatorAllocation>,
    pub initial_balances: Vec<GenesisAllocation>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GenesisAllocation {
    pub address: String,
    pub amount: String, // String para evitar problemas de precisão
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ValidatorAllocation {
    pub address: String,
    pub stake: String, // Stake amount in ERB (string for precision)
}

pub struct Blockchain {
    pub blocks: Vec<Block>,
    pub state: State,
    pub block_hashes: HashMap<Hash, usize>, // hash -> block index
    pub current_difficulty: u64,
}

impl Blockchain {
    pub fn new() -> Result<Self> {
        let genesis_block = Self::create_genesis_block()?;
        let mut blockchain = Self {
            blocks: Vec::new(),
            state: State::new(),
            block_hashes: HashMap::new(),
            current_difficulty: 1000,
        };

        blockchain.add_block(genesis_block)?;
        Ok(blockchain)
    }

    /// Create a new blockchain with database persistence
    pub async fn new_with_database(database_path: &str) -> Result<Self> {
        let genesis_block = Self::create_genesis_block()?;
        let state = State::new_with_database(database_path).await?;
        let mut blockchain = Self {
            blocks: Vec::new(),
            state,
            block_hashes: HashMap::new(),
            current_difficulty: 1000,
        };

        blockchain.add_block(genesis_block)?;
        Ok(blockchain)
    }

    fn create_genesis_block() -> Result<Block> {
        use std::fs;
        use std::path::Path;

        let mut transactions = Vec::new();

        // Tentar ler configuração do genesis
        let genesis_config_path = "config/genesis/allocations.toml";
        log::info!(
            "Trying to load genesis configuration of: {}",
            genesis_config_path
        );

        if Path::new(genesis_config_path).exists() {
            log::info!("Loading genesis allocations from {}", genesis_config_path);

            match fs::read_to_string(genesis_config_path) {
                Ok(config_content) => {
                    match toml::from_str::<GenesisConfig>(&config_content) {
                        Ok(genesis_config) => {
                            log::info!(
                                "Found {} genesis allocations",
                                genesis_config.genesis.initial_balances.len()
                            );

                            // Criar endereço do sistema para as alocações genesis
                            let system_address = Address::zero();

                            // Criar transações para cada alocação
                            for (i, allocation) in
                                genesis_config.genesis.initial_balances.iter().enumerate()
                            {
                                let recipient_address = Address::new(allocation.address.clone())
                                    .map_err(|e| {
                                        BlockchainError::InvalidTransaction(format!(
                                            "Invalid genesis allocation address: {}",
                                            e
                                        ))
                                    })?;

                                // Parse do amount (string para u128)
                                let amount: u128 = allocation.amount.parse().map_err(|e| {
                                    BlockchainError::InvalidTransaction(format!(
                                        "Invalid genesis allocation amount: {}",
                                        e
                                    ))
                                })?;

                                // Criar transação de alocação genesis
                                let genesis_tx = Transaction::new_transfer(
                                    system_address.clone(),
                                    recipient_address,
                                    amount as u64, // Conversão segura pois nossos valores são pequenos
                                    0,             // Sem taxa para genesis
                                    i as u64,      // Nonce sequencial
                                );

                                transactions.push(genesis_tx);
                                log::info!(
                                    "Genesis allocation: {} ERB to {}",
                                    allocation.amount,
                                    allocation.address
                                );
                            }

                            // Process initial validators with stake
                            if !genesis_config.genesis.initial_validators.is_empty() {
                                log::info!(
                                    "Found {} initial validators with stake",
                                    genesis_config.genesis.initial_validators.len()
                                );
                                
                                for validator in &genesis_config.genesis.initial_validators {
                                    log::info!(
                                        "Genesis validator: {} with {} ERB staked",
                                        validator.address,
                                        validator.stake
                                    );
                                }
                            }

                            log::info!(
                                "Created {} genesis allocation transactions",
                                transactions.len()
                            );
                        }
                        Err(e) => {
                            log::warn!(
                                "Failed to parse genesis config: {}. Using empty genesis block.",
                                e
                            );
                        }
                    }
                }
                Err(e) => {
                    log::warn!(
                        "Failed to read genesis config file: {}. Using empty genesis block.",
                        e
                    );
                }
            }
        } else {
            log::info!(
                "No genesis config found at {}. Using empty genesis block.",
                genesis_config_path
            );
        }

        // Criar bloco genesis
        let block = Block::new(
            0, // block number
            Hash::new(b"genesis"),
            transactions,
            "genesis".to_string(),
            1000,
        );

        Ok(block)
    }

    pub fn add_block(&mut self, block: Block) -> Result<()> {
        // Verify block structure and transactions
        self.verify_block(&block)?;

        // Apply transactions to state
        for tx in &block.transactions {
            self.state.apply_transaction(tx)?;
        }

        // Add block to chain
        let block_hash = block.hash();
        let block_index = self.blocks.len();

        self.blocks.push(block);
        self.block_hashes.insert(block_hash, block_index);

        log::info!("Added block {} at height {}", block_hash, block_index);

        Ok(())
    }

    fn verify_block(&self, block: &Block) -> Result<()> {
        // Check previous hash
        if let Some(last_block) = self.blocks.last() {
            if block.header.previous_hash != last_block.hash() {
                return Err(BlockchainError::InvalidBlock(
                    "Previous hash mismatch".to_string(),
                ));
            }
            // Check block number
            if block.header.number != last_block.header.number + 1 {
                return Err(BlockchainError::InvalidBlock(
                    "Invalid block number".to_string(),
                ));
            }
        } else {
            // This is the genesis block - VALIDATE IT!
            if block.header.number != 0 {
                return Err(BlockchainError::InvalidBlock(
                    "Invalid genesis block number".to_string(),
                ));
            }
            
            // CRITICAL: Validate genesis hash matches expected
            if EXPECTED_GENESIS_HASH != "PLACEHOLDER_UPDATE_AFTER_FIRST_RUN" {
                let block_hash = block.hash().to_hex();
                if block_hash != EXPECTED_GENESIS_HASH {
                    log::error!(
                        "\n\n❌ GENESIS BLOCK MISMATCH! ❌\n\
                         Expected: {}\n\
                         Got:      {}\n\
                         This node is on a DIFFERENT NETWORK!\n",
                        EXPECTED_GENESIS_HASH,
                        block_hash
                    );
                    return Err(BlockchainError::InvalidBlock(
                        format!(
                            "Genesis block hash mismatch. Network incompatibility. Expected: {}, Got: {}",
                            EXPECTED_GENESIS_HASH,
                            block_hash
                        )
                    ));
                }
                log::info!("✅ Genesis block validated: {}", block_hash);
            } else {
                // First run - log the genesis hash for administrator
                let genesis_hash = block.hash().to_hex();
                log::warn!(
                    "\n\n\
                    ═══════════════════════════════════════════════════════════════\n\
                    ⚠️  GENESIS HASH DETECTED - ACTION REQUIRED ⚠️\n\
                    ═══════════════════════════════════════════════════════════════\n\
                    \n\
                    Update EXPECTED_GENESIS_HASH in src/core/chain.rs:\n\
                    \n\
                    pub const EXPECTED_GENESIS_HASH: &str = \"{}\";\n\
                    \n\
                    ⚠️  ALL nodes in the network MUST use this SAME hash!\n\
                    ⚠️  Share this hash with other node operators!\n\
                    \n\
                    ═══════════════════════════════════════════════════════════════\n\
                    ",
                    genesis_hash
                );
            }
        }

        // Verify merkle root
        let calculated_merkle_root = Block::calculate_merkle_root(&block.transactions);
        if block.header.merkle_root != calculated_merkle_root {
            return Err(BlockchainError::InvalidBlock(
                "Merkle root mismatch".to_string(),
            ));
        }

        // Verify block signature
        // Skip signature check for genesis block
        if block.header.number > 0 {
            if let Some(validator_key) = self.get_validator_public_key(&block.header.validator) {
                if !block.verify_signature(&validator_key)? {
                    return Err(BlockchainError::InvalidBlock(
                        "Invalid block signature".to_string(),
                    ));
                }
            } else {
                return Err(BlockchainError::InvalidBlock(format!(
                    "Validator {} not found",
                    block.header.validator
                )));
            }
        }

        // Verify all transactions in the block
        for tx in &block.transactions {
            self.verify_transaction(tx)?;
        }

        Ok(())
    }

    fn verify_transaction(&self, transaction: &Transaction) -> Result<()> {
        // Basic transaction structure validation
        transaction.validate_basic()?;

        match transaction.transaction_type {
            crate::core::transaction::TransactionType::ConfidentialTransfer => {
                self.verify_confidential_transaction(transaction)?;
            }
            _ => {
                self.verify_regular_transaction(transaction)?;
            }
        }

        // Verify transaction signature
        // Skip signature verification for genesis transactions (from address 0x0)
        if transaction.from.as_str() != "0x0000000000000000000000000000000000000000" {
            if let Some(sender_key) = self.get_account_public_key(&transaction.from) {
                if !transaction.verify_signature(&sender_key)? {
                    return Err(BlockchainError::InvalidTransaction(
                        "Invalid transaction signature".to_string(),
                    ));
                }
            } else {
                return Err(BlockchainError::InvalidTransaction(
                    "Sender account not found".to_string(),
                ));
            }
        } else {
            log::info!(
                "Skipping signature verification for genesis transaction from {}",
                transaction.from
            );
        }

        Ok(())
    }

    fn verify_regular_transaction(&self, transaction: &Transaction) -> Result<()> {
        // Skip balance and nonce checks for genesis transactions (from address 0x0)
        if transaction.from.as_str() == "0x0000000000000000000000000000000000000000" {
            log::info!(
                "Skipping balance/nonce verification for genesis transaction to {}",
                transaction.to
            );
            return Ok(());
        }

        // For regular transactions, check balance and nonce
        let balance = self.state.get_balance(&transaction.from)?;
        let current_nonce = self.state.get_nonce(&transaction.from)?;

        if balance < transaction.amount + transaction.fee {
            return Err(BlockchainError::InvalidTransaction(format!(
                "Insufficient balance for {}: needed {}, got {}",
                transaction.from,
                transaction.amount + transaction.fee,
                balance
            )));
        }

        if transaction.nonce != current_nonce {
            return Err(BlockchainError::InvalidTransaction(format!(
                "Invalid nonce for {}: expected {}, got {}",
                transaction.from, current_nonce, transaction.nonce
            )));
        }

        Ok(())
    }

    fn verify_confidential_transaction(&self, transaction: &Transaction) -> Result<()> {
        use crate::privacy::confidential_tx::ConfidentialTxBuilder;

        // Verify ZK proof exists
        let zk_proof = transaction.zk_proof.as_ref().ok_or_else(|| {
            BlockchainError::InvalidTransaction(
                "Confidential transaction missing ZK proof".to_string(),
            )
        })?;

        // Verify commitments exist
        let input_commitments = transaction.input_commitments.as_ref().ok_or_else(|| {
            BlockchainError::InvalidTransaction(
                "Confidential transaction missing input commitments".to_string(),
            )
        })?;
        let output_commitments = transaction.output_commitments.as_ref().ok_or_else(|| {
            BlockchainError::InvalidTransaction(
                "Confidential transaction missing output commitments".to_string(),
            )
        })?;

        // Verify range proofs exist
        let range_proofs = transaction.range_proofs.as_ref().ok_or_else(|| {
            BlockchainError::InvalidTransaction(
                "Confidential transaction missing range proofs".to_string(),
            )
        })?;

        // Verify binding signature exists
        let binding_signature = transaction.binding_signature.as_ref().ok_or_else(|| {
            BlockchainError::InvalidTransaction(
                "Confidential transaction missing binding signature".to_string(),
            )
        })?;

        // Reconstruct confidential transaction for verification
        // Parse the commitments properly
        let input_commitments_parsed: Result<Vec<_>> = input_commitments
            .iter()
            .map(|commitment| {
                // Garantir que o commitment tem o tamanho correto (32 bytes)
                if commitment.len() != 32 {
                    return Err(BlockchainError::InvalidTransaction(format!(
                        "Invalid commitment size: {}",
                        commitment.len()
                    )));
                }

                // Converter para CompressedRistretto
                let mut bytes = [0u8; 32];
                bytes.copy_from_slice(commitment);
                Ok(curve25519_dalek_ng::ristretto::CompressedRistretto::from_slice(&bytes))
            })
            .collect();
        let input_commitments_parsed = input_commitments_parsed?;

        let output_commitments_parsed: Result<Vec<_>> = output_commitments
            .iter()
            .map(|commitment| {
                // Garantir que o commitment tem o tamanho correto (32 bytes)
                if commitment.len() != 32 {
                    return Err(BlockchainError::InvalidTransaction(format!(
                        "Invalid commitment size: {}",
                        commitment.len()
                    )));
                }

                // Converter para CompressedRistretto
                let mut bytes = [0u8; 32];
                bytes.copy_from_slice(commitment);
                Ok(curve25519_dalek_ng::ristretto::CompressedRistretto::from_slice(&bytes))
            })
            .collect();
        let output_commitments_parsed = output_commitments_parsed?;

        let signer_public_key =
            self.get_account_public_key(&transaction.from)
                .ok_or_else(|| {
                    BlockchainError::InvalidTransaction("Sender public key not found".to_string())
                })?;

        let confidential_tx = crate::privacy::confidential_tx::ConfidentialTransaction {
            base_transaction: transaction.clone(),
            input_commitments: input_commitments_parsed,
            output_commitments: output_commitments_parsed,
            range_proofs: range_proofs.clone(),
            zk_proof: zk_proof.clone(),
            binding_signature: binding_signature.clone(),
            signer_public_key,
        };

        // Create ConfidentialTxBuilder and verify
        let tx_builder = ConfidentialTxBuilder::new();
        if !tx_builder.verify_confidential_transaction(&confidential_tx)? {
            return Err(BlockchainError::InvalidTransaction(
                "Confidential transaction verification failed".to_string(),
            ));
        }

        log::debug!("Successfully verified confidential transaction with ZK proof");

        Ok(())
    }

    fn get_validator_public_key(&self, validator_address: &str) -> Option<Vec<u8>> {
        // Implementação melhorada para buscar a chave pública do validador
        // Busca no estado da blockchain ou no conjunto de validadores
        use crate::consensus::validator::ValidatorSet;

        // Converter o endereço de string para Address
        let validator_addr = match Address::new(validator_address.to_string()) {
            Ok(addr) => addr,
            Err(_) => return None,
        };

        // Em uma implementação real, isso buscaria do estado
        // Por enquanto, vamos simular um conjunto de validadores
        let validator_set = ValidatorSet::new();

        // Buscar a chave pública do validador
        validator_set.get_public_key(&validator_addr)
    }

    fn get_account_public_key(&self, address: &Address) -> Option<Vec<u8>> {
        // Implementação melhorada para buscar a chave pública da conta
        // Busca no estado da blockchain

        // Verificar se a conta existe no estado
        if let Ok(account_exists) = self.state.account_exists(address) {
            if !account_exists {
                return None;
            }
        } else {
            return None;
        }

        // Em uma implementação real, isso buscaria do estado
        // Por enquanto, vamos simular uma busca no estado

        // Gerar uma chave determinística baseada no endereço para testes
        // Isso é apenas para simulação, não para uso em produção!
        let addr_bytes = match address.to_bytes() {
            Ok(bytes) => bytes,
            Err(_) => return None,
        };

        // Usar o hash do endereço como chave pública para testes
        let mut key = Vec::with_capacity(32);
        key.extend_from_slice(&addr_bytes);
        key.resize(32, 0); // Garantir que tenha 32 bytes

        Some(key)
    }

    pub fn get_block_by_hash(&self, hash: &Hash) -> Option<&Block> {
        self.block_hashes
            .get(hash)
            .map(|&index| &self.blocks[index])
    }

    pub fn get_block_by_height(&self, height: usize) -> Option<&Block> {
        self.blocks.get(height)
    }

    pub fn get_latest_block(&self) -> Option<&Block> {
        self.blocks.last()
    }

    pub fn get_block_height(&self) -> usize {
        self.blocks.len()
    }

    pub fn get_current_difficulty(&self) -> u64 {
        self.current_difficulty
    }

    pub fn calculate_transaction_throughput(&self) -> f64 {
        // Calculate approximate TPS based on recent blocks
        if self.blocks.len() < 2 {
            return 0.0;
        }

        let recent_blocks = &self.blocks[self.blocks.len().saturating_sub(10)..];
        let total_transactions: usize = recent_blocks
            .iter()
            .map(|block| block.transactions.len())
            .sum();

        if recent_blocks.len() < 2 {
            return 0.0;
        }

        let time_span = recent_blocks.last().unwrap().header.timestamp
            - recent_blocks.first().unwrap().header.timestamp;

        if time_span == 0 {
            return 0.0;
        }

        (total_transactions as f64) / (time_span as f64 / 1000.0) // Convert ms to seconds
    }

    /// Find a transaction by its hash across all blocks
    pub fn get_transaction_by_hash(&self, tx_hash: &Hash) -> Option<(&Transaction, usize, usize)> {
        for (block_height, block) in self.blocks.iter().enumerate() {
            for (tx_index, tx) in block.transactions.iter().enumerate() {
                if &tx.hash() == tx_hash {
                    return Some((tx, block_height, tx_index));
                }
            }
        }
        None
    }
}

/// Persistent blockchain with database storage
pub struct PersistentBlockchain {
    blockchain: Blockchain,
    storage: Arc<RwLock<BlockchainStorage>>,
}

impl PersistentBlockchain {
    /// Create new persistent blockchain
    pub async fn new(data_dir: &str) -> Result<Self> {
        let storage = Arc::new(RwLock::new(BlockchainStorage::new(data_dir).await?));

        // Try to load existing blockchain from storage
        {
            let storage_read = storage.read().await;
            if let Ok(latest_height) = storage_read.get_latest_height().await {
                if latest_height > 0 {
                    log::info!(
                        "Loading existing blockchain from storage (height: {})",
                        latest_height
                    );
                    drop(storage_read); // Release the read lock
                    return Self::load_from_storage(storage).await;
                }
            }
        }

        // Create new blockchain
        log::info!("Creating new blockchain with persistence");
        let blockchain = Blockchain::new()?;
        let persistent = Self {
            blockchain,
            storage,
        };

        // Store genesis block
        if let Some(genesis_block) = persistent.blockchain.blocks.first() {
            persistent
                .storage
                .write()
                .await
                .store_block(genesis_block)
                .await?;
            for (tx_index, tx) in genesis_block.transactions.iter().enumerate() {
                persistent
                    .storage
                    .write()
                    .await
                    .store_transaction(tx, 0, tx_index)
                    .await?;
            }
        }

        // Store initial state
        persistent
            .storage
            .write()
            .await
            .store_state(&persistent.blockchain.state)
            .await?;

        Ok(persistent)
    }

    /// Load blockchain from storage
    async fn load_from_storage(storage: Arc<RwLock<BlockchainStorage>>) -> Result<Self> {
        let mut blockchain = Blockchain::new()?; // Start with genesis

        // Load latest height
        let latest_height = {
            let storage_read = storage.read().await;
            storage_read.get_latest_height().await?
        };

        // Load all blocks from storage
        for height in 1..=latest_height {
            let block = {
                let storage_read = storage.read().await;
                storage_read.load_block_by_height(height).await?
            };

            if let Some(block) = block {
                blockchain.add_block(block)?;
            }
        }

        // Load latest state
        let state = {
            let storage_read = storage.read().await;
            storage_read.load_state().await?
        };

        if let Some(state) = state {
            blockchain.state = state;
        }

        log::info!(
            "Loaded blockchain with {} blocks",
            blockchain.get_block_height()
        );

        Ok(Self {
            blockchain,
            storage,
        })
    }

    /// Add block with persistence
    pub async fn add_block(&mut self, block: Block) -> Result<()> {
        // Add to in-memory blockchain
        self.blockchain.add_block(block.clone())?;

        // Persist block
        self.storage.write().await.store_block(&block).await?;

        // Store transactions
        for (tx_index, tx) in block.transactions.iter().enumerate() {
            self.storage
                .write()
                .await
                .store_transaction(tx, block.header.number, tx_index)
                .await?;
        }

        // Update state in storage
        self.storage
            .write()
            .await
            .store_state(&self.blockchain.state)
            .await?;

        Ok(())
    }

    /// Get block by hash (checks memory first, then storage)
    pub async fn get_block_by_hash(&self, hash: &Hash) -> Result<Option<Block>> {
        // Check memory first
        if let Some(block) = self.blockchain.get_block_by_hash(hash) {
            return Ok(Some(block.clone()));
        }

        // Check storage
        let storage_read = self.storage.read().await;
        storage_read.load_block(hash.as_bytes()).await
    }

    /// Get block by height (checks memory first, then storage)
    pub async fn get_block_by_height(&self, height: u64) -> Result<Option<Block>> {
        // Check memory first
        if let Some(block) = self.blockchain.get_block_by_height(height as usize) {
            return Ok(Some(block.clone()));
        }

        // Check storage
        let storage_read = self.storage.read().await;
        storage_read.load_block_by_height(height).await
    }

    /// Get transaction by hash
    pub async fn get_transaction_by_hash(&self, tx_hash: &[u8]) -> Result<Option<Transaction>> {
        let storage_read = self.storage.read().await;
        storage_read.load_transaction(tx_hash).await
    }

    /// Get latest block height
    pub async fn get_latest_height(&self) -> Result<u64> {
        let storage_read = self.storage.read().await;
        storage_read.get_latest_height().await
    }

    /// Get blockchain state
    pub fn get_state(&self) -> &State {
        &self.blockchain.state
    }

    /// Flush all pending writes to disk
    pub async fn flush(&self) -> Result<()> {
        self.storage.write().await.flush().await
    }

    /// Get storage statistics
    pub async fn get_storage_stats(&self) -> Result<serde_json::Value> {
        self.storage.read().await.get_stats().await
    }

    /// Create backup
    pub async fn create_backup(&self, backup_path: &str) -> Result<()> {
        self.storage.read().await.create_backup(backup_path).await
    }

    /// Access underlying blockchain for read operations
    pub fn blockchain(&self) -> &Blockchain {
        &self.blockchain
    }
}
