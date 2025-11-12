// src/bridges/light_clients/mod.rs
pub mod bitcoin_client;
pub mod bitcoin_spv;
pub mod ethereum;
pub mod polkadot;
pub mod polkadot_client;
pub mod cosmos_client;
pub mod header_verifier;
pub mod errors;

pub use bitcoin_client::BitcoinLightClient;
pub use bitcoin_spv::BitcoinSPVClient;
pub use ethereum::EthereumLightClient;
pub use polkadot::PolkadotLightClient;
pub use polkadot_client::PolkadotLightClient;
pub use cosmos_client::CosmosLightClient;
pub use header_verifier::{HeaderVerifier, VerificationResult};
