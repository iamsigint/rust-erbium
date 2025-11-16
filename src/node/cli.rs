// src/node/cli.rs

use clap::Parser;

#[derive(Parser)]
#[command(name = "erbium")]
#[command(about = "Erbium Blockchain Node", version = "1.0.0")]
pub struct Cli {
    #[arg(long, default_value = "mainnet")]
    pub network: String,

    #[arg(long, default_value_t = 3030)]
    pub port: u16,

    #[arg(long, default_value_t = 8545)]
    pub rpc_port: u16,

    #[arg(long, default_value_t = 8080)]
    pub rest_port: u16,

    #[arg(long, default_value = "./data")]
    pub data_dir: String,

    #[arg(long, default_value_t = 50)]
    pub max_peers: u32,

    #[arg(long)]
    pub enable_mining: bool,

    #[arg(long, default_value = "info")]
    pub log_level: String,
}

impl Cli {
    pub fn to_config(&self) -> crate::node::config::NodeConfig {
        crate::node::config::NodeConfig {
            network: self.network.clone(),
            port: self.port,
            rpc_port: self.rpc_port,
            rest_port: self.rest_port,
            data_dir: self.data_dir.clone(),
            max_peers: self.max_peers,
            enable_mining: self.enable_mining,
            log_level: self.log_level.clone(),
            // Defaults for new fields; could be overridden by a config loader later
            chain_id: 0x539,
            gas_price_wei: 1_000_000_000,
            dev_apply_on_send: true,
            asset_name: "Erbium".to_string(),
            asset_symbol: "ERB".to_string(),
            asset_decimals: 8,
        }
    }
}
