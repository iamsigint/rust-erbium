// src/bridges/light_clients/errors.rs
use thiserror::Error;

/// Light client error types
#[derive(Debug, Error)]
pub enum LightClientError {
    #[error("Header not found at height: {0}")]
    HeaderNotFound(u64),
    #[error("Invalid header: {0}")]
    InvalidHeader(String),
    #[error("Invalid proof of work")]
    InvalidProofOfWork,
    #[error("Invalid finality proof")]
    InvalidFinalityProof,
    #[error("Invalid commit")]
    InvalidCommit,
    #[error("Invalid Merkle proof")]
    InvalidMerkleProof,
    #[error("Invalid state proof")]
    InvalidStateProof,
    #[error("Verification failed: {0}")]
    VerificationFailed(String),
    #[error("Chain not supported: {0}")]
    ChainNotSupported(String),
    #[error("IO error: {0}")]
    IoError(String),
    #[error("Serialization error: {0}")]
    SerializationError(String),
}