# Erbium Blockchain Core

## Overview

The **Core** module is the foundation of the Erbium blockchain, containing all essential blockchain functionality including blocks, transactions, state management, and consensus validation.

## Module Structure

```
src/core/
├── mod.rs          # Core module definitions and exports
├── block.rs        # Block structure and validation
├── chain.rs        # Blockchain data structure and management
├── dex.rs          # Decentralized Exchange implementation
├── layer2.rs       # Layer 2 protocols (state channels, sidechains)
├── mempool.rs      # Transaction memory pool
├── state.rs        # World state management
├── transaction.rs  # Transaction types and validation
├── types.rs        # Core type definitions (Address, Hash, etc.)
├── units.rs        # Token and currency units
├── vm.rs           # Virtual Machine for smart contracts
└── precompiles.rs  # Precompiled contracts
```

## Key Components

### 🧱 Block Structure

```rust
pub struct Block {
    pub header: BlockHeader,
    pub transactions: Vec<Transaction>,
    pub validator_signatures: Vec<ValidatorSignature>,
}

pub struct BlockHeader {
    pub version: u32,
    pub previous_hash: Hash,
    pub timestamp: u64,
    pub height: u64,
    pub state_root: Hash,
    pub transactions_root: Hash,
    pub validator_set_root: Hash,
    pub nonce: u64,
    pub difficulty: u64,
}
```

#### Block Validation
- **Header Validation**: Hash verification, timestamp ordering
- **Transaction Validation**: Signature verification, balance checks
- **State Transition**: Merkle tree updates, balance changes
- **Consensus Validation**: Proof-of-stake verification

### 💸 Transaction System

#### Transaction Types

1. **Standard Transfer**
```rust
pub struct Transaction {
    pub from: Address,
    pub to: Address,
    pub amount: u64,
    pub fee: u64,
    pub nonce: u64,
    pub signature: Vec<u8>,
    pub data: Vec<u8>, // For contract calls
}
```

2. **Smart Contract Calls**
```rust
pub struct ContractCall {
    pub contract_address: Address,
    pub function_signature: [u8; 4],
    pub parameters: Vec<u8>,
}
```

3. **DEX Orders**
```rust
pub enum DEXTransaction {
    PlaceLimitOrder(LimitOrder),
    PlaceMarketOrder(MarketOrder),
    CancelOrder(CancelOrder),
}
```

4. **Layer 2 Operations**
```rust
pub enum Layer2Transaction {
    OpenChannel(ChannelOpen),
    UpdateChannel(ChannelUpdate),
    CloseChannel(ChannelClose),
}
```

#### Transaction Lifecycle

1. **Creation**: Signed by sender with valid nonce and balance
2. **Validation**: Pre-validation in mempool, full validation during processing
3. **Inclusion**: Added to block during mining/validation
4. **Execution**: State changes applied, events emitted
5. **Finalization**: Block confirmation and finality

### 📊 State Management

#### World State

```rust
pub struct State {
    accounts: HashMap<Address, Account>,
    contracts: HashMap<Address, Contract>,
    storage: HashMap<Hash, Hash>, // Contract storage
    validators: BTreeSet<Address>,
}
```

#### Account Structure

```rust
pub struct Account {
    pub nonce: u64,
    pub balance: u64,
    pub code: Vec<u8>,        // Contract bytecode
    pub storage_root: Hash,   // Contract storage root
}
```

#### State Transitions

- **Balance Transfers**: Simple value transfers between accounts
- **Contract Creation**: Deploy new smart contracts
- **Contract Execution**: Run contract functions and apply state changes
- **Validator Updates**: Modify validator set for consensus

### 🔄 Consensus Integration

The core module integrates with the consensus engine:

```rust
pub trait ConsensusEngine {
    fn validate_block(&self, block: &Block) -> Result<(), ConsensusError>;
    fn create_block(&self, transactions: Vec<Transaction>) -> Result<Block, ConsensusError>;
    fn get_validator_set(&self) -> Vec<Validator>;
}
```

### 🧪 Testing

#### Unit Tests
```bash
cargo test core::
```

#### Integration Tests
```bash
cargo test --test integration core
```

#### Benchmarks
```bash
cargo bench core
```

## DEX Integration

The core module includes a complete **Decentralized Exchange**:

### Order Types

1. **Limit Orders**: Execute at specific price or better
```rust
pub struct LimitOrder {
    pub trader: Address,
    pub pair: TradingPair,
    pub side: OrderSide, // Buy/Sell
    pub amount: u64,
    pub price: u64,
}
```

2. **Market Orders**: Execute at best available price
```rust
pub struct MarketOrder {
    pub trader: Address,
    pub pair: TradingPair,
    pub side: OrderSide,
    pub amount: u64, // Amount to buy/sell
}
```

### Order Book Structure

```rust
pub struct OrderBook {
    pub bids: BTreeMap<u64, VecDeque<OrderBookEntry>>, // Price -> Orders
    pub asks: BTreeMap<u64, VecDeque<OrderBookEntry>>, // Price -> Orders
}

pub struct OrderBookEntry {
    pub order_id: Hash,
    pub amount: u64,
    pub timestamp: u64,
}
```

### Liquidity Pools (AMM)

```rust
pub struct LiquidityPool {
    pub token_a: Address,
    pub token_b: Address,
    pub reserve_a: u64,
    pub reserve_b: u64,
    pub fee: u64, // Basis points (e.g., 30 = 0.3%)
}
```

## Layer 2 Solutions

### Payment Channels

```rust
pub struct PaymentChannel {
    pub participants: [Address; 2],
    pub balances: [u64; 2],
    pub state: ChannelState,
    pub timeout_height: u64,
}
```

### Channel States

```rust
pub enum ChannelState {
    Opening,      // Waiting for funding
    Active,       // Operational
    Disputing {   // In dispute period
        dispute_height: u64,
    },
    Closed,       // Permanently closed
}
```

### Off-Chain Transactions

```rust
pub struct OffChainTransaction {
    pub channel_id: Hash,
    pub new_balances: [u64; 2],
    pub signatures: Vec<Vec<u8>>,
}
```

## Virtual Machine

The Erbium VM is **EVM-compatible** with quantum-resistant extensions:

### Opcodes
- **Standard EVM opcodes** (ADD, MUL, SSTORE, etc.)
- **Cryptographic operations** (SHA3, Dilithium signatures)
- **Privacy operations** (Zero-knowledge proofs)

### Execution Environment

```rust
pub struct ExecutionContext {
    pub address: Address,        // Contract address
    pub caller: Address,         // Caller address
    pub origin: Address,         // Transaction origin
    pub value: u64,             // ETH value sent
    pub data: Vec<u8>,          // Call data
    pub gas_limit: u64,         // Gas limit
}
```

### Contract Deployment

```rust
pub struct Contract {
    pub address: Address,
    pub bytecode: Vec<u8>,
    pub storage: ContractStorage,
}
```

## Memory Pool (Mempool)

### Transaction Pool Management

```rust
pub struct Mempool {
    config: MempoolConfig,
    transactions: HashMap<Hash, MempoolEntry>,
    by_fee: BTreeMap<(u64, Hash), Hash>, // Fee-ordered priority queue
}
```

### Pool Operations

- **Add Transaction**: Validates and queues pending transactions
- **Remove Transaction**: Removes processed/missed transactions
- **Get Highest Fee**: Returns transactions for block production
- **Cleanup Expired**: Removes stale transactions

### Fee-Based Prioritization

```rust
// Transactions ordered by fee_per_byte
fee_per_byte = transaction.fee / transaction.size_bytes
```

## Merkle Trees

### Block Merkle Root
```rust
// Calculate Merkle root for transaction inclusion proofs
let merkle_root = MerkleTree::calculate_root(
    transactions.iter().map(|tx| tx.hash()).collect()
);
```

### Storage Proofs
```rust
// Generate proof for state inclusion
let proof = MerkleTree::generate_proof(
    state_entries, target_index
);
```

## Configuration

### Core Blockchain Config

```toml
[blockchain]
chain_id = 137              # Erbium mainnet
block_time = 30             # 30 second blocks
max_block_size = 8388608    # 8MB blocks
max_transactions_per_block = 10000

[consensus]
consensus_type = "pos"      # Proof-of-stake
min_stake = 1000000         # Minimum stake (ERB)
validator_reward = 1000     # Block reward

[mempool]
max_size = 50000            # Max transactions
max_size_bytes = 104857600  # 100MB
min_fee_per_byte = 1        # Minimum fee
max_transaction_age = 86400 # 24 hours
```

## Performance Metrics

| Component | TPS | Latency | Throughput |
|-----------|-----|---------|------------|
| Core Validation | 10,000 | <1ms | 100MB/s |
| State Updates | 15,000 | <2ms | 150MB/s |
| Transaction Pool | 50,000 | <0.5ms | 50MB/s |

## Error Handling

### Core Error Types

```rust
pub enum BlockchainError {
    InvalidTransaction(String),
    InvalidBlock(String),
    StateTransition(String),
    Consensus(String),
    Network(String),
    VM(String),
}
```

### Validation Errors

- **Transaction Errors**: Invalid signatures, insufficient balance, nonce issues
- **Block Errors**: Invalid header, wrong difficulty, timestamp issues
- **State Errors**: Account not found, storage conflicts
- **VM Errors**: Stack underflow, invalid opcodes, gas exhaustion

## Testing Strategy

### Unit Tests
- **Block validation**: Header verification, difficulty adjustment
- **Transaction validation**: Signature verification, balance checks
- **State transitions**: Merkle root updates, account modifications
- **VM execution**: Contract deployment, function calls

### Integration Tests
- **Block processing**: Full block validation and state updates
- **Multi-block chains**: Fork handling, reorganization
- **Network synchronization**: Block downloading, validation

### Fuzz Testing
- **Transaction fuzzing**: Random transaction generation and validation
- **Block fuzzing**: Random block structures and validation
- **VM fuzzing**: Contract execution with random inputs

## Future Extensions

### Quantum-Resistant Features
- **Dilithium verification**: In-circuit signature validation
- **Lattice-based hashing**: Quantum-resistant hash functions

### Privacy Enhancements
- **Confidential transactions**: Amount hiding with range proofs
- **Anonymous contracts**: Privacy-preserving smart contracts

### Scalability Improvements
- **Parallel validation**: Multi-core transaction validation
- **State sharding**: Horizontal scaling of state management
- **Optimistic execution**: Parallel transaction execution
