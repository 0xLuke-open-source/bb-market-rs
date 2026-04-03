//! TradingCore 是交易域的真正状态容器。
//!
//! service 层偏“接口编排”，而 core 层偏“状态变更”：
//! 它维护 open orders、history、止损单、流动性订单以及 event 序号。

use std::collections::{BTreeSet, HashMap};

use anyhow::Result;
use rust_decimal::prelude::ToPrimitive;
use rust_decimal::Decimal;

use crate::engine::{
    NewOrderRequest, OrderStatus, OrderType, SelfTradePrevention, Side, SpotMatchingEngine,
    SubmitOrderResult, TimeInForce, Trade,
};
use crate::terminal::application::projection::{
    TraderBalanceJson, TraderOrderJson, TraderStateJson, TraderTradeJson,
};

use super::helpers::{
    decimal_from_f64, now_ts, order_from_submit, parse_order_type, parse_side, parse_tif,
    should_trigger_stop, trader_order_from_view, trader_trade_from_trade, SubmitOrderResultExt,
};
use super::types::{ArchiveEvent, StopOrder, SymbolLiquidity, TradingState};
use super::USER_ACCOUNT_ID;

pub(super) struct TradingCore {
    // 本地撮合引擎，负责真实的订单撮合与余额变更。
    pub engine: SpotMatchingEngine,
    // 与前端展示和回放相关的附加状态。
    pub state: TradingState,
}

impl TradingCore {
    pub fn new(engine: SpotMatchingEngine, assets: BTreeSet<String>) -> Self {
        Self {
            engine,
            state: TradingState {
                assets,
                open_orders: HashMap::new(),
                order_history: Vec::new(),
                trade_history: Vec::new(),
                liquidity: HashMap::new(),
                next_event_seq: 0,
                next_virtual_order_id: 10_000_000_000,
                stop_orders: HashMap::new(),
            },
        }
    }

    pub fn snapshot(&self) -> TraderStateJson {
        // 这里把引擎余额 + 本地订单/成交历史拼成前端需要的 TraderStateJson。
        let mut balances: Vec<TraderBalanceJson> = self
            .state
            .assets
            .iter()
            .map(|asset| {
                let balance = self.engine.balance_of(USER_ACCOUNT_ID, asset);
                TraderBalanceJson {
                    asset: asset.clone(),
                    available: balance.available.to_f64().unwrap_or(0.0),
                    locked: balance.locked.to_f64().unwrap_or(0.0),
                }
            })
            .filter(|balance| balance.available > 0.0 || balance.locked > 0.0)
            .collect();
        balances.sort_by(|left, right| left.asset.cmp(&right.asset));

        let mut open_orders: Vec<TraderOrderJson> =
            self.state.open_orders.values().cloned().collect();
        open_orders.sort_by(|left, right| right.order_id.cmp(&left.order_id));

        TraderStateJson {
            account_id: USER_ACCOUNT_ID,
            balances,
            open_orders,
            order_history: self.state.order_history.clone(),
            trade_history: self.state.trade_history.clone(),
        }
    }

    pub fn apply_user_submit(
        &mut self,
        symbol: &str,
        req: &NewOrderRequest,
        result: &SubmitOrderResult,
    ) -> Result<Vec<TraderTradeJson>> {
        // 对提交结果做二次整理：
        // 1. 捕获用户相关成交
        // 2. 更新 open order / order history
        // 3. 同步 maker 订单状态
        let trade_logs = self.capture_user_trades(symbol, &result.trades)?;

        match result.status {
            OrderStatus::New | OrderStatus::PartiallyFilled => {
                let view = self.engine.get_order(symbol, result.order_id)?;
                self.state
                    .open_orders
                    .insert(view.order_id, trader_order_from_view(&view));
            }
            _ => {
                self.push_order_history(order_from_submit(req, result));
            }
        }

        for trade in &result.trades {
            if trade.maker_account_id == USER_ACCOUNT_ID {
                self.refresh_order_after_trade(symbol, trade.maker_order_id, trade)?;
            }
        }

        Ok(trade_logs)
    }

    pub fn capture_user_trades(
        &mut self,
        symbol: &str,
        trades: &[Trade],
    ) -> Result<Vec<TraderTradeJson>> {
        let mut captured = Vec::new();
        for trade in trades {
            if trade.taker_account_id == USER_ACCOUNT_ID
                || trade.maker_account_id == USER_ACCOUNT_ID
            {
                let trade_json = trader_trade_from_trade(trade);
                self.state.trade_history.insert(0, trade_json.clone());
                if self.state.trade_history.len() > 200 {
                    self.state.trade_history.truncate(200);
                }
                captured.push(trade_json);

                if trade.maker_account_id == USER_ACCOUNT_ID {
                    self.refresh_order_after_trade(symbol, trade.maker_order_id, trade)?;
                }
            }
        }
        Ok(captured)
    }

    pub fn refresh_order_after_trade(
        &mut self,
        symbol: &str,
        order_id: u64,
        trade: &Trade,
    ) -> Result<()> {
        if let Ok(view) = self.engine.get_order(symbol, order_id) {
            self.state
                .open_orders
                .insert(order_id, trader_order_from_view(&view));
            return Ok(());
        }

        if let Some(mut order) = self.state.open_orders.remove(&order_id) {
            order.filled_qty += trade.quantity.to_f64().unwrap_or(0.0);
            order.filled_quote_qty += trade.quote_quantity.to_f64().unwrap_or(0.0);
            order.remaining_qty = 0.0;
            order.status = "Filled".to_string();
            self.push_order_history(order);
        }

        Ok(())
    }

    pub fn push_order_history(&mut self, order: TraderOrderJson) {
        self.state.order_history.insert(0, order);
        if self.state.order_history.len() > 200 {
            self.state.order_history.truncate(200);
        }
    }

    pub fn trigger_stop_orders(
        &mut self,
        symbol: &str,
        mid: Decimal,
    ) -> Result<Vec<super::types::OrderActionResult>> {
        // 这里不是交易所侧的“真实止损单”，而是本地维护的虚拟触发单。
        // 一旦中间价满足条件，就把虚拟单转换成真实 NewOrderRequest 再送入引擎。
        let ids: Vec<u64> = self
            .state
            .stop_orders
            .values()
            .filter(|order| order.request.symbol.eq_ignore_ascii_case(symbol))
            .filter(|order| should_trigger_stop(&order.request, mid))
            .map(|order| order.order_id)
            .collect();

        let mut results = Vec::new();
        for id in ids {
            if let Some(stop_order) = self.state.stop_orders.remove(&id) {
                self.state.open_orders.remove(&id);
                let side = parse_side(&stop_order.request.side)?;
                let parsed = parse_order_type(&stop_order.request.order_type)?;
                let engine_order_type = match parsed {
                    super::types::ParsedOrderType::StopLimit => OrderType::Limit,
                    super::types::ParsedOrderType::StopMarket => OrderType::Market,
                    _ => continue,
                };
                let quantity = decimal_from_f64(stop_order.request.quantity, "quantity")?;
                let price = stop_order
                    .request
                    .price
                    .map(|value| decimal_from_f64(value, "price"))
                    .transpose()?;
                let engine_req = match (engine_order_type, side) {
                    (OrderType::Market, Side::Buy) => NewOrderRequest {
                        account_id: USER_ACCOUNT_ID,
                        symbol: stop_order.request.symbol.to_uppercase(),
                        side,
                        order_type: engine_order_type,
                        time_in_force: TimeInForce::Ioc,
                        price: None,
                        quantity: None,
                        quote_quantity: Some(quantity * price.unwrap_or(mid)),
                        client_order_id: Some(format!("stop-trigger-{}", id)),
                        self_trade_prevention: SelfTradePrevention::CancelNewest,
                    },
                    _ => NewOrderRequest {
                        account_id: USER_ACCOUNT_ID,
                        symbol: stop_order.request.symbol.to_uppercase(),
                        side,
                        order_type: engine_order_type,
                        time_in_force: parse_tif(
                            stop_order.request.time_in_force.as_deref(),
                            engine_order_type,
                        )?,
                        price,
                        quantity: Some(quantity),
                        quote_quantity: None,
                        client_order_id: Some(format!("stop-trigger-{}", id)),
                        self_trade_prevention: SelfTradePrevention::CancelNewest,
                    },
                };
                let result = self.engine.submit_order(engine_req.clone())?;
                let _ = self.apply_user_submit(symbol, &engine_req, &result)?;
                let mut history_order = TraderOrderJson {
                    time: stop_order.created_at.clone(),
                    order_id: id,
                    symbol: stop_order.request.symbol.to_uppercase(),
                    side: stop_order.request.side.to_ascii_uppercase(),
                    order_type: stop_order.request.order_type.to_ascii_uppercase(),
                    time_in_force: stop_order
                        .request
                        .time_in_force
                        .clone()
                        .unwrap_or_else(|| "gtc".to_string())
                        .to_ascii_uppercase(),
                    price: stop_order.request.price,
                    trigger_price: stop_order.request.trigger_price,
                    trigger_kind: stop_order.request.trigger_kind.clone(),
                    quantity: stop_order.request.quantity,
                    remaining_qty: 0.0,
                    filled_qty: result.filled_qty.to_f64().unwrap_or(0.0),
                    filled_quote_qty: result.filled_quote_qty.to_f64().unwrap_or(0.0),
                    status: format!("Triggered->{}", result.status_string()),
                };
                if history_order.filled_qty == 0.0 && history_order.status == "Triggered->New" {
                    history_order.status = "Triggered".to_string();
                }
                self.push_order_history(history_order);
                results.push(super::helpers::summarize_result(&result));
            }
        }

        Ok(results)
    }

    pub fn archive_event(
        &mut self,
        kind: &str,
        symbol: Option<String>,
        summary: String,
        snapshot: Option<TraderStateJson>,
        order: Option<TraderOrderJson>,
        trade: Option<TraderTradeJson>,
    ) -> ArchiveEvent {
        self.state.next_event_seq += 1;
        ArchiveEvent {
            ts: now_ts(),
            seq: self.state.next_event_seq,
            kind: kind.to_string(),
            symbol,
            summary,
            snapshot,
            order,
            trade,
        }
    }

    pub fn liquidity_entry(&mut self, symbol: &str) -> SymbolLiquidity {
        self.state.liquidity.remove(symbol).unwrap_or_default()
    }

    pub fn store_liquidity_entry(&mut self, symbol: String, entry: SymbolLiquidity) {
        self.state.liquidity.insert(symbol, entry);
    }

    pub fn next_virtual_order_id(&mut self) -> u64 {
        let next = self.state.next_virtual_order_id;
        self.state.next_virtual_order_id += 1;
        next
    }

    pub fn open_order(&self, order_id: u64) -> Option<TraderOrderJson> {
        self.state.open_orders.get(&order_id).cloned()
    }

    pub fn remove_stop_order(&mut self, order_id: u64) -> Option<StopOrder> {
        self.state.stop_orders.remove(&order_id)
    }

    pub fn insert_open_order(&mut self, order_id: u64, order: TraderOrderJson) {
        self.state.open_orders.insert(order_id, order);
    }

    pub fn remove_open_order(&mut self, order_id: u64) {
        self.state.open_orders.remove(&order_id);
    }

    pub fn insert_stop_order(&mut self, order_id: u64, order: StopOrder) {
        self.state.stop_orders.insert(order_id, order);
    }

    pub fn open_orders_for_symbol(&self, symbol: Option<&String>) -> Vec<u64> {
        self.state
            .open_orders
            .values()
            .filter(|order| symbol.map(|value| &order.symbol == value).unwrap_or(true))
            .map(|order| order.order_id)
            .collect()
    }
}
