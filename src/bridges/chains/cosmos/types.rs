// src/bridges/chains/cosmos/types.rs
use serde::{Deserialize, Serialize};

/// Information for monitoring a Cosmos address and specific message types
#[derive(Debug, Clone)]
pub struct WatchInfo {
    pub address: String,
    pub message_types: Vec<String>,
}

/// Represents a Cosmos transaction with decoded messages
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CosmosTransaction {
    pub hash: String,
    pub height: u64,
    pub messages: Vec<CosmosMessage>,
    pub fee: CosmosFee,
    pub memo: String,
    pub success: bool,
}

/// Individual message within a Cosmos transaction
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CosmosMessage {
    pub type_url: String,
    pub value: Vec<u8>,
    pub sender: String,
}

/// Transaction fee information for Cosmos
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CosmosFee {
    pub amount: Vec<CosmosCoin>,
    pub gas_limit: u64,
}

/// Cosmos coin representation with denomination and amount
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CosmosCoin {
    pub denom: String,
    pub amount: String,
}

/// Response from broadcasting a transaction
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BroadcastTxResponse {
    pub hash: String,
    pub height: u64,
    pub code: u32,
    pub raw_log: String,
}

/// Configuration for Cosmos adapter connection
#[derive(Debug, Clone)]
pub struct CosmosConfig {
    pub rpc_endpoint: String,
    pub grpc_endpoint: Option<String>,
    pub chain_id: String,
    pub prefix: String,
    pub gas_adjustment: f64,
    pub gas_prices: Vec<CosmosCoin>,
}

/// IBC packet data for cross-chain transfers
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IbcPacket {
    pub source_port: String,
    pub source_channel: String,
    pub destination_port: String,
    pub destination_channel: String,
    pub data: Vec<u8>,
    pub timeout_height: u64,
    pub timeout_timestamp: u64,
}

/// Broadcast modes for transaction submission
#[derive(Debug, Clone)]
pub enum BroadcastMode {
    Sync,
    Async,
    Block,
}

/// Cosmos-specific error types for the bridge adapter
#[derive(Debug, thiserror::Error)]
pub enum CosmosError {
    #[error("Connection error: {0}")]
    Connection(String),
    #[error("RPC error: {0}")]
    RpcError(String),
    #[error("GRPC error: {0}")]
    GrpcError(String),
    #[error("Broadcast error: {0}")]
    BroadcastError(String),
    #[error("Transaction not found: {0}")]
    TransactionNotFound(String),
    #[error("Invalid address: {0}")]
    InvalidAddress(String),
    #[error("Parse error: {0}")]
    ParseError(String),
    #[error("IBC error: {0}")]
    IbcError(String),
    #[error("Serialization error: {0}")]
    SerializationError(String),
    #[error("Query error: {0}")]
    QueryError(String),
}

// Implement From traits for error conversion
impl From<tendermint_rpc::Error> for CosmosError {
    fn from(error: tendermint_rpc::Error) -> Self {
        CosmosError::RpcError(error.to_string())
    }
}

impl From<tonic::transport::Error> for CosmosError {
    fn from(error: tonic::transport::Error) -> Self {
        CosmosError::GrpcError(error.to_string())
    }
}

impl From<cosmrs::Error> for CosmosError {
    fn from(error: cosmrs::Error) -> Self {
        CosmosError::SerializationError(error.to_string())
    }
}

impl From<prost::DecodeError> for CosmosError {
    fn from(error: prost::DecodeError) -> Self {
        CosmosError::ParseError(error.to_string())
    }
}