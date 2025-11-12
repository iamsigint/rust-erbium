// src/crypto/dilithium.rs

use pqcrypto_dilithium::dilithium3::{
    detached_sign, verify_detached_signature, keypair, 
    public_key_bytes, secret_key_bytes, signature_bytes,
    PublicKey as DilithiumPK,
    SecretKey as DilithiumSK, 
    DetachedSignature as DilithiumSig
};
use pqcrypto_traits::sign::{
    PublicKey as PubKeyTrait, 
    SecretKey as SecKeyTrait, 
    DetachedSignature as DetachedSigTrait
};
use crate::utils::error::{BlockchainError, Result};

pub struct Dilithium;

impl Dilithium {
    /// Generate a new Dilithium keypair
    pub fn generate_keypair() -> Result<(Vec<u8>, Vec<u8>)> {
        let (public_key, secret_key) = keypair();
        Ok((
            public_key.as_bytes().to_vec(),
            secret_key.as_bytes().to_vec(),
        ))
    }
    
    /// Sign a message with private key
    pub fn sign(private_key: &[u8], message: &[u8]) -> Result<Vec<u8>> {
        let _sk = DilithiumSK::from_bytes(private_key)
            .map_err(|e| BlockchainError::Crypto(format!("Invalid private key: {}", e)))?;
            
        let signature = detached_sign(message, &_sk);
        Ok(signature.as_bytes().to_vec())
    }
    
    /// Verify a signature with public key
    pub fn verify(public_key: &[u8], message: &[u8], signature: &[u8]) -> Result<bool> {
        let pk = DilithiumPK::from_bytes(public_key)
            .map_err(|e| BlockchainError::Crypto(format!("Invalid public key: {}", e)))?;
            
        let sig = DilithiumSig::from_bytes(signature)
            .map_err(|e| BlockchainError::Crypto(format!("Invalid signature: {}", e)))?;
            
        match verify_detached_signature(&sig, message, &pk) {
            Ok(()) => Ok(true),
            Err(_) => Ok(false),
        }
    }
    
    /// Get public key size in bytes
    pub fn public_key_size() -> usize {
        public_key_bytes()
    }
    
    /// Get private key size in bytes
    pub fn private_key_size() -> usize {
        secret_key_bytes()
    }
    
    /// Get signature size in bytes
    pub fn signature_size() -> usize {
        signature_bytes()
    }
}

/// Dilithium keypair wrapper for easier usage
pub struct DilithiumKeypair {
    pub public_key: Vec<u8>,
    pub secret_key: Vec<u8>,
}

impl DilithiumKeypair {
    /// Generates a new keypair
    pub fn generate() -> Result<Self> {
        let (public_key, secret_key) = Dilithium::generate_keypair()?;
        Ok(Self {
            public_key,
            secret_key,
        })
    }

    /// Create a keypair instance from an existing private key
    /// Note: pqcrypto does not support deriving the public key from the secret key directly.
    /// For safety, require the caller to provide the public key explicitly to avoid mismatches.
    pub fn from_keys(public_key: &[u8], private_key: &[u8]) -> Result<Self> {
        // Validate formats
        let _ = DilithiumPK::from_bytes(public_key)
            .map_err(|e| BlockchainError::Crypto(format!("Invalid public key: {}", e)))?;
        let _ = DilithiumSK::from_bytes(private_key)
            .map_err(|e| BlockchainError::Crypto(format!("Invalid private key: {}", e)))?;

        Ok(Self {
            public_key: public_key.to_vec(),
            secret_key: private_key.to_vec(),
        })
    }
    
    /// Signs a message using the instance's secret key
    pub fn sign(&self, message: &[u8]) -> Result<Vec<u8>> {
        Dilithium::sign(&self.secret_key, message)
    }
    
    /// Verifies a message using the instance's public key
    pub fn verify(&self, message: &[u8], signature: &[u8]) -> Result<bool> {
        Dilithium::verify(&self.public_key, message, signature)
    }
    
    /// Returns a slice to the public key bytes
    pub fn public_key_bytes(&self) -> &[u8] {
        &self.public_key
    }
    
    /// Returns a slice to the secret key bytes
    pub fn secret_key_bytes(&self) -> &[u8] {
        &self.secret_key
    }
}