// src/vm/storage.rs
use crate::core::types::{Address, Hash};
use crate::utils::error::{Result, BlockchainError};
use std::collections::HashMap;
use parity_db::Db;
use serde::{Serialize, Deserialize};
use std::sync::Arc;

// Wrapper para Db para evitar orphan rule
#[derive(Clone)]
pub struct DbWrapper(pub Arc<Db>);

impl std::fmt::Debug for DbWrapper {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "Db{{...}}")
    }
}

#[derive(Clone, Serialize, Deserialize)]
pub struct ContractStorage {
    memory_storage: HashMap<Address, HashMap<Vec<u8>, Vec<u8>>>,
    #[serde(skip)]
    db: Option<DbWrapper>,
}

impl std::fmt::Debug for ContractStorage {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("ContractStorage")
            .field("memory_storage", &self.memory_storage)
            .field("db", &self.db.as_ref().map(|_| "Db{...}"))
            .finish()
    }
}

impl ContractStorage {
    pub fn new() -> Result<Self> {
        let memory_storage = HashMap::new();
        let db = None;
        
        Ok(Self {
            memory_storage,
            db,
        })
    }
    
    // ... restante do código permanece igual ...
    pub fn initialize_contract(&mut self, contract_address: &Address) -> Result<()> {
        self.memory_storage.insert(contract_address.clone(), HashMap::new());
        Ok(())
    }
    
    pub fn store(&mut self, contract_address: &Address, key: Vec<u8>, value: Vec<u8>) -> Result<()> {
        let contract_storage = self.memory_storage.get_mut(contract_address)
            .ok_or_else(|| BlockchainError::VM("Contract storage not initialized".to_string()))?;
        
        contract_storage.insert(key, value);
        Ok(())
    }
    
    pub fn load(&self, contract_address: &Address, key: &[u8]) -> Result<Option<Vec<u8>>> {
        let contract_storage = self.memory_storage.get(contract_address)
            .ok_or_else(|| BlockchainError::VM("Contract storage not initialized".to_string()))?;
        
        Ok(contract_storage.get(key).cloned())
    }
    
    pub fn store_confidential(
        &mut self,
        contract_address: &Address,
        key: Vec<u8>,
        value: Vec<u8>,
        encryption_key: &[u8],
    ) -> Result<()> {
        let encrypted_value = self.encrypt_value(&value, encryption_key)?;
        self.store(contract_address, key, encrypted_value)
    }
    
    pub fn load_confidential(
        &self,
        contract_address: &Address,
        key: &[u8],
        decryption_key: &[u8],
    ) -> Result<Option<Vec<u8>>> {
        if let Some(encrypted_value) = self.load(contract_address, key)? {
            let decrypted_value = self.decrypt_value(&encrypted_value, decryption_key)?;
            Ok(Some(decrypted_value))
        } else {
            Ok(None)
        }
    }
    
    fn encrypt_value(&self, value: &[u8], key: &[u8]) -> Result<Vec<u8>> {
        if key.is_empty() {
            return Err(BlockchainError::Crypto("Encryption key cannot be empty".to_string()));
        }
        let mut encrypted = Vec::with_capacity(value.len());
        for (i, &byte) in value.iter().enumerate() {
            encrypted.push(byte ^ key[i % key.len()]);
        }
        Ok(encrypted)
    }
    
    fn decrypt_value(&self, encrypted_value: &[u8], key: &[u8]) -> Result<Vec<u8>> {
        self.encrypt_value(encrypted_value, key)
    }
    
    pub fn calculate_storage_root(&self, contract_address: &Address) -> Result<Hash> {
        let contract_storage = self.memory_storage.get(contract_address)
            .ok_or_else(|| BlockchainError::VM("Contract storage not initialized".to_string()))?;
        
        let mut entries: Vec<(Vec<u8>, Vec<u8>)> = contract_storage
            .iter()
            .map(|(k, v)| (k.clone(), v.clone()))
            .collect();
        entries.sort_by(|a, b| a.0.cmp(&b.0));
        
        use sha2::{Sha256, Digest};
        let mut hasher = Sha256::new();
        for (key, value) in entries {
            hasher.update(&key);
            hasher.update(&value);
        }
        
        let hash_bytes: [u8; 32] = hasher.finalize().into();
        Ok(Hash::from(hash_bytes))
    }
    
    pub fn get_all_entries(&self, contract_address: &Address) -> Result<Vec<(Vec<u8>, Vec<u8>)>> {
        let contract_storage = self.memory_storage.get(contract_address)
            .ok_or_else(|| BlockchainError::VM("Contract storage not initialized".to_string()))?;
        
        let mut entries: Vec<(Vec<u8>, Vec<u8>)> = contract_storage
            .iter()
            .map(|(k, v)| (k.clone(), v.clone()))
            .collect();
        entries.sort_by(|a, b| a.0.cmp(&b.0));
        
        Ok(entries)
    }
}

impl Default for ContractStorage {
    fn default() -> Self {
        Self::new().unwrap_or_else(|_| Self {
            memory_storage: HashMap::new(),
            db: None,
        })
    }
}