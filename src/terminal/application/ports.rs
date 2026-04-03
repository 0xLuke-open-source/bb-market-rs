use anyhow::Result;
use async_trait::async_trait;
use chrono::{DateTime, Utc};

use crate::terminal::application::dto::{
    OrderBookSnapshotRecord, SalesProofOverviewRecord, SignalPerformanceSampleRecord,
    SymbolPanelSnapshotRecord,
};
use crate::terminal::application::projection::{FeedEntry, SymbolJson};

pub trait OrderBookSnapshotSink: Send + Sync {
    fn submit_snapshot(&self, snapshot: OrderBookSnapshotRecord);
}

#[async_trait]
pub trait SymbolPanelSnapshotStore: Send + Sync {
    async fn decorate_live_snapshot(
        &self,
        snapshot: &mut SymbolJson,
        signal_history: Vec<FeedEntry>,
    );

    fn submit_snapshot(&self, snapshot: &SymbolJson, signal_history: Vec<FeedEntry>);
}

#[async_trait]
pub trait SymbolPanelSnapshotReader: Send + Sync {
    async fn load_recent_snapshots(
        &self,
        symbol: &str,
        limit: usize,
        from: Option<DateTime<Utc>>,
        to: Option<DateTime<Utc>>,
    ) -> Result<Vec<SymbolPanelSnapshotRecord>>;

    async fn load_signal_perf_samples(
        &self,
        symbol: &str,
        limit: usize,
    ) -> Result<Vec<SignalPerformanceSampleRecord>>;

    async fn load_sales_proof_overview(
        &self,
        window_days: i64,
        top_symbols_limit: usize,
    ) -> Result<SalesProofOverviewRecord>;
}
