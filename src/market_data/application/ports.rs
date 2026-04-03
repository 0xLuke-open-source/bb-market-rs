use anyhow::Result;
use async_trait::async_trait;
use chrono::{DateTime, Utc};
use tokio::sync::mpsc::Sender;

use crate::market_data::application::dto::{
    BigTradeHistoryRecord, BigTradeJson, BigTradeStatsRecord,
};
use crate::market_data::domain::order_book::OrderChangeEvent;
use crate::market_data::domain::stream::{AggTrade, StreamMsg};

pub trait RecentTradeSink: Send + Sync {
    fn persist_agg_trade(&self, trade: &AggTrade);
}

pub trait BigTradeSink: Send + Sync {
    fn persist_big_trade(&self, trade: &AggTrade, threshold_quantity: f64);
}

pub trait OrderBookTickSink: Send + Sync {
    fn persist_orderbook_changes(&self, events: Vec<OrderChangeEvent>);
}

#[async_trait]
pub trait RecentTradeReader: Send + Sync {
    async fn load_recent_trades(&self, symbol: &str, limit: usize) -> Result<Vec<BigTradeJson>>;
}

#[async_trait]
pub trait BigTradeReader: Send + Sync {
    async fn load_big_trades(
        &self,
        symbol: &str,
        limit: usize,
        from: Option<DateTime<Utc>>,
        to: Option<DateTime<Utc>>,
    ) -> Result<Vec<BigTradeHistoryRecord>>;

    async fn load_big_trade_stats(
        &self,
        symbol: &str,
        from: Option<DateTime<Utc>>,
        to: Option<DateTime<Utc>>,
    ) -> Result<BigTradeStatsRecord>;
}

#[async_trait]
pub trait MarketStreamClient: Send + Sync {
    async fn run_symbol_stream(&self, symbol: &str, tx: Sender<StreamMsg>) -> Result<()>;
}
