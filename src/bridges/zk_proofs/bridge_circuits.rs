// src/bridges/zk_proofs/bridge_circuits.rs
use crate::crypto::zk_proofs::ZkProof;
use super::types::{BridgeZkProof, ZkProofError, CircuitParams};

/// Main bridge circuit for complex cross-chain operations
pub struct BridgeCircuit {
    circuit_id: String,
    constraints: u32,
}

/// Transfer validity circuit for asset transfers
pub struct TransferCircuit {
    circuit_id: String,
    max_amount: u64,
    supported_tokens: Vec<String>,
}

/// Validity circuit for general bridge operations
pub struct ValidityCircuit {
    circuit_id: String,
    operation_types: Vec<String>,
}

impl BridgeCircuit {
    /// Create a new bridge circuit
    pub fn new(circuit_id: &str) -> Self {
        Self {
            circuit_id: circuit_id.to_string(),
            constraints: 5000, // Complex circuit with many constraints
        }
    }

    /// Generate proof for bridge operation
    pub fn generate_bridge_proof(
        &self,
        operation_data: &BridgeOperationData,
    ) -> Result<BridgeZkProof, ZkProofError> {
        let witness_data = self.create_bridge_witness(operation_data)?;
        
        let zk_proof = ZkProof::generate_proof(&witness_data.public_inputs, &witness_data.private_inputs)
            .map_err(|e| ZkProofError::ProofGenerationFailed(e.to_string()))?;

        let verification_key = zk_proof.verification_key();
        Ok(BridgeZkProof {
            proof_type: super::types::ZkProofType::BridgeOperation,
            proof_data: zk_proof.proof_data,
            public_inputs: witness_data.public_inputs,
            verification_key,
            circuit_id: self.circuit_id.clone(),
        })
    }

    /// Create witness data for bridge circuit
    fn create_bridge_witness(
        &self,
        operation_data: &BridgeOperationData,
    ) -> Result<super::types::WitnessData, ZkProofError> {
        let public_inputs = vec![
            operation_data.source_chain.as_bytes().to_vec(),
            operation_data.target_chain.as_bytes().to_vec(),
            operation_data.operation_type.as_bytes().to_vec(),
            operation_data.asset_id.as_bytes().to_vec(),
        ];

        let private_inputs = vec![
            operation_data.amount.to_be_bytes().to_vec(),
            operation_data.sender.as_bytes().to_vec(),
            operation_data.recipient.as_bytes().to_vec(),
            operation_data.timestamp.to_be_bytes().to_vec(),
            operation_data.nonce.to_be_bytes().to_vec(),
        ];

        Ok(super::types::WitnessData {
            public_inputs,
            private_inputs,
            circuit_params: CircuitParams {
                circuit_id: self.circuit_id.clone(),
                constraints: self.constraints,
                variables: 2500,
                public_inputs: 4,
                private_inputs: 5,
            },
        })
    }
}

impl TransferCircuit {
    /// Create a new transfer circuit
    pub fn new() -> Self {
        Self {
            circuit_id: "transfer_validity_v1".to_string(),
            max_amount: 1_000_000_000, // 1B tokens max
            supported_tokens: vec!["ERB".to_string(), "WERB".to_string()],
        }
    }

    /// Generate proof for transfer validity
    pub fn generate_transfer_proof(
        &self,
        transfer_data: &TransferData,
    ) -> Result<BridgeZkProof, ZkProofError> {
        // Validate transfer amount
        if transfer_data.amount > self.max_amount {
            return Err(ZkProofError::InvalidWitnessData(
                "Transfer amount exceeds maximum".to_string()
            ));
        }

        // Validate token is supported
        if !self.supported_tokens.contains(&transfer_data.token) {
            return Err(ZkProofError::InvalidWitnessData(
                format!("Token not supported: {}", transfer_data.token)
            ));
        }

        let witness_data = self.create_transfer_witness(transfer_data)?;
        
        let zk_proof = ZkProof::generate_proof(&witness_data.public_inputs, &witness_data.private_inputs)
            .map_err(|e| ZkProofError::ProofGenerationFailed(e.to_string()))?;

        let verification_key = zk_proof.verification_key();
        Ok(BridgeZkProof {
            proof_type: super::types::ZkProofType::TransferValidity,
            proof_data: zk_proof.proof_data,
            public_inputs: witness_data.public_inputs,
            verification_key,
            circuit_id: self.circuit_id.clone(),
        })
    }

    /// Create witness data for transfer circuit
    fn create_transfer_witness(
        &self,
        transfer_data: &TransferData,
    ) -> Result<super::types::WitnessData, ZkProofError> {
        let public_inputs = vec![
            transfer_data.token.as_bytes().to_vec(),
            transfer_data.source_chain.as_bytes().to_vec(),
        ];

        let private_inputs = vec![
            transfer_data.amount.to_be_bytes().to_vec(),
            transfer_data.sender.as_bytes().to_vec(),
            transfer_data.recipient.as_bytes().to_vec(),
            transfer_data.fee.to_be_bytes().to_vec(),
        ];

        Ok(super::types::WitnessData {
            public_inputs,
            private_inputs,
            circuit_params: CircuitParams {
                circuit_id: self.circuit_id.clone(),
                constraints: 1500,
                variables: 800,
                public_inputs: 2,
                private_inputs: 4,
            },
        })
    }
}

impl ValidityCircuit {
    /// Create a new validity circuit
    pub fn new() -> Self {
        Self {
            circuit_id: "operation_validity_v1".to_string(),
            operation_types: vec![
                "transfer".to_string(),
                "wrap".to_string(),
                "unwrap".to_string(),
                "governance".to_string(),
            ],
        }
    }

    /// Generate proof for operation validity
    pub fn generate_validity_proof(
        &self,
        operation: &str,
        parameters: &[Vec<u8>],
    ) -> Result<BridgeZkProof, ZkProofError> {
        if !self.operation_types.contains(&operation.to_string()) {
            return Err(ZkProofError::InvalidWitnessData(
                format!("Operation type not supported: {}", operation)
            ));
        }

        let witness_data = self.create_validity_witness(operation, parameters)?;
        
        let zk_proof = ZkProof::generate_proof(&witness_data.public_inputs, &witness_data.private_inputs)
            .map_err(|e| ZkProofError::ProofGenerationFailed(e.to_string()))?;

        let verification_key = zk_proof.verification_key();
        Ok(BridgeZkProof {
            proof_type: super::types::ZkProofType::BridgeOperation,
            proof_data: zk_proof.proof_data,
            public_inputs: witness_data.public_inputs,
            verification_key,
            circuit_id: self.circuit_id.clone(),
        })
    }

    /// Create witness data for validity circuit
    fn create_validity_witness(
        &self,
        operation: &str,
        parameters: &[Vec<u8>],
    ) -> Result<super::types::WitnessData, ZkProofError> {
        let public_inputs = vec![
            operation.as_bytes().to_vec(),
        ];

        let private_inputs = parameters.to_vec();

        Ok(super::types::WitnessData {
            public_inputs,
            private_inputs,
            circuit_params: CircuitParams {
                circuit_id: self.circuit_id.clone(),
                constraints: 1000,
                variables: 500,
                public_inputs: 1,
                private_inputs: parameters.len() as u32,
            },
        })
    }
}

// Data structures for circuit inputs
#[derive(Debug, Clone)]
pub struct BridgeOperationData {
    pub source_chain: String,
    pub target_chain: String,
    pub operation_type: String,
    pub asset_id: String,
    pub amount: u64,
    pub sender: String,
    pub recipient: String,
    pub timestamp: u64,
    pub nonce: u64,
}

#[derive(Debug, Clone)]
pub struct TransferData {
    pub token: String,
    pub amount: u64,
    pub sender: String,
    pub recipient: String,
    pub source_chain: String,
    pub fee: u64,
}

impl Default for TransferCircuit {
    fn default() -> Self {
        Self::new()
    }
}

impl Default for ValidityCircuit {
    fn default() -> Self {
        Self::new()
    }
}