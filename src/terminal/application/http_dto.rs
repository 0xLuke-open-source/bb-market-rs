use serde::Serialize;
use serde_json::Value;

use crate::market_data::application::{BigTradeHistoryRecord, BigTradeStatsRecord};
use crate::terminal::application::dto::{
    SalesProofOverviewRecord, SalesProofSignalTypeRecord, SalesProofSymbolRecord,
    SignalPerformanceSampleRecord, SymbolPanelSnapshotRecord,
};

#[derive(Debug, Serialize)]
pub struct BigTradeHistoryItemJson {
    symbol: String,
    agg_trade_id: i64,
    event_time: String,
    event_ts: i64,
    trade_time: String,
    trade_ts: i64,
    price: f64,
    quantity: f64,
    quote_quantity: f64,
    threshold_quantity: f64,
    is_taker_buy: bool,
    is_buyer_maker: bool,
}

#[derive(Debug, Serialize)]
pub struct BigTradeStatsJson {
    symbol: String,
    total_count: i64,
    buy_count: i64,
    sell_count: i64,
    total_quote_quantity: f64,
    buy_quote_quantity: f64,
    sell_quote_quantity: f64,
    buy_ratio: f64,
    sell_ratio: f64,
    avg_quote_quantity: f64,
    max_quote_quantity: f64,
    avg_threshold_quantity: f64,
    first_trade_time: Option<String>,
    first_trade_ts: Option<i64>,
    last_trade_time: Option<String>,
    last_trade_ts: Option<i64>,
}

#[derive(Debug, Serialize)]
pub struct PanelSnapshotJson {
    event_ts: i64,
    event_time: String,
    bid: f64,
    ask: f64,
    mid: f64,
    spread_bps: f64,
    change_24h_pct: f64,
    high_24h: f64,
    low_24h: f64,
    volume_24h: f64,
    quote_vol_24h: f64,
    ofi: f64,
    ofi_raw: f64,
    obi: f64,
    trend_strength: f64,
    cvd: f64,
    taker_buy_ratio: f64,
    pump_score: i32,
    dump_score: i32,
    pump_signal: bool,
    dump_signal: bool,
    whale_entry: bool,
    whale_exit: bool,
    bid_eating: bool,
    total_bid_volume: f64,
    total_ask_volume: f64,
    max_bid_ratio: f64,
    max_ask_ratio: f64,
    anomaly_count_1m: i32,
    anomaly_max_severity: i32,
    status_summary: String,
    watch_level: String,
    signal_reason: String,
    sentiment: String,
    risk_level: String,
    recommendation: String,
    whale_type: String,
    pump_probability: i32,
    price_precision: i32,
    quantity_precision: i32,
    snapshot: Value,
    signal_history: Value,
    factor_metrics: Value,
    enterprise_metrics: Value,
    update_count: i64,
}

#[derive(Debug, Serialize)]
pub struct SignalPerformanceSampleJson {
    sample_id: String,
    symbol: String,
    signal_type: String,
    triggered_at: String,
    triggered_ts: i64,
    trigger_price: f64,
    trigger_score: i32,
    watch_level: String,
    signal_reason: String,
    update_count: i64,
    resolved_5m: bool,
    resolved_15m: bool,
    resolved_decay: bool,
    outcome_5m_return: Option<f64>,
    outcome_5m_win: Option<bool>,
    outcome_5m_at: Option<String>,
    outcome_5m_ts: Option<i64>,
    outcome_15m_return: Option<f64>,
    outcome_15m_win: Option<bool>,
    outcome_15m_at: Option<String>,
    outcome_15m_ts: Option<i64>,
    decay_minutes: Option<f64>,
    decay_at: Option<String>,
    decay_ts: Option<i64>,
    created_at: String,
    created_ts: i64,
}

#[derive(Debug, Serialize)]
pub struct SalesProofOverviewJson {
    window_days: i64,
    generated_at: String,
    generated_ts: i64,
    total_samples: i64,
    unique_symbols: i64,
    resolved_5m_samples: i64,
    resolved_15m_samples: i64,
    resolved_decay_samples: i64,
    pump_samples: i64,
    dump_samples: i64,
    win_rate_5m: f64,
    win_rate_15m: f64,
    avg_return_5m: f64,
    avg_return_15m: f64,
    avg_decay_minutes: f64,
    avg_trigger_score: f64,
    latest_sample_at: Option<String>,
    latest_sample_ts: Option<i64>,
    top_symbols: Vec<SalesProofSymbolJson>,
    signal_types: Vec<SalesProofSignalTypeJson>,
}

#[derive(Debug, Serialize)]
pub struct SalesProofSymbolJson {
    symbol: String,
    sample_count: i64,
    resolved_5m_samples: i64,
    resolved_15m_samples: i64,
    resolved_decay_samples: i64,
    win_rate_5m: f64,
    win_rate_15m: f64,
    avg_return_5m: f64,
    avg_return_15m: f64,
    avg_decay_minutes: f64,
    avg_trigger_score: f64,
    latest_triggered_at: Option<String>,
    latest_triggered_ts: Option<i64>,
}

#[derive(Debug, Serialize)]
pub struct SalesProofSignalTypeJson {
    signal_type: String,
    sample_count: i64,
    resolved_5m_samples: i64,
    resolved_15m_samples: i64,
    resolved_decay_samples: i64,
    win_rate_5m: f64,
    win_rate_15m: f64,
    avg_return_5m: f64,
    avg_return_15m: f64,
    avg_decay_minutes: f64,
    avg_trigger_score: f64,
}

pub fn panel_snapshot_json_from_record(record: SymbolPanelSnapshotRecord) -> PanelSnapshotJson {
    PanelSnapshotJson {
        event_ts: record.event_time.timestamp_millis(),
        event_time: record.event_time.to_rfc3339(),
        bid: record.bid,
        ask: record.ask,
        mid: record.mid,
        spread_bps: record.spread_bps,
        change_24h_pct: record.change_24h_pct,
        high_24h: record.high_24h,
        low_24h: record.low_24h,
        volume_24h: record.volume_24h,
        quote_vol_24h: record.quote_vol_24h,
        ofi: record.ofi,
        ofi_raw: record.ofi_raw,
        obi: record.obi,
        trend_strength: record.trend_strength,
        cvd: record.cvd,
        taker_buy_ratio: record.taker_buy_ratio,
        pump_score: record.pump_score,
        dump_score: record.dump_score,
        pump_signal: record.pump_signal,
        dump_signal: record.dump_signal,
        whale_entry: record.whale_entry,
        whale_exit: record.whale_exit,
        bid_eating: record.bid_eating,
        total_bid_volume: record.total_bid_volume,
        total_ask_volume: record.total_ask_volume,
        max_bid_ratio: record.max_bid_ratio,
        max_ask_ratio: record.max_ask_ratio,
        anomaly_count_1m: record.anomaly_count_1m,
        anomaly_max_severity: record.anomaly_max_severity,
        status_summary: record.status_summary,
        watch_level: record.watch_level,
        signal_reason: record.signal_reason,
        sentiment: record.sentiment,
        risk_level: record.risk_level,
        recommendation: record.recommendation,
        whale_type: record.whale_type,
        pump_probability: record.pump_probability,
        price_precision: record.price_precision,
        quantity_precision: record.quantity_precision,
        snapshot: parse_json_value(&record.snapshot_json),
        signal_history: parse_json_value(&record.signal_history_json),
        factor_metrics: parse_json_value(&record.factor_metrics_json),
        enterprise_metrics: parse_json_value(&record.enterprise_metrics_json),
        update_count: record.update_count,
    }
}

pub fn signal_perf_sample_json_from_record(
    record: SignalPerformanceSampleRecord,
) -> SignalPerformanceSampleJson {
    SignalPerformanceSampleJson {
        sample_id: record.sample_id.to_string(),
        symbol: record.symbol,
        signal_type: record.signal_type,
        triggered_at: record.triggered_at.to_rfc3339(),
        triggered_ts: record.triggered_at.timestamp_millis(),
        trigger_price: record.trigger_price,
        trigger_score: record.trigger_score,
        watch_level: record.watch_level,
        signal_reason: record.signal_reason,
        update_count: record.update_count,
        resolved_5m: record.resolved_5m,
        resolved_15m: record.resolved_15m,
        resolved_decay: record.resolved_decay,
        outcome_5m_return: record.outcome_5m_return,
        outcome_5m_win: record.outcome_5m_win,
        outcome_5m_at: record.outcome_5m_at.map(|value| value.to_rfc3339()),
        outcome_5m_ts: record.outcome_5m_at.map(|value| value.timestamp_millis()),
        outcome_15m_return: record.outcome_15m_return,
        outcome_15m_win: record.outcome_15m_win,
        outcome_15m_at: record.outcome_15m_at.map(|value| value.to_rfc3339()),
        outcome_15m_ts: record.outcome_15m_at.map(|value| value.timestamp_millis()),
        decay_minutes: record.decay_minutes,
        decay_at: record.decay_at.map(|value| value.to_rfc3339()),
        decay_ts: record.decay_at.map(|value| value.timestamp_millis()),
        created_at: record.created_at.to_rfc3339(),
        created_ts: record.created_at.timestamp_millis(),
    }
}

pub fn sales_proof_overview_json_from_record(
    record: SalesProofOverviewRecord,
) -> SalesProofOverviewJson {
    SalesProofOverviewJson {
        window_days: record.window_days,
        generated_at: record.generated_at.to_rfc3339(),
        generated_ts: record.generated_at.timestamp_millis(),
        total_samples: record.total_samples,
        unique_symbols: record.unique_symbols,
        resolved_5m_samples: record.resolved_5m_samples,
        resolved_15m_samples: record.resolved_15m_samples,
        resolved_decay_samples: record.resolved_decay_samples,
        pump_samples: record.pump_samples,
        dump_samples: record.dump_samples,
        win_rate_5m: record.win_rate_5m,
        win_rate_15m: record.win_rate_15m,
        avg_return_5m: record.avg_return_5m,
        avg_return_15m: record.avg_return_15m,
        avg_decay_minutes: record.avg_decay_minutes,
        avg_trigger_score: record.avg_trigger_score,
        latest_sample_at: record.latest_sample_at.map(|value| value.to_rfc3339()),
        latest_sample_ts: record
            .latest_sample_at
            .map(|value| value.timestamp_millis()),
        top_symbols: record
            .top_symbols
            .into_iter()
            .map(sales_proof_symbol_json_from_record)
            .collect(),
        signal_types: record
            .signal_types
            .into_iter()
            .map(sales_proof_signal_type_json_from_record)
            .collect(),
    }
}

pub fn big_trade_history_item_json_from_record(
    record: BigTradeHistoryRecord,
) -> BigTradeHistoryItemJson {
    BigTradeHistoryItemJson {
        symbol: record.symbol,
        agg_trade_id: record.agg_trade_id,
        event_time: record.event_time.to_rfc3339(),
        event_ts: record.event_time.timestamp_millis(),
        trade_time: record.trade_time.to_rfc3339(),
        trade_ts: record.trade_time.timestamp_millis(),
        price: record.price,
        quantity: record.quantity,
        quote_quantity: record.quote_quantity,
        threshold_quantity: record.threshold_quantity,
        is_taker_buy: record.is_taker_buy,
        is_buyer_maker: record.is_buyer_maker,
    }
}

fn sales_proof_symbol_json_from_record(record: SalesProofSymbolRecord) -> SalesProofSymbolJson {
    SalesProofSymbolJson {
        symbol: record.symbol,
        sample_count: record.sample_count,
        resolved_5m_samples: record.resolved_5m_samples,
        resolved_15m_samples: record.resolved_15m_samples,
        resolved_decay_samples: record.resolved_decay_samples,
        win_rate_5m: record.win_rate_5m,
        win_rate_15m: record.win_rate_15m,
        avg_return_5m: record.avg_return_5m,
        avg_return_15m: record.avg_return_15m,
        avg_decay_minutes: record.avg_decay_minutes,
        avg_trigger_score: record.avg_trigger_score,
        latest_triggered_at: record.latest_triggered_at.map(|value| value.to_rfc3339()),
        latest_triggered_ts: record
            .latest_triggered_at
            .map(|value| value.timestamp_millis()),
    }
}

fn sales_proof_signal_type_json_from_record(
    record: SalesProofSignalTypeRecord,
) -> SalesProofSignalTypeJson {
    SalesProofSignalTypeJson {
        signal_type: record.signal_type,
        sample_count: record.sample_count,
        resolved_5m_samples: record.resolved_5m_samples,
        resolved_15m_samples: record.resolved_15m_samples,
        resolved_decay_samples: record.resolved_decay_samples,
        win_rate_5m: record.win_rate_5m,
        win_rate_15m: record.win_rate_15m,
        avg_return_5m: record.avg_return_5m,
        avg_return_15m: record.avg_return_15m,
        avg_decay_minutes: record.avg_decay_minutes,
        avg_trigger_score: record.avg_trigger_score,
    }
}

pub fn big_trade_stats_json_from_record(record: BigTradeStatsRecord) -> BigTradeStatsJson {
    let total = record.total_count.max(0) as f64;
    let buy_ratio = if total > 0.0 {
        record.buy_count.max(0) as f64 / total * 100.0
    } else {
        0.0
    };
    let sell_ratio = if total > 0.0 {
        record.sell_count.max(0) as f64 / total * 100.0
    } else {
        0.0
    };
    BigTradeStatsJson {
        symbol: record.symbol,
        total_count: record.total_count,
        buy_count: record.buy_count,
        sell_count: record.sell_count,
        total_quote_quantity: record.total_quote_quantity,
        buy_quote_quantity: record.buy_quote_quantity,
        sell_quote_quantity: record.sell_quote_quantity,
        buy_ratio,
        sell_ratio,
        avg_quote_quantity: record.avg_quote_quantity,
        max_quote_quantity: record.max_quote_quantity,
        avg_threshold_quantity: record.avg_threshold_quantity,
        first_trade_time: record.first_trade_time.map(|value| value.to_rfc3339()),
        first_trade_ts: record
            .first_trade_time
            .map(|value| value.timestamp_millis()),
        last_trade_time: record.last_trade_time.map(|value| value.to_rfc3339()),
        last_trade_ts: record.last_trade_time.map(|value| value.timestamp_millis()),
    }
}

fn parse_json_value(raw: &str) -> Value {
    serde_json::from_str(raw).unwrap_or(Value::Null)
}
