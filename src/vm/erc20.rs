//! ERC20 token contract implementation

use crate::core::types::Address;
use crate::utils::error::{BlockchainError, Result};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// ERC20 token contract
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ERC20Contract {
    pub name: String,
    pub symbol: String,
    pub decimals: u8,
    pub total_supply: u64,
    pub owner: Address,
    pub balances: HashMap<Address, u64>,
    pub allowances: HashMap<(Address, Address), u64>, // (owner, spender) -> amount
}

impl ERC20Contract {
    /// Create a new ERC20 contract
    pub fn new(
        name: String,
        symbol: String,
        decimals: u8,
        initial_supply: u64,
        owner: Address,
    ) -> Self {
        let mut balances = HashMap::new();
        balances.insert(owner.clone(), initial_supply);

        Self {
            name,
            symbol,
            decimals,
            total_supply: initial_supply,
            owner,
            balances,
            allowances: HashMap::new(),
        }
    }

    /// Transfer tokens from sender to recipient
    pub fn transfer(&mut self, from: &Address, to: &Address, amount: u64) -> Result<()> {
        let sender_balance = self.balances.get(from).copied().unwrap_or(0);

        if sender_balance < amount {
            return Err(BlockchainError::VM("Insufficient balance".to_string()));
        }

        // Update balances
        *self.balances.entry(from.clone()).or_insert(0) = sender_balance - amount;
        *self.balances.entry(to.clone()).or_insert(0) += amount;

        Ok(())
    }

    /// Transfer tokens from one address to another using allowance
    pub fn transfer_from(
        &mut self,
        spender: &Address,
        from: &Address,
        to: &Address,
        amount: u64,
    ) -> Result<()> {
        let allowance = self
            .allowances
            .get(&(from.clone(), spender.clone()))
            .copied()
            .unwrap_or(0);

        if allowance < amount {
            return Err(BlockchainError::VM("Insufficient allowance".to_string()));
        }

        // First transfer the tokens
        self.transfer(from, to, amount)?;

        // Update allowance
        *self
            .allowances
            .entry((from.clone(), spender.clone()))
            .or_insert(0) = allowance - amount;

        Ok(())
    }

    /// Approve spender to spend tokens on behalf of owner
    pub fn approve(&mut self, owner: &Address, spender: &Address, amount: u64) -> Result<()> {
        self.allowances
            .insert((owner.clone(), spender.clone()), amount);
        Ok(())
    }

    /// Get balance of an account
    pub fn balance_of(&self, account: &Address) -> u64 {
        self.balances.get(account).copied().unwrap_or(0)
    }

    /// Get allowance for spender to spend owner's tokens
    pub fn allowance(&self, owner: &Address, spender: &Address) -> u64 {
        self.allowances
            .get(&(owner.clone(), spender.clone()))
            .copied()
            .unwrap_or(0)
    }

    /// Mint new tokens (only owner)
    pub fn mint(&mut self, caller: &Address, to: &Address, amount: u64) -> Result<()> {
        if caller != &self.owner {
            return Err(BlockchainError::VM("Only owner can mint".to_string()));
        }

        *self.balances.entry(to.clone()).or_insert(0) += amount;
        self.total_supply += amount;

        Ok(())
    }

    /// Burn tokens
    pub fn burn(&mut self, from: &Address, amount: u64) -> Result<()> {
        let balance = self.balances.get(from).copied().unwrap_or(0);

        if balance < amount {
            return Err(BlockchainError::VM(
                "Insufficient balance to burn".to_string(),
            ));
        }

        *self.balances.entry(from.clone()).or_insert(0) = balance - amount;
        self.total_supply -= amount;

        Ok(())
    }

    /// Get contract metadata
    pub fn name(&self) -> &str {
        &self.name
    }

    pub fn symbol(&self) -> &str {
        &self.symbol
    }

    pub fn decimals(&self) -> u8 {
        self.decimals
    }

    pub fn total_supply(&self) -> u64 {
        self.total_supply
    }

    pub fn owner(&self) -> &Address {
        &self.owner
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::core::types::Address;

    fn create_test_address(id: u8) -> Address {
        Address::new(format!("0x{:040x}", id)).unwrap()
    }

    #[test]
    fn test_erc20_creation() {
        let owner = create_test_address(1);
        let contract = ERC20Contract::new(
            "Test Token".to_string(),
            "TEST".to_string(),
            18,
            1000000,
            owner.clone(),
        );

        assert_eq!(contract.name(), "Test Token");
        assert_eq!(contract.symbol(), "TEST");
        assert_eq!(contract.decimals(), 18);
        assert_eq!(contract.total_supply(), 1000000);
        assert_eq!(contract.balance_of(&owner), 1000000);
    }

    #[test]
    fn test_erc20_transfer() {
        let owner = create_test_address(1);
        let recipient = create_test_address(2);
        let mut contract = ERC20Contract::new(
            "Test Token".to_string(),
            "TEST".to_string(),
            18,
            1000,
            owner.clone(),
        );

        // Transfer 500 tokens
        contract.transfer(&owner, &recipient, 500).unwrap();

        assert_eq!(contract.balance_of(&owner), 500);
        assert_eq!(contract.balance_of(&recipient), 500);
    }

    #[test]
    fn test_erc20_transfer_insufficient_balance() {
        let owner = create_test_address(1);
        let recipient = create_test_address(2);
        let mut contract = ERC20Contract::new(
            "Test Token".to_string(),
            "TEST".to_string(),
            18,
            100,
            owner.clone(),
        );

        // Try to transfer more than balance
        let result = contract.transfer(&owner, &recipient, 200);
        assert!(result.is_err());
        assert_eq!(contract.balance_of(&owner), 100);
        assert_eq!(contract.balance_of(&recipient), 0);
    }

    #[test]
    fn test_erc20_approve_and_transfer_from() {
        let owner = create_test_address(1);
        let spender = create_test_address(2);
        let recipient = create_test_address(3);
        let mut contract = ERC20Contract::new(
            "Test Token".to_string(),
            "TEST".to_string(),
            18,
            1000,
            owner.clone(),
        );

        // Approve spender to spend 500 tokens
        contract.approve(&owner, &spender, 500).unwrap();
        assert_eq!(contract.allowance(&owner, &spender), 500);

        // Transfer from owner to recipient using spender's allowance
        contract
            .transfer_from(&spender, &owner, &recipient, 300)
            .unwrap();

        assert_eq!(contract.balance_of(&owner), 700);
        assert_eq!(contract.balance_of(&recipient), 300);
        assert_eq!(contract.allowance(&owner, &spender), 200);
    }

    #[test]
    fn test_erc20_mint_and_burn() {
        let owner = create_test_address(1);
        let user = create_test_address(2);
        let mut contract = ERC20Contract::new(
            "Test Token".to_string(),
            "TEST".to_string(),
            18,
            1000,
            owner.clone(),
        );

        // Mint tokens to user
        contract.mint(&owner, &user, 500).unwrap();
        assert_eq!(contract.total_supply(), 1500);
        assert_eq!(contract.balance_of(&user), 500);

        // Burn tokens from user
        contract.burn(&user, 200).unwrap();
        assert_eq!(contract.total_supply(), 1300);
        assert_eq!(contract.balance_of(&user), 300);
    }

    #[test]
    fn test_erc20_mint_unauthorized() {
        let owner = create_test_address(1);
        let unauthorized = create_test_address(2);
        let user = create_test_address(3);
        let mut contract = ERC20Contract::new(
            "Test Token".to_string(),
            "TEST".to_string(),
            18,
            1000,
            owner.clone(),
        );

        // Try to mint from unauthorized address
        let result = contract.mint(&unauthorized, &user, 500);
        assert!(result.is_err());
        assert_eq!(contract.total_supply(), 1000);
        assert_eq!(contract.balance_of(&user), 0);
    }
}
