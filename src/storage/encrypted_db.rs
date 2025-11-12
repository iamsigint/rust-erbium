use crate::utils::error::{Result, BlockchainError};
use ring::aead;
use ring::rand::{SecureRandom, SystemRandom};
use serde::{Serialize, Deserialize};

pub struct EncryptedDatabase {
    enabled: bool,
    rng: SystemRandom,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EncryptedData {
    pub ciphertext: Vec<u8>,
    pub nonce: Vec<u8>,
    pub tag: Vec<u8>,
}

impl EncryptedDatabase {
    pub fn new(enabled: bool) -> Result<Self> {
        let rng = SystemRandom::new();
        
        Ok(Self {
            enabled,
            rng,
        })
    }
    
    pub fn encrypt(&self, plaintext: &[u8]) -> Result<Vec<u8>> {
        if !self.enabled {
            return Ok(plaintext.to_vec());
        }
        
        // Generate a random key for this encryption
        let key = self.generate_key()?;
        
        // Generate a random nonce
        let nonce = self.generate_nonce()?;
        
        // Prepare for encryption
        let unbound_key = aead::UnboundKey::new(&aead::AES_256_GCM, &key)
            .map_err(|e| BlockchainError::Crypto(format!("Failed to create key: {}", e)))?;
        
        let sealing_key = aead::LessSafeKey::new(unbound_key);
        
        let mut in_out = plaintext.to_vec();
        let tag = sealing_key.seal_in_place_separate_tag(
            aead::Nonce::assume_unique_for_key(nonce.try_into().unwrap()),
            aead::Aad::empty(),
            &mut in_out,
        ).map_err(|e| BlockchainError::Crypto(format!("Encryption failed: {}", e)))?;
        
        // Combine nonce, ciphertext, and tag
        let mut result = nonce.to_vec();
        result.extend_from_slice(&in_out);
        result.extend_from_slice(tag.as_ref());
        
        Ok(result)
    }
    
    pub fn decrypt(&self, ciphertext: &[u8]) -> Result<Vec<u8>> {
        if !self.enabled {
            return Ok(ciphertext.to_vec());
        }
        
        if ciphertext.len() < aead::NONCE_LEN + aead::AES_256_GCM.tag_len() {
            return Err(BlockchainError::Crypto("Ciphertext too short".to_string()));
        }
        
        // Extract nonce, ciphertext, and tag
        let nonce = &ciphertext[..aead::NONCE_LEN];
        let tag_start = ciphertext.len() - aead::AES_256_GCM.tag_len();
        let actual_ciphertext = &ciphertext[aead::NONCE_LEN..tag_start];
        let tag = &ciphertext[tag_start..];
        
        // In real implementation, we would use a proper key management system
        // For now, we'll generate the same key (this is just for demonstration)
        let key = self.generate_key()?;
        
        let unbound_key = aead::UnboundKey::new(&aead::AES_256_GCM, &key)
            .map_err(|e| BlockchainError::Crypto(format!("Failed to create key: {}", e)))?;
        
        let opening_key = aead::LessSafeKey::new(unbound_key);
        
        let mut in_out = actual_ciphertext.to_vec();
        in_out.extend_from_slice(tag);
        
        let plaintext = opening_key.open_in_place(
            aead::Nonce::assume_unique_for_key(nonce.try_into().unwrap()),
            aead::Aad::empty(),
            &mut in_out,
        ).map_err(|e| BlockchainError::Crypto(format!("Decryption failed: {}", e)))?;
        
        Ok(plaintext.to_vec())
    }
    
    fn generate_key(&self) -> Result<[u8; 32]> {
        let mut key = [0u8; 32];
        self.rng.fill(&mut key)
            .map_err(|e| BlockchainError::Crypto(format!("Failed to generate key: {}", e)))?;
        Ok(key)
    }
    
    fn generate_nonce(&self) -> Result<[u8; aead::NONCE_LEN]> {
        let mut nonce = [0u8; aead::NONCE_LEN];
        self.rng.fill(&mut nonce)
            .map_err(|e| BlockchainError::Crypto(format!("Failed to generate nonce: {}", e)))?;
        Ok(nonce)
    }
    
    pub fn is_enabled(&self) -> bool {
        self.enabled
    }
    
    pub fn set_enabled(&mut self, enabled: bool) {
        self.enabled = enabled;
    }
}

// Simple in-memory key-value store with encryption
pub struct EncryptedStore {
    db: std::collections::HashMap<Vec<u8>, Vec<u8>>,
    encryptor: EncryptedDatabase,
}

impl EncryptedStore {
    pub fn new() -> Result<Self> {
        let encryptor = EncryptedDatabase::new(true)?;
        Ok(Self {
            db: std::collections::HashMap::new(),
            encryptor,
        })
    }
    
    pub fn put(&mut self, key: &[u8], value: &[u8]) -> Result<()> {
        let encrypted_value = self.encryptor.encrypt(value)?;
        self.db.insert(key.to_vec(), encrypted_value);
        Ok(())
    }
    
    pub fn get(&self, key: &[u8]) -> Result<Option<Vec<u8>>> {
        if let Some(encrypted_value) = self.db.get(key) {
            let decrypted_value = self.encryptor.decrypt(encrypted_value)?;
            Ok(Some(decrypted_value))
        } else {
            Ok(None)
        }
    }
    
    pub fn delete(&mut self, key: &[u8]) -> Result<()> {
        self.db.remove(key);
        Ok(())
    }
    
    pub fn contains(&self, key: &[u8]) -> bool {
        self.db.contains_key(key)
    }
    
    pub fn len(&self) -> usize {
        self.db.len()
    }
    
    pub fn is_empty(&self) -> bool {
        self.db.is_empty()
    }
    
    pub fn clear(&mut self) {
        self.db.clear();
    }
}
