use std::collections::HashMap;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

use anyhow::Result;
use tokio::sync::mpsc;

use crate::shared::postgres::PgPool;
use crate::terminal::application::dto::OrderBookSnapshotRecord;
use crate::terminal::application::ports::OrderBookSnapshotSink;

const ORDER_BOOK_PERSIST_INTERVAL_SECS: u64 = 2;
const ORDER_BOOK_PERSIST_QUEUE_CAPACITY: usize = 2048;
const ORDER_BOOK_METRICS_LOG_INTERVAL_SECS: u64 = 30;

#[derive(Clone)]
pub struct OrderBookPersistenceService {
    sender: mpsc::Sender<OrderBookSnapshotRecord>,
    state: Arc<Mutex<HashMap<String, PersistState>>>,
    metrics: Arc<PersistMetrics>,
    min_interval: Duration,
}

#[derive(Debug, Clone)]
struct PersistState {
    last_update_count: i64,
    last_persisted_at: Instant,
}

#[derive(Clone)]
struct PostgresOrderBookSnapshotRepository {
    pool: Arc<PgPool>,
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

impl OrderBookPersistenceService {
    pub async fn new(pool: Arc<PgPool>) -> Result<Self> {
        let repository = Arc::new(PostgresOrderBookSnapshotRepository::new(pool));
        repository.ensure_schema().await?;
        let metrics = Arc::new(PersistMetrics::default());

        let (sender, mut receiver) = mpsc::channel(ORDER_BOOK_PERSIST_QUEUE_CAPACITY);
        let worker_repository = repository.clone();
        let worker_metrics = metrics.clone();
        tokio::spawn(async move {
            while let Some(snapshot) = receiver.recv().await {
                if let Err(err) = worker_repository.insert_snapshot(&snapshot).await {
                    worker_metrics.record_error();
                    eprintln!(
                        "order book snapshot persist error [{}]: {}",
                        snapshot.symbol, err
                    );
                } else {
                    worker_metrics.record_success();
                }
            }
        });

        let report_metrics = metrics.clone();
        tokio::spawn(async move {
            let mut tick =
                tokio::time::interval(Duration::from_secs(ORDER_BOOK_METRICS_LOG_INTERVAL_SECS));
            loop {
                tick.tick().await;
                report_metrics.log_snapshot();
            }
        });

        Ok(Self {
            sender,
            state: Arc::new(Mutex::new(HashMap::new())),
            metrics,
            min_interval: Duration::from_secs(ORDER_BOOK_PERSIST_INTERVAL_SECS),
        })
    }

    pub fn submit_snapshot(&self, snapshot: OrderBookSnapshotRecord) {
        if !snapshot.is_persistable() {
            self.metrics.record_filtered();
            return;
        }

        if !self.should_persist(&snapshot) {
            self.metrics.record_skipped();
            return;
        }

        if let Err(err) = self.sender.try_send(snapshot.clone()) {
            self.metrics.record_queue_full();
            eprintln!(
                "order book snapshot queue full [{}]: {}",
                snapshot.symbol, err
            );
            return;
        }

        let mut state = self.state.lock().expect("order book state mutex poisoned");
        state.insert(
            snapshot.symbol.clone(),
            PersistState {
                last_update_count: snapshot.update_count,
                last_persisted_at: Instant::now(),
            },
        );
    }

    fn should_persist(&self, snapshot: &OrderBookSnapshotRecord) -> bool {
        let state = self.state.lock().expect("order book state mutex poisoned");
        match state.get(&snapshot.symbol) {
            Some(last) if last.last_update_count == snapshot.update_count => false,
            Some(last) if last.last_persisted_at.elapsed() < self.min_interval => false,
            _ => true,
        }
    }
}

impl OrderBookSnapshotSink for OrderBookPersistenceService {
    fn submit_snapshot(&self, snapshot: OrderBookSnapshotRecord) {
        OrderBookPersistenceService::submit_snapshot(self, snapshot);
    }
}

impl OrderBookSnapshotRecord {
    fn is_persistable(&self) -> bool {
        self.update_count > 0
            && self.bid > 0.0
            && self.ask > 0.0
            && self.mid > 0.0
            && !self.bid_depth.is_empty()
            && !self.ask_depth.is_empty()
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
            "order book persist metrics: +success={} +filtered={} +skipped={} +queue_full={} +error={} | total success={} filtered={} skipped={} queue_full={} error={}",
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

impl PostgresOrderBookSnapshotRepository {
    fn new(pool: Arc<PgPool>) -> Self {
        Self { pool }
    }

    async fn ensure_schema(&self) -> Result<()> {
        let client = self.pool.acquire().await?;
        client
            .client()
            .batch_execute(include_str!(
                "../../../../sql/postgres/market_order_book.sql"
            ))
            .await?;
        Ok(())
    }

    async fn insert_snapshot(&self, snapshot: &OrderBookSnapshotRecord) -> Result<()> {
        let bid_depth_json = serde_json::to_string(&snapshot.bid_depth)?;
        let ask_depth_json = serde_json::to_string(&snapshot.ask_depth)?;
        let client = self.pool.acquire().await?;
        client
            .client()
            .execute(
                "insert into market.order_book_snapshot (
                    snapshot_id, symbol, event_time,
                    bid, ask, mid, spread_bps,
                    total_bid_volume, total_ask_volume,
                    ofi, ofi_raw, obi, trend_strength,
                    cvd, taker_buy_ratio,
                    price_precision, quantity_precision,
                    bid_depth, ask_depth,
                    update_count
                ) values (
                    $1, $2, $3,
                    $4, $5, $6, $7,
                    $8, $9,
                    $10, $11, $12, $13,
                    $14, $15,
                    $16, $17,
                    cast($18 as text)::jsonb, cast($19 as text)::jsonb,
                    $20
                )",
                &[
                    &snapshot.snapshot_id,
                    &snapshot.symbol,
                    &snapshot.event_time,
                    &snapshot.bid,
                    &snapshot.ask,
                    &snapshot.mid,
                    &snapshot.spread_bps,
                    &snapshot.total_bid_volume,
                    &snapshot.total_ask_volume,
                    &snapshot.ofi,
                    &snapshot.ofi_raw,
                    &snapshot.obi,
                    &snapshot.trend_strength,
                    &snapshot.cvd,
                    &snapshot.taker_buy_ratio,
                    &snapshot.price_precision,
                    &snapshot.quantity_precision,
                    &bid_depth_json,
                    &ask_depth_json,
                    &snapshot.update_count,
                ],
            )
            .await?;
        Ok(())
    }
}
