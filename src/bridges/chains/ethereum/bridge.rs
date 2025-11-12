//! Complete Ethereum Bridge Implementation
//!
//! This module integrates the Ethereum smart contract, light client,
//! and adapter to provide a complete cross-chain bridge solution.

use crate::utils::error::{Result, BlockchainError};
use crate::bridges::light_clients::ethereum::{EthereumLightClient, EthereumHeader};
use crate::core::types::Address;
use serde::{Serialize, Deserialize};
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;

/// Complete Ethereum bridge implementation
pub struct EthereumBridge {
    /// Light client for header verification
    light_client: Arc<RwLock<EthereumLightClient>>,
    /// Bridge contract address on Ethereum
    bridge_contract: [u8; 20],
    /// Token wrapper contract address
    token_wrapper_contract: Option<[u8; 20]>,
    /// Supported ERC20 tokens
    supported_tokens: HashMap<[u8; 20], TokenInfo>,
    /// Bridge configuration
    config: BridgeConfig,
    /// Pending transfers waiting for confirmation
    pending_transfers: HashMap<[u8; 32], PendingTransfer>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BridgeConfig {
    pub min_confirmations: u64,
    pub max_lock_amount: u128,
    pub min_lock_amount: u128,
    pub fee_percentage: u8,
    pub validators: Vec<[u8; 20]>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TokenInfo {
    pub address: [u8; 20],
    pub symbol: String,
    pub decimals: u8,
    pub name: String,
    pub supported: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PendingTransfer {
    pub tx_hash: [u8; 32],
    pub from_address: [u8; 20],
    pub to_address: [u8; 32], // Erbium address
    pub token_address: [u8; 20],
    pub amount: u128,
    pub block_number: u64,
    pub confirmations: u64,
    pub timestamp: u64,
}

/// Bridge event types
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum BridgeEvent {
    TokensLocked {
        user: [u8; 20],
        token: [u8; 20],
        amount: u128,
        erbium_address: [u8; 32],
        nonce: u64,
        tx_hash: [u8; 32],
        block_number: u64,
    },
    TokensUnlocked {
        user: [u8; 20],
        token: [u8; 20],
        amount: u128,
        erbium_tx_hash: [u8; 32],
        tx_hash: [u8; 32],
        block_number: u64,
    },
}

impl EthereumBridge {
    /// Create a new Ethereum bridge
    pub fn new(
        genesis_hash: [u8; 32],
        bridge_contract: [u8; 20],
        config: BridgeConfig,
    ) -> Self {
        let light_client = Arc::new(RwLock::new(EthereumLightClient::new(genesis_hash)));

        Self {
            light_client,
            bridge_contract,
            token_wrapper_contract: None,
            supported_tokens: HashMap::new(),
            config,
            pending_transfers: HashMap::new(),
        }
    }

    /// Initialize the bridge with trusted validators
    pub async fn initialize(&mut self) -> Result<()> {
        let mut light_client = self.light_client.write().await;

        // Add trusted validators for PoA networks if needed
        for validator in &self.config.validators {
            light_client.add_trusted_signer(*validator);
        }

        log::info!("Ethereum bridge initialized with {} validators", self.config.validators.len());
        Ok(())
    }

    /// Add a supported ERC20 token
    pub fn add_supported_token(&mut self, token_info: TokenInfo) -> Result<()> {
        if !token_info.supported {
            return Err(BlockchainError::Bridge("Token not marked as supported".to_string()));
        }

        self.supported_tokens.insert(token_info.address, token_info.clone());
        log::info!("Added supported token: {} ({})", token_info.symbol, token_info.name);
        Ok(())
    }

    /// Submit a new Ethereum block header for verification
    pub async fn submit_block_header(&mut self, header: EthereumHeader) -> Result<()> {
        let mut light_client = self.light_client.write().await;
        light_client.submit_header(header)?;
        Ok(())
    }

    /// Process a bridge event from Ethereum
    pub async fn process_bridge_event(&mut self, event: BridgeEvent) -> Result<()> {
        match event {
            BridgeEvent::TokensLocked {
                user,
                token,
                amount,
                erbium_address,
                nonce,
                tx_hash,
                block_number,
            } => {
                self.process_tokens_locked(
                    user,
                    token,
                    amount,
                    erbium_address,
                    nonce,
                    tx_hash,
                    block_number,
                ).await
            }
            BridgeEvent::TokensUnlocked {
                user,
                token,
                amount,
                erbium_tx_hash,
                tx_hash,
                block_number,
            } => {
                self.process_tokens_unlocked(
                    user,
                    token,
                    amount,
                    erbium_tx_hash,
                    tx_hash,
                    block_number,
                ).await
            }
        }
    }

    /// Process tokens locked event
    async fn process_tokens_locked(
        &mut self,
        user: [u8; 20],
        token: [u8; 20],
        amount: u128,
        erbium_address: [u8; 32],
        _nonce: u64,
        tx_hash: [u8; 32],
        block_number: u64,
    ) -> Result<()> {
        // Validate token is supported
        if !self.supported_tokens.contains_key(&token) && token != [0u8; 20] {
            return Err(BlockchainError::Bridge(format!("Unsupported token: {:?}", token)));
        }

        // Validate amount
        if amount < self.config.min_lock_amount || amount > self.config.max_lock_amount {
            return Err(BlockchainError::Bridge("Invalid lock amount".to_string()));
        }

        // Verify the transaction using light client
        let light_client = self.light_client.read().await;
        if !light_client.verify_bridge_event(
            block_number,
            &self.bridge_contract,
            &Self::tokens_locked_event_signature(),
            &[],
        )? {
            return Err(BlockchainError::Bridge("Bridge event verification failed".to_string()));
        }

        // Create pending transfer
        let pending_transfer = PendingTransfer {
            tx_hash,
            from_address: user,
            to_address: erbium_address,
            token_address: token,
            amount,
            block_number,
            confirmations: 0,
            timestamp: std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_secs(),
        };

        self.pending_transfers.insert(tx_hash, pending_transfer);

        log::info!(
            "Processed tokens locked: {} {} from {:?} to {:?}",
            amount,
            if token == [0u8; 20] { "ETH" } else { "ERC20" },
            user,
            erbium_address
        );

        Ok(())
    }

    /// Process tokens unlocked event
    async fn process_tokens_unlocked(
        &mut self,
        user: [u8; 20],
        token: [u8; 20],
        amount: u128,
        erbium_tx_hash: [u8; 32],
        tx_hash: [u8; 32],
        block_number: u64,
    ) -> Result<()> {
        // Verify the transaction using light client
        let light_client = self.light_client.read().await;
        if !light_client.verify_bridge_event(
            block_number,
            &self.bridge_contract,
            &Self::tokens_unlocked_event_signature(),
            &[],
        )? {
            return Err(BlockchainError::Bridge("Bridge event verification failed".to_string()));
        }

        log::info!(
            "Processed tokens unlocked: {} {} to {:?} (Erbium tx: {:?})",
            amount,
            if token == [0u8; 20] { "ETH" } else { "ERC20" },
            user,
            erbium_tx_hash
        );

        Ok(())
    }

    /// Update confirmations for pending transfers
    pub async fn update_confirmations(&mut self, current_block: u64) -> Result<Vec<PendingTransfer>> {
        let mut confirmed_transfers = Vec::new();

        // Update confirmations for all pending transfers
        for (tx_hash, transfer) in &mut self.pending_transfers {
            let confirmations = current_block.saturating_sub(transfer.block_number);
            transfer.confirmations = confirmations;

            // If we have enough confirmations, mark as confirmed
            if confirmations >= self.config.min_confirmations {
                confirmed_transfers.push(transfer.clone());
            }
        }

        // Remove confirmed transfers
        for transfer in &confirmed_transfers {
            self.pending_transfers.remove(&transfer.tx_hash);
        }

        Ok(confirmed_transfers)
    }

    /// Get pending transfers
    pub fn get_pending_transfers(&self) -> Vec<&PendingTransfer> {
        self.pending_transfers.values().collect()
    }

    /// Get supported tokens
    pub fn get_supported_tokens(&self) -> Vec<&TokenInfo> {
        self.supported_tokens.values().collect()
    }

    /// Get bridge status
    pub async fn get_bridge_status(&self) -> BridgeStatus {
        let light_client = self.light_client.read().await;
        let status = light_client.get_status();

        BridgeStatus {
            light_client_head: status.head_block,
            pending_transfers: self.pending_transfers.len(),
            supported_tokens: self.supported_tokens.len(),
            bridge_contract: self.bridge_contract,
        }
    }

    /// Get event signature for TokensLocked event
    fn tokens_locked_event_signature() -> [u8; 32] {
        use sha3::{Digest, Keccak256};
        let signature = "TokensLocked(address,address,uint256,bytes32,uint256)";
        let mut hasher = Keccak256::new();
        hasher.update(signature.as_bytes());
        let result = hasher.finalize();
        let mut sig = [0u8; 32];
        sig.copy_from_slice(&result);
        sig
    }

    /// Get event signature for TokensUnlocked event
    fn tokens_unlocked_event_signature() -> [u8; 32] {
        use sha3::{Digest, Keccak256};
        let signature = "TokensUnlocked(address,address,uint256,bytes32)";
        let mut hasher = Keccak256::new();
        hasher.update(signature.as_bytes());
        let result = hasher.finalize();
        let mut sig = [0u8; 32];
        sig.copy_from_slice(&result);
        sig
    }
}

/// Bridge status information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BridgeStatus {
    pub light_client_head: u64,
    pub pending_transfers: usize,
    pub supported_tokens: usize,
    pub bridge_contract: [u8; 20],
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_ethereum_bridge_creation() {
        let genesis_hash = [0u8; 32];
        let bridge_contract = [1u8; 20];
        let config = BridgeConfig {
            min_confirmations: 12,
            max_lock_amount: 1000000,
            min_lock_amount: 1,
            fee_percentage: 1,
            validators: vec![[2u8; 20]],
        };

        let bridge = EthereumBridge::new(genesis_hash, bridge_contract, config);
        assert_eq!(bridge.bridge_contract, bridge_contract);
        assert_eq!(bridge.config.min_confirmations, 12);
    }

    #[test]
    fn test_add_supported_token() {
        let genesis_hash = [0u8; 32];
        let bridge_contract = [1u8; 20];
        let config = BridgeConfig {
            min_confirmations: 12,
            max_lock_amount: 1000000,
            min_lock_amount: 1,
            fee_percentage: 1,
            validators: vec![[2u8; 20]],
        };

        let mut bridge = EthereumBridge::new(genesis_hash, bridge_contract, config);

        let token_info = TokenInfo {
            address: [3u8; 20],
            symbol: "TEST".to_string(),
            decimals: 18,
            name: "Test Token".to_string(),
            supported: true,
        };

        bridge.add_supported_token(token_info.clone()).unwrap();
        assert_eq!(bridge.supported_tokens.len(), 1);
        assert_eq!(bridge.supported_tokens[&[3u8; 20]].symbol, "TEST");
    }

    #[tokio::test]
    async fn test_bridge_initialization() {
        let genesis_hash = [0u8; 32];
        let bridge_contract = [1u8; 20];
        let config = BridgeConfig {
            min_confirmations: 12,
            max_lock_amount: 1000000,
            min_lock_amount: 1,
            fee_percentage: 1,
            validators: vec![[2u8; 20], [3u8; 20]],
        };

        let mut bridge = EthereumBridge::new(genesis_hash, bridge_contract, config);
        bridge.initialize().await.unwrap();

        let status = bridge.get_bridge_status().await;
        assert_eq!(status.pending_transfers, 0);
        assert_eq!(status.supported_tokens, 0);
    }
}
