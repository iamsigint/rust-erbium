// Cross-chain bridge functionality
pub mod adapter;
pub mod transaction_builder;
pub mod types;
pub mod core;

// Re-export core components
pub use adapter::BridgeAdapter;
pub use transaction_builder::TransactionBuilder;
pub use types::*;
pub use core::*;
