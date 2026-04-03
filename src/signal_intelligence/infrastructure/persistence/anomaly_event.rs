//! 异动检测事件持久化服务
//!
//! 将 OrderBookAnomalyDetector 检测到的异动事件异步写入 market.anomaly_event。
//! 使用 mpsc channel + background worker 模式（同 trade.rs）。

use std::sync::Arc;

use anyhow::Result;
use chrono::Utc;
use tokio::sync::mpsc;

use crate::shared::postgres::PgPool;

const QUEUE_CAPACITY: usize = 16384;

/// 异动事件记录（扁平化，不依赖 anomaly_detector 内部类型）
#[derive(Debug, Clone)]
pub struct AnomalyEventRecord {
    pub symbol: String,
    pub anomaly_type: String, // 'RapidCancellation' / 'OrderSurge' / 'WallAppear' 等
    pub severity: String,     // 'low' | 'medium' | 'high' | 'critical'
    pub confidence: f64,      // [0, 1]
    pub price_level: Option<f64>,
    pub side: Option<String>, // 'bid' | 'ask' | null
    pub size_qty: Option<f64>,
    pub percentage: Option<f64>,
    pub duration_ms: Option<i32>,
    pub description: String,
}

#[derive(Clone)]
pub struct AnomalyEventPersistenceService {
    sender: mpsc::Sender<AnomalyEventRecord>,
}

impl AnomalyEventPersistenceService {
    pub async fn new(pool: Arc<PgPool>) -> Result<Self> {
        ensure_schema(&pool).await?;

        let (sender, mut receiver) = mpsc::channel::<AnomalyEventRecord>(QUEUE_CAPACITY);
        let pool_clone = pool.clone();

        tokio::spawn(async move {
            while let Some(record) = receiver.recv().await {
                if let Err(err) = insert_record(&pool_clone, &record).await {
                    eprintln!(
                        "anomaly_event persist error [{}:{}]: {}",
                        record.symbol, record.anomaly_type, err
                    );
                }
            }
        });

        Ok(Self { sender })
    }

    /// 提交单条事件（try_send，满则丢弃）
    pub fn submit(&self, record: AnomalyEventRecord) {
        if let Err(_) = self.sender.try_send(record) {
            // 队列满时静默丢弃
        }
    }

    /// 批量提交
    pub fn submit_batch(&self, records: Vec<AnomalyEventRecord>) {
        for r in records {
            self.submit(r);
        }
    }
}

async fn ensure_schema(pool: &Arc<PgPool>) -> Result<()> {
    let client = pool.acquire().await?;
    client
        .client()
        .batch_execute(include_str!(
            "../../../../sql/postgres/market_anomaly_event.sql"
        ))
        .await?;
    Ok(())
}

async fn insert_record(pool: &Arc<PgPool>, record: &AnomalyEventRecord) -> Result<()> {
    let client = pool.acquire().await?;
    client
        .client()
        .execute(
            "insert into market.anomaly_event
                (symbol, detected_at, anomaly_type, severity, confidence,
                 price_level, side, size_qty, percentage, duration_ms, description)
             values ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11)",
            &[
                &record.symbol,
                &Utc::now(),
                &record.anomaly_type,
                &record.severity,
                &record.confidence,
                &record.price_level,
                &record.side,
                &record.size_qty,
                &record.percentage,
                &record.duration_ms,
                &record.description,
            ],
        )
        .await?;
    Ok(())
}
