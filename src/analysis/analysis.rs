// analysis.rs
// 市场分析报告模块 - 独立文件

use std::collections::BTreeMap;
use std::cmp::Reverse;
use rust_decimal::Decimal;
use rust_decimal_macros::dec;
use colored::*;
use chrono;
use rust_decimal::prelude::ToPrimitive;
use crate::store::l2_book::{OrderBook, OrderBookFeatures};

// ==================== 市场分析报告数据结构 ====================

#[derive(Debug, Clone)]
pub struct MarketAnalysis {
    pub timestamp: chrono::DateTime<chrono::Local>,
    pub price_level: Decimal,

    // 市场状态
    pub market_regime: MarketRegime,
    pub confidence: u8,  // 0-100

    // 关键指标
    pub key_indicators: Vec<KeyIndicator>,

    // 支撑阻力
    pub support_levels: Vec<(Decimal, Decimal, String)>,  // (价格, 数量, 类型)
    pub resistance_levels: Vec<(Decimal, Decimal, String)>,

    // 主力意图
    pub whale_intent: WhaleIntent,

    // 预测
    pub short_term_forecast: Forecast,
    pub medium_term_forecast: Forecast,

    // 风险提示
    pub risk_warnings: Vec<String>,

    // 建议操作
    pub recommendations: Vec<String>,
}

#[derive(Debug, Clone, PartialEq)]
pub enum MarketRegime {
    Accumulation,      // 吸筹
    Distribution,      // 出货
    MarkUp,           // 拉升
    MarkDown,         // 砸盘
    Sideways,         // 横盘
    Volatile,         // 剧烈波动
}

#[derive(Debug, Clone)]
pub struct KeyIndicator {
    pub name: String,
    pub value: String,
    pub status: IndicatorStatus,
    pub description: String,
}

#[derive(Debug, Clone, PartialEq)]
pub enum IndicatorStatus {
    Bullish,      // 看涨
    Bearish,      // 看跌
    Neutral,      // 中性
    Warning,      // 警告
}

#[derive(Debug, Clone)]
pub enum WhaleIntent {
    Accumulating,     // 吸筹
    Distributing,     // 出货
    Manipulating,     // 操纵
    Waiting,          // 观望
    Unknown,
}

#[derive(Debug, Clone)]
pub struct Forecast {
    pub direction: ForecastDirection,
    pub probability: u8,  // 0-100
    pub target: Decimal,
    pub stop_loss: Decimal,
    pub time_frame: String,
    pub reasoning: String,
}

#[derive(Debug, Clone, PartialEq)]
pub enum ForecastDirection {
    StrongBullish,
    Bullish,
    Neutral,
    Bearish,
    StrongBearish,
}

impl MarketAnalysis {
    pub fn new(book: &OrderBook, features: &OrderBookFeatures) -> Self {
        let (best_bid, best_ask) = book.best_bid_ask().unwrap_or((Decimal::ZERO, Decimal::ZERO));
        let current_price = (best_bid + best_ask) / dec!(2);

        // 识别市场状态
        let market_regime = Self::identify_regime(features);
        let confidence = Self::calculate_confidence(features);

        // 关键指标分析
        let key_indicators = Self::analyze_indicators(features);

        // 支撑阻力
        let (support_levels, resistance_levels) = Self::find_support_resistance(book, features, current_price);

        // 主力意图
        let whale_intent = Self::detect_whale_intent(features);

        // 预测
        let (short_term, medium_term) = Self::generate_forecasts(book, features, current_price);

        // 风险提示
        let risk_warnings = Self::generate_risk_warnings(features);

        // 建议操作
        let recommendations = Self::generate_recommendations(features, &whale_intent, &short_term);

        Self {
            timestamp: chrono::Local::now(),
            price_level: current_price,
            market_regime,
            confidence,
            key_indicators,
            support_levels,
            resistance_levels,
            whale_intent,
            short_term_forecast: short_term,
            medium_term_forecast: medium_term,
            risk_warnings,
            recommendations,
        }
    }

    fn identify_regime(features: &OrderBookFeatures) -> MarketRegime {
        // 基于多个指标识别市场状态
        let bearish_signals = [
            features.dump_signal,
            features.ask_eating,
            features.whale_exit,
            features.obi < dec!(-20),
            features.slope_bid < dec!(-1000000),
            features.bid_volume_change < dec!(-10),
        ].iter().filter(|&&x| x).count();

        let bullish_signals = [
            features.pump_signal,
            features.bid_eating,
            features.whale_entry,
            features.obi > dec!(20),
            features.slope_bid > dec!(1000000),
            features.bid_volume_change > dec!(10),
        ].iter().filter(|&&x| x).count();

        let volatility = (features.price_change.abs() > dec!(1)) ||
            (features.bid_volume_change.abs() > dec!(20)) ||
            (features.ask_volume_change.abs() > dec!(20));

        if bearish_signals >= 3 && features.obi < dec!(-30) {
            if features.max_ask_ratio > dec!(25) {
                MarketRegime::Distribution
            } else {
                MarketRegime::MarkDown
            }
        } else if bullish_signals >= 3 && features.obi > dec!(30) {
            if features.max_bid_ratio > dec!(25) {
                MarketRegime::Accumulation
            } else {
                MarketRegime::MarkUp
            }
        } else if volatility {
            MarketRegime::Volatile
        } else {
            MarketRegime::Sideways
        }
    }

    fn calculate_confidence(features: &OrderBookFeatures) -> u8 {
        // 计算信号一致性得出置信度
        let mut score = 50; // 基准分

        // OBI 贡献 (±20)
        if features.obi > dec!(30) { score += 20; }
        else if features.obi > dec!(10) { score += 10; }
        else if features.obi < dec!(-30) { score -= 20; }
        else if features.obi < dec!(-10) { score -= 10; }

        // OFI 贡献 (±15)
        if features.ofi > dec!(100000) { score += 15; }
        else if features.ofi > dec!(50000) { score += 8; }
        else if features.ofi < dec!(-100000) { score -= 15; }
        else if features.ofi < dec!(-50000) { score -= 8; }

        // 斜率贡献 (±10)
        if features.slope_bid > dec!(5000000) { score += 10; }
        else if features.slope_bid < dec!(-5000000) { score -= 10; }

        // 大单占比贡献 (±5)
        if features.max_bid_ratio > dec!(30) { score += 5; }
        if features.max_ask_ratio > dec!(30) { score -= 5; }

        // 趋势强度贡献 (±10)
        if features.trend_strength > dec!(30) { score += 10; }
        else if features.trend_strength < dec!(-30) { score -= 10; }

        // 限制在 0-100 范围内
        score.max(0).min(100) as u8
    }

    fn analyze_indicators(features: &OrderBookFeatures) -> Vec<KeyIndicator> {
        let mut indicators = Vec::new();

        // OBI 分析
        indicators.push(KeyIndicator {
            name: "订单簿不平衡 (OBI)".to_string(),
            value: format!("{:.2}%", features.obi),
            status: if features.obi > dec!(20) { IndicatorStatus::Bullish }
            else if features.obi < dec!(-20) { IndicatorStatus::Bearish }
            else { IndicatorStatus::Neutral },
            description: if features.obi > dec!(20) { "买方主导市场".to_string() }
            else if features.obi < dec!(-20) { "卖方主导市场".to_string() }
            else { "买卖相对平衡".to_string() },
        });

        // OFI 分析
        indicators.push(KeyIndicator {
            name: "订单流不平衡 (OFI)".to_string(),
            value: format!("{:.0}", features.ofi),
            status: if features.ofi > dec!(50000) { IndicatorStatus::Bullish }
            else if features.ofi < dec!(-50000) { IndicatorStatus::Bearish }
            else { IndicatorStatus::Neutral },
            description: if features.ofi > dec!(50000) { "买单主动吃筹".to_string() }
            else if features.ofi < dec!(-50000) { "卖单主动砸盘".to_string() }
            else { "订单流平稳".to_string() },
        });

        // 斜率分析
        indicators.push(KeyIndicator {
            name: "买单斜率".to_string(),
            value: format!("{:.0}", features.slope_bid),
            status: if features.slope_bid > dec!(1000000) { IndicatorStatus::Bullish }
            else if features.slope_bid < dec!(-1000000) { IndicatorStatus::Bearish }
            else { IndicatorStatus::Neutral },
            description: if features.slope_bid > dec!(1000000) { "买盘加速堆积".to_string() }
            else if features.slope_bid < dec!(-1000000) { "买盘快速撤离".to_string() }
            else { "买盘平稳".to_string() },
        });

        // 卖单斜率分析
        indicators.push(KeyIndicator {
            name: "卖单斜率".to_string(),
            value: format!("{:.0}", features.slope_ask),
            status: if features.slope_ask < dec!(-1000000) { IndicatorStatus::Bearish }
            else if features.slope_ask > dec!(1000000) { IndicatorStatus::Bullish }
            else { IndicatorStatus::Neutral },
            description: if features.slope_ask < dec!(-1000000) { "卖盘加速堆积".to_string() }
            else if features.slope_ask > dec!(1000000) { "卖盘快速撤离".to_string() }
            else { "卖盘平稳".to_string() },
        });

        // 大单占比
        indicators.push(KeyIndicator {
            name: "最大买单占比".to_string(),
            value: format!("{:.1}%", features.max_bid_ratio),
            status: if features.max_bid_ratio > dec!(30) { IndicatorStatus::Warning }
            else { IndicatorStatus::Neutral },
            description: if features.max_bid_ratio > dec!(30) { "存在托单或吸筹".to_string() }
            else { "大单分散".to_string() },
        });

        indicators.push(KeyIndicator {
            name: "最大卖单占比".to_string(),
            value: format!("{:.1}%", features.max_ask_ratio),
            status: if features.max_ask_ratio > dec!(30) { IndicatorStatus::Warning }
            else { IndicatorStatus::Neutral },
            description: if features.max_ask_ratio > dec!(30) { "存在压盘或出货".to_string() }
            else { "卖单分散".to_string() },
        });

        // 趋势强度
        indicators.push(KeyIndicator {
            name: "趋势强度".to_string(),
            value: format!("{:.1}", features.trend_strength),
            status: if features.trend_strength > dec!(30) { IndicatorStatus::Bullish }
            else if features.trend_strength < dec!(-30) { IndicatorStatus::Bearish }
            else { IndicatorStatus::Neutral },
            description: if features.trend_strength > dec!(30) { "上升趋势明确".to_string() }
            else if features.trend_strength < dec!(-30) { "下降趋势明确".to_string() }
            else { "无明显趋势".to_string() },
        });

        // 价差分析
        indicators.push(KeyIndicator {
            name: "价差 (bps)".to_string(),
            value: format!("{:.2}", features.spread_bps),
            status: if features.spread_bps > dec!(50) { IndicatorStatus::Warning }
            else if features.spread_bps > dec!(20) { IndicatorStatus::Neutral }
            else { IndicatorStatus::Bullish },
            description: if features.spread_bps > dec!(50) { "价差过大，流动性不足".to_string() }
            else if features.spread_bps > dec!(20) { "价差正常".to_string() }
            else { "价差很小，流动性好".to_string() },
        });

        indicators
    }

    fn find_support_resistance(book: &OrderBook, features: &OrderBookFeatures, current_price: Decimal) ->
    (Vec<(Decimal, Decimal, String)>, Vec<(Decimal, Decimal, String)>) {

        let mut supports = Vec::new();
        let mut resistances = Vec::new();

        // 找到明显的支撑位（买单密集区）
        let bid_levels: Vec<_> = book.bids.iter()
            .take(20)
            .map(|(Reverse(p), q)| (*p, *q))
            .collect();

        for i in 0..bid_levels.len() {
            let (price, qty) = bid_levels[i];
            if qty > features.total_bid_volume * dec!(0.03) {  // 超过总量3%
                let strength = if qty > features.total_bid_volume * dec!(0.1) { "🔵 强支撑" }
                else { "🔹 弱支撑" };
                supports.push((price, qty, strength.to_string()));
            }
        }

        // 找到明显的阻力位（卖单密集区）
        let ask_levels: Vec<_> = book.asks.iter()
            .take(20)
            .map(|(p, q)| (*p, *q))
            .collect();

        for i in 0..ask_levels.len() {
            let (price, qty) = ask_levels[i];
            if qty > features.total_ask_volume * dec!(0.03) {  // 超过总量3%
                let strength = if qty > features.total_ask_volume * dec!(0.1) { "🔴 强阻力" }
                else { "🔸 弱阻力" };
                resistances.push((price, qty, strength.to_string()));
            }
        }

        // 按价格排序
        supports.sort_by(|a, b| b.0.cmp(&a.0)); // 从高到低
        resistances.sort_by(|a, b| a.0.cmp(&b.0)); // 从低到高

        (supports, resistances)
    }

    fn detect_whale_intent(features: &OrderBookFeatures) -> WhaleIntent {
        if features.whale_entry && features.bid_eating {
            WhaleIntent::Accumulating
        } else if features.whale_exit && features.ask_eating {
            WhaleIntent::Distributing
        } else if features.fake_breakout || features.pump_signal || features.dump_signal {
            WhaleIntent::Manipulating
        } else if features.whale_bid || features.whale_ask {
            WhaleIntent::Waiting
        } else {
            WhaleIntent::Unknown
        }
    }

    fn generate_forecasts(book: &OrderBook, features: &OrderBookFeatures, current_price: Decimal) -> (Forecast, Forecast) {
        // 短期预测 (15-30分钟)
        let short_term = Self::generate_short_term_forecast(book, features, current_price);

        // 中期预测 (1-2小时)
        let medium_term = Self::generate_medium_term_forecast(features, current_price, &short_term);

        (short_term, medium_term)
    }

    fn generate_short_term_forecast(book: &OrderBook, features: &OrderBookFeatures, current_price: Decimal) -> Forecast {
        // 获取关键价位
        let (best_bid, best_ask) = book.best_bid_ask().unwrap_or((current_price * dec!(0.999), current_price * dec!(1.001)));

        // 计算上方阻力强度
        let ask_pressure: Decimal = book.asks.iter()
            .take(5)
            .map(|(p, q)| p * q)
            .sum();

        // 计算下方支撑强度
        let bid_pressure: Decimal = book.bids.iter()
            .take(5)
            .map(|(Reverse(p), q)| p * q)
            .sum();

        // 判断方向
        let direction = if features.pump_signal || features.bid_eating || features.ofi > dec!(100000) {
            if features.slope_bid > dec!(5000000) {
                ForecastDirection::StrongBullish
            } else {
                ForecastDirection::Bullish
            }
        } else if features.dump_signal || features.ask_eating || features.ofi < dec!(-100000) {
            if features.slope_ask < dec!(-5000000) {
                ForecastDirection::StrongBearish
            } else {
                ForecastDirection::Bearish
            }
        } else if ask_pressure > bid_pressure * dec!(2.0) {
            ForecastDirection::Bearish
        } else if bid_pressure > ask_pressure * dec!(2.0) {
            ForecastDirection::Bullish
        } else {
            ForecastDirection::Neutral
        };

        // 计算概率
        let probability = match direction {
            ForecastDirection::StrongBullish => 85,
            ForecastDirection::Bullish => 70,
            ForecastDirection::Neutral => 50,
            ForecastDirection::Bearish => 70,
            ForecastDirection::StrongBearish => 85,
        };

        // 计算目标位和止损位
        let (target, stop_loss) = match direction {
            ForecastDirection::Bullish | ForecastDirection::StrongBullish => {
                let target = book.asks.iter()
                    .skip(2)
                    .next()
                    .map(|(p, _)| *p)
                    .unwrap_or(best_ask + dec!(0.002));
                let stop = best_bid * dec!(0.995);
                (target, stop)
            },
            ForecastDirection::Bearish | ForecastDirection::StrongBearish => {
                let target = book.bids.iter()
                    .skip(2)
                    .next()
                    .map(|(Reverse(p), _)| *p)
                    .unwrap_or(best_bid - dec!(0.002));
                let stop = best_ask * dec!(1.005);
                (target, stop)
            },
            _ => (current_price, current_price),
        };

        Forecast {
            direction: direction.clone(),
            probability,
            target,
            stop_loss,
            time_frame: "15-30分钟".to_string(),
            reasoning: Self::generate_reasoning(features, direction),
        }
    }

    fn generate_medium_term_forecast(features: &OrderBookFeatures, current_price: Decimal, short_term: &Forecast) -> Forecast {
        // 中期预测基于趋势强度和OBI
        let direction = if features.trend_strength > dec!(20) {
            ForecastDirection::Bullish
        } else if features.trend_strength < dec!(-20) {
            ForecastDirection::Bearish
        } else {
            short_term.direction.clone()
        };

        let probability = (features.trend_strength.abs().to_u64().unwrap_or(50) as u8).min(80).max(40);

        let (target, stop_loss) = match direction {
            ForecastDirection::Bullish | ForecastDirection::StrongBullish => {
                (current_price * dec!(1.02), current_price * dec!(0.985))
            },
            ForecastDirection::Bearish | ForecastDirection::StrongBearish => {
                (current_price * dec!(0.98), current_price * dec!(1.015))
            },
            _ => (current_price, current_price),
        };

        Forecast {
            direction: direction.clone(),
            probability,
            target,
            stop_loss,
            time_frame: "1-2小时".to_string(),
            reasoning: format!("趋势强度{:.1}，OBI{:.1}%，{}",
                               features.trend_strength,
                               features.obi,
                               if features.trend_strength > dec!(20) { "上升趋势延续" }
                               else if features.trend_strength < dec!(-20) { "下降趋势延续" }
                               else { "趋势不明朗" }),
        }
    }

    fn generate_reasoning(features: &OrderBookFeatures, direction: ForecastDirection) -> String {
        let mut reasons = Vec::new();

        match direction {
            ForecastDirection::Bullish | ForecastDirection::StrongBullish => {
                if features.ofi > dec!(100000) {
                    reasons.push(format!("OFI {:.0} 显示买单强劲", features.ofi));
                }
                if features.max_bid_ratio > dec!(30) {
                    reasons.push(format!("最大买单占比{:.1}% 存在托单", features.max_bid_ratio));
                }
                if features.bid_volume_change > dec!(10) {
                    reasons.push(format!("买单量+{:.1}% 正在增加", features.bid_volume_change));
                }
                if features.slope_bid > dec!(5000000) {
                    reasons.push("买单斜率陡峭 买盘加速".to_string());
                }
            },
            ForecastDirection::Bearish | ForecastDirection::StrongBearish => {
                if features.ofi < dec!(-100000) {
                    reasons.push(format!("OFI {:.0} 显示卖单强劲", features.ofi));
                }
                if features.max_ask_ratio > dec!(30) {
                    reasons.push(format!("最大卖单占比{:.1}% 存在压盘", features.max_ask_ratio));
                }
                if features.ask_volume_change > dec!(10) {
                    reasons.push(format!("卖单量+{:.1}% 正在增加", features.ask_volume_change));
                }
                if features.slope_ask < dec!(-5000000) {
                    reasons.push("卖单斜率陡峭 卖盘加速".to_string());
                }
            },
            _ => {
                reasons.push("多空力量相对均衡".to_string());
                if features.bid_ask_ratio > dec!(1.5) {
                    reasons.push("买单略占优".to_string());
                } else if features.bid_ask_ratio < dec!(0.67) {
                    reasons.push("卖单略占优".to_string());
                }
            }
        }

        reasons.join("，")
    }

    fn generate_risk_warnings(features: &OrderBookFeatures) -> Vec<String> {
        let mut warnings = Vec::new();

        if features.liquidity_warning {
            warnings.push("⚠️ 流动性危机预警 - 买卖盘稀薄".to_string());
        }
        if features.fake_breakout {
            warnings.push("🎭 假突破风险 - 价格变动但成交量不足".to_string());
        }
        if features.whale_entry {
            warnings.push("🐋 鲸鱼正在进场 - 大资金建仓迹象".to_string());
        }
        if features.whale_exit {
            warnings.push("🐋 鲸鱼正在离场 - 大资金撤退迹象".to_string());
        }
        if features.spread_bps > dec!(50) {
            warnings.push("💰 价差过大 - 交易成本较高".to_string());
        }
        if features.max_bid_ratio > dec!(40) {
            warnings.push("📈 买单高度集中 - 警惕主力撤单".to_string());
        }
        if features.max_ask_ratio > dec!(40) {
            warnings.push("📉 卖单高度集中 - 警惕主力砸盘".to_string());
        }
        if features.bid_volume_change.abs() > dec!(50) {
            warnings.push("🔄 买单量剧烈波动 - 主力异动".to_string());
        }
        if features.ask_volume_change.abs() > dec!(50) {
            warnings.push("🔄 卖单量剧烈波动 - 主力异动".to_string());
        }

        warnings
    }

    fn generate_recommendations(features: &OrderBookFeatures, whale_intent: &WhaleIntent, forecast: &Forecast) -> Vec<String> {
        let mut recs = Vec::new();

        match forecast.direction {
            ForecastDirection::StrongBullish => {
                recs.push("✅ 强烈看涨，可考虑建仓做多".to_string());
                recs.push(format!("🎯 目标位: {:.6}", forecast.target));
                recs.push(format!("🛑 止损位: {:.6}", forecast.stop_loss));
                recs.push("📊 仓位建议: 30-40%".to_string());
            },
            ForecastDirection::Bullish => {
                recs.push("📈 谨慎看涨，可轻仓试多".to_string());
                recs.push(format!("🎯 目标位: {:.6}", forecast.target));
                recs.push(format!("🛑 止损位: {:.6}", forecast.stop_loss));
                recs.push("📊 仓位建议: 15-20%".to_string());
            },
            ForecastDirection::Neutral => {
                recs.push("⏸️ 建议观望，等待明确信号".to_string());
                // 修复这里的错误
                if features.support_strength > Decimal::ZERO && features.resistance_strength > Decimal::ZERO {
                    recs.push(format!("👀 关注支撑 {:.6} 和阻力 {:.6}",
                                      features.support_strength,
                                      features.resistance_strength));
                }
            },
            ForecastDirection::Bearish => {
                recs.push("📉 谨慎看跌，可轻仓试空".to_string());
                recs.push(format!("🎯 目标位: {:.6}", forecast.target));
                recs.push(format!("🛑 止损位: {:.6}", forecast.stop_loss));
                recs.push("📊 仓位建议: 15-20%".to_string());
            },
            ForecastDirection::StrongBearish => {
                recs.push("❌ 强烈看跌，可考虑做空".to_string());
                recs.push(format!("🎯 目标位: {:.6}", forecast.target));
                recs.push(format!("🛑 止损位: {:.6}", forecast.stop_loss));
                recs.push("📊 仓位建议: 30-40%".to_string());
            },
        }

        match whale_intent {
            WhaleIntent::Accumulating => {
                recs.push("🐋 鲸鱼正在吸筹，可跟随主力方向".to_string());
            },
            WhaleIntent::Distributing => {
                recs.push("🐋 鲸鱼正在出货，注意风险".to_string());
            },
            WhaleIntent::Manipulating => {
                recs.push("🎭 主力正在操纵，警惕假突破".to_string());
            },
            WhaleIntent::Waiting => {
                recs.push("👀 鲸鱼观望中，等待明确信号".to_string());
            },
            _ => {}
        }

        // 添加通用建议
        if features.bid_concentration > dec!(50) {
            recs.push("⚠️ 买单集中度高，建议设置移动止损".to_string());
        }
        if features.ask_concentration > dec!(50) {
            recs.push("⚠️ 卖单集中度高，建议分批止盈".to_string());
        }

        recs
    }

    pub fn display(&self) {
        println!("\n{}", "=".repeat(120));
        println!("📊 市场分析报告 - {}", self.timestamp.format("%Y-%m-%d %H:%M:%S"));
        println!("{}", "=".repeat(120));

        // 市场概览
        println!("\n📌 市场概览");
        println!("{}", "-".repeat(60));
        println!("当前价格: {:.6}", self.price_level);

        let regime_str = match self.market_regime {
            MarketRegime::Accumulation => "📈 吸筹阶段".bright_green(),
            MarketRegime::Distribution => "📉 出货阶段".bright_red(),
            MarketRegime::MarkUp => "🚀 拉升阶段".bright_green(),
            MarketRegime::MarkDown => "💥 砸盘阶段".bright_red(),
            MarketRegime::Sideways => "➡️ 横盘整理".bright_yellow(),
            MarketRegime::Volatile => "⚡ 剧烈波动".bright_purple(),
        };
        println!("市场状态: {} (置信度 {}%)", regime_str, self.confidence);

        let intent_str = match self.whale_intent {
            WhaleIntent::Accumulating => "🐋 正在吸筹".bright_green(),
            WhaleIntent::Distributing => "🐋 正在出货".bright_red(),
            WhaleIntent::Manipulating => "🎭 正在操纵".bright_yellow(),
            WhaleIntent::Waiting => "👀 观望等待".bright_blue(),
            WhaleIntent::Unknown => "❓ 意图不明".normal(),
        };
        println!("主力意图: {}", intent_str);

        // 关键指标
        println!("\n📊 关键指标分析");
        println!("{}", "-".repeat(80));
        for indicator in &self.key_indicators {
            let status_symbol = match indicator.status {
                IndicatorStatus::Bullish => "🟢",
                IndicatorStatus::Bearish => "🔴",
                IndicatorStatus::Neutral => "⚪",
                IndicatorStatus::Warning => "🟡",
            };
            println!("{} {}: {} - {}",
                     status_symbol,
                     indicator.name.bold(),
                     indicator.value.bright_cyan(),
                     indicator.description
            );
        }

        // 支撑阻力
        println!("\n🛡️ 支撑阻力位");
        println!("{}", "-".repeat(60));
        println!("【支撑位】");
        for (i, (price, qty, strength)) in self.support_levels.iter().take(3).enumerate() {
            println!("  {}. {:.6} ({} {:.0}) {}",
                     i+1, price,
                     if strength.contains("强") { "🔵" } else { "🔹" },
                     qty.round_dp(0), strength
            );
        }
        if self.support_levels.is_empty() {
            println!("  暂无明显支撑位");
        }

        println!("\n【阻力位】");
        for (i, (price, qty, strength)) in self.resistance_levels.iter().take(3).enumerate() {
            println!("  {}. {:.6} ({} {:.0}) {}",
                     i+1, price,
                     if strength.contains("强") { "🔴" } else { "🔸" },
                     qty.round_dp(0), strength
            );
        }
        if self.resistance_levels.is_empty() {
            println!("  暂无明显阻力位");
        }

        // 预测
        println!("\n🎯 走势预测");
        println!("{}", "-".repeat(80));

        // 短期预测
        let (short_dir, short_color) = match self.short_term_forecast.direction {
            ForecastDirection::StrongBullish => ("🚀 强烈看涨", "green"),
            ForecastDirection::Bullish => ("📈 看涨", "green"),
            ForecastDirection::Neutral => ("➡️ 横盘", "yellow"),
            ForecastDirection::Bearish => ("📉 看跌", "red"),
            ForecastDirection::StrongBearish => ("💥 强烈看跌", "red"),
        };
        println!("【短期 {}】", self.short_term_forecast.time_frame);
        println!("  方向: {}", short_dir);
        println!("  概率: {}%", self.short_term_forecast.probability);
        println!("  目标: {:.6}", self.short_term_forecast.target);
        println!("  止损: {:.6}", self.short_term_forecast.stop_loss);
        println!("  理由: {}", self.short_term_forecast.reasoning);

        // 中期预测
        let (medium_dir, _) = match self.medium_term_forecast.direction {
            ForecastDirection::StrongBullish => ("🚀 强烈看涨", "green"),
            ForecastDirection::Bullish => ("📈 看涨", "green"),
            ForecastDirection::Neutral => ("➡️ 横盘", "yellow"),
            ForecastDirection::Bearish => ("📉 看跌", "red"),
            ForecastDirection::StrongBearish => ("💥 强烈看跌", "red"),
        };
        println!("\n【中期 {}】", self.medium_term_forecast.time_frame);
        println!("  方向: {}", medium_dir);
        println!("  概率: {}%", self.medium_term_forecast.probability);
        println!("  目标: {:.6}", self.medium_term_forecast.target);
        println!("  止损: {:.6}", self.medium_term_forecast.stop_loss);
        println!("  理由: {}", self.medium_term_forecast.reasoning);

        // 风险提示
        if !self.risk_warnings.is_empty() {
            println!("\n⚠️ 风险提示");
            println!("{}", "-".repeat(60));
            for warning in &self.risk_warnings {
                println!("  {}", warning.bright_yellow());
            }
        }

        // 操作建议
        println!("\n💡 操作建议");
        println!("{}", "-".repeat(60));
        for rec in &self.recommendations {
            println!("  {}", rec);
        }

        println!("\n{}", "=".repeat(120));
    }

    // 导出报告到文件
    pub fn save_to_file(&self, file_path: &str) -> std::io::Result<()> {
        use std::fs::OpenOptions;
        use std::io::Write;

        let mut file = OpenOptions::new()
            .create(true)
            .append(true)
            .open(file_path)?;

        writeln!(file, "\n{}", "=".repeat(100))?;
        writeln!(file, "市场分析报告 - {}", self.timestamp.format("%Y-%m-%d %H:%M:%S"))?;
        writeln!(file, "{}", "=".repeat(100))?;
        writeln!(file, "当前价格: {:.6}", self.price_level)?;
        writeln!(file, "市场状态: {:?}, 置信度: {}%", self.market_regime, self.confidence)?;
        writeln!(file, "主力意图: {:?}", self.whale_intent)?;
        writeln!(file, "短期预测: {:?}, 概率: {}%, 目标: {:.6}",
                 self.short_term_forecast.direction,
                 self.short_term_forecast.probability,
                 self.short_term_forecast.target)?;
        writeln!(file, "中期预测: {:?}, 概率: {}%, 目标: {:.6}",
                 self.medium_term_forecast.direction,
                 self.medium_term_forecast.probability,
                 self.medium_term_forecast.target)?;

        Ok(())
    }
}