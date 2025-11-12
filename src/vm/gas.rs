
#[derive(Default)]
pub struct GasCalculator {
    base_gas_cost: u64,
    memory_gas_cost: u64,
    storage_gas_cost: u64,
    computation_gas_cost: u64,
}

impl GasCalculator {
    pub fn new() -> Self {
        Self {
            base_gas_cost: 21000, // Base cost for any transaction
            memory_gas_cost: 3,   // Gas per byte of memory
            storage_gas_cost: 20000, // Gas per storage operation
            computation_gas_cost: 10, // Gas per computation step
        }
    }
    
    /// Calculate gas cost for contract deployment
    pub fn calculate_deployment_gas(&self, code_size: usize) -> u64 {
        let memory_gas = code_size as u64 * self.memory_gas_cost;
        let computation_gas = code_size as u64 * self.computation_gas_cost;
        
        self.base_gas_cost + memory_gas + computation_gas
    }
    
    /// Calculate gas cost for contract call
    pub fn calculate_call_gas(&self, data_size: usize) -> u64 {
        let memory_gas = data_size as u64 * self.memory_gas_cost;
        
        self.base_gas_cost + memory_gas
    }
    
    /// Calculate gas cost for storage operations
    pub fn calculate_storage_gas(&self, key_size: usize, value_size: usize) -> u64 {
        let total_size = key_size + value_size;
        total_size as u64 * self.storage_gas_cost
    }
    
    /// Calculate gas cost for ZK proof verification
    pub fn calculate_zk_proof_gas(&self, proof_size: usize) -> u64 {
        // ZK proofs are more computationally expensive
        let base_zk_gas = 100000; // Base cost for ZK verification
        let size_gas = proof_size as u64 * 100; // Additional cost based on proof size
        
        base_zk_gas + size_gas
    }
    
    /// Calculate gas cost for confidential transaction
    pub fn calculate_confidential_tx_gas(&self, tx_size: usize) -> u64 {
        // Confidential transactions require more computation
        let base_cost = self.base_gas_cost * 2; // Double base cost
        let size_cost = tx_size as u64 * self.memory_gas_cost * 2;
        
        base_cost + size_cost + 50000 // Additional fixed cost for ZK
    }
    
    /// Validate gas limit for block
    pub fn validate_block_gas_limit(&self, total_gas: u64, block_gas_limit: u64) -> bool {
        total_gas <= block_gas_limit
    }
}