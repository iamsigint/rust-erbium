#![recursion_limit = "1024"]

// src/lib.rs

pub mod core;
pub mod consensus;
pub mod crypto;
pub mod network;
pub mod privacy;
pub mod vm;
pub mod storage;
// Temporarily disable API module for release 1.0.0 due to compilation issues
// pub mod api;
pub mod governance;
pub mod bridges;
pub mod utils;
pub mod node;
pub mod analytics;
pub mod security;
pub mod compliance;

// Re-export commonly used types
pub use core::{Block, Transaction, Blockchain};
pub use crypto::{Dilithium, ZkProofs};
pub use utils::error::{BlockchainError, Result};
pub use node::NodeManager;

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
