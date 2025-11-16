//! Project Constants
//!
//! Constants derived from configuration files that should be used
//! throughout the application instead of hardcoded values.

/// Gas price configuration (from config/gas.toml)
pub mod gas {
    /// Minimum gas price in ions per gas
    /// Corresponds to `min_gas_price_ions` in config/gas.toml
    pub const MIN_GAS_PRICE_IONS: u64 = 10;

    /// Target gas price in ions per gas
    /// Corresponds to `target_gas_price_ions` in config/gas.toml
    pub const TARGET_GAS_PRICE_IONS: u64 = 50;

    /// Maximum gas price in ions per gas
    /// Corresponds to `max_gas_price_ions` in config/gas.toml
    pub const MAX_GAS_PRICE_IONS: u64 = 500;

    /// Base transaction gas cost in gas units
    /// Corresponds to `base_transaction_gas` in config/gas.toml
    pub const BASE_TRANSACTION_GAS: u64 = 21000;

    /// Maximum gas per transaction for safety
    /// Corresponds to `max_gas_per_transaction` in config/gas.toml
    pub const MAX_GAS_PER_TRANSACTION: u64 = 10000000;

    /// Maximum gas per block for safety
    /// Corresponds to `max_gas_per_block` in config/gas.toml
    pub const MAX_GAS_PER_BLOCK: u64 = 50000000;

    /// Gas cost per transferred byte
    /// Corresponds to `transfer_gas_per_byte` in config/gas.toml
    pub const TRANSFER_GAS_PER_BYTE: u64 = 68;

    /// Signature verification gas cost
    /// Corresponds to `signature_verification_gas` in config/gas.toml
    pub const SIGNATURE_VERIFICATION_GAS: u64 = 3450;

    /// Cross-chain transaction gas cost
    /// Corresponds to `cross_chain_gas` in config/gas.toml
    pub const CROSS_CHAIN_GAS: u64 = 50000;

    /// Layer 2 transaction gas cost
    /// Corresponds to `layer2_gas` in config/gas.toml
    pub const LAYER2_GAS: u64 = 10000;
}

/// Mempool configuration (from config/gas.toml)
pub mod mempool {
    /// Maximum mempool size in MB
    /// Corresponds to `mempool_max_size_mb` in config/gas.toml
    pub const MAX_SIZE_MB: u64 = 200;

    /// Target mempool size for fee adjustments in MB
    /// Corresponds to `mempool_target_size_mb` in config/gas.toml
    pub const TARGET_SIZE_MB: u64 = 150;
}

/// Network constants (not configurable, derived from consensus)
pub mod network {
    /// ERB currency symbol
    pub const CURRENCY_SYMBOL: &str = "ERB";

    /// ERB currency name
    pub const CURRENCY_NAME: &str = "Erbium";

    /// Network identifier
    pub const NETWORK_ID: u64 = 1;

    /// Mainnet network name
    pub const NETWORK_NAME: &str = "mainnet";
}

/// System constants
pub mod system {
    /// Maximum time window for network stats (5 minutes in milliseconds)
    pub const MAX_NETWORK_STATS_WINDOW_MS: i64 = 5 * 60 * 1000;

    /// Default RPC port
    pub const DEFAULT_RPC_PORT: u16 = 8545;

    /// Maximum RPC response size in bytes (100MB)
    pub const MAX_RPC_RESPONSE_SIZE: usize = 100 * 1024 * 1024;

    /// Maximum number of RPC batch requests
    pub const MAX_RPC_BATCH_SIZE: usize = 100;

    /// RPC timeout in seconds
    pub const RPC_TIMEOUT_SECS: u64 = 30;
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_gas_constants() {
        assert_eq!(gas::MIN_GAS_PRICE_IONS, 10);
        assert_eq!(gas::TARGET_GAS_PRICE_IONS, 50);
        assert_eq!(gas::MAX_GAS_PRICE_IONS, 500);
        assert_eq!(gas::BASE_TRANSACTION_GAS, 21000);
    }

    #[test]
    fn test_mempool_constants() {
        assert_eq!(mempool::MAX_SIZE_MB, 200);
        assert_eq!(mempool::TARGET_SIZE_MB, 150);
    }

    #[test]
    fn test_network_constants() {
        assert_eq!(network::CURRENCY_SYMBOL, "ERB");
        assert_eq!(network::NETWORK_ID, 1);
    }
}
