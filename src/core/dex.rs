// src/core/dex.rs

use crate::core::{Address, Hash, State};
use crate::utils::error::{BlockchainError, Result};
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, HashMap, VecDeque};
use std::sync::Arc;
use tokio::sync::RwLock;

/// DEX Configuration
#[derive(Debug, Clone)]
pub struct DEXConfig {
    pub trading_fee: u64,         // Fee in basis points (e.g., 30 = 0.3%)
    pub min_order_size: u64,      // Minimum order size
    pub max_order_size: u64,      // Maximum order size
    pub price_precision: u8,      // Decimal places for prices
    pub amount_precision: u8,     // Decimal places for amounts
    pub order_expiry_blocks: u64, // Order expiry in blocks
}

impl Default for DEXConfig {
    fn default() -> Self {
        Self {
            trading_fee: 30,               // 0.3% fee
            min_order_size: 1000,          // Minimum 1000 units
            max_order_size: 1_000_000_000, // Maximum 1B units
            price_precision: 8,            // 8 decimal places
            amount_precision: 6,           // 6 decimal places
            order_expiry_blocks: 1000,     // ~8 hours at 30s blocks
        }
    }
}

/// Trading Pair (Token Pair)
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct TradingPair {
    pub base_token: Address,  // Token being bought/sold
    pub quote_token: Address, // Token used as reference (usually stablecoin)
    pub pair_id: Hash,
}

impl TradingPair {
    /// Create new trading pair
    pub fn new(base_token: Address, quote_token: Address) -> Self {
        let pair_id = Self::generate_pair_id(&base_token, &quote_token);
        Self {
            base_token,
            quote_token,
            pair_id,
        }
    }

    /// Generate unique pair ID
    fn generate_pair_id(base: &Address, quote: &Address) -> Hash {
        let mut data = Vec::new();
        data.extend_from_slice(&base.to_bytes().unwrap_or_default());
        data.extend_from_slice(&quote.to_bytes().unwrap_or_default());
        Hash::new(&data)
    }

    /// Get pair symbol (e.g., "ERB/USDC")
    pub fn symbol(&self) -> String {
        format!("{}/{}", self.base_token.as_str(), self.quote_token.as_str())
    }
}

/// Order Types
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum OrderType {
    Limit,  // Limit order with specific price
    Market, // Market order (fill at best available price)
}

/// Order Sides
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum OrderSide {
    Buy,  // Buy base token with quote token
    Sell, // Sell base token for quote token
}

/// Order Status
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum OrderStatus {
    Pending,   // Order placed, waiting for matching
    Partial,   // Partially filled
    Filled,    // Completely filled
    Cancelled, // Cancelled by user
    Expired,   // Expired due to time limit
}

/// Trading Order
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Order {
    pub order_id: Hash,
    pub trader: Address,
    pub pair: TradingPair,
    pub order_type: OrderType,
    pub side: OrderSide,
    pub amount: u64,           // Amount of base token
    pub price: Option<u64>,    // Price in quote token (None for market orders)
    pub filled_amount: u64,    // Amount already filled
    pub remaining_amount: u64, // Amount still to fill
    pub status: OrderStatus,
    pub created_at: u64,
    pub updated_at: u64,
    pub expiry_height: u64,
    pub fee_paid: u64,
}

impl Order {
    /// Create new limit order
    pub fn new_limit(
        trader: Address,
        pair: TradingPair,
        side: OrderSide,
        amount: u64,
        price: u64,
        current_height: u64,
        config: &DEXConfig,
    ) -> Self {
        let order_id = Self::generate_order_id(&trader, &pair, current_height);
        let expiry_height = current_height + config.order_expiry_blocks;

        Self {
            order_id,
            trader,
            pair,
            order_type: OrderType::Limit,
            side,
            amount,
            price: Some(price),
            filled_amount: 0,
            remaining_amount: amount,
            status: OrderStatus::Pending,
            created_at: current_timestamp(),
            updated_at: current_timestamp(),
            expiry_height,
            fee_paid: 0,
        }
    }

    /// Create new market order
    pub fn new_market(
        trader: Address,
        pair: TradingPair,
        side: OrderSide,
        amount: u64,
        current_height: u64,
        config: &DEXConfig,
    ) -> Self {
        let order_id = Self::generate_order_id(&trader, &pair, current_height);
        let expiry_height = current_height + config.order_expiry_blocks;

        Self {
            order_id,
            trader,
            pair,
            order_type: OrderType::Market,
            side,
            amount,
            price: None,
            filled_amount: 0,
            remaining_amount: amount,
            status: OrderStatus::Pending,
            created_at: current_timestamp(),
            updated_at: current_timestamp(),
            expiry_height,
            fee_paid: 0,
        }
    }

    /// Generate unique order ID
    fn generate_order_id(trader: &Address, pair: &TradingPair, height: u64) -> Hash {
        let mut data = Vec::new();
        data.extend_from_slice(&trader.to_bytes().unwrap_or_default());
        data.extend_from_slice(pair.pair_id.as_bytes());
        data.extend_from_slice(&height.to_be_bytes());
        data.extend_from_slice(&current_timestamp().to_be_bytes());
        Hash::new(&data)
    }

    /// Fill order with given amount
    pub fn fill(&mut self, fill_amount: u64) -> Result<()> {
        if fill_amount > self.remaining_amount {
            return Err(BlockchainError::InvalidTransaction(
                "Fill amount exceeds remaining amount".to_string(),
            ));
        }

        self.filled_amount += fill_amount;
        self.remaining_amount -= fill_amount;
        self.updated_at = current_timestamp();

        if self.remaining_amount == 0 {
            self.status = OrderStatus::Filled;
        } else {
            self.status = OrderStatus::Partial;
        }

        Ok(())
    }

    /// Cancel order
    pub fn cancel(&mut self) {
        self.status = OrderStatus::Cancelled;
        self.updated_at = current_timestamp();
    }

    /// Check if order is active
    pub fn is_active(&self) -> bool {
        matches!(self.status, OrderStatus::Pending | OrderStatus::Partial)
    }

    /// Check if order has expired
    pub fn is_expired(&self, current_height: u64) -> bool {
        current_height >= self.expiry_height
    }
}

/// Order Book Entry (for price-time priority)
#[derive(Debug, Clone)]
struct OrderBookEntry {
    order_id: Hash,
    price: u64,
    amount: u64,
    #[allow(dead_code)]
    timestamp: u64,
}

/// Order Book for a trading pair
#[derive(Debug, Clone)]
pub struct OrderBook {
    #[allow(dead_code)]
    pair: TradingPair,
    bids: BTreeMap<u64, VecDeque<OrderBookEntry>>, // Buy orders: price -> orders (sorted descending)
    asks: BTreeMap<u64, VecDeque<OrderBookEntry>>, // Sell orders: price -> orders (sorted ascending)
}

impl OrderBook {
    /// Create new order book
    pub fn new(pair: TradingPair) -> Self {
        Self {
            pair,
            bids: BTreeMap::new(),
            asks: BTreeMap::new(),
        }
    }

    /// Add order to book
    pub fn add_order(&mut self, order: &Order) {
        let entry = OrderBookEntry {
            order_id: order.order_id,
            price: order.price.unwrap_or(0), // Market orders use 0 as placeholder
            amount: order.remaining_amount,
            timestamp: order.created_at,
        };

        match (&order.side, &order.order_type) {
            (OrderSide::Buy, OrderType::Limit) => {
                self.bids
                    .entry(entry.price)
                    .or_insert(VecDeque::new())
                    .push_back(entry);
            }
            (OrderSide::Sell, OrderType::Limit) => {
                self.asks
                    .entry(entry.price)
                    .or_insert(VecDeque::new())
                    .push_back(entry);
            }
            _ => {} // Market orders handled separately
        }
    }

    /// Remove order from book
    pub fn remove_order(&mut self, order_id: &Hash, side: &OrderSide, price: Option<u64>) {
        let book = match side {
            OrderSide::Buy => &mut self.bids,
            OrderSide::Sell => &mut self.asks,
        };

        if let Some(price) = price {
            if let Some(orders) = book.get_mut(&price) {
                orders.retain(|entry| entry.order_id != *order_id);
                if orders.is_empty() {
                    book.remove(&price);
                }
            }
        }
    }

    /// Get best bid price
    pub fn best_bid(&self) -> Option<u64> {
        self.bids.keys().next_back().copied()
    }

    /// Get best ask price
    pub fn best_ask(&self) -> Option<u64> {
        self.asks.keys().next().copied()
    }

    /// Get spread
    pub fn spread(&self) -> Option<u64> {
        if let (Some(bid), Some(ask)) = (self.best_bid(), self.best_ask()) {
            Some(ask.saturating_sub(bid))
        } else {
            None
        }
    }

    /// Get market depth at price level
    pub fn depth_at_price(&self, price: u64, side: &OrderSide) -> u64 {
        match side {
            OrderSide::Buy => self
                .bids
                .get(&price)
                .map(|orders| orders.iter().map(|o| o.amount).sum())
                .unwrap_or(0),
            OrderSide::Sell => self
                .asks
                .get(&price)
                .map(|orders| orders.iter().map(|o| o.amount).sum())
                .unwrap_or(0),
        }
    }
}

/// Liquidity Pool (AMM)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LiquidityPool {
    pub pool_id: Hash,
    pub token_a: Address,
    pub token_b: Address,
    pub reserve_a: u64,
    pub reserve_b: u64,
    pub total_liquidity: u64,
    pub liquidity_providers: HashMap<Address, u64>, // Provider -> liquidity tokens
    pub fee: u64,                                   // Fee in basis points
    pub created_at: u64,
}

impl LiquidityPool {
    /// Create new liquidity pool
    pub fn new(
        token_a: Address,
        token_b: Address,
        amount_a: u64,
        amount_b: u64,
        creator: Address,
    ) -> Self {
        let pool_id = Self::generate_pool_id(&token_a, &token_b);
        let mut liquidity_providers = HashMap::new();
        liquidity_providers.insert(creator, amount_a + amount_b); // Initial liquidity

        Self {
            pool_id,
            token_a,
            token_b,
            reserve_a: amount_a,
            reserve_b: amount_b,
            total_liquidity: amount_a + amount_b,
            liquidity_providers,
            fee: 30, // 0.3% fee
            created_at: current_timestamp(),
        }
    }

    /// Generate unique pool ID
    fn generate_pool_id(token_a: &Address, token_b: &Address) -> Hash {
        let mut data = Vec::new();
        data.extend_from_slice(&token_a.to_bytes().unwrap_or_default());
        data.extend_from_slice(&token_b.to_bytes().unwrap_or_default());
        Hash::new(&data)
    }

    /// Add liquidity to pool
    pub fn add_liquidity(
        &mut self,
        provider: Address,
        amount_a: u64,
        amount_b: u64,
    ) -> Result<u64> {
        if amount_a == 0 || amount_b == 0 {
            return Err(BlockchainError::InvalidTransaction(
                "Cannot add zero liquidity".to_string(),
            ));
        }

        // Calculate liquidity tokens to mint
        let liquidity_minted = if self.total_liquidity == 0 {
            // First liquidity provision
            amount_a + amount_b
        } else {
            // Proportional to existing reserves
            let liquidity_a =
                (amount_a as u128 * self.total_liquidity as u128) / self.reserve_a as u128;
            let liquidity_b =
                (amount_b as u128 * self.total_liquidity as u128) / self.reserve_b as u128;
            liquidity_a.min(liquidity_b) as u64
        };

        self.reserve_a += amount_a;
        self.reserve_b += amount_b;
        self.total_liquidity += liquidity_minted;

        *self.liquidity_providers.entry(provider).or_insert(0) += liquidity_minted;

        Ok(liquidity_minted)
    }

    /// Remove liquidity from pool
    pub fn remove_liquidity(
        &mut self,
        provider: &Address,
        liquidity_amount: u64,
    ) -> Result<(u64, u64)> {
        let provider_liquidity = self.liquidity_providers.get(provider).copied().unwrap_or(0);

        if provider_liquidity < liquidity_amount {
            return Err(BlockchainError::InvalidTransaction(
                "Insufficient liquidity tokens".to_string(),
            ));
        }

        // Calculate token amounts to return
        let amount_a =
            (liquidity_amount as u128 * self.reserve_a as u128) / self.total_liquidity as u128;
        let amount_b =
            (liquidity_amount as u128 * self.reserve_b as u128) / self.total_liquidity as u128;

        self.reserve_a -= amount_a as u64;
        self.reserve_b -= amount_b as u64;
        self.total_liquidity -= liquidity_amount;

        *self.liquidity_providers.get_mut(provider).unwrap() -= liquidity_amount;

        Ok((amount_a as u64, amount_b as u64))
    }

    /// Calculate output amount for swap
    pub fn get_swap_amount_out(&self, amount_in: u64, token_in: &Address) -> Result<u64> {
        if token_in != &self.token_a && token_in != &self.token_b {
            return Err(BlockchainError::InvalidTransaction(
                "Token not in pool".to_string(),
            ));
        }

        let (reserve_in, reserve_out) = if token_in == &self.token_a {
            (self.reserve_a, self.reserve_b)
        } else {
            (self.reserve_b, self.reserve_a)
        };

        // Uniswap V2 formula: amount_out = (amount_in * reserve_out) / (reserve_in + amount_in)
        let amount_in_with_fee = amount_in * (10000 - self.fee as u64); // Fee subtracted
        let numerator = amount_in_with_fee * reserve_out as u64;
        let denominator = (reserve_in * 10000) + amount_in_with_fee;

        Ok(numerator / denominator)
    }

    /// Execute swap
    pub fn swap(&mut self, amount_in: u64, token_in: &Address) -> Result<u64> {
        let amount_out = self.get_swap_amount_out(amount_in, token_in)?;

        if token_in == &self.token_a {
            self.reserve_a += amount_in;
            self.reserve_b -= amount_out;
        } else {
            self.reserve_b += amount_in;
            self.reserve_a -= amount_out;
        }

        Ok(amount_out)
    }

    /// Get pool price (token_a per token_b)
    pub fn get_price(&self) -> f64 {
        if self.reserve_b == 0 {
            return 0.0;
        }
        self.reserve_a as f64 / self.reserve_b as f64
    }
}

/// DEX Manager
pub struct DEXManager {
    config: DEXConfig,
    order_books: HashMap<Hash, OrderBook>,
    orders: HashMap<Hash, Order>,
    liquidity_pools: HashMap<Hash, LiquidityPool>,
    trading_pairs: HashMap<Hash, TradingPair>,
    #[allow(dead_code)]
    state: Arc<RwLock<State>>,
}

impl DEXManager {
    /// Create new DEX manager
    pub fn new(config: DEXConfig, state: Arc<RwLock<State>>) -> Self {
        Self {
            config,
            order_books: HashMap::new(),
            orders: HashMap::new(),
            liquidity_pools: HashMap::new(),
            trading_pairs: HashMap::new(),
            state,
        }
    }

    /// Add trading pair
    pub fn add_trading_pair(&mut self, base_token: Address, quote_token: Address) -> Result<Hash> {
        let pair = TradingPair::new(base_token, quote_token);
        let pair_id = pair.pair_id;

        if self.trading_pairs.contains_key(&pair_id) {
            return Err(BlockchainError::InvalidTransaction(
                "Trading pair already exists".to_string(),
            ));
        }

        let order_book = OrderBook::new(pair.clone());
        self.order_books.insert(pair_id, order_book);
        self.trading_pairs.insert(pair_id, pair);

        log::info!("Added trading pair {}", pair_id.to_hex());

        Ok(pair_id)
    }

    /// Place limit order
    pub async fn place_limit_order(
        &mut self,
        trader: Address,
        pair_id: &Hash,
        side: OrderSide,
        amount: u64,
        price: u64,
        current_height: u64,
    ) -> Result<Hash> {
        // Validate inputs
        if amount < self.config.min_order_size || amount > self.config.max_order_size {
            return Err(BlockchainError::InvalidTransaction(format!(
                "Order amount must be between {} and {}",
                self.config.min_order_size, self.config.max_order_size
            )));
        }

        let pair = self
            .trading_pairs
            .get(pair_id)
            .ok_or_else(|| {
                BlockchainError::InvalidTransaction("Trading pair not found".to_string())
            })?
            .clone();

        // Check trader balance (TODO: implement actual balance checking)
        let _required_balance = match side {
            OrderSide::Buy => amount * price, // Quote tokens needed
            OrderSide::Sell => amount,        // Base tokens needed
        };

        // Create order
        let mut order = Order::new_limit(
            trader,
            pair,
            side,
            amount,
            price,
            current_height,
            &self.config,
        );
        let order_id = order.order_id;

        // Try to match order immediately
        let filled_amount = self.match_order(&mut order).await?;

        if filled_amount > 0 {
            // Partial fill - order already updated by match_order
            self.orders.insert(order_id, order.clone());

            // Add remaining to order book if not fully filled
            if order.is_active() {
                if let Some(order_book) = self.order_books.get_mut(pair_id) {
                    order_book.add_order(&order);
                }
            }
        } else {
            // No immediate match - add to order book
            self.orders.insert(order_id, order.clone());
            if let Some(order_book) = self.order_books.get_mut(pair_id) {
                order_book.add_order(&order);
            }
        }

        log::info!(
            "Placed limit order {} for {} amount at price {}",
            order_id.to_hex(),
            amount,
            price
        );

        Ok(order_id)
    }

    /// Place market order
    pub async fn place_market_order(
        &mut self,
        trader: Address,
        pair_id: &Hash,
        side: OrderSide,
        amount: u64,
        current_height: u64,
    ) -> Result<Hash> {
        let pair = self
            .trading_pairs
            .get(pair_id)
            .ok_or_else(|| {
                BlockchainError::InvalidTransaction("Trading pair not found".to_string())
            })?
            .clone();

        // Create market order
        let mut order = Order::new_market(trader, pair, side, amount, current_height, &self.config);
        let order_id = order.order_id;

        // Match against order book
        let _filled_amount = self.match_order(&mut order).await?;

        // Market orders are either filled immediately or cancelled
        if order.filled_amount == 0 {
            order.status = OrderStatus::Cancelled;
        }

        self.orders.insert(order_id, order);

        log::info!(
            "Placed market order {} for {} amount",
            order_id.to_hex(),
            amount
        );

        Ok(order_id)
    }

    /// Cancel order
    pub async fn cancel_order(&mut self, order_id: &Hash, trader: &Address) -> Result<()> {
        let order = self
            .orders
            .get_mut(order_id)
            .ok_or_else(|| BlockchainError::InvalidTransaction("Order not found".to_string()))?;

        // Verify ownership
        if &order.trader != trader {
            return Err(BlockchainError::InvalidTransaction(
                "Not order owner".to_string(),
            ));
        }

        // Cancel order
        order.cancel();

        // Remove from order book
        if let Some(order_book) = self.order_books.get_mut(&order.pair.pair_id) {
            order_book.remove_order(order_id, &order.side, order.price);
        }

        log::info!("Cancelled order {}", order_id.to_hex());

        Ok(())
    }

    /// Match order against order book
    async fn match_order(&mut self, order: &mut Order) -> Result<u64> {
        let mut total_filled = 0;
        let order_book = match self.order_books.get_mut(&order.pair.pair_id) {
            Some(book) => book,
            None => return Ok(0),
        };

        let (opposite_book, is_buy_order) = match order.side {
            OrderSide::Buy => (&mut order_book.asks, true),
            OrderSide::Sell => (&mut order_book.bids, false),
        };

        // For market orders, match at best available price
        // For limit orders, match at limit price or better
        let target_prices: Vec<u64> = if let Some(limit_price) = order.price {
            if is_buy_order {
                // Buy order: match at ask prices <= limit_price
                opposite_book
                    .range(..=limit_price)
                    .rev()
                    .map(|(p, _)| *p)
                    .collect()
            } else {
                // Sell order: match at bid prices >= limit_price
                opposite_book
                    .range(limit_price..)
                    .map(|(p, _)| *p)
                    .collect()
            }
        } else {
            // Market order: match at best available prices
            if is_buy_order {
                opposite_book.keys().copied().collect()
            } else {
                opposite_book.keys().rev().copied().collect()
            }
        };

        for price in target_prices {
            if let Some(orders_at_price) = opposite_book.get_mut(&price) {
                while let Some(opposite_order_entry) = orders_at_price.front() {
                    if let Some(opposite_order) =
                        self.orders.get_mut(&opposite_order_entry.order_id)
                    {
                        let match_amount =
                            order.remaining_amount.min(opposite_order.remaining_amount);

                        if match_amount > 0 {
                            // Fill both orders
                            order.fill(match_amount)?;
                            opposite_order.fill(match_amount)?;
                            total_filled += match_amount;

                            // Remove filled order from book
                            if !opposite_order.is_active() {
                                orders_at_price.pop_front();
                            }

                            // Break if current order is filled
                            if !order.is_active() {
                                break;
                            }
                        } else {
                            break;
                        }
                    } else {
                        // Order not found, remove from book
                        orders_at_price.pop_front();
                    }
                }

                // Remove empty price level
                if orders_at_price.is_empty() {
                    opposite_book.remove(&price);
                }

                if !order.is_active() {
                    break;
                }
            }
        }

        Ok(total_filled)
    }

    /// Create liquidity pool
    pub async fn create_liquidity_pool(
        &mut self,
        token_a: Address,
        token_b: Address,
        amount_a: u64,
        amount_b: u64,
        creator: Address,
    ) -> Result<Hash> {
        if amount_a == 0 || amount_b == 0 {
            return Err(BlockchainError::InvalidTransaction(
                "Cannot create pool with zero liquidity".to_string(),
            ));
        }

        let pool = LiquidityPool::new(token_a, token_b, amount_a, amount_b, creator);
        let pool_id = pool.pool_id;

        if self.liquidity_pools.contains_key(&pool_id) {
            return Err(BlockchainError::InvalidTransaction(
                "Pool already exists".to_string(),
            ));
        }

        self.liquidity_pools.insert(pool_id, pool);

        log::info!("Created liquidity pool {}", pool_id.to_hex());

        Ok(pool_id)
    }

    /// Add liquidity to pool
    pub async fn add_liquidity(
        &mut self,
        pool_id: &Hash,
        provider: Address,
        amount_a: u64,
        amount_b: u64,
    ) -> Result<u64> {
        let pool = self
            .liquidity_pools
            .get_mut(pool_id)
            .ok_or_else(|| BlockchainError::InvalidTransaction("Pool not found".to_string()))?;

        let liquidity_minted = pool.add_liquidity(provider, amount_a, amount_b)?;

        log::info!(
            "Added {} liquidity to pool {}",
            liquidity_minted,
            pool_id.to_hex()
        );

        Ok(liquidity_minted)
    }

    /// Remove liquidity from pool
    pub async fn remove_liquidity(
        &mut self,
        pool_id: &Hash,
        provider: &Address,
        liquidity_amount: u64,
    ) -> Result<(u64, u64)> {
        let pool = self
            .liquidity_pools
            .get_mut(pool_id)
            .ok_or_else(|| BlockchainError::InvalidTransaction("Pool not found".to_string()))?;

        let (amount_a, amount_b) = pool.remove_liquidity(provider, liquidity_amount)?;

        log::info!(
            "Removed {} liquidity from pool {}: {} token_a, {} token_b",
            liquidity_amount,
            pool_id.to_hex(),
            amount_a,
            amount_b
        );

        Ok((amount_a, amount_b))
    }

    /// Swap tokens via pool
    pub async fn swap_via_pool(
        &mut self,
        pool_id: &Hash,
        amount_in: u64,
        token_in: Address,
    ) -> Result<u64> {
        let pool = self
            .liquidity_pools
            .get_mut(pool_id)
            .ok_or_else(|| BlockchainError::InvalidTransaction("Pool not found".to_string()))?;

        let amount_out = pool.swap(amount_in, &token_in)?;

        log::info!(
            "Swapped {} {} for {} {} in pool {}",
            amount_in,
            token_in.as_str(),
            amount_out,
            if token_in == pool.token_a {
                pool.token_b.as_str()
            } else {
                pool.token_a.as_str()
            },
            pool_id.to_hex()
        );

        Ok(amount_out)
    }

    /// Get order book for pair
    pub fn get_order_book(&self, pair_id: &Hash) -> Option<&OrderBook> {
        self.order_books.get(pair_id)
    }

    /// Get order details
    pub fn get_order(&self, order_id: &Hash) -> Option<&Order> {
        self.orders.get(order_id)
    }

    /// Get liquidity pool
    pub fn get_liquidity_pool(&self, pool_id: &Hash) -> Option<&LiquidityPool> {
        self.liquidity_pools.get(pool_id)
    }

    /// Get trading pair
    pub fn get_trading_pair(&self, pair_id: &Hash) -> Option<&TradingPair> {
        self.trading_pairs.get(pair_id)
    }

    /// Get DEX statistics
    pub fn get_stats(&self) -> DEXStats {
        DEXStats {
            total_trading_pairs: self.trading_pairs.len(),
            total_orders: self.orders.len(),
            active_orders: self.orders.values().filter(|o| o.is_active()).count(),
            total_liquidity_pools: self.liquidity_pools.len(),
            total_liquidity_locked: self
                .liquidity_pools
                .values()
                .map(|p| p.reserve_a + p.reserve_b)
                .sum(),
        }
    }

    /// Process expired orders
    pub async fn process_expired_orders(&mut self, current_height: u64) -> Result<Vec<Hash>> {
        let mut expired_orders = Vec::new();

        for (order_id, order) in &mut self.orders {
            if order.is_active() && order.is_expired(current_height) {
                order.status = OrderStatus::Expired;
                expired_orders.push(*order_id);

                // Remove from order book
                if let Some(order_book) = self.order_books.get_mut(&order.pair.pair_id) {
                    order_book.remove_order(order_id, &order.side, order.price);
                }
            }
        }

        if !expired_orders.is_empty() {
            log::info!("Expired {} orders", expired_orders.len());
        }

        Ok(expired_orders)
    }
}

/// DEX Statistics
#[derive(Debug, Clone)]
pub struct DEXStats {
    pub total_trading_pairs: usize,
    pub total_orders: usize,
    pub active_orders: usize,
    pub total_liquidity_pools: usize,
    pub total_liquidity_locked: u64,
}

fn current_timestamp() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_secs()
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::Arc;
    use tokio::sync::RwLock;

    #[tokio::test]
    async fn test_trading_pair_creation() {
        let state = Arc::new(RwLock::new(State::new()));
        let config = DEXConfig::default();
        let mut dex = DEXManager::new(config, state);

        let base_token =
            Address::new_unchecked("0x0000000000000000000000000000000000000001".to_string());
        let quote_token =
            Address::new_unchecked("0x0000000000000000000000000000000000000002".to_string());

        let pair_id = dex
            .add_trading_pair(base_token.clone(), quote_token.clone())
            .unwrap();

        let pair = dex.get_trading_pair(&pair_id).unwrap();
        assert_eq!(pair.base_token, base_token);
        assert_eq!(pair.quote_token, quote_token);
    }

    #[tokio::test]
    async fn test_limit_order_placement() {
        let state = Arc::new(RwLock::new(State::new()));
        let config = DEXConfig::default();
        let mut dex = DEXManager::new(config, state);

        let base_token =
            Address::new_unchecked("0x0000000000000000000000000000000000000001".to_string());
        let quote_token =
            Address::new_unchecked("0x0000000000000000000000000000000000000002".to_string());
        let trader =
            Address::new_unchecked("0x0000000000000000000000000000000000000003".to_string());

        let pair_id = dex.add_trading_pair(base_token, quote_token).unwrap();

        let order_id = dex
            .place_limit_order(trader, &pair_id, OrderSide::Buy, 10000, 100, 1000)
            .await
            .unwrap();

        let order = dex.get_order(&order_id).unwrap();
        assert_eq!(order.amount, 10000);
        assert_eq!(order.price, Some(100));
        assert_eq!(order.side, OrderSide::Buy);
    }

    #[tokio::test]
    async fn test_liquidity_pool_creation() {
        let state = Arc::new(RwLock::new(State::new()));
        let config = DEXConfig::default();
        let mut dex = DEXManager::new(config, state);

        let token_a =
            Address::new_unchecked("0x0000000000000000000000000000000000000001".to_string());
        let token_b =
            Address::new_unchecked("0x0000000000000000000000000000000000000002".to_string());
        let creator =
            Address::new_unchecked("0x0000000000000000000000000000000000000003".to_string());

        let pool_id = dex
            .create_liquidity_pool(token_a, token_b, 100000, 100000, creator)
            .await
            .unwrap();

        let pool = dex.get_liquidity_pool(&pool_id).unwrap();
        assert_eq!(pool.reserve_a, 100000);
        assert_eq!(pool.reserve_b, 100000);
        assert_eq!(pool.total_liquidity, 200000);
    }

    #[tokio::test]
    async fn test_liquidity_pool_swap() {
        let state = Arc::new(RwLock::new(State::new()));
        let config = DEXConfig::default();
        let mut dex = DEXManager::new(config, state);

        let token_a =
            Address::new_unchecked("0x0000000000000000000000000000000000000001".to_string());
        let token_b =
            Address::new_unchecked("0x0000000000000000000000000000000000000002".to_string());
        let creator =
            Address::new_unchecked("0x0000000000000000000000000000000000000003".to_string());

        let pool_id = dex
            .create_liquidity_pool(token_a.clone(), token_b, 100000, 100000, creator)
            .await
            .unwrap();

        // Test swap calculation
        let pool = dex.get_liquidity_pool(&pool_id).unwrap();
        let amount_out = pool.get_swap_amount_out(1000, &token_a).unwrap();
        assert!(amount_out > 0 && amount_out < 1000); // Should get less due to fee

        // Execute swap
        let amount_out_actual = dex.swap_via_pool(&pool_id, 1000, token_a).await.unwrap();
        assert_eq!(amount_out, amount_out_actual);
    }

    #[tokio::test]
    async fn test_dex_stats() {
        let state = Arc::new(RwLock::new(State::new()));
        let config = DEXConfig::default();
        let dex = DEXManager::new(config, state);

        let stats = dex.get_stats();
        assert_eq!(stats.total_trading_pairs, 0);
        assert_eq!(stats.total_orders, 0);
        assert_eq!(stats.total_liquidity_locked, 0);
    }
}
