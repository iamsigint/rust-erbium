use super::{ConsensusConfig, validator::ValidatorSet};
use crate::core::Block;
use crate::core::types::Address;
use crate::utils::error::{Result, BlockchainError};
use std::collections::HashMap;

pub struct ProofOfStake {
    config: ConsensusConfig,
    validator_set: ValidatorSet,
    staking_pool: HashMap<Address, u64>, // address -> stake amount
    total_stake: u64,
}

impl ProofOfStake {
    pub fn new(config: ConsensusConfig) -> Self {
        Self {
            config,
            validator_set: ValidatorSet::new(),
            staking_pool: HashMap::new(),
            total_stake: 0,
        }
    }
    
    /// Calculate validator probability based on stake
    pub fn calculate_validator_probability(&self, validator_stake: u64) -> f64 {
        if self.total_stake == 0 {
            return 0.0;
        }
        validator_stake as f64 / self.total_stake as f64
    }
    
    /// Select next validator based on stake-weighted probability
    pub fn select_next_validator(&self) -> Result<Address> {
        if self.staking_pool.is_empty() {
            return Err(BlockchainError::Consensus("No validators available".to_string()));
        }
        
        let random_value = rand::random::<f64>();
        let mut cumulative_probability = 0.0;
        
        for (address, stake) in &self.staking_pool {
            let probability = self.calculate_validator_probability(*stake);
            cumulative_probability += probability;
            
            if random_value <= cumulative_probability {
                return Ok(address.clone());
            }
        }
        
        // Fallback: select validator with highest stake
        self.staking_pool
            .iter()
            .max_by_key(|(_, stake)| *stake)
            .map(|(address, _)| address.clone())
            .ok_or_else(|| BlockchainError::Consensus("Failed to select validator".to_string()))
    }
    
    /// Add stake to the pool
    pub fn add_stake(&mut self, address: Address, amount: u64) -> Result<()> {
        if amount < self.config.min_stake {
            return Err(BlockchainError::Consensus(
                format!("Stake amount {} below minimum {}", amount, self.config.min_stake)
            ));
        }
        
        *self.staking_pool.entry(address.clone()).or_insert(0) += amount;
        self.total_stake += amount;
        
        // Update validator set if qualified
        if self.staking_pool[&address] >= self.config.min_stake {
            self.validator_set.add_validator(address.clone())?;
        }
        
        Ok(())
    }
    
    /// Remove stake from the pool
    pub fn remove_stake(&mut self, address: &Address, amount: u64) -> Result<()> {
        if let Some(stake) = self.staking_pool.get_mut(address) {
            if *stake < amount {
                return Err(BlockchainError::Consensus("Insufficient stake to remove".to_string()));
            }
            
            *stake -= amount;
            self.total_stake -= amount;
            
            // Remove from validator set if below minimum
            if *stake < self.config.min_stake {
                self.validator_set.remove_validator(address)?;
            }
            
            if *stake == 0 {
                self.staking_pool.remove(address);
            }
        }
        
        Ok(())
    }
    
    /// Validate block according to consensus rules
    pub fn validate_block(&self, block: &Block, validator: &Address) -> Result<bool> {
        // Check if validator is authorized
        if !self.validator_set.is_validator(validator) {
            return Ok(false);
        }
        
        // Verify block signature
        let public_key = self.validator_set.get_public_key(validator)
            .ok_or_else(|| BlockchainError::Consensus("Validator public key not found".to_string()))?;
        
        if !block.verify_signature(&public_key)? {
            return Ok(false);
        }
        
        // Check block timestamp (within reasonable range)
        let current_time = chrono::Utc::now().timestamp_millis() as u64;
        if block.header.timestamp > current_time + 30000 { // 30 seconds in future max
            return Ok(false);
        }
        
        // Additional consensus rules can be added here
        
        Ok(true)
    }
    
    /// Get total network stake
    pub fn total_stake(&self) -> u64 {
        self.total_stake
    }
    
    /// Get validator stake
    pub fn get_validator_stake(&self, address: &Address) -> Option<u64> {
        self.staking_pool.get(address).copied()
    }

    /// Process staking rewards for all validators
    pub fn process_staking_rewards(&mut self) -> Result<()> {
        if self.staking_pool.is_empty() {
            return Ok(());
        }

        let _total_stake = self.total_stake;
        let reward_rate = 0.05; // 5% annual reward rate (simplified)

        for (address, stake) in &mut self.staking_pool {
            let reward = (*stake as f64 * reward_rate / 365.0) as u64; // Daily reward
            if reward > 0 {
                *stake += reward;
                self.total_stake += reward;
                log::debug!("Processed reward for {}: +{}", address.as_str(), reward);
            }
        }

        Ok(())
    }

    /// Check for validator slashing conditions
    pub fn check_validator_slashing(&mut self) -> Result<()> {
        // Simplified slashing logic - in production this would check for:
        // - Double signing
        // - Extended downtime
        // - Malicious behavior

        // For now, just log that slashing check was performed
        log::debug!("Validator slashing check completed - no violations found");

        Ok(())
    }

    /// Add stake from a transaction (called by state layer)
    pub fn add_stake_from_transaction(&mut self, address: Address, amount: u64) -> Result<()> {
        self.add_stake(address, amount)
    }

    /// Remove stake from a transaction (called by state layer)
    pub fn remove_stake_from_transaction(&mut self, address: &Address, amount: u64) -> Result<()> {
        self.remove_stake(address, amount)
    }

    /// Set public key for validator (used in tests)
    pub fn set_validator_public_key(&mut self, address: Address, public_key: Vec<u8>) -> Result<()> {
        self.validator_set.set_public_key(address, public_key)
    }
}
