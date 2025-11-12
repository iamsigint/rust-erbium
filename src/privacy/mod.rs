pub mod zk_circuits;
pub mod confidential_tx;
pub mod range_proofs;
pub mod stealth_addresses;

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PrivacyConfig {
    pub enable_zk_proofs: bool,
    pub enable_stealth_addresses: bool,
    pub max_confidential_amount: u64,
    pub zk_circuit_complexity: u32,
}

impl Default for PrivacyConfig {
    fn default() -> Self {
        Self {
            enable_zk_proofs: true,
            enable_stealth_addresses: true,
            max_confidential_amount: 1_000_000_000, // 1M ERB
            zk_circuit_complexity: 1000,
        }
    }
}