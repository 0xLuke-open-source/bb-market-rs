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

#[derive(Debug, Clone)]
pub struct BigTradeEvent {
    pub time_ms: u64,
    pub price: f64,
    pub qty: f64,
    pub is_buy: bool,
}

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

    pub fn handle_depth_update(
        &mut self,
        update: DepthUpdate,
        report_interval: Duration,
    ) -> anyhow::Result<()> {
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

    pub fn apply_trade(&mut self, trade: &AggTrade) {
        let qty = trade.qty_decimal();
        let delta = trade.delta();

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

        let threshold = self
            .book
            .history
            .samples_raw
            .back()
            .map(|sample| sample.bid_volume)
            .unwrap_or(Decimal::ZERO)
            * dec!(0.01);

        if qty > threshold && !threshold.is_zero() {
            let now = trade.trade_time;
            while self
                .big_trades
                .front()
                .map(|entry| entry.time_ms + 120_000 < now)
                .unwrap_or(false)
            {
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

    pub fn apply_ticker(&mut self, ticker: &MiniTicker) {
        self.price_24h_open = ticker.open_f64();
        self.price_24h_high = ticker.high_f64();
        self.price_24h_low = ticker.low_f64();
        self.change_24h_pct = ticker.change_pct();
        self.volume_24h = ticker.volume_f64();
        self.quote_vol_24h = ticker.quote_volume_f64();
    }

    pub fn apply_kline(&mut self, event: &KlineEvent) {
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
