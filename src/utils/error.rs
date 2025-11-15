// src/utils/error.rs

use thiserror::Error;
use std::{io, error::Error as StdError};
use crate::core::types::AddressError;

#[derive(Error, Debug)]
pub enum BlockchainError {
    #[error("I/O error: {0}")]
    Io(#[from] io::Error),

    #[error("Serialization error: {0}")]
    Serialization(String),

    #[error("Cryptographic error: {0}")]
    Crypto(String),

    #[error("Consensus error: {0}")]
    Consensus(String),

    #[error("Network error: {0}")]
    Network(String),

    #[error("Insufficient funds error")]
    InsufficientFunds,

    #[error("Invalid nonce error")]
    InvalidNonce,

    #[error("No signatures error")]
    NoSignatures,

    #[error("Insufficient signatures error")]
    InsufficientSignatures,

    #[error("Invalid gas estimate error")]
    InvalidGasEstimate,

    #[error("Template not found error")]
    TemplateNotFound,

    #[error("Storage error: {0}")]
    Storage(String),

    #[error("VM execution error: {0}")]
    VM(String),

    #[error("Invalid transaction: {0}")]
    InvalidTransaction(String),

    #[error("Invalid block: {0}")]
    InvalidBlock(String),

    #[error("Validator error: {0}")]
    Validator(String),

    #[error("Governance error: {0}")]
    Governance(String),

    #[error("Analytics error: {0}")]
    Analytics(String),

    #[error("Security error: {0}")]
    Security(String),

    #[error("Encryption error: {0}")]
    Encryption(String),

    #[error("Compliance error: {0}")]
    Compliance(String),

    #[error("Bridge error: {0}")]
    Bridge(String),

    #[error("Address error: {0}")]
    Address(#[from] AddressError),

    #[error("Database error: {0}")]
    Database(String),

    #[error("Other error: {0}")]
    Other(String),
}

pub type Result<T> = std::result::Result<T, BlockchainError>;

impl From<parity_db::Error> for BlockchainError {
    fn from(err: parity_db::Error) -> Self {
        BlockchainError::Database(format!("{}", err))
    }
}

impl From<hex::FromHexError> for BlockchainError {
    fn from(err: hex::FromHexError) -> Self {
        BlockchainError::Crypto(format!("Hex decoding error: {}", err))
    }
}

impl From<Box<bincode::ErrorKind>> for BlockchainError {
    fn from(err: Box<bincode::ErrorKind>) -> Self {
        BlockchainError::Serialization(format!("Bincode error: {}", err))
    }
}

impl From<Box<dyn StdError>> for BlockchainError {
    fn from(err: Box<dyn StdError>) -> Self {
        BlockchainError::Other(err.to_string())
    }
}

impl From<libp2p::swarm::ConnectionDenied> for BlockchainError {
    fn from(err: libp2p::swarm::ConnectionDenied) -> Self {
        BlockchainError::Network(format!("Connection denied: {}", err))
    }
}

impl From<libp2p::dns::ResolveError> for BlockchainError {
    fn from(err: libp2p::dns::ResolveError) -> Self {
        BlockchainError::Network(format!("DNS resolution error: {}", err))
    }
}

impl From<argon2::password_hash::Error> for BlockchainError {
    fn from(err: argon2::password_hash::Error) -> Self {
        BlockchainError::Encryption(format!("Argon2 error: {}", err))
    }
}
