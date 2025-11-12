// src/vm/executor.rs

use super::gas::GasCalculator;
use super::storage::ContractStorage;
use crate::core::types::{Address, Hash};
use crate::core::transaction::Transaction;
use crate::utils::error::{Result, BlockchainError};
use std::collections::HashMap;

#[derive(Debug, Clone)]
pub struct ExecutionResult {
    pub success: bool,
    pub output: Vec<u8>,
    pub gas_used: u64,
    pub logs: Vec<LogEntry>,
    pub state_changes: HashMap<Vec<u8>, Vec<u8>>,
}

#[derive(Debug, Clone)]
pub struct LogEntry {
    pub address: Address,
    pub topics: Vec<Hash>,
    pub data: Vec<u8>,
}

// Added Clone to resolve borrow checker error E0502
#[derive(Debug, Clone)]
pub struct Contract {
    pub code: Vec<u8>,
    pub code_hash: Hash,
    pub storage_root: Hash,
    pub is_zk_enabled: bool,
}

// The struct definition was missing, causing error E0412
pub struct VMExecutor {
    gas_calculator: GasCalculator,
    contract_storage: ContractStorage,
    contracts: HashMap<Address, Contract>,
}

impl VMExecutor {
    pub fn new() -> Result<Self> {
        Ok(Self {
            gas_calculator: GasCalculator::new(),
            contract_storage: ContractStorage::new()?,
            contracts: HashMap::new(),
        })
    }
    
    /// Execute a contract deployment transaction
    pub fn execute_contract_deployment(
        &mut self,
        transaction: &Transaction,
    ) -> Result<ExecutionResult> {
        let mut gas_used = 0u64;
        
        // Validate contract code size
        if transaction.data.len() > 24 * 1024 {
            return Ok(ExecutionResult {
                success: false,
                output: vec![],
                gas_used,
                logs: vec![],
                state_changes: HashMap::new(),
            });
        }
        
        // Calculate gas for deployment
        let deployment_gas = self.gas_calculator.calculate_deployment_gas(transaction.data.len());
        gas_used += deployment_gas;
        
        // Create contract address
        let contract_address = self.create_contract_address(&transaction.from, transaction.nonce);
        
        // Hash the contract code
        let code_hash = Hash::new(&transaction.data);
        
        // Create and store contract
        let contract = Contract {
            code: transaction.data.clone(),
            code_hash,
            storage_root: Hash::new(b"empty"),
            is_zk_enabled: false, // Default to non-ZK execution
        };
        
        self.contracts.insert(contract_address.clone(), contract);
        
        // Store initial contract state
        self.contract_storage.initialize_contract(&contract_address)?;
        
        let mut state_changes = HashMap::new();
        state_changes.insert(
            b"contract_deployed".to_vec(),
            contract_address.as_str().as_bytes().to_vec(),
        );
        
        Ok(ExecutionResult {
            success: true,
            output: contract_address.as_str().as_bytes().to_vec(),
            gas_used,
            logs: vec![],
            state_changes,
        })
    }
    
    /// Execute a contract call transaction
    pub fn execute_contract_call(
        &mut self,
        transaction: &Transaction,
    ) -> Result<ExecutionResult> {
        let mut gas_used = 0u64;
        let mut logs = Vec::new();
        let mut state_changes = HashMap::new();
        
        // Get the contract code
        // We clone the necessary data (code) to avoid the immutable borrow (E0502)
        // when calling execute_contract_code, which requires `&mut self`.
        let contract_code = self.contracts.get(&transaction.to)
            .map(|c| c.code.clone())
            .ok_or_else(|| BlockchainError::VM("Contract not found".to_string()))?;
        
        // Calculate base call gas
        let call_gas = self.gas_calculator.calculate_call_gas(transaction.data.len());
        gas_used += call_gas;
        
        // Execute contract code
        let output = self.execute_contract_code(
            &contract_code, // Pass the cloned code
            &transaction.from,
            &transaction.to,
            transaction.amount,
            &transaction.data,
            &mut gas_used,
            &mut logs,
            &mut state_changes,
        )?;
        
        Ok(ExecutionResult {
            success: true,
            output,
            gas_used,
            logs,
            state_changes,
        })
    }
    
    /// Execute contract code (simplified bytecode execution)
    #[allow(clippy::too_many_arguments)]
    fn execute_contract_code(
        &mut self,
        _contract_code: &[u8], // Prefixed unused variable
        _caller: &Address,     // Prefixed unused variable
        contract_address: &Address,
        _value: u64,           // Prefixed unused variable
        call_data: &[u8],
        gas_used: &mut u64,
        logs: &mut Vec<LogEntry>,
        state_changes: &mut HashMap<Vec<u8>, Vec<u8>>,
    ) -> Result<Vec<u8>> {
        // Simplified execution - in real implementation, this would be a full bytecode VM
        
        // Check if this is an ERC20-like transfer
        if call_data.len() >= 4 {
            let selector = &call_data[0..4];
            
            // ERC20 transfer function selector (simplified)
            if selector == [0xa9, 0x05, 0x9c, 0xbb] {
                return self.execute_erc20_transfer(
                    contract_address,
                    call_data,
                    gas_used,
                    logs,
                    state_changes,
                );
            }
            
            // ERC20 balanceOf function selector (simplified)
            if selector == [0x70, 0xa0, 0x82, 0x31] {
                return self.execute_erc20_balance_of(contract_address, call_data);
            }
        }
        
        // Default execution - return empty result
        Ok(vec![])
    }
    
    /// Execute ERC20 transfer function
    fn execute_erc20_transfer(
        &mut self,
        contract_address: &Address,
        _call_data: &[u8], // Prefixed unused variable
        gas_used: &mut u64,
        logs: &mut Vec<LogEntry>,
        state_changes: &mut HashMap<Vec<u8>, Vec<u8>>,
    ) -> Result<Vec<u8>> {
        // Simplified ERC20 transfer execution
        // In real implementation, this would properly decode parameters and update balances
        
        *gas_used += 1000; // Gas cost for transfer
        
        // Create transfer event log
        let transfer_log = LogEntry {
            address: contract_address.clone(),
            topics: vec![
                Hash::new(b"Transfer"), // Event signature
                Hash::new(b"from_address"), // From
                Hash::new(b"to_address"),   // To
            ],
            data: vec![], // Transfer amount would be here
        };
        logs.push(transfer_log);
        
        // Update state (simplified)
        state_changes.insert(b"balance_updated".to_vec(), b"true".to_vec());
        
        // Return success
        Ok(vec![0x01])
    }
    
    /// Execute ERC20 balanceOf function
    fn execute_erc20_balance_of(
        &self,
        _contract_address: &Address, // Prefixed unused variable
        _call_data: &[u8],           // Prefixed unused variable
    ) -> Result<Vec<u8>> {
        // Simplified balanceOf - return a fixed balance
        // In real implementation, this would read from storage
        let balance: u64 = 1000; // Example balance
        Ok(balance.to_be_bytes().to_vec())
    }
    
    /// Create deterministic contract address
    fn create_contract_address(&self, sender: &Address, nonce: u64) -> Address {
        let mut data = sender.as_str().as_bytes().to_vec();
        data.extend(&nonce.to_be_bytes());
        let hash = Hash::new(&data);
        
        // Take last 20 bytes for address (Ethereum-style)
        let address_bytes = &hash.as_bytes()[12..];
        let hex_address = hex::encode(address_bytes);
        
        // Address::new returns a Result, so we must handle it.
        // Using expect() here as an invalid address format is a panic-worthy internal error.
        Address::new(format!("0x{}", hex_address))
            .expect("Failed to create valid contract address from hash")
    }
    
    /// Execute contract with ZK proofs for confidential state
    pub fn execute_confidential_contract(
        &mut self,
        transaction: &Transaction,
        zk_proof: &[u8],
    ) -> Result<ExecutionResult> {
        // Verify ZK proof before execution
        if !self.verify_zk_execution_proof(transaction, zk_proof)? {
            return Ok(ExecutionResult {
                success: false,
                output: vec![],
                gas_used: 0,
                logs: vec![],
                state_changes: HashMap::new(),
            });
        }
        
        // Execute with confidential state
        self.execute_contract_call(transaction)
    }
    
    /// Verify ZK proof for confidential contract execution
    fn verify_zk_execution_proof(
        &self,
        _transaction: &Transaction, // Prefixed unused variable
        _zk_proof: &[u8],          // Prefixed unused variable
    ) -> Result<bool> {
        // In real implementation, this would verify a ZK proof that the contract
        // execution is valid without revealing the internal state
        
        // Placeholder verification
        log::debug!("Verifying ZK execution proof for confidential contract");
        Ok(true)
    }
    
    /// Get contract code by address
    pub fn get_contract_code(&self, address: &Address) -> Option<&Vec<u8>> {
        self.contracts.get(address).map(|c| &c.code)
    }
    
    /// Check if contract exists
    pub fn contract_exists(&self, address: &Address) -> bool {
        self.contracts.contains_key(address)
    }
}