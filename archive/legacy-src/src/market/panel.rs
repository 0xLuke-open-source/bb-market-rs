use std::collections::HashMap;
use std::panic::AssertUnwindSafe;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

use anyhow::Result;
use chrono::{DateTime, Utc};
use futures_util::FutureExt;
use serde::Serialize;
use tokio::sync::mpsc;
use tokio::sync::mpsc::error::TrySendError;
use uuid::Uuid;

use crate::postgres::PgPool;
use crate::terminal::application::projection::{
    EnterpriseMetricRowJson as EnterpriseMetricRow,
    EnterpriseMetricSectionJson as EnterpriseMetricSection, FactorMetricJson as FactorMetric,
    FeedEntry, KlineJson, SymbolJson,
};

const PANEL_PERSIST_INTERVAL_SECS: u64 = 2;
const PANEL_PERSIST_QUEUE_CAPACITY: usize = 4096;
const PANEL_METRICS_LOG_INTERVAL_SECS: u64 = 30;
const PANEL_METRIC_HISTORY_LIMIT: usize = 180;
const PANEL_SIGNAL_HISTORY_LIMIT: usize = 20;
const PANEL_SIGNAL_PERF_CACHE_TTL_SECS: u64 = 10;

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

#[derive(Clone)]
pub struct SymbolPanelPersistenceService {
    sender: mpsc::Sender<SymbolPanelSnapshotRecord>,
    repository: Arc<PostgresSymbolPanelSnapshotRepository>,
    state: Arc<Mutex<HashMap<String, PersistState>>>,
    derived: Arc<Mutex<HashMap<String, PanelDerivedState>>>,
    perf_cache: Arc<Mutex<HashMap<String, SignalPerfCacheEntry>>>,
    metrics: Arc<PersistMetrics>,
    min_interval: Duration,
}

#[derive(Clone)]
pub struct SymbolPanelQueryService {
    pool: Arc<PgPool>,
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
struct PersistState {
    last_update_count: i64,
    last_persisted_at: Instant,
    active_signal_type: Option<String>,
}

#[derive(Debug, Clone, Default)]
struct PanelDerivedState {
    metric_history: Vec<MetricPoint>,
}

#[derive(Debug, Clone)]
struct MetricPoint {
    t: i64,
    mid: f64,
    cvd: f64,
    tbr: f64,
    ps: f64,
    ds: f64,
    bid5: f64,
    ask5: f64,
    bid10: f64,
    ask10: f64,
    bid20: f64,
    ask20: f64,
    wall_bid: f64,
    wall_ask: f64,
    spread: f64,
    anomaly: f64,
}

#[derive(Clone)]
struct PostgresSymbolPanelSnapshotRepository {
    pool: Arc<PgPool>,
}

#[derive(Debug, Clone)]
struct SignalPerfCacheEntry {
    summary: SignalPerfSummary,
    loaded_at: Instant,
}

struct PersistMetrics {
    success_total: AtomicU64,
    filtered_total: AtomicU64,
    skipped_total: AtomicU64,
    queue_full_total: AtomicU64,
    error_total: AtomicU64,
    success_delta: AtomicU64,
    filtered_delta: AtomicU64,
    skipped_delta: AtomicU64,
    queue_full_delta: AtomicU64,
    error_delta: AtomicU64,
}

#[derive(Debug, Clone, Default)]
struct SignalPerfSummary {
    win5: f64,
    win15: f64,
    count5: usize,
    count15: usize,
    decay: f64,
}

struct WalkBookCost {
    spent: f64,
    filled_qty: f64,
    remain: f64,
}

impl SymbolPanelPersistenceService {
    pub async fn new(pool: Arc<PgPool>) -> Result<Self> {
        let repository = Arc::new(PostgresSymbolPanelSnapshotRepository::new(pool));
        repository.ensure_schema().await?;
        let perf_cache = Arc::new(Mutex::new(HashMap::new()));
        let metrics = Arc::new(PersistMetrics::default());

        let (sender, mut receiver) = mpsc::channel(PANEL_PERSIST_QUEUE_CAPACITY);
        let worker_repository = repository.clone();
        let worker_perf_cache = perf_cache.clone();
        let worker_metrics = metrics.clone();
        tokio::spawn(async move {
            while let Some(snapshot) = receiver.recv().await {
                match AssertUnwindSafe(worker_repository.insert_snapshot(&snapshot))
                    .catch_unwind()
                    .await
                {
                    Ok(Ok(summary)) => {
                        cache_signal_perf_summary(
                            &worker_perf_cache,
                            snapshot.symbol.clone(),
                            summary,
                        );
                        worker_metrics.record_success();
                    }
                    Ok(Err(err)) => {
                        worker_metrics.record_error();
                        eprintln!(
                            "symbol panel snapshot persist error [{}]: {}",
                            snapshot.symbol, err
                        );
                    }
                    Err(payload) => {
                        worker_metrics.record_error();
                        eprintln!(
                            "symbol panel snapshot persist panic [{}]: {}",
                            snapshot.symbol,
                            panic_payload_message(payload.as_ref())
                        );
                    }
                }
            }
        });

        let report_metrics = metrics.clone();
        tokio::spawn(async move {
            let mut tick =
                tokio::time::interval(Duration::from_secs(PANEL_METRICS_LOG_INTERVAL_SECS));
            loop {
                tick.tick().await;
                report_metrics.log_snapshot();
            }
        });

        Ok(Self {
            sender,
            repository,
            state: Arc::new(Mutex::new(HashMap::new())),
            derived: Arc::new(Mutex::new(HashMap::new())),
            perf_cache,
            metrics,
            min_interval: Duration::from_secs(PANEL_PERSIST_INTERVAL_SECS),
        })
    }

    pub async fn decorate_live_snapshot(
        &self,
        snapshot: &mut SymbolJson,
        signal_history: Vec<FeedEntry>,
    ) {
        let history = self.update_metric_history(snapshot);
        let perf = self
            .signal_perf_summary(&snapshot.symbol)
            .await
            .unwrap_or_default();
        let mut enterprise = build_enterprise_metrics(snapshot, &history);
        apply_signal_perf_to_enterprise_sections(&mut enterprise, &perf);

        snapshot.signal_history = signal_history
            .into_iter()
            .take(PANEL_SIGNAL_HISTORY_LIMIT)
            .collect();
        snapshot.factor_metrics = build_factor_metrics(snapshot);
        snapshot.enterprise_metrics = enterprise;
    }

    pub fn submit_snapshot(&self, snapshot: &SymbolJson, signal_history: Vec<FeedEntry>) {
        if !is_snapshot_persistable(snapshot) {
            self.metrics.record_filtered();
            return;
        }

        if !self.should_persist(snapshot) {
            self.metrics.record_skipped();
            return;
        }

        let active_signal = detect_active_signal(snapshot);
        let sample_signal_type = {
            let state = self.state.lock().expect("panel state mutex poisoned");
            let previous_active = state
                .get(&snapshot.symbol)
                .and_then(|entry| entry.active_signal_type.as_deref());
            match active_signal.as_ref() {
                Some((signal_type, _)) if previous_active != Some(signal_type.as_str()) => {
                    Some(signal_type.clone())
                }
                _ => None,
            }
        };

        let record = self.build_record(snapshot, signal_history, sample_signal_type);
        if let Err(err) = self.sender.try_send(record) {
            match err {
                TrySendError::Full(_) => {
                    self.metrics.record_queue_full();
                    eprintln!("symbol panel snapshot queue full [{}]", snapshot.symbol);
                }
                TrySendError::Closed(_) => {
                    self.metrics.record_error();
                    eprintln!(
                        "symbol panel snapshot queue closed [{}]: persistence worker exited",
                        snapshot.symbol
                    );
                }
            }
            return;
        }

        let mut state = self.state.lock().expect("panel state mutex poisoned");
        state.insert(
            snapshot.symbol.clone(),
            PersistState {
                last_update_count: snapshot.update_count.min(i64::MAX as u64) as i64,
                last_persisted_at: Instant::now(),
                active_signal_type: active_signal.map(|(signal_type, _)| signal_type),
            },
        );
    }

    async fn signal_perf_summary(&self, symbol: &str) -> Result<SignalPerfSummary> {
        {
            let cache = self
                .perf_cache
                .lock()
                .expect("panel perf cache mutex poisoned");
            if let Some(entry) = cache.get(symbol) {
                if entry.loaded_at.elapsed() < Duration::from_secs(PANEL_SIGNAL_PERF_CACHE_TTL_SECS)
                {
                    return Ok(entry.summary.clone());
                }
            }
        }

        let summary = self.repository.load_signal_perf_summary(symbol).await?;
        cache_signal_perf_summary(&self.perf_cache, symbol.to_string(), summary.clone());
        Ok(summary)
    }

    fn should_persist(&self, snapshot: &SymbolJson) -> bool {
        let update_count = snapshot.update_count.min(i64::MAX as u64) as i64;
        let state = self.state.lock().expect("panel state mutex poisoned");
        match state.get(&snapshot.symbol) {
            Some(last) if last.last_update_count == update_count => false,
            Some(last) if last.last_persisted_at.elapsed() < self.min_interval => false,
            _ => true,
        }
    }

    fn build_record(
        &self,
        snapshot: &SymbolJson,
        signal_history: Vec<FeedEntry>,
        sample_signal_type: Option<String>,
    ) -> SymbolPanelSnapshotRecord {
        let history = self.update_metric_history(snapshot);
        let factors = build_factor_metrics(snapshot);
        let enterprise = build_enterprise_metrics(snapshot, &history);
        let trimmed_signal_history = signal_history
            .into_iter()
            .take(PANEL_SIGNAL_HISTORY_LIMIT)
            .collect::<Vec<_>>();

        SymbolPanelSnapshotRecord {
            snapshot_id: Uuid::new_v4(),
            symbol: snapshot.symbol.clone(),
            event_time: Utc::now(),
            bid: snapshot.bid,
            ask: snapshot.ask,
            mid: snapshot.mid,
            spread_bps: snapshot.spread_bps,
            change_24h_pct: snapshot.change_24h_pct,
            high_24h: snapshot.high_24h,
            low_24h: snapshot.low_24h,
            volume_24h: snapshot.volume_24h,
            quote_vol_24h: snapshot.quote_vol_24h,
            ofi: snapshot.ofi,
            ofi_raw: snapshot.ofi_raw,
            obi: snapshot.obi,
            trend_strength: snapshot.trend_strength,
            cvd: snapshot.cvd,
            taker_buy_ratio: snapshot.taker_buy_ratio,
            pump_score: snapshot.pump_score as i32,
            dump_score: snapshot.dump_score as i32,
            pump_signal: snapshot.pump_signal,
            dump_signal: snapshot.dump_signal,
            whale_entry: snapshot.whale_entry,
            whale_exit: snapshot.whale_exit,
            bid_eating: snapshot.bid_eating,
            total_bid_volume: snapshot.total_bid_volume,
            total_ask_volume: snapshot.total_ask_volume,
            max_bid_ratio: snapshot.max_bid_ratio,
            max_ask_ratio: snapshot.max_ask_ratio,
            anomaly_count_1m: snapshot.anomaly_count_1m as i32,
            anomaly_max_severity: snapshot.anomaly_max_severity as i32,
            status_summary: snapshot.status_summary.clone(),
            watch_level: snapshot.watch_level.clone(),
            signal_reason: snapshot.signal_reason.clone(),
            sentiment: snapshot.sentiment.clone(),
            risk_level: snapshot.risk_level.clone(),
            recommendation: snapshot.recommendation.clone(),
            whale_type: snapshot.whale_type.clone(),
            pump_probability: snapshot.pump_probability as i32,
            price_precision: snapshot.price_precision as i32,
            quantity_precision: snapshot.quantity_precision as i32,
            snapshot_json: to_json_string(snapshot, "{}"),
            signal_history_json: to_json_string(&trimmed_signal_history, "[]"),
            factor_metrics_json: to_json_string(&factors, "[]"),
            enterprise_metrics_json: to_json_string(&enterprise, "[]"),
            update_count: snapshot.update_count.min(i64::MAX as u64) as i64,
            sample_trigger_score: sample_signal_type
                .as_deref()
                .map(|signal_type| signal_score(snapshot, signal_type)),
            sample_signal_type,
        }
    }

    fn update_metric_history(&self, snapshot: &SymbolJson) -> Vec<MetricPoint> {
        let mut derived = self.derived.lock().expect("panel derived mutex poisoned");
        let state = derived.entry(snapshot.symbol.clone()).or_default();
        let point = MetricPoint::from_snapshot(snapshot);
        match state.metric_history.last_mut() {
            Some(last) if point.t - last.t <= 4_000 => *last = point,
            _ => {
                state.metric_history.push(point);
                if state.metric_history.len() > PANEL_METRIC_HISTORY_LIMIT {
                    let overflow = state.metric_history.len() - PANEL_METRIC_HISTORY_LIMIT;
                    state.metric_history.drain(0..overflow);
                }
            }
        }
        state.metric_history.clone()
    }
}

impl SymbolPanelQueryService {
    pub async fn new(pool: Arc<PgPool>) -> Result<Self> {
        let repository = PostgresSymbolPanelSnapshotRepository::new(pool.clone());
        repository.ensure_schema().await?;
        Ok(Self { pool })
    }

    pub async fn load_recent_snapshots(
        &self,
        symbol: &str,
        limit: usize,
        from: Option<DateTime<Utc>>,
        to: Option<DateTime<Utc>>,
    ) -> Result<Vec<SymbolPanelSnapshotRecord>> {
        let client = self.pool.acquire().await?;
        let normalized_symbol = symbol.trim().to_ascii_uppercase();
        let query_limit = i64::try_from(limit).unwrap_or(i64::MAX);
        let rows = client
            .client()
            .query(
                "select
                    snapshot_id, symbol, event_time,
                    bid, ask, mid, spread_bps,
                    change_24h_pct, high_24h, low_24h, volume_24h, quote_vol_24h,
                    ofi, ofi_raw, obi, trend_strength, cvd, taker_buy_ratio,
                    pump_score, dump_score, pump_signal, dump_signal,
                    whale_entry, whale_exit, bid_eating,
                    total_bid_volume, total_ask_volume, max_bid_ratio, max_ask_ratio,
                    anomaly_count_1m, anomaly_max_severity,
                    status_summary, watch_level, signal_reason,
                    sentiment, risk_level, recommendation, whale_type, pump_probability,
                    price_precision, quantity_precision,
                    snapshot_json::text as snapshot_json,
                    signal_history_json::text as signal_history_json,
                    factor_metrics_json::text as factor_metrics_json,
                    enterprise_metrics_json::text as enterprise_metrics_json,
                    update_count
                 from market.symbol_panel_snapshot
                where symbol = $1
                  and ($2::timestamptz is null or event_time >= $2)
                  and ($3::timestamptz is null or event_time <= $3)
                order by event_time desc
                limit $4",
                &[&normalized_symbol, &from, &to, &query_limit],
            )
            .await?;

        Ok(rows
            .into_iter()
            .map(|row| SymbolPanelSnapshotRecord {
                snapshot_id: row.get("snapshot_id"),
                symbol: row.get("symbol"),
                event_time: row.get("event_time"),
                bid: row.get("bid"),
                ask: row.get("ask"),
                mid: row.get("mid"),
                spread_bps: row.get("spread_bps"),
                change_24h_pct: row.get("change_24h_pct"),
                high_24h: row.get("high_24h"),
                low_24h: row.get("low_24h"),
                volume_24h: row.get("volume_24h"),
                quote_vol_24h: row.get("quote_vol_24h"),
                ofi: row.get("ofi"),
                ofi_raw: row.get("ofi_raw"),
                obi: row.get("obi"),
                trend_strength: row.get("trend_strength"),
                cvd: row.get("cvd"),
                taker_buy_ratio: row.get("taker_buy_ratio"),
                pump_score: row.get("pump_score"),
                dump_score: row.get("dump_score"),
                pump_signal: row.get("pump_signal"),
                dump_signal: row.get("dump_signal"),
                whale_entry: row.get("whale_entry"),
                whale_exit: row.get("whale_exit"),
                bid_eating: row.get("bid_eating"),
                total_bid_volume: row.get("total_bid_volume"),
                total_ask_volume: row.get("total_ask_volume"),
                max_bid_ratio: row.get("max_bid_ratio"),
                max_ask_ratio: row.get("max_ask_ratio"),
                anomaly_count_1m: row.get("anomaly_count_1m"),
                anomaly_max_severity: row.get("anomaly_max_severity"),
                status_summary: row.get("status_summary"),
                watch_level: row.get("watch_level"),
                signal_reason: row.get("signal_reason"),
                sentiment: row.get("sentiment"),
                risk_level: row.get("risk_level"),
                recommendation: row.get("recommendation"),
                whale_type: row.get("whale_type"),
                pump_probability: row.get("pump_probability"),
                price_precision: row.get("price_precision"),
                quantity_precision: row.get("quantity_precision"),
                snapshot_json: row.get("snapshot_json"),
                signal_history_json: row.get("signal_history_json"),
                factor_metrics_json: row.get("factor_metrics_json"),
                enterprise_metrics_json: row.get("enterprise_metrics_json"),
                update_count: row.get("update_count"),
                sample_signal_type: None,
                sample_trigger_score: None,
            })
            .collect())
    }

    pub async fn load_signal_perf_samples(
        &self,
        symbol: &str,
        limit: usize,
    ) -> Result<Vec<SignalPerformanceSampleRecord>> {
        let client = self.pool.acquire().await?;
        let normalized_symbol = symbol.trim().to_ascii_uppercase();
        let query_limit = i64::try_from(limit).unwrap_or(i64::MAX);
        let rows = client
            .client()
            .query(
                "select
                    sample_id, symbol, signal_type, triggered_at, trigger_price, trigger_score,
                    watch_level, signal_reason, update_count,
                    resolved_5m, resolved_15m, resolved_decay,
                    outcome_5m_return, outcome_5m_win, outcome_5m_at,
                    outcome_15m_return, outcome_15m_win, outcome_15m_at,
                    decay_minutes, decay_at, created_at
                 from market.signal_performance_sample
                where symbol = $1
                order by triggered_at desc
                limit $2",
                &[&normalized_symbol, &query_limit],
            )
            .await?;

        Ok(rows
            .into_iter()
            .map(|row| SignalPerformanceSampleRecord {
                sample_id: row.get("sample_id"),
                symbol: row.get("symbol"),
                signal_type: row.get("signal_type"),
                triggered_at: row.get("triggered_at"),
                trigger_price: row.get("trigger_price"),
                trigger_score: row.get("trigger_score"),
                watch_level: row.get("watch_level"),
                signal_reason: row.get("signal_reason"),
                update_count: row.get("update_count"),
                resolved_5m: row.get("resolved_5m"),
                resolved_15m: row.get("resolved_15m"),
                resolved_decay: row.get("resolved_decay"),
                outcome_5m_return: row.get("outcome_5m_return"),
                outcome_5m_win: row.get("outcome_5m_win"),
                outcome_5m_at: row.get("outcome_5m_at"),
                outcome_15m_return: row.get("outcome_15m_return"),
                outcome_15m_win: row.get("outcome_15m_win"),
                outcome_15m_at: row.get("outcome_15m_at"),
                decay_minutes: row.get("decay_minutes"),
                decay_at: row.get("decay_at"),
                created_at: row.get("created_at"),
            })
            .collect())
    }
}

impl PersistMetrics {
    fn record_success(&self) {
        self.success_total.fetch_add(1, Ordering::Relaxed);
        self.success_delta.fetch_add(1, Ordering::Relaxed);
    }

    fn record_filtered(&self) {
        self.filtered_total.fetch_add(1, Ordering::Relaxed);
        self.filtered_delta.fetch_add(1, Ordering::Relaxed);
    }

    fn record_skipped(&self) {
        self.skipped_total.fetch_add(1, Ordering::Relaxed);
        self.skipped_delta.fetch_add(1, Ordering::Relaxed);
    }

    fn record_queue_full(&self) {
        self.queue_full_total.fetch_add(1, Ordering::Relaxed);
        self.queue_full_delta.fetch_add(1, Ordering::Relaxed);
    }

    fn record_error(&self) {
        self.error_total.fetch_add(1, Ordering::Relaxed);
        self.error_delta.fetch_add(1, Ordering::Relaxed);
    }

    fn log_snapshot(&self) {
        let success_delta = self.success_delta.swap(0, Ordering::Relaxed);
        let filtered_delta = self.filtered_delta.swap(0, Ordering::Relaxed);
        let skipped_delta = self.skipped_delta.swap(0, Ordering::Relaxed);
        let queue_full_delta = self.queue_full_delta.swap(0, Ordering::Relaxed);
        let error_delta = self.error_delta.swap(0, Ordering::Relaxed);

        eprintln!(
            "symbol panel persist metrics: +success={} +filtered={} +skipped={} +queue_full={} +error={} | total success={} filtered={} skipped={} queue_full={} error={}",
            success_delta,
            filtered_delta,
            skipped_delta,
            queue_full_delta,
            error_delta,
            self.success_total.load(Ordering::Relaxed),
            self.filtered_total.load(Ordering::Relaxed),
            self.skipped_total.load(Ordering::Relaxed),
            self.queue_full_total.load(Ordering::Relaxed),
            self.error_total.load(Ordering::Relaxed),
        );
    }
}

impl Default for PersistMetrics {
    fn default() -> Self {
        Self {
            success_total: AtomicU64::new(0),
            filtered_total: AtomicU64::new(0),
            skipped_total: AtomicU64::new(0),
            queue_full_total: AtomicU64::new(0),
            error_total: AtomicU64::new(0),
            success_delta: AtomicU64::new(0),
            filtered_delta: AtomicU64::new(0),
            skipped_delta: AtomicU64::new(0),
            queue_full_delta: AtomicU64::new(0),
            error_delta: AtomicU64::new(0),
        }
    }
}

impl PostgresSymbolPanelSnapshotRepository {
    fn new(pool: Arc<PgPool>) -> Self {
        Self { pool }
    }

    async fn ensure_schema(&self) -> Result<()> {
        let client = self.pool.acquire().await?;
        client
            .client()
            .batch_execute(include_str!("../../sql/postgres/market_symbol_panel.sql"))
            .await?;
        client
            .client()
            .batch_execute(include_str!(
                "../../sql/postgres/market_signal_performance.sql"
            ))
            .await?;
        client
            .client()
            .batch_execute(include_str!(
                "../../sql/postgres/market_signal_factor_detail.sql"
            ))
            .await?;
        Ok(())
    }

    async fn insert_snapshot(
        &self,
        snapshot: &SymbolPanelSnapshotRecord,
    ) -> Result<SignalPerfSummary> {
        if let Some(signal_type) = snapshot.sample_signal_type.as_deref() {
            self.insert_signal_sample(snapshot, signal_type).await?;
        }
        self.resolve_signal_samples(snapshot).await?;
        let perf_summary = self.load_signal_perf_summary(&snapshot.symbol).await?;
        let enterprise_metrics_json = apply_signal_perf_to_enterprise_metrics(
            &snapshot.enterprise_metrics_json,
            &perf_summary,
        );

        let client = self.pool.acquire().await?;
        client
            .client()
            .execute(
                "insert into market.symbol_panel_snapshot (
                    snapshot_id, symbol, event_time,
                    bid, ask, mid, spread_bps,
                    change_24h_pct, high_24h, low_24h, volume_24h, quote_vol_24h,
                    ofi, ofi_raw, obi, trend_strength, cvd, taker_buy_ratio,
                    pump_score, dump_score, pump_signal, dump_signal,
                    whale_entry, whale_exit, bid_eating,
                    total_bid_volume, total_ask_volume, max_bid_ratio, max_ask_ratio,
                    anomaly_count_1m, anomaly_max_severity,
                    status_summary, watch_level, signal_reason,
                    sentiment, risk_level, recommendation, whale_type, pump_probability,
                    price_precision, quantity_precision,
                    snapshot_json, signal_history_json, factor_metrics_json, enterprise_metrics_json,
                    update_count
                ) values (
                    $1, $2, $3,
                    $4, $5, $6, $7,
                    $8, $9, $10, $11, $12,
                    $13, $14, $15, $16, $17, $18,
                    $19, $20, $21, $22,
                    $23, $24, $25,
                    $26, $27, $28, $29,
                    $30, $31,
                    $32, $33, $34,
                    $35, $36, $37, $38, $39,
                    $40, $41,
                    cast($42 as text)::jsonb,
                    cast($43 as text)::jsonb,
                    cast($44 as text)::jsonb,
                    cast($45 as text)::jsonb,
                    $46
                )",
                &[
                    &snapshot.snapshot_id,
                    &snapshot.symbol,
                    &snapshot.event_time,
                    &snapshot.bid,
                    &snapshot.ask,
                    &snapshot.mid,
                    &snapshot.spread_bps,
                    &snapshot.change_24h_pct,
                    &snapshot.high_24h,
                    &snapshot.low_24h,
                    &snapshot.volume_24h,
                    &snapshot.quote_vol_24h,
                    &snapshot.ofi,
                    &snapshot.ofi_raw,
                    &snapshot.obi,
                    &snapshot.trend_strength,
                    &snapshot.cvd,
                    &snapshot.taker_buy_ratio,
                    &snapshot.pump_score,
                    &snapshot.dump_score,
                    &snapshot.pump_signal,
                    &snapshot.dump_signal,
                    &snapshot.whale_entry,
                    &snapshot.whale_exit,
                    &snapshot.bid_eating,
                    &snapshot.total_bid_volume,
                    &snapshot.total_ask_volume,
                    &snapshot.max_bid_ratio,
                    &snapshot.max_ask_ratio,
                    &snapshot.anomaly_count_1m,
                    &snapshot.anomaly_max_severity,
                    &snapshot.status_summary,
                    &snapshot.watch_level,
                    &snapshot.signal_reason,
                    &snapshot.sentiment,
                    &snapshot.risk_level,
                    &snapshot.recommendation,
                    &snapshot.whale_type,
                    &snapshot.pump_probability,
                    &snapshot.price_precision,
                    &snapshot.quantity_precision,
                    &snapshot.snapshot_json,
                    &snapshot.signal_history_json,
                    &snapshot.factor_metrics_json,
                    &enterprise_metrics_json,
                    &snapshot.update_count,
                ],
            )
            .await?;
        Ok(perf_summary)
    }

    async fn insert_signal_sample(
        &self,
        snapshot: &SymbolPanelSnapshotRecord,
        signal_type: &str,
    ) -> Result<()> {
        let client = self.pool.acquire().await?;
        let trigger_score = snapshot
            .sample_trigger_score
            .unwrap_or_else(|| signal_score_from_record(snapshot, signal_type));
        let sample_id = Uuid::new_v4();
        client
            .client()
            .execute(
                "insert into market.signal_performance_sample (
                    sample_id, symbol, signal_type, triggered_at, trigger_price, trigger_score,
                    watch_level, signal_reason, update_count
                 ) values ($1, $2, $3, $4, $5, $6, $7, $8, $9)",
                &[
                    &sample_id,
                    &snapshot.symbol,
                    &signal_type,
                    &snapshot.event_time,
                    &snapshot.mid,
                    &trigger_score,
                    &snapshot.watch_level,
                    &snapshot.signal_reason,
                    &snapshot.update_count,
                ],
            )
            .await?;

        // 插入因子详情（从 factor_metrics_json 展开）
        if let Err(e) = self
            .insert_signal_factor_detail(&client, sample_id, snapshot, signal_type)
            .await
        {
            eprintln!(
                "signal_factor_detail insert error [{}]: {}",
                snapshot.symbol, e
            );
        }

        Ok(())
    }

    /// 将 factor_metrics_json 里的每个因子展开写入 market.signal_factor_detail
    async fn insert_signal_factor_detail(
        &self,
        client: &crate::postgres::PooledClient,
        sample_id: Uuid,
        snapshot: &SymbolPanelSnapshotRecord,
        signal_type: &str,
    ) -> Result<()> {
        // factor_metrics_json 是 Vec<FactorMetric>，结构: [{name, value, change, signal, ...}]
        #[derive(serde::Deserialize)]
        struct FactorEntry {
            name: Option<String>,
            value: Option<serde_json::Value>,
            signal: Option<String>,
        }
        let factors: Vec<FactorEntry> =
            serde_json::from_str(&snapshot.factor_metrics_json).unwrap_or_default();
        if factors.is_empty() {
            return Ok(());
        }

        for factor in &factors {
            let name = match &factor.name {
                Some(n) => n.clone(),
                None => continue,
            };
            let raw_value: f64 = match &factor.value {
                Some(serde_json::Value::Number(n)) => n.as_f64().unwrap_or(0.0),
                Some(serde_json::Value::String(s)) => s.parse::<f64>().unwrap_or(0.0),
                _ => 0.0,
            };
            // signal_factor_detail 的 contribution_score 暂用 0（真实值在 PumpDetector 层）
            client
                .client()
                .execute(
                    "insert into market.signal_factor_detail
                        (sample_id, symbol, signal_type, triggered_at,
                         factor_name, raw_value, z_score, contribution_score)
                     values ($1, $2, $3, $4, $5, $6, null, 0.0)",
                    &[
                        &sample_id,
                        &snapshot.symbol,
                        &signal_type,
                        &snapshot.event_time,
                        &name,
                        &raw_value,
                    ],
                )
                .await?;
        }
        Ok(())
    }

    async fn resolve_signal_samples(&self, snapshot: &SymbolPanelSnapshotRecord) -> Result<()> {
        let client = self.pool.acquire().await?;
        let rows = client
            .client()
            .query(
                "select
                    sample_id, signal_type, triggered_at, trigger_price,
                    resolved_5m, resolved_15m, resolved_decay
                 from market.signal_performance_sample
                where symbol = $1
                  and (resolved_5m = false or resolved_15m = false or resolved_decay = false)
                order by triggered_at asc",
                &[&snapshot.symbol],
            )
            .await?;

        let active_signal_type = detect_active_signal_from_record(snapshot);
        for row in rows {
            let sample_id: Uuid = row.get("sample_id");
            let signal_type: String = row.get("signal_type");
            let triggered_at: DateTime<Utc> = row.get("triggered_at");
            let trigger_price: f64 = row.get("trigger_price");
            let resolved_5m: bool = row.get("resolved_5m");
            let resolved_15m: bool = row.get("resolved_15m");
            let resolved_decay: bool = row.get("resolved_decay");
            let elapsed_secs = (snapshot.event_time - triggered_at).num_seconds();
            if elapsed_secs < 0 {
                continue;
            }

            if !resolved_5m && elapsed_secs >= 5 * 60 {
                let ret = calc_signal_return(trigger_price, snapshot.mid, &signal_type);
                let win = is_signal_win(&signal_type, ret);
                client
                    .client()
                    .execute(
                        "update market.signal_performance_sample
                            set resolved_5m = true,
                                outcome_5m_return = $2,
                                outcome_5m_win = $3,
                                outcome_5m_at = $4
                          where sample_id = $1",
                        &[&sample_id, &ret, &win, &snapshot.event_time],
                    )
                    .await?;
            }

            if !resolved_15m && elapsed_secs >= 15 * 60 {
                let ret = calc_signal_return(trigger_price, snapshot.mid, &signal_type);
                let win = is_signal_win(&signal_type, ret);
                client
                    .client()
                    .execute(
                        "update market.signal_performance_sample
                            set resolved_15m = true,
                                outcome_15m_return = $2,
                                outcome_15m_win = $3,
                                outcome_15m_at = $4
                          where sample_id = $1",
                        &[&sample_id, &ret, &win, &snapshot.event_time],
                    )
                    .await?;
            }

            if !resolved_decay && active_signal_type.as_deref() != Some(signal_type.as_str()) {
                let decay_minutes = elapsed_secs as f64 / 60.0;
                client
                    .client()
                    .execute(
                        "update market.signal_performance_sample
                            set resolved_decay = true,
                                decay_minutes = $2,
                                decay_at = $3
                          where sample_id = $1",
                        &[&sample_id, &decay_minutes, &snapshot.event_time],
                    )
                    .await?;
            }
        }

        Ok(())
    }

    async fn load_signal_perf_summary(&self, symbol: &str) -> Result<SignalPerfSummary> {
        let client = self.pool.acquire().await?;
        let row = client
            .client()
            .query_one(
                "select
                    coalesce(avg(case
                        when resolved_5m and outcome_5m_win then 100.0::double precision
                        when resolved_5m and outcome_5m_win = false then 0.0::double precision
                    end), 0.0::double precision) as win5,
                    count(*) filter (where resolved_5m) as count5,
                    coalesce(avg(case
                        when resolved_15m and outcome_15m_win then 100.0::double precision
                        when resolved_15m and outcome_15m_win = false then 0.0::double precision
                    end), 0.0::double precision) as win15,
                    count(*) filter (where resolved_15m) as count15,
                    coalesce(avg(decay_minutes), 0.0::double precision) as decay
                 from market.signal_performance_sample
                where symbol = $1",
                &[&symbol],
            )
            .await?;
        Ok(SignalPerfSummary {
            win5: row.try_get::<_, f64>("win5")?,
            count5: i64_to_usize(row.try_get::<_, i64>("count5")?),
            win15: row.try_get::<_, f64>("win15")?,
            count15: i64_to_usize(row.try_get::<_, i64>("count15")?),
            decay: row.try_get::<_, f64>("decay")?,
        })
    }
}

impl MetricPoint {
    fn from_snapshot(snapshot: &SymbolJson) -> Self {
        Self {
            t: Utc::now().timestamp_millis(),
            mid: snapshot.mid,
            cvd: snapshot.cvd,
            tbr: snapshot.taker_buy_ratio,
            ps: snapshot.pump_score as f64,
            ds: snapshot.dump_score as f64,
            bid5: depth_totals(&snapshot.top_bids, 5),
            ask5: depth_totals(&snapshot.top_asks, 5),
            bid10: depth_totals(&snapshot.top_bids, 10),
            ask10: depth_totals(&snapshot.top_asks, 10),
            bid20: depth_totals(&snapshot.top_bids, 20),
            ask20: depth_totals(&snapshot.top_asks, 20),
            wall_bid: snapshot.max_bid_ratio,
            wall_ask: snapshot.max_ask_ratio,
            spread: snapshot.spread_bps,
            anomaly: snapshot.anomaly_max_severity as f64,
        }
    }
}

fn is_snapshot_persistable(snapshot: &SymbolJson) -> bool {
    snapshot.update_count > 0 && !snapshot.symbol.is_empty() && snapshot.mid > 0.0
}

fn to_json_string<T: Serialize>(value: &T, fallback: &str) -> String {
    serde_json::to_string(value).unwrap_or_else(|_| fallback.to_string())
}

fn build_factor_metrics(snapshot: &SymbolJson) -> Vec<FactorMetric> {
    let ps = snapshot.pump_score as f64;
    let ds = snapshot.dump_score as f64;
    let obi = snapshot.obi;
    let tbr = snapshot.taker_buy_ratio;
    let cvd = snapshot.cvd;
    let ofi = snapshot.ofi;
    let spread = snapshot.spread_bps;
    let anomaly = snapshot.anomaly_count_1m as f64;
    vec![
        FactorMetric {
            name: "上涨动能".into(),
            value: format!("{}/100", ps.round() as i64),
            score: clamp(ps, 0.0, 100.0),
            tip: if ps >= 70.0 {
                "上涨力量很强"
            } else if ps >= 60.0 {
                "上涨信号明显"
            } else if ps >= 30.0 {
                "略偏强"
            } else {
                "暂不明显"
            }
            .into(),
            tone: if ps >= 60.0 {
                "fg"
            } else if ps >= 30.0 {
                "fy"
            } else {
                "fn"
            }
            .into(),
        },
        FactorMetric {
            name: "下跌压力".into(),
            value: format!("{}/100", ds.round() as i64),
            score: clamp(ds, 0.0, 100.0),
            tip: if ds >= 70.0 {
                "下跌压力很强"
            } else if ds >= 60.0 {
                "回落风险偏高"
            } else if ds >= 30.0 {
                "略偏弱"
            } else {
                "暂不明显"
            }
            .into(),
            tone: if ds >= 60.0 {
                "fr"
            } else if ds >= 30.0 {
                "fy"
            } else {
                "fn"
            }
            .into(),
        },
        FactorMetric {
            name: "买卖盘失衡".into(),
            value: format!("{}{:.1}%", if obi >= 0.0 { "+" } else { "" }, obi),
            score: clamp(obi.abs() * 2.0, 0.0, 100.0),
            tip: if obi > 20.0 {
                "买盘明显压过卖盘"
            } else if obi > 10.0 {
                "买盘偏多"
            } else if obi < -20.0 {
                "卖盘明显压过买盘"
            } else if obi < -10.0 {
                "卖盘偏多"
            } else {
                "买卖比较均衡"
            }
            .into(),
            tone: if obi > 10.0 {
                "fg"
            } else if obi < -10.0 {
                "fr"
            } else {
                "fn"
            }
            .into(),
        },
        FactorMetric {
            name: "主动买入占比".into(),
            value: format!("{:.1}%", tbr),
            score: clamp(tbr, 0.0, 100.0),
            tip: if tbr > 70.0 {
                "主动买入很强"
            } else if tbr > 60.0 {
                "偏多"
            } else if tbr < 30.0 {
                "主动卖出很强"
            } else {
                "偏空"
            }
            .into(),
            tone: if tbr > 60.0 {
                "fg"
            } else if tbr < 40.0 {
                "fr"
            } else {
                "fy"
            }
            .into(),
        },
        FactorMetric {
            name: "主动买卖量差".into(),
            value: format_compact(cvd),
            score: clamp(cvd.abs() / 500.0, 0.0, 100.0),
            tip: if cvd > 50_000.0 {
                "大量净流入"
            } else if cvd > 0.0 {
                "净买入"
            } else if cvd < -50_000.0 {
                "大量净流出"
            } else {
                "净卖出"
            }
            .into(),
            tone: if cvd > 0.0 { "fg" } else { "fr" }.into(),
        },
        FactorMetric {
            name: "挂单变化强度".into(),
            value: format_compact(ofi),
            score: clamp(ofi.abs() / 100.0, 0.0, 100.0),
            tip: if ofi > 5_000.0 {
                "买方挂单明显增强"
            } else if ofi > 2_000.0 {
                "买方在持续加单"
            } else if ofi < -5_000.0 {
                "卖方挂单明显增强"
            } else {
                "买卖挂单较平衡"
            }
            .into(),
            tone: if ofi > 3_000.0 {
                "fg"
            } else if ofi < -3_000.0 {
                "fr"
            } else {
                "fn"
            }
            .into(),
        },
        FactorMetric {
            name: "买卖价差".into(),
            value: format!("{:.2}%", spread / 100.0),
            score: clamp(spread * 3.0, 0.0, 100.0),
            tip: if spread < 10.0 {
                "成交环境很好"
            } else if spread < 20.0 {
                "正常"
            } else {
                "价差偏大"
            }
            .into(),
            tone: if spread < 10.0 {
                "fg"
            } else if spread < 30.0 {
                "fy"
            } else {
                "fn"
            }
            .into(),
        },
        FactorMetric {
            name: "大户资金".into(),
            value: if snapshot.whale_entry {
                "进场".into()
            } else if snapshot.whale_exit {
                "离场".into()
            } else {
                "观望".into()
            },
            score: if snapshot.whale_entry {
                80.0
            } else if snapshot.whale_exit {
                60.0
            } else {
                20.0
            },
            tip: if snapshot.whale_entry {
                format!("大单占比{:.1}%", snapshot.max_bid_ratio)
            } else if snapshot.whale_exit {
                "大户有离场迹象".into()
            } else {
                "暂无明显动作".into()
            },
            tone: if snapshot.whale_entry {
                "fg"
            } else if snapshot.whale_exit {
                "fr"
            } else {
                "fn"
            }
            .into(),
        },
        FactorMetric {
            name: "异常波动".into(),
            value: format!("{}次", snapshot.anomaly_count_1m),
            score: clamp(anomaly, 0.0, 100.0),
            tip: if snapshot.anomaly_count_1m > 200 {
                "波动非常剧烈"
            } else if snapshot.anomaly_count_1m > 50 {
                "波动偏多"
            } else {
                "整体平稳"
            }
            .into(),
            tone: if snapshot.anomaly_count_1m > 100 {
                "fr"
            } else if snapshot.anomaly_count_1m > 50 {
                "fy"
            } else {
                "fn"
            }
            .into(),
        },
    ]
}

fn build_enterprise_metrics(
    snapshot: &SymbolJson,
    history: &[MetricPoint],
) -> Vec<EnterpriseMetricSection> {
    let last = history.last();
    let prev = history.get(history.len().saturating_sub(6));
    let bars1m = get_bars(snapshot, "1m", 20);
    let bars5m = get_bars(snapshot, "5m", 20);
    let bars15m = get_bars(snapshot, "15m", 20);
    let bars1h = get_bars(snapshot, "1h", 20);
    let cur1m = snapshot.current_kline.get("1m");
    let current_px = snapshot.mid;
    let now = Utc::now().timestamp_millis() as u64;
    let recent_big = snapshot
        .big_trades
        .iter()
        .filter(|trade| now.saturating_sub(trade.t) <= 60_000)
        .cloned()
        .collect::<Vec<_>>();
    let prev_big = snapshot
        .big_trades
        .iter()
        .filter(|trade| {
            let age = now.saturating_sub(trade.t);
            age > 60_000 && age <= 120_000
        })
        .cloned()
        .collect::<Vec<_>>();
    let big_recent_notional = sum(recent_big.iter().map(|trade| trade.p * trade.q));
    let big_prev_notional = sum(prev_big.iter().map(|trade| trade.p * trade.q));
    let est_minute_quote = snapshot.quote_vol_24h / 1440.0;
    let large_trade_ratio = pct(big_recent_notional, est_minute_quote.max(1.0));
    let continuity = 100.0
        - clamp(
            avg(history
                .iter()
                .rev()
                .take(8)
                .map(|item| ((item.tbr - 50.0).abs()) * 2.0)),
            0.0,
            100.0,
        );
    let directional_continuity = (avg(history
        .iter()
        .rev()
        .take(8)
        .map(|item| (item.tbr - 50.0) * 2.0)))
    .abs();
    let trade_density = clamp(recent_big.len() as f64 * 16.0, 0.0, 100.0);
    let count_surge = if prev_big.is_empty() {
        clamp(recent_big.len() as f64 * 20.0, 0.0, 100.0)
    } else {
        clamp(
            recent_big.len() as f64 / prev_big.len() as f64 * 25.0,
            0.0,
            100.0,
        )
    };
    let amount_surge = if big_prev_notional > 0.0 {
        clamp(big_recent_notional / big_prev_notional * 25.0, 0.0, 100.0)
    } else {
        clamp(
            big_recent_notional / est_minute_quote.max(1.0) * 100.0,
            0.0,
            100.0,
        )
    };
    let wall_strength = clamp(
        snapshot.max_bid_ratio.max(snapshot.max_ask_ratio) * 2.2,
        0.0,
        100.0,
    );
    let cancel_ratio_est = clamp(
        snapshot.ofi_raw.abs() / (snapshot.ofi.abs() + 1.0) * 30.0,
        0.0,
        100.0,
    );
    let recovery_speed = clamp(
        100.0 - (snapshot.spread_bps * 2.0)
            + (metric_delta_pct(
                last.map(|item| item.bid10).unwrap_or(0.0),
                prev.map(|item| item.bid10).unwrap_or(0.0),
            )
            .abs()
                * 0.3)
                .min(30.0),
        0.0,
        100.0,
    );
    let depth5_delta = metric_delta_pct(
        last.map(|item| item.bid5 + item.ask5).unwrap_or(0.0),
        prev.map(|item| item.bid5 + item.ask5).unwrap_or(0.0),
    );
    let depth10_delta = metric_delta_pct(
        last.map(|item| item.bid10 + item.ask10).unwrap_or(0.0),
        prev.map(|item| item.bid10 + item.ask10).unwrap_or(0.0),
    );
    let depth20_delta = metric_delta_pct(
        last.map(|item| item.bid20 + item.ask20).unwrap_or(0.0),
        prev.map(|item| item.bid20 + item.ask20).unwrap_or(0.0),
    );
    let depth_gap_bps = depth_gap(&snapshot.top_bids).max(depth_gap(&snapshot.top_asks));
    let ret1 = calc_return_pct(&bars1m[bars1m.len().saturating_sub(5)..]);
    let ret5 = calc_return_pct(&bars5m[bars5m.len().saturating_sub(6)..]);
    let ret15 = calc_return_pct(&bars15m[bars15m.len().saturating_sub(6)..]);
    let ret60 = calc_return_pct(&bars1h[bars1h.len().saturating_sub(6)..]);
    let same_dir = [ret1, ret5, ret15, ret60]
        .into_iter()
        .filter(|value| *value != 0.0)
        .collect::<Vec<_>>();
    let multi_tf_consistency = if same_dir.is_empty() {
        0.0
    } else {
        clamp(
            (sum(same_dir
                .iter()
                .map(|value| value.signum())
                .collect::<Vec<_>>())
            .abs()
                / same_dir.len() as f64)
                * 100.0,
            0.0,
            100.0,
        )
    };
    let range_now = cur1m
        .map(|bar| ((bar.h - bar.l) / (if bar.o == 0.0 { 1.0 } else { bar.o })) * 100.0)
        .unwrap_or(0.0);
    let range_avg = avg(bars1m
        .iter()
        .rev()
        .take(10)
        .map(|bar| ((bar.h - bar.l) / (if bar.o == 0.0 { 1.0 } else { bar.o })) * 100.0));
    let vol_expand = if range_avg > 0.0 {
        clamp(range_now / range_avg * 35.0, 0.0, 100.0)
    } else {
        0.0
    };
    let prev_high = bars1m
        .iter()
        .rev()
        .take(12)
        .map(|bar| bar.h)
        .fold(0.0, f64::max);
    let prev_low = bars1m
        .iter()
        .rev()
        .take(12)
        .map(|bar| bar.l)
        .fold(f64::MAX, f64::min);
    let false_break = (current_px > prev_high
        && cur1m.map(|bar| bar.c).unwrap_or(current_px) < prev_high)
        || (current_px < prev_low && cur1m.map(|bar| bar.c).unwrap_or(current_px) > prev_low);
    let recent_spike_base = bars1m
        .iter()
        .rev()
        .take(10)
        .last()
        .map(|bar| bar.o)
        .unwrap_or(current_px);
    let recent_extreme = bars1m
        .iter()
        .rev()
        .take(10)
        .map(|bar| ((bar.h - recent_spike_base) / recent_spike_base.max(1.0)).abs() * 100.0)
        .fold(0.0, f64::max);
    let pullback = (recent_extreme
        - ((current_px - recent_spike_base) / recent_spike_base.max(1.0)).abs() * 100.0)
        .abs();
    let acceptance = if current_px >= prev_high * 0.998 || current_px <= prev_low * 1.002 {
        clamp(snapshot.taker_buy_ratio, 0.0, 100.0)
    } else {
        45.0
    };
    let accumulation =
        if (if snapshot.cvd > 0.0 { 1.0 } else { -1.0 }) * (snapshot.taker_buy_ratio - 50.0) >= 0.0
            && snapshot.obi > 0.0
        {
            78.0
        } else if snapshot.cvd < 0.0 && snapshot.obi < 0.0 {
            25.0
        } else {
            52.0
        };
    let whale_follow = if snapshot.whale_entry {
        clamp(
            (snapshot.pump_score as f64 + snapshot.taker_buy_ratio) / 1.5,
            0.0,
            100.0,
        )
    } else if snapshot.whale_exit {
        clamp(
            (snapshot.dump_score as f64 + (100.0 - snapshot.taker_buy_ratio)) / 1.5,
            0.0,
            100.0,
        )
    } else {
        45.0
    };
    let wall_dwell = if history.len() >= 3 {
        clamp(
            avg(history
                .iter()
                .rev()
                .take(6)
                .map(|item| item.wall_bid.max(item.wall_ask)))
                * 2.0,
            0.0,
            100.0,
        )
    } else {
        0.0
    };
    let cvd_slope = if history.len() >= 2 {
        let prev_cvd = prev.map(|item| item.cvd).unwrap_or(0.0);
        let last_cvd = last.map(|item| item.cvd).unwrap_or(0.0);
        ((last_cvd - prev_cvd) / prev_cvd.abs().max(1000.0)) * 100.0
    } else {
        0.0
    };
    let resonance = clamp(
        (multi_tf_consistency + snapshot.pump_score.max(snapshot.dump_score) as f64) / 2.0,
        0.0,
        100.0,
    );
    let confirmation = if ret1.abs() > 0.2 && ret5.abs() > 0.2 && ret1.signum() == ret5.signum() {
        78.0
    } else {
        38.0
    };
    let spread_level = clamp(100.0 - snapshot.spread_bps * 2.5, 0.0, 100.0);
    let book_impact = walk_book_cost(&snapshot.top_asks, 1000.0);
    let buy_avg_px = if book_impact.filled_qty > 0.0 {
        book_impact.spent / book_impact.filled_qty
    } else {
        current_px
    };
    let slippage_risk = ((buy_avg_px - current_px).abs() / current_px.max(1.0)) * 10_000.0;
    let executable_depth = clamp((1.0 - (book_impact.remain / 1000.0)) * 100.0, 0.0, 100.0);
    let liquidity_warning = clamp(
        snapshot.spread_bps * 1.6
            + (100.0 - executable_depth)
            + snapshot.anomaly_max_severity as f64 * 0.35,
        0.0,
        100.0,
    );
    let perf = SignalPerfSummary {
        win5: 0.0,
        win15: 0.0,
        count5: 0,
        count15: 0,
        decay: 0.0,
    };

    vec![
        EnterpriseMetricSection {
            title: "成交结构".into(),
            subtitle: "大单与主动成交".into(),
            items: vec![
                metric_row(
                    "大单成交占比",
                    clamp(large_trade_ratio, 0.0, 100.0),
                    fmt_metric_value(large_trade_ratio, "%"),
                    "最近1分钟大单成交额，相对日均每分钟成交额的占比。",
                    false,
                ),
                metric_row(
                    "买卖连续性",
                    clamp(directional_continuity, 0.0, 100.0),
                    fmt_metric_value(directional_continuity, "%"),
                    "主动买卖方向是否持续偏向单边，越高说明延续性越强。",
                    false,
                ),
                metric_row(
                    "短时成交密度",
                    trade_density,
                    fmt_metric_value(recent_big.len() as f64, "count"),
                    &format!("最近1分钟捕捉到 {} 笔大额成交。", recent_big.len()),
                    false,
                ),
                metric_row(
                    "笔数突变",
                    count_surge,
                    fmt_metric_value(recent_big.len() as f64 - prev_big.len() as f64, "count"),
                    "对比前1分钟，大额成交笔数的变化幅度。",
                    false,
                ),
                metric_row(
                    "成交额突变",
                    amount_surge,
                    fmt_metric_value(big_recent_notional, "compact"),
                    "对比前1分钟，大额成交额的变化幅度。",
                    false,
                ),
            ],
        },
        EnterpriseMetricSection {
            title: "盘口结构".into(),
            subtitle: "深度与挂单质量".into(),
            items: vec![
                metric_row(
                    "买卖墙强度",
                    wall_strength,
                    fmt_metric_value(snapshot.max_bid_ratio.max(snapshot.max_ask_ratio), "%"),
                    "前排深度中大额挂单的占比，越高说明墙体越明显。",
                    false,
                ),
                metric_row(
                    "挂撤单比",
                    cancel_ratio_est,
                    fmt_metric_value(cancel_ratio_est, "%"),
                    "根据深度净变化估算撤单与改单的活跃程度。",
                    true,
                ),
                metric_row(
                    "恢复速度",
                    recovery_speed,
                    fmt_metric_value(recovery_speed, "%"),
                    "盘口深度被打掉后，重新回补的速度。",
                    false,
                ),
                metric_row(
                    "前5/10/20档变化",
                    clamp(
                        (depth5_delta.abs() + depth10_delta.abs() + depth20_delta.abs()) / 3.0,
                        0.0,
                        100.0,
                    ),
                    format!(
                        "{:.0} / {:.0} / {:.0}%",
                        depth5_delta, depth10_delta, depth20_delta
                    ),
                    "对比约 20 到 30 秒前，前排深度的变化情况。",
                    false,
                ),
                metric_row(
                    "深度断层",
                    clamp(depth_gap_bps / 2.0, 0.0, 100.0),
                    fmt_metric_value(depth_gap_bps, "bps"),
                    "相邻档位之间的跳空程度，越高说明深度越不连续。",
                    true,
                ),
            ],
        },
        EnterpriseMetricSection {
            title: "价格行为".into(),
            subtitle: "多周期价格状态".into(),
            items: vec![
                metric_row(
                    "多周期一致性",
                    multi_tf_consistency,
                    fmt_metric_value(multi_tf_consistency, "%"),
                    "1m、5m、15m、1h 几个周期的方向一致程度。",
                    false,
                ),
                metric_row(
                    "波动扩张/收缩",
                    vol_expand,
                    fmt_metric_value(vol_expand, "%"),
                    "当前 1m 波动相对最近 10 根 1m 的放大程度。",
                    false,
                ),
                metric_row(
                    "假突破识别",
                    if false_break { 82.0 } else { 28.0 },
                    if false_break {
                        "疑似假突破"
                    } else {
                        "暂未发现"
                    }
                    .into(),
                    "破位后快速收回时，追单风险通常会更高。",
                    false_break,
                ),
                metric_row(
                    "回吐幅度",
                    clamp(pullback * 10.0, 0.0, 100.0),
                    fmt_metric_value(pullback, "%"),
                    "急拉或急砸之后，价格已经回吐的幅度。",
                    true,
                ),
                metric_row(
                    "新高/低承接",
                    acceptance,
                    fmt_metric_value(acceptance, "%"),
                    "接近新高或新低时，是否仍有主动资金承接。",
                    false,
                ),
            ],
        },
        EnterpriseMetricSection {
            title: "资金痕迹".into(),
            subtitle: "吸筹、派发与大户跟随".into(),
            items: vec![
                metric_row(
                    "持续吸筹/派发",
                    accumulation,
                    if snapshot.cvd >= 0.0 {
                        "偏吸筹"
                    } else {
                        "偏派发"
                    }
                    .into(),
                    "结合 CVD、买入占比和盘口失衡做出的综合判断。",
                    false,
                ),
                metric_row(
                    "大户跟随强度",
                    whale_follow,
                    fmt_metric_value(whale_follow, "%"),
                    "大户信号出现后，盘口和主动成交是否继续跟随。",
                    false,
                ),
                metric_row(
                    "大单停留时间",
                    wall_dwell,
                    fmt_metric_value(wall_dwell, "%"),
                    "大墙挂单在前排停留的时长估算。",
                    false,
                ),
                metric_row(
                    "主动买卖量差斜率",
                    clamp(cvd_slope.abs(), 0.0, 100.0),
                    fmt_metric_value(cvd_slope, "%"),
                    "主动买卖量差的增长或衰减速度。",
                    false,
                ),
            ],
        },
        EnterpriseMetricSection {
            title: "跨周期指标".into(),
            subtitle: "信号共振与确认".into(),
            items: vec![
                metric_row(
                    "1m/5m/15m/1h共振",
                    resonance,
                    fmt_metric_value(resonance, "%"),
                    "短中周期信号是否同时偏向同一方向。",
                    false,
                ),
                metric_row(
                    "短期获中期确认",
                    confirmation,
                    if confirmation >= 60.0 {
                        "已确认"
                    } else {
                        "待确认"
                    }
                    .into(),
                    "短周期异动是否已经获得 5m / 15m 的方向确认。",
                    false,
                ),
            ],
        },
        EnterpriseMetricSection {
            title: "交易质量".into(),
            subtitle: "可成交性与流动性".into(),
            items: vec![
                metric_row(
                    "点差水平",
                    spread_level,
                    fmt_metric_value(snapshot.spread_bps, "bps"),
                    "点差越小，短线执行环境通常越友好。",
                    false,
                ),
                metric_row(
                    "深度可成交性",
                    executable_depth,
                    fmt_metric_value(executable_depth, "%"),
                    "按 1000 USDT 吃单试算，盘口可立即承接的程度。",
                    false,
                ),
                metric_row(
                    "滑点风险估计",
                    clamp(slippage_risk * 6.0, 0.0, 100.0),
                    fmt_metric_value(slippage_risk, "bps"),
                    "按 1000 USDT 吃单估算出来的滑点水平。",
                    true,
                ),
                metric_row(
                    "流动性恶化预警",
                    liquidity_warning,
                    if liquidity_warning >= 70.0 {
                        "偏高"
                    } else {
                        "正常"
                    }
                    .into(),
                    "结合点差、可成交深度和异常波动得出的综合风险。",
                    true,
                ),
            ],
        },
        EnterpriseMetricSection {
            title: "信号质量".into(),
            subtitle: "最近触发后的表现".into(),
            items: vec![
                metric_row(
                    "过去5分钟表现",
                    perf.win5,
                    if perf.count5 > 0 {
                        format!("{:.0}%", perf.win5)
                    } else {
                        "样本少".into()
                    },
                    "信号触发后 5 分钟内，方向判断的正确率。",
                    false,
                ),
                metric_row(
                    "过去15分钟表现",
                    perf.win15,
                    if perf.count15 > 0 {
                        format!("{:.0}%", perf.win15)
                    } else {
                        "样本少".into()
                    },
                    "信号触发后 15 分钟内，方向判断的正确率。",
                    false,
                ),
                metric_row(
                    "胜率/误报率",
                    perf.win15.max(perf.win5),
                    if perf.count15 > 0 {
                        format!("{:.0} / {:.0}%", perf.win15, 100.0 - perf.win15)
                    } else {
                        "待积累".into()
                    },
                    "胜率越高说明更稳定，误报率越低说明噪音更少。",
                    false,
                ),
                metric_row(
                    "信号衰减速度",
                    clamp(100.0 - perf.decay * 6.0, 0.0, 100.0),
                    if perf.decay > 0.0 {
                        format!("{:.1} 分钟", perf.decay)
                    } else {
                        "待积累".into()
                    },
                    "信号从强提醒回落到普通关注所需的平均时间。",
                    false,
                ),
            ],
        },
        EnterpriseMetricSection {
            title: "上下文".into(),
            subtitle: "面板原始数据留存".into(),
            items: vec![
                metric_row(
                    "连续性稳定度",
                    continuity,
                    fmt_metric_value(continuity, "%"),
                    "最近若干个快照中，主动成交占比的稳定程度。",
                    false,
                ),
                metric_row(
                    "价格精度",
                    clamp(snapshot.price_precision as f64 * 10.0, 0.0, 100.0),
                    snapshot.price_precision.to_string(),
                    "格式化价格时使用的精度，便于历史回放一致渲染。",
                    false,
                ),
                metric_row(
                    "数量精度",
                    clamp(snapshot.quantity_precision as f64 * 10.0, 0.0, 100.0),
                    snapshot.quantity_precision.to_string(),
                    "格式化数量时使用的精度，便于历史回放一致渲染。",
                    false,
                ),
                metric_row(
                    "历史快照样本",
                    clamp(history.len() as f64, 0.0, 100.0),
                    history.len().to_string(),
                    "企业级指标计算已经使用的本地历史样本数。",
                    false,
                ),
            ],
        },
    ]
}

fn metric_row(
    name: &str,
    score: f64,
    value: String,
    tip: &str,
    invert: bool,
) -> EnterpriseMetricRow {
    EnterpriseMetricRow {
        name: name.into(),
        score: clamp(score, 0.0, 100.0),
        value,
        tip: tip.into(),
        invert,
    }
}

fn depth_totals(levels: &[[f64; 2]], n: usize) -> f64 {
    levels.iter().take(n).map(|[price, qty]| price * qty).sum()
}

fn depth_gap(levels: &[[f64; 2]]) -> f64 {
    let list = levels
        .iter()
        .take(8)
        .map(|level| level[0])
        .filter(|price| *price > 0.0)
        .collect::<Vec<_>>();
    if list.len() < 3 {
        return 0.0;
    }
    let mut max_gap: f64 = 0.0;
    for idx in 1..list.len() {
        let prev = list[idx - 1];
        let cur = list[idx];
        max_gap = max_gap.max(((cur - prev).abs() / prev.max(1.0)) * 10_000.0);
    }
    max_gap
}

fn get_bars(snapshot: &SymbolJson, interval: &str, count: usize) -> Vec<KlineJson> {
    let bars = snapshot.klines.get(interval).cloned().unwrap_or_default();
    bars.into_iter()
        .rev()
        .take(count)
        .collect::<Vec<_>>()
        .into_iter()
        .rev()
        .collect()
}

fn calc_return_pct(bars: &[KlineJson]) -> f64 {
    if bars.len() < 2 {
        return 0.0;
    }
    let first = if bars[0].o != 0.0 {
        bars[0].o
    } else {
        bars[0].c
    };
    let last = bars.last().map(|bar| bar.c).unwrap_or(0.0);
    if first == 0.0 {
        0.0
    } else {
        (last - first) / first * 100.0
    }
}

fn walk_book_cost(levels: &[[f64; 2]], notional_target: f64) -> WalkBookCost {
    let mut remain = notional_target;
    let mut filled_qty = 0.0;
    let mut spent = 0.0;
    for [price, quantity] in levels {
        if *price <= 0.0 || *quantity <= 0.0 {
            continue;
        }
        let level_notional = price * quantity;
        let take = remain.min(level_notional);
        spent += take;
        filled_qty += take / price;
        remain -= take;
        if remain <= 0.0 {
            break;
        }
    }
    WalkBookCost {
        spent,
        filled_qty,
        remain,
    }
}

fn fmt_metric_value(value: f64, unit: &str) -> String {
    if !value.is_finite() {
        return "--".into();
    }
    match unit {
        "%" => format!("{:.1}%", value),
        "x" => format!("{:.2}x", value),
        "bps" => format!("{:.2}%", value / 100.0),
        "count" => format!("{}", value.round() as i64),
        "ratio" => format!("{:.2}", value),
        "compact" => format_compact(value),
        _ => format!("{:.2}{}", value, unit),
    }
}

fn format_compact(value: f64) -> String {
    let abs = value.abs();
    if abs >= 1_000_000_000.0 {
        format!("{:.2}B", value / 1_000_000_000.0)
    } else if abs >= 1_000_000.0 {
        format!("{:.2}M", value / 1_000_000.0)
    } else if abs >= 1_000.0 {
        format!("{:.2}K", value / 1_000.0)
    } else {
        format!("{:.2}", value)
    }
}

fn pct(value: f64, total: f64) -> f64 {
    if total > 0.0 {
        value / total * 100.0
    } else {
        0.0
    }
}

fn clamp(value: f64, min: f64, max: f64) -> f64 {
    value.max(min).min(max)
}

fn avg<I>(iter: I) -> f64
where
    I: IntoIterator<Item = f64>,
{
    let values = iter.into_iter().collect::<Vec<_>>();
    if values.is_empty() {
        0.0
    } else {
        values.iter().sum::<f64>() / values.len() as f64
    }
}

fn sum<I>(iter: I) -> f64
where
    I: IntoIterator<Item = f64>,
{
    iter.into_iter().sum()
}

fn metric_delta_pct(current: f64, previous: f64) -> f64 {
    if previous == 0.0 {
        0.0
    } else {
        (current - previous) / previous * 100.0
    }
}

fn detect_active_signal(snapshot: &SymbolJson) -> Option<(String, i32)> {
    if snapshot.pump_signal || snapshot.pump_score >= 70 {
        Some(("pump".into(), snapshot.pump_score as i32))
    } else if snapshot.dump_signal || snapshot.dump_score >= 70 {
        Some(("dump".into(), snapshot.dump_score as i32))
    } else {
        None
    }
}

fn detect_active_signal_from_record(snapshot: &SymbolPanelSnapshotRecord) -> Option<String> {
    if snapshot.pump_signal || snapshot.pump_score >= 70 {
        Some("pump".into())
    } else if snapshot.dump_signal || snapshot.dump_score >= 70 {
        Some("dump".into())
    } else {
        None
    }
}

fn signal_score(snapshot: &SymbolJson, signal_type: &str) -> i32 {
    match signal_type {
        "pump" => snapshot.pump_score as i32,
        "dump" => snapshot.dump_score as i32,
        _ => 0,
    }
}

fn signal_score_from_record(snapshot: &SymbolPanelSnapshotRecord, signal_type: &str) -> i32 {
    match signal_type {
        "pump" => snapshot.pump_score,
        "dump" => snapshot.dump_score,
        _ => 0,
    }
}

fn calc_signal_return(trigger_price: f64, current_price: f64, signal_type: &str) -> f64 {
    if trigger_price <= 0.0 || current_price <= 0.0 {
        return 0.0;
    }
    let raw = (current_price - trigger_price) / trigger_price * 100.0;
    match signal_type {
        "dump" => -raw,
        _ => raw,
    }
}

fn is_signal_win(signal_type: &str, signed_return_pct: f64) -> bool {
    match signal_type {
        "pump" | "dump" => signed_return_pct >= 0.0,
        _ => false,
    }
}

fn i64_to_usize(value: i64) -> usize {
    if value <= 0 {
        0
    } else {
        value as usize
    }
}

fn panic_payload_message(payload: &(dyn std::any::Any + Send)) -> &str {
    if let Some(msg) = payload.downcast_ref::<&'static str>() {
        msg
    } else if let Some(msg) = payload.downcast_ref::<String>() {
        msg.as_str()
    } else {
        "unknown panic payload"
    }
}

fn cache_signal_perf_summary(
    cache: &Arc<Mutex<HashMap<String, SignalPerfCacheEntry>>>,
    symbol: String,
    summary: SignalPerfSummary,
) {
    let mut guard = cache.lock().expect("panel perf cache mutex poisoned");
    guard.insert(
        symbol,
        SignalPerfCacheEntry {
            summary,
            loaded_at: Instant::now(),
        },
    );
}

fn signal_perf_section(perf: &SignalPerfSummary) -> EnterpriseMetricSection {
    EnterpriseMetricSection {
        title: "信号质量".into(),
        subtitle: "最近触发后的表现".into(),
        items: vec![
            metric_row(
                "过去5分钟表现",
                clamp(perf.win5, 0.0, 100.0),
                if perf.count5 > 0 {
                    format!("{:.0}%", perf.win5)
                } else {
                    "样本少".into()
                },
                "信号触发后 5 分钟内，方向判断的正确率。",
                false,
            ),
            metric_row(
                "过去15分钟表现",
                clamp(perf.win15, 0.0, 100.0),
                if perf.count15 > 0 {
                    format!("{:.0}%", perf.win15)
                } else {
                    "样本少".into()
                },
                "信号触发后 15 分钟内，方向判断的正确率。",
                false,
            ),
            metric_row(
                "胜率/误报率",
                clamp(perf.win15.max(perf.win5), 0.0, 100.0),
                if perf.count15 > 0 {
                    format!("{:.0} / {:.0}%", perf.win15, 100.0 - perf.win15)
                } else {
                    "待积累".into()
                },
                "胜率越高说明更稳定，误报率越低说明噪音更少。",
                false,
            ),
            metric_row(
                "信号衰减速度",
                clamp(100.0 - perf.decay * 6.0, 0.0, 100.0),
                if perf.decay > 0.0 {
                    format!("{:.1} 分钟", perf.decay)
                } else {
                    "待积累".into()
                },
                "信号从强提醒回落到普通关注所需的平均时间。",
                false,
            ),
        ],
    }
}

fn apply_signal_perf_to_enterprise_sections(
    sections: &mut Vec<EnterpriseMetricSection>,
    perf: &SignalPerfSummary,
) {
    let replacement = signal_perf_section(perf);
    if let Some(existing) = sections
        .iter_mut()
        .find(|section| section.title == "信号质量")
    {
        *existing = replacement;
    } else {
        sections.push(replacement);
    }
}

fn apply_signal_perf_to_enterprise_metrics(raw: &str, perf: &SignalPerfSummary) -> String {
    let mut sections =
        serde_json::from_str::<Vec<EnterpriseMetricSection>>(raw).unwrap_or_default();
    apply_signal_perf_to_enterprise_sections(&mut sections, perf);
    serde_json::to_string(&sections).unwrap_or_else(|_| raw.to_string())
}
