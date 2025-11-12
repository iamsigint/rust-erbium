// src/bridges/chains/polkadot/types.rs
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Configuration for Polkadot adapter connection
#[derive(Debug, Clone)]
pub struct PolkadotConfig {
    pub rpc_endpoint: String,
    pub chain_id: String,
    pub ss58_format: u16,
    pub types_registry: Option<serde_json::Value>,
}

/// Represents a Polkadot extrinsic with relevant bridge information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PolkadotExtrinsic {
    pub hash: String,
    pub module: String,
    pub method: String,
    pub signer: String,
    pub params: HashMap<String, serde_json::Value>,
    pub block_number: u64,
    pub success: bool,
}

/// Runtime version information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RuntimeVersion {
    pub spec_name: String,
    pub spec_version: u32,
    pub impl_version: u32,
    pub transaction_version: u32,
}

/// Chain metadata for decoding extrinsics and events
#[derive(Debug, Clone)]
pub struct Metadata {
    pub raw: Vec<u8>,
    pub modules: Vec<Pallet>,
    pub version: u32,
}

/// Pallet information from metadata
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Pallet {
    pub name: String,
    pub index: u8,
    pub calls: Vec<Call>,
    pub events: Vec<Event>,
    pub constants: Vec<Constant>,
}

/// Call information for a pallet
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Call {
    pub name: String,
    pub fields: Vec<Field>,
    pub index: u8,
}

/// Event information for a pallet
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Event {
    pub name: String,
    pub fields: Vec<Field>,
    pub index: u8,
}

/// Constant information for a pallet
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Constant {
    pub name: String,
    pub value: Vec<u8>,
    pub type_name: String,
}

/// Field information for calls and events
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Field {
    pub name: String,
    pub type_name: String,
    pub type_id: u32,
}

/// XCM message for cross-chain communication
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct XcmMessage {
    pub version: u32,
    pub instructions: Vec<XcmInstruction>,
    pub source: String,
    pub destination: String,
}

/// XCM instruction types
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum XcmInstruction {
    WithdrawAsset { assets: Vec<XcmAsset>, effects: Vec<XcmOrder> },
    DepositAsset { assets: Vec<XcmAsset>, dest: String },
    TransferReserveAsset { assets: Vec<XcmAsset>, dest: String, effects: Vec<XcmOrder> },
    // Other XCM instructions would be added here
}

/// XCM asset representation
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct XcmAsset {
    pub id: XcmAssetId,
    pub fungibility: XcmFungibility,
}

/// XCM asset identifier
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum XcmAssetId {
    Concrete { parents: u8, interior: Vec<XcmJunction> },
    Abstract(Vec<u8>),
}

/// XCM junction for location specification
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum XcmJunction {
    Parachain(u32),
    AccountId32 { network: u8, id: [u8; 32] },
    AccountIndex64 { network: u8, index: u64 },
    AccountKey20 { network: u8, key: [u8; 20] },
    PalletInstance(u8),
    GeneralIndex(u128),
    GeneralKey(Vec<u8>),
    OnlyChild,
}

/// XCM fungibility type
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum XcmFungibility {
    Fungible(u128),
    NonFungible(Vec<u8>),
}

/// XCM order for asset handling
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum XcmOrder {
    Noop,
    DepositAsset { assets: Vec<XcmAsset>, dest: String },
    DepositReserveAsset { assets: Vec<XcmAsset>, dest: String, effects: Vec<XcmOrder> },
    ExchangeAsset { give: Vec<XcmAsset>, receive: Vec<XcmAsset> },
}

/// Storage query response
#[derive(Debug, Clone)]
pub struct StorageResponse {
    pub key: String,
    pub value: Option<Vec<u8>>,
    pub proof: Option<Vec<Vec<u8>>>,
}

/// Polkadot-specific error types for the bridge adapter
#[derive(Debug, thiserror::Error)]
pub enum PolkadotError {
    #[error("Connection error: {0}")]
    Connection(String),
    #[error("RPC error: {0}")]
    RpcError(String),
    #[error("Block not found")]
    BlockNotFound,
    #[error("Metadata error: {0}")]
    MetadataError(String),
    #[error("Parse error: {0}")]
    ParseError(String),
    #[error("Extrinsic failed")]
    ExtrinsicFailed,
    #[error("Storage key not found")]
    StorageKeyNotFound,
    #[error("XCM error: {0}")]
    XcmError(String),
    #[error("Scale codec error: {0}")]
    ScaleCodecError(String),
    #[error("Invalid parameters: {0}")]
    InvalidParameters(String),
}

// Implement From traits for error conversion
impl From<jsonrpsee::core::client::Error> for PolkadotError {
    fn from(error: jsonrpsee::core::client::Error) -> Self {
        PolkadotError::RpcError(error.to_string())
    }
}

impl From<parity_scale_codec::Error> for PolkadotError {
    fn from(error: parity_scale_codec::Error) -> Self {
        PolkadotError::ScaleCodecError(error.to_string())
    }
}

impl From<hex::FromHexError> for PolkadotError {
    fn from(error: hex::FromHexError) -> Self {
        PolkadotError::ParseError(error.to_string())
    }
}

impl From<std::num::ParseIntError> for PolkadotError {
    fn from(error: std::num::ParseIntError) -> Self {
        PolkadotError::ParseError(error.to_string())
    }
}