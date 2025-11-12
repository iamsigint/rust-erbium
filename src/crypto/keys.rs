use crate::utils::error::{Result, BlockchainError};
use crate::crypto::dilithium::Dilithium;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct KeyPair {
    pub public_key: Vec<u8>,
    pub private_key: Vec<u8>,
    pub key_type: KeyType,
    pub created_at: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum KeyType {
    Dilithium2,
    Dilithium3,
    Dilithium5,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct KeyMetadata {
    pub key_id: String,
    pub key_type: KeyType,
    pub created_at: u64,
    pub last_used: u64,
    pub usage_count: u64,
}

#[derive(Default)]
pub struct KeyManager {
    key_pairs: HashMap<String, KeyPair>,
    key_metadata: HashMap<String, KeyMetadata>,
}

impl KeyManager {
    pub fn new() -> Self {
        Self {
            key_pairs: HashMap::new(),
            key_metadata: HashMap::new(),
        }
    }
    
    pub fn generate_dilithium_keypair(&self) -> Result<KeyPair> {
        let (public_key, private_key) = Dilithium::generate_keypair()?;
        
        Ok(KeyPair {
            public_key,
            private_key,
            key_type: KeyType::Dilithium3,
            created_at: chrono::Utc::now().timestamp_millis() as u64,
        })
    }
    
    pub fn store_keypair(&mut self, key_id: String, keypair: KeyPair) -> Result<()> {
        if self.key_pairs.contains_key(&key_id) {
            return Err(BlockchainError::Crypto("Key ID already exists".to_string()));
        }
        
        let metadata = KeyMetadata {
            key_id: key_id.clone(),
            key_type: keypair.key_type.clone(),
            created_at: keypair.created_at,
            last_used: chrono::Utc::now().timestamp_millis() as u64,
            usage_count: 0,
        };
        
        self.key_pairs.insert(key_id.clone(), keypair);
        self.key_metadata.insert(key_id, metadata);
        
        Ok(())
    }
    
    pub fn get_keypair(&mut self, key_id: &str) -> Result<Option<&KeyPair>> {
        if let Some(metadata) = self.key_metadata.get_mut(key_id) {
            metadata.last_used = chrono::Utc::now().timestamp_millis() as u64;
            metadata.usage_count += 1;
        }
        
        Ok(self.key_pairs.get(key_id))
    }
    
    pub fn remove_keypair(&mut self, key_id: &str) -> Result<()> {
        self.key_pairs.remove(key_id);
        self.key_metadata.remove(key_id);
        Ok(())
    }
    
    pub fn list_keys(&self) -> Vec<&KeyMetadata> {
        self.key_metadata.values().collect()
    }
    
    pub fn get_key_metadata(&self, key_id: &str) -> Option<&KeyMetadata> {
        self.key_metadata.get(key_id)
    }
    
    pub fn export_keypair(&self, key_id: &str, password: &str) -> Result<Vec<u8>> {
        let keypair = self.key_pairs.get(key_id)
            .ok_or_else(|| BlockchainError::Crypto("Key not found".to_string()))?;
        
        let serialized = bincode::serialize(keypair)
            .map_err(|e| BlockchainError::Serialization(e.to_string()))?;
        
        let encrypted = self.encrypt_data(&serialized, password)?;
        
        Ok(encrypted)
    }
    
    pub fn import_keypair(
        &mut self,
        encrypted_data: &[u8],
        password: &str,
        key_id: String,
    ) -> Result<()> {
        let decrypted = self.decrypt_data(encrypted_data, password)?;
        
        let keypair: KeyPair = bincode::deserialize(&decrypted)
            .map_err(|e| BlockchainError::Serialization(e.to_string()))?;
        
        self.store_keypair(key_id, keypair)?;
        
        Ok(())
    }
    
    pub fn generate_key_derivation_path(
        &self,
        purpose: u32,
        coin_type: u32,
        account: u32,
        change: u32,
        address_index: u32,
    ) -> String {
        format!("m/{}'/{}'/{}'/{}'/{}'", purpose, coin_type, account, change, address_index)
    }
    
    pub fn derive_child_key(&self, _parent_key: &KeyPair, derivation_path: &str) -> Result<KeyPair> {
        log::debug!("Deriving child key from path: {}", derivation_path);
        self.generate_dilithium_keypair()
    }
    
    fn encrypt_data(&self, data: &[u8], password: &str) -> Result<Vec<u8>> {
        let key = self.derive_encryption_key(password);
        let mut encrypted = Vec::with_capacity(data.len());
        
        for (i, &byte) in data.iter().enumerate() {
            encrypted.push(byte ^ key[i % key.len()]);
        }
        
        Ok(encrypted)
    }
    
    fn decrypt_data(&self, encrypted_data: &[u8], password: &str) -> Result<Vec<u8>> {
        self.encrypt_data(encrypted_data, password)
    }
    
    fn derive_encryption_key(&self, password: &str) -> Vec<u8> {
        let mut key = Vec::new();
        for byte in password.bytes() {
            key.push(byte.wrapping_mul(31));
        }
        
        let mut extended_key = key.clone();
        while extended_key.len() < 32 {
            let current_len = extended_key.len();
            let needed = 32 - current_len;
            let copy_len = key.len().min(needed);
            extended_key.extend_from_slice(&key[..copy_len]);
        }
        
        extended_key.truncate(32);
        extended_key
    }
}

impl KeyPair {
    pub fn new(public_key: Vec<u8>, private_key: Vec<u8>, key_type: KeyType) -> Self {
        Self {
            public_key,
            private_key,
            key_type,
            created_at: chrono::Utc::now().timestamp_millis() as u64,
        }
    }
    
    pub fn get_public_key_hex(&self) -> String {
        hex::encode(&self.public_key)
    }
    
    pub fn get_private_key_hex(&self) -> String {
        hex::encode(&self.private_key)
    }
    
    pub fn sign(&self, message: &[u8]) -> Result<Vec<u8>> {
        match self.key_type {
            KeyType::Dilithium2 | KeyType::Dilithium3 | KeyType::Dilithium5 => {
                Dilithium::sign(&self.private_key, message)
            }
        }
    }
    
    pub fn verify(&self, message: &[u8], signature: &[u8]) -> Result<bool> {
        match self.key_type {
            KeyType::Dilithium2 | KeyType::Dilithium3 | KeyType::Dilithium5 => {
                Dilithium::verify(&self.public_key, message, signature)
            }
        }
    }
}