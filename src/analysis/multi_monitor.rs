// src/analysis/multi_monitor.rs — 多数据流版本
//
// 新增：处理 AggTrade / MiniTicker / KlineEvent
// 新增字段：CVD、Taker买入比、24h成交量/高低/涨跌、K线历史

use std::collections::{HashMap, VecDeque};
use std::fs::File;
use std::io::{BufRead, BufReader, Write};
use std::sync::Arc;
use rust_decimal::Decimal;
use rust_decimal::prelude::ToPrimitive;
use rust_decimal_macros::dec;
use tokio::sync::{mpsc, Mutex};
use tokio::time::{Duration, Instant};

use crate::codec::binance_msg::{DepthUpdate, AggTrade, MiniTicker, KlineEvent, StreamMsg};
use crate::store::l2_book::{OrderBook, OrderBookFeatures};
use crate::analysis::algorithms::MarketIntelligence;
use crate::analysis::orderbook_anomaly::OrderBookAnomalyDetector;
use crate::analysis::pump_detector::PumpDetector;

lazy_static::lazy_static! {
    static ref PUMP_DETECTOR: PumpDetector = PumpDetector::new("pump_signals.txt")
        .with_min_strength(30);
}

// ── 简化 K线结构（只存需要的字段） ───────────────────────────────
#[derive(Debug, Clone, Default)]
pub struct KlineBar {
    pub interval:  String,  // "1m","5m","1h"...
    pub open_time: u64,
    pub open:      f64,
    pub high:      f64,
    pub low:       f64,
    pub close:     f64,
    pub volume:    f64,
    pub taker_buy_ratio: f64,
    pub closed:    bool,
}

// ── 滚动成交窗口（用于大单检测） ─────────────────────────────────
#[derive(Debug, Clone)]
pub struct BigTradeEvent {
    pub time_ms: u64,
    pub price:   f64,
    pub qty:     f64,
    pub is_buy:  bool,
}

// ── SymbolMonitor ────────────────────────────────────────────────
pub struct SymbolMonitor {
    pub symbol:           String,
    pub book:             OrderBook,
    pub market_intel:     Option<MarketIntelligence>,
    pub last_report:      Instant,
    pub update_count:     u64,
    pub report_file:      String,
    pub anomaly_detector: OrderBookAnomalyDetector,
    pub anomaly_file:     String,

    // ── 新增：成交流数据 ───────────────────────────────────────
    /// 累计成交量差（主动买 − 主动卖），滚动保留最近 1 分钟
    pub cvd: Decimal,
    /// 最近1分钟 taker 买入量
    pub taker_buy_vol_1m:  Decimal,
    /// 最近1分钟 taker 卖出量
    pub taker_sell_vol_1m: Decimal,
    /// 主动买入占比（0-100）
    pub taker_buy_ratio:   f64,
    /// 大单成交事件（最近 2 分钟，>0.5% 盘口深度）
    pub big_trades:        VecDeque<BigTradeEvent>,

    // ── 新增：24h Ticker ──────────────────────────────────────
    pub price_24h_open:   f64,
    pub price_24h_high:   f64,
    pub price_24h_low:    f64,
    pub change_24h_pct:   f64,
    pub volume_24h:       f64,     // 成交量（基础资产）
    pub quote_vol_24h:    f64,     // 成交额（USDT）

    // ── 多周期K线（HashMap<interval, VecDeque<KlineBar>>）─────
    pub klines:        std::collections::HashMap<String, VecDeque<KlineBar>>,
    pub current_kline: std::collections::HashMap<String, KlineBar>,

    // 信号去重
    last_pump_signal_at:  Option<Instant>,
    last_dump_signal_at:  Option<Instant>,
}

impl SymbolMonitor {
    pub fn new(symbol: &str) -> Self {
        let ts = chrono::Local::now().format("%Y%m%d_%H%M%S");
        let report_file  = format!("reports/{}_{}.txt", symbol.to_lowercase(), ts);
        let anomaly_file = format!("anomaly/{}_{}.txt", symbol.to_lowercase(), ts);
        std::fs::create_dir_all("reports").unwrap_or_default();
        std::fs::create_dir_all("anomaly").unwrap_or_default();
        Self::init_file(symbol, &anomaly_file, "异动日志");
        Self::init_file(symbol, &report_file,  "市场分析报告");

        Self {
            symbol: symbol.to_string(),
            book: OrderBook::new(symbol),
            market_intel: Some(MarketIntelligence::new()),
            last_report: Instant::now(),
            update_count: 0,
            report_file,
            anomaly_file,
            anomaly_detector: OrderBookAnomalyDetector::new(),
            cvd: Decimal::ZERO,
            taker_buy_vol_1m:  Decimal::ZERO,
            taker_sell_vol_1m: Decimal::ZERO,
            taker_buy_ratio:   50.0,
            big_trades:        VecDeque::with_capacity(200),
            price_24h_open:  0.0,
            price_24h_high:  0.0,
            price_24h_low:   0.0,
            change_24h_pct:  0.0,
            volume_24h:      0.0,
            quote_vol_24h:   0.0,
            klines:        std::collections::HashMap::new(),
            current_kline: std::collections::HashMap::new(),
            last_pump_signal_at: None,
            last_dump_signal_at: None,
        }
    }

    fn init_file(symbol: &str, path: &str, label: &str) {
        let _ = std::fs::OpenOptions::new()
            .create(true).write(true).truncate(true).open(path)
            .map(|mut f| { let _ = writeln!(f, "=== {} {} ===", symbol, label); });
    }

    // ── 处理归集成交 ────────────────────────────────────────────
    pub fn apply_trade(&mut self, trade: &AggTrade) {
        let qty   = trade.qty_decimal();
        let delta = trade.delta(); // 主动买为正，主动卖为负

        self.cvd += delta;

        if trade.is_taker_buy() {
            self.taker_buy_vol_1m += qty;
        } else {
            self.taker_sell_vol_1m += qty;
        }

        // 更新主动买入占比
        let total = self.taker_buy_vol_1m + self.taker_sell_vol_1m;
        self.taker_buy_ratio = if total.is_zero() {
            50.0
        } else {
            (self.taker_buy_vol_1m / total * dec!(100)).to_f64().unwrap_or(50.0)
        };

        // 大单检测：成交量 > 买盘总量的 1%
        let threshold = self.book.history.samples_raw.back()
            .map(|s| s.bid_volume)
            .unwrap_or(Decimal::ZERO) * dec!(0.01);

        if qty > threshold && !threshold.is_zero() {
            let now = trade.trade_time;
            // 清除2分钟前的
            while self.big_trades.front().map(|t| t.time_ms + 120_000 < now).unwrap_or(false) {
                self.big_trades.pop_front();
            }
            self.big_trades.push_back(BigTradeEvent {
                time_ms: now,
                price: trade.price.parse().unwrap_or(0.0),
                qty: qty.to_f64().unwrap_or(0.0),
                is_buy: trade.is_taker_buy(),
            });
        }
    }

    // ── 处理 miniTicker ─────────────────────────────────────────
    pub fn apply_ticker(&mut self, ticker: &MiniTicker) {
        self.price_24h_open  = ticker.open_f64();
        self.price_24h_high  = ticker.high_f64();
        self.price_24h_low   = ticker.low_f64();
        self.change_24h_pct  = ticker.change_pct();
        self.volume_24h      = ticker.volume_f64();
        self.quote_vol_24h   = ticker.quote_volume_f64();
    }

    // ── 处理 K线（支持所有周期）───────────────────────────────
    pub fn apply_kline(&mut self, event: &KlineEvent) {
        let k = &event.kline;
        let interval = event.kline_interval();
        let bar = KlineBar {
            interval:        interval.clone(),
            open_time:       k.open_time,
            open:            k.open_f64(),
            high:            k.high_f64(),
            low:             k.low_f64(),
            close:           k.close_f64(),
            volume:          k.volume_f64(),
            taker_buy_ratio: k.taker_buy_ratio(),
            closed:          k.is_closed,
        };
        if k.is_closed {
            let hist = self.klines.entry(interval.clone()).or_insert_with(VecDeque::new);
            hist.push_back(bar);
            // 不同周期保留不同数量的历史
            let max_len: usize = match interval.as_str() {
                "1m"|"3m"  => 200,
                "5m"|"15m" => 150,
                "30m"|"1h" => 100,
                _           => 60,
            };
            while hist.len() > max_len { hist.pop_front(); }
            self.current_kline.remove(&interval);
            // 1m 收盘时重置短期成交量
            if interval == "1m" {
                self.taker_buy_vol_1m  = Decimal::ZERO;
                self.taker_sell_vol_1m = Decimal::ZERO;
            }
        } else {
            self.current_kline.insert(interval, bar);
        }
    }

    pub fn should_emit_pump(&mut self) -> bool {
        let now = Instant::now();
        match self.last_pump_signal_at {
            Some(t) if now.duration_since(t) < Duration::from_secs(30) => false,
            _ => { self.last_pump_signal_at = Some(now); true }
        }
    }
    pub fn should_emit_dump(&mut self) -> bool {
        let now = Instant::now();
        match self.last_dump_signal_at {
            Some(t) if now.duration_since(t) < Duration::from_secs(30) => false,
            _ => { self.last_dump_signal_at = Some(now); true }
        }
    }
}

// ── MultiSymbolMonitor ───────────────────────────────────────────
pub struct MultiSymbolMonitor {
    pub monitors: Arc<Mutex<HashMap<String, Arc<Mutex<SymbolMonitor>>>>>,
    pub report_interval: Duration,
}

impl MultiSymbolMonitor {
    pub fn new(report_interval_secs: u64) -> Self {
        Self {
            monitors: Arc::new(Mutex::new(HashMap::new())),
            report_interval: Duration::from_secs(report_interval_secs),
        }
    }

    pub async fn load_symbols_from_file(&self, path: &str, max: usize) -> anyhow::Result<Vec<String>> {
        let file   = File::open(path)?;
        let reader = BufReader::new(file);
        let mut symbols = Vec::new();
        for line in reader.lines() {
            let s = line?.trim().to_string();
            if !s.is_empty() && s != "币安人生" {
                symbols.push(format!("{}USDT", s));
                if symbols.len() >= max { break; }
            }
        }
        println!("✅ 加载 {} 个币种", symbols.len());
        Ok(symbols)
    }

    pub async fn init_monitors(&self, symbols: Vec<String>) {
        let mut monitors = self.monitors.lock().await;
        for symbol in symbols {
            monitors.insert(symbol.clone(), Arc::new(Mutex::new(SymbolMonitor::new(&symbol))));
        }
        println!("🚀 初始化 {} 个监控器", monitors.len());
    }

    // ── 统一处理所有 stream 消息 ─────────────────────────────────
    pub async fn handle_msg(&self, symbol: &str, msg: StreamMsg) -> anyhow::Result<()> {
        let arc = {
            let m = self.monitors.lock().await;
            m.get(symbol).cloned()
        };
        let Some(arc) = arc else { return Ok(()); };
        let mut guard = arc.lock().await;

        match msg {
            StreamMsg::Depth(update) => {
                if let Err(e) = guard.book.apply_incremental_update(update) {
                    eprintln!("[{}] depth error: {}", symbol, e);
                    return Ok(());
                }
                guard.update_count += 1;

                // 异动检测（每次深度更新都跑）
                let features = guard.book.compute_features(10);
                // let anomalies = guard.anomaly_detector.detect(&guard.book, &features);


                let anomalies = {
                    let sm_ptr: *mut SymbolMonitor = &mut *guard;
                    unsafe {
                        let book_ref = &(*sm_ptr).book;
                        let detector = &mut (*sm_ptr).anomaly_detector;
                        detector.detect(book_ref, &features)
                    }
                };

                for a in &anomalies {
                    if a.severity >= 60 {
                        eprintln!("[{}] ⚠️ {:?} sev:{} {}", symbol, a.anomaly_type, a.severity, a.description);
                    }
                }

                // 定期生成分析报告
                if guard.last_report.elapsed() >= self.report_interval {
                    if guard.book.best_bid_ask().is_some() {
                        let features = guard.book.compute_features(10);
                        guard.book.auto_sample(&features);
                    }
                    guard.last_report = Instant::now();
                }
            }
            StreamMsg::Trade(trade) => {
                guard.apply_trade(&trade);
            }
            StreamMsg::Ticker(ticker) => {
                guard.apply_ticker(&ticker);
            }
            StreamMsg::Kline(kline) => {
                guard.apply_kline(&kline);
            }
        }
        Ok(())
    }

    pub async fn get_active_symbols(&self) -> Vec<String> {
        self.monitors.lock().await.keys().cloned().collect()
    }

    pub async fn detect_pump_signals(&self) -> anyhow::Result<()> {
        let arcs: Vec<(String, Arc<Mutex<SymbolMonitor>>)> = {
            let m = self.monitors.lock().await;
            m.iter().map(|(k, v)| (k.clone(), v.clone())).collect()
        };

        let mut signals = Vec::new();
        for (symbol, arc) in arcs {
            let mut guard = arc.lock().await;
            let features = guard.book.compute_features(10);
            if features.pump_score < 30 && features.dump_score < 30 { continue; }

            let (features2, top_bids, top_asks, mid, price_change) = {
                let f = guard.book.compute_features(10);
                let (b, a) = guard.book.top_n(12);
                let mid = (f.weighted_bid_price + f.weighted_ask_price).to_f64().unwrap_or(0.0) / 2.0;
                let pc  = guard.book.history.samples_raw.get(1)
                    .map(|s| s.mid_price.to_f64().unwrap_or(mid)).unwrap_or(mid);
                let pct = if pc != 0.0 { (mid - pc) / pc * 100.0 } else { 0.0 };
                (f, b, a, mid, pct)
            };

            let (ac1m, asev) = {
                let s = guard.anomaly_detector.get_stats();
                (s.last_minute_count, s.max_severity)
            };
            let update_count = guard.update_count;

            let analysis = {
                let sm = &mut *guard;
                let sm_ptr: *mut SymbolMonitor = sm;
                unsafe {
                    let book_ref = &(*sm_ptr).book;
                    let intel    = (*sm_ptr).market_intel.as_mut().unwrap();
                    intel.analyze(book_ref, &features2)
                }
            };

            if let Some(signal) = PUMP_DETECTOR.analyze_symbol(
                &symbol, &features2,
                analysis.pump_dump.pump_probability,
                analysis.whale.accumulation_score.to_u8().unwrap_or(0),
                analysis.pump_dump.pump_target,
            ) {
                let ok = if signal.ofi > 0.0 { guard.should_emit_pump() } else { guard.should_emit_dump() };
                if ok { let _ = PUMP_DETECTOR.write_pump_signal(&signal); signals.push(signal); }
            }
        }

        use std::sync::atomic::{AtomicUsize, Ordering};
        static CNT: AtomicUsize = AtomicUsize::new(0);
        let c = CNT.fetch_add(1, Ordering::SeqCst);
        if c % 10 == 0 && !signals.is_empty() {
            let _ = PUMP_DETECTOR.write_top_signals(&mut signals);
        }
        Ok(())
    }
}

// ── WebSocket 管理器 ─────────────────────────────────────────────
pub struct MultiWebSocketManager {
    monitors: Arc<MultiSymbolMonitor>,
    tasks:    tokio::task::JoinSet<()>,
}

impl MultiWebSocketManager {
    pub fn new(monitors: Arc<MultiSymbolMonitor>) -> Self {
        Self { monitors, tasks: tokio::task::JoinSet::new() }
    }

    pub async fn start_all(&mut self, symbols: Vec<String>) {
        for symbol in symbols {
            let monitors  = self.monitors.clone();
            let sym_clone = symbol.clone();

            self.tasks.spawn(async move {
                loop {
                    let (tx, mut rx) = mpsc::channel::<StreamMsg>(2000);

                    let ws_sym = sym_clone.clone();
                    let ws_task = tokio::spawn(async move {
                        loop {
                            match crate::client::websocket::run_client(&ws_sym, tx.clone()).await {
                                Ok(())  => eprintln!("[{}] WS exited", ws_sym),
                                Err(e)  => eprintln!("[{}] WS error: {}", ws_sym, e),
                            }
                            tokio::time::sleep(Duration::from_secs(5)).await;
                        }
                    });

                    while let Some(msg) = rx.recv().await {
                        if let Err(e) = monitors.handle_msg(&sym_clone, msg).await {
                            eprintln!("[{}] handle error: {}", sym_clone, e);
                        }
                    }

                    ws_task.abort();
                    tokio::time::sleep(Duration::from_secs(5)).await;
                }
            });

            tokio::time::sleep(Duration::from_millis(300)).await;
        }
    }

    pub async fn wait(&mut self) {
        while let Some(r) = self.tasks.join_next().await {
            if let Err(e) = r { eprintln!("task error: {}", e); }
        }
    }
}