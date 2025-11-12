// src/vm/contracts/mod.rs
pub mod erc20;
pub mod dao;
pub mod bridge_contracts;
pub mod contracts;

// Re-export the contract types
pub use erc20::ERC20Contract;
pub use dao::DAOContract;
pub use bridge_contracts::BridgeContract;
