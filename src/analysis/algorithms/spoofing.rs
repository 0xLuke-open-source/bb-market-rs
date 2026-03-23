//! Spoofing / layering / wash trading 识别实现。
//!
//! 这里是启发式规则，不是交易所级取证逻辑，重点在于提供实时预警。

use super::*;

impl SpoofingDetector {
    /// 创建默认 spoofing 检测器。
    pub fn new() -> Self {
        Self {
            order_history: VecDeque::with_capacity(100),
        }
    }

    /// 执行一轮 spoofing 检测。
    ///
    /// 这里会并行尝试识别：
    /// - 买盘/卖盘欺骗挂单
    /// - 分层挂单
    /// - 对倒特征
    pub fn detect_spoofing(
        &mut self,
        book: &OrderBook,
        features: &OrderBookFeatures,
    ) -> SpoofingDetectionResult {
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

    /// 缓存最近盘口快照，供之后的行为识别使用。
    fn update_history(&mut self, book: &OrderBook) {
        let snapshot = OrderBookSnapshot {
            timestamp: Local::now(),
            bids: book
                .bids
                .iter()
                .take(20)
                .map(|(Reverse(p), q)| (*p, *q))
                .collect(),
            asks: book.asks.iter().take(20).map(|(p, q)| (*p, *q)).collect(),
        };

        self.order_history.push_back(snapshot);
        if self.order_history.len() > 100 {
            self.order_history.pop_front();
        }
    }

    /// 检测买盘远端的大额诱多挂单。
    fn detect_bid_spoofing(
        &self,
        book: &OrderBook,
        features: &OrderBookFeatures,
    ) -> SpoofingDetection {
        let mut detection = SpoofingDetection::new();
        let (best_bid, _) = book
            .best_bid_ask()
            .unwrap_or((Decimal::ZERO, Decimal::ZERO));

        for (Reverse(price), qty) in book.bids.iter().take(10) {
            // 检查是否是大单且远离盘口
            let distance = (best_bid - *price).abs();
            if qty > &(features.total_bid_volume * dec!(0.1))
                && distance > features.spread * dec!(5)
            {
                detection.detected = true;
                detection.levels.push(SpoofingLevel {
                    price: *price,
                    quantity: *qty,
                    side: OrderSide::Bid,
                    lifetime_secs: 5.0, // 简化值
                    cancel_rate: 0.4,   // 简化值
                });
            }
        }

        detection
    }

    /// 检测卖盘远端的大额诱空挂单。
    fn detect_ask_spoofing(
        &self,
        book: &OrderBook,
        features: &OrderBookFeatures,
    ) -> SpoofingDetection {
        let mut detection = SpoofingDetection::new();
        let (_, best_ask) = book
            .best_bid_ask()
            .unwrap_or((Decimal::ZERO, Decimal::ZERO));

        for (price, qty) in book.asks.iter().take(10) {
            let distance = (*price - best_ask).abs();
            if qty > &(features.total_ask_volume * dec!(0.1))
                && distance > features.spread * dec!(5)
            {
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

    /// 检测是否出现多个价格层级同时堆出大单。
    fn detect_layering(&self, book: &OrderBook) -> bool {
        // 检测分层挂单（多个价格层级都有大单）
        let mut bid_layers = 0;
        let mut ask_layers = 0;

        for (_, qty) in book.bids.iter().take(5) {
            if qty > &dec!(100000) {
                bid_layers += 1;
            }
        }
        for (_, qty) in book.asks.iter().take(5) {
            if qty > &dec!(100000) {
                ask_layers += 1;
            }
        }

        bid_layers >= 3 || ask_layers >= 3
    }

    /// 检测高 OFI 但价格基本不动的异常成交行为。
    fn detect_wash_trading(&self, features: &OrderBookFeatures) -> bool {
        // 检测对倒：高OFI但价格不变
        features.ofi.abs() > dec!(500000) && features.price_change.abs() < dec!(0.1)
    }

    /// 基于浓度和假突破特征计算欺骗行为置信度。
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

    /// 粗略估计这些可疑挂单可能带来的价格操纵幅度。
    fn estimate_price_manipulation(&self, result: &SpoofingDetectionResult) -> Decimal {
        let mut manipulation = Decimal::ZERO;

        for level in &result.spoofing_levels {
            manipulation += level.quantity * level.price / dec!(1000000);
        }

        manipulation
    }
}

/// 内部中间结构，用来统一买盘/卖盘 spoofing 的检测返回值。
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
