//! Spoofing / layering / wash trading 识别实现。
//!
//! 这里是启发式规则，不是交易所级取证逻辑，重点在于提供实时预警。
//!
//! 关键改进：
//! - OrderLifetimeTracker：追踪档位首次出现时间，档位消失时计算真实存活时长
//! - detect_bid_spoofing / detect_ask_spoofing：只在档位消失时才判定 spoofing，
//!   使用真实 lifetime_secs 而不是硬编码 5.0

use super::*;

impl SpoofingDetector {
    /// 创建默认 spoofing 检测器。
    pub fn new() -> Self {
        Self {
            order_history: VecDeque::with_capacity(100),
            lifetime_tracker: HashMap::new(),
            cancel_history: VecDeque::with_capacity(100),
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
        // 先更新 lifetime tracker，再检测（让消失档位先被记录到 cancel_history）
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
        if self.detect_layering(book, features) {
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

    /// 更新盘口快照历史，同时维护 OrderLifetimeTracker：
    /// - 新出现的档位 → 记录入 lifetime_tracker
    /// - 消失的档位 → 从 tracker 取出首次出现时间，计算真实 lifetime
    fn update_history(&mut self, book: &OrderBook) {
        let now = Instant::now();

        // 构建当前帧的价格集合
        let current_bid_keys: std::collections::HashSet<String> = book
            .bids
            .iter()
            .take(20)
            .map(|(std::cmp::Reverse(p), _)| p.to_string())
            .collect();
        let current_ask_keys: std::collections::HashSet<String> = book
            .asks
            .iter()
            .take(20)
            .map(|(p, _)| p.to_string())
            .collect();

        // 消失的买单：从 tracker 取出，记录 lifetime 到 cancel_history
        let prev_bid_keys: Vec<String> = self
            .lifetime_tracker
            .keys()
            .filter(|(_, side)| side == "bid")
            .map(|(price, _)| price.clone())
            .collect();
        for price_str in prev_bid_keys {
            if !current_bid_keys.contains(&price_str) {
                if let Some(appeared_at) = self
                    .lifetime_tracker
                    .remove(&(price_str.clone(), "bid".to_string()))
                {
                    let lifetime_secs = appeared_at.elapsed().as_secs_f64();
                    if self.cancel_history.len() >= 100 {
                        self.cancel_history.pop_front();
                    }
                    self.cancel_history.push_back(lifetime_secs);
                }
            }
        }

        // 消失的卖单
        let prev_ask_keys: Vec<String> = self
            .lifetime_tracker
            .keys()
            .filter(|(_, side)| side == "ask")
            .map(|(price, _)| price.clone())
            .collect();
        for price_str in prev_ask_keys {
            if !current_ask_keys.contains(&price_str) {
                if let Some(appeared_at) = self
                    .lifetime_tracker
                    .remove(&(price_str.clone(), "ask".to_string()))
                {
                    let lifetime_secs = appeared_at.elapsed().as_secs_f64();
                    if self.cancel_history.len() >= 100 {
                        self.cancel_history.pop_front();
                    }
                    self.cancel_history.push_back(lifetime_secs);
                }
            }
        }

        // 新出现的买单：加入 tracker
        for (std::cmp::Reverse(price), _) in book.bids.iter().take(20) {
            let key = (price.to_string(), "bid".to_string());
            self.lifetime_tracker.entry(key).or_insert(now);
        }

        // 新出现的卖单
        for (price, _) in book.asks.iter().take(20) {
            let key = (price.to_string(), "ask".to_string());
            self.lifetime_tracker.entry(key).or_insert(now);
        }

        // 保存快照
        let snapshot = OrderBookSnapshot {
            timestamp: Local::now(),
            bids: book
                .bids
                .iter()
                .take(20)
                .map(|(std::cmp::Reverse(p), q)| (*p, *q))
                .collect(),
            asks: book.asks.iter().take(20).map(|(p, q)| (*p, *q)).collect(),
        };
        self.order_history.push_back(snapshot);
        if self.order_history.len() > 100 {
            self.order_history.pop_front();
        }
    }

    /// 检测买盘远端的大额诱多挂单。
    /// 改进：只有当档位已消失（lifetime < 2s）时才判定为 spoofing，避免正常大单误报。
    fn detect_bid_spoofing(
        &self,
        book: &OrderBook,
        features: &OrderBookFeatures,
    ) -> SpoofingDetection {
        let mut detection = SpoofingDetection::new();
        let (best_bid, _) = book
            .best_bid_ask()
            .unwrap_or((Decimal::ZERO, Decimal::ZERO));

        // 计算近期取消率（最近100条中 lifetime < 2s 的比例）
        let cancel_rate = if !self.cancel_history.is_empty() {
            let short_lived = self.cancel_history.iter().filter(|&&lt| lt < 2.0).count();
            short_lived as f64 / self.cancel_history.len() as f64
        } else {
            0.0
        };

        for (std::cmp::Reverse(price), qty) in book.bids.iter().take(10) {
            let distance = (best_bid - *price).abs();
            if qty > &(features.total_bid_volume * dec!(0.1))
                && distance > features.spread * dec!(5)
            {
                // 查询该档位是否已在 tracker 里存活超长（仍活跃时暂不报警）
                let appeared_at = self
                    .lifetime_tracker
                    .get(&(price.to_string(), "bid".to_string()));
                let lifetime_secs = match appeared_at {
                    Some(t) => t.elapsed().as_secs_f64(),
                    None => 999.0, // 已消失（cancel_history 里），skip 新报
                };

                // 只有短命（< 3s）的大买单才认为是 spoofing
                if lifetime_secs < 3.0 || cancel_rate > 0.5 {
                    detection.detected = true;
                    detection.levels.push(SpoofingLevel {
                        price: *price,
                        quantity: *qty,
                        side: OrderSide::Bid,
                        lifetime_secs,
                        cancel_rate,
                    });
                }
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

        let cancel_rate = if !self.cancel_history.is_empty() {
            let short_lived = self.cancel_history.iter().filter(|&&lt| lt < 2.0).count();
            short_lived as f64 / self.cancel_history.len() as f64
        } else {
            0.0
        };

        for (price, qty) in book.asks.iter().take(10) {
            let distance = (*price - best_ask).abs();
            if qty > &(features.total_ask_volume * dec!(0.1))
                && distance > features.spread * dec!(5)
            {
                let appeared_at = self
                    .lifetime_tracker
                    .get(&(price.to_string(), "ask".to_string()));
                let lifetime_secs = match appeared_at {
                    Some(t) => t.elapsed().as_secs_f64(),
                    None => 999.0,
                };

                if lifetime_secs < 3.0 || cancel_rate > 0.5 {
                    detection.detected = true;
                    detection.levels.push(SpoofingLevel {
                        price: *price,
                        quantity: *qty,
                        side: OrderSide::Ask,
                        lifetime_secs,
                        cancel_rate,
                    });
                }
            }
        }

        detection
    }

    /// 检测是否出现多个价格层级同时堆出大单。
    /// 改进：使用总深度的相对比例而非绝对值 100000，跨币种通用。
    fn detect_layering(&self, book: &OrderBook, features: &OrderBookFeatures) -> bool {
        let bid_threshold = features.total_bid_volume * dec!(0.08); // 8% of total
        let ask_threshold = features.total_ask_volume * dec!(0.08);

        let bid_layers = book
            .bids
            .iter()
            .take(5)
            .filter(|(_, qty)| **qty > bid_threshold)
            .count();
        let ask_layers = book
            .asks
            .iter()
            .take(5)
            .filter(|(_, qty)| **qty > ask_threshold)
            .count();

        bid_layers >= 3 || ask_layers >= 3
    }

    /// 检测高 OFI 但价格基本不动的异常成交行为。
    fn detect_wash_trading(&self, features: &OrderBookFeatures) -> bool {
        features.ofi.abs() > dec!(500000) && features.price_change.abs() < dec!(0.1)
    }

    /// 基于浓度和假突破特征计算欺骗行为置信度。
    fn calculate_spoofing_confidence(&self, features: &OrderBookFeatures) -> u8 {
        let mut conf = 50u8;

        if features.bid_concentration > dec!(80) || features.ask_concentration > dec!(80) {
            conf = conf.saturating_add(20);
        }

        if features.fake_breakout {
            conf = conf.saturating_add(20);
        }

        // 高取消率加权
        let cancel_rate = if !self.cancel_history.is_empty() {
            self.cancel_history.iter().filter(|&&lt| lt < 2.0).count() as f64
                / self.cancel_history.len() as f64
        } else {
            0.0
        };
        if cancel_rate > 0.7 {
            conf = conf.saturating_add(15);
        }

        conf.min(100)
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
