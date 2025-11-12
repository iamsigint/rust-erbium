// src/core/transaction.rs
use serde::{Deserialize, Serialize};
use crate::core::types::{Hash, Address};
use crate::utils::error::{Result, BlockchainError};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)] // Added Eq
pub enum TransactionType {
    Transfer,
    ConfidentialTransfer,
    ContractDeployment,
    ContractCall,
    Stake,
    Unstake,
    Vote,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Transaction {
    pub version: u32,
    pub transaction_type: TransactionType,
    pub from: Address,
    pub to: Address,
    pub amount: u64,
    pub fee: u64,
    pub nonce: u64,
    pub timestamp: u64,
    pub data: Vec<u8>, // For contract deployment/calls
    pub signature: Vec<u8>,
    pub zk_proof: Option<Vec<u8>>, // Zero-knowledge proof for private transactions
    // New fields for confidential transactions
    pub input_commitments: Option<Vec<Vec<u8>>>, // Entry commitments for confidential transactions
    pub output_commitments: Option<Vec<Vec<u8>>>, // Exit commitments for confidential transactions
    pub range_proofs: Option<Vec<Vec<u8>>>, // Range proofs for confidential values
    pub binding_signature: Option<Vec<u8>>, // Connection signature for confidential transactions
}

impl Transaction {
    pub fn new_transfer(
        from: Address,
        to: Address,
        amount: u64,
        fee: u64,
        nonce: u64,
    ) -> Self {
        Transaction {
            version: 1,
            transaction_type: TransactionType::Transfer,
            from,
            to,
            amount,
            fee,
            nonce,
            timestamp: chrono::Utc::now().timestamp_millis() as u64,
            data: Vec::new(),
            signature: Vec::new(),
            zk_proof: None,
            input_commitments: None,
            output_commitments: None,
            range_proofs: None,
            binding_signature: None,
        }
    }
    
    /// Creates a new confidential transfer transaction
    #[allow(clippy::too_many_arguments)]
    pub fn new_confidential_transfer(
        from: Address,
        to: Address,
        amount: u64, // This amount is usually 0, as the real amount is in commitments
        fee: u64,
        nonce: u64,
        zk_proof: Vec<u8>,
        input_commitments: Vec<Vec<u8>>,
        output_commitments: Vec<Vec<u8>>,
        range_proofs: Vec<Vec<u8>>,
        binding_signature: Vec<u8>,
    ) -> Self {
        Transaction {
            version: 1,
            transaction_type: TransactionType::ConfidentialTransfer,
            from,
            to,
            amount, // Public amount, should be 0 for full privacy
            fee,
            nonce,
            timestamp: chrono::Utc::now().timestamp_millis() as u64,
            data: Vec::new(),
            signature: Vec::new(),
            zk_proof: Some(zk_proof),
            input_commitments: Some(input_commitments),
            output_commitments: Some(output_commitments),
            range_proofs: Some(range_proofs),
            binding_signature: Some(binding_signature),
        }
    }
    
    /// Creates a contract deployment transaction
    pub fn new_contract_deployment(
        from: Address,
        contract_code: Vec<u8>,
        fee: u64,
        nonce: u64,
    ) -> Self {
        Transaction {
            version: 1,
            transaction_type: TransactionType::ContractDeployment,
            from: from.clone(),
            // `to` address for contract deployment is conventionally the zero address
            to: Address::new("0x0000000000000000000000000000000000000000".to_string()).unwrap(),
            amount: 0,
            fee,
            nonce,
            timestamp: chrono::Utc::now().timestamp_millis() as u64,
            data: contract_code,
            signature: Vec::new(),
            zk_proof: None,
            input_commitments: None,
            output_commitments: None,
            range_proofs: None,
            binding_signature: None,
        }
    }
    
    /// Creates a contract call transaction
    pub fn new_contract_call(
        from: Address,
        to: Address, // Contract address
        amount: u64, // Amount to be transferred to the contract
        fee: u64,
        nonce: u64,
        call_data: Vec<u8>, // Contract call data
    ) -> Self {
        Transaction {
            version: 1,
            transaction_type: TransactionType::ContractCall,
            from,
            to,
            amount,
            fee,
            nonce,
            timestamp: chrono::Utc::now().timestamp_millis() as u64,
            data: call_data,
            signature: Vec::new(),
            zk_proof: None,
            input_commitments: None,
            output_commitments: None,
            range_proofs: None,
            binding_signature: None,
        }
    }
    
    /// Creates a staking transaction
    pub fn new_stake(
        from: Address,
        amount: u64,
        fee: u64,
        nonce: u64,
    ) -> Self {
        Transaction {
            version: 1,
            transaction_type: TransactionType::Stake,
            from: from.clone(),
            // Staking transactions might go to a specific staking contract address
            to: from, // Placeholder: Staking is done for your own address
            amount,
            fee,
            nonce,
            timestamp: chrono::Utc::now().timestamp_millis() as u64,
            data: Vec::new(),
            signature: Vec::new(),
            zk_proof: None,
            input_commitments: None,
            output_commitments: None,
            range_proofs: None,
            binding_signature: None,
        }
    }
    
    /// Creates an unstaking transaction
    pub fn new_unstake(
        from: Address,
        amount: u64,
        fee: u64,
        nonce: u64,
    ) -> Self {
        Transaction {
            version: 1,
            transaction_type: TransactionType::Unstake,
            from: from.clone(),
            to: from, // Unstaking is done from the address itself
            amount,
            fee,
            nonce,
            timestamp: chrono::Utc::now().timestamp_millis() as u64,
            data: Vec::new(),
            signature: Vec::new(),
            zk_proof: None,
            input_commitments: None,
            output_commitments: None,
            range_proofs: None,
            binding_signature: None,
        }
    }
    
    /// Creates a governance voting transaction
    pub fn new_vote(
        from: Address,
        proposal_id: u64,
        vote_data: Vec<u8>, // Voting data (yes/no/amount)
        fee: u64,
        nonce: u64,
    ) -> Self {
        let mut data = proposal_id.to_be_bytes().to_vec();
        data.extend(vote_data);
        
        Transaction {
            version: 1,
            transaction_type: TransactionType::Vote,
            from: from.clone(),
            // Votes usually go to a governance contract
            to: from, // Placeholder: Vote is from the address itself
            amount: 0,
            fee,
            nonce,
            timestamp: chrono::Utc::now().timestamp_millis() as u64,
            data,
            signature: Vec::new(),
            zk_proof: None,
            input_commitments: None,
            output_commitments: None,
            range_proofs: None,
            binding_signature: None,
        }
    }
    
    pub fn hash(&self) -> Hash {
        // For hashing, we exclude temporary fields such as signature
        let tx_data = bincode::serialize(&TransactionForHashing {
            version: self.version,
            transaction_type: self.transaction_type.clone(),
            from: self.from.clone(),
            to: self.to.clone(),
            amount: self.amount,
            fee: self.fee,
            nonce: self.nonce,
            timestamp: self.timestamp,
            data: self.data.clone(),
            zk_proof: self.zk_proof.clone(),
            input_commitments: self.input_commitments.clone(),
            output_commitments: self.output_commitments.clone(),
            range_proofs: self.range_proofs.clone(),
            binding_signature: self.binding_signature.clone(),
        }).unwrap();
        
        Hash::new(&tx_data)
    }
    
    pub fn sign(&mut self, private_key: &[u8]) -> Result<()> {
        use crate::crypto::dilithium::Dilithium;
        
        let tx_hash = self.hash();
        self.signature = Dilithium::sign(private_key, tx_hash.as_bytes())?;
        Ok(())
    }
    
    pub fn verify_signature(&self, public_key: &[u8]) -> Result<bool> {
        use crate::crypto::dilithium::Dilithium;
        
        let tx_hash = self.hash();
        Dilithium::verify(public_key, tx_hash.as_bytes(), &self.signature)
    }
    
    pub fn is_private(&self) -> bool {
        self.zk_proof.is_some() || self.transaction_type == TransactionType::ConfidentialTransfer
    }
    
    pub fn is_confidential(&self) -> bool {
        self.transaction_type == TransactionType::ConfidentialTransfer
    }
    
    /// Returns proposal data for voting transactions
    pub fn get_vote_data(&self) -> Option<(u64, Vec<u8>)> {
        if self.transaction_type != TransactionType::Vote {
            return None;
        }
        
        if self.data.len() < 8 {
            return None;
        }
        
        let proposal_id = u64::from_be_bytes([
            self.data[0], self.data[1], self.data[2], self.data[3],
            self.data[4], self.data[5], self.data[6], self.data[7],
        ]);
        
        let vote_data = self.data[8..].to_vec();
        
        Some((proposal_id, vote_data))
    }
    
    /// Validates the basic structure of the transaction
    pub fn validate_basic(&self) -> Result<()> {
        if self.fee == 0 {
            return Err(BlockchainError::InvalidTransaction("Zero fee".to_string()));
        }
        
        if self.timestamp > chrono::Utc::now().timestamp_millis() as u64 + 300000 { // 5 minutes in the future at most
            return Err(BlockchainError::InvalidTransaction("Timestamp too far in future".to_string()));
        }
        
        // Specific validations by transaction type
        match self.transaction_type {
            TransactionType::ConfidentialTransfer => {
                if self.zk_proof.is_none() {
                    return Err(BlockchainError::InvalidTransaction(
                        "Confidential transfer requires ZK proof".to_string()
                    ));
                }
                if self.input_commitments.is_none() || self.output_commitments.is_none() {
                    return Err(BlockchainError::InvalidTransaction(
                        "Confidential transfer requires commitments".to_string()
                    ));
                }
            }
            TransactionType::Transfer => {
                // Allow 0-value transfers (e.g., for contract interaction)
            }
            TransactionType::ContractDeployment => {
                if self.data.is_empty() {
                    return Err(BlockchainError::InvalidTransaction(
                        "Contract deployment requires code".to_string()
                    ));
                }
            }
            TransactionType::Stake | TransactionType::Unstake => {
                if self.amount == 0 {
                    return Err(BlockchainError::InvalidTransaction("Zero stake amount".to_string()));
                }
            }
            TransactionType::Vote => {
                if self.data.len() < 8 {
                    return Err(BlockchainError::InvalidTransaction(
                        "Vote transaction requires proposal ID and vote data".to_string()
                    ));
                }
            }
            _ => {}
        }
        
        Ok(())
    }
}

// Auxiliary structure for hashing (excludes signature)
#[derive(Serialize)]
struct TransactionForHashing {
    version: u32,
    transaction_type: TransactionType,
    from: Address,
    to: Address,
    amount: u64,
    fee: u64,
    nonce: u64,
    timestamp: u64,
    data: Vec<u8>,
    zk_proof: Option<Vec<u8>>,
    input_commitments: Option<Vec<Vec<u8>>>,
    output_commitments: Option<Vec<Vec<u8>>>,
    range_proofs: Option<Vec<Vec<u8>>>,
    binding_signature: Option<Vec<u8>>,
}