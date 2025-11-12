// src/bridges/zk_proofs/types.rs
use serde::{Deserialize, Serialize};

/// Types of ZK proofs supported for bridge operations
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ZkProofType {
    /// Merkle inclusion proof for transaction verification
    MerkleInclusion,
    /// State proof for cross-chain state verification
    StateProof,
    /// Transfer validity proof for asset transfers
    TransferValidity,
    /// Bridge operation proof for complex operations
    BridgeOperation,
}

/// ZK proof for bridge operations
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BridgeZkProof {
    pub proof_type: ZkProofType,
    pub proof_data: Vec<u8>,
    pub public_inputs: Vec<Vec<u8>>,
    pub verification_key: Vec<u8>,
    pub circuit_id: String,
}

/// Verification result for ZK proofs
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ZkVerificationResult {
    pub is_valid: bool,
    pub verification_time: u64,
    pub circuit_checks: Vec<CircuitCheck>,
    pub errors: Vec<String>,
}

/// Individual circuit check result
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CircuitCheck {
    pub check_name: String,
    pub passed: bool,
    pub details: String,
}

/// Circuit parameters for proof generation
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CircuitParams {
    pub circuit_id: String,
    pub constraints: u32,
    pub variables: u32,
    pub public_inputs: u32,
    pub private_inputs: u32,
}

/// Witness data for proof generation
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WitnessData {
    pub public_inputs: Vec<Vec<u8>>,
    pub private_inputs: Vec<Vec<u8>>,
    pub circuit_params: CircuitParams,
}

/// ZK proof error types
#[derive(Debug, thiserror::Error)]
pub enum ZkProofError {
    #[error("Proof generation failed: {0}")]
    ProofGenerationFailed(String),
    #[error("Proof verification failed: {0}")]
    ProofVerificationFailed(String),
    #[error("Invalid circuit parameters: {0}")]
    InvalidCircuitParameters(String),
    #[error("Witness data invalid: {0}")]
    InvalidWitnessData(String),
    #[error("Verification key invalid: {0}")]
    InvalidVerificationKey(String),
    #[error("Circuit not supported: {0}")]
    CircuitNotSupported(String),
    #[error("Serialization error: {0}")]
    SerializationError(String),
}