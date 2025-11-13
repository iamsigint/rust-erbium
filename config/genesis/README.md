# Erbium Genesis Allocations

This document explains how to configure initial token allocations in the Erbium blockchain genesis block.

## IMPORTANT - SECURITY

**NEVER generate wallets directly in blockchain code!** This compromises system integrity.

### Recommended Secure Process:

1. **Generate wallets manually** using external post-quantum tools
2. **Store private keys** in secure cold storage
3. **Backup seed phrases** on paper
4. **Configure addresses** in the `allocations.toml` file
5. **Test** genesis in development environment
6. **Validate** genesis block before launch

## Allocations Configuration

### File: `allocations.toml`

```toml
[genesis]
timestamp = 1704067200  # 2024-01-01 00:00:00 UTC
initial_validators = []

initial_balances = [
  # Personal Reserve (50M ERB - 5% of supply)
  { address = "0xYOUR_ADDRESS_HERE", amount = "5000000000000000" },

  # Development Fund (20M ERB - 2% of supply)
  { address = "0xDEV_FUND_ADDRESS", amount = "2000000000000000" },

  # Ecosystem Fund (20M ERB - 2% of supply)
  { address = "0xECO_FUND_ADDRESS", amount = "2000000000000000" },

  # Treasury (10M ERB - 1% of supply)
  { address = "0xTREASURY_ADDRESS", amount = "1000000000000000" }
]
```

### ERB Units System

The Erbium blockchain uses a hierarchical unit system similar to Bitcoin (satoshi) and Ethereum (wei), with **8 decimal places** for high precision.

#### Available Units

| Unit         | Symbol | Value in ERB    | Conversion Factor | Equivalent |
|--------------|--------|-----------------|-------------------|------------|
| **Ion**      | **ion**| 0.00000001 ERB  | 10^-8            | Minimum unit |
| MicroERB     | μERB   | 0.000001 ERB    | 10^-6            | 100 ions |
| MilliERB     | mERB   | 0.001 ERB       | 10^-3            | Thousandth |
| CentiERB     | cERB   | 0.01 ERB        | 10^-2            | Hundredth |
| DeciERB     | dERB   | 0.1 ERB         | 10^-1            | Tenth |
| **ERB**      | **ERB**| 1.0 ERB         | 10^0             | **Main unit** |
| KiloERB      | kERB   | 1,000 ERB       | 10^3             | Thousand ERB |
| MegaERB      | M-ERB  | 1,000,000 ERB   | 10^6             | Million ERB |

#### Practical Examples

```rust
use erbium_blockchain::core::units::{ErbAmount, Unit};

// Create values
let amount1 = ErbAmount::from_erb(1.5);        // 1.5 ERB
let amount2 = ErbAmount::from_ions(150);       // 150 ions = 0.0000015 ERB

// Automatic formatting
assert_eq!(amount1.format_auto(), "1.50000000 ERB");
assert_eq!(amount2.format_auto(), "1.500000 μERB");

// Specific conversions
assert_eq!(amount1.format(Unit::Ion), "150000000 ion");
assert_eq!(amount1.format(Unit::ERB), "1.50000000 ERB");
```

#### Allocation Values

- **1 ERB** = 100,000,000 ions
- **1M ERB** = 100,000,000,000,000 ions
- **Total Pre-allocated**: 100M ERB = 10,000,000,000,000,000 ions

## Testing Allocations

Run the test to validate configurations:

```bash
cargo run --bin test_genesis
```

### What the test checks:

- File exists
- TOML format is valid
- Addresses are valid (42 characters, 0x... format)
- Values are valid numbers
- Genesis system loads correctly
- Genesis transactions are created
- Balances are applied correctly
- Warnings for placeholder addresses

## Production Configuration

### 1. Generate Secure Wallets

Use external tools to generate post-quantum wallets:

```bash
# Example with external tool (not included in this project)
pq-wallet-generator --algorithm=dilithium --format=ethereum
```

### 2. Configure Addresses

Edit `config/genesis/allocations.toml`:

```toml
initial_balances = [
  { address = "0x1234567890abcdef...", amount = "5000000000000000" },
  { address = "0xfedcba0987654321...", amount = "2000000000000000" },
  # ... other addresses
]
```

### 3. Secure Backup

- **Seed phrases**: Print on paper, store in safe
- **Private keys**: Never on connected devices
- **Addresses**: Document for reference

### 4. Test in Safe Environment

```bash
# Test in devnet
cargo run --bin test_genesis

# Check logs to confirm everything is correct
```

### 5. Validate Before Launch

- All addresses are valid
- All values are correct
- Total pre-allocated matches plan
- No private keys in repository
- Secure backups made

## Allocation Strategy

Following Satoshi Nakamoto model:

- **10% pre-allocated** for founders/developers/ecosystem
- **90% issued organically** through consensus
- **Fair distribution** among different stakeholders
- **Full transparency** in genesis allocations

### Breakdown:

| Recipient | Percentage | Value (ERB) | Purpose |
|-----------|------------|-------------|---------|
| Personal Reserve | 5% | 50M | Main Founder/Developer |
| Development Fund | 2% | 20M | Ongoing development funding |
| Ecosystem Fund | 2% | 20M | Ecosystem growth and adoption |
| Treasury | 1% | 10M | Strategic network reserve |
| **Total Pre-allocated** | **10%** | **100M** | |
| Block Emission | 90% | 900M | Consensus rewards |

## Security Features

### What was implemented:

- **Genesis Transactions**: Can only come from address `0x0`
- **Special Validation**: Zero fees allowed only for genesis
- **Balance Verification**: Skips checks for genesis transactions
- **Supply Control**: Increases total supply for genesis allocations
- **Signatures**: Skips signature verification for genesis

### What was removed for security:

- Automatic wallet generation in code
- Private key storage in repository
- Seed phrases in development files

## Next Steps

1. **Generate secure wallets** externally
2. **Configure addresses** in `allocations.toml`
3. **Test thoroughly** in development environment
4. **Make secure backup** of all credentials
5. **Audit** genesis block before launch
6. **Launch mainnet** with confidence

---

**Remember**: Genesis allocation security is critical for the entire blockchain integrity. Never compromise this process!
