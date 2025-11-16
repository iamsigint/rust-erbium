// src/core/mod.rs

pub mod block;
pub mod chain;
pub mod dex;
pub mod erbium_engine;
pub mod layer2;
pub mod mempool;
pub mod precompiles;
pub mod state;
pub mod transaction;
pub mod transaction_templates;
pub mod types;
pub mod units;
pub mod vm;

// Re-export commonly used types and functions
pub use block::Block;
pub use chain::Blockchain;
pub use state::State;
pub use transaction::{Transaction, TransactionType};
pub use types::{Address, Difficulty, Hash, Nonce, Timestamp};

use crate::utils::error::{BlockchainError, Result};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Core blockchain configuration
#[derive(Debug, Clone)]
pub struct BlockchainConfig {
    pub block_time: u64,       // Target block time in seconds
    pub max_block_size: usize, // Maximum block size in bytes
    pub max_transactions_per_block: usize,
    pub genesis_timestamp: u64, // Genesis block timestamp
    pub chain_id: u64,          // Network chain ID
}

impl Default for BlockchainConfig {
    fn default() -> Self {
        Self {
            block_time: 30,                  // 30 seconds per block
            max_block_size: 8 * 1024 * 1024, // 8MB
            max_transactions_per_block: 10000,
            genesis_timestamp: 1635724800000, // Example timestamp
            chain_id: 137,                    // Erbium mainnet chain ID
        }
    }
}

/// Blockchain status information
#[derive(Debug, Clone)]
pub struct BlockchainStatus {
    pub block_height: u64,
    pub latest_block_hash: Hash,
    pub total_transactions: u64,
    pub total_validators: usize,
    pub current_difficulty: u64,
    pub sync_status: SyncStatus,
    pub network_hashrate: f64,
}

#[derive(Debug, Clone)]
pub enum SyncStatus {
    Synced,
    Syncing(u64), // Current block height being synced
    Behind(u64),  // Blocks behind
}

/// Utility functions for core blockchain operations
pub fn validate_address(address: &Address) -> bool {
    // Basic address validation
    let addr_str = address.as_str();
    if addr_str.starts_with("0x") && addr_str.len() == 42 {
        // Ethereum-style address
        hex::decode(&addr_str[2..]).is_ok()
    } else {
        // Other address formats could be supported here
        false
    }
}

/// Calculate transaction fee based on gas and gas price
pub fn calculate_transaction_fee(gas_used: u64, gas_price: u64) -> u64 {
    gas_used.saturating_mul(gas_price)
}

/// Validate transaction basic structure
/// This performs comprehensive validation before signature verification
pub fn validate_transaction_structure(transaction: &Transaction) -> Result<()> {
    // 1. Validate addresses are not empty and have correct format
    if transaction.from.as_str().is_empty() {
        return Err(BlockchainError::InvalidTransaction(
            "Sender address cannot be empty".to_string(),
        ));
    }
    
    if transaction.to.as_str().is_empty() {
        return Err(BlockchainError::InvalidTransaction(
            "Recipient address cannot be empty".to_string(),
        ));
    }

    if !validate_address(&transaction.from) {
        return Err(BlockchainError::InvalidTransaction(
            format!("Invalid sender address format: {}", transaction.from.as_str()),
        ));
    }

    if !validate_address(&transaction.to) {
        return Err(BlockchainError::InvalidTransaction(
            format!("Invalid recipient address format: {}", transaction.to.as_str()),
        ));
    }

    // 2. Validate amounts don't overflow
    if transaction.amount.checked_add(transaction.fee).is_none() {
        return Err(BlockchainError::InvalidTransaction(
            "Amount + fee would overflow".to_string(),
        ));
    }

    // 3. Validate fee is non-zero (except for genesis transactions)
    if transaction.fee == 0 && !transaction.from.is_zero() {
        return Err(BlockchainError::InvalidTransaction(
            "Zero fee not allowed (except genesis)".to_string(),
        ));
    }

    // 4. Validate timestamp is reasonable (not too far in future)
    let now = chrono::Utc::now().timestamp_millis() as u64;
    if transaction.timestamp > now + 300_000 {
        // 5 minutes in future maximum
        return Err(BlockchainError::InvalidTransaction(
            format!("Timestamp too far in future: {} vs now {}", 
                    transaction.timestamp, now),
        ));
    }

    // 5. Validate timestamp is not too old (24 hours)
    if transaction.timestamp + 86_400_000 < now {
        return Err(BlockchainError::InvalidTransaction(
            "Transaction too old (>24 hours)".to_string(),
        ));
    }

    // 6. Validate transaction type specific fields
    match transaction.transaction_type {
        TransactionType::Transfer => {
            if transaction.amount == 0 {
                return Err(BlockchainError::InvalidTransaction(
                    "Transfer amount cannot be zero".to_string(),
                ));
            }
        }
        TransactionType::ContractCall => {
            if transaction.data.is_empty() {
                return Err(BlockchainError::InvalidTransaction(
                    "Contract call must have data".to_string(),
                ));
            }
        }
        TransactionType::ContractDeployment => {
            if transaction.data.is_empty() {
                return Err(BlockchainError::InvalidTransaction(
                    "Contract deploy must have bytecode".to_string(),
                ));
            }
        }
        TransactionType::ConfidentialTransfer => {
            if transaction.zk_proof.is_none() {
                log::warn!("Confidential transaction without proof - this should be validated");
                return Err(BlockchainError::InvalidTransaction(
                    "Confidential transaction must include zero-knowledge proof".to_string(),
                ));
            }
        }
        TransactionType::Stake | TransactionType::Unstake => {
            if transaction.amount == 0 {
                return Err(BlockchainError::InvalidTransaction(
                    "Stake amount cannot be zero".to_string(),
                ));
            }
        }
        _ => {} // Other types don't have specific requirements
    }

    // 7. Validate signature exists (will be verified separately)
    if transaction.signature.is_empty() && !transaction.from.is_zero() {
        return Err(BlockchainError::InvalidTransaction(
            "Transaction signature is missing (required for non-genesis)".to_string(),
        ));
    }

    Ok(())
}

/// Calculate block reward based on block height and configuration
pub fn calculate_block_reward(block_height: u64, config: &BlockchainConfig) -> u64 {
    // Simple block reward calculation
    // In real implementation, this would follow the emission schedule
    let base_reward = 10_000_000; // 10 ERB base reward

    // Halving every 4 years (assuming 30s blocks)
    let halving_interval = 4 * 365 * 24 * 60 * 60 / config.block_time;
    let halvings = block_height / halving_interval;

    base_reward / 2u64.pow(halvings as u32)
}

/// Calculate next block difficulty based on current difficulty and block times
pub fn calculate_next_difficulty(
    current_difficulty: u64,
    recent_block_times: &[u64],
    target_block_time: u64,
) -> u64 {
    if recent_block_times.len() < 2 {
        return current_difficulty;
    }

    let total_time: u64 = recent_block_times
        .windows(2)
        .map(|window| window[1] - window[0])
        .sum();

    let average_block_time = total_time / (recent_block_times.len() - 1) as u64;

    if average_block_time < target_block_time {
        // Blocks are coming too fast, increase difficulty
        current_difficulty * target_block_time / average_block_time
    } else {
        // Blocks are coming too slow, decrease difficulty
        current_difficulty * average_block_time / target_block_time
    }
}

/// Multi-Layer Transaction Architecture (Q1 2025 Implementation)
/// Adaptação inovadora do SegWit - Witness Isolation System

/// Core transaction data (inputs, outputs, amount, nonce)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CoreTransactionData {
    pub from: Address,
    pub to: Address,
    pub amount: u64,
    pub nonce: u64,
    pub data: Vec<u8>, // Contract call data
    pub gas_limit: u64,
    pub gas_price: u64,
}

/// Witness layer with advanced cryptography
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WitnessLayer {
    pub signatures: Vec<crate::crypto::signatures::Signature>,
    pub zk_proofs: Vec<crate::crypto::ZkProof>,
    pub quantum_proofs: Vec<crate::crypto::QuantumProof>,
    pub validation_hash: Hash, // Prevents malleability
}

impl WitnessLayer {
    pub fn new() -> Self {
        Self {
            signatures: Vec::new(),
            zk_proofs: Vec::new(),
            quantum_proofs: Vec::new(),
            validation_hash: Hash::zero(),
        }
    }

    pub fn add_signature(&mut self, signature: crate::crypto::Signature) {
        self.signatures.push(signature);
    }

    pub fn add_zk_proof(&mut self, proof: crate::crypto::ZkProof) {
        self.zk_proofs.push(proof);
    }

    pub fn calculate_validation_hash(&mut self) -> &Hash {
        let mut data = Vec::new();

        // Include all signatures in hash calculation
        for sig in &self.signatures {
            data.extend_from_slice(&sig.data);
        }

        // Include ZK proof commitments
        for proof in &self.zk_proofs {
            data.extend_from_slice(&proof.commitment);
        }

        // Include quantum proof data
        for qproof in &self.quantum_proofs {
            data.extend_from_slice(&qproof.challenge);
            data.extend_from_slice(&qproof.response);
        }

        self.validation_hash = Hash::new(&data);
        &self.validation_hash
    }
}

/// Metadata layer for network and cross-chain data
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MetadataLayer {
    pub timestamp: u64,
    pub network_congestion: f64, // 0.0 - 1.0
    pub cross_chain_proofs: Vec<CrossChainProof>,
    pub layer2_commits: Vec<Layer2Commit>,
    pub gas_calculations: GasCalculation,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CrossChainProof {
    pub target_chain: String,
    pub proof_data: Vec<u8>,
    pub validator_count: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Layer2Commit {
    pub channel_id: Hash,
    pub sequence: u64,
    pub state_hash: Hash,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GasCalculation {
    pub computational_gas: u64,
    pub storage_gas: u64,
    pub cross_chain_gas: u64,
    pub layer2_gas: u64,
    pub quantum_gas: u64, // Premium for quantum operations
}

/// Extension layer for future-proofing
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExtensionLayer {
    pub protocol_version: u32,
    pub custom_fields: HashMap<String, Vec<u8>>,
    pub ai_predictions: Vec<AiPrediction>,
    pub temporal_data: Vec<TemporalCommitment>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AiPrediction {
    pub model_hash: Hash,
    pub prediction_data: Vec<u8>,
    pub confidence_score: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TemporalCommitment {
    pub timestamp: u64,
    pub commitment_hash: Hash,
    pub quantum_randomness: Vec<u8>,
}

/// Enhanced Multi-Layer Transaction (Erbium Innovation)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MultiLayerTransaction {
    /// Dados base (inputs, outputs, nonce) - Adaptado SegWit
    pub core_data: CoreTransactionData,
    /// Witness isolado (assinaturas + proofs) - SegWit Isolation
    pub witness_layer: WitnessLayer,
    /// Metadata layer (gas calculations, cross-chain proofs)
    pub metadata_layer: MetadataLayer,
    /// Extension layer (future-proof custom data)
    pub extension_layer: ExtensionLayer,

    /// Transaction hash cache
    pub hash: Hash,
    /// Transaction size cache
    pub size: u64,
}

impl MultiLayerTransaction {
    pub fn new(from: Address, to: Address, amount: u64) -> Self {
        Self {
            core_data: CoreTransactionData {
                from,
                to,
                amount,
                nonce: 0,
                data: Vec::new(),
                gas_limit: 10_000_000,
                gas_price: 1,
            },
            witness_layer: WitnessLayer::new(),
            metadata_layer: MetadataLayer {
                timestamp: current_timestamp(),
                network_congestion: 0.0,
                cross_chain_proofs: Vec::new(),
                layer2_commits: Vec::new(),
                gas_calculations: GasCalculation {
                    computational_gas: 0,
                    storage_gas: 0,
                    cross_chain_gas: 0,
                    layer2_gas: 0,
                    quantum_gas: 0,
                },
            },
            extension_layer: ExtensionLayer {
                protocol_version: 2, // Version 2.0 for multi-layer
                custom_fields: HashMap::new(),
                ai_predictions: Vec::new(),
                temporal_data: Vec::new(),
            },
            hash: Hash::zero(),
            size: 0,
        }
    }

    /// Calculate transaction hash with multi-layer validation
    pub fn calculate_hash(&mut self) -> &Hash {
        let mut data = Vec::new();

        // Include core data
        if let Ok(from_bytes) = self.core_data.from.to_bytes() {
            data.extend_from_slice(&from_bytes);
        }
        if let Ok(to_bytes) = self.core_data.to.to_bytes() {
            data.extend_from_slice(&to_bytes);
        }
        data.extend_from_slice(&self.core_data.amount.to_be_bytes());
        data.extend_from_slice(&self.core_data.nonce.to_be_bytes());
        data.extend_from_slice(&self.core_data.data);

        // Include witness validation hash (SegWit-style separation)
        let witness_hash = self.witness_layer.calculate_validation_hash();
        data.extend_from_slice(witness_hash.as_bytes());

        // Include metadata commitment
        let metadata_hash = self.metadata_layer.calculate_hash();
        data.extend_from_slice(metadata_hash.as_bytes());

        // Include extension commitment
        let extension_hash = self.extension_layer.calculate_hash();
        data.extend_from_slice(extension_hash.as_bytes());

        self.hash = Hash::new(&data);
        &self.hash
    }

    /// Verify transaction integrity across all layers
    pub fn verify_integrity(&mut self) -> bool {
        // TODO: Implement full verification logic
        // For now, basic structural validation
        !self.witness_layer.signatures.is_empty()
            || !self.metadata_layer.cross_chain_proofs.is_empty()
            || !self.extension_layer.ai_predictions.is_empty()
    }

    /// Add cross-chain proof (innovation layer)
    pub fn add_cross_chain_proof(&mut self, chain: String, proof: Vec<u8>, validators: u32) {
        self.metadata_layer
            .cross_chain_proofs
            .push(CrossChainProof {
                target_chain: chain,
                proof_data: proof,
                validator_count: validators,
            });
    }

    /// Add Layer-2 commit (innovation layer)
    pub fn add_layer2_commit(&mut self, channel_id: Hash, sequence: u64, state_hash: Hash) {
        self.metadata_layer.layer2_commits.push(Layer2Commit {
            channel_id,
            sequence,
            state_hash,
        });
    }

    /// Calculate size with layer optimization
    pub fn calculate_size(&mut self) -> u64 {
        let core_size = std::mem::size_of::<CoreTransactionData>() as u64;
        let witness_size = self.witness_layer.size();
        let metadata_size = self.metadata_layer.size();
        let extension_size = self.extension_layer.size();

        self.size = core_size + witness_size + metadata_size + extension_size;
        self.size
    }
}

impl WitnessLayer {
    pub fn size(&self) -> u64 {
        let mut size = std::mem::size_of::<Vec<crate::crypto::Signature>>() as u64;
        size +=
            self.signatures.len() as u64 * std::mem::size_of::<crate::crypto::Signature>() as u64;
        size += self.zk_proofs.len() as u64 * 64; // Estimated
        size += self.quantum_proofs.len() as u64 * 128; // Estimated
        size += 32; // validation_hash
        size
    }
}

impl MetadataLayer {
    pub fn calculate_hash(&self) -> Hash {
        let mut data = Vec::new();
        data.extend_from_slice(&self.timestamp.to_be_bytes());
        data.extend_from_slice(&self.network_congestion.to_be_bytes());
        // Include all proofs and commits
        for proof in &self.cross_chain_proofs {
            data.extend_from_slice(proof.target_chain.as_bytes());
            data.extend_from_slice(&proof.proof_data);
        }
        for commit in &self.layer2_commits {
            data.extend_from_slice(commit.channel_id.as_bytes());
            data.extend_from_slice(&commit.sequence.to_be_bytes());
        }
        Hash::new(&data)
    }

    pub fn size(&self) -> u64 {
        let mut size = 16 + 8; // timestamp + congestion
        size += self.cross_chain_proofs.len() as u64 * 256; // Estimated
        size += self.layer2_commits.len() as u64 * 96; // Estimated
        size += 40; // gas_calculations
        size
    }
}

impl ExtensionLayer {
    pub fn calculate_hash(&self) -> Hash {
        let mut data = Vec::new();
        data.extend_from_slice(&self.protocol_version.to_be_bytes());
        // Include custom fields, AI predictions, temporal data
        for (key, value) in &self.custom_fields {
            data.extend_from_slice(key.as_bytes());
            data.extend_from_slice(value);
        }
        for prediction in &self.ai_predictions {
            data.extend_from_slice(prediction.model_hash.as_bytes());
        }
        for temporal in &self.temporal_data {
            data.extend_from_slice(&temporal.timestamp.to_be_bytes());
        }
        Hash::new(&data)
    }

    pub fn size(&self) -> u64 {
        let mut size = 4; // protocol_version
        size += self.custom_fields.len() as u64 * 128; // Estimated
        size += self.ai_predictions.len() as u64 * 96; // Estimated
        size += self.temporal_data.len() as u64 * 64; // Estimated
        size
    }
}

/// Advanced Quantum-Safe Accumulators (Q1 2025 Implementation)
/// Inovação baseada em Merkle Mountain Ranges mas adaptada para Erbium

/// Quantum-Safe Accumulator for efficient set commitments
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct QuantumAccumulator {
    /// Classical MMR structure
    pub mmr: MerkleMountainRange,
    /// Quantum resistance commitments
    pub quantum_commitments: Vec<QuantumCommitment>,
    /// Zero-knowledge inclusion proofs
    pub zk_inclusion_proofs: Vec<ZkInclusionProof>,
    /// Temporal commitments for historical proofs
    pub temporal_commitments: Vec<TemporalCommitment>,
}

impl QuantumAccumulator {
    pub fn new() -> Self {
        Self {
            mmr: MerkleMountainRange::new(),
            quantum_commitments: Vec::new(),
            zk_inclusion_proofs: Vec::new(),
            temporal_commitments: Vec::new(),
        }
    }

    /// Add a new element with quantum-safe commitment
    pub fn add_element(&mut self, element: Hash, timestamp: u64) -> Result<()> {
        self.mmr.append(element)?;

        // Add quantum commitment (placeholder for actual quantum algorithm)
        let quantum_commit = QuantumCommitment {
            element_hash: element,
            quantum_proof: vec![0; 128], // Placeholder 1KB quantum proof
            timestamp,
        };
        self.quantum_commitments.push(quantum_commit);

        // Add temporal commitment for blockchain finality tracking
        let temporal_commit = TemporalCommitment {
            timestamp,
            commitment_hash: element,
            quantum_randomness: vec![0; 32], // Placeholder 32-byte entropy
        };
        self.temporal_commitments.push(temporal_commit);

        Ok(())
    }

    /// Generate non-existence proof (element is not in the set)
    pub fn prove_non_existence(&self, element: &[u8], timestamp: u64) -> Option<NonExistenceProof> {
        // Find if element exists in current state
        let target_hash = Hash::new(element);

        for commitment in &self.quantum_commitments {
            if commitment.element_hash == target_hash && commitment.timestamp <= timestamp {
                return None; // Element exists, cannot prove non-existence
            }
        }

        // Generate non-existence proof
        Some(NonExistenceProof {
            target_hash,
            timestamp,
            proof_data: vec![], // Placeholder - would contain MMR proof
            quantum_verification: vec![0; 64],
        })
    }

    /// Generate inclusion proof (element is in the set)
    pub fn prove_inclusion(&self, element: &[u8], leaf_index: usize) -> Option<InclusionProof> {
        let proof = self.mmr.generate_proof(leaf_index)?;

        Some(InclusionProof {
            element_hash: Hash::new(element),
            leaf_index,
            merkle_proof: proof,
            quantum_verification: vec![0; 128],
            timestamp: current_timestamp(),
        })
    }

    /// Verify inclusion proof with quantum safety
    pub fn verify_inclusion(&self, proof: &InclusionProof) -> bool {
        // Basic Merkle verification
        let root = self.mmr.get_root();
        if !MerkleTree::verify_proof(
            &proof.element_hash,
            &proof.merkle_proof,
            &root,
            proof.leaf_index,
        ) {
            return false;
        }

        // Quantum verification (placeholder)
        // In production, this would verify quantum-resistant signatures
        proof.quantum_verification.len() >= 64
    }

    /// Verify non-existence proof
    pub fn verify_non_existence(&self, proof: &NonExistenceProof) -> bool {
        // Check if element exists in the commitment set
        for commitment in &self.quantum_commitments {
            if commitment.element_hash == proof.target_hash
                && commitment.timestamp <= proof.timestamp
            {
                return false; // Element exists, proof invalid
            }
        }

        // Verify quantum aspects (placeholder)
        proof.quantum_verification.len() >= 32
    }

    /// Generate temporal inclusion proof (element existed at specific time)
    pub fn prove_temporal_inclusion(
        &self,
        element: &[u8],
        timestamp: u64,
    ) -> Option<TemporalProof> {
        let target_hash = Hash::new(element);

        let mut temporal_chain = Vec::new();

        // Find all temporal commitments for this element
        for commit in &self.temporal_commitments {
            if commit.timestamp <= timestamp {
                temporal_chain.push(commit.clone());
            }
        }

        if temporal_chain.is_empty() {
            return None;
        }

        Some(TemporalProof {
            element_hash: target_hash,
            timestamp,
            temporal_chain,
            merkle_root: self.mmr.get_root(),
            quantum_timestamp_proof: vec![0; 256], // Placeholder
        })
    }

    /// Verify temporal proof
    pub fn verify_temporal_inclusion(&self, proof: &TemporalProof) -> bool {
        // Verify merkle aspects
        if proof.merkle_root != self.mmr.get_root() {
            return false;
        }

        // Verify temporal chain consistency
        for i in 1..proof.temporal_chain.len() {
            if proof.temporal_chain[i - 1].timestamp >= proof.temporal_chain[i].timestamp {
                return false; // Timestamps must be ordered
            }
        }

        // Quantum temporal verification (placeholder)
        proof.quantum_timestamp_proof.len() >= 128
    }

    /// Create ZK inclusion proof for privacy-preserving verification
    pub fn create_zk_inclusion_proof(
        &self,
        _element: &[u8],
        leaf_index: usize,
    ) -> Option<ZkInclusionProof> {
        if leaf_index >= self.mmr.leaves.len() {
            return None;
        }

        // Generate ZK proof that element exists without revealing it
        Some(ZkInclusionProof {
            public_commitment: vec![0; 32], // Hash commitment
            zk_proof_data: vec![0; 512],    // ZK proof (placeholder)
            verification_key: vec![0; 64],  // Verification key
            leaf_index,
        })
    }

    /// Verify ZK inclusion proof
    pub fn verify_zk_inclusion(&self, proof: &ZkInclusionProof) -> bool {
        // Placeholder ZK verification
        // In production: Use Bulletproofs, Groth16, or similar
        proof.zk_proof_data.len() >= 256 && proof.verification_key.len() >= 32
    }
}

/// Merkle Mountain Range for accumulator efficiency
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MerkleMountainRange {
    /// All leaves (UTXOs, transactions, etc.)
    pub leaves: Vec<Hash>,
    /// Merkle root for current state
    pub root: Hash,
    /// Size for efficient appending
    pub size: usize,
}

impl MerkleMountainRange {
    pub fn new() -> Self {
        Self {
            leaves: Vec::new(),
            root: Hash::zero(),
            size: 0,
        }
    }

    /// Append new hash to MMR
    pub fn append(&mut self, hash: Hash) -> Result<usize> {
        self.leaves.push(hash);
        self.size = self.leaves.len();
        self.root = MerkleTree::calculate_root(&self.leaves);

        Ok(self.size - 1) // Return index of new leaf
    }

    /// Get current root
    pub fn get_root(&self) -> Hash {
        if self.leaves.is_empty() {
            Hash::zero()
        } else {
            MerkleTree::calculate_root(&self.leaves)
        }
    }

    /// Generate inclusion proof for leaf at index
    pub fn generate_proof(&self, leaf_index: usize) -> Option<Vec<Hash>> {
        if leaf_index >= self.leaves.len() {
            return None;
        }
        MerkleTree::generate_proof(&self.leaves, leaf_index)
    }
}

/// Quantum commitment for future-proof cryptography
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct QuantumCommitment {
    pub element_hash: Hash,     // Classical hash
    pub quantum_proof: Vec<u8>, // Quantum-resistant proof data
    pub timestamp: u64,         // Timestamp commitment
}

/// Zero-knowledge inclusion proof
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ZkInclusionProof {
    pub public_commitment: Vec<u8>, // Public commitment without revealing data
    pub zk_proof_data: Vec<u8>,     // Actual ZK proof
    pub verification_key: Vec<u8>,  // Key to verify proof
    pub leaf_index: usize,          // Index in the accumulator
}

/// Proof that an element does NOT exist in the set
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NonExistenceProof {
    pub target_hash: Hash,             // Hash we're proving doesn't exist
    pub timestamp: u64,                // Timestamp for temporal verification
    pub proof_data: Vec<u8>,           // Merkle proof data
    pub quantum_verification: Vec<u8>, // Quantum-resistant verification
}

/// Proof that an element exists in the set
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InclusionProof {
    pub element_hash: Hash,            // Hash being proven
    pub leaf_index: usize,             // Index in the tree
    pub merkle_proof: Vec<Hash>,       // Standard merkle proof
    pub quantum_verification: Vec<u8>, // Quantum-resistant verification
    pub timestamp: u64,                // Timestamp of inclusion
}

/// Temporal proof for time-sensitive verification
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TemporalProof {
    pub element_hash: Hash,
    pub timestamp: u64,
    pub temporal_chain: Vec<TemporalCommitment>,
    pub merkle_root: Hash,
    pub quantum_timestamp_proof: Vec<u8>,
}

/// Merkle tree utilities (legacy - enhanced by accumulator)
pub struct MerkleTree;

impl MerkleTree {
    /// Calculate Merkle root from a list of hashes
    pub fn calculate_root(hashes: &[Hash]) -> Hash {
        if hashes.is_empty() {
            return Hash::new(b"empty");
        }

        if hashes.len() == 1 {
            return hashes[0];
        }

        let mut current_level = hashes.to_vec();

        while current_level.len() > 1 {
            let mut next_level = Vec::new();

            for chunk in current_level.chunks(2) {
                if chunk.len() == 2 {
                    // CORRIGIDO: Use chain e collect em vez de concat
                    let combined: Vec<u8> = chunk[0]
                        .as_bytes()
                        .iter()
                        .chain(chunk[1].as_bytes().iter())
                        .cloned()
                        .collect();
                    next_level.push(Hash::new(&combined));
                } else {
                    next_level.push(chunk[0]);
                }
            }

            current_level = next_level;
        }

        current_level[0]
    }

    /// Generate Merkle proof for a specific leaf
    pub fn generate_proof(hashes: &[Hash], leaf_index: usize) -> Option<Vec<Hash>> {
        if leaf_index >= hashes.len() {
            return None;
        }

        let mut proof = Vec::new();
        let mut current_index = leaf_index;
        let mut current_level = hashes.to_vec();

        while current_level.len() > 1 {
            if current_index.is_multiple_of(2) {
                // Current is left child, right sibling exists
                if current_index + 1 < current_level.len() {
                    proof.push(current_level[current_index + 1]);
                }
            } else {
                // Current is right child, left sibling exists
                proof.push(current_level[current_index - 1]);
            }

            // Move to next level
            current_index /= 2;
            current_level = Self::build_next_level(&current_level);
        }

        Some(proof)
    }

    /// Verify Merkle proof
    pub fn verify_proof(leaf: &Hash, proof: &[Hash], root: &Hash, index: usize) -> bool {
        let mut computed_hash = *leaf;
        let mut current_index = index;

        for proof_hash in proof {
            if current_index.is_multiple_of(2) {
                // Current is left child
                // CORRIGIDO: Use chain e collect em vez de concat
                let combined: Vec<u8> = computed_hash
                    .as_bytes()
                    .iter()
                    .chain(proof_hash.as_bytes().iter())
                    .cloned()
                    .collect();
                computed_hash = Hash::new(&combined);
            } else {
                // Current is right child
                // CORRIGIDO: Use chain e collect em vez de concat
                let combined: Vec<u8> = proof_hash
                    .as_bytes()
                    .iter()
                    .chain(computed_hash.as_bytes().iter())
                    .cloned()
                    .collect();
                computed_hash = Hash::new(&combined);
            }
            current_index /= 2;
        }

        &computed_hash == root
    }

    fn build_next_level(level: &[Hash]) -> Vec<Hash> {
        let mut next_level = Vec::new();

        for chunk in level.chunks(2) {
            if chunk.len() == 2 {
                // CORRIGIDO: Use chain e collect em vez de concat
                let combined: Vec<u8> = chunk[0]
                    .as_bytes()
                    .iter()
                    .chain(chunk[1].as_bytes().iter())
                    .cloned()
                    .collect();
                next_level.push(Hash::new(&combined));
            } else {
                next_level.push(chunk[0]);
            }
        }

        next_level
    }
}

fn current_timestamp() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_secs()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::core::types::AddressError;

    #[test]
    fn test_merkle_tree() {
        let hashes = vec![
            Hash::new(b"hash1"),
            Hash::new(b"hash2"),
            Hash::new(b"hash3"),
        ];

        let root = MerkleTree::calculate_root(&hashes);
        assert_ne!(root, Hash::new(b"empty"));

        let proof = MerkleTree::generate_proof(&hashes, 0).unwrap();
        assert!(!proof.is_empty());

        let is_valid = MerkleTree::verify_proof(&hashes[0], &proof, &root, 0);
        assert!(is_valid);
    }

    #[test]
    fn test_block_reward_calculation() {
        let config = BlockchainConfig::default();
        let reward = calculate_block_reward(0, &config);
        assert_eq!(reward, 10_000_000);
    }

    #[test]
    fn test_address_validation() -> std::result::Result<(), AddressError> {
        // Address::new returns a Result, so we test the result
        let valid_address = Address::new("0x0000000000000000000000000000000000000000".to_string())?;
        let invalid_address_result = Address::new("invalid".to_string());

        assert!(validate_address(&valid_address));
        assert!(invalid_address_result.is_err());

        // Test the validation logic for an address that parses but might be "invalid"
        // In this case, our validation just checks parsing, so an invalid_address that parses
        // is not possible with the current Address::new implementation.
        // We can test the logic directly:
        let invalid_format_address = Address::new_unchecked("invalid".to_string());
        assert!(!validate_address(&invalid_format_address));

        Ok(())
    }
}
