use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::fmt;
use std::str::FromStr;

/// Hash type used throughout the blockchain - 32 bytes
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, PartialOrd, Ord)]
pub struct Hash([u8; 32]);

impl Hash {
    /// Creates a new SHA-256 hash from data
    pub fn new(data: &[u8]) -> Self {
        let mut hasher = Sha256::new();
        hasher.update(data);
        Hash(hasher.finalize().into())
    }

    /// Creates a hash from raw bytes (must be 32 bytes)
    pub fn from_bytes(bytes: [u8; 32]) -> Self {
        Hash(bytes)
    }

    /// Returns the hash as a byte array reference
    pub fn as_bytes(&self) -> &[u8; 32] {
        &self.0
    }

    /// Converts hash to hexadecimal string
    pub fn to_hex(&self) -> String {
        hex::encode(self.0)
    }

    /// Converts hash to byte vector
    pub fn to_vec(&self) -> Vec<u8> {
        self.0.to_vec()
    }

    /// Creates a hash from a hexadecimal string
    pub fn from_hex(hex_str: &str) -> Result<Self, hex::FromHexError> {
        let bytes = hex::decode(hex_str)?;
        if bytes.len() != 32 {
            return Err(hex::FromHexError::InvalidStringLength);
        }

        let mut array = [0u8; 32];
        array.copy_from_slice(&bytes);
        Ok(Hash(array))
    }

    /// Zero hash (all zeros)
    pub fn zero() -> Self {
        Hash([0u8; 32])
    }

    /// Check if hash is zero
    pub fn is_zero(&self) -> bool {
        self.0 == [0u8; 32]
    }
}

impl fmt::Display for Hash {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.to_hex())
    }
}

impl From<[u8; 32]> for Hash {
    fn from(bytes: [u8; 32]) -> Self {
        Hash(bytes)
    }
}

impl From<Hash> for [u8; 32] {
    fn from(hash: Hash) -> Self {
        hash.0
    }
}

impl AsRef<[u8]> for Hash {
    fn as_ref(&self) -> &[u8] {
        &self.0
    }
}

impl FromStr for Hash {
    type Err = hex::FromHexError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Hash::from_hex(s)
    }
}

/// Address type for accounts with validation
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct Address(String);

impl Address {
    /// Creates a new address with basic validation
    pub fn new(addr: String) -> Result<Self, AddressError> {
        if addr.len() != 42 || !addr.starts_with("0x") {
            return Err(AddressError::InvalidFormat);
        }

        // Basic hex validation
        if hex::decode(&addr[2..]).is_err() {
            return Err(AddressError::InvalidHex);
        }

        Ok(Address(addr))
    }

    /// Creates an address without validation (use carefully)
    pub fn new_unchecked(addr: String) -> Self {
        Address(addr)
    }

    /// Returns address as string slice
    pub fn as_str(&self) -> &str {
        &self.0
    }

    /// Returns address as string
    pub fn as_string(&self) -> String {
        self.0.clone()
    }

    /// Returns the raw bytes of the address (without 0x prefix)
    pub fn to_bytes(&self) -> Result<Vec<u8>, hex::FromHexError> {
        hex::decode(&self.0[2..])
    }

    /// Creates address from bytes (adds 0x prefix)
    pub fn from_bytes(bytes: &[u8]) -> Self {
        let hex_str = hex::encode(bytes);
        Address(format!("0x{}", hex_str))
    }

    /// Check if address is zero address
    pub fn is_zero(&self) -> bool {
        self.0 == "0x0000000000000000000000000000000000000000"
    }
}

impl fmt::Display for Address {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl From<String> for Address {
    fn from(s: String) -> Self {
        Address::new_unchecked(s)
    }
}

impl From<&str> for Address {
    fn from(s: &str) -> Self {
        Address::new_unchecked(s.to_string())
    }
}

impl AsRef<str> for Address {
    fn as_ref(&self) -> &str {
        &self.0
    }
}

/// Address validation errors
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum AddressError {
    InvalidFormat,
    InvalidHex,
    InvalidLength,
}

impl fmt::Display for AddressError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            AddressError::InvalidFormat => {
                write!(f, "Address must start with 0x and be 42 characters long")
            }
            AddressError::InvalidHex => {
                write!(f, "Address contains invalid hexadecimal characters")
            }
            AddressError::InvalidLength => write!(f, "Address has invalid length"),
        }
    }
}

impl std::error::Error for AddressError {}

/// Timestamp in milliseconds since UNIX epoch
pub type Timestamp = u64;

/// Difficulty for consensus (Proof of Work/Stake)
pub type Difficulty = u64;

/// Nonce for mining/validation
#[derive(
    Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, PartialOrd, Ord, Hash, Default,
)]
pub struct Nonce(pub u64);

impl Nonce {
    /// Creates a new nonce
    pub fn new(value: u64) -> Self {
        Nonce(value)
    }

    /// Returns the inner value
    pub fn value(&self) -> u64 {
        self.0
    }

    /// Increments the nonce by 1
    pub fn increment(&mut self) {
        self.0 += 1;
    }

    /// Zero nonce
    pub fn zero() -> Self {
        Nonce(0)
    }
}

impl From<u64> for Nonce {
    fn from(n: u64) -> Self {
        Nonce(n)
    }
}

impl From<Nonce> for u64 {
    fn from(nonce: Nonce) -> Self {
        nonce.0
    }
}

impl fmt::Display for Nonce {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}

/// Amount type for transactions (in smallest unit, e.g., wei/satoshi)
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, PartialOrd, Ord)]
pub struct Amount(pub u128);

impl Amount {
    /// Creates a new amount
    pub fn new(value: u128) -> Self {
        Amount(value)
    }

    /// Zero amount
    pub fn zero() -> Self {
        Amount(0)
    }

    /// Check if amount is zero
    pub fn is_zero(&self) -> bool {
        self.0 == 0
    }

    /// Add two amounts
    pub fn checked_add(self, other: Self) -> Option<Self> {
        self.0.checked_add(other.0).map(Amount)
    }

    /// Subtract two amounts
    pub fn checked_sub(self, other: Self) -> Option<Self> {
        self.0.checked_sub(other.0).map(Amount)
    }

    /// Multiply amount by scalar
    pub fn checked_mul(self, scalar: u128) -> Option<Self> {
        self.0.checked_mul(scalar).map(Amount)
    }
}

impl From<u128> for Amount {
    fn from(value: u128) -> Self {
        Amount(value)
    }
}

impl From<u64> for Amount {
    fn from(value: u64) -> Self {
        Amount(value as u128)
    }
}

impl fmt::Display for Amount {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}

/// Block height type
pub type BlockHeight = u64;

/// Gas units for transaction execution
pub type Gas = u64;

/// Gas price in smallest unit
pub type GasPrice = u64;

/// Transaction version
pub type Version = u16;

/// Chain identifier
pub type ChainId = u32;

/// Merkle root type (alias for Hash)
pub type MerkleRoot = Hash;

/// State root type (alias for Hash)
pub type StateRoot = Hash;

/// Transaction hash type (alias for Hash)
pub type TransactionHash = Hash;

/// Block hash type (alias for Hash)
pub type BlockHash = Hash;

/// Public key bytes
pub type PublicKey = Vec<u8>;

/// Signature bytes
pub type Signature = Vec<u8>;

/// Cryptographic seed for key generation
pub type Seed = [u8; 32];

/// Database key type
pub type DbKey = Vec<u8>;

/// Database value type
pub type DbValue = Vec<u8>;

// Remove duplicate BlockchainError definition - it's already defined in utils/error.rs

// Unit tests
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_hash_operations() {
        let data = b"hello world";
        let hash = Hash::new(data);

        // Test hex conversion
        let hex_str = hash.to_hex();
        let hash_from_hex = Hash::from_hex(&hex_str).unwrap();
        assert_eq!(hash, hash_from_hex);

        // Test zero hash
        let zero_hash = Hash::zero();
        assert!(zero_hash.is_zero());

        // Test byte conversion
        let bytes = hash.as_bytes();
        let hash_from_bytes = Hash::from_bytes(*bytes);
        assert_eq!(hash, hash_from_bytes);
    }

    #[test]
    fn test_address_validation() {
        // Valid address
        let valid_addr = "0x742d35Cc6634C0532925a3b8D4a5b1a4b6c6d7e8";
        let address = Address::new(valid_addr.to_string()).unwrap();
        assert_eq!(address.as_str(), valid_addr);

        // Invalid format
        let invalid_addr = "invalid";
        assert!(Address::new(invalid_addr.to_string()).is_err());

        // Invalid length
        let short_addr = "0x1234";
        assert!(Address::new(short_addr.to_string()).is_err());
    }

    #[test]
    fn test_nonce_operations() {
        let mut nonce = Nonce::default();
        assert_eq!(nonce.value(), 0);

        nonce.increment();
        assert_eq!(nonce.value(), 1);

        let nonce_from_u64 = Nonce::from(42);
        assert_eq!(nonce_from_u64.value(), 42);
    }

    #[test]
    fn test_amount_operations() {
        let amount1 = Amount::new(100);
        let amount2 = Amount::new(50);

        let sum = amount1.checked_add(amount2).unwrap();
        assert_eq!(sum, Amount::new(150));

        let diff = amount1.checked_sub(amount2).unwrap();
        assert_eq!(diff, Amount::new(50));

        let zero = Amount::zero();
        assert!(zero.is_zero());
    }
}
