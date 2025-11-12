# 🏗️ Erbium Blockchain - Technical Architecture

## Overview

The Erbium Blockchain is a next-generation blockchain that combines Proof-of-Stake (PoS), advanced privacy with Zero-Knowledge Proofs (ZKP), smart contracts, on-chain governance, and cross-chain bridges. This documentation describes the current system architecture.

## 🏛️ Main Components

### 1. Core Blockchain (`src/core/`)

#### Data Structures
- **Block**: Block structure with header, transactions, and state
- **Transaction**: Transaction system with support for multiple types
- **State**: Global blockchain state management
- **Chain**: Chain validation and processing logic

#### Transaction Types
- **Transfer**: Standard token transfers
- **ConfidentialTransfer**: Private transfers with ZKP
- **ContractDeployment**: Smart contract deployment
- **ContractCall**: Calls to existing contracts
- **Stake**: Token staking for validation
- **Unstake**: Stake removal
- **Vote**: Voting on governance proposals

#### Halving System
```rust
pub const INITIAL_BLOCK_REWARD: u128 = 15 * ERB_DECIMALS; // 15 ERB
pub const HALVING_INTERVAL_BLOCKS: u64 = 1_051_200; // ~4 years @30s

impl RewardPolicy {
    pub fn reward_for_height(&self, height: u64) -> u128 {
        let halvings = height / self.halving_interval_blocks;
        self.initial_block_reward.checked_shr(halvings as u32).unwrap_or(0)
    }
}
```

### 2. Consensus (`src/consensus/`)

#### Proof-of-Stake (PoS)
- **Validation**: Stake-based validator system
- **Rewards**: Distribution based on participation
- **Slashing**: Penalties for malicious behavior
- **Randomness**: Randomness generation for validator selection

#### Consensus Configuration
```rust
pub struct ConsensusConfig {
    pub block_time: u64,           // 30 seconds
    pub epoch_length: u64,         // 7200 blocks (~24h)
    pub max_validators: u64,       // 100 validators
    pub min_stake: u64,            // 10,000 ERB minimum
    pub slashing_percentage: u8,   // 100% zero tolerance
}
```

### 3. Privacy (`src/privacy/`)

#### Zero-Knowledge Proofs (ZKP)
- **Bulletproofs**: For range proofs and confidential transactions
- **Circuits**: Customizable ZK circuits
- **Stealth Addresses**: Ephemeral addresses for privacy

#### Confidential Transactions
```rust
pub struct Transaction {
    // ... standard fields
    pub zk_proof: Option<Vec<u8>>,
    pub input_commitments: Option<Vec<Vec<u8>>>,
    pub output_commitments: Option<Vec<Vec<u8>>>,
    pub range_proofs: Option<Vec<Vec<u8>>>,
    pub binding_signature: Option<Vec<u8>>,
}
```

### 4. Governance (`src/governance/`)

#### DAO System
- **Proposals**: Proposal creation and voting
- **Voting**: Voting system with stake-based weights
- **Treasury**: Community fund management
- **Timelock**: Delayed execution of approved proposals

#### Governance Configuration
```rust
pub struct GovernanceConfig {
    pub voting_delay: u64,        // 1 block delay
    pub voting_period: u64,       // 17280 blocks (~1 day)
    pub proposal_threshold: u64,  // 10,000 tokens to propose
    pub quorum_votes: u64,        // 400,000 minimum votes
    pub timelock_delay: u64,      // No timelock by default
}
```

### 5. Cross-Chain Bridges (`src/bridges/`)

#### Bridge Management
- **Multi-Chain**: Support for Bitcoin, Ethereum, Polkadot, Cosmos
- **Asset Registry**: Cross-chain asset registration
- **Fee Calculator**: Dynamic fee calculation
- **Message Verifier**: Cross-chain message verification

#### Bridge Security
- **Light Clients**: Trustless verification
- **ZK Proofs**: Inclusion and state proofs
- **Monitoring**: Suspicious activity detection
- **Slashing**: Penalties for malicious behavior

### 6. Virtual Machine (VM) (`src/vm/`)

#### Smart Contracts
- **ERC20**: Standard fungible tokens
- **DAO**: Governance contracts
- **Bridge Contracts**: Contracts for cross-chain operations

#### Gas System
- **Gas Metering**: Precise resource measurement
- **Dynamic Pricing**: Dynamic gas price adjustment
- **Storage**: Efficient state management

### 7. APIs (`src/api/`)

#### REST API
- **Endpoints**: `/node`, `/chain`, `/blocks`, `/transactions`, `/validators`, `/bridges`
- **WebSocket**: Real-time notifications
- **RPC**: Ethereum JSON-RPC compatibility

#### API Configuration
```rust
pub struct ApiConfig {
    pub rpc_enabled: bool,        // Port 8545
    pub rest_enabled: bool,       // Port 8080
    pub websocket_enabled: bool,  // Port 8546
    pub graphql_enabled: bool,    // Port 8081 (disabled)
}
```

## 🔐 Cryptography

### Post-Quantum Signatures
- **Dilithium**: Quantum-resistant signature algorithm
- **Elliptic Curves**: For conventional operations

### Zero-Knowledge Proofs
- **Bulletproofs**: For efficient range proofs
- **R1CS**: For customizable arithmetic circuits
- **Pedersen Commitments**: For confidential transactions

## 📊 Economics and Tokenomics

### Emission and Halving
- **Initial Reward**: 15 ERB per block
- **Halving Interval**: 1,051,200 blocks (~4 years)
- **Fee Burning**: Transaction fees are burned

### Supply Schedule
```
Year 0-4:   15 ERB/block
Year 4-8:   7.5 ERB/block
Year 8-12:  3.75 ERB/block
Year 12+:   1.875 ERB/block
```

### Staking and Rewards
- **Variable APR**: Based on total participation
- **Validator Commission**: Configurable
- **Slashing**: Penalties for downtime/misconduct

## 🛡️ Security

### Transaction Validation
- **Signatures**: Verification of all signatures
- **Nonce**: Prevention of replay attacks
- **Balance**: Verification of sufficient funds
- **Gas Limits**: Prevention of denial of service attacks

### Secure Consensus
- **Finality**: Economic finality via PoS
- **Slashing**: Economic penalties
- **Randomness**: Unpredictable validator selection

### Privacy
- **Confidential Transactions**: Hidden values
- **Stealth Addresses**: Ephemeral addresses
- **ZK Proofs**: Proofs without revealing data

## 🌐 Supported Networks

### Mainnets
- **Bitcoin**: Integration via SPV proofs
- **Ethereum**: Light client with ZK proofs
- **Polkadot**: Parachain integration
- **Cosmos**: IBC protocol support

### Testnets
- Full support for all network testnets

## 📈 Scalability

### Optimizations
- **Merkle Trees**: Efficient inclusion verification
- **State Pruning**: Historical state cleanup
- **Parallel Processing**: Parallel transaction validation
- **Caching**: Smart frequent state caching

### Block Limits
- **Maximum Size**: 8MB per block
- **Maximum Transactions**: 10,000 per block
- **Gas Limit**: Dynamic based on demand

## 🔧 Configuration

### Network Parameters
```rust
pub struct BlockchainConfig {
    pub block_time: u64,              // 30 seconds
    pub max_block_size: usize,         // 8MB
    pub max_transactions_per_block: usize, // 10,000
    pub genesis_timestamp: u64,        // Genesis timestamp
    pub chain_id: u64,                 // 137 (Erbium mainnet)
}
```

### Development Configurations
- **Dev Mode**: Direct transaction application
- **Fast Sync**: Accelerated synchronization
- **Debug Logging**: Detailed development logs

## 🚀 Development Roadmap

### Current Phase (v0.1.0)
- ✅ Core blockchain implemented
- ✅ Basic PoS consensus
- ✅ Confidential transactions
- ✅ Cross-chain bridges
- ✅ REST/RPC/WebSocket APIs

### Next Phases
- 🔄 Performance optimization
- 🔄 Advanced smart contracts
- 🔄 Complete governance
- 🔄 Mainnet launch
- 🔄 DeFi ecosystem

## 📚 References

- [Bulletproofs Paper](https://crypto.stanford.edu/bulletproofs/)
- [Dilithium Specification](https://pq-crystals.org/dilithium/)
- [Ethereum JSON-RPC](https://eth.wiki/json-rpc/API)
- [IBC Protocol](https://ibcprotocol.org/)

---

*This documentation reflects the current state of the code in development. Features may be added or modified as the project progresses.*
