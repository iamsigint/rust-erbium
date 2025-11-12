//! Bridge manager for cross-chain operations

use crate::utils::error::{Result, BlockchainError};
use serde::{Serialize, Deserialize};

/// Bridge manager for coordinating cross-chain operations
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BridgeManager {
    pub supported_chains: Vec<String>,
    pub active_bridges: Vec<String>,
}

impl BridgeManager {
    /// Create a new bridge manager
    pub fn new() -> Self {
        Self {
            supported_chains: vec!["ethereum".to_string(), "bitcoin".to_string(), "polkadot".to_string()],
            active_bridges: Vec::new(),
        }
    }

    /// Register a new bridge
    pub fn register_bridge(&mut self, bridge_id: String) -> Result<()> {
        if !self.active_bridges.contains(&bridge_id) {
            self.active_bridges.push(bridge_id);
            Ok(())
        } else {
            Err(BlockchainError::Bridge("Bridge already registered".to_string()))
        }
    }

    /// Get active bridges
    pub fn get_active_bridges(&self) -> &[String] {
        &self.active_bridges
    }
}
