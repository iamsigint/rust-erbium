use crate::core::types::Address;
use crate::utils::error::{BlockchainError, Result};
use std::collections::{HashMap, HashSet};

#[derive(Default)]
pub struct ValidatorSet {
    validators: HashSet<Address>,
    public_keys: HashMap<Address, Vec<u8>>, // validator address -> public key
    active_set: HashSet<Address>,           // currently active validators
}

impl ValidatorSet {
    pub fn new() -> Self {
        Self {
            validators: HashSet::new(),
            public_keys: HashMap::new(),
            active_set: HashSet::new(),
        }
    }

    pub fn add_validator(&mut self, address: Address) -> Result<()> {
        if self.validators.contains(&address) {
            return Err(BlockchainError::Consensus(
                "Validator already exists".to_string(),
            ));
        }

        self.validators.insert(address.clone());
        self.active_set.insert(address);
        Ok(())
    }

    pub fn remove_validator(&mut self, address: &Address) -> Result<()> {
        if !self.validators.contains(address) {
            return Err(BlockchainError::Consensus(
                "Validator not found".to_string(),
            ));
        }

        self.validators.remove(address);
        self.active_set.remove(address);
        self.public_keys.remove(address);
        Ok(())
    }

    pub fn is_validator(&self, address: &Address) -> bool {
        self.validators.contains(address)
    }

    pub fn is_active(&self, address: &Address) -> bool {
        self.active_set.contains(address)
    }

    pub fn set_public_key(&mut self, address: Address, public_key: Vec<u8>) -> Result<()> {
        if !self.validators.contains(&address) {
            return Err(BlockchainError::Consensus(
                "Validator not found".to_string(),
            ));
        }

        self.public_keys.insert(address, public_key);
        Ok(())
    }

    pub fn get_public_key(&self, address: &Address) -> Option<Vec<u8>> {
        self.public_keys.get(address).cloned()
    }

    pub fn get_active_validators(&self) -> Vec<Address> {
        self.active_set.iter().cloned().collect()
    }

    pub fn get_all_validators(&self) -> Vec<Address> {
        self.validators.iter().cloned().collect()
    }

    pub fn len(&self) -> usize {
        self.validators.len()
    }

    pub fn is_empty(&self) -> bool {
        self.validators.is_empty()
    }
}
