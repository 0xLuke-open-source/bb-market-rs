//! 高级市场分析算法总入口。
//!
//! 这一层不直接处理网络消息，而是消费 `OrderBook` 与 `OrderBookFeatures`，
//! 对当前盘口做多种维度的解释：
//! - 鲸鱼行为
//! - spoofing / layering
//! - pump / dump 概率
//! - 做市商行为
//! - 订单流 alpha
//!
//! `MarketIntelligence` 会把这些子算法组合成统一分析结果。

use crate::market_data::domain::order_book::{OrderBook, OrderBookFeatures, TrendPeriod};
use chrono::{DateTime, Local};
use rust_decimal::prelude::{FromPrimitive, ToPrimitive};
use rust_decimal::Decimal;
use rust_decimal_macros::dec;
use std::cmp::Reverse;
use std::collections::{HashMap, VecDeque};
use std::time::Instant;

// ==================== 多周期分析辅助结构 ====================

/// 多周期背离信号。
#[derive(Debug, Clone)]
pub struct DivergenceSignal {
    pub period: String,
    pub direction: String,
    pub strength: u8,
    pub description: String,
}

/// 多周期加速度曲线。
#[derive(Debug, Clone)]
pub struct AccelerationCurve {
    pub micro: Decimal,  // 5s加速度
    pub short: Decimal,  // 1m加速度
    pub medium: Decimal, // 5m加速度
    pub long: Decimal,   // 1h加速度
}

/// 多周期趋势一致性摘要。
#[derive(Debug, Clone)]
pub struct TrendCoherence {
    pub coherence: String,
    pub std_deviation: Decimal,
    pub micro: Decimal,
    pub short: Decimal,
    pub medium: Decimal,
}

// ==================== 1. 鲸鱼检测算法 ====================

/// 鲸鱼检测结果。
#[derive(Debug, Clone)]
pub struct WhaleDetectionResult {
    pub detected: bool,
    pub whale_type: WhaleType,
    pub whale_size: Decimal,                 // 鲸鱼订单大小
    pub whale_price: Decimal,                // 鲸鱼价格
    pub total_whale_volume: Decimal,         // 总鲸鱼交易量
    pub whale_count: u32,                    // 鲸鱼订单数量
    pub dominance_ratio: Decimal,            // 主导率 (鲸鱼量/总量)
    pub accumulation_score: Decimal,         // 吸筹评分 (0-100)
    pub distribution_score: Decimal,         // 出货评分 (0-100)
    pub intent_confidence: u8,               // 意图置信度
    pub whale_positions: Vec<WhalePosition>, // 鲸鱼仓位
}

/// 识别出的单个鲸鱼挂单位置。
#[derive(Debug, Clone)]
pub struct WhalePosition {
    pub side: OrderSide,
    pub price: Decimal,
    pub quantity: Decimal,
    pub percentage: Decimal,
    pub is_stealth: bool,
}

/// 买卖方向枚举，供多个子算法复用。
#[derive(Debug, Clone, PartialEq)]
pub enum OrderSide {
    Bid,
    Ask,
}

/// 鲸鱼类型的粗粒度分类。
#[derive(Debug, Clone, PartialEq)]
pub enum WhaleType {
    Accumulator,        // 吸筹鲸鱼
    Distributor,        // 出货鲸鱼
    StealthWhale,       // 隐形鲸鱼 (拆单)
    HighFrequencyWhale, // 高频鲸鱼
    Unknown,
}

/// 鲸鱼检测器，会维护最近若干轮盘口历史来识别拆单行为。
pub struct WhaleDetector {
    history_size: usize,
    bid_history: VecDeque<Vec<(Decimal, Decimal)>>,
    ask_history: VecDeque<Vec<(Decimal, Decimal)>>,
    whale_threshold: Decimal, // 鲸鱼阈值 (默认5%)
}

// ==================== 2. Spoofing 识别算法 ====================

/// Spoofing 检测结果。
#[derive(Debug, Clone)]
pub struct SpoofingDetectionResult {
    pub detected: bool,
    pub confidence: u8,
    pub spoofing_type: SpoofingType,
    pub spoofing_levels: Vec<SpoofingLevel>,
    pub estimated_manipulation: Decimal, // 估计的价格操纵幅度
}

/// 欺骗性交易的细分类别。
#[derive(Debug, Clone, PartialEq)]
pub enum SpoofingType {
    BidSpoofing, // 买单欺诈
    AskSpoofing, // 卖单欺诈
    Layering,    // 分层欺诈
    WashTrading, // 对倒
    Unknown,
}

/// 被识别为可疑的盘口层级。
#[derive(Debug, Clone)]
pub struct SpoofingLevel {
    pub price: Decimal,
    pub quantity: Decimal,
    pub side: OrderSide,
    pub lifetime_secs: f64,
    pub cancel_rate: f64,
}

/// 维护盘口快照历史的 spoofing 检测器。
pub struct SpoofingDetector {
    order_history: VecDeque<OrderBookSnapshot>,
    /// 订单挂单时刻追踪器：key = (price_str, side_str)，value = 首次出现时间
    /// 当档位消失时计算真实存活时间，用于 spoofing 判断
    lifetime_tracker: HashMap<(String, String), Instant>,
    /// 消失档位的历史（最近100条），记录 (lifetime_secs, cancel_count)
    cancel_history: VecDeque<f64>, // 历史 lifetime_secs
}

/// 简化后的订单簿快照，只保留 spoofing 需要的前几档信息。
#[derive(Debug, Clone)]
struct OrderBookSnapshot {
    timestamp: DateTime<Local>,
    bids: Vec<(Decimal, Decimal)>,
    asks: Vec<(Decimal, Decimal)>,
}

// ==================== 3. Pump / Dump 预测模型 ====================

/// 短线 pump / dump 预测结果。
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

/// 促成预测结论的单个信号项。
#[derive(Debug, Clone)]
pub struct PumpDumpSignal {
    pub signal_type: SignalType,
    pub strength: u8,
    pub description: String,
}

/// 预测模型内部使用的信号类型枚举。
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

/// Pump / Dump 预测器，会维护价格与深度历史。
pub struct PumpDumpPredictor {
    history: VecDeque<PriceVolumeSnapshot>,
    volume_surge_threshold: Decimal,
}

/// 面向预测器的轻量价格/成交量快照。
#[derive(Debug, Clone)]
struct PriceVolumeSnapshot {
    timestamp: DateTime<Local>,
    price: Decimal,
    volume: Decimal,
}

// ==================== 4. 做市商行为识别 ====================

/// 做市商行为总结。
#[derive(Debug, Clone)]
pub struct MarketMakerBehavior {
    pub is_active: bool,
    pub mm_type: MarketMakerType,
    pub strategy: MMStrategy,
    pub inventory_bias: Decimal, // 库存偏向 (-100 到 100)
    pub spread_policy: SpreadPolicy,
    pub quote_frequency: f64,
}

/// 做市商类型的粗粒度分类。
#[derive(Debug, Clone, PartialEq)]
pub enum MarketMakerType {
    HighFrequencyMM,
    InstitutionalMM,
    RetailMM,
    Unknown,
}

/// 估计出的做市策略。
#[derive(Debug, Clone, PartialEq)]
pub enum MMStrategy {
    SpreadCapture,      // 赚取价差
    InventoryMgmt,      // 库存管理
    PriceStabilization, // 价格稳定
    Directional,        // 方向性交易
}

/// 报价价差风格分类。
#[derive(Debug, Clone, PartialEq)]
pub enum SpreadPolicy {
    Tight,  // 紧价差 (< 10bps)
    Normal, // 正常价差 (10-30bps)
    Wide,   // 宽价差 (> 30bps)
}

/// 基于持续报价特征识别做市行为。
pub struct MarketMakerDetector {
    quote_history: VecDeque<QuoteSnapshot>,
}

/// 做市分析使用的最小报价快照。
#[derive(Debug, Clone)]
struct QuoteSnapshot {
    timestamp: DateTime<Local>,
    bid_price: Decimal,
    ask_price: Decimal,
}

// ==================== 5. 订单流 Alpha 信号 ====================

/// 订单流 alpha 结果。
#[derive(Debug, Clone)]
pub struct OrderFlowAlpha {
    pub signal: AlphaSignal,
    pub strength: u8,
    pub confidence: u8,
    pub expected_return: Decimal,
    pub time_horizon: String,
    pub components: Vec<AlphaComponent>,
}

/// 交易方向信号。
#[derive(Debug, Clone, PartialEq)]
pub enum AlphaSignal {
    StrongBuy,
    Buy,
    Neutral,
    Sell,
    StrongSell,
}

/// 构成 alpha 的单个评分项。
#[derive(Debug, Clone)]
pub struct AlphaComponent {
    pub name: String,
    pub value: Decimal,
    pub contribution: u8,
}

/// 订单流 alpha 生成器。
pub struct OrderFlowAlphaGenerator {
    history: VecDeque<OrderFlowSnapshot>,
}

/// Alpha 生成器内部保留的简化历史快照。
#[derive(Debug, Clone)]
struct OrderFlowSnapshot {
    timestamp: DateTime<Local>,
    ofi: Decimal,
    obi: Decimal,
    price: Decimal,
}

// ==================== 子模块实现 ====================

mod alpha;
mod intelligence;
mod market_maker;
mod pump;
mod spoofing;
mod whale;

/// 高级市场智能分析器。
///
/// 它不定义指标，而是负责把多个独立算法的结果汇总到统一出口。
pub struct MarketIntelligence {
    pub whale_detector: WhaleDetector,
    pub spoofing_detector: SpoofingDetector,
    pub pump_dump_predictor: PumpDumpPredictor,
    pub mm_detector: MarketMakerDetector,
    pub alpha_generator: OrderFlowAlphaGenerator,
}

/// 综合分析结果。
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

/// 总体市场情绪。
#[derive(Debug, Clone, PartialEq)]
pub enum OverallSentiment {
    StrongBullish,
    Bullish,
    Neutral,
    Bearish,
    StrongBearish,
}

/// 风险等级。
#[derive(Debug, Clone, PartialEq)]
pub enum RiskLevel {
    VeryLow,
    Low,
    Medium,
    High,
    VeryHigh,
}

/// 最终交易建议。
#[derive(Debug, Clone, PartialEq)]
pub enum TradingRecommendation {
    StrongBuy,
    Buy,
    Neutral,
    Sell,
    StrongSell,
    Wait,
}
