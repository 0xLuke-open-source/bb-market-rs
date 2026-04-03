use std::cmp::Reverse;
use std::collections::{BTreeMap, HashMap, VecDeque};

use anyhow::{anyhow, ensure, Result};
use rust_decimal::Decimal;
use rust_decimal_macros::dec;

pub type AccountId = u64;
pub type OrderId = u64;
pub type TradeId = u64;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Side {
    Buy,
    Sell,
}

impl Side {
    fn opposite(self) -> Self {
        match self {
            Self::Buy => Self::Sell,
            Self::Sell => Self::Buy,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum OrderType {
    Limit,
    Market,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TimeInForce {
    Gtc,
    Ioc,
    Fok,
    PostOnly,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SelfTradePrevention {
    CancelNewest,
    CancelOldest,
    DecrementAndCancel,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum OrderStatus {
    New,
    PartiallyFilled,
    Filled,
    Cancelled,
    Rejected,
}

#[derive(Debug, Clone)]
pub struct SpotMarketConfig {
    pub symbol: String,
    pub base_asset: String,
    pub quote_asset: String,
    pub tick_size: Decimal,
    pub lot_size: Decimal,
    pub min_qty: Decimal,
    pub min_notional: Decimal,
    pub maker_fee_rate: Decimal,
    pub taker_fee_rate: Decimal,
}

impl SpotMarketConfig {
    pub fn validate(&self) -> Result<()> {
        ensure!(self.tick_size > Decimal::ZERO, "tick_size must be > 0");
        ensure!(self.lot_size > Decimal::ZERO, "lot_size must be > 0");
        ensure!(self.min_qty >= Decimal::ZERO, "min_qty must be >= 0");
        ensure!(
            self.min_notional >= Decimal::ZERO,
            "min_notional must be >= 0"
        );
        ensure!(
            self.maker_fee_rate >= Decimal::ZERO,
            "maker_fee_rate must be >= 0"
        );
        ensure!(
            self.taker_fee_rate >= Decimal::ZERO,
            "taker_fee_rate must be >= 0"
        );
        Ok(())
    }
}

#[derive(Debug, Clone)]
pub struct Balance {
    pub available: Decimal,
    pub locked: Decimal,
}

impl Default for Balance {
    fn default() -> Self {
        Self {
            available: Decimal::ZERO,
            locked: Decimal::ZERO,
        }
    }
}

#[derive(Debug, Clone, Default)]
pub struct Account {
    pub balances: HashMap<String, Balance>,
}

#[derive(Debug, Clone)]
pub struct NewOrderRequest {
    pub account_id: AccountId,
    pub symbol: String,
    pub side: Side,
    pub order_type: OrderType,
    pub time_in_force: TimeInForce,
    pub price: Option<Decimal>,
    pub quantity: Option<Decimal>,
    pub quote_quantity: Option<Decimal>,
    pub client_order_id: Option<String>,
    pub self_trade_prevention: SelfTradePrevention,
}

#[derive(Debug, Clone)]
pub struct ReplaceOrderRequest {
    pub order_id: OrderId,
    pub new_price: Option<Decimal>,
    pub new_quantity: Decimal,
    pub new_time_in_force: Option<TimeInForce>,
}

#[derive(Debug, Clone)]
pub struct SubmitOrderResult {
    pub order_id: OrderId,
    pub status: OrderStatus,
    pub filled_qty: Decimal,
    pub filled_quote_qty: Decimal,
    pub remaining_qty: Decimal,
    pub trades: Vec<Trade>,
}

#[derive(Debug, Clone)]
pub struct CancelResult {
    pub order_id: OrderId,
    pub cancelled_qty: Decimal,
    pub released_funds: Decimal,
    pub status: OrderStatus,
}

#[derive(Debug, Clone)]
pub struct Trade {
    pub trade_id: TradeId,
    pub symbol: String,
    pub price: Decimal,
    pub quantity: Decimal,
    pub quote_quantity: Decimal,
    pub taker_order_id: OrderId,
    pub maker_order_id: OrderId,
    pub taker_account_id: AccountId,
    pub maker_account_id: AccountId,
    pub taker_side: Side,
    pub maker_fee: Decimal,
    pub taker_fee: Decimal,
    pub maker_fee_asset: String,
    pub taker_fee_asset: String,
    pub sequence: u64,
}

#[derive(Debug, Clone)]
pub struct LevelSnapshot {
    pub price: Decimal,
    pub total_qty: Decimal,
    pub order_count: usize,
}

#[derive(Debug, Clone)]
pub struct OrderBookSnapshot {
    pub symbol: String,
    pub sequence: u64,
    pub bids: Vec<LevelSnapshot>,
    pub asks: Vec<LevelSnapshot>,
}

#[derive(Debug, Clone)]
pub struct OrderView {
    pub order_id: OrderId,
    pub client_order_id: Option<String>,
    pub account_id: AccountId,
    pub symbol: String,
    pub side: Side,
    pub order_type: OrderType,
    pub time_in_force: TimeInForce,
    pub price: Option<Decimal>,
    pub original_qty: Decimal,
    pub remaining_qty: Decimal,
    pub filled_qty: Decimal,
    pub filled_quote_qty: Decimal,
    pub locked_funds: Decimal,
    pub status: OrderStatus,
    pub sequence: u64,
}

#[derive(Debug, Clone)]
struct RestingOrder {
    order_id: OrderId,
    client_order_id: Option<String>,
    account_id: AccountId,
    symbol: String,
    side: Side,
    order_type: OrderType,
    time_in_force: TimeInForce,
    price: Option<Decimal>,
    original_qty: Decimal,
    remaining_qty: Decimal,
    filled_qty: Decimal,
    filled_quote_qty: Decimal,
    locked_funds: Decimal,
    status: OrderStatus,
    sequence: u64,
}

impl RestingOrder {
    fn view(&self) -> OrderView {
        OrderView {
            order_id: self.order_id,
            client_order_id: self.client_order_id.clone(),
            account_id: self.account_id,
            symbol: self.symbol.clone(),
            side: self.side,
            order_type: self.order_type,
            time_in_force: self.time_in_force,
            price: self.price,
            original_qty: self.original_qty,
            remaining_qty: self.remaining_qty,
            filled_qty: self.filled_qty,
            filled_quote_qty: self.filled_quote_qty,
            locked_funds: self.locked_funds,
            status: self.status,
            sequence: self.sequence,
        }
    }
}

#[derive(Debug, Default, Clone)]
struct PriceLevel {
    order_ids: VecDeque<OrderId>,
    total_qty: Decimal,
}

#[derive(Debug)]
struct SpotMarket {
    config: SpotMarketConfig,
    bids: BTreeMap<Reverse<Decimal>, PriceLevel>,
    asks: BTreeMap<Decimal, PriceLevel>,
    orders: HashMap<OrderId, RestingOrder>,
    sequence: u64,
    last_trade_id: TradeId,
}

impl SpotMarket {
    fn new(config: SpotMarketConfig) -> Self {
        Self {
            config,
            bids: BTreeMap::new(),
            asks: BTreeMap::new(),
            orders: HashMap::new(),
            sequence: 0,
            last_trade_id: 0,
        }
    }

    fn next_sequence(&mut self) -> u64 {
        self.sequence += 1;
        self.sequence
    }

    fn next_trade_id(&mut self) -> TradeId {
        self.last_trade_id += 1;
        self.last_trade_id
    }

    fn best_bid(&self) -> Option<Decimal> {
        self.bids.keys().next().map(|price| price.0)
    }

    fn best_ask(&self) -> Option<Decimal> {
        self.asks.keys().next().copied()
    }

    fn would_cross(&self, side: Side, price: Decimal) -> bool {
        match side {
            Side::Buy => self.best_ask().map(|ask| price >= ask).unwrap_or(false),
            Side::Sell => self.best_bid().map(|bid| price <= bid).unwrap_or(false),
        }
    }

    fn add_to_book(&mut self, order: RestingOrder) -> Result<()> {
        let price = order
            .price
            .ok_or_else(|| anyhow!("resting order missing price"))?;
        let order_id = order.order_id;
        let remaining = order.remaining_qty;
        match order.side {
            Side::Buy => {
                let level = self.bids.entry(Reverse(price)).or_default();
                level.order_ids.push_back(order_id);
                level.total_qty += remaining;
            }
            Side::Sell => {
                let level = self.asks.entry(price).or_default();
                level.order_ids.push_back(order_id);
                level.total_qty += remaining;
            }
        }
        self.orders.insert(order_id, order);
        Ok(())
    }

    fn remove_order_id(&mut self, side: Side, price: Decimal, order_id: OrderId) {
        match side {
            Side::Buy => {
                let key = Reverse(price);
                let should_remove = if let Some(level) = self.bids.get_mut(&key) {
                    level.order_ids.retain(|id| *id != order_id);
                    level.order_ids.is_empty()
                } else {
                    false
                };
                if should_remove {
                    self.bids.remove(&key);
                }
            }
            Side::Sell => {
                let should_remove = if let Some(level) = self.asks.get_mut(&price) {
                    level.order_ids.retain(|id| *id != order_id);
                    level.order_ids.is_empty()
                } else {
                    false
                };
                if should_remove {
                    self.asks.remove(&price);
                }
            }
        }
    }

    fn reduce_level_qty(
        &mut self,
        side: Side,
        price: Decimal,
        filled_qty: Decimal,
        order_id: OrderId,
    ) {
        match side {
            Side::Buy => {
                let key = Reverse(price);
                let should_remove = if let Some(level) = self.bids.get_mut(&key) {
                    level.total_qty -= filled_qty;
                    if level.total_qty < Decimal::ZERO {
                        level.total_qty = Decimal::ZERO;
                    }
                    level.order_ids.front().copied() == Some(order_id)
                        && level.total_qty == Decimal::ZERO
                } else {
                    false
                };
                if should_remove {
                    self.bids.remove(&key);
                }
            }
            Side::Sell => {
                let should_remove = if let Some(level) = self.asks.get_mut(&price) {
                    level.total_qty -= filled_qty;
                    if level.total_qty < Decimal::ZERO {
                        level.total_qty = Decimal::ZERO;
                    }
                    level.order_ids.front().copied() == Some(order_id)
                        && level.total_qty == Decimal::ZERO
                } else {
                    false
                };
                if should_remove {
                    self.asks.remove(&price);
                }
            }
        }
    }

    fn best_resting_order(&self, taker_side: Side) -> Option<OrderId> {
        match taker_side {
            Side::Buy => self
                .asks
                .iter()
                .find_map(|(_, level)| level.order_ids.front().copied()),
            Side::Sell => self
                .bids
                .iter()
                .find_map(|(_, level)| level.order_ids.front().copied()),
        }
    }
}

#[derive(Debug, Default)]
pub struct SpotMatchingEngine {
    accounts: HashMap<AccountId, Account>,
    markets: HashMap<String, SpotMarket>,
    next_order_id: OrderId,
}

impl SpotMatchingEngine {
    pub fn new() -> Self {
        Self {
            accounts: HashMap::new(),
            markets: HashMap::new(),
            next_order_id: 0,
        }
    }

    pub fn create_market(&mut self, config: SpotMarketConfig) -> Result<()> {
        config.validate()?;
        let symbol = config.symbol.clone();
        ensure!(
            !self.markets.contains_key(&symbol),
            "market already exists: {}",
            symbol
        );
        self.markets.insert(symbol, SpotMarket::new(config));
        Ok(())
    }

    pub fn deposit(&mut self, account_id: AccountId, asset: &str, amount: Decimal) -> Result<()> {
        ensure!(amount > Decimal::ZERO, "deposit amount must be > 0");
        let balance = self
            .accounts
            .entry(account_id)
            .or_default()
            .balances
            .entry(asset.to_string())
            .or_default();
        balance.available += amount;
        Ok(())
    }

    pub fn balance_of(&self, account_id: AccountId, asset: &str) -> Balance {
        self.accounts
            .get(&account_id)
            .and_then(|account| account.balances.get(asset))
            .cloned()
            .unwrap_or_default()
    }

    pub fn orderbook_snapshot(&self, symbol: &str, depth: usize) -> Result<OrderBookSnapshot> {
        let market = self.market(symbol)?;
        let bids = market
            .bids
            .iter()
            .take(depth)
            .map(|(price, level)| LevelSnapshot {
                price: price.0,
                total_qty: level.total_qty,
                order_count: level.order_ids.len(),
            })
            .collect();
        let asks = market
            .asks
            .iter()
            .take(depth)
            .map(|(price, level)| LevelSnapshot {
                price: *price,
                total_qty: level.total_qty,
                order_count: level.order_ids.len(),
            })
            .collect();
        Ok(OrderBookSnapshot {
            symbol: symbol.to_string(),
            sequence: market.sequence,
            bids,
            asks,
        })
    }

    pub fn get_order(&self, symbol: &str, order_id: OrderId) -> Result<OrderView> {
        let market = self.market(symbol)?;
        market
            .orders
            .get(&order_id)
            .map(RestingOrder::view)
            .ok_or_else(|| anyhow!("order not found: {}", order_id))
    }

    pub fn cancel_order(&mut self, symbol: &str, order_id: OrderId) -> Result<CancelResult> {
        let mut market = self
            .markets
            .remove(symbol)
            .ok_or_else(|| anyhow!("unknown market: {}", symbol))?;

        let mut order = market
            .orders
            .remove(&order_id)
            .ok_or_else(|| anyhow!("order not found: {}", order_id))?;

        ensure!(
            matches!(
                order.status,
                OrderStatus::New | OrderStatus::PartiallyFilled
            ),
            "order is not cancellable"
        );

        let released = order.locked_funds;
        self.release_locked_for_order(&market.config, &order, released)?;
        order.locked_funds = Decimal::ZERO;
        order.status = OrderStatus::Cancelled;
        order.sequence = market.next_sequence();

        if let Some(price) = order.price {
            market.remove_order_id(order.side, price, order.order_id);
        }

        let result = CancelResult {
            order_id,
            cancelled_qty: order.remaining_qty,
            released_funds: released,
            status: order.status,
        };

        self.markets.insert(symbol.to_string(), market);
        Ok(result)
    }

    pub fn replace_order(
        &mut self,
        symbol: &str,
        req: ReplaceOrderRequest,
    ) -> Result<SubmitOrderResult> {
        let old = self.get_order(symbol, req.order_id)?;
        let _ = self.cancel_order(symbol, req.order_id)?;
        self.submit_order(NewOrderRequest {
            account_id: old.account_id,
            symbol: symbol.to_string(),
            side: old.side,
            order_type: old.order_type,
            time_in_force: req.new_time_in_force.unwrap_or(old.time_in_force),
            price: req.new_price.or(old.price),
            quantity: Some(req.new_quantity),
            quote_quantity: None,
            client_order_id: old.client_order_id,
            self_trade_prevention: SelfTradePrevention::CancelNewest,
        })
    }

    pub fn submit_order(&mut self, req: NewOrderRequest) -> Result<SubmitOrderResult> {
        self.validate_request(&req)?;

        let symbol = req.symbol.clone();
        let mut market = self
            .markets
            .remove(&symbol)
            .ok_or_else(|| anyhow!("unknown market: {}", symbol))?;

        let order_id = self.next_order_id();
        let (original_qty, locked_funds) = self.prepare_funds(&market.config, &req)?;
        self.lock_order_funds(&market.config, &req, locked_funds)?;

        if req.time_in_force == TimeInForce::PostOnly {
            let price = req
                .price
                .ok_or_else(|| anyhow!("post only order requires price"))?;
            if market.would_cross(req.side, price) {
                self.unlock_order_funds(&market.config, &req, locked_funds)?;
                self.markets.insert(symbol, market);
                return Ok(SubmitOrderResult {
                    order_id,
                    status: OrderStatus::Rejected,
                    filled_qty: Decimal::ZERO,
                    filled_quote_qty: Decimal::ZERO,
                    remaining_qty: original_qty,
                    trades: Vec::new(),
                });
            }
        }

        if req.time_in_force == TimeInForce::Fok
            && !self.can_fully_fill(&market, &req, original_qty)?
        {
            self.unlock_order_funds(&market.config, &req, locked_funds)?;
            self.markets.insert(symbol, market);
            return Ok(SubmitOrderResult {
                order_id,
                status: OrderStatus::Cancelled,
                filled_qty: Decimal::ZERO,
                filled_quote_qty: Decimal::ZERO,
                remaining_qty: original_qty,
                trades: Vec::new(),
            });
        }

        let mut order = RestingOrder {
            order_id,
            client_order_id: req.client_order_id.clone(),
            account_id: req.account_id,
            symbol: req.symbol.clone(),
            side: req.side,
            order_type: req.order_type,
            time_in_force: req.time_in_force,
            price: req.price,
            original_qty,
            remaining_qty: original_qty,
            filled_qty: Decimal::ZERO,
            filled_quote_qty: Decimal::ZERO,
            locked_funds,
            status: OrderStatus::New,
            sequence: market.next_sequence(),
        };

        let trades = self.match_order(&mut market, &mut order, req.self_trade_prevention)?;

        if order.filled_qty > Decimal::ZERO {
            order.status = if order.remaining_qty > Decimal::ZERO {
                OrderStatus::PartiallyFilled
            } else {
                OrderStatus::Filled
            };
        }

        let restable = order.remaining_qty > Decimal::ZERO
            && order.order_type == OrderType::Limit
            && order.time_in_force == TimeInForce::Gtc;

        if restable {
            order.sequence = market.next_sequence();
            market.add_to_book(order.clone())?;
        } else if order.remaining_qty > Decimal::ZERO {
            let release_amount = order.locked_funds;
            self.release_locked_for_order(&market.config, &order, release_amount)?;
            order.locked_funds = Decimal::ZERO;
            if order.filled_qty > Decimal::ZERO {
                order.status = OrderStatus::Cancelled;
            } else if order.time_in_force == TimeInForce::Ioc
                || order.time_in_force == TimeInForce::Fok
            {
                order.status = OrderStatus::Cancelled;
            }
        }

        let result = SubmitOrderResult {
            order_id: order.order_id,
            status: order.status,
            filled_qty: order.filled_qty,
            filled_quote_qty: order.filled_quote_qty,
            remaining_qty: order.remaining_qty,
            trades,
        };

        self.markets.insert(symbol, market);
        Ok(result)
    }

    fn match_order(
        &mut self,
        market: &mut SpotMarket,
        taker: &mut RestingOrder,
        stp: SelfTradePrevention,
    ) -> Result<Vec<Trade>> {
        let mut trades = Vec::new();

        loop {
            if taker.remaining_qty <= Decimal::ZERO {
                break;
            }

            let maker_id = match market.best_resting_order(taker.side) {
                Some(id) => id,
                None => break,
            };

            let maker_preview = market
                .orders
                .get(&maker_id)
                .cloned()
                .ok_or_else(|| anyhow!("maker order disappeared: {}", maker_id))?;

            let maker_price = maker_preview
                .price
                .ok_or_else(|| anyhow!("maker missing price"))?;

            if !self.can_cross_order(taker, maker_price)? {
                break;
            }

            if maker_preview.account_id == taker.account_id {
                let stop = self.apply_self_trade_prevention(market, taker, maker_preview, stp)?;
                if stop {
                    break;
                }
                continue;
            }

            let mut fill_qty = taker.remaining_qty.min(maker_preview.remaining_qty);
            if taker.side == Side::Buy && taker.order_type == OrderType::Market {
                fill_qty = fill_qty.min(taker.locked_funds / maker_price);
            }
            if fill_qty <= Decimal::ZERO {
                break;
            }
            let quote_qty = fill_qty * maker_price;

            let maker_fee = quote_qty * market.config.maker_fee_rate;
            let taker_fee = match taker.side {
                Side::Buy => fill_qty * market.config.taker_fee_rate,
                Side::Sell => quote_qty * market.config.taker_fee_rate,
            };
            let (maker_fee_asset, taker_fee_asset) = match taker.side {
                Side::Buy => (
                    market.config.quote_asset.clone(),
                    market.config.base_asset.clone(),
                ),
                Side::Sell => (
                    market.config.base_asset.clone(),
                    market.config.quote_asset.clone(),
                ),
            };

            self.settle_trade(
                &market.config,
                taker,
                &maker_preview,
                fill_qty,
                quote_qty,
                maker_price,
                maker_fee,
                taker_fee,
            )?;

            taker.remaining_qty -= fill_qty;
            taker.filled_qty += fill_qty;
            taker.filled_quote_qty += quote_qty;

            if taker.side == Side::Buy && taker.order_type == OrderType::Limit {
                let reserved = taker.price.unwrap_or(maker_price) * fill_qty;
                self.debit_locked(taker.account_id, &market.config.quote_asset, reserved)?;
                taker.locked_funds -= reserved;
                let actual_cost = quote_qty;
                let refund = reserved - actual_cost;
                if refund > Decimal::ZERO {
                    self.credit_available(taker.account_id, &market.config.quote_asset, refund)?;
                }
            } else if taker.side == Side::Buy && taker.order_type == OrderType::Market {
                self.debit_locked(taker.account_id, &market.config.quote_asset, quote_qty)?;
                taker.locked_funds -= quote_qty;
            } else if taker.side == Side::Sell {
                taker.locked_funds -= fill_qty;
            }

            let maker_filled;
            {
                let next_seq = market.next_sequence();
                let maker = market
                    .orders
                    .get_mut(&maker_id)
                    .ok_or_else(|| anyhow!("maker order disappeared during fill"))?;
                maker.remaining_qty -= fill_qty;
                maker.filled_qty += fill_qty;
                maker.filled_quote_qty += quote_qty;
                maker.status = if maker.remaining_qty > Decimal::ZERO {
                    OrderStatus::PartiallyFilled
                } else {
                    OrderStatus::Filled
                };
                if maker.side == Side::Buy {
                    let reserved = maker.price.unwrap_or(maker_price) * fill_qty;
                    maker.locked_funds -= reserved;
                    let refund = reserved - quote_qty;
                    if refund > Decimal::ZERO {
                        self.credit_available(
                            maker.account_id,
                            &market.config.quote_asset,
                            refund,
                        )?;
                    }
                } else {
                    maker.locked_funds -= fill_qty;
                }
                maker.sequence = next_seq;
                maker_filled = maker.remaining_qty <= Decimal::ZERO;
            }

            market.reduce_level_qty(maker_preview.side, maker_price, fill_qty, maker_id);
            if maker_filled {
                market.remove_order_id(maker_preview.side, maker_price, maker_id);
                market.orders.remove(&maker_id);
            }

            taker.sequence = market.next_sequence();
            let trade = Trade {
                trade_id: market.next_trade_id(),
                symbol: market.config.symbol.clone(),
                price: maker_price,
                quantity: fill_qty,
                quote_quantity: quote_qty,
                taker_order_id: taker.order_id,
                maker_order_id: maker_id,
                taker_account_id: taker.account_id,
                maker_account_id: maker_preview.account_id,
                taker_side: taker.side,
                maker_fee,
                taker_fee,
                maker_fee_asset,
                taker_fee_asset,
                sequence: market.sequence,
            };
            trades.push(trade);
        }

        Ok(trades)
    }

    fn apply_self_trade_prevention(
        &mut self,
        market: &mut SpotMarket,
        taker: &mut RestingOrder,
        maker: RestingOrder,
        stp: SelfTradePrevention,
    ) -> Result<bool> {
        match stp {
            SelfTradePrevention::CancelNewest => {
                let release = taker.locked_funds;
                self.release_locked_for_order(&market.config, taker, release)?;
                taker.locked_funds = Decimal::ZERO;
                taker.status = OrderStatus::Cancelled;
                Ok(true)
            }
            SelfTradePrevention::CancelOldest => {
                self.cancel_resting_maker(market, &maker)?;
                Ok(false)
            }
            SelfTradePrevention::DecrementAndCancel => {
                let qty = taker.remaining_qty.min(maker.remaining_qty);
                if qty <= Decimal::ZERO {
                    return Ok(true);
                }
                taker.remaining_qty -= qty;
                if taker.side == Side::Buy {
                    let release = match taker.order_type {
                        OrderType::Limit => taker.price.unwrap_or_default() * qty,
                        OrderType::Market => Decimal::ZERO,
                    };
                    if release > Decimal::ZERO {
                        taker.locked_funds -= release;
                        self.credit_available(
                            taker.account_id,
                            &market.config.quote_asset,
                            release,
                        )?;
                    }
                } else {
                    taker.locked_funds -= qty;
                    self.credit_available(taker.account_id, &market.config.base_asset, qty)?;
                }
                self.cancel_resting_maker(market, &maker)?;
                Ok(taker.remaining_qty <= Decimal::ZERO)
            }
        }
    }

    fn cancel_resting_maker(
        &mut self,
        market: &mut SpotMarket,
        maker: &RestingOrder,
    ) -> Result<()> {
        let mut order = market
            .orders
            .remove(&maker.order_id)
            .ok_or_else(|| anyhow!("maker not found for stp cancel"))?;
        let release = order.locked_funds;
        self.release_locked_for_order(&market.config, &order, release)?;
        order.locked_funds = Decimal::ZERO;
        order.status = OrderStatus::Cancelled;
        if let Some(price) = order.price {
            market.remove_order_id(order.side, price, order.order_id);
        }
        Ok(())
    }

    fn settle_trade(
        &mut self,
        config: &SpotMarketConfig,
        taker: &RestingOrder,
        maker: &RestingOrder,
        qty: Decimal,
        quote_qty: Decimal,
        _price: Decimal,
        maker_fee: Decimal,
        taker_fee: Decimal,
    ) -> Result<()> {
        match taker.side {
            Side::Buy => {
                self.credit_available(taker.account_id, &config.base_asset, qty - taker_fee)?;
                self.debit_locked(maker.account_id, &config.base_asset, qty)?;
                self.credit_available(
                    maker.account_id,
                    &config.quote_asset,
                    quote_qty - maker_fee,
                )?;
            }
            Side::Sell => {
                self.credit_available(
                    taker.account_id,
                    &config.quote_asset,
                    quote_qty - taker_fee,
                )?;
                self.debit_locked(
                    maker.account_id,
                    &config.quote_asset,
                    maker.price.unwrap_or_default() * qty,
                )?;
                let maker_release = maker.price.unwrap_or_default() * qty - quote_qty;
                if maker_release > Decimal::ZERO {
                    self.credit_available(maker.account_id, &config.quote_asset, maker_release)?;
                }
                self.credit_available(maker.account_id, &config.base_asset, qty - maker_fee)?;
                self.debit_locked(taker.account_id, &config.base_asset, qty)?;
            }
        }
        Ok(())
    }

    fn can_cross_order(&self, taker: &RestingOrder, maker_price: Decimal) -> Result<bool> {
        Ok(match taker.order_type {
            OrderType::Market => match taker.side {
                Side::Buy => taker.locked_funds >= maker_price * dec!(0.00000001),
                Side::Sell => taker.remaining_qty > Decimal::ZERO,
            },
            OrderType::Limit => {
                let price = taker
                    .price
                    .ok_or_else(|| anyhow!("limit order missing price"))?;
                match taker.side {
                    Side::Buy => price >= maker_price,
                    Side::Sell => price <= maker_price,
                }
            }
        })
    }

    fn can_fully_fill(
        &self,
        market: &SpotMarket,
        req: &NewOrderRequest,
        qty: Decimal,
    ) -> Result<bool> {
        let mut remaining = qty;
        match req.side {
            Side::Buy => {
                if req.order_type == OrderType::Market {
                    let mut quote_budget = req.quote_quantity.unwrap_or_default();
                    for (price, level) in &market.asks {
                        if quote_budget <= Decimal::ZERO {
                            break;
                        }
                        let spendable_qty = (quote_budget / *price).min(level.total_qty);
                        remaining -= spendable_qty;
                        quote_budget -= spendable_qty * *price;
                        if remaining <= Decimal::ZERO {
                            return Ok(true);
                        }
                    }
                    Ok(false)
                } else {
                    let max_price = req.price.unwrap_or_default();
                    for (price, level) in &market.asks {
                        if *price > max_price {
                            break;
                        }
                        remaining -= level.total_qty;
                        if remaining <= Decimal::ZERO {
                            return Ok(true);
                        }
                    }
                    Ok(false)
                }
            }
            Side::Sell => {
                let limit = req.price.unwrap_or(Decimal::MAX);
                for (price, level) in &market.bids {
                    if req.order_type == OrderType::Limit && price.0 < limit {
                        break;
                    }
                    remaining -= level.total_qty;
                    if remaining <= Decimal::ZERO {
                        return Ok(true);
                    }
                }
                Ok(false)
            }
        }
    }

    fn validate_request(&self, req: &NewOrderRequest) -> Result<()> {
        let market = self.market(&req.symbol)?;
        match req.order_type {
            OrderType::Limit => {
                let price = req
                    .price
                    .ok_or_else(|| anyhow!("limit order requires price"))?;
                ensure!(price > Decimal::ZERO, "price must be > 0");
                ensure!(
                    (price / market.config.tick_size).fract().is_zero(),
                    "price does not align with tick size"
                );
                let qty = req
                    .quantity
                    .ok_or_else(|| anyhow!("limit order requires quantity"))?;
                self.validate_quantity(&market.config, qty, Some(price))?;
            }
            OrderType::Market => match req.side {
                Side::Buy => {
                    let quote_qty = req
                        .quote_quantity
                        .ok_or_else(|| anyhow!("market buy requires quote_quantity"))?;
                    ensure!(quote_qty > Decimal::ZERO, "quote_quantity must be > 0");
                }
                Side::Sell => {
                    let qty = req
                        .quantity
                        .ok_or_else(|| anyhow!("market sell requires quantity"))?;
                    self.validate_quantity(&market.config, qty, None)?;
                }
            },
        }
        Ok(())
    }

    fn validate_quantity(
        &self,
        config: &SpotMarketConfig,
        qty: Decimal,
        price: Option<Decimal>,
    ) -> Result<()> {
        ensure!(qty > Decimal::ZERO, "quantity must be > 0");
        ensure!(
            (qty / config.lot_size).fract().is_zero(),
            "quantity does not align with lot size"
        );
        ensure!(qty >= config.min_qty, "quantity below min_qty");
        if let Some(price) = price {
            ensure!(
                price * qty >= config.min_notional,
                "notional below min_notional"
            );
        }
        Ok(())
    }

    fn prepare_funds(
        &self,
        config: &SpotMarketConfig,
        req: &NewOrderRequest,
    ) -> Result<(Decimal, Decimal)> {
        match (req.order_type, req.side) {
            (OrderType::Limit, Side::Buy) => {
                let qty = req.quantity.unwrap_or_default();
                let price = req.price.unwrap_or_default();
                Ok((qty, price * qty))
            }
            (OrderType::Limit, Side::Sell) => {
                let qty = req.quantity.unwrap_or_default();
                Ok((qty, qty))
            }
            (OrderType::Market, Side::Sell) => {
                let qty = req.quantity.unwrap_or_default();
                Ok((qty, qty))
            }
            (OrderType::Market, Side::Buy) => {
                let budget = req.quote_quantity.unwrap_or_default();
                let best_ask = self
                    .market(&req.symbol)?
                    .best_ask()
                    .ok_or_else(|| anyhow!("market buy requires ask liquidity"))?;
                let qty = budget / best_ask;
                ensure!(qty >= config.min_qty, "market buy budget below min_qty");
                Ok((qty, budget))
            }
        }
    }

    fn lock_order_funds(
        &mut self,
        config: &SpotMarketConfig,
        req: &NewOrderRequest,
        amount: Decimal,
    ) -> Result<()> {
        let asset = match req.side {
            Side::Buy => &config.quote_asset,
            Side::Sell => &config.base_asset,
        };
        self.move_available_to_locked(req.account_id, asset, amount)
    }

    fn unlock_order_funds(
        &mut self,
        config: &SpotMarketConfig,
        req: &NewOrderRequest,
        amount: Decimal,
    ) -> Result<()> {
        let asset = match req.side {
            Side::Buy => &config.quote_asset,
            Side::Sell => &config.base_asset,
        };
        self.release_locked(req.account_id, asset, amount)
    }

    fn release_locked_for_order(
        &mut self,
        config: &SpotMarketConfig,
        order: &RestingOrder,
        amount: Decimal,
    ) -> Result<()> {
        let asset = match order.side {
            Side::Buy => &config.quote_asset,
            Side::Sell => &config.base_asset,
        };
        self.release_locked(order.account_id, asset, amount)
    }

    fn move_available_to_locked(
        &mut self,
        account_id: AccountId,
        asset: &str,
        amount: Decimal,
    ) -> Result<()> {
        if amount <= Decimal::ZERO {
            return Ok(());
        }
        let balance = self.balance_mut(account_id, asset);
        ensure!(
            balance.available >= amount,
            "insufficient available balance for {}",
            asset
        );
        balance.available -= amount;
        balance.locked += amount;
        Ok(())
    }

    fn release_locked(
        &mut self,
        account_id: AccountId,
        asset: &str,
        amount: Decimal,
    ) -> Result<()> {
        if amount <= Decimal::ZERO {
            return Ok(());
        }
        let balance = self.balance_mut(account_id, asset);
        ensure!(
            balance.locked >= amount,
            "insufficient locked balance for {}",
            asset
        );
        balance.locked -= amount;
        balance.available += amount;
        Ok(())
    }

    fn debit_locked(&mut self, account_id: AccountId, asset: &str, amount: Decimal) -> Result<()> {
        if amount <= Decimal::ZERO {
            return Ok(());
        }
        let balance = self.balance_mut(account_id, asset);
        ensure!(
            balance.locked >= amount,
            "insufficient locked balance for {}",
            asset
        );
        balance.locked -= amount;
        Ok(())
    }

    fn credit_available(
        &mut self,
        account_id: AccountId,
        asset: &str,
        amount: Decimal,
    ) -> Result<()> {
        if amount <= Decimal::ZERO {
            return Ok(());
        }
        let balance = self.balance_mut(account_id, asset);
        balance.available += amount;
        Ok(())
    }

    fn balance_mut(&mut self, account_id: AccountId, asset: &str) -> &mut Balance {
        self.accounts
            .entry(account_id)
            .or_default()
            .balances
            .entry(asset.to_string())
            .or_default()
    }

    fn market(&self, symbol: &str) -> Result<&SpotMarket> {
        self.markets
            .get(symbol)
            .ok_or_else(|| anyhow!("unknown market: {}", symbol))
    }

    fn next_order_id(&mut self) -> OrderId {
        self.next_order_id += 1;
        self.next_order_id
    }
}

pub fn run_spot_engine_demo() -> Result<()> {
    let mut engine = SpotMatchingEngine::new();
    engine.create_market(SpotMarketConfig {
        symbol: "BTCUSDT".to_string(),
        base_asset: "BTC".to_string(),
        quote_asset: "USDT".to_string(),
        tick_size: dec!(0.01),
        lot_size: dec!(0.0001),
        min_qty: dec!(0.0001),
        min_notional: dec!(10),
        maker_fee_rate: dec!(0.001),
        taker_fee_rate: dec!(0.001),
    })?;

    engine.deposit(1001, "USDT", dec!(50000))?;
    engine.deposit(2001, "BTC", dec!(2))?;
    engine.deposit(3001, "BTC", dec!(1))?;

    let ask1 = engine.submit_order(NewOrderRequest {
        account_id: 2001,
        symbol: "BTCUSDT".to_string(),
        side: Side::Sell,
        order_type: OrderType::Limit,
        time_in_force: TimeInForce::Gtc,
        price: Some(dec!(64000.00)),
        quantity: Some(dec!(0.4000)),
        quote_quantity: None,
        client_order_id: Some("maker-ask-1".to_string()),
        self_trade_prevention: SelfTradePrevention::CancelNewest,
    })?;

    let ask2 = engine.submit_order(NewOrderRequest {
        account_id: 3001,
        symbol: "BTCUSDT".to_string(),
        side: Side::Sell,
        order_type: OrderType::Limit,
        time_in_force: TimeInForce::Gtc,
        price: Some(dec!(64010.00)),
        quantity: Some(dec!(0.3000)),
        quote_quantity: None,
        client_order_id: Some("maker-ask-2".to_string()),
        self_trade_prevention: SelfTradePrevention::CancelNewest,
    })?;

    let taker = engine.submit_order(NewOrderRequest {
        account_id: 1001,
        symbol: "BTCUSDT".to_string(),
        side: Side::Buy,
        order_type: OrderType::Limit,
        time_in_force: TimeInForce::Ioc,
        price: Some(dec!(64010.00)),
        quantity: Some(dec!(0.5000)),
        quote_quantity: None,
        client_order_id: Some("taker-buy-1".to_string()),
        self_trade_prevention: SelfTradePrevention::CancelNewest,
    })?;

    println!("== Spot Matching Demo ==");
    println!("maker ask order ids: {}, {}", ask1.order_id, ask2.order_id);
    println!(
        "taker result: status={:?}, filled_qty={}, filled_quote={}, trades={}",
        taker.status,
        taker.filled_qty,
        taker.filled_quote_qty,
        taker.trades.len()
    );
    for trade in &taker.trades {
        println!(
            "trade#{} price={} qty={} maker={} taker={} seq={}",
            trade.trade_id,
            trade.price,
            trade.quantity,
            trade.maker_order_id,
            trade.taker_order_id,
            trade.sequence
        );
    }

    let snapshot = engine.orderbook_snapshot("BTCUSDT", 5)?;
    println!("book sequence={}", snapshot.sequence);
    println!("bids={:?}", snapshot.bids);
    println!("asks={:?}", snapshot.asks);
    println!("acct1001 USDT={:?}", engine.balance_of(1001, "USDT"));
    println!("acct1001 BTC={:?}", engine.balance_of(1001, "BTC"));

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_engine() -> SpotMatchingEngine {
        let mut engine = SpotMatchingEngine::new();
        engine
            .create_market(SpotMarketConfig {
                symbol: "BTCUSDT".to_string(),
                base_asset: "BTC".to_string(),
                quote_asset: "USDT".to_string(),
                tick_size: dec!(0.01),
                lot_size: dec!(0.0001),
                min_qty: dec!(0.0001),
                min_notional: dec!(10),
                maker_fee_rate: dec!(0.001),
                taker_fee_rate: dec!(0.001),
            })
            .unwrap();
        engine.deposit(1, "USDT", dec!(100000)).unwrap();
        engine.deposit(2, "BTC", dec!(10)).unwrap();
        engine.deposit(3, "BTC", dec!(10)).unwrap();
        engine
    }

    #[test]
    fn limit_order_matches_price_time_priority() {
        let mut engine = make_engine();

        let maker1 = engine
            .submit_order(NewOrderRequest {
                account_id: 2,
                symbol: "BTCUSDT".to_string(),
                side: Side::Sell,
                order_type: OrderType::Limit,
                time_in_force: TimeInForce::Gtc,
                price: Some(dec!(60000)),
                quantity: Some(dec!(0.3)),
                quote_quantity: None,
                client_order_id: None,
                self_trade_prevention: SelfTradePrevention::CancelNewest,
            })
            .unwrap();
        let maker2 = engine
            .submit_order(NewOrderRequest {
                account_id: 3,
                symbol: "BTCUSDT".to_string(),
                side: Side::Sell,
                order_type: OrderType::Limit,
                time_in_force: TimeInForce::Gtc,
                price: Some(dec!(60000)),
                quantity: Some(dec!(0.4)),
                quote_quantity: None,
                client_order_id: None,
                self_trade_prevention: SelfTradePrevention::CancelNewest,
            })
            .unwrap();

        let taker = engine
            .submit_order(NewOrderRequest {
                account_id: 1,
                symbol: "BTCUSDT".to_string(),
                side: Side::Buy,
                order_type: OrderType::Limit,
                time_in_force: TimeInForce::Ioc,
                price: Some(dec!(60000)),
                quantity: Some(dec!(0.5)),
                quote_quantity: None,
                client_order_id: None,
                self_trade_prevention: SelfTradePrevention::CancelNewest,
            })
            .unwrap();

        assert_eq!(maker1.status, OrderStatus::New);
        assert_eq!(maker2.status, OrderStatus::New);
        assert_eq!(taker.trades.len(), 2);
        assert_eq!(taker.trades[0].maker_order_id, maker1.order_id);
        assert_eq!(taker.trades[1].maker_order_id, maker2.order_id);
        assert_eq!(taker.filled_qty, dec!(0.5));

        let book = engine.orderbook_snapshot("BTCUSDT", 5).unwrap();
        assert_eq!(book.asks[0].total_qty, dec!(0.2));
    }

    #[test]
    fn post_only_order_rejects_when_crossing() {
        let mut engine = make_engine();
        engine
            .submit_order(NewOrderRequest {
                account_id: 2,
                symbol: "BTCUSDT".to_string(),
                side: Side::Sell,
                order_type: OrderType::Limit,
                time_in_force: TimeInForce::Gtc,
                price: Some(dec!(60000)),
                quantity: Some(dec!(0.1)),
                quote_quantity: None,
                client_order_id: None,
                self_trade_prevention: SelfTradePrevention::CancelNewest,
            })
            .unwrap();

        let res = engine
            .submit_order(NewOrderRequest {
                account_id: 1,
                symbol: "BTCUSDT".to_string(),
                side: Side::Buy,
                order_type: OrderType::Limit,
                time_in_force: TimeInForce::PostOnly,
                price: Some(dec!(60000)),
                quantity: Some(dec!(0.1)),
                quote_quantity: None,
                client_order_id: None,
                self_trade_prevention: SelfTradePrevention::CancelNewest,
            })
            .unwrap();

        assert_eq!(res.status, OrderStatus::Rejected);
    }

    #[test]
    fn cancel_releases_locked_balance() {
        let mut engine = make_engine();
        let res = engine
            .submit_order(NewOrderRequest {
                account_id: 1,
                symbol: "BTCUSDT".to_string(),
                side: Side::Buy,
                order_type: OrderType::Limit,
                time_in_force: TimeInForce::Gtc,
                price: Some(dec!(50000)),
                quantity: Some(dec!(1)),
                quote_quantity: None,
                client_order_id: None,
                self_trade_prevention: SelfTradePrevention::CancelNewest,
            })
            .unwrap();

        let before = engine.balance_of(1, "USDT");
        assert_eq!(before.locked, dec!(50000));

        engine.cancel_order("BTCUSDT", res.order_id).unwrap();
        let after = engine.balance_of(1, "USDT");
        assert_eq!(after.locked, Decimal::ZERO);
        assert_eq!(after.available, dec!(100000));
    }
}
