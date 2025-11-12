// src/bridges/zk_proofs/mod.rs
pub mod inclusion_proofs;
pub mod state_proofs;
pub mod bridge_circuits;
pub mod verifier;
pub mod types;

pub use inclusion_proofs::{InclusionProofVerifier, MerkleInclusionProof};
pub use state_proofs::{StateProofVerifier, CrossChainStateProof};
pub use bridge_circuits::{BridgeCircuit, TransferCircuit, ValidityCircuit};
pub use verifier::ZkProofVerifier;
pub use types::*;