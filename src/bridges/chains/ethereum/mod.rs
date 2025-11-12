// src/bridges/chains/ethereum/mod.rs
pub mod adapter;
pub mod bridge;
pub mod erc20_handler;
pub mod evm_adapter;
pub mod smart_contracts;
pub mod types;

pub use adapter::EthereumAdapter;
pub use bridge::{EthereumBridge, BridgeConfig, BridgeEvent, BridgeStatus, TokenInfo, PendingTransfer};
pub use erc20_handler::Erc20Handler;
pub use evm_adapter::EvmAdapter;
pub use smart_contracts::EthereumContracts;
pub use types::*;
