// src/terminal/application/projection.rs
//
// 这一层定义的是“前端展示模型”，而不是算法内部模型。
// 换句话说，这里的结构体是为了让 Dashboard 和 API 更容易消费，
// 所以字段会偏平坦、偏 JSON 化。
use serde::{Deserialize, Serialize};
use std::collections::{HashMap, VecDeque};
use std::sync::Arc;
use tokio::sync::RwLock;

#[allow(unused_imports)]
pub use crate::execution::application::spot::{
    TraderBalanceJson, TraderOrderJson, TraderStateJson, TraderTradeJson,
};
#[allow(unused_imports)]
pub use crate::market_data::application::BigTradeJson;
use crate::signal_intelligence::domain::strategy_engine::StrategyProfile;

const LIVE_RECENT_TRADES_LIMIT: usize = 60;

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct KlineJson {
    pub interval: String, // "1m","5m","1h"...
    pub t: u64,
    pub o: f64,
    pub h: f64,
    pub l: f64,
    pub c: f64,
    pub v: f64,
    pub tbr: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct FactorMetricJson {
    pub name: String,
    pub value: String,
    pub score: f64,
    pub tip: String,
    pub tone: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct EnterpriseMetricRowJson {
    pub name: String,
    pub score: f64,
    pub value: String,
    pub tip: String,
    pub invert: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct EnterpriseMetricSectionJson {
    pub title: String,
    pub subtitle: String,
    pub items: Vec<EnterpriseMetricRowJson>,
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
    #[serde(default)]
    pub price_precision: u32,
    #[serde(default)]
    pub quantity_precision: u32,

    // 24h 真实 Ticker（来自 miniTicker）
    pub change_24h_pct: f64,
    pub high_24h: f64,
    pub low_24h: f64,
    pub volume_24h: f64,    // 基础资产成交量
    pub quote_vol_24h: f64, // USDT 成交额

    // 订单簿指标
    pub ofi: f64,
    pub ofi_raw: f64,
    pub obi: f64,
    pub trend_strength: f64,

    // 成交流（来自 aggTrade）
    pub cvd: f64,             // 累计成交量差（正=买方主导）
    pub taker_buy_ratio: f64, // 主动买入占比 % (0-100)

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

    // 盘口
    pub top_bids: Vec<[f64; 2]>,
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
    #[serde(default)]
    pub strategy_profile: StrategyProfile,

    // 所有周期K线历史 interval -> Vec<bar>
    pub klines: std::collections::HashMap<String, Vec<KlineJson>>,
    // 各周期当前未收盘K线
    pub current_kline: std::collections::HashMap<String, KlineJson>,

    // 近期大单（最多10条）
    pub big_trades: Vec<BigTradeJson>,

    // 最近成交（30分钟窗口）
    pub recent_trades: Vec<BigTradeJson>,

    #[serde(default)]
    pub signal_history: Vec<FeedEntry>,
    #[serde(default)]
    pub factor_metrics: Vec<FactorMetricJson>,
    #[serde(default)]
    pub enterprise_metrics: Vec<EnterpriseMetricSectionJson>,

    pub update_count: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FeedEntry {
    pub time: String,
    pub symbol: String,
    pub r#type: String,
    pub score: Option<u8>,
    pub desc: String,
    #[serde(default)]
    pub ts: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct AccessInfoJson {
    pub authenticated: bool,
    pub subscribed: bool,
    pub full_access: bool,
    pub visible_symbols: usize,
    pub total_symbols: usize,
    pub symbol_limit: Option<usize>,
    pub subscription_plan: Option<String>,
    pub subscription_expires_at: Option<String>,
    pub message: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FullSnapshot {
    pub symbols: Vec<SymbolJson>,
    pub feed: Vec<FeedEntry>,
    pub total_updates: u64,
    pub uptime_secs: u64,
    pub trader: TraderStateJson,
    pub access: AccessInfoJson,
}

pub struct DashboardState {
    // 所有币种的最新展示快照。
    pub symbols: HashMap<String, SymbolJson>,
    // 前端展示顺序。这里单独存一份，避免每次序列化时临时排序。
    pub sorted_keys: Vec<String>,
    // 全局实时 feed，按最新优先排列。
    pub feed: VecDeque<FeedEntry>,
    pub total_updates: u64,
    pub start_time: std::time::Instant,
}

impl DashboardState {
    pub fn new() -> Self {
        Self {
            symbols: HashMap::new(),
            sorted_keys: Vec::new(),
            feed: VecDeque::with_capacity(200),
            total_updates: 0,
            start_time: std::time::Instant::now(),
        }
    }

    pub fn upsert(&mut self, snap: SymbolJson) {
        // 每次更新单个币种后，顺手重建排序。
        // 当前规则是 pump_score 优先，其次按 symbol 字典序稳定排序。
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
        self.feed
            .retain(|item| is_same_local_day(item.ts, entry.ts));
        if self.feed.len() >= 200 {
            self.feed.pop_back();
        }
        self.feed.push_front(entry);
    }

    pub fn to_full_snapshot(&self, trader: TraderStateJson) -> FullSnapshot {
        let symbols = self
            .sorted_keys
            .iter()
            .filter_map(|k| self.symbols.get(k).cloned())
            .collect();
        FullSnapshot {
            symbols,
            feed: self.feed.iter().cloned().collect(),
            total_updates: self.total_updates,
            uptime_secs: self.start_time.elapsed().as_secs(),
            trader,
            access: AccessInfoJson::default(),
        }
    }

    pub fn to_light_snapshot(&self, trader: TraderStateJson) -> FullSnapshot {
        // 轻量快照用于 /api/state 和 WebSocket 高频推送。
        // 这里保留必要的盘口/评分/少量最近成交，避免 30 分钟成交历史把消息体撑爆。
        let symbols = self
            .sorted_keys
            .iter()
            .filter_map(|k| self.symbols.get(k).cloned())
            .map(|mut symbol| {
                symbol.klines.clear();
                symbol.current_kline.clear();
                if symbol.recent_trades.len() > LIVE_RECENT_TRADES_LIMIT {
                    symbol.recent_trades.truncate(LIVE_RECENT_TRADES_LIMIT);
                }
                symbol.signal_history.clear();
                symbol.factor_metrics.clear();
                symbol.enterprise_metrics.clear();
                symbol
            })
            .collect();
        FullSnapshot {
            symbols,
            feed: self.feed.iter().cloned().collect(),
            total_updates: self.total_updates,
            uptime_secs: self.start_time.elapsed().as_secs(),
            trader,
            access: AccessInfoJson::default(),
        }
    }

    pub fn to_cache_snapshot(&self) -> FullSnapshot {
        // 启动恢复缓存时不需要交易账户信息，
        // 但要保留 30 分钟最近成交，方便刷新后立即回显。
        let symbols = self
            .sorted_keys
            .iter()
            .filter_map(|k| self.symbols.get(k).cloned())
            .map(|mut symbol| {
                symbol.klines.clear();
                symbol.current_kline.clear();
                symbol.signal_history.clear();
                symbol.factor_metrics.clear();
                symbol.enterprise_metrics.clear();
                symbol
            })
            .collect();
        FullSnapshot {
            symbols,
            feed: self.feed.iter().cloned().collect(),
            total_updates: self.total_updates,
            uptime_secs: self.start_time.elapsed().as_secs(),
            trader: TraderStateJson::default(),
            access: AccessInfoJson::default(),
        }
    }

    pub fn get_symbol(&self, symbol: &str) -> Option<SymbolJson> {
        self.symbols.get(symbol).cloned()
    }

    pub fn replace_from_snapshot(&mut self, snapshot: FullSnapshot) {
        self.symbols.clear();
        self.sorted_keys.clear();
        self.feed.clear();

        for symbol in snapshot.symbols {
            let key = symbol.symbol.clone();
            self.sorted_keys.push(key.clone());
            self.symbols.insert(key, symbol);
        }

        let current_ts = chrono::Local::now().timestamp_millis();
        for entry in snapshot
            .feed
            .into_iter()
            .filter(|entry| is_same_local_day(entry.ts, current_ts))
            .take(200)
        {
            self.feed.push_back(entry);
        }

        self.total_updates = snapshot.total_updates;
        self.start_time = std::time::Instant::now();
    }
}

pub type SharedDashboardState = Arc<RwLock<DashboardState>>;

pub fn new_dashboard_state() -> SharedDashboardState {
    Arc::new(RwLock::new(DashboardState::new()))
}

fn is_same_local_day(entry_ts: i64, reference_ts: i64) -> bool {
    if entry_ts <= 0 || reference_ts <= 0 {
        return false;
    }
    use chrono::{Local, TimeZone};
    let Some(entry_dt) = Local.timestamp_millis_opt(entry_ts).single() else {
        return false;
    };
    let Some(reference_dt) = Local.timestamp_millis_opt(reference_ts).single() else {
        return false;
    };
    entry_dt.date_naive() == reference_dt.date_naive()
}
