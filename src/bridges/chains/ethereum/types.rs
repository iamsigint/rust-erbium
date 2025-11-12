// src/bridges/chains/ethereum/types.rs
use web3::types::{Address, H256, U256, Transaction, Block, Log, TransactionReceipt};
use serde::{Deserialize, Serialize};

/// Configuration for Ethereum adapter connection
#[derive(Debug, Clone)]
pub struct EthereumConfig {
    pub rpc_endpoint: String,
    pub chain_id: u64,
    pub gas_limit: U256,
    pub gas_price: U256,
    pub confirmations_required: u32,
}

/// Represents an Ethereum transaction with relevant bridge information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EthereumTransaction {
    pub hash: H256,
    pub from: Address,
    pub to: Option<Address>,
    pub value: U256,
    pub input: Vec<u8>,
    pub block_number: u64,
    pub block_hash: H256,
    pub logs: Vec<Log>,
}

/// ERC20 token transfer event data
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Erc20Transfer {
    pub token_address: Address,
    pub from: Address,
    pub to: Address,
    pub value: U256,
    pub transaction_hash: H256,
}

/// Proof data for verifying Ethereum transaction inclusion
#[derive(Debug, Clone)]
pub struct EthereumTransactionProof {
    pub transaction: Transaction,
    pub receipt: TransactionReceipt,
    pub block: Block<H256>,
    pub state_proof: Vec<u8>, // For Ethereum, state proofs are used instead of merkle proofs
}

/// Contract watch information for monitoring specific contracts
#[derive(Debug, Clone)]
pub struct ContractWatchInfo {
    pub contract_name: String,
    pub contract_address: Address,
    pub event_signatures: Vec<H256>,
}

/// ERC20 token information and ABI
#[derive(Debug, Clone)]
pub struct Erc20TokenInfo {
    pub address: Address,
    pub name: String,
    pub symbol: String,
    pub decimals: u8,
    pub total_supply: U256,
}

/// Call request for contract interactions
#[derive(Debug, Clone)]
pub struct ContractCall {
    pub to: Address,
    pub data: Vec<u8>,
    pub value: U256,
    pub gas_limit: Option<U256>,
}

/// Transaction parameters for sending transactions
#[derive(Debug, Clone)]
pub struct TransactionParams {
    pub to: Address,
    pub value: U256,
    pub data: Vec<u8>,
    pub gas_limit: U256,
    pub gas_price: U256,
    pub nonce: Option<U256>,
}

/// Ethereum-specific error types for the bridge adapter
#[derive(Debug, thiserror::Error)]
pub enum EthereumError {
    #[error("Connection error: {0}")]
    Connection(String),
    #[error("RPC error: {0}")]
    RpcError(String),
    #[error("Transaction not found: {0}")]
    TransactionNotFound(String),
    #[error("Block not found: {0}")]
    BlockNotFound(String),
    #[error("Contract call error: {0}")]
    ContractCall(String),
    #[error("Invalid address")]
    InvalidAddress,
    #[error("ABI decoding error: {0}")]
    AbiDecodeError(String),
    #[error("Invalid transaction parameters: {0}")]
    InvalidTransaction(String),
    #[error("Insufficient funds")]
    InsufficientFunds,
    #[error("Gas estimation failed: {0}")]
    GasEstimationFailed(String),
    #[error("Event decoding failed: {0}")]
    EventDecodingFailed(String),
}

// Implement From traits for error conversion
impl From<web3::Error> for EthereumError {
    fn from(error: web3::Error) -> Self {
        EthereumError::RpcError(error.to_string())
    }
}

impl From<web3::contract::Error> for EthereumError {
    fn from(error: web3::contract::Error) -> Self {
        EthereumError::ContractCall(error.to_string())
    }
}

impl From<std::io::Error> for EthereumError {
    fn from(error: std::io::Error) -> Self {
        EthereumError::Connection(error.to_string())
    }
}