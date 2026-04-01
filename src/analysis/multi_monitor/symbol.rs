//! 单个交易对的实时监控状态机。
//!
//! `SymbolMonitor` 是项目里最核心的运行时对象之一。它持续接收：
//! - 深度更新，维护本地订单簿与深度特征
//! - 成交通知，累计 CVD、主动买卖量和大单事件
//! - ticker / kline，补齐更高层展示和判断所需的背景信息
//!
//! 最终这些状态会被综合分析模块消费，用来判断异常、鲸鱼行为和 pump/dump 信号。

use std::collections::{HashMap, VecDeque};

use rust_decimal::prelude::ToPrimitive;
use rust_decimal::Decimal;
use rust_decimal_macros::dec;
use tokio::time::{Duration, Instant};

use crate::analysis::algorithms::MarketIntelligence;
use crate::analysis::multi_monitor::signal;
use crate::analysis::orderbook_anomaly::OrderBookAnomalyDetector;
use crate::analysis::pump_detector::PumpSignal;
use crate::codec::binance_msg::{AggTrade, DepthUpdate, KlineEvent, MiniTicker};
use crate::store::l2_book::OrderBook;

const RECENT_TRADES_WINDOW_MS: u64 = 30 * 60 * 1000;
const RECENT_TRADES_MAX_LEN: usize = 3000;

/// 统一的 K 线缓存结构。
///
/// 项目同时订阅多个周期 K 线，因此这里用统一结构承接 Binance 事件，
/// 后续再按周期存到 `klines` / `current_kline` 两组缓存中。
#[derive(Debug, Clone, Default)]
pub struct KlineBar {
    pub interval: String,
    pub open_time: u64,
    pub open: f64,
    pub high: f64,
    pub low: f64,
    pub close: f64,
    pub volume: f64,
    pub taker_buy_ratio: f64,
    pub closed: bool,
}

/// 近两分钟内的大额成交事件。
///
/// 这部分数据主要用于辅助解释短时成交冲击，而不是直接驱动所有信号。
#[derive(Debug, Clone)]
pub struct BigTradeEvent {
    pub time_ms: u64,
    pub price: f64,
    pub qty: f64,
    pub is_buy: bool,
    pub threshold_qty: f64,
}

/// 单个 symbol 的完整监控上下文。
///
/// 这里聚合了订单簿、K 线、24h ticker、主动成交统计和信号冷却时间，
/// 使得单个交易对的状态完全封装在一个对象里。
pub struct SymbolMonitor {
    pub symbol: String,
    pub book: OrderBook,
    pub market_intel: Option<MarketIntelligence>,
    pub last_report: Instant,
    pub update_count: u64,
    pub anomaly_detector: OrderBookAnomalyDetector,
    pub cvd: Decimal,
    pub taker_buy_vol_1m: Decimal,
    pub taker_sell_vol_1m: Decimal,
    pub taker_buy_ratio: f64,
    pub big_trades: VecDeque<BigTradeEvent>,
    pub recent_trades: VecDeque<BigTradeEvent>,
    pub price_24h_open: f64,
    pub price_24h_high: f64,
    pub price_24h_low: f64,
    pub change_24h_pct: f64,
    pub volume_24h: f64,
    pub quote_vol_24h: f64,
    pub klines: HashMap<String, VecDeque<KlineBar>>,
    pub current_kline: HashMap<String, KlineBar>,
    last_pump_signal_at: Option<Instant>,
    last_dump_signal_at: Option<Instant>,
}

impl SymbolMonitor {
    /// 初始化一个全新的 symbol 监控器。
    pub fn new(symbol: &str) -> Self {
        Self {
            symbol: symbol.to_string(),
            book: OrderBook::new(symbol),
            market_intel: Some(MarketIntelligence::new()),
            last_report: Instant::now(),
            update_count: 0,
            anomaly_detector: OrderBookAnomalyDetector::new(),
            cvd: Decimal::ZERO,
            taker_buy_vol_1m: Decimal::ZERO,
            taker_sell_vol_1m: Decimal::ZERO,
            taker_buy_ratio: 50.0,
            big_trades: VecDeque::with_capacity(200),
            recent_trades: VecDeque::with_capacity(RECENT_TRADES_MAX_LEN),
            price_24h_open: 0.0,
            price_24h_high: 0.0,
            price_24h_low: 0.0,
            change_24h_pct: 0.0,
            volume_24h: 0.0,
            quote_vol_24h: 0.0,
            klines: HashMap::new(),
            current_kline: HashMap::new(),
            last_pump_signal_at: None,
            last_dump_signal_at: None,
        }
    }

    /// 应用一笔深度增量更新。
    ///
    /// 处理顺序是：
    /// 1. 更新本地 L2 订单簿
    /// 2. 基于最新盘口重算特征
    /// 3. 将特征送入异常检测器
    /// 4. 在采样周期到达时把特征写入历史
    ///
    /// 如果增量序列不连续，`OrderBook` 会返回错误；这里直接跳过，
    /// 由上游继续流式同步，而不是在本层中断整个监控流程。
    pub fn handle_depth_update(
        &mut self,
        update: DepthUpdate,
        report_interval: Duration,
    ) -> anyhow::Result<()> {
        if !update.symbol.eq_ignore_ascii_case(&self.symbol) {
            return Ok(());
        }
        if self.book.apply_incremental_update(update).is_err() {
            return Ok(());
        }
        self.update_count += 1;

        let features = self.book.compute_features(10);
        {
            let book = &self.book;
            let detector = &mut self.anomaly_detector;
            let _ = detector.detect(book, &features);
        }

        if self.last_report.elapsed() >= report_interval {
            if self.book.best_bid_ask().is_some() {
                self.book.auto_sample(&features);
            }
            self.last_report = Instant::now();
        }

        Ok(())
    }

    /// 处理聚合成交，更新主动买卖统计。
    ///
    /// 这里会维护：
    /// - `cvd`：累计主动成交 delta
    /// - `taker_buy_ratio`：当前 1m 窗口主动买比例
    /// - `big_trades`：相对盘口深度足够大的成交事件
    pub fn apply_trade(&mut self, trade: &AggTrade) -> Option<BigTradeEvent> {
        if !trade.symbol.eq_ignore_ascii_case(&self.symbol) {
            return None;
        }
        let qty = trade.qty_decimal();
        let delta = trade.delta();
        let now = trade.trade_time;
        let price = trade.price.parse().unwrap_or(0.0);
        let qty_f64 = qty.to_f64().unwrap_or(0.0);
        let is_buy = trade.is_taker_buy();
        self.cvd += delta;

        if trade.is_taker_buy() {
            self.taker_buy_vol_1m += qty;
        } else {
            self.taker_sell_vol_1m += qty;
        }

        let total = self.taker_buy_vol_1m + self.taker_sell_vol_1m;
        self.taker_buy_ratio = if total.is_zero() {
            50.0
        } else {
            (self.taker_buy_vol_1m / total * dec!(100))
                .to_f64()
                .unwrap_or(50.0)
        };

        while self
            .recent_trades
            .front()
            .map(|entry| entry.time_ms + RECENT_TRADES_WINDOW_MS < now)
            .unwrap_or(false)
        {
            self.recent_trades.pop_front();
        }
        if self.recent_trades.len() >= RECENT_TRADES_MAX_LEN {
            self.recent_trades.pop_front();
        }
        self.recent_trades.push_back(BigTradeEvent {
            time_ms: now,
            price,
            qty: qty_f64,
            is_buy,
            threshold_qty: 0.0,
        });

        let threshold = self
            .book
            .history
            .samples_raw
            .back()
            .map(|sample| sample.bid_volume)
            .unwrap_or(Decimal::ZERO)
            * dec!(0.01);

        if qty > threshold && !threshold.is_zero() {
            while self
                .big_trades
                .front()
                .map(|entry| entry.time_ms + 120_000 < now)
                .unwrap_or(false)
            {
                self.big_trades.pop_front();
            }
            let event = BigTradeEvent {
                time_ms: now,
                price,
                qty: qty_f64,
                is_buy,
                threshold_qty: threshold.to_f64().unwrap_or(0.0),
            };
            self.big_trades.push_back(event.clone());
            return Some(event);
        }

        None
    }

    /// 写入 24h ticker 背景数据，主要服务于前端展示和辅助解释。
    pub fn apply_ticker(&mut self, ticker: &MiniTicker) {
        if !ticker.symbol.eq_ignore_ascii_case(&self.symbol) {
            return;
        }
        self.price_24h_open = ticker.open_f64();
        self.price_24h_high = ticker.high_f64();
        self.price_24h_low = ticker.low_f64();
        self.change_24h_pct = ticker.change_pct();
        self.volume_24h = ticker.volume_f64();
        self.quote_vol_24h = ticker.quote_volume_f64();
    }

    /// 维护多周期 K 线缓存。
    ///
    /// 已收盘 K 线进入历史队列，未收盘 K 线放进 `current_kline`；
    /// 当 1m K 线收盘时，同时重置本分钟主动买卖量统计。
    pub fn apply_kline(&mut self, event: &KlineEvent) {
        if !event.symbol.eq_ignore_ascii_case(&self.symbol) {
            return;
        }
        let kline = &event.kline;
        let interval = event.kline_interval();
        let bar = KlineBar {
            interval: interval.clone(),
            open_time: kline.open_time,
            open: kline.open_f64(),
            high: kline.high_f64(),
            low: kline.low_f64(),
            close: kline.close_f64(),
            volume: kline.volume_f64(),
            taker_buy_ratio: kline.taker_buy_ratio(),
            closed: kline.is_closed,
        };

        if kline.is_closed {
            let history = self.klines.entry(interval.clone()).or_default();
            history.push_back(bar);
            let max_len = match interval.as_str() {
                "1m" | "3m" => 200,
                "5m" | "15m" => 150,
                "30m" | "1h" => 100,
                _ => 60,
            };
            while history.len() > max_len {
                history.pop_front();
            }
            self.current_kline.remove(&interval);
            if interval == "1m" {
                self.taker_buy_vol_1m = Decimal::ZERO;
                self.taker_sell_vol_1m = Decimal::ZERO;
            }
        } else {
            self.current_kline.insert(interval, bar);
        }
    }

    /// 结合订单簿特征与综合算法结果生成 pump/dump 信号。
    ///
    /// 本方法有两层过滤：
    /// - 第一层：`pump_score` / `dump_score` 太低时直接跳过，避免无意义分析
    /// - 第二层：信号生成后再走 30 秒冷却，避免同一交易对频繁重复报警
    pub fn detect_pump_signal(&mut self) -> Option<PumpSignal> {
        let features = self.book.compute_features(10);
        if features.pump_score < 30 && features.dump_score < 30 {
            return None;
        }

        let analysis = {
            let intel = self.market_intel.as_mut()?;
            intel.analyze(&self.book, &features)
        };

        let signal = signal::analyze_signal(
            &self.symbol,
            &features,
            analysis.pump_dump.pump_probability,
            analysis.whale.accumulation_score.to_u8().unwrap_or(0),
            analysis.pump_dump.pump_target,
        )?;

        let should_emit = if signal.ofi > 0.0 {
            self.should_emit_pump()
        } else {
            self.should_emit_dump()
        };
        if !should_emit {
            return None;
        }

        signal::write_signal(&signal);
        Some(signal)
    }

    /// 对偏多信号做节流，防止单个 symbol 高频重复触发。
    fn should_emit_pump(&mut self) -> bool {
        let now = Instant::now();
        match self.last_pump_signal_at {
            Some(last) if now.duration_since(last) < Duration::from_secs(30) => false,
            _ => {
                self.last_pump_signal_at = Some(now);
                true
            }
        }
    }

    /// 对偏空信号做节流，逻辑与 pump 冷却对称。
    fn should_emit_dump(&mut self) -> bool {
        let now = Instant::now();
        match self.last_dump_signal_at {
            Some(last) if now.duration_since(last) < Duration::from_secs(30) => false,
            _ => {
                self.last_dump_signal_at = Some(now);
                true
            }
        }
    }
}
