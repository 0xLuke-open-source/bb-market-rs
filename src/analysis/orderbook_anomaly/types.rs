use std::collections::HashMap;

use chrono::{DateTime, Local};
use rust_decimal::Decimal;

#[derive(Debug, Clone, Eq, PartialEq, Hash)]
pub enum AnomalyType {
    MegaBid,
    MegaAsk,
    WhaleWall,
    RapidCancellation,
    MassCancellation,
    GhostOrder,
    PriceSpike,
    FlashCrash,
    StalePrice,
    OrderFlowSurge,
    ImbalanceSpike,
    BidAskFlip,
    LiquidityDrop,
    DepthGap,
    WallCollapse,
    Spoofing,
    Layering,
    WashTrading,
    ComplexPattern,
}

#[derive(Debug, Clone)]
pub struct AnomalyEvent {
    pub timestamp: DateTime<Local>,
    pub anomaly_type: AnomalyType,
    pub severity: u8,
    pub confidence: u8,
    pub price_level: Option<Decimal>,
    pub side: Option<OrderSide>,
    pub size: Option<Decimal>,
    pub percentage: Option<Decimal>,
    pub duration_ms: Option<u64>,
    pub frequency: Option<f64>,
    pub price_impact: Option<Decimal>,
    pub volume_impact: Option<Decimal>,
    pub description: String,
    pub details: HashMap<String, String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum OrderSide {
    Bid,
    Ask,
    Both,
}

#[derive(Debug, Clone, Default)]
pub struct AnomalyStats {
    pub total_events: u32,
    pub events_by_type: HashMap<AnomalyType, u32>,
    pub avg_severity: f64,
    pub max_severity: u8,
    pub last_minute_count: u32,
    pub last_hour_count: u32,
}

#[derive(Debug, Clone)]
pub struct AnomalyConfig {
    pub mega_bid_threshold: Decimal,
    pub mega_ask_threshold: Decimal,
    pub whale_wall_min_orders: usize,
    pub rapid_cancel_ms: u64,
    pub mass_cancel_threshold: usize,
    pub price_spike_bps: Decimal,
    pub stale_price_secs: u64,
    pub liquidity_drop_threshold: Decimal,
    pub depth_gap_bps: Decimal,
    pub order_surge_threshold: f64,
    pub imbalance_spike_threshold: Decimal,
    pub history_window_secs: u64,
    pub detection_cooldown_ms: u64,
}

impl Default for AnomalyConfig {
    fn default() -> Self {
        Self {
            mega_bid_threshold: rust_decimal_macros::dec!(20),
            mega_ask_threshold: rust_decimal_macros::dec!(20),
            whale_wall_min_orders: 3,
            rapid_cancel_ms: 100,
            mass_cancel_threshold: 10,
            price_spike_bps: rust_decimal_macros::dec!(50),
            stale_price_secs: 10,
            liquidity_drop_threshold: rust_decimal_macros::dec!(30),
            depth_gap_bps: rust_decimal_macros::dec!(100),
            order_surge_threshold: 5.0,
            imbalance_spike_threshold: rust_decimal_macros::dec!(50),
            history_window_secs: 60,
            detection_cooldown_ms: 1000,
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub(crate) enum ChangeType {
    New,
    Cancel,
    Update,
}
