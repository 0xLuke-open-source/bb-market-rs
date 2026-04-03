use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct BigTradeJson {
    pub t: u64,
    pub p: f64,
    pub q: f64,
    pub buy: bool,
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
