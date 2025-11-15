pub mod dilithium;
pub mod zk_proofs;
pub mod keys;
pub mod hashing;
pub mod signatures;

// Re-export commonly used cryptographic functions
pub use dilithium::Dilithium;
pub use zk_proofs::ZkProofs;
pub use keys::{KeyPair, KeyManager};
pub use hashing::{HashAlgorithm, Hasher};
pub use signatures::{SignatureScheme, Signature};

use crate::utils::error::Result;
use serde::{Deserialize, Serialize};

/// Main cryptographic manager that coordinates all crypto operations
pub struct CryptoManager {
    pub dilithium: Dilithium,
    pub zk_proofs: ZkProofs,
    pub key_manager: KeyManager,
    pub hasher: Hasher,
}

impl CryptoManager {
    pub fn new() -> Result<Self> {
        Ok(Self {
            dilithium: Dilithium,
            zk_proofs: ZkProofs::new()?,
            key_manager: KeyManager::new(),
            hasher: Hasher::new(),
        })
    }
    
    /// Generate a new post-quantum keypair
    pub fn generate_keypair(&self) -> Result<KeyPair> {
        self.key_manager.generate_dilithium_keypair()
    }
    
    /// Hash data using the configured algorithm
    pub fn hash(&self, data: &[u8]) -> crate::core::types::Hash {
        self.hasher.hash(data)
    }
    
    /// Verify a Dilithium signature
    pub fn verify_signature(
        &self,
        public_key: &[u8],
        message: &[u8],
        signature: &[u8],
    ) -> Result<bool> {
        Dilithium::verify(public_key, message, signature)
    }
}

impl Default for CryptoManager {
    fn default() -> Self {
        Self::new().expect("Failed to create CryptoManager")
    }
}

/// Multi-Layer Transaction Cryptography Types
/// Novas estruturas para suportar witness isolation e crypto avançada

// NOTE: Using existing Signature type from signatures module

/// Zero-Knowledge Proof for privacy-preserving operations
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ZkProof {
    pub proof_type: String, // "range", "membership", "aggregate", etc.
    pub commitment: Vec<u8>,
    pub proof_data: Vec<u8>,
    pub public_inputs: Vec<u8>,
}

impl ZkProof {
    pub fn new(proof_type: &str) -> Self {
        Self {
            proof_type: proof_type.to_string(),
            commitment: Vec::new(),
            proof_data: Vec::new(),
            public_inputs: Vec::new(),
        }
    }

    /// Verify ZK proof integrity
    pub fn verify(&self) -> Result<bool> {
        // TODO: Implement actual ZK verification
        // For now, basic structural validation
        if self.proof_data.is_empty() {
            return Ok(false);
        }
        Ok(true)
    }
}

/// Quantum-resistant proof structure
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct QuantumProof {
    pub algorithm: String, // "dilithium", "falcon", etc.
    pub challenge: Vec<u8>,
    pub response: Vec<u8>,
    pub commitment: Vec<u8>,
}

impl QuantumProof {
    pub fn new(algorithm: &str) -> Self {
        Self {
            algorithm: algorithm.to_string(),
            challenge: Vec::new(),
            response: Vec::new(),
            commitment: Vec::new(),
        }
    }

    /// Verify quantum proof
    pub fn verify(&self, public_key: &[u8], message: &[u8]) -> Result<bool> {
        // TODO: Implement actual quantum proof verification
        // For now, basic validation
        if self.response.is_empty() || self.challenge.is_empty() {
            return Ok(false);
        }

        // Delegate to appropriate quantum algorithm
        match self.algorithm.as_str() {
            "dilithium" => {
                // Use existing Dilithium verification
                Dilithium::verify(public_key, message, &self.response)
            }
            _ => Ok(false), // Unsupported algorithm
        }
    }
}

/// Schnorr signature for enhanced witness operations
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SchnorrSignature {
    pub r: Vec<u8>, // Random point commitment
    pub s: Vec<u8>, // Signature scalar
    pub pub_key: Vec<u8>, // Public key point
}

impl SchnorrSignature {
    pub fn new(r: Vec<u8>, s: Vec<u8>, pub_key: Vec<u8>) -> Self {
        Self { r, s, pub_key }
    }

    /// Verify Schnorr signature
    pub fn verify(&self, _message: &[u8]) -> Result<bool> {
        // TODO: Implement Schnorr verification algorithm
        // This is a simplified placeholder
        if self.r.is_empty() || self.s.is_empty() || self.pub_key.is_empty() {
            return Ok(false);
        }
        Ok(true)
    }
}

/// Distributed threshold signature for multi-party operations
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ThresholdSignature {
    pub signatures: Vec<crate::crypto::signatures::Signature>,
    pub threshold: usize,
    pub public_keys: Vec<Vec<u8>>,
}

impl ThresholdSignature {
    pub fn new(threshold: usize) -> Self {
        Self {
            signatures: Vec::new(),
            threshold,
            public_keys: Vec::new(),
        }
    }

    /// Add participant signature
    pub fn add_signature(&mut self, signature: crate::crypto::signatures::Signature, public_key: Vec<u8>) {
        self.signatures.push(signature);
        self.public_keys.push(public_key);
    }

    /// Check if threshold is met
    pub fn is_threshold_met(&self) -> bool {
        self.signatures.len() >= self.threshold
    }

    /// Verify all signatures (TODO: Implement proper verification)
    pub fn verify_all(&self, _message: &[u8]) -> Result<bool> {
        // TODO: Implement actual threshold signature verification
        Ok(self.is_threshold_met())
    }
}
