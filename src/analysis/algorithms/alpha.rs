use super::*;

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
