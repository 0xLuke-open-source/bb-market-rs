use super::*;

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
