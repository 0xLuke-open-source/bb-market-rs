//! service 层提供 SpotTradingService 的对外接口。
//!
//! 前端/HTTP handler 只应该依赖这一层，而不直接碰 TradingCore，
//! 这样业务流程、日志记录、回放归档都能集中在一个地方维护。

use std::collections::BTreeSet;
use std::fs;
use std::path::Path;

use anyhow::{anyhow, ensure, Result};
use rust_decimal::prelude::ToPrimitive;
use rust_decimal::Decimal;
use rust_decimal_macros::dec;

use crate::execution::domain::matching::{
    NewOrderRequest, OrderType, SelfTradePrevention, Side, SpotMarketConfig, TimeInForce,
};
use crate::instrument_catalog::domain::SymbolPrecision;

use super::core::TradingCore;
use super::helpers::{
    decimal_from_f64, now_text, now_ts, parse_base_asset, parse_order_type, parse_side, parse_tif,
    summarize_result,
};
use super::types::{
    ApiOrderRequest, ArchiveEvent, CancelAllRequest, CancelAllResult, OrderActionResult,
    ParsedOrderType, ReplayEventJson, ReplayQuery, ReplayResponse, StopOrder, TraderOrderJson,
    TraderStateJson,
};
use super::{SpotTradingService, LIQUIDITY_ACCOUNT_ID, USER_ACCOUNT_ID};

impl SpotTradingService {
    pub fn new(
        symbols: &[String],
        precisions: std::collections::HashMap<String, SymbolPrecision>,
        log_dir: impl AsRef<Path>,
    ) -> Result<Self> {
        // 启动时顺手创建一个“带流动性账户”的本地市场。
        // 用户账户和流动性账户都预存资产，这样前端下单后立刻可以成交。
        let log_dir = log_dir.as_ref().to_path_buf();
        fs::create_dir_all(&log_dir)?;

        let mut engine = crate::execution::domain::matching::SpotMatchingEngine::new();
        let mut assets = BTreeSet::new();
        assets.insert("USDT".to_string());

        engine.deposit(USER_ACCOUNT_ID, "USDT", dec!(1000000))?;
        engine.deposit(LIQUIDITY_ACCOUNT_ID, "USDT", dec!(1000000000))?;

        for symbol in symbols {
            let base_asset = parse_base_asset(symbol)?;
            assets.insert(base_asset.clone());
            let precision = precisions.get(symbol).copied().unwrap_or_default();
            let tick_size = step_from_precision(precision.price_precision, dec!(0.00000001));
            let lot_size = step_from_precision(precision.quantity_precision, dec!(0.00000001));

            engine.create_market(SpotMarketConfig {
                symbol: symbol.clone(),
                base_asset: base_asset.clone(),
                quote_asset: "USDT".to_string(),
                tick_size,
                lot_size,
                min_qty: lot_size,
                min_notional: dec!(0),
                maker_fee_rate: dec!(0.001),
                taker_fee_rate: dec!(0.001),
            })?;

            engine.deposit(USER_ACCOUNT_ID, &base_asset, dec!(10000))?;
            engine.deposit(LIQUIDITY_ACCOUNT_ID, &base_asset, dec!(1000000000))?;
        }

        Ok(Self {
            inner: std::sync::Arc::new(tokio::sync::Mutex::new(TradingCore::new(engine, assets))),
            log_dir: std::sync::Arc::new(log_dir),
            market_precisions: std::sync::Arc::new(precisions),
        })
    }

    pub async fn snapshot(&self) -> TraderStateJson {
        let core = self.inner.lock().await;
        core.snapshot()
    }

    pub async fn replay(&self, query: ReplayQuery) -> Result<ReplayResponse> {
        // replay 不重放引擎状态机，而是基于 archive 日志构造某个时间点附近的快照与事件。
        let mut events = self.load_archive_events()?;
        events.sort_by(|left, right| left.ts.cmp(&right.ts).then(left.seq.cmp(&right.seq)));

        let at_ts = query.at_ts.unwrap_or(i64::MAX);
        let from_ts = query.from_ts.unwrap_or(0);
        let limit = query.limit.unwrap_or(200);

        let snapshot = events
            .iter()
            .filter(|event| event.ts <= at_ts)
            .rev()
            .find_map(|event| event.snapshot.clone())
            .unwrap_or_default();

        let mut filtered: Vec<ReplayEventJson> = events
            .into_iter()
            .filter(|event| event.ts >= from_ts && event.ts <= at_ts)
            .map(|event| ReplayEventJson {
                ts: event.ts,
                seq: event.seq,
                kind: event.kind,
                symbol: event.symbol,
                summary: event.summary,
            })
            .collect();
        if filtered.len() > limit {
            filtered = filtered.split_off(filtered.len() - limit);
        }
        filtered.reverse();

        Ok(ReplayResponse {
            snapshot,
            events: filtered,
        })
    }

    pub async fn submit_order(&self, req: ApiOrderRequest) -> Result<OrderActionResult> {
        // 先把前端请求翻译成引擎请求，再统一做状态刷新与日志落盘。
        let symbol = req.symbol.to_uppercase();
        let side = parse_side(&req.side)?;
        let order_type = parse_order_type(&req.order_type)?;
        if matches!(
            order_type,
            ParsedOrderType::StopLimit | ParsedOrderType::StopMarket
        ) {
            return self.submit_stop_order(req).await;
        }

        let engine_order_type = match order_type {
            ParsedOrderType::Limit => OrderType::Limit,
            ParsedOrderType::Market => OrderType::Market,
            ParsedOrderType::StopLimit | ParsedOrderType::StopMarket => unreachable!(),
        };
        let time_in_force = parse_tif(req.time_in_force.as_deref(), engine_order_type)?;
        let quantity = decimal_from_f64(req.quantity, "quantity")?;
        self.ensure_quantity_precision(&symbol, quantity, "quantity")?;
        let price = req
            .price
            .map(|value| decimal_from_f64(value, "price"))
            .transpose()?;
        if let Some(price) = price {
            self.ensure_price_precision(&symbol, price, "price")?;
        }

        let engine_req = match (engine_order_type, side) {
            (OrderType::Market, Side::Buy) => NewOrderRequest {
                account_id: USER_ACCOUNT_ID,
                symbol: symbol.clone(),
                side,
                order_type: engine_order_type,
                time_in_force,
                price: None,
                quantity: None,
                quote_quantity: Some(quantity * price.unwrap_or(dec!(0))),
                client_order_id: None,
                self_trade_prevention: SelfTradePrevention::CancelNewest,
            },
            _ => NewOrderRequest {
                account_id: USER_ACCOUNT_ID,
                symbol: symbol.clone(),
                side,
                order_type: engine_order_type,
                time_in_force,
                price,
                quantity: Some(quantity),
                quote_quantity: None,
                client_order_id: None,
                self_trade_prevention: SelfTradePrevention::CancelNewest,
            },
        };

        let (result, trader_snapshot, trade_logs, archive_events) = {
            let mut core = self.inner.lock().await;
            let result = core.engine.submit_order(engine_req.clone())?;
            let trade_logs = core.apply_user_submit(&symbol, &engine_req, &result)?;
            let snapshot = core.snapshot();
            let mut archive_events = Vec::new();
            archive_events.push(core.archive_event(
                "submit",
                Some(symbol.clone()),
                format!(
                    "submit {:?} {:?} {} qty={}",
                    side, order_type, symbol, req.quantity
                ),
                Some(snapshot.clone()),
                None,
                None,
            ));
            for trade in &trade_logs {
                archive_events.push(core.archive_event(
                    "trade",
                    Some(trade.symbol.clone()),
                    format!("trade {} {} @ {}", trade.side, trade.quantity, trade.price),
                    None,
                    None,
                    Some(trade.clone()),
                ));
            }
            archive_events.push(core.archive_event(
                "snapshot",
                Some(symbol.clone()),
                format!("snapshot {}", symbol),
                Some(snapshot.clone()),
                None,
                None,
            ));
            (result, snapshot, trade_logs, archive_events)
        };

        self.log_json(
            "orders.jsonl",
            &serde_json::json!({
                "ts": now_ts(),
                "action": "submit",
                "request": req,
                "result": summarize_result(&result),
            }),
        )?;
        for trade in &trade_logs {
            self.log_json("trades.jsonl", trade)?;
        }
        self.log_json("balances.jsonl", &trader_snapshot)?;
        self.log_archive_events(&archive_events)?;

        Ok(summarize_result(&result))
    }

    pub async fn cancel_order(&self, order_id: u64) -> Result<OrderActionResult> {
        let (order, trader_snapshot, archive_events) = {
            let mut core = self.inner.lock().await;
            let order = core
                .open_order(order_id)
                .ok_or_else(|| anyhow!("order not found: {}", order_id))?;

            if core.remove_stop_order(order_id).is_some() {
                let mut final_order = order.clone();
                final_order.status = "Cancelled".to_string();
                final_order.remaining_qty = 0.0;
                core.remove_open_order(order_id);
                core.push_order_history(final_order.clone());
                let snapshot = core.snapshot();
                let archive_events = vec![
                    core.archive_event(
                        "cancel",
                        Some(order.symbol.clone()),
                        format!("cancel stop order={} symbol={}", order_id, order.symbol),
                        Some(snapshot.clone()),
                        Some(final_order),
                        None,
                    ),
                    core.archive_event(
                        "snapshot",
                        Some(order.symbol.clone()),
                        format!("snapshot {}", order.symbol),
                        Some(snapshot.clone()),
                        None,
                        None,
                    ),
                ];
                drop(core);
                self.log_json(
                    "orders.jsonl",
                    &serde_json::json!({
                        "ts": now_ts(),
                        "action": "cancel",
                        "order_id": order_id,
                        "symbol": order.symbol,
                    }),
                )?;
                self.log_json("balances.jsonl", &snapshot)?;
                self.log_archive_events(&archive_events)?;
                return Ok(OrderActionResult {
                    order_id,
                    status: "Cancelled".to_string(),
                    filled_qty: order.filled_qty,
                    filled_quote_qty: order.filled_quote_qty,
                    remaining_qty: 0.0,
                    trade_count: 0,
                });
            }

            let res = match core.engine.cancel_order(&order.symbol, order_id) {
                Ok(result) => result,
                Err(_) => {
                    let mut final_order = order.clone();
                    final_order.status = "Cancelled".to_string();
                    final_order.remaining_qty = 0.0;
                    core.remove_open_order(order_id);
                    core.push_order_history(final_order.clone());
                    let snapshot = core.snapshot();
                    let archive_events = vec![
                        core.archive_event(
                            "cancel",
                            Some(order.symbol.clone()),
                            format!("cancel local order={} symbol={}", order_id, order.symbol),
                            Some(snapshot.clone()),
                            Some(final_order),
                            None,
                        ),
                        core.archive_event(
                            "snapshot",
                            Some(order.symbol.clone()),
                            format!("snapshot {}", order.symbol),
                            Some(snapshot.clone()),
                            None,
                            None,
                        ),
                    ];
                    drop(core);
                    self.log_json(
                        "orders.jsonl",
                        &serde_json::json!({
                            "ts": now_ts(),
                            "action": "cancel",
                            "order_id": order_id,
                            "symbol": order.symbol,
                        }),
                    )?;
                    self.log_json("balances.jsonl", &snapshot)?;
                    self.log_archive_events(&archive_events)?;
                    return Ok(OrderActionResult {
                        order_id,
                        status: "Cancelled".to_string(),
                        filled_qty: order.filled_qty,
                        filled_quote_qty: order.filled_quote_qty,
                        remaining_qty: 0.0,
                        trade_count: 0,
                    });
                }
            };
            let mut final_order = order.clone();
            final_order.status = format!("{:?}", res.status);
            final_order.remaining_qty = 0.0;
            core.remove_open_order(order_id);
            core.push_order_history(final_order);
            let snapshot = core.snapshot();
            let archive_events = vec![
                core.archive_event(
                    "cancel",
                    Some(order.symbol.clone()),
                    format!("cancel order={} symbol={}", order_id, order.symbol),
                    Some(snapshot.clone()),
                    Some(order.clone()),
                    None,
                ),
                core.archive_event(
                    "snapshot",
                    Some(order.symbol.clone()),
                    format!("snapshot {}", order.symbol),
                    Some(snapshot.clone()),
                    None,
                    None,
                ),
            ];
            (order, snapshot, archive_events)
        };

        self.log_json(
            "orders.jsonl",
            &serde_json::json!({
                "ts": now_ts(),
                "action": "cancel",
                "order_id": order_id,
                "symbol": order.symbol,
            }),
        )?;
        self.log_json("balances.jsonl", &trader_snapshot)?;
        self.log_archive_events(&archive_events)?;

        Ok(OrderActionResult {
            order_id,
            status: "Cancelled".to_string(),
            filled_qty: order.filled_qty,
            filled_quote_qty: order.filled_quote_qty,
            remaining_qty: 0.0,
            trade_count: 0,
        })
    }

    pub async fn cancel_all(&self, req: CancelAllRequest) -> Result<CancelAllResult> {
        let symbol = req.symbol.map(|value| value.to_uppercase());
        let (cancelled, trader_snapshot, archive_events) = {
            let mut core = self.inner.lock().await;
            let ids = core.open_orders_for_symbol(symbol.as_ref());

            for order_id in &ids {
                if let Some(order) = core.open_order(*order_id) {
                    if core.remove_stop_order(*order_id).is_none() {
                        let _ = core.engine.cancel_order(&order.symbol, *order_id);
                    }
                    let mut final_order = order.clone();
                    final_order.status = "Cancelled".to_string();
                    final_order.remaining_qty = 0.0;
                    core.remove_open_order(*order_id);
                    core.push_order_history(final_order);
                }
            }
            let snapshot = core.snapshot();
            let archive_events = vec![
                core.archive_event(
                    "cancel_all",
                    symbol.clone(),
                    format!("cancel_all symbol={:?} cancelled={}", symbol, ids.len()),
                    Some(snapshot.clone()),
                    None,
                    None,
                ),
                core.archive_event(
                    "snapshot",
                    symbol.clone(),
                    "snapshot after cancel_all".to_string(),
                    Some(snapshot.clone()),
                    None,
                    None,
                ),
            ];
            (ids.len(), snapshot, archive_events)
        };

        self.log_json(
            "orders.jsonl",
            &serde_json::json!({
                "ts": now_ts(),
                "action": "cancel_all",
                "symbol": symbol,
                "cancelled": cancelled,
            }),
        )?;
        self.log_json("balances.jsonl", &trader_snapshot)?;
        self.log_archive_events(&archive_events)?;

        Ok(CancelAllResult { cancelled })
    }

    pub async fn sync_liquidity(
        &self,
        symbol: &str,
        bids: &[(Decimal, Decimal)],
        asks: &[(Decimal, Decimal)],
    ) -> Result<()> {
        // bridge 每轮都会把盘口前几档同步进来。
        // 这里的策略是“先清空旧流动性，再重建新流动性”，保证模拟深度和最新行情近似一致。
        let symbol = symbol.to_uppercase();
        let mut core = self.inner.lock().await;
        let mut entry = core.liquidity_entry(&symbol);
        let mut trade_logs = Vec::new();

        for id in entry.bid_ids.drain(..) {
            let _ = core.engine.cancel_order(&symbol, id);
        }
        for id in entry.ask_ids.drain(..) {
            let _ = core.engine.cancel_order(&symbol, id);
        }

        for (price, qty) in bids.iter().take(10) {
            if *price <= Decimal::ZERO || *qty <= Decimal::ZERO {
                continue;
            }
            let result = core.engine.submit_order(NewOrderRequest {
                account_id: LIQUIDITY_ACCOUNT_ID,
                symbol: symbol.clone(),
                side: Side::Buy,
                order_type: OrderType::Limit,
                time_in_force: TimeInForce::Gtc,
                price: Some(*price),
                quantity: Some(*qty),
                quote_quantity: None,
                client_order_id: None,
                self_trade_prevention: SelfTradePrevention::CancelNewest,
            })?;
            entry.bid_ids.push(result.order_id);
            trade_logs.extend(core.capture_user_trades(&symbol, &result.trades)?);
        }

        for (price, qty) in asks.iter().take(10) {
            if *price <= Decimal::ZERO || *qty <= Decimal::ZERO {
                continue;
            }
            let result = core.engine.submit_order(NewOrderRequest {
                account_id: LIQUIDITY_ACCOUNT_ID,
                symbol: symbol.clone(),
                side: Side::Sell,
                order_type: OrderType::Limit,
                time_in_force: TimeInForce::Gtc,
                price: Some(*price),
                quantity: Some(*qty),
                quote_quantity: None,
                client_order_id: None,
                self_trade_prevention: SelfTradePrevention::CancelNewest,
            })?;
            entry.ask_ids.push(result.order_id);
            trade_logs.extend(core.capture_user_trades(&symbol, &result.trades)?);
        }

        core.store_liquidity_entry(symbol.clone(), entry);
        let mid = match (bids.first(), asks.first()) {
            (Some((bid_price, _)), Some((ask_price, _))) => {
                Some((*bid_price + *ask_price) / dec!(2))
            }
            _ => None,
        };
        let stop_trigger_results = if let Some(mid) = mid {
            core.trigger_stop_orders(&symbol, mid)?
        } else {
            Vec::new()
        };
        let snapshot = if trade_logs.is_empty() {
            None
        } else {
            Some(core.snapshot())
        };
        let archive_events: Vec<ArchiveEvent> = if let Some(snapshot) = snapshot.clone() {
            let mut events = Vec::new();
            for trade in &trade_logs {
                events.push(core.archive_event(
                    "trade",
                    Some(trade.symbol.clone()),
                    format!("trade {} {} @ {}", trade.side, trade.quantity, trade.price),
                    None,
                    None,
                    Some(trade.clone()),
                ));
            }
            events.push(core.archive_event(
                "snapshot",
                Some(symbol.clone()),
                format!("snapshot {}", symbol),
                Some(snapshot),
                None,
                None,
            ));
            events
        } else {
            Vec::new()
        };
        drop(core);

        for trade in &trade_logs {
            self.log_json("trades.jsonl", trade)?;
        }
        for result in &stop_trigger_results {
            self.log_json(
                "orders.jsonl",
                &serde_json::json!({
                    "ts": now_ts(),
                    "action": "stop_trigger",
                    "result": result,
                }),
            )?;
        }
        if !archive_events.is_empty() {
            self.log_archive_events(&archive_events)?;
        }

        Ok(())
    }

    async fn submit_stop_order(&self, req: ApiOrderRequest) -> Result<OrderActionResult> {
        // 止损单在本项目里是“虚拟挂单”：
        // 先记录在 open_orders + stop_orders，等价格触发后再转换成真实订单提交给引擎。
        let symbol = req.symbol.to_uppercase();
        let quantity = decimal_from_f64(req.quantity, "quantity")?;
        self.ensure_quantity_precision(&symbol, quantity, "quantity")?;
        let trigger_price = decimal_from_f64(
            req.trigger_price
                .ok_or_else(|| anyhow!("trigger_price is required"))?,
            "trigger_price",
        )?;
        self.ensure_price_precision(&symbol, trigger_price, "trigger_price")?;
        if let Some(price) = req.price {
            self.ensure_price_precision(&symbol, decimal_from_f64(price, "price")?, "price")?;
        }
        let mut core = self.inner.lock().await;
        let order_id = core.next_virtual_order_id();

        let order = TraderOrderJson {
            time: now_text(),
            order_id,
            symbol: symbol.clone(),
            side: req.side.to_ascii_uppercase(),
            order_type: req.order_type.to_ascii_uppercase(),
            time_in_force: req
                .time_in_force
                .clone()
                .unwrap_or_else(|| "gtc".to_string())
                .to_ascii_uppercase(),
            price: req.price,
            trigger_price: Some(trigger_price.to_f64().unwrap_or(0.0)),
            trigger_kind: Some(
                req.trigger_kind
                    .clone()
                    .unwrap_or_else(|| "stop_loss".to_string()),
            ),
            quantity: quantity.to_f64().unwrap_or(req.quantity),
            remaining_qty: quantity.to_f64().unwrap_or(req.quantity),
            filled_qty: 0.0,
            filled_quote_qty: 0.0,
            status: "TriggerPending".to_string(),
        };
        core.insert_open_order(order_id, order.clone());
        core.insert_stop_order(
            order_id,
            StopOrder {
                order_id,
                request: req.clone(),
                created_at: order.time.clone(),
            },
        );
        let snapshot = core.snapshot();
        let events = vec![
            core.archive_event(
                "stop_submit",
                Some(symbol.clone()),
                format!(
                    "stop order {} trigger={} kind={}",
                    symbol,
                    trigger_price,
                    req.trigger_kind.clone().unwrap_or_default()
                ),
                Some(snapshot.clone()),
                Some(order.clone()),
                None,
            ),
            core.archive_event(
                "snapshot",
                Some(symbol),
                "snapshot stop pending".to_string(),
                Some(snapshot.clone()),
                None,
                None,
            ),
        ];
        drop(core);

        self.log_json(
            "orders.jsonl",
            &serde_json::json!({
                "ts": now_ts(),
                "action": "stop_submit",
                "order": order,
            }),
        )?;
        self.log_json("balances.jsonl", &snapshot)?;
        self.log_archive_events(&events)?;

        Ok(OrderActionResult {
            order_id,
            status: "TriggerPending".to_string(),
            filled_qty: 0.0,
            filled_quote_qty: 0.0,
            remaining_qty: quantity.to_f64().unwrap_or(req.quantity),
            trade_count: 0,
        })
    }

    fn precision_for_symbol(&self, symbol: &str) -> SymbolPrecision {
        self.market_precisions
            .get(&symbol.to_ascii_uppercase())
            .copied()
            .unwrap_or_default()
    }

    fn ensure_price_precision(&self, symbol: &str, price: Decimal, field: &str) -> Result<()> {
        let precision = self.precision_for_symbol(symbol).price_precision;
        if precision > 0 {
            ensure!(
                price.round_dp(precision) == price,
                "{} exceeds {} price precision {}",
                field,
                symbol,
                precision
            );
        }
        Ok(())
    }

    fn ensure_quantity_precision(
        &self,
        symbol: &str,
        quantity: Decimal,
        field: &str,
    ) -> Result<()> {
        let precision = self.precision_for_symbol(symbol).quantity_precision;
        if precision > 0 {
            ensure!(
                quantity.round_dp(precision) == quantity,
                "{} exceeds {} quantity precision {}",
                field,
                symbol,
                precision
            );
        }
        Ok(())
    }
}

fn step_from_precision(precision: u32, fallback: Decimal) -> Decimal {
    if precision == 0 {
        fallback
    } else {
        Decimal::new(1, precision)
    }
}
