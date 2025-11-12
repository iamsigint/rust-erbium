use crate::core::types::{Address, Hash};
use crate::utils::error::{Result, BlockchainError};
use crate::vm::storage::ContractStorage;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ERC20Contract {
    pub address: Address,
    pub name: String,
    pub symbol: String,
    pub decimals: u8,
    pub total_supply: u64,
    storage: ContractStorage,
    balances: HashMap<Address, u64>,
    allowances: HashMap<(Address, Address), u64>,
}

impl ERC20Contract {
    pub fn new(
        address: Address,
        name: String,
        symbol: String,
        decimals: u8,
        initial_supply: u64,
    ) -> Result<Self> {
        let mut contract = Self {
            address,
            name,
            symbol,
            decimals,
            total_supply: initial_supply,
            storage: ContractStorage::new()?,
            balances: HashMap::new(),
            allowances: HashMap::new(),
        };
        
        // Initialize storage
        contract.storage.initialize_contract(&contract.address)?;
        
        // Set creator balance to initial supply
        contract.balances.insert(contract.address.clone(), initial_supply);
        
        Ok(contract)
    }
    
    pub fn transfer(&mut self, from: &Address, to: &Address, amount: u64) -> Result<bool> {
        // Check if sender has sufficient balance
        let from_balance = self.balance_of(from)?;
        if from_balance < amount {
            return Ok(false);
        }
        
        // Update balances
        *self.balances.entry(from.clone()).or_insert(0) -= amount;
        *self.balances.entry(to.clone()).or_insert(0) += amount;
        
        // Update storage
        self.update_balance_in_storage(from)?;
        self.update_balance_in_storage(to)?;
        
        // Emit transfer event
        self.emit_transfer_event(from, to, amount)?;
        
        Ok(true)
    }
    
    pub fn transfer_from(
        &mut self,
        spender: &Address,
        from: &Address,
        to: &Address,
        amount: u64,
    ) -> Result<bool> {
        // Check allowance
        let allowance = self.allowances.get(&(from.clone(), spender.clone()))
            .copied()
            .unwrap_or(0);
        
        if allowance < amount {
            return Ok(false);
        }
        
        // Check balance
        let from_balance = self.balance_of(from)?;
        if from_balance < amount {
            return Ok(false);
        }
        
        // Update balances and allowance
        *self.balances.entry(from.clone()).or_insert(0) -= amount;
        *self.balances.entry(to.clone()).or_insert(0) += amount;
        self.allowances.insert((from.clone(), spender.clone()), allowance - amount);
        
        // Update storage
        self.update_balance_in_storage(from)?;
        self.update_balance_in_storage(to)?;
        self.update_allowance_in_storage(from, spender)?;
        
        // Emit transfer event
        self.emit_transfer_event(from, to, amount)?;
        
        Ok(true)
    }
    
    pub fn approve(&mut self, owner: &Address, spender: &Address, amount: u64) -> Result<bool> {
        self.allowances.insert((owner.clone(), spender.clone()), amount);
        self.update_allowance_in_storage(owner, spender)?;
        
        // Emit approval event
        self.emit_approval_event(owner, spender, amount)?;
        
        Ok(true)
    }
    
    pub fn balance_of(&self, address: &Address) -> Result<u64> {
        Ok(self.balances.get(address).copied().unwrap_or(0))
    }
    
    pub fn allowance(&self, owner: &Address, spender: &Address) -> Result<u64> {
        Ok(self.allowances.get(&(owner.clone(), spender.clone())).copied().unwrap_or(0))
    }
    
    pub fn mint(&mut self, to: &Address, amount: u64) -> Result<bool> {
        *self.balances.entry(to.clone()).or_insert(0) += amount;
        self.total_supply += amount;
        
        self.update_balance_in_storage(to)?;
        self.update_total_supply_in_storage()?;
        
        // Emit transfer event from zero address for minting
        let zero_address = Address::new_unchecked("0x0000000000000000000000000000000000000000".to_string());
        self.emit_transfer_event(&zero_address, to, amount)?;
        
        Ok(true)
    }
    
    pub fn burn(&mut self, from: &Address, amount: u64) -> Result<bool> {
        let balance = self.balance_of(from)?;
        if balance < amount {
            return Ok(false);
        }
        
        *self.balances.entry(from.clone()).or_insert(0) -= amount;
        self.total_supply -= amount;
        
        self.update_balance_in_storage(from)?;
        self.update_total_supply_in_storage()?;
        
        // Emit transfer event to zero address for burning
        let zero_address = Address::new_unchecked("0x0000000000000000000000000000000000000000".to_string());
        self.emit_transfer_event(from, &zero_address, amount)?;
        
        Ok(true)
    }
    
    fn update_balance_in_storage(&mut self, address: &Address) -> Result<()> {
        let balance = self.balance_of(address)?;
        let balance_key = Self::get_balance_key(address);
        self.storage.store(&self.address, balance_key, balance.to_be_bytes().to_vec())?;
        Ok(())
    }
    
    fn update_allowance_in_storage(&mut self, owner: &Address, spender: &Address) -> Result<()> {
        let allowance = self.allowance(owner, spender)?;
        let allowance_key = Self::get_allowance_key(owner, spender);
        self.storage.store(&self.address, allowance_key, allowance.to_be_bytes().to_vec())?;
        Ok(())
    }
    
    fn update_total_supply_in_storage(&mut self) -> Result<()> {
        let total_supply_key = b"total_supply".to_vec();
        self.storage.store(&self.address, total_supply_key, self.total_supply.to_be_bytes().to_vec())?;
        Ok(())
    }
    
    fn emit_transfer_event(&self, from: &Address, to: &Address, amount: u64) -> Result<()> {
        // In real implementation, this would add to transaction logs
        log::debug!("ERC20 Transfer: {} from {} to {}", amount, from.as_str(), to.as_str());
        Ok(())
    }
    
    fn emit_approval_event(&self, owner: &Address, spender: &Address, amount: u64) -> Result<()> {
        // In real implementation, this would add to transaction logs
        log::debug!("ERC20 Approval: {} approved for {} by {}", amount, spender.as_str(), owner.as_str());
        Ok(())
    }
    
    fn get_balance_key(address: &Address) -> Vec<u8> {
        let mut key = b"balance_".to_vec();
        key.extend_from_slice(address.as_str().as_bytes());
        key
    }
    
    fn get_allowance_key(owner: &Address, spender: &Address) -> Vec<u8> {
        let mut key = b"allowance_".to_vec();
        key.extend_from_slice(owner.as_str().as_bytes());
        key.extend_from_slice(b"_");
        key.extend_from_slice(spender.as_str().as_bytes());
        key
    }
    
    pub fn get_storage_root(&self) -> Result<Hash> {
        self.storage.calculate_storage_root(&self.address)
    }
}

impl crate::vm::SmartContract for ERC20Contract {
    fn execute(&mut self, function: &str, args: &[u8]) -> Result<Vec<u8>> {
        match function {
            "transfer" => {
                if args.len() < 40 {
                    return Err(BlockchainError::VM("Invalid transfer arguments".to_string()));
                }
                let to = Address::new_unchecked(String::from_utf8_lossy(&args[0..20]).to_string());
                let amount = u64::from_be_bytes(args[20..28].try_into().unwrap());
                let from = Address::new_unchecked(String::from_utf8_lossy(&args[28..48]).to_string());
                
                let success = self.transfer(&from, &to, amount)?;
                Ok(vec![if success { 1 } else { 0 }])
            }
            "balanceOf" => {
                if args.len() < 20 {
                    return Err(BlockchainError::VM("Invalid balanceOf arguments".to_string()));
                }
                let address = Address::new_unchecked(String::from_utf8_lossy(&args[0..20]).to_string());
                let balance = self.balance_of(&address)?;
                Ok(balance.to_be_bytes().to_vec())
            }
            "approve" => {
                if args.len() < 48 {
                    return Err(BlockchainError::VM("Invalid approve arguments".to_string()));
                }
                let spender = Address::new_unchecked(String::from_utf8_lossy(&args[0..20]).to_string());
                let amount = u64::from_be_bytes(args[20..28].try_into().unwrap());
                let owner = Address::new_unchecked(String::from_utf8_lossy(&args[28..48]).to_string());
                
                let success = self.approve(&owner, &spender, amount)?;
                Ok(vec![if success { 1 } else { 0 }])
            }
            _ => Err(BlockchainError::VM(format!("Unknown function: {}", function))),
        }
    }
    
    fn get_storage_root(&self) -> Result<Hash> {
        self.get_storage_root()
    }
    
    fn get_address(&self) -> &Address {
        &self.address
    }
}