// Cross-chain bridge functionality
pub mod adapter;
pub mod core;
pub mod transaction_builder;
pub mod types;

// Re-export core components
pub use adapter::BridgeAdapter;
pub use core::*;
pub use transaction_builder::TransactionBuilder;
pub use types::*;
