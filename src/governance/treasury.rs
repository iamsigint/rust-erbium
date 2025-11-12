use crate::core::types::Address;
use crate::utils::error::{Result, BlockchainError};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Treasury {
    pub address: Address,
    balances: HashMap<Address, u64>, // token address -> balance
    transactions: Vec<TreasuryTransaction>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TreasuryTransaction {
    pub id: u64,
    pub from: Address,
    pub to: Address,
    pub amount: u64,
    pub token: Address,
    pub description: String,
    pub timestamp: u64,
    pub executed_by: Address,
}

impl Treasury {
    pub fn new(address: Address) -> Result<Self> {
        Ok(Self {
            address,
            balances: HashMap::new(),
            transactions: Vec::new(),
        })
    }
    
    pub fn deposit(&mut self, from: Address, token: Address, amount: u64) -> Result<()> {
        *self.balances.entry(token.clone()).or_insert(0) += amount;
        
        let transaction = TreasuryTransaction {
            id: self.transactions.len() as u64 + 1,
            from,
            to: self.address.clone(),
            amount,
            token,
            description: "Deposit".to_string(),
            timestamp: chrono::Utc::now().timestamp_millis() as u64,
            executed_by: self.address.clone(),
        };
        
        self.transactions.push(transaction);
        Ok(())
    }
    
    pub fn transfer(&mut self, to: &Address, amount: u64) -> Result<()> {
        // For simplicity, assume native token transfer
        let native_token = self.address.clone();
        let balance = self.balances.get(&native_token).copied().unwrap_or(0);
        
        if balance < amount {
            return Err(BlockchainError::Validator("Insufficient treasury funds".to_string()));
        }
        
        *self.balances.entry(native_token.clone()).or_insert(0) -= amount;
        
        let transaction = TreasuryTransaction {
            id: self.transactions.len() as u64 + 1,
            from: self.address.clone(),
            to: to.clone(),
            amount,
            token: native_token,
            description: "Treasury transfer".to_string(),
            timestamp: chrono::Utc::now().timestamp_millis() as u64,
            executed_by: self.address.clone(),
        };
        
        self.transactions.push(transaction);
        Ok(())
    }
    
    pub fn get_balance(&self, token: &Address) -> u64 {
        self.balances.get(token).copied().unwrap_or(0)
    }
    
    pub fn get_total_balance(&self) -> u64 {
        self.balances.values().sum()
    }
    
    pub fn get_transaction_history(&self, limit: Option<usize>) -> Vec<&TreasuryTransaction> {
        let mut history: Vec<&TreasuryTransaction> = self.transactions.iter().collect();
        history.reverse(); // Most recent first
        
        if let Some(limit) = limit {
            history.truncate(limit);
        }
        
        history
    }
    
    pub fn get_transaction(&self, id: u64) -> Option<&TreasuryTransaction> {
        self.transactions.iter().find(|tx| tx.id == id)
    }
    
    pub fn get_balances(&self) -> &HashMap<Address, u64> {
        &self.balances
    }
}