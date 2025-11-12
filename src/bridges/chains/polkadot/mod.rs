// src/bridges/chains/polkadot/mod.rs
pub mod adapter;
pub mod bridge;
pub mod xcm_adapter;
pub mod types;

pub use adapter::PolkadotAdapter;
pub use bridge::{PolkadotBridge, BridgeConfig, ParachainInfo, PendingXcmMessage, PolkadotBridgeEvent, BridgeStatus};
pub use xcm_adapter::XcmAdapter;
pub use types::*;
