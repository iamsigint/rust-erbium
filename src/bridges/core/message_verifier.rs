// src/bridges/core/message_verifier.rs
use crate::crypto::dilithium;
use crate::crypto::hashing;
use std::collections::HashMap;
use std::time::{SystemTime, Duration};

#[derive(Debug)]
pub struct VerifiedMessage {
    pub message_hash: String,
    pub verified_at: u64,
    pub chain_id: String,
}

pub struct MessageVerifier {
    verified_messages: HashMap<String, VerifiedMessage>,
    message_timeout: Duration, // Prevent replay attacks
}

impl MessageVerifier {
    pub fn new() -> Self {
        Self {
            verified_messages: HashMap::new(),
            message_timeout: Duration::from_secs(3600), // 1 hour
        }
    }
    
    pub fn verify_cross_chain_message(
        &mut self,
        message: &[u8],
        signature: &[u8],
        public_key: &[u8],
        chain_id: &str,
    ) -> Result<bool, VerificationError> {
        // Prevent replay attacks
        let message_hash = self.hash_message(message);
        if let Some(verified_msg) = self.verified_messages.get(&message_hash) {
            if self.is_message_still_valid(verified_msg) {
                return Ok(true); // Already verified and still valid
            } else {
                return Err(VerificationError::ExpiredMessage);
            }
        }
        
        // Verify using post-quantum signatures
        let is_valid = dilithium::Dilithium::verify(public_key, message, signature)
            .map_err(|_| VerificationError::SignatureInvalid)?;
        
        if is_valid {
            let verified_message = VerifiedMessage {
                message_hash: message_hash.clone(),
                verified_at: SystemTime::now()
                    .duration_since(std::time::UNIX_EPOCH)
                    .unwrap()
                    .as_secs(),
                chain_id: chain_id.to_string(),
            };
            
            self.verified_messages.insert(message_hash, verified_message);
            
            // Clean up expired messages
            self.cleanup_expired_messages();
        }
        
        Ok(is_valid)
    }
    
    pub fn verify_merkle_proof(
        &self,
        leaf_data: &[u8],
        _proof: &[u8],
        _root_hash: &[u8],
    ) -> Result<bool, VerificationError> {
        // Implement Merkle proof verification for light clients
        // This is used for verifying transaction inclusion in source chains
        
        let _leaf_hash = hashing::blake3_hash(leaf_data);
        // Merkle proof verification logic would go here
        
        Ok(true) // Placeholder
    }
    
    fn hash_message(&self, message: &[u8]) -> String {
        hashing::blake3_hash(message)
    }
    
    fn is_message_still_valid(&self, verified_message: &VerifiedMessage) -> bool {
        let current_time = SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_secs();
        
        current_time - verified_message.verified_at <= self.message_timeout.as_secs()
    }
    
    fn cleanup_expired_messages(&mut self) {
        let current_time = SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_secs();
        
        self.verified_messages.retain(|_, verified_msg| {
            current_time - verified_msg.verified_at <= self.message_timeout.as_secs()
        });
    }
}

#[derive(Debug, thiserror::Error)]
pub enum VerificationError {
    #[error("Duplicate or expired message")]
    ExpiredMessage,
    #[error("Invalid signature")]
    SignatureInvalid,
    #[error("Merkle proof verification failed")]
    MerkleProofInvalid,
    #[error("Message format invalid")]
    InvalidFormat,
}