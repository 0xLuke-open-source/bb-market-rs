// algorithms.rs (修复版本)
// 高级市场分析算法 - 使用现有的 OrderBookFeatures 字段

use rust_decimal::Decimal;
use rust_decimal_macros::dec;
use std::collections::{VecDeque};
use std::cmp::Reverse;
use chrono::{DateTime, Local};
use rust_decimal::prelude::ToPrimitive;
use crate::store::l2_book::{OrderBook, OrderBookFeatures};

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

impl WhaleDetector {
    pub fn new() -> Self {
        Self {
            history_size: 100,
            bid_history: VecDeque::with_capacity(100),
            ask_history: VecDeque::with_capacity(100),
            whale_threshold: dec!(0.05),    // 5%
        }
    }

    pub fn detect_whales(&mut self, book: &OrderBook, features: &OrderBookFeatures) -> WhaleDetectionResult {
        self.update_history(book);

        let mut result = WhaleDetectionResult {
            detected: false,
            whale_type: WhaleType::Unknown,
            whale_size: Decimal::ZERO,
            whale_price: Decimal::ZERO,
            total_whale_volume: Decimal::ZERO,
            whale_count: 0,
            dominance_ratio: Decimal::ZERO,
            accumulation_score: Decimal::ZERO,
            distribution_score: Decimal::ZERO,
            intent_confidence: 0,
            whale_positions: Vec::new(),
        };

        // 检测大单
        let (positions, total_vol, count) = self.find_whale_positions(book, features);
        result.whale_positions = positions;
        result.total_whale_volume = total_vol;
        result.whale_count = count;

        // 计算主导率
        let total_volume = features.total_bid_volume + features.total_ask_volume;
        if total_volume > Decimal::ZERO {
            result.dominance_ratio = result.total_whale_volume / total_volume * dec!(100);
        }

        // 判断是否有鲸鱼
        if count > 0 {
            result.detected = true;
            if let Some(pos) = result.whale_positions.first() {
                result.whale_size = pos.quantity;
                result.whale_price = pos.price;
            }
        }

        // 判断鲸鱼类型
        result.whale_type = self.determine_whale_type(&result, features);

        // 计算意图评分
        let (acc_score, dist_score) = self.calculate_intent_scores(&result, features);
        result.accumulation_score = acc_score;
        result.distribution_score = dist_score;

        // 计算置信度
        result.intent_confidence = self.calculate_confidence(&result, features);

        result
    }

    fn update_history(&mut self, book: &OrderBook) {
        let bids: Vec<(Decimal, Decimal)> = book.bids.iter()
            .take(50)
            .map(|(Reverse(p), q)| (*p, *q))
            .collect();
        let asks: Vec<(Decimal, Decimal)> = book.asks.iter()
            .take(50)
            .map(|(p, q)| (*p, *q))
            .collect();

        self.bid_history.push_back(bids);
        self.ask_history.push_back(asks);

        if self.bid_history.len() > self.history_size {
            self.bid_history.pop_front();
            self.ask_history.pop_front();
        }
    }

    fn find_whale_positions(&self, book: &OrderBook, features: &OrderBookFeatures) -> (Vec<WhalePosition>, Decimal, u32) {
        let mut positions = Vec::new();
        let mut total_vol = Decimal::ZERO;
        let mut count = 0;

        // 检测买单中的鲸鱼
        for (Reverse(price), qty) in book.bids.iter().take(30) {
            if qty > &(features.total_bid_volume * self.whale_threshold) {
                let is_stealth = self.detect_stealth_order(*price, *qty, true);
                positions.push(WhalePosition {
                    side: OrderSide::Bid,
                    price: *price,
                    quantity: *qty,
                    percentage: qty / features.total_bid_volume * dec!(100),
                    is_stealth,
                });
                total_vol += qty;
                count += 1;
            }
        }

        // 检测卖单中的鲸鱼
        for (price, qty) in book.asks.iter().take(30) {
            if qty > &(features.total_ask_volume * self.whale_threshold) {
                let is_stealth = self.detect_stealth_order(*price, *qty, false);
                positions.push(WhalePosition {
                    side: OrderSide::Ask,
                    price: *price,
                    quantity: *qty,
                    percentage: qty / features.total_ask_volume * dec!(100),
                    is_stealth,
                });
                total_vol += qty;
                count += 1;
            }
        }

        (positions, total_vol, count)
    }

    fn detect_stealth_order(&self, price: Decimal, quantity: Decimal, is_bid: bool) -> bool {
        let history = if is_bid { &self.bid_history } else { &self.ask_history };
        if history.is_empty() { return false; }

        let mut nearby_total = Decimal::ZERO;
        let mut similar_orders = 0;

        if let Some(latest) = history.back() {
            for &(p, q) in latest {
                if (p - price).abs() < dec!(0.0001) {
                    nearby_total += q;
                    similar_orders += 1;
                }
            }
        }

        // 如果附近有多个小单且总量接近当前大单，可能是拆单
        similar_orders > 3 && (nearby_total - quantity).abs() < quantity * dec!(0.2)
    }

    fn determine_whale_type(&self, result: &WhaleDetectionResult, features: &OrderBookFeatures) -> WhaleType {
        let bid_whales = result.whale_positions.iter().filter(|p| p.side == OrderSide::Bid).count();
        let ask_whales = result.whale_positions.iter().filter(|p| p.side == OrderSide::Ask).count();
        let stealth_count = result.whale_positions.iter().filter(|p| p.is_stealth).count();

        if stealth_count > result.whale_positions.len() / 2 {
            return WhaleType::StealthWhale;
        }

        if features.ofi.abs() > dec!(1000000) && result.whale_count > 5 {
            return WhaleType::HighFrequencyWhale;
        }

        if bid_whales > ask_whales && features.obi > dec!(10) {
            return WhaleType::Accumulator;
        } else if ask_whales > bid_whales && features.obi < dec!(-10) {
            return WhaleType::Distributor;
        }

        WhaleType::Unknown
    }

    fn calculate_intent_scores(&self, result: &WhaleDetectionResult, features: &OrderBookFeatures) -> (Decimal, Decimal) {
        let mut acc_score = dec!(50);
        let mut dist_score = dec!(50);

        // 基于OBI
        if features.obi > dec!(20) { acc_score += dec!(20); }
        else if features.obi < dec!(-20) { dist_score += dec!(20); }

        // 基于OFI
        if features.ofi > dec!(100000) { acc_score += dec!(15); }
        else if features.ofi < dec!(-100000) { dist_score += dec!(15); }

        // 基于斜率
        if features.slope_bid > dec!(1000000) { acc_score += dec!(15); }
        if features.slope_ask < dec!(-1000000) { dist_score += dec!(15); }

        // 基于鲸鱼位置
        for pos in &result.whale_positions {
            match pos.side {
                OrderSide::Bid => acc_score += pos.percentage / dec!(5),
                OrderSide::Ask => dist_score += pos.percentage / dec!(5),
            }
        }

        (acc_score.min(dec!(100)), dist_score.min(dec!(100)))
    }

    fn calculate_confidence(&self, result: &WhaleDetectionResult, features: &OrderBookFeatures) -> u8 {
        let mut conf = 50;

        if result.detected { conf += 20; }
        if result.dominance_ratio > dec!(30) { conf += 15; }
        if (result.accumulation_score - result.distribution_score).abs() > dec!(30) { conf += 15; }
        if features.whale_entry || features.whale_exit { conf += 10; }

        conf.min(100).max(0) as u8
    }
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

impl SpoofingDetector {
    pub fn new() -> Self {
        Self {
            order_history: VecDeque::with_capacity(100),
        }
    }

    pub fn detect_spoofing(&mut self, book: &OrderBook, features: &OrderBookFeatures) -> SpoofingDetectionResult {
        self.update_history(book);

        let mut result = SpoofingDetectionResult {
            detected: false,
            confidence: 0,
            spoofing_type: SpoofingType::Unknown,
            spoofing_levels: Vec::new(),
            estimated_manipulation: Decimal::ZERO,
        };

        // 检测买单欺诈
        let bid_spoofing = self.detect_bid_spoofing(book, features);
        if bid_spoofing.detected {
            result.detected = true;
            result.spoofing_type = SpoofingType::BidSpoofing;
            result.spoofing_levels.extend(bid_spoofing.levels);
        }

        // 检测卖单欺诈
        let ask_spoofing = self.detect_ask_spoofing(book, features);
        if ask_spoofing.detected {
            result.detected = true;
            result.spoofing_type = SpoofingType::AskSpoofing;
            result.spoofing_levels.extend(ask_spoofing.levels);
        }

        // 检测分层欺诈
        if self.detect_layering(book) {
            result.detected = true;
            result.spoofing_type = SpoofingType::Layering;
        }

        // 检测对倒
        if self.detect_wash_trading(features) {
            result.detected = true;
            result.spoofing_type = SpoofingType::WashTrading;
        }

        if result.detected {
            result.confidence = self.calculate_spoofing_confidence(features);
            result.estimated_manipulation = self.estimate_price_manipulation(&result);
        }

        result
    }

    fn update_history(&mut self, book: &OrderBook) {
        let snapshot = OrderBookSnapshot {
            timestamp: Local::now(),
            bids: book.bids.iter().take(20).map(|(Reverse(p), q)| (*p, *q)).collect(),
            asks: book.asks.iter().take(20).map(|(p, q)| (*p, *q)).collect(),
        };

        self.order_history.push_back(snapshot);
        if self.order_history.len() > 100 {
            self.order_history.pop_front();
        }
    }

    fn detect_bid_spoofing(&self, book: &OrderBook, features: &OrderBookFeatures) -> SpoofingDetection {
        let mut detection = SpoofingDetection::new();
        let (best_bid, _) = book.best_bid_ask().unwrap_or((Decimal::ZERO, Decimal::ZERO));

        for (Reverse(price), qty) in book.bids.iter().take(10) {
            // 检查是否是大单且远离盘口
            let distance = (best_bid - *price).abs();
            if qty > &(features.total_bid_volume * dec!(0.1)) && distance > features.spread * dec!(5) {
                detection.detected = true;
                detection.levels.push(SpoofingLevel {
                    price: *price,
                    quantity: *qty,
                    side: OrderSide::Bid,
                    lifetime_secs: 5.0, // 简化值
                    cancel_rate: 0.4,    // 简化值
                });
            }
        }

        detection
    }

    fn detect_ask_spoofing(&self, book: &OrderBook, features: &OrderBookFeatures) -> SpoofingDetection {
        let mut detection = SpoofingDetection::new();
        let (_, best_ask) = book.best_bid_ask().unwrap_or((Decimal::ZERO, Decimal::ZERO));

        for (price, qty) in book.asks.iter().take(10) {
            let distance = (*price - best_ask).abs();
            if qty > &(features.total_ask_volume * dec!(0.1)) && distance > features.spread * dec!(5) {
                detection.detected = true;
                detection.levels.push(SpoofingLevel {
                    price: *price,
                    quantity: *qty,
                    side: OrderSide::Ask,
                    lifetime_secs: 5.0,
                    cancel_rate: 0.4,
                });
            }
        }

        detection
    }

    fn detect_layering(&self, book: &OrderBook) -> bool {
        // 检测分层挂单（多个价格层级都有大单）
        let mut bid_layers = 0;
        let mut ask_layers = 0;

        for (_, qty) in book.bids.iter().take(5) {
            if qty > &dec!(100000) { bid_layers += 1; }
        }
        for (_, qty) in book.asks.iter().take(5) {
            if qty > &dec!(100000) { ask_layers += 1; }
        }

        bid_layers >= 3 || ask_layers >= 3
    }

    fn detect_wash_trading(&self, features: &OrderBookFeatures) -> bool {
        // 检测对倒：高OFI但价格不变
        features.ofi.abs() > dec!(500000) && features.price_change.abs() < dec!(0.1)
    }

    fn calculate_spoofing_confidence(&self, features: &OrderBookFeatures) -> u8 {
        let mut conf = 50;

        if features.bid_concentration > dec!(80) || features.ask_concentration > dec!(80) {
            conf += 20;
        }

        if features.fake_breakout {
            conf += 20;
        }

        conf.min(100).max(0) as u8
    }


    fn estimate_price_manipulation(&self, result: &SpoofingDetectionResult) -> Decimal {
        let mut manipulation = Decimal::ZERO;

        for level in &result.spoofing_levels {
            manipulation += level.quantity * level.price / dec!(1000000);
        }

        manipulation
    }
}

#[derive(Debug, Clone)]
struct SpoofingDetection {
    detected: bool,
    levels: Vec<SpoofingLevel>,
}

impl SpoofingDetection {
    fn new() -> Self {
        Self {
            detected: false,
            levels: Vec::new(),
        }
    }
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

impl PumpDumpPredictor {
    pub fn new() -> Self {
        Self {
            history: VecDeque::with_capacity(100),
            volume_surge_threshold: dec!(2.0),
        }
    }

    pub fn predict(&mut self, book: &OrderBook, features: &OrderBookFeatures) -> PumpDumpPrediction {
        self.update_history(book, features);

        let mut signals = Vec::new();
        let mut pump_prob = 0;
        let mut dump_prob = 0;

        // 检测成交量激增
        if let Some(vol_signal) = self.detect_volume_surge(features) {
            signals.push(vol_signal);
            pump_prob += 15;
            dump_prob += 15;
        }

        // 检测订单流失衡
        if let Some(ofi_signal) = self.detect_orderflow_imbalance(features) {
            signals.push(ofi_signal);
            if features.ofi > Decimal::ZERO {
                pump_prob += 20;
            } else {
                dump_prob += 20;
            }
        }

        // 检测鲸鱼活动
        if let Some(whale_signal) = self.detect_whale_activity(features) {
            signals.push(whale_signal);
            if features.whale_entry {
                pump_prob += 25;
            }
            if features.whale_exit {
                dump_prob += 25;
            }
        }

        // 检测价格加速
        if features.price_change.abs() > dec!(0.5) {
            if features.price_change > Decimal::ZERO {
                pump_prob += 15;
            } else {
                dump_prob += 15;
            }
        }

        // 检测支撑/阻力突破
        if let Some(break_signal) = self.detect_level_break(book, features) {
            if break_signal.signal_type == SignalType::ResistanceBreak {
                pump_prob += 30;
            } else {
                dump_prob += 30;
            }
            signals.push(break_signal);
        }


        // 计算目标位
        let (pump_target, dump_target) = self.calculate_targets(book, features);

        // 计算置信度
        let confidence = ((pump_prob.max(dump_prob) as f64) * 0.8) as u8;

        PumpDumpPrediction {
            pump_probability: pump_prob.min(100),
            dump_probability: dump_prob.min(100),
            pump_target,
            dump_target,
            time_horizon: "5-15分钟".to_string(),
            confidence,
            signals,
        }
    }

    fn update_history(&mut self, book: &OrderBook, features: &OrderBookFeatures) {
        let (best_bid, best_ask) = book.best_bid_ask().unwrap_or((Decimal::ZERO, Decimal::ZERO));
        let mid_price = (best_bid + best_ask) / dec!(2);

        let snapshot = PriceVolumeSnapshot {
            timestamp: Local::now(),
            price: mid_price,
            volume: features.bid_volume_depth + features.ask_volume_depth,
        };

        self.history.push_back(snapshot);
        if self.history.len() > 100 {
            self.history.pop_front();
        }
    }

    fn detect_volume_surge(&self, features: &OrderBookFeatures) -> Option<PumpDumpSignal> {
        if self.history.len() < 10 {
            return None;
        }

        let avg_volume: Decimal = self.history.iter().rev().take(10).map(|h| h.volume).sum();
        let avg_volume = avg_volume / Decimal::from(10);
        let current_volume = features.bid_volume_depth + features.ask_volume_depth;

        if current_volume > avg_volume * self.volume_surge_threshold {
            Some(PumpDumpSignal {
                signal_type: SignalType::VolumeSurge,
                strength: ((current_volume / avg_volume).to_u64().unwrap_or(2) as u8).min(100),
                description: format!("成交量激增 {:.1}倍", current_volume / avg_volume),
            })
        } else {
            None
        }
    }

    fn detect_orderflow_imbalance(&self, features: &OrderBookFeatures) -> Option<PumpDumpSignal> {
        if features.ofi.abs() > dec!(200000) {
            let strength = (features.ofi.abs() / dec!(10000)).to_u64().unwrap_or(50) as u8;
            Some(PumpDumpSignal {
                signal_type: SignalType::OrderFlowImbalance,
                strength: strength.min(100),
                description: format!("订单流{} {:.0}",
                                     if features.ofi > Decimal::ZERO { "偏多" } else { "偏空" },
                                     features.ofi),
            })
        } else {
            None
        }
    }

    fn detect_whale_activity(&self, features: &OrderBookFeatures) -> Option<PumpDumpSignal> {
        if features.whale_entry {
            Some(PumpDumpSignal {
                signal_type: SignalType::WhaleActivity,
                strength: 80,
                description: "鲸鱼进场".to_string(),
            })
        } else if features.whale_exit {
            Some(PumpDumpSignal {
                signal_type: SignalType::WhaleActivity,
                strength: 80,
                description: "鲸鱼离场".to_string(),
            })
        } else {
            None
        }
    }

    fn detect_level_break(&self, book: &OrderBook, features: &OrderBookFeatures) -> Option<PumpDumpSignal> {
        let (best_bid, best_ask) = book.best_bid_ask().unwrap_or((Decimal::ZERO, Decimal::ZERO));

        // 检测阻力突破
        for (price, qty) in book.asks.iter().take(3) {
            if *price > best_ask && qty < &dec!(1000) && features.ofi > dec!(100000) {
                return Some(PumpDumpSignal {
                    signal_type: SignalType::ResistanceBreak,
                    strength: 85,
                    description: format!("阻力突破 {:.6}", price),
                });
            }
        }

        // 检测支撑跌破
        for (Reverse(price), qty) in book.bids.iter().take(3) {
            if *price < best_bid && qty < &dec!(1000) && features.ofi < dec!(-100000) {
                return Some(PumpDumpSignal {
                    signal_type: SignalType::SupportBreak,
                    strength: 85,
                    description: format!("支撑跌破 {:.6}", price),
                });
            }
        }

        None
    }

    fn calculate_targets(&self, book: &OrderBook, features: &OrderBookFeatures) -> (Decimal, Decimal) {
        let (best_bid, best_ask) = book.best_bid_ask().unwrap_or((Decimal::ZERO, Decimal::ZERO));

        // 泵目标：下一个阻力位
        let pump_target = book.asks.iter()
            .skip(2)
            .next()
            .map(|(p, _)| *p)
            .unwrap_or(best_ask * dec!(1.05));

        // 砸目标：下一个支撑位
        let dump_target = book.bids.iter()
            .skip(2)
            .next()
            .map(|(Reverse(p), _)| *p)
            .unwrap_or(best_bid * dec!(0.95));

        (pump_target, dump_target)
    }
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

impl MarketMakerDetector {
    pub fn new() -> Self {
        Self {
            quote_history: VecDeque::with_capacity(1000),
        }
    }

    pub fn detect(&mut self, book: &OrderBook, features: &OrderBookFeatures) -> MarketMakerBehavior {
        self.update_history(book);

        let mm_type = self.determine_mm_type(features);
        let strategy = self.determine_strategy(features);
        let inventory_bias = self.calculate_inventory_bias(features);
        let spread_policy = self.determine_spread_policy(features);
        let quote_frequency = self.calculate_quote_frequency();

        MarketMakerBehavior {
            is_active: quote_frequency > 0.1,
            mm_type,
            strategy,
            inventory_bias,
            spread_policy,
            quote_frequency,
        }
    }

    fn update_history(&mut self, book: &OrderBook) {
        if let Some((bid, ask)) = book.best_bid_ask() {
            let snapshot = QuoteSnapshot {
                timestamp: Local::now(),
                bid_price: bid,
                ask_price: ask,
            };

            self.quote_history.push_back(snapshot);
            if self.quote_history.len() > 1000 {
                self.quote_history.pop_front();
            }
        }
    }

    fn determine_mm_type(&self, features: &OrderBookFeatures) -> MarketMakerType {
        if features.slope_bid.abs() < dec!(100000) && features.slope_ask.abs() < dec!(100000) {
            MarketMakerType::HighFrequencyMM
        } else if features.max_bid_ratio > dec!(20) || features.max_ask_ratio > dec!(20) {
            MarketMakerType::InstitutionalMM
        } else {
            MarketMakerType::RetailMM
        }
    }

    fn determine_strategy(&self, features: &OrderBookFeatures) -> MMStrategy {
        let spread = features.spread_bps;
        let inventory_bias = self.calculate_inventory_bias(features);

        if spread < dec!(10) && inventory_bias.abs() < dec!(20) {
            MMStrategy::SpreadCapture
        } else if inventory_bias.abs() > dec!(50) {
            MMStrategy::InventoryMgmt
        } else if features.price_change.abs() < dec!(0.1) && features.obi.abs() < dec!(10) {
            MMStrategy::PriceStabilization
        } else {
            MMStrategy::Directional
        }
    }

    fn calculate_inventory_bias(&self, features: &OrderBookFeatures) -> Decimal {
        let bid_depth = features.bid_volume_depth;
        let ask_depth = features.ask_volume_depth;
        let total = bid_depth + ask_depth;

        if total > Decimal::ZERO {
            (bid_depth - ask_depth) / total * dec!(100)
        } else {
            Decimal::ZERO
        }
    }

    fn determine_spread_policy(&self, features: &OrderBookFeatures) -> SpreadPolicy {
        let bps = features.spread_bps;

        if bps < dec!(10) {
            SpreadPolicy::Tight
        } else if bps < dec!(30) {
            SpreadPolicy::Normal
        } else {
            SpreadPolicy::Wide
        }
    }

    fn calculate_quote_frequency(&self) -> f64 {
        if self.quote_history.len() < 2 {
            return 0.0;
        }

        let time_span = (self.quote_history.back().unwrap().timestamp -
            self.quote_history.front().unwrap().timestamp).num_seconds();

        if time_span > 0 {
            self.quote_history.len() as f64 / time_span as f64
        } else {
            0.0
        }
    }
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

impl OrderFlowAlphaGenerator {
    pub fn new() -> Self {
        Self {
            history: VecDeque::with_capacity(200),
        }
    }

    pub fn generate(&mut self, book: &OrderBook, features: &OrderBookFeatures) -> OrderFlowAlpha {
        self.update_history(book, features);

        let mut components = Vec::new();
        let mut total_score = 0;

        // 1. OFI动量
        let ofi_momentum = self.calculate_ofi_momentum(features);
        components.push(AlphaComponent {
            name: "OFI动量".to_string(),
            value: ofi_momentum,
            contribution: (ofi_momentum.abs().to_u64().unwrap_or(0) / 1000) as u8,
        });
        total_score += ofi_momentum.to_i64().unwrap_or(0) / 1000;

        // 2. OBI失衡
        let obi_score = features.obi;
        components.push(AlphaComponent {
            name: "OBI失衡".to_string(),
            value: obi_score,
            contribution: obi_score.abs().to_u64().unwrap_or(0) as u8,
        });
        total_score += obi_score.to_i64().unwrap_or(0);

        // 3. 斜率差异
        let slope_diff = features.slope_bid - features.slope_ask;
        components.push(AlphaComponent {
            name: "斜率差异".to_string(),
            value: slope_diff / dec!(1000000),
            contribution: (slope_diff.abs().to_u64().unwrap_or(0) / 1000000) as u8,
        });
        total_score += (slope_diff / dec!(1000000)).to_i64().unwrap_or(0);

        // 4. 价格压力
        let pressure_score = features.price_pressure * dec!(1000);
        components.push(AlphaComponent {
            name: "价格压力".to_string(),
            value: features.price_pressure,
            contribution: pressure_score.abs().to_u64().unwrap_or(0) as u8,
        });
        total_score += pressure_score.to_i64().unwrap_or(0);

        // 5. 鲸鱼活动
        let whale_score: i64 = if features.whale_entry { 50 } else if features.whale_exit { -50 } else { 0 };
        components.push(AlphaComponent {
            name: "鲸鱼活动".to_string(),
            value: Decimal::from(whale_score),
            contribution: whale_score.abs() as u8,
        });
        total_score += whale_score;

        // 6. 累计Delta
        let delta_score = features.cum_delta / dec!(1000000);
        components.push(AlphaComponent {
            name: "累计Delta".to_string(),
            value: delta_score,
            contribution: delta_score.abs().to_u64().unwrap_or(0) as u8,
        });
        total_score += delta_score.to_i64().unwrap_or(0);

        // 7. 微价格偏离
        let (best_bid, best_ask) = book.best_bid_ask().unwrap_or((Decimal::ZERO, Decimal::ZERO));
        let mid = (best_bid + best_ask) / dec!(2);
        let micro_deviation = (features.microprice - mid) * dec!(10000);
        components.push(AlphaComponent {
            name: "微价格偏离".to_string(),
            value: micro_deviation,
            contribution: micro_deviation.abs().to_u64().unwrap_or(0) as u8,
        });
        total_score += micro_deviation.to_i64().unwrap_or(0);

        // 8. 买卖压力比
        let pressure_ratio_score = (features.bid_pressure_ratio - dec!(1)) * dec!(100);
        components.push(AlphaComponent {
            name: "压力比".to_string(),
            value: pressure_ratio_score,
            contribution: pressure_ratio_score.abs().to_u64().unwrap_or(0) as u8,
        });
        total_score += pressure_ratio_score.to_i64().unwrap_or(0);

        // 归一化总分到 -100 到 100
        let normalized_score = (total_score as f64 / 10.0).round() as i64;
        let normalized_score = normalized_score.max(-100).min(100);

        // 确定信号
        let signal = if normalized_score > 50 {
            AlphaSignal::StrongBuy
        } else if normalized_score > 20 {
            AlphaSignal::Buy
        } else if normalized_score < -50 {
            AlphaSignal::StrongSell
        } else if normalized_score < -20 {
            AlphaSignal::Sell
        } else {
            AlphaSignal::Neutral
        };

        // 计算置信度
        let confidence = normalized_score.abs() as u8;

        // 计算预期收益
        let expected_return = features.price_pressure * dec!(0.01) * Decimal::from(confidence);

        OrderFlowAlpha {
            signal,
            strength: normalized_score.abs() as u8,
            confidence,
            expected_return,
            time_horizon: "1-5分钟".to_string(),
            components,
        }
    }

    fn update_history(&mut self, book: &OrderBook, features: &OrderBookFeatures) {
        let (best_bid, best_ask) = book.best_bid_ask().unwrap_or((Decimal::ZERO, Decimal::ZERO));
        let mid = (best_bid + best_ask) / dec!(2);

        let snapshot = OrderFlowSnapshot {
            timestamp: chrono::Local::now(),
            ofi: features.ofi,
            obi: features.obi,
            price: mid,
        };

        self.history.push_back(snapshot);
        if self.history.len() > 200 {
            self.history.pop_front();
        }
    }

    fn calculate_ofi_momentum(&self, features: &OrderBookFeatures) -> Decimal {
        if self.history.len() < 10 {
            return features.ofi / dec!(1000);
        }

        let recent_ofi: Decimal = self.history.iter().rev().take(10).map(|h| h.ofi).sum();
        let avg_ofi = recent_ofi / Decimal::from(10);

        (features.ofi - avg_ofi) / dec!(1000)
    }
}

// ==================== 集成所有算法的综合分析器 ====================

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

impl MarketIntelligence {
    pub fn new() -> Self {
        Self {
            whale_detector: WhaleDetector::new(),
            spoofing_detector: SpoofingDetector::new(),
            pump_dump_predictor: PumpDumpPredictor::new(),
            mm_detector: MarketMakerDetector::new(),
            alpha_generator: OrderFlowAlphaGenerator::new(),
        }
    }

    pub fn analyze(&mut self, book: &OrderBook, features: &OrderBookFeatures) -> ComprehensiveAnalysis {
        // 运行所有算法
        let whale_result = self.whale_detector.detect_whales(book, features);
        let spoofing_result = self.spoofing_detector.detect_spoofing(book, features);
        let pumpdump_result = self.pump_dump_predictor.predict(book, features);
        let mm_result = self.mm_detector.detect(book, features);
        let alpha_result = self.alpha_generator.generate(book, features);

        // 计算综合评分
        let overall_sentiment = self.calculate_overall_sentiment(&whale_result, &pumpdump_result, &alpha_result);
        let risk_level = self.calculate_risk_level(&spoofing_result, &mm_result, features);
        let recommendation = self.generate_recommendation(&overall_sentiment, &risk_level, &pumpdump_result);

        ComprehensiveAnalysis {
            timestamp: chrono::Local::now(),
            whale: whale_result,
            spoofing: spoofing_result,
            pump_dump: pumpdump_result,
            market_maker: mm_result,
            alpha: alpha_result,
            overall_sentiment,
            risk_level,
            trading_recommendation: recommendation,
        }
    }

    fn calculate_overall_sentiment(&self,
                                   whale: &WhaleDetectionResult,
                                   pumpdump: &PumpDumpPrediction,
                                   alpha: &OrderFlowAlpha) -> OverallSentiment {
        let mut score = 0;

        // 鲸鱼贡献
        if whale.accumulation_score > whale.distribution_score + dec!(20) {
            score += 2;
        } else if whale.distribution_score > whale.accumulation_score + dec!(20) {
            score -= 2;
        }

        // Pump/Dump贡献
        if pumpdump.pump_probability > pumpdump.dump_probability + 30 {
            score += 3;
        } else if pumpdump.dump_probability > pumpdump.pump_probability + 30 {
            score -= 3;
        }

        // Alpha信号贡献
        match alpha.signal {
            AlphaSignal::StrongBuy => score += 3,
            AlphaSignal::Buy => score += 1,
            AlphaSignal::StrongSell => score -= 3,
            AlphaSignal::Sell => score -= 1,
            _ => {}
        }

        match score {
            5..=8 => OverallSentiment::StrongBullish,
            2..=4 => OverallSentiment::Bullish,
            -1..=1 => OverallSentiment::Neutral,
            -4..=-2 => OverallSentiment::Bearish,
            _ => OverallSentiment::StrongBearish,
        }
    }

    fn calculate_risk_level(&self,
                            spoofing: &SpoofingDetectionResult,
                            mm: &MarketMakerBehavior,
                            features: &OrderBookFeatures) -> RiskLevel {
        let mut risk_score = 0;

        // Spoofing风险
        if spoofing.detected {
            risk_score += 3;
        }

        // 做市商库存风险
        if mm.inventory_bias.abs() > dec!(50) {
            risk_score += 2;
        }

        // 波动率风险
        if features.price_change.abs() > dec!(2) {
            risk_score += 2;
        }

        // 流动性风险
        if features.bid_volume_depth + features.ask_volume_depth < dec!(100000) {
            risk_score += 3;
        }

        match risk_score {
            0 => RiskLevel::VeryLow,
            1..=2 => RiskLevel::Low,
            3..=4 => RiskLevel::Medium,
            5..=6 => RiskLevel::High,
            _ => RiskLevel::VeryHigh,
        }
    }

    fn generate_recommendation(&self,
                               sentiment: &OverallSentiment,
                               risk: &RiskLevel,
                               pumpdump: &PumpDumpPrediction) -> TradingRecommendation {
        match (sentiment, risk) {
            (OverallSentiment::StrongBullish, RiskLevel::VeryLow | RiskLevel::Low) => {
                if pumpdump.pump_probability > 70 {
                    TradingRecommendation::StrongBuy
                } else {
                    TradingRecommendation::Buy
                }
            },
            (OverallSentiment::Bullish, RiskLevel::VeryLow | RiskLevel::Low) => {
                TradingRecommendation::Buy
            },
            (OverallSentiment::StrongBearish, RiskLevel::VeryLow | RiskLevel::Low) => {
                if pumpdump.dump_probability > 70 {
                    TradingRecommendation::StrongSell
                } else {
                    TradingRecommendation::Sell
                }
            },
            (OverallSentiment::Bearish, RiskLevel::VeryLow | RiskLevel::Low) => {
                TradingRecommendation::Sell
            },
            (_, RiskLevel::High | RiskLevel::VeryHigh) => {
                TradingRecommendation::Wait
            },
            _ => TradingRecommendation::Neutral,
        }
    }

    pub fn display_summary(&self, analysis: &ComprehensiveAnalysis) {
        println!("\n{}", "🔬".repeat(30));
        println!("📊 市场智能综合分析 - {}", analysis.timestamp.format("%Y-%m-%d %H:%M:%S"));
        println!("{}", "🔬".repeat(30));

        // 1. 鲸鱼检测结果
        println!("\n🐋 鲸鱼检测:");
        if analysis.whale.detected {
            println!("  发现 {} 只鲸鱼", analysis.whale.whale_count);
            println!("  类型: {:?}", analysis.whale.whale_type);
            println!("  主导率: {:.1}%", analysis.whale.dominance_ratio);
            println!("  吸筹评分: {:.1}, 出货评分: {:.1}",
                     analysis.whale.accumulation_score,
                     analysis.whale.distribution_score);
            println!("  置信度: {}%", analysis.whale.intent_confidence);

            for (i, pos) in analysis.whale.whale_positions.iter().enumerate().take(3) {
                println!("    {}. {:?} {:.6} 数量:{:.0} ({:.1}%){}",
                         i+1, pos.side, pos.price, pos.quantity, pos.percentage,
                         if pos.is_stealth { " [拆单]" } else { "" });
            }
        } else {
            println!("  未检测到明显鲸鱼活动");
        }

        // 2. Spoofing检测
        println!("\n🎭 Spoofing检测:");
        if analysis.spoofing.detected {
            println!("  检测到! 类型: {:?}", analysis.spoofing.spoofing_type);
            println!("  置信度: {}%", analysis.spoofing.confidence);
            println!("  估计价格操纵: {:.6}", analysis.spoofing.estimated_manipulation);
        } else {
            println!("  未检测到明显Spoofing行为");
        }

        // 3. Pump/Dump预测
        println!("\n🚀 Pump/Dump预测:");
        println!("  拉升概率: {}%", analysis.pump_dump.pump_probability);
        println!("  砸盘概率: {}%", analysis.pump_dump.dump_probability);
        println!("  拉升目标: {:.6}", analysis.pump_dump.pump_target);
        println!("  砸盘目标: {:.6}", analysis.pump_dump.dump_target);
        println!("  置信度: {}%", analysis.pump_dump.confidence);

        for signal in &analysis.pump_dump.signals {
            println!("    [{:?}] 强度:{}% - {}",
                     signal.signal_type, signal.strength, signal.description);
        }

        // 4. 做市商行为
        println!("\n🏦 做市商行为:");
        println!("  活跃度: {}", if analysis.market_maker.is_active { "是" } else { "否" });
        println!("  类型: {:?}", analysis.market_maker.mm_type);
        println!("  策略: {:?}", analysis.market_maker.strategy);
        println!("  库存偏向: {:.1}%", analysis.market_maker.inventory_bias);
        println!("  价差策略: {:?}", analysis.market_maker.spread_policy);

        // 5. Alpha信号
        println!("\n⚡ Alpha信号:");
        println!("  信号: {:?} (强度:{}%)",
                 analysis.alpha.signal, analysis.alpha.strength);
        println!("  置信度: {}%", analysis.alpha.confidence);
        println!("  预期收益: {:.4}%", analysis.alpha.expected_return);

        // 6. 综合评分
        println!("\n📈 综合评分:");
        println!("  市场情绪: {:?}", analysis.overall_sentiment);
        println!("  风险等级: {:?}", analysis.risk_level);
        println!("  交易建议: {:?}", analysis.trading_recommendation);

        println!("\n{}", "🔬".repeat(30));
    }
}

