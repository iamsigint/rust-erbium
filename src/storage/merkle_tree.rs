use crate::utils::error::{Result, BlockchainError};
use serde::{Serialize, Deserialize};
use sha2::{Sha256, Digest};

/// Hash type used throughout the blockchain
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct Hash([u8; 32]);

impl Hash {
    pub fn new(data: &[u8]) -> Self {
        let mut hasher = Sha256::new();
        hasher.update(data);
        Hash(hasher.finalize().into())
    }

    pub fn as_bytes(&self) -> &[u8; 32] {
        &self.0
    }

    pub fn from_bytes(bytes: [u8; 32]) -> Self {
        Hash(bytes)
    }

    pub fn to_hex(&self) -> String {
        hex::encode(self.0)
    }

    pub fn from_hex(hex_str: &str) -> Result<Self> {
        let bytes = hex::decode(hex_str)
            .map_err(|e| BlockchainError::Crypto(format!("Invalid hex string: {}", e)))?;
        
        if bytes.len() != 32 {
            return Err(BlockchainError::Crypto("Hash must be 32 bytes".to_string()));
        }

        let mut array = [0u8; 32];
        array.copy_from_slice(&bytes);
        Ok(Hash(array))
    }
}

impl From<[u8; 32]> for Hash {
    fn from(bytes: [u8; 32]) -> Self {
        Hash(bytes)
    }
}

impl AsRef<[u8]> for Hash {
    fn as_ref(&self) -> &[u8] {
        &self.0
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MerkleTree {
    root: Hash,
    leaves: Vec<Hash>,
    levels: Vec<Vec<Hash>>,
}

impl MerkleTree {
    pub fn new(leaves: Vec<Hash>) -> Self {
        let levels = Self::build_levels(&leaves);
        let root = levels.last().map(|level| level[0]).unwrap_or_else(|| Hash::new(b"empty"));
        
        Self {
            root,
            leaves,
            levels,
        }
    }
    
    pub fn from_data(data: Vec<Vec<u8>>) -> Self {
        let leaves: Vec<Hash> = data.iter().map(|d| Hash::new(d)).collect();
        Self::new(leaves)
    }
    
    pub fn root(&self) -> &Hash {
        &self.root
    }
    
    pub fn leaves(&self) -> &[Hash] {
        &self.leaves
    }
    
    pub fn levels(&self) -> &[Vec<Hash>] {
        &self.levels
    }
    
    pub fn proof(&self, index: usize) -> Option<MerkleProof> {
        if index >= self.leaves.len() {
            return None;
        }
        
        let mut proof_hashes = Vec::new();
        let mut current_index = index;
        let mut current_level = 0;
        
        while current_level < self.levels.len() - 1 {
            let level = &self.levels[current_level];
            
            if current_index % 2 == 0 {
                // Current is left child, include right sibling if exists
                if current_index + 1 < level.len() {
                    proof_hashes.push(level[current_index + 1]);
                }
            } else {
                // Current is right child, include left sibling
                proof_hashes.push(level[current_index - 1]);
            }
            
            current_index /= 2;
            current_level += 1;
        }
        
        Some(MerkleProof {
            leaf_hash: self.leaves[index],
            proof_hashes,
            leaf_index: index,
            total_leaves: self.leaves.len(),
        })
    }
    
    pub fn verify_proof(proof: &MerkleProof, root: &Hash) -> bool {
        let mut computed_hash = proof.leaf_hash;
        let mut current_index = proof.leaf_index;
        
        for proof_hash in &proof.proof_hashes {
            let mut combined = Vec::new();
            
            if current_index % 2 == 0 {
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
    
    pub fn update_leaf(&mut self, index: usize, new_leaf: Hash) -> Result<()> {
        if index >= self.leaves.len() {
            return Err(BlockchainError::Storage("Leaf index out of bounds".to_string()));
        }
        
        // Update the leaf
        self.leaves[index] = new_leaf;
        
        // Rebuild the tree from the updated leaf
        self.levels = Self::build_levels(&self.leaves);
        self.root = self.levels.last().map(|level| level[0]).unwrap_or_else(|| Hash::new(b"empty"));
        
        Ok(())
    }
    
    pub fn add_leaf(&mut self, leaf: Hash) {
        self.leaves.push(leaf);
        self.levels = Self::build_levels(&self.leaves);
        self.root = self.levels.last().map(|level| level[0]).unwrap_or_else(|| Hash::new(b"empty"));
    }
    
    pub fn add_leaves(&mut self, leaves: Vec<Hash>) {
        self.leaves.extend(leaves);
        self.levels = Self::build_levels(&self.leaves);
        self.root = self.levels.last().map(|level| level[0]).unwrap_or_else(|| Hash::new(b"empty"));
    }
    
    fn build_levels(leaves: &[Hash]) -> Vec<Vec<Hash>> {
        if leaves.is_empty() {
            return vec![vec![Hash::new(b"empty")]];
        }
        
        let mut levels = Vec::new();
        levels.push(leaves.to_vec());
        
        while levels.last().unwrap().len() > 1 {
            let current_level = levels.last().unwrap();
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
            
            levels.push(next_level);
        }
        
        levels
    }
    
    pub fn contains_leaf(&self, leaf: &Hash) -> bool {
        self.leaves.contains(leaf)
    }
    
    pub fn size(&self) -> usize {
        self.leaves.len()
    }
    
    pub fn is_empty(&self) -> bool {
        self.leaves.is_empty()
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MerkleProof {
    pub leaf_hash: Hash,
    pub proof_hashes: Vec<Hash>,
    pub leaf_index: usize,
    pub total_leaves: usize,
}

impl MerkleProof {
    pub fn verify(&self, root: &Hash) -> bool {
        MerkleTree::verify_proof(self, root)
    }
    
    pub fn to_bytes(&self) -> Vec<u8> {
        let mut bytes = Vec::new();
        
        // Add leaf hash
        bytes.extend_from_slice(self.leaf_hash.as_bytes());
        
        // Add proof hashes count
        bytes.extend_from_slice(&(self.proof_hashes.len() as u32).to_be_bytes());
        
        // Add proof hashes
        for hash in &self.proof_hashes {
            bytes.extend_from_slice(hash.as_bytes());
        }
        
        // Add leaf index and total leaves
        bytes.extend_from_slice(&(self.leaf_index as u32).to_be_bytes());
        bytes.extend_from_slice(&(self.total_leaves as u32).to_be_bytes());
        
        bytes
    }
    
    pub fn from_bytes(bytes: &[u8]) -> Result<Self> {
        if bytes.len() < 32 + 4 + 4 + 4 {
            return Err(BlockchainError::Storage("Invalid proof bytes".to_string()));
        }
        
        let mut offset = 0;
        
        // Read leaf hash
        let leaf_hash_bytes: [u8; 32] = bytes[offset..offset + 32].try_into().unwrap();
        let leaf_hash = Hash::from(leaf_hash_bytes);
        offset += 32;
        
        // Read proof hashes count
        let proof_count = u32::from_be_bytes(bytes[offset..offset + 4].try_into().unwrap()) as usize;
        offset += 4;
        
        if bytes.len() < offset + (proof_count * 32) + 8 {
            return Err(BlockchainError::Storage("Invalid proof bytes length".to_string()));
        }
        
        // Read proof hashes
        let mut proof_hashes = Vec::with_capacity(proof_count);
        for _ in 0..proof_count {
            let hash_bytes: [u8; 32] = bytes[offset..offset + 32].try_into().unwrap();
            proof_hashes.push(Hash::from(hash_bytes));
            offset += 32;
        }
        
        // Read leaf index and total leaves
        let leaf_index = u32::from_be_bytes(bytes[offset..offset + 4].try_into().unwrap()) as usize;
        offset += 4;
        
        let total_leaves = u32::from_be_bytes(bytes[offset..offset + 4].try_into().unwrap()) as usize;
        
        Ok(Self {
            leaf_hash,
            proof_hashes,
            leaf_index,
            total_leaves,
        })
    }
}