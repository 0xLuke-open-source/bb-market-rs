// src/web/state.rs
use std::collections::{HashMap, VecDeque};
use std::sync::Arc;
use tokio::sync::RwLock;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct KlineJson {
    pub interval: String, // "1m","5m","1h"...
    pub t:   u64,
    pub o:   f64,
    pub h:   f64,
    pub l:   f64,
    pub c:   f64,
    pub v:   f64,
    pub tbr: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct BigTradeJson {
    pub t:   u64,   // time_ms
    pub p:   f64,   // price
    pub q:   f64,   // qty
    pub buy: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct SymbolJson {
    pub symbol: String,
    pub status_summary: String,
    pub watch_level: String,
    pub signal_reason: String,

    // 价格
    pub bid: f64,
    pub ask: f64,
    pub mid: f64,
    pub spread_bps: f64,

    // 24h 真实 Ticker（来自 miniTicker）
    pub change_24h_pct:  f64,
    pub high_24h:        f64,
    pub low_24h:         f64,
    pub volume_24h:      f64,    // 基础资产成交量
    pub quote_vol_24h:   f64,    // USDT 成交额

    // 订单簿指标
    pub ofi:             f64,
    pub ofi_raw:         f64,
    pub obi:             f64,
    pub trend_strength:  f64,

    // 成交流（来自 aggTrade）
    pub cvd:             f64,    // 累计成交量差（正=买方主导）
    pub taker_buy_ratio: f64,    // 主动买入占比 % (0-100)

    // 信号评分
    pub pump_score:  u8,
    pub dump_score:  u8,
    pub pump_signal: bool,
    pub dump_signal: bool,
    pub whale_entry: bool,
    pub whale_exit:  bool,
    pub bid_eating:  bool,

    // 深度
    pub total_bid_volume: f64,
    pub total_ask_volume: f64,
    pub max_bid_ratio:    f64,
    pub max_ask_ratio:    f64,

    // 盘口
    pub top_bids: Vec<[f64; 2]>,
    pub top_asks: Vec<[f64; 2]>,

    // 异动
    pub anomaly_count_1m:    u32,
    pub anomaly_max_severity: u8,

    // 综合分析
    pub sentiment:      String,
    pub risk_level:     String,
    pub recommendation: String,
    pub whale_type:     String,
    pub pump_probability: u8,

    // 所有周期K线历史 interval -> Vec<bar>
    pub klines: std::collections::HashMap<String, Vec<KlineJson>>,
    // 各周期当前未收盘K线
    pub current_kline: std::collections::HashMap<String, KlineJson>,

    // 近期大单（最多10条）
    pub big_trades: Vec<BigTradeJson>,

    pub update_count: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FeedEntry {
    pub time:   String,
    pub symbol: String,
    pub r#type: String,
    pub score:  Option<u8>,
    pub desc:   String,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct TraderBalanceJson {
    pub asset: String,
    pub available: f64,
    pub locked: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct TraderOrderJson {
    pub time: String,
    pub order_id: u64,
    pub symbol: String,
    pub side: String,
    pub order_type: String,
    pub time_in_force: String,
    pub price: Option<f64>,
    pub trigger_price: Option<f64>,
    pub trigger_kind: Option<String>,
    pub quantity: f64,
    pub remaining_qty: f64,
    pub filled_qty: f64,
    pub filled_quote_qty: f64,
    pub status: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct TraderTradeJson {
    pub time: String,
    pub trade_id: u64,
    pub symbol: String,
    pub side: String,
    pub price: f64,
    pub quantity: f64,
    pub quote_quantity: f64,
    pub liquidity: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct TraderStateJson {
    pub account_id: u64,
    pub balances: Vec<TraderBalanceJson>,
    pub open_orders: Vec<TraderOrderJson>,
    pub order_history: Vec<TraderOrderJson>,
    pub trade_history: Vec<TraderTradeJson>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FullSnapshot {
    pub symbols:       Vec<SymbolJson>,
    pub feed:          Vec<FeedEntry>,
    pub total_updates: u64,
    pub uptime_secs:   u64,
    pub trader:        TraderStateJson,
}

pub struct DashboardState {
    pub symbols:       HashMap<String, SymbolJson>,
    pub sorted_keys:   Vec<String>,
    pub feed:          VecDeque<FeedEntry>,
    pub total_updates: u64,
    pub start_time:    std::time::Instant,
}

impl DashboardState {
    pub fn new() -> Self {
        Self {
            symbols:       HashMap::new(),
            sorted_keys:   Vec::new(),
            feed:          VecDeque::with_capacity(200),
            total_updates: 0,
            start_time:    std::time::Instant::now(),
        }
    }

    pub fn upsert(&mut self, snap: SymbolJson) {
        let sym = snap.symbol.clone();
        self.symbols.insert(sym.clone(), snap);
        self.total_updates += 1;
        self.sorted_keys = {
            let mut keys: Vec<String> = self.symbols.keys().cloned().collect();
            keys.sort_by(|a, b| {
                let sa = self.symbols[a].pump_score;
                let sb = self.symbols[b].pump_score;
                sb.cmp(&sa).then(a.cmp(b))
            });
            keys
        };
    }

    pub fn push_feed(&mut self, entry: FeedEntry) {
        if self.feed.len() >= 200 { self.feed.pop_back(); }
        self.feed.push_front(entry);
    }

    pub fn to_full_snapshot(&self, trader: TraderStateJson) -> FullSnapshot {
        let symbols = self.sorted_keys.iter()
            .filter_map(|k| self.symbols.get(k).cloned())
            .collect();
        FullSnapshot {
            symbols,
            feed: self.feed.iter().cloned().collect(),
            total_updates: self.total_updates,
            uptime_secs: self.start_time.elapsed().as_secs(),
            trader,
        }
    }

    pub fn to_light_snapshot(&self, trader: TraderStateJson) -> FullSnapshot {
        let symbols = self.sorted_keys.iter()
            .filter_map(|k| self.symbols.get(k).cloned())
            .map(|mut symbol| {
                symbol.klines.clear();
                symbol.current_kline.clear();
                symbol
            })
            .collect();
        FullSnapshot {
            symbols,
            feed: self.feed.iter().cloned().collect(),
            total_updates: self.total_updates,
            uptime_secs: self.start_time.elapsed().as_secs(),
            trader,
        }
    }

    pub fn get_symbol(&self, symbol: &str) -> Option<SymbolJson> {
        self.symbols.get(symbol).cloned()
    }
}

pub type SharedDashboardState = Arc<RwLock<DashboardState>>;

pub fn new_dashboard_state() -> SharedDashboardState {
    Arc::new(RwLock::new(DashboardState::new()))
}
