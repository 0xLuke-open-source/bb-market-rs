use std::collections::HashMap;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Arc, Mutex};

use anyhow::Result;
use chrono::{DateTime, TimeZone, Utc};
use tokio::sync::mpsc;

use crate::codec::binance_msg::AggTrade;
use crate::postgres::PgPool;

const BIG_TRADE_PERSIST_QUEUE_CAPACITY: usize = 4096;
const BIG_TRADE_METRICS_LOG_INTERVAL_SECS: u64 = 30;

#[derive(Debug, Clone)]
pub struct BigTradeRecord {
    pub symbol: String,
    pub agg_trade_id: i64,
    pub event_time: DateTime<Utc>,
    pub trade_time: DateTime<Utc>,
    pub price: f64,
    pub quantity: f64,
    pub quote_quantity: f64,
    pub threshold_quantity: f64,
    pub is_taker_buy: bool,
    pub is_buyer_maker: bool,
}

#[derive(Clone)]
pub struct BigTradePersistenceService {
    sender: mpsc::Sender<BigTradeRecord>,
    state: Arc<Mutex<HashMap<String, i64>>>,
    metrics: Arc<PersistMetrics>,
}

#[derive(Clone)]
pub struct BigTradeQueryService {
    pool: Arc<PgPool>,
}

#[derive(Clone)]
struct PostgresBigTradeRepository {
    pool: Arc<PgPool>,
}

#[derive(Debug, Clone)]
pub struct BigTradeHistoryRecord {
    pub symbol: String,
    pub agg_trade_id: i64,
    pub event_time: DateTime<Utc>,
    pub trade_time: DateTime<Utc>,
    pub price: f64,
    pub quantity: f64,
    pub quote_quantity: f64,
    pub threshold_quantity: f64,
    pub is_taker_buy: bool,
    pub is_buyer_maker: bool,
}

#[derive(Debug, Clone, Default)]
pub struct BigTradeStatsRecord {
    pub symbol: String,
    pub total_count: i64,
    pub buy_count: i64,
    pub sell_count: i64,
    pub total_quote_quantity: f64,
    pub buy_quote_quantity: f64,
    pub sell_quote_quantity: f64,
    pub avg_quote_quantity: f64,
    pub max_quote_quantity: f64,
    pub avg_threshold_quantity: f64,
    pub first_trade_time: Option<DateTime<Utc>>,
    pub last_trade_time: Option<DateTime<Utc>>,
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

impl BigTradeRecord {
    pub fn from_agg_trade(trade: &AggTrade, threshold_quantity: f64) -> Self {
        let price = trade.price.parse::<f64>().unwrap_or(0.0);
        let quantity = trade.qty.parse::<f64>().unwrap_or(0.0);
        let agg_trade_id = trade.agg_trade_id.min(i64::MAX as u64) as i64;
        Self {
            symbol: trade.symbol.trim().to_ascii_uppercase(),
            agg_trade_id,
            event_time: ts_millis_to_utc(trade.event_time),
            trade_time: ts_millis_to_utc(trade.trade_time),
            price,
            quantity,
            quote_quantity: price * quantity,
            threshold_quantity,
            is_taker_buy: trade.is_taker_buy(),
            is_buyer_maker: trade.is_buyer_maker,
        }
    }

    fn is_persistable(&self) -> bool {
        !self.symbol.is_empty()
            && self.agg_trade_id > 0
            && self.price > 0.0
            && self.quantity > 0.0
            && self.quote_quantity > 0.0
            && self.threshold_quantity > 0.0
    }
}

impl BigTradePersistenceService {
    pub async fn new(pool: Arc<PgPool>) -> Result<Self> {
        let repository = Arc::new(PostgresBigTradeRepository::new(pool));
        repository.ensure_schema().await?;
        let metrics = Arc::new(PersistMetrics::default());

        let (sender, mut receiver) = mpsc::channel(BIG_TRADE_PERSIST_QUEUE_CAPACITY);
        let worker_repository = repository.clone();
        let worker_metrics = metrics.clone();
        tokio::spawn(async move {
            while let Some(record) = receiver.recv().await {
                if let Err(err) = worker_repository.insert_trade(&record).await {
                    worker_metrics.record_error();
                    eprintln!(
                        "big trade persist error [{}:{}]: {}",
                        record.symbol, record.agg_trade_id, err
                    );
                } else {
                    worker_metrics.record_success();
                }
            }
        });

        let report_metrics = metrics.clone();
        tokio::spawn(async move {
            let mut tick = tokio::time::interval(std::time::Duration::from_secs(
                BIG_TRADE_METRICS_LOG_INTERVAL_SECS,
            ));
            loop {
                tick.tick().await;
                report_metrics.log_snapshot();
            }
        });

        Ok(Self {
            sender,
            state: Arc::new(Mutex::new(HashMap::new())),
            metrics,
        })
    }

    pub fn submit_trade(&self, record: BigTradeRecord) {
        if !record.is_persistable() {
            self.metrics.record_filtered();
            return;
        }

        if !self.should_persist(&record) {
            self.metrics.record_skipped();
            return;
        }

        if let Err(err) = self.sender.try_send(record.clone()) {
            self.metrics.record_queue_full();
            eprintln!(
                "big trade queue full [{}:{}]: {}",
                record.symbol, record.agg_trade_id, err
            );
            return;
        }

        let mut state = self.state.lock().expect("big trade state mutex poisoned");
        state.insert(record.symbol.clone(), record.agg_trade_id);
    }

    fn should_persist(&self, record: &BigTradeRecord) -> bool {
        let state = self.state.lock().expect("big trade state mutex poisoned");
        match state.get(&record.symbol) {
            Some(last_agg_trade_id) => record.agg_trade_id > *last_agg_trade_id,
            None => true,
        }
    }
}

impl BigTradeQueryService {
    pub async fn new(pool: Arc<PgPool>) -> Result<Self> {
        let repository = PostgresBigTradeRepository::new(pool.clone());
        repository.ensure_schema().await?;
        Ok(Self { pool })
    }

    pub async fn load_big_trades(
        &self,
        symbol: &str,
        limit: usize,
        from: Option<DateTime<Utc>>,
        to: Option<DateTime<Utc>>,
    ) -> Result<Vec<BigTradeHistoryRecord>> {
        let client = self.pool.acquire().await?;
        let normalized_symbol = symbol.trim().to_ascii_uppercase();
        let query_limit = i64::try_from(limit).unwrap_or(i64::MAX);
        let rows = client
            .client()
            .query(
                "select symbol, agg_trade_id, event_time, trade_time, price, quantity,
                        quote_quantity, threshold_quantity, is_taker_buy, is_buyer_maker
                   from market.big_trade_event
                  where symbol = $1
                    and ($2::timestamptz is null or trade_time >= $2)
                    and ($3::timestamptz is null or trade_time <= $3)
                  order by trade_time desc
                  limit $4",
                &[&normalized_symbol, &from, &to, &query_limit],
            )
            .await?;

        Ok(rows
            .into_iter()
            .map(|row| BigTradeHistoryRecord {
                symbol: row.get("symbol"),
                agg_trade_id: row.get("agg_trade_id"),
                event_time: row.get("event_time"),
                trade_time: row.get("trade_time"),
                price: row.get("price"),
                quantity: row.get("quantity"),
                quote_quantity: row.get("quote_quantity"),
                threshold_quantity: row.get("threshold_quantity"),
                is_taker_buy: row.get("is_taker_buy"),
                is_buyer_maker: row.get("is_buyer_maker"),
            })
            .collect())
    }

    pub async fn load_big_trade_stats(
        &self,
        symbol: &str,
        from: Option<DateTime<Utc>>,
        to: Option<DateTime<Utc>>,
    ) -> Result<BigTradeStatsRecord> {
        let client = self.pool.acquire().await?;
        let normalized_symbol = symbol.trim().to_ascii_uppercase();
        let row = client
            .client()
            .query_one(
                "select $1::varchar as symbol,
                        count(*)::bigint as total_count,
                        count(*) filter (where is_taker_buy) as buy_count,
                        count(*) filter (where not is_taker_buy) as sell_count,
                        coalesce(sum(quote_quantity), 0) as total_quote_quantity,
                        coalesce(sum(quote_quantity) filter (where is_taker_buy), 0) as buy_quote_quantity,
                        coalesce(sum(quote_quantity) filter (where not is_taker_buy), 0) as sell_quote_quantity,
                        coalesce(avg(quote_quantity), 0) as avg_quote_quantity,
                        coalesce(max(quote_quantity), 0) as max_quote_quantity,
                        coalesce(avg(threshold_quantity), 0) as avg_threshold_quantity,
                        min(trade_time) as first_trade_time,
                        max(trade_time) as last_trade_time
                   from market.big_trade_event
                  where symbol = $1
                    and ($2::timestamptz is null or trade_time >= $2)
                    and ($3::timestamptz is null or trade_time <= $3)",
                &[&normalized_symbol, &from, &to],
            )
            .await?;

        Ok(BigTradeStatsRecord {
            symbol: row.get("symbol"),
            total_count: row.get("total_count"),
            buy_count: row.get("buy_count"),
            sell_count: row.get("sell_count"),
            total_quote_quantity: row.get("total_quote_quantity"),
            buy_quote_quantity: row.get("buy_quote_quantity"),
            sell_quote_quantity: row.get("sell_quote_quantity"),
            avg_quote_quantity: row.get("avg_quote_quantity"),
            max_quote_quantity: row.get("max_quote_quantity"),
            avg_threshold_quantity: row.get("avg_threshold_quantity"),
            first_trade_time: row.get("first_trade_time"),
            last_trade_time: row.get("last_trade_time"),
        })
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
            "big trade persist metrics: +success={} +filtered={} +skipped={} +queue_full={} +error={} | total success={} filtered={} skipped={} queue_full={} error={}",
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

impl PostgresBigTradeRepository {
    fn new(pool: Arc<PgPool>) -> Self {
        Self { pool }
    }

    async fn ensure_schema(&self) -> Result<()> {
        let client = self.pool.acquire().await?;
        client
            .client()
            .batch_execute(include_str!("../../sql/postgres/market_big_trade.sql"))
            .await?;
        Ok(())
    }

    async fn insert_trade(&self, record: &BigTradeRecord) -> Result<()> {
        let client = self.pool.acquire().await?;
        client
            .client()
            .execute(
                "insert into market.big_trade_event (
                    symbol, agg_trade_id, event_time, trade_time,
                    price, quantity, quote_quantity, threshold_quantity,
                    is_taker_buy, is_buyer_maker
                ) values (
                    $1, $2, $3, $4,
                    $5, $6, $7, $8,
                    $9, $10
                )
                on conflict (symbol, agg_trade_id) do nothing",
                &[
                    &record.symbol,
                    &record.agg_trade_id,
                    &record.event_time,
                    &record.trade_time,
                    &record.price,
                    &record.quantity,
                    &record.quote_quantity,
                    &record.threshold_quantity,
                    &record.is_taker_buy,
                    &record.is_buyer_maker,
                ],
            )
            .await?;
        Ok(())
    }
}

fn ts_millis_to_utc(value: u64) -> DateTime<Utc> {
    Utc.timestamp_millis_opt(value.min(i64::MAX as u64) as i64)
        .single()
        .unwrap_or_else(Utc::now)
}
