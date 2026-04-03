//! snapshot 构建器负责把 `SymbolMonitor` 转成前端使用的 `SymbolJson`。
//!
//! 这里是 bridge 层最重要的映射点：算法域字段很多、层级也深，
//! 但前端真正需要的是一份扁平、完整、稳定的快照。

use std::collections::HashMap;

use rust_decimal::prelude::ToPrimitive;
use rust_decimal::Decimal;

use crate::market_data::application::runtime::SymbolMonitor;
use crate::market_data::domain::order_book::OrderBookFeatures;
use crate::signal_intelligence::domain::algorithms::ComprehensiveAnalysis;
use crate::signal_intelligence::domain::strategy_engine::StrategyEngine;
use crate::terminal::application::projection::{BigTradeJson, FeedEntry, KlineJson, SymbolJson};

use super::feed::build_feed_entries;
use super::labels::{
    recommendation_str, risk_str, sentiment_str, signal_reason, status_summary, watch_level_label,
};

pub struct BridgeUpdate {
    // 给前端的完整 symbol 快照。
    pub snapshot: SymbolJson,
    // 需要追加到信号流中的事件列表。
    pub feed_entries: Vec<FeedEntry>,
    // 原始盘口深度，供 spot 模块同步成模拟流动性。
    pub top_bids_raw: Vec<(Decimal, Decimal)>,
    pub top_asks_raw: Vec<(Decimal, Decimal)>,
}

pub fn build_bridge_update(symbol: &str, monitor: &mut SymbolMonitor) -> BridgeUpdate {
    let ctx = build_symbol_snapshot(symbol, monitor, SnapshotMode::Light);
    let feed_entries = build_feed_entries(
        symbol,
        &ctx.watch_level,
        &ctx.reason,
        &ctx.features,
        ctx.anomaly_max_severity,
        ctx.cvd,
    );

    BridgeUpdate {
        snapshot: ctx.snapshot,
        feed_entries,
        top_bids_raw: ctx.top_bids_raw,
        top_asks_raw: ctx.top_asks_raw,
    }
}

pub fn build_symbol_detail(symbol: &str, monitor: &mut SymbolMonitor) -> SymbolJson {
    build_symbol_snapshot(symbol, monitor, SnapshotMode::Full).snapshot
}

pub fn build_panel_persistence_snapshot(base: &SymbolJson, monitor: &SymbolMonitor) -> SymbolJson {
    let mut snapshot = base.clone();
    snapshot.klines = map_klines(monitor);
    snapshot.current_kline = map_current_klines(monitor);
    snapshot.big_trades = map_big_trades(monitor);
    snapshot.recent_trades = map_recent_trades(monitor);
    snapshot
}

struct SnapshotContext {
    snapshot: SymbolJson,
    top_bids_raw: Vec<(Decimal, Decimal)>,
    top_asks_raw: Vec<(Decimal, Decimal)>,
    features: OrderBookFeatures,
    watch_level: String,
    reason: String,
    anomaly_max_severity: u8,
    cvd: f64,
}

enum SnapshotMode {
    Light,
    Full,
}

fn build_symbol_snapshot(
    symbol: &str,
    monitor: &mut SymbolMonitor,
    mode: SnapshotMode,
) -> SnapshotContext {
    // 先统一计算一次特征，避免在同一轮桥接里重复跑多次 compute_features。
    let features = monitor.book.compute_features(10);
    let (top_bids_raw, top_asks_raw) = monitor.book.top_n(25);
    let mid = (features.weighted_bid_price + features.weighted_ask_price)
        .to_f64()
        .unwrap_or(0.0)
        / 2.0;

    let (anomaly_count_1m, anomaly_max_severity) = {
        let stats = monitor.anomaly_detector.get_stats();
        (stats.last_minute_count, stats.max_severity)
    };

    let analysis = analyze_monitor(monitor, &features);
    let strategy = StrategyEngine::analyze(
        &features,
        &analysis,
        &monitor.build_strategy_context(&features, anomaly_count_1m, anomaly_max_severity),
    );
    let update_count = monitor.update_count;

    let cvd = monitor.cvd.to_f64().unwrap_or(0.0);
    let taker_buy_ratio = monitor.taker_buy_ratio;
    let change_24h_pct = monitor.change_24h_pct;
    let high_24h = monitor.price_24h_high;
    let low_24h = monitor.price_24h_low;
    let volume_24h = monitor.volume_24h;
    let quote_vol_24h = monitor.quote_vol_24h;

    let watch_level = watch_level_label(
        &strategy,
        features.pump_score,
        features.dump_score,
        anomaly_max_severity,
        features.whale_entry,
    )
    .to_string();
    let summary = status_summary(
        &strategy,
        features.pump_score,
        features.dump_score,
        taker_buy_ratio,
        cvd,
        features.obi.to_f64().unwrap_or(0.0),
        anomaly_max_severity,
        features.whale_entry,
        features.whale_exit,
    );
    let reason = signal_reason(
        &strategy,
        features.pump_score,
        features.dump_score,
        taker_buy_ratio,
        cvd,
        features.obi.to_f64().unwrap_or(0.0),
        anomaly_count_1m,
        features.whale_entry,
        features.whale_exit,
    );

    let (klines, current_kline, big_trades, recent_trades) = match mode {
        SnapshotMode::Light => (HashMap::new(), HashMap::new(), Vec::new(), Vec::new()),
        SnapshotMode::Full => (
            map_klines(monitor),
            map_current_klines(monitor),
            map_big_trades(monitor),
            map_recent_trades(monitor),
        ),
    };

    let snapshot = SymbolJson {
        symbol: symbol.to_string(),
        status_summary: summary,
        watch_level: watch_level.clone(),
        signal_reason: reason.clone(),
        bid: features.weighted_bid_price.to_f64().unwrap_or(0.0),
        ask: features.weighted_ask_price.to_f64().unwrap_or(0.0),
        mid,
        spread_bps: features.spread_bps.to_f64().unwrap_or(0.0),
        price_precision: monitor.book.price_scale,
        quantity_precision: monitor.book.qty_scale,
        change_24h_pct,
        high_24h,
        low_24h,
        volume_24h,
        quote_vol_24h,
        ofi: features.ofi.to_f64().unwrap_or(0.0),
        ofi_raw: features.ofi_raw.to_f64().unwrap_or(0.0),
        obi: features.obi.to_f64().unwrap_or(0.0),
        trend_strength: features.trend_strength.to_f64().unwrap_or(0.0),
        cvd,
        taker_buy_ratio,
        pump_score: features.pump_score,
        dump_score: features.dump_score,
        pump_signal: features.pump_signal,
        dump_signal: features.dump_signal,
        whale_entry: features.whale_entry,
        whale_exit: features.whale_exit,
        bid_eating: features.bid_eating,
        total_bid_volume: features.total_bid_volume.to_f64().unwrap_or(0.0),
        total_ask_volume: features.total_ask_volume.to_f64().unwrap_or(0.0),
        max_bid_ratio: features.max_bid_ratio.to_f64().unwrap_or(0.0),
        max_ask_ratio: features.max_ask_ratio.to_f64().unwrap_or(0.0),
        top_bids: top_bids_raw
            .iter()
            .take(12)
            .map(|(price, qty)| [price.to_f64().unwrap_or(0.0), qty.to_f64().unwrap_or(0.0)])
            .collect(),
        top_asks: top_asks_raw
            .iter()
            .take(12)
            .map(|(price, qty)| [price.to_f64().unwrap_or(0.0), qty.to_f64().unwrap_or(0.0)])
            .collect(),
        anomaly_count_1m,
        anomaly_max_severity,
        sentiment: sentiment_str(&analysis.overall_sentiment),
        risk_level: risk_str(&analysis.risk_level),
        recommendation: recommendation_str(&analysis.trading_recommendation),
        whale_type: format!("{:?}", analysis.whale.whale_type),
        pump_probability: analysis.pump_dump.pump_probability,
        strategy_profile: strategy,
        klines,
        current_kline,
        big_trades,
        recent_trades,
        signal_history: Vec::new(),
        factor_metrics: Vec::new(),
        enterprise_metrics: Vec::new(),
        update_count,
    };

    SnapshotContext {
        snapshot,
        top_bids_raw,
        top_asks_raw,
        features,
        watch_level,
        reason,
        anomaly_max_severity,
        cvd,
    }
}

fn analyze_monitor(
    monitor: &mut SymbolMonitor,
    features: &OrderBookFeatures,
) -> ComprehensiveAnalysis {
    // `MarketIntelligence::analyze` 需要可变借用。
    // 这里用 take/put 回填的方式，避免与 monitor 其它字段借用冲突。
    let mut intel = monitor
        .market_intel
        .take()
        .expect("market_intel should be initialized");
    let result = intel.analyze(&monitor.book, features);
    monitor.market_intel = Some(intel);
    result
}

fn map_klines(monitor: &SymbolMonitor) -> HashMap<String, Vec<KlineJson>> {
    monitor
        .klines
        .iter()
        .map(|(interval, bars)| {
            let items = bars
                .iter()
                .map(|bar| KlineJson {
                    interval: bar.interval.clone(),
                    t: bar.open_time,
                    o: bar.open,
                    h: bar.high,
                    l: bar.low,
                    c: bar.close,
                    v: bar.volume,
                    tbr: bar.taker_buy_ratio,
                })
                .collect();
            (interval.clone(), items)
        })
        .collect()
}

fn map_current_klines(monitor: &SymbolMonitor) -> HashMap<String, KlineJson> {
    monitor
        .current_kline
        .iter()
        .map(|(interval, bar)| {
            (
                interval.clone(),
                KlineJson {
                    interval: bar.interval.clone(),
                    t: bar.open_time,
                    o: bar.open,
                    h: bar.high,
                    l: bar.low,
                    c: bar.close,
                    v: bar.volume,
                    tbr: bar.taker_buy_ratio,
                },
            )
        })
        .collect()
}

fn map_big_trades(monitor: &SymbolMonitor) -> Vec<BigTradeJson> {
    monitor
        .big_trades
        .iter()
        .rev()
        .take(10)
        .map(|trade| BigTradeJson {
            t: trade.time_ms,
            p: trade.price,
            q: trade.qty,
            buy: trade.is_buy,
        })
        .collect()
}

fn map_recent_trades(monitor: &SymbolMonitor) -> Vec<BigTradeJson> {
    monitor
        .recent_trades
        .iter()
        .rev()
        .map(|trade| BigTradeJson {
            t: trade.time_ms,
            p: trade.price,
            q: trade.qty,
            buy: trade.is_buy,
        })
        .collect()
}
