use crate::core::transaction::Transaction;
use crate::core::types::Address;
use crate::privacy::range_proofs::RangeProofSystem;
use crate::privacy::stealth_addresses::StealthAddressSystem;
use crate::utils::error::Result;
use curve25519_dalek_ng::ristretto::CompressedRistretto;
use curve25519_dalek_ng::scalar::Scalar;
use merlin::Transcript;
use rand::rngs::OsRng;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConfidentialTransaction {
    pub base_transaction: Transaction,
    pub input_commitments: Vec<CompressedRistretto>,
    pub output_commitments: Vec<CompressedRistretto>,
    pub range_proofs: Vec<Vec<u8>>,
    pub zk_proof: Vec<u8>,
    pub binding_signature: Vec<u8>,
    pub signer_public_key: Vec<u8>,
}

pub struct ConfidentialTxBuilder {
    range_prover: RangeProofSystem,
    stealth_system: StealthAddressSystem,
}

impl ConfidentialTxBuilder {
    pub fn new() -> Self {
        Self {
            range_prover: RangeProofSystem::new(),
            stealth_system: StealthAddressSystem::new(),
        }
    }

    /// Create a confidential transfer transaction
    pub fn create_confidential_transfer(
        &mut self,
        from: Address,
        to: Address,
        amount: u64,
        fee: u64,
        nonce: u64,
        private_key: &[u8],
        public_key: &[u8],
    ) -> Result<ConfidentialTransaction> {
        // Generate stealth address for recipient
        // Placeholder: using recipient address bytes directly; in a real system, decode proper public keys
        let recipient_bytes = to.as_str().as_bytes();
        let mut key_bytes = [0u8; 32];
        for (i, b) in recipient_bytes.iter().take(32).enumerate() {
            key_bytes[i] = *b;
        }
        let recipient_pk = CompressedRistretto(key_bytes);
        let (_stealth_address, _ephemeral_key) = self
            .stealth_system
            .generate_stealth_address(&recipient_pk, &recipient_pk)?;

        // Create Pedersen commitments for amount
        let (amount_commitment, amount_blinding) = self.create_amount_commitment(amount)?;
        let (fee_commitment, fee_blinding) = self.create_amount_commitment(fee)?;

        let input_commitments = vec![amount_commitment.clone()];
        let output_commitments = vec![amount_commitment.clone(), fee_commitment.clone()];

        // Generate range proofs
        let amount_range_proof =
            self.range_prover
                .generate_range_proof(amount, &amount_blinding, 64)?;
        let fee_range_proof = self
            .range_prover
            .generate_range_proof(fee, &fee_blinding, 32)?;

        // Generate ZK proof for transaction validity (simplified)
        let zk_proof = self.generate_confidential_tx_proof()?;

        // Prepare binary representations for transaction payload
        let input_commitments_bytes: Vec<Vec<u8>> = input_commitments
            .iter()
            .map(|commitment| commitment.to_bytes().to_vec())
            .collect();
        let output_commitments_bytes: Vec<Vec<u8>> = output_commitments
            .iter()
            .map(|commitment| commitment.to_bytes().to_vec())
            .collect();
        let range_proofs_bytes = vec![amount_range_proof.clone(), fee_range_proof.clone()];

        // Create base confidential transaction with placeholder signature
        let mut base_tx = Transaction::new_confidential_transfer(
            from.clone(),
            to.clone(),
            0, // Public amount stays hidden for confidential transfers
            fee,
            nonce,
            zk_proof.clone(),
            input_commitments_bytes,
            output_commitments_bytes,
            range_proofs_bytes,
            Vec::new(),
        );

        // Ensure binding signature is excluded from hashing when generating signature
        base_tx.binding_signature = None;

        // Create binding signature
        let binding_signature = self.create_binding_signature(
            &base_tx,
            &input_commitments,
            &output_commitments,
            private_key,
        )?;

        base_tx.binding_signature = Some(binding_signature.clone());

        let confidential_tx = ConfidentialTransaction {
            base_transaction: base_tx,
            input_commitments,
            output_commitments,
            range_proofs: vec![amount_range_proof, fee_range_proof],
            zk_proof,
            binding_signature,
            signer_public_key: public_key.to_vec(),
        };

        Ok(confidential_tx)
    }

    fn create_amount_commitment(&self, amount: u64) -> Result<(CompressedRistretto, Scalar)> {
        let blinding = self.generate_random_scalar();
        let commitment = self.range_prover.commit_amount(amount, &blinding)?;
        Ok((commitment, blinding))
    }

    fn generate_confidential_tx_proof(&self) -> Result<Vec<u8>> {
        // Simplified proof generation - in real implementation this would use proper ZK circuits
        let proof_data = b"simplified_zk_proof".to_vec();
        Ok(proof_data)
    }

    fn create_binding_signature(
        &self,
        transaction: &Transaction,
        input_commitments: &[CompressedRistretto],
        output_commitments: &[CompressedRistretto],
        private_key: &[u8],
    ) -> Result<Vec<u8>> {
        let mut transcript = Transcript::new(b"BindingSignature");

        let mut tx_clone = transaction.clone();
        tx_clone.binding_signature = None;

        // Add transaction data to transcript
        transcript.append_message(b"tx_hash", tx_clone.hash().as_bytes());

        for commitment in input_commitments {
            transcript.append_message(b"input_commitment", commitment.as_bytes());
        }

        for commitment in output_commitments {
            transcript.append_message(b"output_commitment", commitment.as_bytes());
        }

        // Generate signature using Dilithium
        use crate::crypto::dilithium::Dilithium;

        let mut challenge = [0u8; 64];
        transcript.challenge_bytes(b"signature_challenge", &mut challenge);

        Dilithium::sign(private_key, &challenge)
    }

    /// Verify a confidential transaction
    pub fn verify_confidential_transaction(
        &self,
        confidential_tx: &ConfidentialTransaction,
    ) -> Result<bool> {
        // Verify range proofs
        for (i, (commitment, proof)) in confidential_tx
            .output_commitments
            .iter()
            .zip(confidential_tx.range_proofs.iter())
            .enumerate()
        {
            let bit_length = if i == 0 { 64 } else { 32 }; // Amount vs fee
            if !self
                .range_prover
                .verify_range_proof(proof, commitment.as_bytes(), bit_length)?
            {
                return Ok(false);
            }
        }

        // Verify binding signature
        if !self.verify_binding_signature(confidential_tx)? {
            return Ok(false);
        }

        Ok(true)
    }

    fn verify_binding_signature(&self, confidential_tx: &ConfidentialTransaction) -> Result<bool> {
        let mut transcript = Transcript::new(b"BindingSignature");

        let mut tx_clone = confidential_tx.base_transaction.clone();
        tx_clone.binding_signature = None;

        transcript.append_message(b"tx_hash", tx_clone.hash().as_bytes());

        for commitment in &confidential_tx.input_commitments {
            transcript.append_message(b"input_commitment", commitment.as_bytes());
        }

        for commitment in &confidential_tx.output_commitments {
            transcript.append_message(b"output_commitment", commitment.as_bytes());
        }

        let mut challenge = [0u8; 64];
        transcript.challenge_bytes(b"signature_challenge", &mut challenge);

        if confidential_tx.signer_public_key.is_empty() {
            return Ok(false);
        }

        use crate::crypto::dilithium::Dilithium;
        Dilithium::verify(
            &confidential_tx.signer_public_key,
            &challenge,
            &confidential_tx.binding_signature,
        )
    }

    fn generate_random_scalar(&self) -> Scalar {
        let mut rng = OsRng;
        Scalar::random(&mut rng)
    }
}

impl Default for ConfidentialTxBuilder {
    fn default() -> Self {
        Self::new()
    }
}
