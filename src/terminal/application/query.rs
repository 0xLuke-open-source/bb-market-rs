use anyhow::Result;
use chrono::{DateTime, Utc};
use std::sync::Arc;

use crate::market_data::application::ports::{BigTradeReader, RecentTradeReader};
use crate::market_data::application::BigTradeJson;
use crate::terminal::application::ports::{SymbolPanelSnapshotReader, SymbolPanelSnapshotStore};
use crate::terminal::application::projection::{FeedEntry, SymbolJson};

#[derive(Clone)]
pub struct TerminalQueryService {
    panel_runtime: Arc<dyn SymbolPanelSnapshotStore>,
    panel_query: Arc<dyn SymbolPanelSnapshotReader>,
    recent_trade_query: Arc<dyn RecentTradeReader>,
    big_trade_query: Arc<dyn BigTradeReader>,
}

impl TerminalQueryService {
    pub fn new(
        panel_runtime: Arc<dyn SymbolPanelSnapshotStore>,
        panel_query: Arc<dyn SymbolPanelSnapshotReader>,
        recent_trade_query: Arc<dyn RecentTradeReader>,
        big_trade_query: Arc<dyn BigTradeReader>,
    ) -> Self {
        Self {
            panel_runtime,
            panel_query,
            recent_trade_query,
            big_trade_query,
        }
    }

    pub async fn load_recent_trades(
        &self,
        symbol: &str,
        limit: usize,
    ) -> Result<Vec<BigTradeJson>> {
        self.recent_trade_query
            .load_recent_trades(symbol, limit)
            .await
    }

    pub async fn load_big_trade_history(
        &self,
        symbol: &str,
        limit: usize,
        from: Option<DateTime<Utc>>,
        to: Option<DateTime<Utc>>,
    ) -> Result<Vec<BigTradeHistoryRecord>> {
        self.big_trade_query
            .load_big_trades(symbol, limit, from, to)
            .await
    }

    pub async fn load_big_trade_stats(
        &self,
        symbol: &str,
        from: Option<DateTime<Utc>>,
        to: Option<DateTime<Utc>>,
    ) -> Result<BigTradeStatsRecord> {
        self.big_trade_query
            .load_big_trade_stats(symbol, from, to)
            .await
    }

    pub async fn load_panel_history(
        &self,
        symbol: &str,
        limit: usize,
        from: Option<DateTime<Utc>>,
        to: Option<DateTime<Utc>>,
    ) -> Result<Vec<SymbolPanelSnapshotRecord>> {
        self.panel_query
            .load_recent_snapshots(symbol, limit, from, to)
            .await
    }

    pub async fn load_signal_perf_history(
        &self,
        symbol: &str,
        limit: usize,
    ) -> Result<Vec<SignalPerformanceSampleRecord>> {
        self.panel_query
            .load_signal_perf_samples(symbol, limit)
            .await
    }

    pub async fn load_sales_proof_overview(
        &self,
        window_days: i64,
        top_symbols_limit: usize,
    ) -> Result<SalesProofOverviewRecord> {
        self.panel_query
            .load_sales_proof_overview(window_days, top_symbols_limit)
            .await
    }

    pub async fn decorate_live_snapshot(
        &self,
        snapshot: &mut SymbolJson,
        signal_history: Vec<FeedEntry>,
    ) {
        self.panel_runtime
            .decorate_live_snapshot(snapshot, signal_history)
            .await;
    }
}

pub use crate::market_data::application::{BigTradeHistoryRecord, BigTradeStatsRecord};
pub use crate::terminal::application::dto::{
    SalesProofOverviewRecord, SignalPerformanceSampleRecord, SymbolPanelSnapshotRecord,
};
