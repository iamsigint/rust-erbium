use crate::core::types::Address;
use crate::utils::error::{Result, BlockchainError};
use std::collections::HashMap;

pub struct SlashingEngine {
    slashed_validators: HashMap<Address, SlashReason>,
    _slashing_percentage: u8,
}

#[derive(Debug, Clone)]
pub enum SlashReason {
    DoubleSigning,
    InvalidBlock,
    Unavailability,
    SecurityBreach,
}

impl SlashingEngine {
    pub fn new(slashing_percentage: u8) -> Self {
        Self {
            slashed_validators: HashMap::new(),
            _slashing_percentage: slashing_percentage,
        }
    }
    
    pub fn slash_validator(
        &mut self, 
        validator: Address, 
        reason: SlashReason,
        current_stake: u64
    ) -> Result<u64> {
        // Apply percentage-based slashing
        let slashed_amount = ((current_stake as u128 * self._slashing_percentage as u128) / 100u128) as u64;
        
        let validator_clone = validator.clone();
        let reason_clone = reason.clone();
        
        self.slashed_validators.insert(validator, reason);
        
        log::warn!(
            "Validator {} slashed {} ERB for reason: {:?}",
            validator_clone.as_str(),
            slashed_amount,
            reason_clone
        );
        
        Ok(slashed_amount)
    }
    
    pub fn is_slashed(&self, validator: &Address) -> bool {
        self.slashed_validators.contains_key(validator)
    }
    
    pub fn get_slash_reason(&self, validator: &Address) -> Option<&SlashReason> {
        self.slashed_validators.get(validator)
    }
    
    pub fn forgive_validator(&mut self, validator: &Address) -> Result<()> {
        if self.slashed_validators.remove(validator).is_none() {
            return Err(BlockchainError::Consensus("Validator not found in slashed list".to_string()));
        }
        Ok(())
    }
    
    pub fn get_slashed_validators(&self) -> Vec<(&Address, &SlashReason)> {
        self.slashed_validators.iter().collect()
    }
}