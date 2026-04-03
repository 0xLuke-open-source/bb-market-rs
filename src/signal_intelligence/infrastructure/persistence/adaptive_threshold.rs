//! 自适应阈值快照持久化服务
//!
//! 每 5 分钟将每个 symbol 的滚动统计快照写入 market.adaptive_threshold。
//! 使用 mpsc channel + background worker 模式。

use std::sync::Arc;

use anyhow::Result;
use chrono::Utc;
use tokio::sync::mpsc;

use crate::shared::postgres::PgPool;
use crate::signal_intelligence::domain::adaptive_threshold::AdaptiveThresholdSnapshot;

const QUEUE_CAPACITY: usize = 4096;

/// 带 symbol 的快照包装（AdaptiveThresholdSnapshot 自身不含 symbol）
#[derive(Debug, Clone)]
pub struct AdaptiveThresholdRecord {
    pub symbol: String,
    pub snapshot: AdaptiveThresholdSnapshot,
}

#[derive(Clone)]
pub struct AdaptiveThresholdPersistenceService {
    sender: mpsc::Sender<AdaptiveThresholdRecord>,
}

impl AdaptiveThresholdPersistenceService {
    pub async fn new(pool: Arc<PgPool>) -> Result<Self> {
        ensure_schema(&pool).await?;

        let (sender, mut receiver) = mpsc::channel::<AdaptiveThresholdRecord>(QUEUE_CAPACITY);
        let pool_clone = pool.clone();

        tokio::spawn(async move {
            while let Some(record) = receiver.recv().await {
                if let Err(err) = insert_record(&pool_clone, &record).await {
                    eprintln!(
                        "adaptive_threshold persist error [{}]: {}",
                        record.symbol, err
                    );
                }
            }
        });

        Ok(Self { sender })
    }

    /// 提交快照（try_send，满则丢弃）
    pub fn submit(&self, symbol: &str, snapshot: AdaptiveThresholdSnapshot) {
        let record = AdaptiveThresholdRecord {
            symbol: symbol.to_string(),
            snapshot,
        };
        if let Err(_) = self.sender.try_send(record) {
            // 队列满时静默丢弃
        }
    }
}

async fn ensure_schema(pool: &Arc<PgPool>) -> Result<()> {
    let client = pool.acquire().await?;
    client
        .client()
        .batch_execute(include_str!(
            "../../../../sql/postgres/market_adaptive_threshold.sql"
        ))
        .await?;
    Ok(())
}

async fn insert_record(pool: &Arc<PgPool>, record: &AdaptiveThresholdRecord) -> Result<()> {
    let s = &record.snapshot;
    let client = pool.acquire().await?;
    client
        .client()
        .execute(
            "insert into market.adaptive_threshold
                (symbol, window_end_at, sample_count, is_warm,
                 ofi_mean, ofi_std, obi_mean, obi_std,
                 vol_mean, vol_std, bid_vol_mean, bid_vol_std,
                 spread_mean, spread_std)
             values ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12, $13, $14)",
            &[
                &record.symbol,
                &Utc::now(),
                &s.sample_count,
                &s.is_warm,
                &s.ofi_mean,
                &s.ofi_std,
                &s.obi_mean,
                &s.obi_std,
                &s.vol_mean,
                &s.vol_std,
                &s.bid_vol_mean,
                &s.bid_vol_std,
                &s.spread_mean,
                &s.spread_std,
            ],
        )
        .await?;
    Ok(())
}
