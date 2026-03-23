use std::collections::{BTreeSet, HashMap};
use std::fs::{self, OpenOptions};
use std::io::{BufRead, BufReader, Write};
use std::path::{Path, PathBuf};
use std::str::FromStr;
use std::sync::Arc;

use anyhow::{anyhow, Result};
use chrono::Local;
use rust_decimal::prelude::ToPrimitive;
use rust_decimal::Decimal;
use rust_decimal_macros::dec;
use serde::{Deserialize, Serialize};
use tokio::sync::Mutex;

use crate::engine::{
    NewOrderRequest, OrderId, OrderStatus, OrderType, SelfTradePrevention, Side, SpotMatchingEngine,
    SpotMarketConfig, SubmitOrderResult, TimeInForce, Trade,
};
use crate::web::state::{TraderBalanceJson, TraderOrderJson, TraderStateJson, TraderTradeJson};

const USER_ACCOUNT_ID: u64 = 900001;
const LIQUIDITY_ACCOUNT_ID: u64 = 999999;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ApiOrderRequest {
    pub symbol: String,
    pub side: String,
    pub order_type: String,
    pub time_in_force: Option<String>,
    pub price: Option<f64>,
    pub quantity: f64,
    pub trigger_price: Option<f64>,
    pub trigger_kind: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CancelAllRequest {
    pub symbol: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
pub struct ApiResponse<T> {
    pub ok: bool,
    pub message: String,
    pub data: T,
}

#[derive(Debug, Clone, Serialize, Default)]
pub struct OrderActionResult {
    pub order_id: u64,
    pub status: String,
    pub filled_qty: f64,
    pub filled_quote_qty: f64,
    pub remaining_qty: f64,
    pub trade_count: usize,
}

#[derive(Debug, Clone, Serialize, Default)]
pub struct CancelAllResult {
    pub cancelled: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct ReplayQuery {
    pub at_ts: Option<i64>,
    pub from_ts: Option<i64>,
    pub limit: Option<usize>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct ReplayEventJson {
    pub ts: i64,
    pub seq: u64,
    pub kind: String,
    pub symbol: Option<String>,
    pub summary: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct ReplayResponse {
    pub snapshot: TraderStateJson,
    pub events: Vec<ReplayEventJson>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct ArchiveEvent {
    ts: i64,
    seq: u64,
    kind: String,
    symbol: Option<String>,
    summary: String,
    snapshot: Option<TraderStateJson>,
    order: Option<TraderOrderJson>,
    trade: Option<TraderTradeJson>,
}

#[derive(Debug, Default)]
struct SymbolLiquidity {
    bid_ids: Vec<OrderId>,
    ask_ids: Vec<OrderId>,
}

#[derive(Debug, Clone)]
struct StopOrder {
    order_id: u64,
    request: ApiOrderRequest,
    created_at: String,
}

#[derive(Debug)]
struct TradingCore {
    engine: SpotMatchingEngine,
    assets: BTreeSet<String>,
    open_orders: HashMap<OrderId, TraderOrderJson>,
    order_history: Vec<TraderOrderJson>,
    trade_history: Vec<TraderTradeJson>,
    liquidity: HashMap<String, SymbolLiquidity>,
    next_event_seq: u64,
    next_virtual_order_id: u64,
    stop_orders: HashMap<u64, StopOrder>,
}

#[derive(Clone)]
pub struct SpotTradingService {
    inner: Arc<Mutex<TradingCore>>,
    log_dir: Arc<PathBuf>,
}

impl SpotTradingService {
    pub fn new(symbols: &[String], log_dir: impl AsRef<Path>) -> Result<Self> {
        let log_dir = log_dir.as_ref().to_path_buf();
        fs::create_dir_all(&log_dir)?;

        let mut engine = SpotMatchingEngine::new();
        let mut assets = BTreeSet::new();
        assets.insert("USDT".to_string());

        engine.deposit(USER_ACCOUNT_ID, "USDT", dec!(1000000))?;
        engine.deposit(LIQUIDITY_ACCOUNT_ID, "USDT", dec!(1000000000))?;

        for symbol in symbols {
            let base_asset = parse_base_asset(symbol)?;
            assets.insert(base_asset.clone());

            engine.create_market(SpotMarketConfig {
                symbol: symbol.clone(),
                base_asset: base_asset.clone(),
                quote_asset: "USDT".to_string(),
                tick_size: dec!(0.00000001),
                lot_size: dec!(0.00000001),
                min_qty: dec!(0.00000001),
                min_notional: dec!(0),
                maker_fee_rate: dec!(0.001),
                taker_fee_rate: dec!(0.001),
            })?;

            engine.deposit(USER_ACCOUNT_ID, &base_asset, dec!(10000))?;
            engine.deposit(LIQUIDITY_ACCOUNT_ID, &base_asset, dec!(1000000000))?;
        }

        Ok(Self {
            inner: Arc::new(Mutex::new(TradingCore {
                engine,
                assets,
                open_orders: HashMap::new(),
                order_history: Vec::new(),
                trade_history: Vec::new(),
                liquidity: HashMap::new(),
                next_event_seq: 0,
                next_virtual_order_id: 10_000_000_000,
                stop_orders: HashMap::new(),
            })),
            log_dir: Arc::new(log_dir),
        })
    }

    pub async fn snapshot(&self) -> TraderStateJson {
        let core = self.inner.lock().await;
        core.snapshot()
    }

    pub async fn replay(&self, query: ReplayQuery) -> Result<ReplayResponse> {
        let mut events = self.load_archive_events()?;
        events.sort_by(|a, b| a.ts.cmp(&b.ts).then(a.seq.cmp(&b.seq)));

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

        Ok(ReplayResponse { snapshot, events: filtered })
    }

    pub async fn submit_order(&self, req: ApiOrderRequest) -> Result<OrderActionResult> {
        let symbol = req.symbol.to_uppercase();
        let side = parse_side(&req.side)?;
        let order_type = parse_order_type(&req.order_type)?;
        if matches!(order_type, ParsedOrderType::StopLimit | ParsedOrderType::StopMarket) {
            return self.submit_stop_order(req).await;
        }
        let engine_order_type = match order_type {
            ParsedOrderType::Limit => OrderType::Limit,
            ParsedOrderType::Market => OrderType::Market,
            ParsedOrderType::StopLimit | ParsedOrderType::StopMarket => unreachable!(),
        };
        let time_in_force = parse_tif(req.time_in_force.as_deref(), engine_order_type)?;
        let quantity = decimal_from_f64(req.quantity, "quantity")?;
        let price = req.price.map(|v| decimal_from_f64(v, "price")).transpose()?;

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
                format!("submit {:?} {:?} {} qty={}", side, order_type, symbol, req.quantity),
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
                .open_orders
                .get(&order_id)
                .cloned()
                .ok_or_else(|| anyhow!("order not found: {}", order_id))?;

            if let Some(stop_order) = core.stop_orders.remove(&order_id) {
                let mut final_order = order.clone();
                final_order.status = "Cancelled".to_string();
                final_order.remaining_qty = 0.0;
                core.open_orders.remove(&order_id);
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
                let _ = stop_order;
                return {
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
                    Ok(OrderActionResult {
                        order_id,
                        status: "Cancelled".to_string(),
                        filled_qty: order.filled_qty,
                        filled_quote_qty: order.filled_quote_qty,
                        remaining_qty: 0.0,
                        trade_count: 0,
                    })
                };
            }

            let res = match core.engine.cancel_order(&order.symbol, order_id) {
                Ok(res) => res,
                Err(_) => {
                    let mut final_order = order.clone();
                    final_order.status = "Cancelled".to_string();
                    final_order.remaining_qty = 0.0;
                    core.open_orders.remove(&order_id);
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
                    return {
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
                        Ok(OrderActionResult {
                            order_id,
                            status: "Cancelled".to_string(),
                            filled_qty: order.filled_qty,
                            filled_quote_qty: order.filled_quote_qty,
                            remaining_qty: 0.0,
                            trade_count: 0,
                        })
                    };
                }
            };
            let mut final_order = order.clone();
            final_order.status = format!("{:?}", res.status);
            final_order.remaining_qty = 0.0;
            core.open_orders.remove(&order_id);
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
        let symbol = req.symbol.map(|s| s.to_uppercase());
        let (cancelled, trader_snapshot, archive_events) = {
            let mut core = self.inner.lock().await;
            let ids: Vec<u64> = core
                .open_orders
                .values()
                .filter(|order| symbol.as_ref().map(|s| &order.symbol == s).unwrap_or(true))
                .map(|order| order.order_id)
                .collect();

            for order_id in &ids {
                if let Some(order) = core.open_orders.get(order_id).cloned() {
                    if core.stop_orders.remove(order_id).is_none() {
                        let _ = core.engine.cancel_order(&order.symbol, *order_id);
                    }
                    let mut final_order = order.clone();
                    final_order.status = "Cancelled".to_string();
                    final_order.remaining_qty = 0.0;
                    core.open_orders.remove(order_id);
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

    pub async fn sync_liquidity(&self, symbol: &str, bids: &[(Decimal, Decimal)], asks: &[(Decimal, Decimal)]) -> Result<()> {
        let symbol = symbol.to_uppercase();
        let mut core = self.inner.lock().await;
        let mut entry = core.liquidity.remove(&symbol).unwrap_or_default();
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
            let res = core.engine.submit_order(NewOrderRequest {
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
            entry.bid_ids.push(res.order_id);
            trade_logs.extend(core.capture_user_trades(&symbol, &res.trades)?);
        }

        for (price, qty) in asks.iter().take(10) {
            if *price <= Decimal::ZERO || *qty <= Decimal::ZERO {
                continue;
            }
            let res = core.engine.submit_order(NewOrderRequest {
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
            entry.ask_ids.push(res.order_id);
            trade_logs.extend(core.capture_user_trades(&symbol, &res.trades)?);
        }

        core.liquidity.insert(symbol.clone(), entry);
        let mid = match (bids.first(), asks.first()) {
            (Some((bp, _)), Some((ap, _))) => Some((*bp + *ap) / dec!(2)),
            _ => None,
        };
        let stop_trigger_results = if let Some(mid) = mid {
            core.trigger_stop_orders(&symbol, mid)?
        } else {
            Vec::new()
        };
        let snapshot = if trade_logs.is_empty() { None } else { Some(core.snapshot()) };
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
        let trigger_price = decimal_from_f64(
            req.trigger_price.ok_or_else(|| anyhow!("trigger_price is required"))?,
            "trigger_price",
        )?;
        let symbol = req.symbol.to_uppercase();
        let mut core = self.inner.lock().await;
        let order_id = core.next_virtual_order_id;
        core.next_virtual_order_id += 1;

        let order = TraderOrderJson {
            time: now_text(),
            order_id,
            symbol: symbol.clone(),
            side: req.side.to_ascii_uppercase(),
            order_type: req.order_type.to_ascii_uppercase(),
            time_in_force: req.time_in_force.clone().unwrap_or_else(|| "gtc".to_string()).to_ascii_uppercase(),
            price: req.price,
            trigger_price: Some(trigger_price.to_f64().unwrap_or(0.0)),
            trigger_kind: Some(req.trigger_kind.clone().unwrap_or_else(|| "stop_loss".to_string())),
            quantity: req.quantity,
            remaining_qty: req.quantity,
            filled_qty: 0.0,
            filled_quote_qty: 0.0,
            status: "TriggerPending".to_string(),
        };
        core.open_orders.insert(order_id, order.clone());
        core.stop_orders.insert(order_id, StopOrder {
            order_id,
            request: req.clone(),
            created_at: order.time.clone(),
        });
        let snapshot = core.snapshot();
        let events = vec![
            core.archive_event(
                "stop_submit",
                Some(symbol.clone()),
                format!("stop order {} trigger={} kind={}", symbol, trigger_price, req.trigger_kind.clone().unwrap_or_default()),
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
            remaining_qty: req.quantity,
            trade_count: 0,
        })
    }

    fn log_json<T: Serialize>(&self, file_name: &str, value: &T) -> Result<()> {
        let path = self.log_dir.join(file_name);
        let mut file = OpenOptions::new().create(true).append(true).open(path)?;
        serde_json::to_writer(&mut file, value)?;
        file.write_all(b"\n")?;
        Ok(())
    }

    fn log_archive_events(&self, events: &[ArchiveEvent]) -> Result<()> {
        for event in events {
            let day = Local::now().format("%Y-%m-%d").to_string();
            let dir = self.log_dir.join("archive").join(day);
            fs::create_dir_all(&dir)?;
            let path = dir.join("events.jsonl");
            let mut file = OpenOptions::new().create(true).append(true).open(path)?;
            serde_json::to_writer(&mut file, event)?;
            file.write_all(b"\n")?;
        }
        Ok(())
    }

    fn load_archive_events(&self) -> Result<Vec<ArchiveEvent>> {
        let archive_root = self.log_dir.join("archive");
        if !archive_root.exists() {
            return Ok(Vec::new());
        }
        let mut events = Vec::new();
        for day_entry in fs::read_dir(archive_root)? {
            let day_entry = day_entry?;
            if !day_entry.file_type()?.is_dir() {
                continue;
            }
            let file_path = day_entry.path().join("events.jsonl");
            if !file_path.exists() {
                continue;
            }
            let file = OpenOptions::new().read(true).open(file_path)?;
            let reader = BufReader::new(file);
            for line in reader.lines() {
                let line = line?;
                if line.trim().is_empty() {
                    continue;
                }
                if let Ok(event) = serde_json::from_str::<ArchiveEvent>(&line) {
                    events.push(event);
                }
            }
        }
        Ok(events)
    }
}

impl TradingCore {
    fn snapshot(&self) -> TraderStateJson {
        let mut balances: Vec<TraderBalanceJson> = self
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
            .filter(|b| b.available > 0.0 || b.locked > 0.0)
            .collect();
        balances.sort_by(|a, b| a.asset.cmp(&b.asset));

        let mut open_orders: Vec<TraderOrderJson> = self.open_orders.values().cloned().collect();
        open_orders.sort_by(|a, b| b.order_id.cmp(&a.order_id));

        TraderStateJson {
            account_id: USER_ACCOUNT_ID,
            balances,
            open_orders,
            order_history: self.order_history.clone(),
            trade_history: self.trade_history.clone(),
        }
    }

    fn apply_user_submit(
        &mut self,
        symbol: &str,
        req: &NewOrderRequest,
        result: &SubmitOrderResult,
    ) -> Result<Vec<TraderTradeJson>> {
        let trade_logs = self.capture_user_trades(symbol, &result.trades)?;

        match result.status {
            OrderStatus::New | OrderStatus::PartiallyFilled => {
                let view = self.engine.get_order(symbol, result.order_id)?;
                self.open_orders.insert(view.order_id, trader_order_from_view(&view));
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

    fn capture_user_trades(&mut self, symbol: &str, trades: &[Trade]) -> Result<Vec<TraderTradeJson>> {
        let mut captured = Vec::new();
        for trade in trades {
            if trade.taker_account_id == USER_ACCOUNT_ID || trade.maker_account_id == USER_ACCOUNT_ID {
                let trade_json = trader_trade_from_trade(trade);
                self.trade_history.insert(0, trade_json.clone());
                if self.trade_history.len() > 200 {
                    self.trade_history.truncate(200);
                }
                captured.push(trade_json);

                if trade.maker_account_id == USER_ACCOUNT_ID {
                    self.refresh_order_after_trade(symbol, trade.maker_order_id, trade)?;
                }
            }
        }
        Ok(captured)
    }

    fn refresh_order_after_trade(&mut self, symbol: &str, order_id: u64, trade: &Trade) -> Result<()> {
        if let Ok(view) = self.engine.get_order(symbol, order_id) {
            self.open_orders.insert(order_id, trader_order_from_view(&view));
            return Ok(());
        }

        if let Some(mut order) = self.open_orders.remove(&order_id) {
            order.filled_qty += trade.quantity.to_f64().unwrap_or(0.0);
            order.filled_quote_qty += trade.quote_quantity.to_f64().unwrap_or(0.0);
            order.remaining_qty = 0.0;
            order.status = "Filled".to_string();
            self.push_order_history(order);
        }
        Ok(())
    }

    fn push_order_history(&mut self, order: TraderOrderJson) {
        self.order_history.insert(0, order);
        if self.order_history.len() > 200 {
            self.order_history.truncate(200);
        }
    }

    fn trigger_stop_orders(&mut self, symbol: &str, mid: Decimal) -> Result<Vec<OrderActionResult>> {
        let ids: Vec<u64> = self
            .stop_orders
            .values()
            .filter(|order| order.request.symbol.eq_ignore_ascii_case(symbol))
            .filter(|order| should_trigger_stop(&order.request, mid))
            .map(|order| order.order_id)
            .collect();

        let mut results = Vec::new();
        for id in ids {
            if let Some(stop_order) = self.stop_orders.remove(&id) {
                self.open_orders.remove(&id);
                let side = parse_side(&stop_order.request.side)?;
                let parsed = parse_order_type(&stop_order.request.order_type)?;
                let engine_order_type = match parsed {
                    ParsedOrderType::StopLimit => OrderType::Limit,
                    ParsedOrderType::StopMarket => OrderType::Market,
                    _ => continue,
                };
                let quantity = decimal_from_f64(stop_order.request.quantity, "quantity")?;
                let price = stop_order.request.price.map(|v| decimal_from_f64(v, "price")).transpose()?;
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
                        time_in_force: parse_tif(stop_order.request.time_in_force.as_deref(), engine_order_type)?,
                        price,
                        quantity: Some(quantity),
                        quote_quantity: None,
                        client_order_id: Some(format!("stop-trigger-{}", id)),
                        self_trade_prevention: SelfTradePrevention::CancelNewest,
                    },
                };
                let result = self.engine.submit_order(engine_req.clone())?;
                let _ = self.apply_user_submit(symbol, &engine_req, &result)?;
                let mut hist = TraderOrderJson {
                    time: stop_order.created_at.clone(),
                    order_id: id,
                    symbol: stop_order.request.symbol.to_uppercase(),
                    side: stop_order.request.side.to_ascii_uppercase(),
                    order_type: stop_order.request.order_type.to_ascii_uppercase(),
                    time_in_force: stop_order.request.time_in_force.clone().unwrap_or_else(|| "gtc".to_string()).to_ascii_uppercase(),
                    price: stop_order.request.price,
                    trigger_price: stop_order.request.trigger_price,
                    trigger_kind: stop_order.request.trigger_kind.clone(),
                    quantity: stop_order.request.quantity,
                    remaining_qty: 0.0,
                    filled_qty: result.filled_qty.to_f64().unwrap_or(0.0),
                    filled_quote_qty: result.filled_quote_qty.to_f64().unwrap_or(0.0),
                    status: format!("Triggered->{}", result.status_string()),
                };
                if hist.filled_qty == 0.0 && hist.status == "Triggered->New" {
                    hist.status = "Triggered".to_string();
                }
                self.push_order_history(hist);
                results.push(summarize_result(&result));
            }
        }
        Ok(results)
    }

    fn archive_event(
        &mut self,
        kind: &str,
        symbol: Option<String>,
        summary: String,
        snapshot: Option<TraderStateJson>,
        order: Option<TraderOrderJson>,
        trade: Option<TraderTradeJson>,
    ) -> ArchiveEvent {
        self.next_event_seq += 1;
        ArchiveEvent {
            ts: now_ts(),
            seq: self.next_event_seq,
            kind: kind.to_string(),
            symbol,
            summary,
            snapshot,
            order,
            trade,
        }
    }
}

fn parse_base_asset(symbol: &str) -> Result<String> {
    symbol
        .strip_suffix("USDT")
        .map(|base| base.to_string())
        .ok_or_else(|| anyhow!("unsupported symbol: {}", symbol))
}

fn parse_side(input: &str) -> Result<Side> {
    match input.to_ascii_lowercase().as_str() {
        "buy" => Ok(Side::Buy),
        "sell" => Ok(Side::Sell),
        _ => Err(anyhow!("invalid side: {}", input)),
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum ParsedOrderType {
    Limit,
    Market,
    StopLimit,
    StopMarket,
}

fn parse_order_type(input: &str) -> Result<ParsedOrderType> {
    match input.to_ascii_lowercase().as_str() {
        "limit" => Ok(ParsedOrderType::Limit),
        "market" => Ok(ParsedOrderType::Market),
        "stop_limit" => Ok(ParsedOrderType::StopLimit),
        "stop_market" => Ok(ParsedOrderType::StopMarket),
        _ => Err(anyhow!("invalid order_type: {}", input)),
    }
}

fn parse_tif(input: Option<&str>, order_type: OrderType) -> Result<TimeInForce> {
    match input.unwrap_or(match order_type {
        OrderType::Limit => "gtc",
        OrderType::Market => "ioc",
    }).to_ascii_lowercase().as_str() {
        "gtc" => Ok(TimeInForce::Gtc),
        "ioc" => Ok(TimeInForce::Ioc),
        "fok" => Ok(TimeInForce::Fok),
        "post_only" | "postonly" => Ok(TimeInForce::PostOnly),
        other => Err(anyhow!("invalid time_in_force: {}", other)),
    }
}

fn decimal_from_f64(value: f64, field: &str) -> Result<Decimal> {
    Decimal::from_str(&value.to_string()).map_err(|_| anyhow!("invalid {}: {}", field, value))
}

fn now_text() -> String {
    Local::now().format("%H:%M:%S").to_string()
}

fn now_ts() -> i64 {
    Local::now().timestamp_millis()
}

fn trader_order_from_view(view: &crate::engine::OrderView) -> TraderOrderJson {
    TraderOrderJson {
        time: now_text(),
        order_id: view.order_id,
        symbol: view.symbol.clone(),
        side: format!("{:?}", view.side),
        order_type: format!("{:?}", view.order_type),
        time_in_force: format!("{:?}", view.time_in_force),
        price: view.price.and_then(|v| v.to_f64()),
        trigger_price: None,
        trigger_kind: None,
        quantity: view.original_qty.to_f64().unwrap_or(0.0),
        remaining_qty: view.remaining_qty.to_f64().unwrap_or(0.0),
        filled_qty: view.filled_qty.to_f64().unwrap_or(0.0),
        filled_quote_qty: view.filled_quote_qty.to_f64().unwrap_or(0.0),
        status: format!("{:?}", view.status),
    }
}

fn order_from_submit(req: &NewOrderRequest, result: &SubmitOrderResult) -> TraderOrderJson {
    TraderOrderJson {
        time: now_text(),
        order_id: result.order_id,
        symbol: req.symbol.clone(),
        side: format!("{:?}", req.side),
        order_type: format!("{:?}", req.order_type),
        time_in_force: format!("{:?}", req.time_in_force),
        price: req.price.and_then(|v| v.to_f64()),
        trigger_price: None,
        trigger_kind: None,
        quantity: req
            .quantity
            .or(req.quote_quantity)
            .and_then(|v| v.to_f64())
            .unwrap_or(0.0),
        remaining_qty: result.remaining_qty.to_f64().unwrap_or(0.0),
        filled_qty: result.filled_qty.to_f64().unwrap_or(0.0),
        filled_quote_qty: result.filled_quote_qty.to_f64().unwrap_or(0.0),
        status: format!("{:?}", result.status),
    }
}

fn trader_trade_from_trade(trade: &Trade) -> TraderTradeJson {
    let user_is_taker = trade.taker_account_id == USER_ACCOUNT_ID;
    let side = if user_is_taker {
        format!("{:?}", trade.taker_side)
    } else {
        format!("{:?}", opposite_side(trade.taker_side))
    };

    TraderTradeJson {
        time: now_text(),
        trade_id: trade.trade_id,
        symbol: trade.symbol.clone(),
        side,
        price: trade.price.to_f64().unwrap_or(0.0),
        quantity: trade.quantity.to_f64().unwrap_or(0.0),
        quote_quantity: trade.quote_quantity.to_f64().unwrap_or(0.0),
        liquidity: if user_is_taker { "Taker" } else { "Maker" }.to_string(),
    }
}

fn opposite_side(side: Side) -> Side {
    match side {
        Side::Buy => Side::Sell,
        Side::Sell => Side::Buy,
    }
}

fn summarize_result(result: &SubmitOrderResult) -> OrderActionResult {
    OrderActionResult {
        order_id: result.order_id,
        status: format!("{:?}", result.status),
        filled_qty: result.filled_qty.to_f64().unwrap_or(0.0),
        filled_quote_qty: result.filled_quote_qty.to_f64().unwrap_or(0.0),
        remaining_qty: result.remaining_qty.to_f64().unwrap_or(0.0),
        trade_count: result.trades.len(),
    }
}

fn should_trigger_stop(req: &ApiOrderRequest, mid: Decimal) -> bool {
    let trigger = match req.trigger_price.and_then(|v| Decimal::from_str(&v.to_string()).ok()) {
        Some(v) => v,
        None => return false,
    };
    let kind = req.trigger_kind.clone().unwrap_or_else(|| "stop_loss".to_string()).to_ascii_lowercase();
    let side = req.side.to_ascii_lowercase();
    match (side.as_str(), kind.as_str()) {
        ("buy", "stop_loss") => mid >= trigger,
        ("buy", "take_profit") => mid <= trigger,
        ("sell", "stop_loss") => mid <= trigger,
        ("sell", "take_profit") => mid >= trigger,
        ("buy", _) => mid >= trigger,
        ("sell", _) => mid <= trigger,
        _ => false,
    }
}

trait SubmitOrderResultExt {
    fn status_string(&self) -> String;
}

impl SubmitOrderResultExt for SubmitOrderResult {
    fn status_string(&self) -> String {
        format!("{:?}", self.status)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn test_log_dir(name: &str) -> PathBuf {
        let mut path = std::env::temp_dir();
        path.push(format!("bb_market_spot_test_{}_{}", name, std::process::id()));
        let _ = fs::remove_dir_all(&path);
        path
    }

    #[tokio::test]
    async fn submit_order_updates_snapshot() {
        let service = SpotTradingService::new(&["BTCUSDT".to_string()], test_log_dir("submit")).unwrap();
        service
            .sync_liquidity(
                "BTCUSDT",
                &[(dec!(63990), dec!(1.0))],
                &[(dec!(64000), dec!(1.0))],
            )
            .await
            .unwrap();

        let result = service
            .submit_order(ApiOrderRequest {
                symbol: "BTCUSDT".to_string(),
                side: "buy".to_string(),
                order_type: "limit".to_string(),
                time_in_force: Some("ioc".to_string()),
                price: Some(64000.0),
                quantity: 1.0,
                trigger_price: None,
                trigger_kind: None,
            })
            .await
            .unwrap();

        assert_eq!(result.status, "Filled");
        let snapshot = service.snapshot().await;
        assert!(!snapshot.trade_history.is_empty());
        assert!(snapshot
            .balances
            .iter()
            .any(|b| b.asset == "BTC" && b.available > 10000.0));
    }

    #[tokio::test]
    async fn cancel_all_clears_open_orders() {
        let service = SpotTradingService::new(&["BTCUSDT".to_string()], test_log_dir("cancel")).unwrap();
        service
            .submit_order(ApiOrderRequest {
                symbol: "BTCUSDT".to_string(),
                side: "buy".to_string(),
                order_type: "limit".to_string(),
                time_in_force: Some("gtc".to_string()),
                price: Some(100.0),
                quantity: 1.0,
                trigger_price: None,
                trigger_kind: None,
            })
            .await
            .unwrap();

        let snapshot = service.snapshot().await;
        assert_eq!(snapshot.open_orders.len(), 1);

        let res = service
            .cancel_all(CancelAllRequest {
                symbol: Some("BTCUSDT".to_string()),
            })
            .await
            .unwrap();
        assert_eq!(res.cancelled, 1);

        let snapshot = service.snapshot().await;
        assert!(snapshot.open_orders.is_empty());
        assert!(!snapshot.order_history.is_empty());
    }
}
