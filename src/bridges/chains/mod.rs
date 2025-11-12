// src/bridges/chains/bitcoin/mod.rs
pub mod adapter;
pub mod transaction_builder;
pub mod types;

pub use adapter::BitcoinAdapter;
pub use transaction_builder::BitcoinTransactionBuilder;
pub use types::*;