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