#![recursion_limit = "1024"]

// src/lib.rs

pub mod analytics;
pub mod api;
pub mod bridges;
pub mod compliance;
pub mod consensus;
pub mod core;
pub mod crypto;
pub mod governance;
pub mod network;
pub mod node;
pub mod privacy;
pub mod security;
pub mod storage;
pub mod utils;
pub mod vm;

// Re-export commonly used types
pub use core::{Block, Blockchain, Transaction};
pub use crypto::{Dilithium, ZkProofs};
pub use node::NodeManager;
pub use utils::error::{BlockchainError, Result};

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_basic_block_creation() {
        // Basic test to verify the structure works
        let transactions = Vec::new();
        let block = Block::new(
            1, // version parameter
            crate::core::types::Hash::new(b"genesis"),
            transactions,
            "validator".to_string(),
            1000,
        );

        assert_eq!(block.header.version, 1);
    }
}
