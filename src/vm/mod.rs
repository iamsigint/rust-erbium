// src/vm/contracts/mod.rs
pub mod bridge_contracts;
pub mod contracts;
pub mod dao;
pub mod erc20;

// Re-export the contract types
pub use bridge_contracts::BridgeContract;
pub use dao::DAOContract;
pub use erc20::ERC20Contract;
