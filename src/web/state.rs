// src/web/state.rs
//
// DashboardState：后端维护的实时快照，可直接序列化为 JSON 推送给前端

use std::collections::{HashMap, VecDeque};
use std::sync::Arc;
use tokio::sync::RwLock;
use serde::{Deserialize, Serialize};
use chrono::{DateTime, Local};

// ── 单个币种快照（完全可序列化）────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct SymbolJson {
    pub symbol: String,

    // 价格
    pub bid: f64,
    pub ask: f64,
    pub mid: f64,
    pub spread_bps: f64,
    pub price_change_pct: f64,

    // 核心指标
    pub ofi: f64,
    pub ofi_raw: f64,
    pub obi: f64,
    pub trend_strength: f64,

    // 信号评分
    pub pump_score: u8,
    pub dump_score: u8,
    pub pump_signal: bool,
    pub dump_signal: bool,
    pub whale_entry: bool,
    pub whale_exit: bool,
    pub bid_eating: bool,

    // 深度
    pub total_bid_volume: f64,
    pub total_ask_volume: f64,
    pub max_bid_ratio: f64,
    pub max_ask_ratio: f64,

    // 顶部盘口
    pub top_bids: Vec<[f64; 2]>,  // [[price, qty], ...]
    pub top_asks: Vec<[f64; 2]>,

    // 异动
    pub anomaly_count_1m: u32,
    pub anomaly_max_severity: u8,

    // 综合分析
    pub sentiment: String,
    pub risk_level: String,
    pub recommendation: String,
    pub whale_type: String,
    pub pump_probability: u8,

    pub update_count: u64,
}

// ── 信号 Feed 条目 ────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FeedEntry {
    pub time:   String,
    pub symbol: String,
    /// "pump" | "dump" | "whale" | "anomaly"
    pub r#type: String,
    pub score:  Option<u8>,
    pub desc:   String,
}

// ── 全量快照（每次 WebSocket 推送的顶层结构）───────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FullSnapshot {
    pub symbols:       Vec<SymbolJson>,
    pub feed:          Vec<FeedEntry>,
    pub total_updates: u64,
    pub uptime_secs:   u64,
}

// ── DashboardState（由 bridge 写入，由 server 读取）─────────────

pub struct DashboardState {
    pub symbols:       HashMap<String, SymbolJson>,
    pub sorted_keys:   Vec<String>,
    pub feed:          VecDeque<FeedEntry>,     // 最新在前，最多 100 条
    pub total_updates: u64,
    pub start_time:    std::time::Instant,
}

impl DashboardState {
    pub fn new() -> Self {
        Self {
            symbols:       HashMap::new(),
            sorted_keys:   Vec::new(),
            feed:          VecDeque::with_capacity(100),
            total_updates: 0,
            start_time:    std::time::Instant::now(),
        }
    }

    pub fn upsert(&mut self, snap: SymbolJson) {
        let sym = snap.symbol.clone();
        self.symbols.insert(sym.clone(), snap);
        self.total_updates += 1;

        // 按 pump_score 降序排列
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
        if self.feed.len() >= 100 { self.feed.pop_back(); }
        self.feed.push_front(entry);
    }

    pub fn to_full_snapshot(&self) -> FullSnapshot {
        let symbols: Vec<SymbolJson> = self.sorted_keys.iter()
            .filter_map(|k| self.symbols.get(k).cloned())
            .collect();
        FullSnapshot {
            symbols,
            feed:          self.feed.iter().cloned().collect(),
            total_updates: self.total_updates,
            uptime_secs:   self.start_time.elapsed().as_secs(),
        }
    }
}

pub type SharedDashboardState = Arc<RwLock<DashboardState>>;

pub fn new_dashboard_state() -> SharedDashboardState {
    Arc::new(RwLock::new(DashboardState::new()))
}