# Troubleshooting Guide

## Compilation Errors

### Error: `cargo check` fails with compilation errors

**Symptoms:**
```
error[E0425]: cannot find value `filled_amount` in this scope
```

**Solution:**
1. Check if all variables are being used correctly
2. Make sure underscore prefix (`_`) is not being used incorrectly
3. Add `#[allow(dead_code)]` for fields not yet used

### Error: Missing dependencies

**Symptoms:**
```
error: linking with `link.exe` failed: exit code: 1120
```

**Windows:**
```powershell
# Install required dependencies
powershell -ExecutionPolicy Bypass -File scripts/install_dependencies.ps1

# Verify OpenSSL is installed
openssl version
```

**Linux:**
```bash
sudo apt update
sudo apt install build-essential libssl-dev pkg-config
```

## Runtime Errors

### Error: Node cannot synchronize

**Symptoms:**
- Logs show network connection errors
- Node cannot find peers

**Solution:**
```bash
# Check network configuration
cargo run --bin erbium-node -- --config config/testnet.toml --bootstrap-node

# Check open ports
netstat -tulpn | grep 30303
```

### Error: Transactions failing

**Symptoms:**
```
error: Insufficient balance
```

**Solution:**
```rust
// Check account balance
let balance = state.get_balance(&sender_address)?;
ensure!(balance >= transaction.amount + transaction.fee, "Insufficient balance");
```

## Performance Issues

### Low TPS

**Possible causes:**
- Insufficient memory
- Overloaded CPU
- Slow network

**Optimization:**
```toml
# config/mainnet.toml
[performance]
max_concurrent_transactions = 1000
db_cache_size = "2GB"
network_buffer_size = 67108864  # 64MB
```

### High CPU usage

**Analysis:**
```bash
# Monitor processes
top -p $(pidof erbium-node)

# Check memory usage
vmstat 1
```

## Network Issues

### Rejected connections

**Symptoms:**
- Failure to connect with peers
- Network timeouts

**Solution:**
```rust
// Check connection timeout
transport_config.connection_timeout = Duration::from_secs(30);
transport_config.heartbeat_interval = Duration::from_secs(30);
```

### High latency

**Diagnosis:**
```bash
# Test network latency
ping us-central.pool.erbium.io

# Check node connectivity
curl http://localhost:8080/status
```

## Security Issues

### Invalid signatures

**Symptoms:**
```
error: Invalid signature
```

**Verification:**
```rust
// Verify public key
let pk = PublicKey::from_bytes(&pk_bytes)?;
let sig = Signature::from_bytes(&sig_bytes)?;
pk.verify(message, &sig)?;
```

### Denial of service attacks

**Protection:**
1. Connection limits per IP
2. Transaction rate limiting
3. Spam filters
4. Circuit breakers

```rust
pub struct DoSProtection {
    pub max_connections_per_ip: usize,
    pub transaction_rate_limit: usize,
    pub request_size_limit: usize,
}
```

## DEX Issues

### Orders not being filled

**Possible causes:**
- Insufficient liquidity in order book
- Price outside of spread
- Order expired

**Verification:**
```rust
// Check order status
let order = dex.get_order(&order_id)?;
match order.status {
    OrderStatus::Expired => println!("Order expired"),
    OrderStatus::Filled => println!("Order filled"),
    _ => println!("Order active"),
}
```

## Layer 2 Issues

### Channels not opening

**Verification:**
```rust
// Check balances
let balance_a = state.get_balance(&participant_a)?;
let balance_b = state.get_balance(&participant_b)?;

// Minimum capacity
const MIN_CHANNEL_CAPACITY: u64 = 1000; // 1000 ERB
```

### Disputes failing

**Solution:**
```rust
// Minimum challenge period
const MIN_CHALLENGE_PERIOD: u64 = 100; // blocks

// Verify fraud proof
fraud_proof.validate(&channel_state)?;
```

## VM Issues

### Contracts reverting

**Debugging:**
```rust
// Check sufficient gas
ensure!(context.gas_limit >= MIN_GAS, "Insufficient gas");

// Verify valid data
ensure!(!data.is_empty(), "Empty call data");
```

### Infinite loops

**Protection:**
```rust
pub struct ExecutionLimits {
    pub max_steps: usize,
    pub max_memory: usize,
    pub timeout: Duration,
}

// Infinite loop check
fn check_execution_limits(&self, context: &ExecutionContext) -> Result<()> {
    ensure!(self.steps < self.max_steps, "Execution timeout");
    ensure!(self.memory.len() <= self.max_memory, "Out of memory");
    Ok(())
}
```

## Storage Issues

### Database corruption

**Recovery:**
```bash
# Database backup
cp -r data/ data.backup/

# Integrity verification
parity-db --db data/ --verify

# Rebuild indexes
parity-db --db data/ --rebuild-indexes
```

### Insufficient disk space

**Monitoring:**
```bash
# Check disk usage
df -h

# Clean old data (periodic)
find data/ -name "*.log" -mtime +30 -delete
```

## Environment Configuration

### Windows Issues

**Compilation error:**
```powershell
# Install Visual Studio Build Tools
# https://visualstudio.microsoft.com/visual-cpp-build-tools/

# Configure vcvars
call "C:\Program Files (x86)\Microsoft Visual Studio\2019\BuildTools\VC\Auxiliary\Build\x64\vcvars64.bat"
```

### Linux Issues

**Permissions:**
```bash
# Fix data permissions
sudo chown -R $USER:$USER data/
sudo chmod -R 755 data/
```

**Missing dependencies:**
```bash
# Ubuntu/Debian
sudo apt install build-essential libssl-dev llvm-dev clang

# Fedora/CentOS
sudo dnf install make gcc openssl-devel llvm clang
```

## Logging and Debug

### Log Levels

```rust
use log::{info, warn, error, debug, trace};

// Configuration
env_logger::Builder::new()
    .filter_level(log::LevelFilter::Debug)
    .init();
```

### Debug transactions

```rust
// Detailed transaction logs
log::debug!("Processing transaction: {:?}", tx.hash());
log::debug!("Sender balance: {}", sender_balance);
log::debug!("Transaction fee: {}", tx.fee);
log::debug!("Gas used: {}", gas_used);
```

## Updates and Migration

### Breaking Changes

**v0.1.0 → v0.2.0:**
1. `state` field moved to DEX manager
2. OrderSide structure changed
3. Hash::random() method removed

**Migration:**
```bash
# Backup configuration
cp config/ config.backup/

# Update binaries
cargo build --release

# Migrate data if needed
./migrate_data.sh
```

## DIY Support

### Useful Resources

- **GitHub Issues:** [Report bugs](https://github.com/iamsigint/rust-erbium/issues)
- **Documentation:** [docs.erbium.io](https://docs.erbium.io)
- **Discord:** [discord.gg/erbium](https://discord.gg/erbium)

### Debug Tools

```bash
# Log analyzer
cargo run --bin log_analyzer -- logs/

# Performance monitor
cargo run --bin performance_monitor

# Network utility
cargo run --bin network_debugger
```

## Problem Prevention

### Regular Maintenance

```bash
# Weekly cleanup
0 2 * * 1 /path/to/cleanup.sh

# Daily backup
0 1 * * * /path/to/backup.sh

# Continuous monitoring
*/5 * * * * /path/to/health_check.sh
```

### Monitoring

| Metric | Threshold | Action |
|--------|-----------|--------|
| CPU Usage | >80% | Scale up |
| Memory Usage | >90% | Restart |
| Disk Space | <10% free | Cleanup |
| Error Rate | >1% | Alert |

## Emergency Contact

**Critical Issues:**
- Email: security@erbium.io
- Response: < 1 hour

**General Support:**
- GitHub Discussions
- Discord #support channel

---

*This documentation is kept up to date. Last update: 2025-11-14*
