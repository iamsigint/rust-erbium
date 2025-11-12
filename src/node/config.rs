// src/node/config.rs

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NodeConfig {
    pub network: String,
    pub port: u16,
    pub rpc_port: u16,
    pub rest_port: u16,
    pub data_dir: String,
    pub max_peers: u32,
    pub enable_mining: bool,
    pub log_level: String,
    // New RPC-related params
    pub chain_id: u64,
    pub gas_price_wei: u64,
    pub dev_apply_on_send: bool,
    // Asset metadata
    pub asset_name: String,
    pub asset_symbol: String,
    pub asset_decimals: u8,
}

impl Default for NodeConfig {
    fn default() -> Self {
        Self {
            network: "mainnet".to_string(),
            port: 3030,
            rpc_port: 8545,
            rest_port: 8080,
            data_dir: "./data".to_string(),
            max_peers: 50,
            enable_mining: false,
            log_level: "info".to_string(),
            chain_id: 0x539, // 1337
            gas_price_wei: 1_000_000_000, // 1 gwei default
            dev_apply_on_send: true,
            asset_name: "Erbium".to_string(),
            asset_symbol: "ERB".to_string(),
            asset_decimals: 8,
        }
    }
}