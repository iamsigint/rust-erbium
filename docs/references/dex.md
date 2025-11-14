# Decentralized Exchange (DEX)

## Overview

Erbium's **Decentralized Exchange** provides a complete on-chain trading platform with order books, automated market makers (AMMs), and cross-chain liquidity. Built directly into the core blockchain for maximum security and efficiency.

## Architecture

### DEX Manager

```rust
pub struct DEXManager {
    config: DEXConfig,
    order_books: HashMap<Hash, OrderBook>,      // Trading pair -> Order book
    orders: HashMap<Hash, Order>,               // Order ID -> Order
    liquidity_pools: HashMap<Hash, LiquidityPool>, // Pool ID -> AMM Pool
    trading_pairs: HashMap<Hash, TradingPair>,     // Pair ID -> Pair info
}
```

### Trading Pairs

```rust
pub struct TradingPair {
    pub base_token: Address,    // Token being traded
    pub quote_token: Address,   // Reference token (usually stablecoin)
    pub pair_id: Hash,         // Unique pair identifier
}
```

## Order Management

### Order Types

#### 1. Limit Orders

Execute at a specific price or better:

```rust
pub struct Order {
    pub order_id: Hash,
    pub trader: Address,
    pub pair: TradingPair,
    pub order_type: OrderType,    // Limit
    pub side: OrderSide,          // Buy/Sell
    pub amount: u64,             // Base token amount
    pub price: Option<u64>,       // Limit price (quote tokens)
    pub status: OrderStatus,
}
```

#### 2. Market Orders

Execute immediately at the best available price:

```rust
pub struct MarketOrder {
    pub trader: Address,
    pub pair: TradingPair,
    pub side: OrderSide,         // Buy/Sell
    pub amount: u64,             // Amount to trade
}
```

### Order Lifecycle

1. **Placement**: Order submitted with signature and fee
2. **Validation**: Balance and signature verification
3. **Matching**: Immediate fill attempt against order book
4. **Partial Fill**: Remaining amount added to order book (if limit order)
5. **Cancellation**: Trader or expiry-based removal
6. **Settlement**: Executed trades update balances

### Order Book Structure

```rust
pub struct OrderBook {
    pub pair: TradingPair,
    pub bids: BTreeMap<u64, VecDeque<OrderBookEntry>>, // Buy orders: price descending
    pub asks: BTreeMap<u64, VecDeque<OrderBookEntry>>, // Sell orders: price ascending
}

pub struct OrderBookEntry {
    pub order_id: Hash,
    pub amount: u64,     // Remaining amount
    pub timestamp: u64,  // For time priority
}
```

### Order Matching Algorithm

#### Price-Time Priority

Orders are matched using **price-time priority**:
- **Best Price**: Highest bid/lowest ask first
- **Time Priority**: Earlier orders at same price first

#### Cross Validation

```rust
// Price crossing condition for matching
if bid_price >= ask_price {
    // Orders cross - execute trade
    let trade_price = ask_price;  // Price priority: use ask price
    let trade_amount = min(bid_amount, ask_amount);
    execute_trade(trade_price, trade_amount);
}
```

## Automated Market Makers (AMMs)

### Liquidity Pool Structure

```rust
pub struct LiquidityPool {
    pub pool_id: Hash,
    pub token_a: Address,
    pub token_b: Address,
    pub reserve_a: u64,      // Token A reserve
    pub reserve_b: u64,      // Token B reserve
    pub total_liquidity: u64, // Total liquidity tokens minted
    pub fee: u64,           // Fee in basis points (e.g., 30 = 0.3%)
}
```

### AMM Formula

Erbium uses Uniswap V2-style AMM with constant product market maker:

```
(x + dx) * (y + dy) = x * y

Where:
- x, y: Current reserves
- dx, dy: Token amounts exchanged
```

#### Swap Calculation

```rust
pub fn get_swap_amount_out(
    &self,
    amount_in: u64,
    token_in: &Address
) -> Result<u64> {
    let (reserve_in, reserve_out) = if token_in == &self.token_a {
        (self.reserve_a, self.reserve_b)
    } else {
        (self.reserve_b, self.reserve_a)
    };

    // Apply fee (subtract from input amount)
    let amount_in_with_fee = amount_in * (10000 - self.fee);

    // Calculate output amount
    let numerator = amount_in_with_fee * reserve_out;
    let denominator = (reserve_in * 10000) + amount_in_with_fee;

    Ok(numerator / denominator)
}
```

### Liquidity Provision

#### Adding Liquidity

```rust
pub fn add_liquidity(
    &mut self,
    provider: Address,
    amount_a: u64,
    amount_b: u64,
) -> Result<u64> {
    // Calculate liquidity tokens to mint
    let liquidity_minted = if self.total_liquidity == 0 {
        // First liquidity provision
        (amount_a as u128 * amount_b as u128).sqrt() as u64
    } else {
        // Proportional to existing reserves
        min(
            (amount_a as u128 * self.total_liquidity as u128) / self.reserve_a as u128,
            (amount_b as u128 * self.total_liquidity as u128) / self.reserve_b as u128,
        ) as u64
    };

    // Update reserves and provider balance
    self.reserve_a += amount_a;
    self.reserve_b += amount_b;
    self.total_liquidity += liquidity_minted;

    Ok(liquidity_minted)
}
```

#### Removing Liquidity

```rust
pub fn remove_liquidity(
    &mut self,
    provider: &Address,
    liquidity_amount: u64,
) -> Result<(u64, u64)> {
    // Calculate token amounts to return
    let amount_a = (liquidity_amount as u128 * self.reserve_a as u128) / self.total_liquidity as u128;
    let amount_b = (liquidity_amount as u128 * self.reserve_b as u128) / self.total_liquidity as u128;

    // Update reserves
    self.reserve_a -= amount_a as u64;
    self.reserve_b -= amount_b as u64;
    self.total_liquidity -= liquidity_amount;

    Ok((amount_a as u64, amount_b as u64))
}
```

## Transaction Integration

### DEX Transactions

```rust
pub enum DEXTransaction {
    // Order Management
    PlaceLimitOrder(PlaceLimitOrder),
    PlaceMarketOrder(PlaceMarketOrder),
    CancelOrder(CancelOrder),

    // Liquidity Management
    AddLiquidity(AddLiquidity),
    RemoveLiquidity(RemoveLiquidity),

    // Cross-Pool Swaps
    SwapViaPool(SwapTransaction),
}
```

### Fee Structure

#### Trading Fees

- **Order Book**: 0.1% maker/taker fees
- **AMM Pools**: 0.3% swap fees
- **Cross-chain**: Additional bridge fees

#### Fee Distribution

- **60%** to liquidity providers
- **20%** to protocol treasury
- **20%** to stakers/validators

## Cross-Chain Integration

### Bridge Integration

```rust
pub struct CrossChainSwap {
    pub source_chain: ChainId,
    pub target_chain: ChainId,
    pub token_in: Address,
    pub token_out: Address,
    pub amount_in: u64,
    pub min_amount_out: u64,
    pub deadline: u64,
}
```

### Bridge Validators

- **Decentralized Validators**: Multi-sig validation across chains
- **Merkle Proofs**: Cryptographic proof of cross-chain events
- **Time Locks**: Security timeouts for disputed transfers

## Order Types & Strategies

### Advanced Order Types

#### Iceberg Orders
- Large orders broken into smaller visible portions
- Hide full order size to avoid price impact

#### TWAP Orders
- Time-weighted average price execution
- Reduces slippage for large orders

#### Bracket Orders
- Conditional orders with stop-loss/take-profit
- Automated risk management

### Market Making Strategies

#### Pure AMM Pools
```rust
// Single pool with mathematical formula
Pool: ERB/USDC
Formula: x * y = k
Price: dy/dx = y/x
```

#### Hybrid Order Book + AMM
```rust
// Order book for institutional trades
// AMM for instant liquidity
struct HybridDEX {
    order_book: OrderBook,
    amm_pools: Vec<LiquidityPool>,
    arbitrage_engine: ArbitrageEngine,
}
```

## Position Management

### Trader Positions

```rust
pub struct TraderPosition {
    pub trader: Address,
    pub pair: TradingPair,
    pub long_amount: u64,     // Long position size
    pub short_amount: u64,    // Short position size
    pub margin_used: u64,     // Margin utilized
    pub pnl: i64,            // Unrealized P&L
}
```

### Liquidation Engine

```rust
pub struct LiquidationEngine {
    pub positions: HashMap<Address, Vec<TraderPosition>>,
    pub liquidation_price_threshold: f64,
    pub forced_liquidation_penalty: u64,
}
```

## Analytics & Monitoring

### DEX Statistics

```rust
pub struct DEXStats {
    pub total_trading_pairs: usize,
    pub total_orders: usize,
    pub active_orders: usize,
    pub total_liquidity_locked: u64,
    pub daily_volume: u64,
    pub tvl: u64,  // Total Value Locked
}
```

### Order Book Analytics

- **Spread Analysis**: Bid-ask spread monitoring
- **Depth Charts**: Market depth visualization
- **Volume Profiles**: Price level volume analysis
- **Order Flow**: Market/microstructure analysis

### Performance Metrics

| Metric | Target | Description |
|--------|--------|-------------|
| **Order Matching Latency** | <1ms | From order placement to fill |
| **Slippage** | <0.1% | Price impact for market orders |
| **Uptime** | 99.99% | DEX availability |
| **Fill Rate** | >99% | Percentage of orders filled |

## Risk Management

### Circuit Breakers

- **Price Limits**: Maximum price deviations per block
- **Volume Limits**: Maximum trading volume per time window
- **Liquidity Thresholds**: Minimum liquidity requirements

### Self-Regulation Mechanisms

- **Automatic Fee Adjustment**: Fee reduction during high volatility
- **Liquidity Incentives**: Automatic rewards for providing liquidity
- **Arbitrage Bounds**: Price convergence mechanisms

## Protocol Economics

### Fee Distribution Model

```rust
pub struct FeeDistribution {
    pub liquidity_providers: u64,    // 60%
    pub protocol_treasury: u64,      // 20%
    pub stakers_validators: u64,     // 15%
    pub insurance_fund: u64,         // 5%
}
```

### Economic Incentives

#### Liquidity Mining
- **Rewards**: ERB token rewards for liquidity provision
- **Boosted Returns**: Higher yields during low liquidity periods
- **Time-Locked Staking**: Commitment-based rewards

#### Trading Incentives
- **Fee Discounts**: Reduced fees for high-volume traders
- **VIP Tiers**: Tiered benefits based on trading volume
- **Referral Program**: Rewards for bringing new users

## Cross-Chain DeFi

### Bridge-Enabled Trading

```rust
pub struct CrossChainTrade {
    pub source_chain: ChainId,
    pub source_token: Address,
    pub target_chain: ChainId,
    pub target_token: Address,
    pub amount: u64,
    pub expected_output: u64,
    pub slippage_tolerance: f64,
}
```

### Wrapped Asset Standards

- **ERC-20 Wrappers**: Ethereum-compatible wrapped assets
- **Native Bridges**: Direct cross-chain asset transfers
- **Liquidity Pools**: Cross-chain liquidity provision

### Bridge Security

#### Multi-Signature Validation
```rust
pub struct BridgeValidator {
    pub validators: Vec<Address>,
    pub threshold: usize,
    pub bridge_contract: Address,
}
```

#### Challenge Periods
- **Dispute Window**: Time for transaction challenges
- **Validator Challenges**: Fraud proof mechanisms
- **Emergency Pauses**: Safety circuit breakers

## Simulation & Backtesting

### Historical Replay

```rust
pub struct OrderBookReplay {
    pub historical_orders: Vec<Order>,
    pub price_feed: PriceFeed,
    pub replay_speed: f64,
}
```

### Strategy Testing

```rust
pub trait TradingStrategy {
    fn on_price_update(&mut self, price: PriceUpdate) -> Vec<Order>;
    fn on_order_fill(&mut self, fill: OrderFill);
    fn on_market_data(&mut self, data: MarketData);
}
```

## Security Considerations

### Smart Contract Security

#### Access Control
```rust
#[access_control(owner_only)]
fn emergency_pause(&mut self) {
    // Emergency pause functionality
    self.paused = true;
}
```

#### Reentrancy Protection
```rust
#[non_reentrant]
fn swap(&mut self, amount_in: u64) -> u64 {
    // Protected swap operation
    let amount_out = self.calculate_output(amount_in);
    self.update_reserves(amount_in, amount_out);
    amount_out
}
```

### Economic Attacks

- **Sandwich Attacks**: Frontrunning protection via private mempool
- **Price Manipulation**: Circuit breakers and rate limits
- **Flash Loan Attacks**: Time-locked operations where needed

## Testing & Validation

### Unit Tests

```bash
# Test order matching
cargo test dex::test_order_matching

# Test AMM calculations
cargo test dex::test_amm_calculations

# Test cross-chain swaps
cargo test dex::test_cross_chain_swaps
```

### Integration Tests

```bash
# Full DEX workflow test
cargo test --test integration dex_workflow

# Cross-chain integration test
cargo test --test integration cross_chain_dex
```

### Load Testing

```bash
# Simulate 1000 TPS
cargo run --bin load_tester -- --scenario dex_high_load

# Stress test AMM pools
cargo run --bin stress_test -- --target amm_pools
```

## Future Enhancements

### Layer 2 DEX Solutions

- **Rollup-based DEX**: Off-chain order matching with on-chain settlement
- **Optimistic DEX**: Fraud-proof order execution
- **ZK-rollup DEX**: Privacy-preserving trading with validity proofs

### Advanced Features

#### Perpetual Markets
```rust
pub struct PerpetualContract {
    pub underlying: Address,
    pub leverage: u64,
    pub funding_rate: i64,
    pub liquidation_price: u64,
}
```

#### Derivatives
- **Options**: Call/put options on trading pairs
- **Futures**: Futures contracts with leverage
- **Synthetic Assets**: Price feeds for synthetic tokens

#### AI-Powered Features
- **Market Prediction**: AI-driven price predictions
- **Arbitrage Detection**: Automated cross-exchange arbitrage
- **Risk Management**: Machine learning-based risk assessment

### Interoperability Expansion

- **Multi-Chain DEX**: Unified liquidity across multiple blockchains
- **Cross-Chain Margin Trading**: Leveraged trading using cross-chain collateral
- **DeFi Protocol Integration**: Connection to lending protocols, yield farming platforms

### Privacy Enhancements

- **Private Orders**: Zero-knowledge order placement
- **Confidential Trading**: Amount-hiding order book
- **Dark Pool Trading**: Private order matching for institutional traders

## Conclusion

Erbium's DEX provides a comprehensive platform that combines the best of traditional order book trading with modern AMM liquidity. The integration with quantum-resistant cryptography and cross-chain capabilities positions it as a leader in next-generation DeFi infrastructure.
