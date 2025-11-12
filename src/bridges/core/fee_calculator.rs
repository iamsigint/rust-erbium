// src/bridges/core/fee_calculator.rs
use std::collections::HashMap;

pub struct FeeCalculator {
    base_fees: HashMap<String, u64>, // chain_id -> base_fee
    dynamic_fee_multipliers: HashMap<String, f64>, // chain_id -> multiplier
}

impl FeeCalculator {
    pub fn new() -> Self {
        let mut base_fees = HashMap::new();
        let mut dynamic_fee_multipliers = HashMap::new();
        
        // Initialize with default fees
        base_fees.insert("bitcoin".to_string(), 1000); // satoshis
        base_fees.insert("ethereum".to_string(), 5000000000000000); // wei
        base_fees.insert("polkadot".to_string(), 100000000); // planck
        base_fees.insert("erbium".to_string(), 100000000); // smallest units
        
        dynamic_fee_multipliers.insert("bitcoin".to_string(), 1.2);
        dynamic_fee_multipliers.insert("ethereum".to_string(), 1.5);
        dynamic_fee_multipliers.insert("polkadot".to_string(), 1.1);
        dynamic_fee_multipliers.insert("erbium".to_string(), 1.0);
        
        Self {
            base_fees,
            dynamic_fee_multipliers,
        }
    }
    
    pub fn calculate_bridge_fee(
        &self,
        amount: u64,
        source_chain: &str,
        target_chain: &str,
        bridge_fee_percent: f64,
    ) -> Result<u64, FeeError> {
        if amount == 0 {
            return Err(FeeError::InvalidAmount);
        }
        
        if bridge_fee_percent < 0.0 || bridge_fee_percent > 10.0 {
            return Err(FeeError::InvalidFeePercentage);
        }
        
        // Calculate percentage-based fee
        let percentage_fee = (amount as f64 * bridge_fee_percent / 100.0) as u64;
        
        // Calculate network fees for both chains
        let source_network_fee = self.calculate_network_fee(source_chain)?;
        let target_network_fee = self.calculate_network_fee(target_chain)?;
        
        // Total fee
        let total_fee = percentage_fee + source_network_fee + target_network_fee;
        
        // Ensure fee is not more than amount
        if total_fee >= amount {
            return Err(FeeError::FeeExceedsAmount);
        }
        
        Ok(total_fee)
    }
    
    pub fn calculate_network_fee(&self, chain_id: &str) -> Result<u64, FeeError> {
        let base_fee = self.base_fees.get(chain_id)
            .ok_or_else(|| FeeError::ChainNotSupported(chain_id.to_string()))?;
        
        let multiplier = self.dynamic_fee_multipliers.get(chain_id)
            .unwrap_or(&1.0);
        
        let dynamic_fee = (*base_fee as f64 * multiplier) as u64;
        
        Ok(dynamic_fee)
    }
    
    pub fn update_base_fee(&mut self, chain_id: &str, new_fee: u64) -> Result<(), FeeError> {
        if new_fee == 0 {
            return Err(FeeError::InvalidAmount);
        }
        
        self.base_fees.insert(chain_id.to_string(), new_fee);
        log::info!("Updated base fee for {}: {}", chain_id, new_fee);
        
        Ok(())
    }
    
    pub fn update_dynamic_multiplier(&mut self, chain_id: &str, multiplier: f64) -> Result<(), FeeError> {
        if multiplier <= 0.0 || multiplier > 10.0 {
            return Err(FeeError::InvalidMultiplier);
        }
        
        self.dynamic_fee_multipliers.insert(chain_id.to_string(), multiplier);
        log::info!("Updated dynamic multiplier for {}: {}", chain_id, multiplier);
        
        Ok(())
    }
}

#[derive(Debug, thiserror::Error)]
pub enum FeeError {
    #[error("Invalid amount")]
    InvalidAmount,
    #[error("Invalid fee percentage")]
    InvalidFeePercentage,
    #[error("Fee exceeds transfer amount")]
    FeeExceedsAmount,
    #[error("Chain not supported: {0}")]
    ChainNotSupported(String),
    #[error("Invalid multiplier")]
    InvalidMultiplier,
}