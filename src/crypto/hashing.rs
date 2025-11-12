use sha2::{Sha256, Sha512, Digest};
use sha3::{Sha3_256, Sha3_512};
use crate::core::types::Hash;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum HashAlgorithm {
    #[default]
    Sha256,
    Sha512,
    Sha3_256,
    Sha3_512,
    Blake3,
}

pub struct Hasher {
    algorithm: HashAlgorithm,
}

impl Default for Hasher {
    fn default() -> Self {
        Self {
            algorithm: HashAlgorithm::Sha3_256, // Default to SHA3-256
        }
    }
}

impl Hasher {
    pub fn new() -> Self {
        Self::default()
    }
    
    pub fn with_algorithm(algorithm: HashAlgorithm) -> Self {
        Self { algorithm }
    }
    
    pub fn hash(&self, data: &[u8]) -> Hash {
        match self.algorithm {
            HashAlgorithm::Sha256 => {
                let mut hasher = Sha256::new();
                hasher.update(data);
                let result: [u8; 32] = hasher.finalize().into();
                Hash::from(result)
            }
            HashAlgorithm::Sha512 => {
                let mut hasher = Sha512::new();
                hasher.update(data);
                // SHA512 produces 64 bytes, we take first 32 for Hash type
                let result = hasher.finalize();
                let mut hash_bytes = [0u8; 32];
                hash_bytes.copy_from_slice(&result[..32]);
                Hash::from(hash_bytes)
            }
            HashAlgorithm::Sha3_256 => {
                let mut hasher = Sha3_256::new();
                hasher.update(data);
                let result: [u8; 32] = hasher.finalize().into();
                Hash::from(result)
            }
            HashAlgorithm::Sha3_512 => {
                let mut hasher = Sha3_512::new();
                hasher.update(data);
                // SHA3-512 produces 64 bytes, we take first 32 for Hash type
                let result = hasher.finalize();
                let mut hash_bytes = [0u8; 32];
                hash_bytes.copy_from_slice(&result[..32]);
                Hash::from(hash_bytes)
            }
            HashAlgorithm::Blake3 => {
                let hash = blake3::hash(data);
                Hash::from(*hash.as_bytes())
            }
        }
    }
    
    pub fn hash_with_salt(&self, data: &[u8], salt: &[u8]) -> Hash {
        let mut combined = Vec::with_capacity(data.len() + salt.len());
        combined.extend_from_slice(data);
        combined.extend_from_slice(salt);
        self.hash(&combined)
    }
    
    pub fn hmac(&self, key: &[u8], data: &[u8]) -> Hash {
        // Simple HMAC implementation
        let mut combined = Vec::with_capacity(key.len() + data.len());
        combined.extend_from_slice(key);
        combined.extend_from_slice(data);
        combined.extend_from_slice(key); // Add key again for HMAC
        self.hash(&combined)
    }
    
    pub fn derive_key_from_password(&self, password: &[u8], salt: &[u8], iterations: u32) -> Hash {
        let mut derived = password.to_vec();
        
        for _ in 0..iterations {
            let mut input = derived.clone();
            input.extend_from_slice(salt);
            derived = self.hash(&input).as_bytes().to_vec();
        }
        
        // Take first 32 bytes as the derived key
        let mut key_bytes = [0u8; 32];
        key_bytes.copy_from_slice(&derived[..32]);
        Hash::from(key_bytes)
    }
    
    pub fn set_algorithm(&mut self, algorithm: HashAlgorithm) {
        self.algorithm = algorithm;
    }
    
    pub fn get_algorithm(&self) -> HashAlgorithm {
        self.algorithm
    }
}

// Additional utility functions
impl Hasher {
    /// Calculate Merkle root from a list of hashes
    pub fn calculate_merkle_root(hashes: &[Hash]) -> Hash {
        if hashes.is_empty() {
            return Hash::new(b"empty");
        }
        
        if hashes.len() == 1 {
            return hashes[0];
        }
        
        let mut current_level: Vec<Hash> = hashes.to_vec();
        
        while current_level.len() > 1 {
            let mut next_level = Vec::new();
            
            for chunk in current_level.chunks(2) {
                if chunk.len() == 2 {
                    let mut combined = Vec::new();
                    combined.extend_from_slice(chunk[0].as_bytes());
                    combined.extend_from_slice(chunk[1].as_bytes());
                    next_level.push(Hash::new(&combined));
                } else {
                    next_level.push(chunk[0]);
                }
            }
            
            current_level = next_level;
        }
        
        current_level[0]
    }
    
    /// Verify a Merkle proof
    pub fn verify_merkle_proof(
        leaf: &Hash,
        proof: &[Hash],
        root: &Hash,
        index: usize,
    ) -> bool {
        let mut computed_hash = *leaf;
        let mut current_index = index;
        
        for proof_hash in proof {
            let mut combined = Vec::new();
            
            if current_index.is_multiple_of(2) {
                // Current is left child
                combined.extend_from_slice(computed_hash.as_bytes());
                combined.extend_from_slice(proof_hash.as_bytes());
            } else {
                // Current is right child
                combined.extend_from_slice(proof_hash.as_bytes());
                combined.extend_from_slice(computed_hash.as_bytes());
            }
            
            computed_hash = Hash::new(&combined);
            current_index /= 2;
        }
        
        &computed_hash == root
    }
}

// Convenience functions for common hash operations
pub fn blake3_hash(data: &[u8]) -> String {
    let hash = blake3::hash(data);
    hex::encode(hash.as_bytes())
}

pub fn sha256_hash(data: &[u8]) -> Vec<u8> {
    let mut hasher = Sha256::new();
    hasher.update(data);
    hasher.finalize().to_vec()
}

pub fn keccak256_hash(data: &[u8]) -> Vec<u8> {
    use sha3::Keccak256;
    let mut hasher = Keccak256::new();
    hasher.update(data);
    hasher.finalize().to_vec()
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_hasher_consistency() {
        let hasher = Hasher::new();
        let data = b"test data";
        let hash1 = hasher.hash(data);
        let hash2 = hasher.hash(data);
        
        assert_eq!(hash1, hash2, "Hashes should be consistent");
    }
    
    #[test]
    fn test_merkle_root_calculation() {
        let hashes = vec![
            Hash::new(b"hash1"),
            Hash::new(b"hash2"),
            Hash::new(b"hash3"),
        ];
        
        let root = Hasher::calculate_merkle_root(&hashes);
        assert_ne!(root, Hash::new(b"empty"), "Merkle root should not be empty");
    }
    
    #[test]
    fn test_blake3_hash() {
        let data = b"test data";
        let hash1 = blake3_hash(data);
        let hash2 = blake3_hash(data);
        assert_eq!(hash1, hash2, "Blake3 hashes should be consistent");
    }
    
    #[test]
    fn test_sha256_hash() {
        let data = b"test data";
        let hash1 = sha256_hash(data);
        let hash2 = sha256_hash(data);
        assert_eq!(hash1, hash2, "SHA256 hashes should be consistent");
        assert_eq!(hash1.len(), 32, "SHA256 should produce 32 bytes");
    }
    
    #[test]
    fn test_keccak256_hash() {
        let data = b"test data";
        let hash1 = keccak256_hash(data);
        let hash2 = keccak256_hash(data);
        assert_eq!(hash1, hash2, "Keccak256 hashes should be consistent");
        assert_eq!(hash1.len(), 32, "Keccak256 should produce 32 bytes");
    }
}
