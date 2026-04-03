use crate::identity::application::AuthStatusView;
use crate::instrument_catalog::application::SymbolRegistryService;
use crate::terminal::application::http_access::{build_access_info, visible_keys_for_access};
use crate::terminal::application::projection::{
    AccessInfoJson, DashboardState, FeedEntry, SymbolJson, TraderStateJson,
};
use serde::Serialize;

const WS_FEED_LIMIT: usize = 32;
const WS_TRADER_OPEN_ORDERS_LIMIT: usize = 64;
const WS_TRADER_HISTORY_LIMIT: usize = 40;

#[derive(Debug, Serialize)]
pub struct CompactWsSnapshot {
    #[serde(rename = "k")]
    pub kind: &'static str,
    #[serde(rename = "u")]
    pub total_updates: u64,
    #[serde(rename = "up")]
    pub uptime_secs: u64,
    #[serde(rename = "a")]
    pub access: AccessInfoJson,
    #[serde(rename = "t")]
    pub trader: TraderStateJson,
    #[serde(rename = "f")]
    pub feed: Vec<CompactFeedRow>,
    #[serde(rename = "s")]
    pub symbols: Vec<CompactSymbolRow>,
}

#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct CompactFeedRow(
    pub String,
    pub String,
    pub String,
    pub Option<u8>,
    pub String,
    pub i64,
);

#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct CompactSymbolRow(
    pub String,
    pub String,
    pub String,
    pub String,
    pub f64,
    pub f64,
    pub f64,
    pub f64,
    pub u32,
    pub u32,
    pub f64,
    pub f64,
    pub f64,
    pub f64,
    pub f64,
    pub f64,
    pub f64,
    pub f64,
    pub f64,
    pub f64,
    pub u8,
    pub u8,
    pub bool,
    pub bool,
    pub bool,
    pub bool,
    pub bool,
    pub f64,
    pub f64,
    pub f64,
    pub f64,
    pub u32,
    pub u8,
    pub u64,
);

#[derive(Debug, Serialize)]
pub struct CompactWsDelta {
    #[serde(rename = "k")]
    pub kind: &'static str,
    #[serde(rename = "u")]
    pub total_updates: u64,
    #[serde(rename = "up")]
    pub uptime_secs: u64,
    #[serde(rename = "a", skip_serializing_if = "Option::is_none")]
    pub access: Option<AccessInfoJson>,
    #[serde(rename = "t", skip_serializing_if = "Option::is_none")]
    pub trader: Option<TraderStateJson>,
    #[serde(rename = "f", skip_serializing_if = "Vec::is_empty")]
    pub feed: Vec<CompactFeedRow>,
    #[serde(rename = "s", skip_serializing_if = "Vec::is_empty")]
    pub symbols: Vec<CompactSymbolRow>,
    #[serde(rename = "rm", skip_serializing_if = "Vec::is_empty")]
    pub removed_symbols: Vec<String>,
}

pub fn compact_feed_row_key(row: &CompactFeedRow) -> String {
    format!("{}|{}|{}|{}|{}", row.5, row.1, row.2, row.0, row.4)
}

pub async fn build_compact_ws_snapshot(
    symbol_registry: &SymbolRegistryService,
    dashboard: &DashboardState,
    trader: TraderStateJson,
    access: &AuthStatusView,
) -> CompactWsSnapshot {
    let total_symbols = dashboard.sorted_keys.len();
    let visible_keys = visible_keys_for_access(symbol_registry, dashboard, access).await;
    let visible_set = visible_keys
        .iter()
        .map(String::as_str)
        .collect::<std::collections::HashSet<&str>>();
    let symbols = visible_keys
        .iter()
        .filter_map(|key| dashboard.symbols.get(key))
        .map(CompactSymbolRow::from)
        .collect();
    let feed = dashboard
        .feed
        .iter()
        .filter(|entry| visible_set.contains(entry.symbol.as_str()))
        .take(WS_FEED_LIMIT)
        .map(CompactFeedRow::from)
        .collect();

    CompactWsSnapshot {
        kind: "m1",
        total_updates: dashboard.total_updates,
        uptime_secs: dashboard.start_time.elapsed().as_secs(),
        access: build_access_info(access, visible_keys.len(), total_symbols),
        trader: compact_trader_for_ws(trader, access.authenticated),
        feed,
        symbols,
    }
}

pub fn encode_ws_binary<T: Serialize>(value: &T) -> Option<Vec<u8>> {
    rmp_serde::to_vec_named(value).ok()
}

fn compact_trader_for_ws(mut trader: TraderStateJson, authenticated: bool) -> TraderStateJson {
    if !authenticated {
        return TraderStateJson::default();
    }
    if trader.open_orders.len() > WS_TRADER_OPEN_ORDERS_LIMIT {
        trader.open_orders.truncate(WS_TRADER_OPEN_ORDERS_LIMIT);
    }
    if trader.order_history.len() > WS_TRADER_HISTORY_LIMIT {
        trader.order_history.truncate(WS_TRADER_HISTORY_LIMIT);
    }
    if trader.trade_history.len() > WS_TRADER_HISTORY_LIMIT {
        trader.trade_history.truncate(WS_TRADER_HISTORY_LIMIT);
    }
    trader
}

impl From<&FeedEntry> for CompactFeedRow {
    fn from(entry: &FeedEntry) -> Self {
        Self(
            entry.time.clone(),
            entry.symbol.clone(),
            entry.r#type.clone(),
            entry.score,
            entry.desc.clone(),
            entry.ts,
        )
    }
}

impl From<&SymbolJson> for CompactSymbolRow {
    fn from(symbol: &SymbolJson) -> Self {
        Self(
            symbol.symbol.clone(),
            symbol.status_summary.clone(),
            symbol.watch_level.clone(),
            symbol.signal_reason.clone(),
            symbol.bid,
            symbol.ask,
            symbol.mid,
            symbol.spread_bps,
            symbol.price_precision,
            symbol.quantity_precision,
            symbol.change_24h_pct,
            symbol.high_24h,
            symbol.low_24h,
            symbol.volume_24h,
            symbol.quote_vol_24h,
            symbol.ofi,
            symbol.ofi_raw,
            symbol.obi,
            symbol.cvd,
            symbol.taker_buy_ratio,
            symbol.pump_score,
            symbol.dump_score,
            symbol.pump_signal,
            symbol.dump_signal,
            symbol.whale_entry,
            symbol.whale_exit,
            symbol.bid_eating,
            symbol.total_bid_volume,
            symbol.total_ask_volume,
            symbol.max_bid_ratio,
            symbol.max_ask_ratio,
            symbol.anomaly_count_1m,
            symbol.anomaly_max_severity,
            symbol.update_count,
        )
    }
}
