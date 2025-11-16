use crate::crypto::dilithium::Dilithium;
use crate::utils::error::{BlockchainError, Result};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Signature {
    pub data: Vec<u8>,
    pub scheme: SignatureScheme,
    pub public_key: Vec<u8>,
    pub timestamp: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Hash)]
pub enum SignatureScheme {
    Dilithium2,
    Dilithium3,
    Dilithium5,
}

pub trait SignatureSchemeTrait {
    fn sign(&self, private_key: &[u8], message: &[u8]) -> Result<Vec<u8>>;
    fn verify(&self, public_key: &[u8], message: &[u8], signature: &[u8]) -> Result<bool>;
    fn key_size(&self) -> (usize, usize); // (public_key_size, private_key_size)
    fn signature_size(&self) -> usize;
}

pub struct DilithiumScheme {
    scheme: SignatureScheme,
}

impl DilithiumScheme {
    pub fn new(scheme: SignatureScheme) -> Self {
        Self { scheme }
    }
}

impl SignatureSchemeTrait for DilithiumScheme {
    fn sign(&self, private_key: &[u8], message: &[u8]) -> Result<Vec<u8>> {
        Dilithium::sign(private_key, message)
    }

    fn verify(&self, public_key: &[u8], message: &[u8], signature: &[u8]) -> Result<bool> {
        Dilithium::verify(public_key, message, signature)
    }

    fn key_size(&self) -> (usize, usize) {
        match self.scheme {
            SignatureScheme::Dilithium2 => (1312, 2528),
            SignatureScheme::Dilithium3 => (1952, 4000),
            SignatureScheme::Dilithium5 => (2592, 4864),
        }
    }

    fn signature_size(&self) -> usize {
        match self.scheme {
            SignatureScheme::Dilithium2 => 2420,
            SignatureScheme::Dilithium3 => 3309,
            SignatureScheme::Dilithium5 => 4627,
        }
    }
}

#[derive(Default)]
pub struct SignatureManager {
    schemes: HashMap<SignatureScheme, Box<dyn SignatureSchemeTrait>>,
}

impl SignatureManager {
    pub fn new() -> Self {
        let mut schemes = HashMap::new();

        schemes.insert(
            SignatureScheme::Dilithium2,
            Box::new(DilithiumScheme::new(SignatureScheme::Dilithium2))
                as Box<dyn SignatureSchemeTrait>,
        );

        schemes.insert(
            SignatureScheme::Dilithium3,
            Box::new(DilithiumScheme::new(SignatureScheme::Dilithium3)),
        );

        schemes.insert(
            SignatureScheme::Dilithium5,
            Box::new(DilithiumScheme::new(SignatureScheme::Dilithium5)),
        );

        Self { schemes }
    }

    pub fn get_scheme(&self, scheme: &SignatureScheme) -> Option<&dyn SignatureSchemeTrait> {
        self.schemes.get(scheme).map(|s| s.as_ref())
    }

    pub fn create_signature(
        &self,
        private_key: &[u8],
        message: &[u8],
        scheme: SignatureScheme,
    ) -> Result<Signature> {
        let signer = self
            .schemes
            .get(&scheme)
            .ok_or_else(|| BlockchainError::Crypto("Unsupported signature scheme".to_string()))?;

        let signature_data = signer.sign(private_key, message)?;

        // Extract public key from private key (simplified - in real implementation, store separately)
        let public_key = self.derive_public_key(private_key, &scheme)?;

        Ok(Signature {
            data: signature_data,
            scheme,
            public_key,
            timestamp: chrono::Utc::now().timestamp_millis() as u64,
        })
    }

    pub fn verify_signature(&self, signature: &Signature, message: &[u8]) -> Result<bool> {
        let verifier = self
            .schemes
            .get(&signature.scheme)
            .ok_or_else(|| BlockchainError::Crypto("Unsupported signature scheme".to_string()))?;

        verifier.verify(&signature.public_key, message, &signature.data)
    }

    pub fn batch_verify_signatures(
        &self,
        signatures: &[(Signature, Vec<u8>)], // (signature, message)
    ) -> Result<Vec<bool>> {
        let mut results = Vec::with_capacity(signatures.len());

        for (signature, message) in signatures {
            let result = self.verify_signature(signature, message)?;
            results.push(result);
        }

        Ok(results)
    }

    fn derive_public_key(&self, private_key: &[u8], scheme: &SignatureScheme) -> Result<Vec<u8>> {
        // In real implementation, this would properly derive the public key
        // For Dilithium, we typically store public key separately
        // This is a simplified version
        match scheme {
            SignatureScheme::Dilithium2 => {
                // Return first 1312 bytes as "public key" (simplified)
                Ok(private_key[..1312.min(private_key.len())].to_vec())
            }
            SignatureScheme::Dilithium3 => Ok(private_key[..1952.min(private_key.len())].to_vec()),
            SignatureScheme::Dilithium5 => Ok(private_key[..2592.min(private_key.len())].to_vec()),
        }
    }
}

impl Signature {
    pub fn new(data: Vec<u8>, scheme: SignatureScheme, public_key: Vec<u8>) -> Self {
        Self {
            data,
            scheme,
            public_key,
            timestamp: chrono::Utc::now().timestamp_millis() as u64,
        }
    }

    pub fn to_bytes(&self) -> Vec<u8> {
        // Serialize signature to bytes
        let mut bytes = Vec::new();

        // Add scheme identifier
        bytes.push(match self.scheme {
            SignatureScheme::Dilithium2 => 0x02,
            SignatureScheme::Dilithium3 => 0x03,
            SignatureScheme::Dilithium5 => 0x05,
        });

        // Add public key length and public key
        bytes.extend_from_slice(&(self.public_key.len() as u32).to_be_bytes());
        bytes.extend_from_slice(&self.public_key);

        // Add signature data
        bytes.extend_from_slice(&self.data);

        // Add timestamp
        bytes.extend_from_slice(&self.timestamp.to_be_bytes());

        bytes
    }

    pub fn from_bytes(bytes: &[u8]) -> Result<Self> {
        if bytes.len() < 1 + 4 + 4 {
            return Err(BlockchainError::Crypto(
                "Invalid signature bytes".to_string(),
            ));
        }

        let mut offset = 0;

        // Read scheme
        let scheme = match bytes[offset] {
            0x02 => SignatureScheme::Dilithium2,
            0x03 => SignatureScheme::Dilithium3,
            0x05 => SignatureScheme::Dilithium5,
            _ => {
                return Err(BlockchainError::Crypto(
                    "Unknown signature scheme".to_string(),
                ))
            }
        };
        offset += 1;

        // Read public key length
        let pubkey_len = u32::from_be_bytes(bytes[offset..offset + 4].try_into().unwrap()) as usize;
        offset += 4;

        if bytes.len() < offset + pubkey_len + 8 {
            return Err(BlockchainError::Crypto(
                "Invalid signature bytes length".to_string(),
            ));
        }

        // Read public key
        let public_key = bytes[offset..offset + pubkey_len].to_vec();
        offset += pubkey_len;

        // Read signature data (remaining bytes minus timestamp)
        let sig_data_len = bytes.len() - offset - 8;
        let data = bytes[offset..offset + sig_data_len].to_vec();
        offset += sig_data_len;

        // Read timestamp
        let timestamp = u64::from_be_bytes(bytes[offset..offset + 8].try_into().unwrap());

        Ok(Signature {
            data,
            scheme,
            public_key,
            timestamp,
        })
    }
}
