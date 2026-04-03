use chrono::{DateTime, Utc};
use rust_decimal::prelude::ToPrimitive;
use rust_decimal::Decimal;
use serde::Serialize;
use uuid::Uuid;

use crate::terminal::application::projection::SymbolJson;

#[derive(Debug, Clone)]
pub struct OrderBookSnapshotRecord {
    pub snapshot_id: Uuid,
    pub symbol: String,
    pub event_time: DateTime<Utc>,
    pub bid: f64,
    pub ask: f64,
    pub mid: f64,
    pub spread_bps: f64,
    pub total_bid_volume: f64,
    pub total_ask_volume: f64,
    pub ofi: f64,
    pub ofi_raw: f64,
    pub obi: f64,
    pub trend_strength: f64,
    pub cvd: f64,
    pub taker_buy_ratio: f64,
    pub price_precision: i32,
    pub quantity_precision: i32,
    pub bid_depth: Vec<DepthLevel>,
    pub ask_depth: Vec<DepthLevel>,
    pub update_count: i64,
}

#[derive(Debug, Clone, Serialize)]
pub struct DepthLevel {
    pub price: f64,
    pub quantity: f64,
}

#[derive(Debug, Clone)]
pub struct SymbolPanelSnapshotRecord {
    pub snapshot_id: Uuid,
    pub symbol: String,
    pub event_time: DateTime<Utc>,
    pub bid: f64,
    pub ask: f64,
    pub mid: f64,
    pub spread_bps: f64,
    pub change_24h_pct: f64,
    pub high_24h: f64,
    pub low_24h: f64,
    pub volume_24h: f64,
    pub quote_vol_24h: f64,
    pub ofi: f64,
    pub ofi_raw: f64,
    pub obi: f64,
    pub trend_strength: f64,
    pub cvd: f64,
    pub taker_buy_ratio: f64,
    pub pump_score: i32,
    pub dump_score: i32,
    pub pump_signal: bool,
    pub dump_signal: bool,
    pub whale_entry: bool,
    pub whale_exit: bool,
    pub bid_eating: bool,
    pub total_bid_volume: f64,
    pub total_ask_volume: f64,
    pub max_bid_ratio: f64,
    pub max_ask_ratio: f64,
    pub anomaly_count_1m: i32,
    pub anomaly_max_severity: i32,
    pub status_summary: String,
    pub watch_level: String,
    pub signal_reason: String,
    pub sentiment: String,
    pub risk_level: String,
    pub recommendation: String,
    pub whale_type: String,
    pub pump_probability: i32,
    pub price_precision: i32,
    pub quantity_precision: i32,
    pub snapshot_json: String,
    pub signal_history_json: String,
    pub factor_metrics_json: String,
    pub enterprise_metrics_json: String,
    pub update_count: i64,
    pub sample_signal_type: Option<String>,
    pub sample_trigger_score: Option<i32>,
}

#[derive(Debug, Clone)]
pub struct SignalPerformanceSampleRecord {
    pub sample_id: Uuid,
    pub symbol: String,
    pub signal_type: String,
    pub triggered_at: DateTime<Utc>,
    pub trigger_price: f64,
    pub trigger_score: i32,
    pub watch_level: String,
    pub signal_reason: String,
    pub update_count: i64,
    pub resolved_5m: bool,
    pub resolved_15m: bool,
    pub resolved_decay: bool,
    pub outcome_5m_return: Option<f64>,
    pub outcome_5m_win: Option<bool>,
    pub outcome_5m_at: Option<DateTime<Utc>>,
    pub outcome_15m_return: Option<f64>,
    pub outcome_15m_win: Option<bool>,
    pub outcome_15m_at: Option<DateTime<Utc>>,
    pub decay_minutes: Option<f64>,
    pub decay_at: Option<DateTime<Utc>>,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Clone)]
pub struct SalesProofOverviewRecord {
    pub window_days: i64,
    pub generated_at: DateTime<Utc>,
    pub total_samples: i64,
    pub unique_symbols: i64,
    pub resolved_5m_samples: i64,
    pub resolved_15m_samples: i64,
    pub resolved_decay_samples: i64,
    pub pump_samples: i64,
    pub dump_samples: i64,
    pub win_rate_5m: f64,
    pub win_rate_15m: f64,
    pub avg_return_5m: f64,
    pub avg_return_15m: f64,
    pub avg_decay_minutes: f64,
    pub avg_trigger_score: f64,
    pub latest_sample_at: Option<DateTime<Utc>>,
    pub top_symbols: Vec<SalesProofSymbolRecord>,
    pub signal_types: Vec<SalesProofSignalTypeRecord>,
}

#[derive(Debug, Clone)]
pub struct SalesProofSymbolRecord {
    pub symbol: String,
    pub sample_count: i64,
    pub resolved_5m_samples: i64,
    pub resolved_15m_samples: i64,
    pub resolved_decay_samples: i64,
    pub win_rate_5m: f64,
    pub win_rate_15m: f64,
    pub avg_return_5m: f64,
    pub avg_return_15m: f64,
    pub avg_decay_minutes: f64,
    pub avg_trigger_score: f64,
    pub latest_triggered_at: Option<DateTime<Utc>>,
}

#[derive(Debug, Clone)]
pub struct SalesProofSignalTypeRecord {
    pub signal_type: String,
    pub sample_count: i64,
    pub resolved_5m_samples: i64,
    pub resolved_15m_samples: i64,
    pub resolved_decay_samples: i64,
    pub win_rate_5m: f64,
    pub win_rate_15m: f64,
    pub avg_return_5m: f64,
    pub avg_return_15m: f64,
    pub avg_decay_minutes: f64,
    pub avg_trigger_score: f64,
}

impl OrderBookSnapshotRecord {
    pub fn from_snapshot(
        snapshot: &SymbolJson,
        top_bids_raw: &[(Decimal, Decimal)],
        top_asks_raw: &[(Decimal, Decimal)],
    ) -> Self {
        Self {
            snapshot_id: Uuid::new_v4(),
            symbol: snapshot.symbol.clone(),
            event_time: Utc::now(),
            bid: snapshot.bid,
            ask: snapshot.ask,
            mid: snapshot.mid,
            spread_bps: snapshot.spread_bps,
            total_bid_volume: snapshot.total_bid_volume,
            total_ask_volume: snapshot.total_ask_volume,
            ofi: snapshot.ofi,
            ofi_raw: snapshot.ofi_raw,
            obi: snapshot.obi,
            trend_strength: snapshot.trend_strength,
            cvd: snapshot.cvd,
            taker_buy_ratio: snapshot.taker_buy_ratio,
            price_precision: snapshot.price_precision as i32,
            quantity_precision: snapshot.quantity_precision as i32,
            bid_depth: to_depth_levels(top_bids_raw),
            ask_depth: to_depth_levels(top_asks_raw),
            update_count: snapshot.update_count.min(i64::MAX as u64) as i64,
        }
    }
}

fn to_depth_levels(levels: &[(Decimal, Decimal)]) -> Vec<DepthLevel> {
    levels
        .iter()
        .take(25)
        .filter_map(|(price, quantity)| {
            let price = price.to_f64()?;
            let quantity = quantity.to_f64()?;
            Some(DepthLevel { price, quantity })
        })
        .collect()
}
