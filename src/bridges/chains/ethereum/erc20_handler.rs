// src/bridges/chains/ethereum/erc20_handler.rs
use super::types::{Erc20Transfer, Erc20TokenInfo, EthereumError};
use std::str::FromStr;
use web3::types::{Address, H256, U256, Log};

/// ERC20 token handler for Ethereum bridge operations
pub struct Erc20Handler {
    tokens: std::collections::HashMap<Address, Erc20TokenInfo>,
}

impl Erc20Handler {
    /// Create a new ERC20 handler
    pub fn new() -> Self {
        Self {
            tokens: std::collections::HashMap::new(),
        }
    }
    
    /// Register an ERC20 token for monitoring
    pub fn register_token(&mut self, token_info: Erc20TokenInfo) -> Result<(), EthereumError> {
        self.tokens.insert(token_info.address, token_info);
        Ok(())
    }
    
    /// Decode ERC20 Transfer event from transaction log
    pub fn decode_erc20_transfer(&self, log: &Log) -> Option<Erc20Transfer> {
        // ERC20 Transfer event signature: Transfer(address,address,uint256)
        let transfer_signature = H256::from_str("ddf252ad1be2c89b69c2b068fc378daa952ba7f163c4a11628f55a4df523b3ef")
            .unwrap();
        
        // Check if this is a Transfer event
        if log.topics.len() == 3 && log.topics[0] == transfer_signature {
            let from = Address::from_slice(&log.topics[1].as_bytes()[12..]);
            let to = Address::from_slice(&log.topics[2].as_bytes()[12..]);
            
            // Value is in the data field (uint256)
            let value = if log.data.0.len() >= 32 {
                U256::from_big_endian(&log.data.0[0..32])
            } else {
                U256::zero()
            };
            
            Some(Erc20Transfer {
                token_address: log.address,
                from,
                to,
                value,
                transaction_hash: log.transaction_hash.unwrap_or_default(),
            })
        } else {
            None
        }
    }
    
    /// Get token information by address
    pub fn get_token_info(&self, token_address: Address) -> Option<&Erc20TokenInfo> {
        self.tokens.get(&token_address)
    }
    
    /// Check if an address is a registered ERC20 token
    pub fn is_erc20_token(&self, address: Address) -> bool {
        self.tokens.contains_key(&address)
    }
}

impl Default for Erc20Handler {
    fn default() -> Self {
        Self::new()
    }
}