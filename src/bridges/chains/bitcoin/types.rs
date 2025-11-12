// src/bridges/chains/bitcoin/types.rs
use bitcoin::{Transaction, Network};
use serde::{Deserialize, Serialize};

/// Information for monitoring a Bitcoin address
#[derive(Debug, Clone)]
pub struct WatchInfo {
    pub address: String,
    pub confirmations_required: u32,
    pub callback_url: Option<String>,
}

/// Represents a Bitcoin transaction with relevant bridge information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BitcoinTransaction {
    pub txid: String,
    pub amount: u64,
    pub sender: String,
    pub recipient: String,
    pub confirmations: u32,
    pub block_hash: Option<String>,
    pub vout: u32,
}

/// Proof data for verifying Bitcoin transaction inclusion in a block
#[derive(Debug, Clone)]
pub struct BitcoinTransactionProof {
    pub txid: String,
    pub transaction: Transaction,
    pub block_header: Vec<u8>, // Serialized block header
    pub merkle_proof: Vec<u8>,
    pub confirmations: u32,
}

/// Unsigned Bitcoin transaction ready for signing
#[derive(Debug, Clone)]
pub struct UnsignedBitcoinTransaction {
    pub recipient: String,
    pub amount: u64,
    pub fee_rate: f64,
    pub inputs: Vec<BitcoinUtxo>,
    pub change_address: Option<String>,
}

/// Unspent Transaction Output (UTXO) for Bitcoin
#[derive(Debug, Clone)]
pub struct BitcoinUtxo {
    pub txid: String,
    pub vout: u32,
    pub amount: u64,
    pub script_pubkey: Vec<u8>,
}

/// Configuration for Bitcoin adapter connection
#[derive(Debug, Clone)]
pub struct BitcoinConfig {
    pub rpc_host: String,
    pub rpc_port: u16,
    pub rpc_user: String,
    pub rpc_pass: String,
    pub network: Network,
    pub confirmations_required: u32,
}

/// Bitcoin-specific error types for the bridge adapter
#[derive(Debug, thiserror::Error)]
pub enum BitcoinError {
    #[error("RPC connection error: {0}")]
    RpcConnection(String),
    #[error("Invalid Bitcoin address: {0}")]
    InvalidAddress(String),
    #[error("Address not being watched: {0}")]
    AddressNotWatched(String),
    #[error("Transaction not found: {0}")]
    TransactionNotFound(String),
    #[error("Invalid transaction ID: {0}")]
    InvalidTxid(String),
    #[error("Block not found: {0}")]
    BlockNotFound(String),
    #[error("Insufficient funds")]
    InsufficientFunds,
    #[error("Network mismatch")]
    NetworkMismatch,
    #[error("IO error: {0}")]
    IoError(String),
    #[error("Serialization error: {0}")]
    SerializationError(String),
}

// Implement From traits for error conversion
impl From<bitcoincore_rpc::Error> for BitcoinError {
    fn from(error: bitcoincore_rpc::Error) -> Self {
        BitcoinError::RpcConnection(error.to_string())
    }
}

impl From<bitcoin::consensus::encode::Error> for BitcoinError {
    fn from(error: bitcoin::consensus::encode::Error) -> Self {
        BitcoinError::SerializationError(error.to_string())
    }
}

impl From<std::io::Error> for BitcoinError {
    fn from(error: std::io::Error) -> Self {
        BitcoinError::IoError(error.to_string())
    }
}

// Remove the From impl for bitcoin::hashes::hex::Error as it doesn't exist in this version
