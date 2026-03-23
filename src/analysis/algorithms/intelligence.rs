use super::*;

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

    // ==================== 新增：多周期分析接口 ====================

    /// 多周期背离检测
    pub fn detect_multi_period_divergence(&self, book: &OrderBook) -> Vec<DivergenceSignal> {
        let mut signals = Vec::new();

        // 检查不同周期的背离
        let periods = [
            (TrendPeriod::Micro, "5s", 0.001),
            (TrendPeriod::Short, "1m", 0.005),
            (TrendPeriod::Medium, "5m", 0.02),
        ];

        for (period, name, threshold) in periods {
            if let Some(signal) = self.check_period_divergence(book, period, name, threshold) {
                signals.push(signal);
            }
        }

        signals
    }

    fn check_period_divergence(&self, book: &OrderBook, period: TrendPeriod,
                               name: &str, threshold: f64) -> Option<DivergenceSignal> {
        let samples = match period {
            TrendPeriod::Micro => &book.history.samples_5s,
            TrendPeriod::Short => &book.history.samples_1m,
            TrendPeriod::Medium => &book.history.samples_5m,
            TrendPeriod::Long => &book.history.samples_1h,
        };

        if samples.len() < 10 { return None; }

        let current = samples.back().unwrap();
        let older = samples.front().unwrap();

        // 价格涨了但OBI跌了 → 看跌背离
        let price_up = current.mid_price > older.mid_price + Decimal::from_f64(threshold).unwrap();
        let obi_down = current.obi < older.obi - dec!(10);

        if price_up && obi_down {
            return Some(DivergenceSignal {
                period: name.to_string(),
                direction: "看跌背离".to_string(),
                strength: ((older.obi - current.obi) / dec!(10)).round().to_u8().unwrap_or(50).min(100),
                description: format!("价格↑ {:.4} 但买盘↓ {:.1}%",
                                     current.mid_price - older.mid_price,
                                     older.obi - current.obi),
            });
        }

        // 价格跌了但OBI涨了 → 看涨背离
        let price_down = current.mid_price < older.mid_price - Decimal::from_f64(threshold).unwrap();
        let obi_up = current.obi > older.obi + dec!(10);

        if price_down && obi_up {
            return Some(DivergenceSignal {
                period: name.to_string(),
                direction: "看涨背离".to_string(),
                strength: ((current.obi - older.obi) / dec!(10)).round().to_u8().unwrap_or(50).min(100),
                description: format!("价格↓ {:.4} 但买盘↑ {:.1}%",
                                     older.mid_price - current.mid_price,
                                     current.obi - older.obi),
            });
        }

        None
    }

    /// 计算多周期加速度曲线
    pub fn calculate_acceleration_curve(&self, book: &OrderBook) -> AccelerationCurve {
        AccelerationCurve {
            micro: book.history.stats_5s.acceleration,
            short: book.history.stats_1m.acceleration,
            medium: book.history.stats_5m.acceleration,
            long: book.history.stats_1h.acceleration,
        }
    }

    /// 识别趋势共振/分歧
    pub fn analyze_trend_coherence(&self, book: &OrderBook) -> TrendCoherence {
        let micro_trend = book.get_trend_strength(TrendPeriod::Micro);
        let short_trend = book.get_trend_strength(TrendPeriod::Short);
        let medium_trend = book.get_trend_strength(TrendPeriod::Medium);

        // 计算趋势一致性
        let trends = [micro_trend, short_trend, medium_trend];
        let avg = trends.iter().sum::<Decimal>() / Decimal::from(3);
        let variance = trends.iter()
            .map(|&t| (t - avg) * (t - avg))
            .sum::<Decimal>() / Decimal::from(3);
        let std = Decimal::from_f64_retain(variance.to_f64().unwrap_or(0.0).sqrt()).unwrap_or(Decimal::ZERO);

        let coherence = if std < dec!(0.5) {
            "高度共振"
        } else if std < dec!(1.0) {
            "基本一致"
        } else if std < dec!(2.0) {
            "出现分歧"
        } else {
            "严重分歧"
        };

        TrendCoherence {
            coherence: coherence.to_string(),
            std_deviation: std,
            micro: micro_trend,
            short: short_trend,
            medium: medium_trend,
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
