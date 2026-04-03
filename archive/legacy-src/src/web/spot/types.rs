//! spot 模块的请求/响应 DTO 与内部辅助结构。
//!
//! 这一层把 HTTP 语义和撮合引擎内部状态隔开，避免 service/core 直接依赖 JSON 细节。

use std::collections::HashMap;

use serde::{Deserialize, Serialize};

use crate::engine::OrderId;
use crate::terminal::application::projection::{TraderOrderJson, TraderStateJson, TraderTradeJson};

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
pub(super) struct ArchiveEvent {
    pub ts: i64,
    pub seq: u64,
    pub kind: String,
    pub symbol: Option<String>,
    pub summary: String,
    pub snapshot: Option<TraderStateJson>,
    pub order: Option<TraderOrderJson>,
    pub trade: Option<TraderTradeJson>,
}

#[derive(Debug, Default)]
pub(super) struct SymbolLiquidity {
    pub bid_ids: Vec<OrderId>,
    pub ask_ids: Vec<OrderId>,
}

#[derive(Debug, Clone)]
pub(super) struct StopOrder {
    pub order_id: u64,
    pub request: ApiOrderRequest,
    pub created_at: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum ParsedOrderType {
    Limit,
    Market,
    StopLimit,
    StopMarket,
}

#[derive(Debug)]
pub(super) struct TradingState {
    pub assets: std::collections::BTreeSet<String>,
    pub open_orders: HashMap<OrderId, TraderOrderJson>,
    pub order_history: Vec<TraderOrderJson>,
    pub trade_history: Vec<TraderTradeJson>,
    pub liquidity: HashMap<String, SymbolLiquidity>,
    pub next_event_seq: u64,
    pub next_virtual_order_id: u64,
    pub stop_orders: HashMap<u64, StopOrder>,
}
