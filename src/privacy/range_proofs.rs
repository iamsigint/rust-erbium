use bulletproofs::{BulletproofGens, PedersenGens, RangeProof};
use curve25519_dalek_ng::ristretto::CompressedRistretto;
use curve25519_dalek_ng::scalar::Scalar;
use merlin::Transcript;
use crate::utils::error::{Result, BlockchainError};
use rand::rngs::OsRng;

pub struct RangeProofSystem {
    bp_gens: BulletproofGens,
    pc_gens: PedersenGens,
}

impl RangeProofSystem {
    pub fn new() -> Self {
        let bp_gens = BulletproofGens::new(64, 1); // Support up to 64-bit values
        let pc_gens = PedersenGens::default();
        
        Self {
            bp_gens,
            pc_gens,
        }
    }
    
    /// Commit to an amount with blinding factor
    pub fn commit_amount(&self, amount: u64, blinding: &Scalar) -> Result<CompressedRistretto> {
        let amount_scalar = Scalar::from(amount);
        let commitment = self.pc_gens.commit(amount_scalar, *blinding);
        Ok(commitment.compress())
    }
    
    /// Generate range proof for a committed value
    pub fn generate_range_proof(
        &self,
        value: u64,
        blinding: &Scalar,
        bit_length: usize,
    ) -> Result<Vec<u8>> {
        let mut transcript = Transcript::new(b"RangeProof");
        
        let (proof, _commitment) = RangeProof::prove_single(
            &self.bp_gens,
            &self.pc_gens,
            &mut transcript,
            value,
            blinding,
            bit_length,
        ).map_err(|e| BlockchainError::Crypto(format!("Range proof generation failed: {}", e)))?;
        
        // Serialize the proof
        let proof_bytes = proof.to_bytes();
        
        Ok(proof_bytes)
    }
    
    /// Verify a range proof
    pub fn verify_range_proof(
        &self,
        proof_bytes: &[u8],
        commitment_bytes: &[u8],
        bit_length: usize,
    ) -> Result<bool> {
        // Deserialize the proof
        let proof = RangeProof::from_bytes(proof_bytes)
            .map_err(|e| BlockchainError::Serialization(format!("Proof deserialization failed: {}", e)))?;
        
        if commitment_bytes.len() != 32 {
            return Err(BlockchainError::Crypto("Invalid commitment length".to_string()));
        }
        
        let mut commitment_array = [0u8; 32];
        commitment_array.copy_from_slice(commitment_bytes);
        let commitment = CompressedRistretto(commitment_array);
        
        let mut transcript = Transcript::new(b"RangeProof");
        
        match proof.verify_single(
            &self.bp_gens,
            &self.pc_gens,
            &mut transcript,
            &commitment,
            bit_length,
        ) {
            Ok(()) => Ok(true),
            Err(_) => Ok(false),
        }
    }
    
    /// Verify multiple range proofs efficiently
    pub fn verify_batch_range_proofs(
        &self,
        proofs: &[Vec<u8>],
        commitments: &[CompressedRistretto],
        bit_lengths: &[usize],
    ) -> Result<bool> {
        if proofs.len() != commitments.len() || proofs.len() != bit_lengths.len() {
            return Err(BlockchainError::Crypto("Mismatched proof batch sizes".to_string()));
        }
        
        // Verify each proof individually
        for (i, ((proof_bytes, commitment), bit_length)) in proofs.iter()
    .zip(commitments.iter())
    .zip(bit_lengths.iter())
    .enumerate() 
        {
            let proof = RangeProof::from_bytes(proof_bytes)
                .map_err(|e| BlockchainError::Serialization(format!("Proof deserialization failed: {}", e)))?;
            
            let mut transcript = Transcript::new(b"RangeProof");
            transcript.append_message(b"index", &i.to_le_bytes());
            
            match proof.verify_single(
                &self.bp_gens,
                &self.pc_gens,
                &mut transcript,
                commitment,
                *bit_length,
            ) {
                Ok(()) => continue,
                Err(_) => return Ok(false),
            }
        }
        
        Ok(true)
    }
    
    /// Generate a random scalar for blinding factors
    pub fn generate_random_scalar(&self) -> Scalar {
        let mut rng = OsRng;
        Scalar::random(&mut rng)
    }
    
    /// Create a commitment and range proof in one operation
    pub fn create_commitment_with_proof(
        &self,
        value: u64,
        bit_length: usize,
    ) -> Result<(CompressedRistretto, Vec<u8>, Scalar)> {
        let blinding = self.generate_random_scalar();
        let commitment = self.commit_amount(value, &blinding)?;
        let proof = self.generate_range_proof(value, &blinding, bit_length)?;
        
        Ok((commitment, proof, blinding))
    }
}

impl Default for RangeProofSystem {
    fn default() -> Self {
        Self::new()
    }
}