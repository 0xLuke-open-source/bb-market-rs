// algorithms.rs (修复版本)
// 高级市场分析算法 - 使用现有的 OrderBookFeatures 字段

use rust_decimal::Decimal;
use rust_decimal_macros::dec;
use std::collections::{VecDeque};
use std::cmp::Reverse;
use chrono::{DateTime, Local};
use rust_decimal::prelude::{FromPrimitive, ToPrimitive};
use crate::store::l2_book::{OrderBook, OrderBookFeatures, TrendPeriod};

// ==================== 新增：多周期分析数据结构 ====================

#[derive(Debug, Clone)]
pub struct DivergenceSignal {
    pub period: String,
    pub direction: String,
    pub strength: u8,
    pub description: String,
}

#[derive(Debug, Clone)]
pub struct AccelerationCurve {
    pub micro: Decimal,   // 5s加速度
    pub short: Decimal,   // 1m加速度
    pub medium: Decimal,  // 5m加速度
    pub long: Decimal,    // 1h加速度
}

#[derive(Debug, Clone)]
pub struct TrendCoherence {
    pub coherence: String,
    pub std_deviation: Decimal,
    pub micro: Decimal,
    pub short: Decimal,
    pub medium: Decimal,
}

// ==================== 1. 鲸鱼检测算法 ====================

#[derive(Debug, Clone)]
pub struct WhaleDetectionResult {
    pub detected: bool,
    pub whale_type: WhaleType,
    pub whale_size: Decimal,           // 鲸鱼订单大小
    pub whale_price: Decimal,           // 鲸鱼价格
    pub total_whale_volume: Decimal,     // 总鲸鱼交易量
    pub whale_count: u32,                // 鲸鱼订单数量
    pub dominance_ratio: Decimal,        // 主导率 (鲸鱼量/总量)
    pub accumulation_score: Decimal,      // 吸筹评分 (0-100)
    pub distribution_score: Decimal,      // 出货评分 (0-100)
    pub intent_confidence: u8,            // 意图置信度
    pub whale_positions: Vec<WhalePosition>, // 鲸鱼仓位
}

#[derive(Debug, Clone)]
pub struct WhalePosition {
    pub side: OrderSide,
    pub price: Decimal,
    pub quantity: Decimal,
    pub percentage: Decimal,
    pub is_stealth: bool,
}

#[derive(Debug, Clone, PartialEq)]
pub enum OrderSide {
    Bid,
    Ask,
}

#[derive(Debug, Clone, PartialEq)]
pub enum WhaleType {
    Accumulator,      // 吸筹鲸鱼
    Distributor,      // 出货鲸鱼
    StealthWhale,     // 隐形鲸鱼 (拆单)
    HighFrequencyWhale, // 高频鲸鱼
    Unknown,
}

pub struct WhaleDetector {
    history_size: usize,
    bid_history: VecDeque<Vec<(Decimal, Decimal)>>,
    ask_history: VecDeque<Vec<(Decimal, Decimal)>>,
    whale_threshold: Decimal,           // 鲸鱼阈值 (默认5%)
}

// ==================== 2. Spoofing 识别算法 ====================

#[derive(Debug, Clone)]
pub struct SpoofingDetectionResult {
    pub detected: bool,
    pub confidence: u8,
    pub spoofing_type: SpoofingType,
    pub spoofing_levels: Vec<SpoofingLevel>,
    pub estimated_manipulation: Decimal,  // 估计的价格操纵幅度
}

#[derive(Debug, Clone, PartialEq)]
pub enum SpoofingType {
    BidSpoofing,     // 买单欺诈
    AskSpoofing,     // 卖单欺诈
    Layering,        // 分层欺诈
    WashTrading,     // 对倒
    Unknown,
}

#[derive(Debug, Clone)]
pub struct SpoofingLevel {
    pub price: Decimal,
    pub quantity: Decimal,
    pub side: OrderSide,
    pub lifetime_secs: f64,
    pub cancel_rate: f64,
}

pub struct SpoofingDetector {
    order_history: VecDeque<OrderBookSnapshot>,
}

#[derive(Debug, Clone)]
struct OrderBookSnapshot {
    timestamp: DateTime<Local>,
    bids: Vec<(Decimal, Decimal)>,
    asks: Vec<(Decimal, Decimal)>,
}

// ==================== 3. Pump / Dump 预测模型 ====================

#[derive(Debug, Clone)]
pub struct PumpDumpPrediction {
    pub pump_probability: u8,
    pub dump_probability: u8,
    pub pump_target: Decimal,
    pub dump_target: Decimal,
    pub time_horizon: String,
    pub confidence: u8,
    pub signals: Vec<PumpDumpSignal>,
}

#[derive(Debug, Clone)]
pub struct PumpDumpSignal {
    pub signal_type: SignalType,
    pub strength: u8,
    pub description: String,
}

#[derive(Debug, Clone, PartialEq)]
pub enum SignalType {
    VolumeSurge,
    OrderFlowImbalance,
    WhaleActivity,
    SpoofingDetected,
    PriceAcceleration,
    SupportBreak,
    ResistanceBreak,
}

pub struct PumpDumpPredictor {
    history: VecDeque<PriceVolumeSnapshot>,
    volume_surge_threshold: Decimal,
}

#[derive(Debug, Clone)]
struct PriceVolumeSnapshot {
    timestamp: DateTime<Local>,
    price: Decimal,
    volume: Decimal,
}

// ==================== 4. 做市商行为识别 ====================

#[derive(Debug, Clone)]
pub struct MarketMakerBehavior {
    pub is_active: bool,
    pub mm_type: MarketMakerType,
    pub strategy: MMStrategy,
    pub inventory_bias: Decimal,        // 库存偏向 (-100 到 100)
    pub spread_policy: SpreadPolicy,
    pub quote_frequency: f64,
}

#[derive(Debug, Clone, PartialEq)]
pub enum MarketMakerType {
    HighFrequencyMM,
    InstitutionalMM,
    RetailMM,
    Unknown,
}

#[derive(Debug, Clone, PartialEq)]
pub enum MMStrategy {
    SpreadCapture,      // 赚取价差
    InventoryMgmt,      // 库存管理
    PriceStabilization, // 价格稳定
    Directional,        // 方向性交易
}

#[derive(Debug, Clone, PartialEq)]
pub enum SpreadPolicy {
    Tight,      // 紧价差 (< 10bps)
    Normal,     // 正常价差 (10-30bps)
    Wide,       // 宽价差 (> 30bps)
}

pub struct MarketMakerDetector {
    quote_history: VecDeque<QuoteSnapshot>,
}

#[derive(Debug, Clone)]
struct QuoteSnapshot {
    timestamp: DateTime<Local>,
    bid_price: Decimal,
    ask_price: Decimal,
}

// ==================== 5. 订单流 Alpha 信号 ====================

#[derive(Debug, Clone)]
pub struct OrderFlowAlpha {
    pub signal: AlphaSignal,
    pub strength: u8,
    pub confidence: u8,
    pub expected_return: Decimal,
    pub time_horizon: String,
    pub components: Vec<AlphaComponent>,
}

#[derive(Debug, Clone, PartialEq)]
pub enum AlphaSignal {
    StrongBuy,
    Buy,
    Neutral,
    Sell,
    StrongSell,
}

#[derive(Debug, Clone)]
pub struct AlphaComponent {
    pub name: String,
    pub value: Decimal,
    pub contribution: u8,
}

pub struct OrderFlowAlphaGenerator {
    history: VecDeque<OrderFlowSnapshot>,
}

#[derive(Debug, Clone)]
struct OrderFlowSnapshot {
    timestamp: DateTime<Local>,
    ofi: Decimal,
    obi: Decimal,
    price: Decimal,
}

// ==================== 所有算法的实现 ====================


mod whale;
mod spoofing;
mod pump;
mod market_maker;
mod alpha;
mod intelligence;

pub struct MarketIntelligence {
    pub whale_detector: WhaleDetector,
    pub spoofing_detector: SpoofingDetector,
    pub pump_dump_predictor: PumpDumpPredictor,
    pub mm_detector: MarketMakerDetector,
    pub alpha_generator: OrderFlowAlphaGenerator,
}

#[derive(Debug, Clone)]
pub struct ComprehensiveAnalysis {
    pub timestamp: chrono::DateTime<chrono::Local>,
    pub whale: WhaleDetectionResult,
    pub spoofing: SpoofingDetectionResult,
    pub pump_dump: PumpDumpPrediction,
    pub market_maker: MarketMakerBehavior,
    pub alpha: OrderFlowAlpha,

    // 综合评分
    pub overall_sentiment: OverallSentiment,
    pub risk_level: RiskLevel,
    pub trading_recommendation: TradingRecommendation,
}

#[derive(Debug, Clone, PartialEq)]
pub enum OverallSentiment {
    StrongBullish,
    Bullish,
    Neutral,
    Bearish,
    StrongBearish,
}

#[derive(Debug, Clone, PartialEq)]
pub enum RiskLevel {
    VeryLow,
    Low,
    Medium,
    High,
    VeryHigh,
}

#[derive(Debug, Clone, PartialEq)]
pub enum TradingRecommendation {
    StrongBuy,
    Buy,
    Neutral,
    Sell,
    StrongSell,
    Wait,
}

