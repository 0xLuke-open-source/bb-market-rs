//! 订单簿有意义变化事件持久化服务
//!
//! 只记录 add / cancel / modify(>=5%) 三类变化，以批量方式写入 market.orderbook_tick。
//! 批量触发条件：积累 >= 100 条 或 距上次写入 >= 500ms。

use std::sync::Arc;
use std::time::{Duration, Instant};

use anyhow::Result;
use chrono::{TimeZone, Utc};
use tokio::sync::mpsc;

use crate::market_data::application::ports::OrderBookTickSink;
use crate::market_data::domain::order_book::OrderChangeEvent;
use crate::shared::postgres::PgPool;

const QUEUE_CAPACITY: usize = 65536;
const BATCH_SIZE: usize = 100;
const FLUSH_INTERVAL: Duration = Duration::from_millis(500);

#[derive(Clone)]
pub struct OrderBookTickPersistenceService {
    sender: mpsc::Sender<OrderChangeEvent>,
}

impl OrderBookTickPersistenceService {
    pub async fn new(pool: Arc<PgPool>) -> Result<Self> {
        ensure_schema(&pool).await?;

        let (sender, mut receiver) = mpsc::channel::<OrderChangeEvent>(QUEUE_CAPACITY);
        let pool_clone = pool.clone();

        tokio::spawn(async move {
            let mut batch: Vec<OrderChangeEvent> = Vec::with_capacity(BATCH_SIZE * 2);
            let mut last_flush = Instant::now();

            loop {
                // 非阻塞接收直到 batch 满或超时
                loop {
                    match receiver.try_recv() {
                        Ok(ev) => {
                            batch.push(ev);
                            if batch.len() >= BATCH_SIZE {
                                break;
                            }
                        }
                        Err(_) => break,
                    }
                }

                let should_flush = !batch.is_empty() && last_flush.elapsed() >= FLUSH_INTERVAL
                    || batch.len() >= BATCH_SIZE;

                if should_flush {
                    if let Err(err) = insert_batch(&pool_clone, &batch).await {
                        eprintln!("orderbook_tick batch insert error: {}", err);
                    }
                    batch.clear();
                    last_flush = Instant::now();
                }

                // 等待下一条消息（避免 busy-loop）
                tokio::time::sleep(Duration::from_millis(50)).await;
            }
        });

        Ok(Self { sender })
    }

    /// 批量提交变化事件（不阻塞，满则丢弃）
    pub fn submit_batch(&self, events: Vec<OrderChangeEvent>) {
        for ev in events {
            if let Err(_) = self.sender.try_send(ev) {
                // 队列满时静默丢弃，不影响主路径
                break;
            }
        }
    }
}

impl OrderBookTickSink for OrderBookTickPersistenceService {
    fn persist_orderbook_changes(&self, events: Vec<OrderChangeEvent>) {
        self.submit_batch(events);
    }
}

async fn ensure_schema(pool: &Arc<PgPool>) -> Result<()> {
    let client = pool.acquire().await?;
    client
        .client()
        .batch_execute(include_str!(
            "../../../../sql/postgres/market_orderbook_tick.sql"
        ))
        .await?;
    Ok(())
}

async fn insert_batch(pool: &Arc<PgPool>, batch: &[OrderChangeEvent]) -> Result<()> {
    if batch.is_empty() {
        return Ok(());
    }
    let client = pool.acquire().await?;
    let db = client.client();

    // 使用 unnest 批量插入
    let mut symbols: Vec<String> = Vec::with_capacity(batch.len());
    let mut event_times: Vec<chrono::DateTime<Utc>> = Vec::with_capacity(batch.len());
    let mut sides: Vec<String> = Vec::with_capacity(batch.len());
    let mut prices: Vec<f64> = Vec::with_capacity(batch.len());
    let mut qty_befores: Vec<f64> = Vec::with_capacity(batch.len());
    let mut qty_afters: Vec<f64> = Vec::with_capacity(batch.len());
    let mut change_types: Vec<String> = Vec::with_capacity(batch.len());

    for ev in batch {
        symbols.push(ev.symbol.clone());
        event_times.push(
            Utc.timestamp_millis_opt(ev.event_time_ms.min(i64::MAX as u64) as i64)
                .single()
                .unwrap_or_else(Utc::now),
        );
        sides.push(ev.side.as_str().to_string());
        prices.push(ev.price.to_string().parse::<f64>().unwrap_or(0.0));
        qty_befores.push(ev.qty_before.to_string().parse::<f64>().unwrap_or(0.0));
        qty_afters.push(ev.qty_after.to_string().parse::<f64>().unwrap_or(0.0));
        change_types.push(ev.change_type.as_str().to_string());
    }

    db.execute(
        "insert into market.orderbook_tick
            (symbol, event_time, side, price, qty_before, qty_after, change_type)
         select * from unnest($1::varchar[], $2::timestamptz[], $3::varchar[], $4::float8[], $5::float8[], $6::float8[], $7::varchar[])",
        &[
            &symbols,
            &event_times,
            &sides,
            &prices,
            &qty_befores,
            &qty_afters,
            &change_types,
        ],
    )
    .await?;

    Ok(())
}
