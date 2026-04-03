//! 市场分析报告模块。
//!
//! 这个文件负责把底层订单簿特征整理成一份面向阅读和展示的分析报告：
//! - 识别市场阶段
//! - 汇总关键指标
//! - 计算一组高级衍生指标
//! - 给出支撑阻力、主力意图、风险提示和操作建议

use crate::market_data::domain::order_book::{OrderBook, OrderBookFeatures};
use chrono;
use rust_decimal::prelude::ToPrimitive;
use rust_decimal::Decimal;
use rust_decimal_macros::dec;
use std::cmp::Reverse;
use std::fs::OpenOptions;
use std::io::Write;

// ==================== 报告使用的基础枚举和类型 ====================

/// 当前市场所处阶段。
#[derive(Debug, Clone, PartialEq)]
pub enum MarketRegime {
    Accumulation, // 吸筹
    Distribution, // 出货
    MarkUp,       // 拉升
    MarkDown,     // 砸盘
    Sideways,     // 横盘
    Volatile,     // 剧烈波动
}

/// 单项指标的偏向状态。
#[derive(Debug, Clone, PartialEq)]
pub enum IndicatorStatus {
    Bullish, // 看涨
    Bearish, // 看跌
    Neutral, // 中性
    Warning, // 警告
}

/// 对主力/鲸鱼意图的解释。
#[derive(Debug, Clone)]
pub enum WhaleIntent {
    Accumulating, // 吸筹
    Distributing, // 出货
    Manipulating, // 操纵
    Waiting,      // 观望
    Unknown,
}

/// 预测方向。
#[derive(Debug, Clone, PartialEq)]
pub enum ForecastDirection {
    StrongBullish,
    Bullish,
    Neutral,
    Bearish,
    StrongBearish,
}

/// 用于展示的单项关键指标。
#[derive(Debug, Clone)]
pub struct KeyIndicator {
    pub name: String,
    pub value: String,
    pub status: IndicatorStatus,
    pub description: String,
}

/// 某个时间尺度下的预测结果。
#[derive(Debug, Clone)]
pub struct Forecast {
    pub direction: ForecastDirection,
    pub probability: u8, // 0-100
    pub target: Decimal,
    pub stop_loss: Decimal,
    pub time_frame: String,
    pub reasoning: String,
}

// ==================== 高级指标分组 ====================

/// 分组后的高级市场微观结构指标。
///
/// 这里字段很多，但它们都只服务一个目标：把 `OrderBookFeatures`
/// 扩展成更易读、可解释、适合展示的一组指标面板。
#[derive(Debug, Clone)]
pub struct AdvancedMetrics {
    // 流动性深度指标 (8个)
    pub liquidity_score: Decimal,        // 流动性综合评分
    pub market_depth_ratio: Decimal,     // 深度比率 (买/卖)
    pub bid_ask_concentration: Decimal,  // 买卖盘集中度
    pub depth_elasticity: Decimal,       // 深度弹性
    pub quote_slippage_buy_1k: Decimal,  // 1kU买入滑点
    pub quote_slippage_sell_1k: Decimal, // 1kU卖出滑点
    pub quote_slippage_10k: Decimal,     // 10kU滑点
    pub weighted_spread: Decimal,        // 加权价差

    // 订单流质量指标 (7个)
    pub order_flow_quality: Decimal,            // 订单流质量
    pub aggressive_order_ratio: Decimal,        // 主动单比例
    pub passive_order_ratio: Decimal,           // 被动单比例
    pub order_cancellation_rate: Decimal,       // 撤单率
    pub order_submission_rate: Decimal,         // 下单率
    pub trade_flow_efficiency: Decimal,         // 成交流效率
    pub order_book_imbalance_velocity: Decimal, // 订单簿失衡速度

    // 价格发现指标 (6个)
    pub price_discovery_quality: Decimal, // 价格发现质量
    pub microprice_efficiency: Decimal,   // 微价格效率
    pub weighted_mid_price: Decimal,      // 加权中间价
    pub price_impact_buy: Decimal,        // 买入价格影响
    pub price_impact_sell: Decimal,       // 卖出价格影响
    pub market_impact_cost: Decimal,      // 市场冲击成本

    // 波动率结构指标 (7个)
    pub volatility_term_structure: Decimal, // 波动率期限结构
    pub realized_volatility_1m: Decimal,    // 1分钟已实现波动率
    pub realized_volatility_5m: Decimal,    // 5分钟已实现波动率
    pub implied_volatility: Decimal,        // 隐含波动率
    pub volatility_skew: Decimal,           // 波动率偏斜
    pub volatility_cones: Decimal,          // 波动率锥
    pub vix_equivalent: Decimal,            // 恐慌指数等效

    // 市场微观结构指标 (8个)
    pub market_efficiency_ratio: Decimal,       // 市场效率比率
    pub information_asymmetry: Decimal,         // 信息不对称度
    pub adverse_selection_risk: Decimal,        // 逆向选择风险
    pub price_clustering: Decimal,              // 价格聚集度
    pub tick_utilization: Decimal,              // 最小变动单位利用率
    pub order_book_resilience: Decimal,         // 订单簿恢复力
    pub liquidity_provider_confidence: Decimal, // 流动性提供者信心
    pub market_maker_profitability: Decimal,    // 做市商盈利性

    // 资金流向指标 (6个)
    pub net_flow_volume: Decimal,          // 净流量
    pub buy_flow_pressure: Decimal,        // 买入流压力
    pub sell_flow_pressure: Decimal,       // 卖出流压力
    pub flow_concentration: Decimal,       // 流集中度
    pub institutional_flow_ratio: Decimal, // 机构流比例
    pub retail_flow_ratio: Decimal,        // 散户流比例

    // 风险指标 (8个)
    pub var_95_1m: Decimal,              // 95% VaR 1分钟
    pub cvar_95_1m: Decimal,             // 95% CVaR 1分钟
    pub expected_shortfall: Decimal,     // 预期亏损
    pub tail_risk_index: Decimal,        // 尾部风险指数
    pub stress_test_scenario: Decimal,   // 压力测试情景
    pub correlation_risk: Decimal,       // 相关性风险
    pub liquidity_adjusted_var: Decimal, // 流动性调整VaR
    pub systemic_risk_index: Decimal,    // 系统性风险指数

    // 市场情绪指标 (5个)
    pub fear_greed_index: Decimal,    // 恐惧贪婪指数
    pub market_sentiment: Decimal,    // 市场情绪
    pub bull_bear_ratio: Decimal,     // 多空比
    pub investor_confidence: Decimal, // 投资者信心
    pub market_momentum: Decimal,     // 市场动量

    // 做市商专用指标 (7个)
    pub inventory_risk: Decimal,           // 库存风险
    pub spread_profitability: Decimal,     // 价差盈利性
    pub adverse_selection_cost: Decimal,   // 逆向选择成本
    pub inventory_decay_rate: Decimal,     // 库存衰减率
    pub optimal_spread: Decimal,           // 最优价差
    pub market_making_signal: Decimal,     // 做市信号
    pub liquidity_provision_cost: Decimal, // 流动性提供成本
}

/// 最终输出给上层展示或日志的市场分析报告。
#[derive(Debug, Clone)]
pub struct MarketAnalysis {
    pub timestamp: chrono::DateTime<chrono::Local>,
    pub price_level: Decimal,

    // 市场状态
    pub market_regime: MarketRegime,
    pub confidence: u8,

    // 关键指标
    pub key_indicators: Vec<KeyIndicator>,

    // 新增：高级指标分组
    pub advanced_metrics: AdvancedMetrics,

    // 支撑阻力
    pub support_levels: Vec<(Decimal, Decimal, String)>,
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

impl MarketAnalysis {
    /// 基于当前订单簿和特征，构建一份完整的市场分析报告。
    ///
    /// 这个构造过程本质上是“编排器”：
    /// 先识别市场状态，再补充指标、支撑阻力、预测和建议。
    pub fn new(book: &OrderBook, features: &OrderBookFeatures) -> Self {
        let (best_bid, best_ask) = book
            .best_bid_ask()
            .unwrap_or((Decimal::ZERO, Decimal::ZERO));
        let current_price = (best_bid + best_ask) / dec!(2);

        // 识别市场状态
        let market_regime = Self::identify_regime(features);
        let confidence = Self::calculate_confidence(features);

        // 关键指标分析
        let key_indicators = Self::analyze_indicators(features);

        // 计算高级指标
        let advanced_metrics = Self::calculate_advanced_metrics(book, features, current_price);

        // 支撑阻力
        let (support_levels, resistance_levels) =
            Self::find_support_resistance_advanced(book, features, current_price);

        // 主力意图
        let whale_intent = Self::detect_whale_intent_advanced(features, &advanced_metrics);

        // 预测
        let (short_term, medium_term) =
            Self::generate_forecasts_advanced(book, features, &advanced_metrics, current_price);

        // 风险提示
        let risk_warnings = Self::generate_risk_warnings_advanced(features, &advanced_metrics);

        // 建议操作
        let recommendations = Self::generate_recommendations_advanced(
            features,
            &whale_intent,
            &short_term,
            &advanced_metrics,
        );

        Self {
            timestamp: chrono::Local::now(),
            price_level: current_price,
            market_regime,
            confidence,
            key_indicators,
            advanced_metrics,
            support_levels,
            resistance_levels,
            whale_intent,
            short_term_forecast: short_term,
            medium_term_forecast: medium_term,
            risk_warnings,
            recommendations,
        }
    }

    /// 基于一组核心特征判断市场更像吸筹、拉升、出货还是砸盘。
    fn identify_regime(features: &OrderBookFeatures) -> MarketRegime {
        // 基于多个指标识别市场状态
        let bearish_signals = [
            features.dump_signal,
            features.ask_eating,
            features.whale_exit,
            features.obi < dec!(-20),
            features.slope_bid < dec!(-1000000),
            features.bid_volume_change < dec!(-10),
        ]
        .iter()
        .filter(|&&x| x)
        .count();

        let bullish_signals = [
            features.pump_signal,
            features.bid_eating,
            features.whale_entry,
            features.obi > dec!(20),
            features.slope_bid > dec!(1000000),
            features.bid_volume_change > dec!(10),
        ]
        .iter()
        .filter(|&&x| x)
        .count();

        let volatility = (features.price_change.abs() > dec!(1))
            || (features.bid_volume_change.abs() > dec!(20))
            || (features.ask_volume_change.abs() > dec!(20));

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

    /// 计算整体判断的置信度。
    ///
    /// 实现上是一个经验打分器，用多个特征的一致性来提升或削弱分值。
    fn calculate_confidence(features: &OrderBookFeatures) -> u8 {
        // 计算信号一致性得出置信度
        let mut score = 50; // 基准分

        // OBI 贡献 (±20)
        if features.obi > dec!(30) {
            score += 20;
        } else if features.obi > dec!(10) {
            score += 10;
        } else if features.obi < dec!(-30) {
            score -= 20;
        } else if features.obi < dec!(-10) {
            score -= 10;
        }

        // OFI 贡献 (±15)
        if features.ofi > dec!(100000) {
            score += 15;
        } else if features.ofi > dec!(50000) {
            score += 8;
        } else if features.ofi < dec!(-100000) {
            score -= 15;
        } else if features.ofi < dec!(-50000) {
            score -= 8;
        }

        // 斜率贡献 (±10)
        if features.slope_bid > dec!(5000000) {
            score += 10;
        } else if features.slope_bid < dec!(-5000000) {
            score -= 10;
        }

        // 大单占比贡献 (±5)
        if features.max_bid_ratio > dec!(30) {
            score += 5;
        }
        if features.max_ask_ratio > dec!(30) {
            score -= 5;
        }

        // 趋势强度贡献 (±10)
        if features.trend_strength > dec!(30) {
            score += 10;
        } else if features.trend_strength < dec!(-30) {
            score -= 10;
        }

        // 限制在 0-100 范围内
        score.max(0).min(100) as u8
    }

    /// 把底层特征转成适合展示的关键指标列表。
    fn analyze_indicators(features: &OrderBookFeatures) -> Vec<KeyIndicator> {
        let mut indicators = Vec::new();

        // OBI 分析
        indicators.push(KeyIndicator {
            name: "订单簿不平衡 (OBI)".to_string(),
            value: format!("{:.2}%", features.obi),
            status: if features.obi > dec!(20) {
                IndicatorStatus::Bullish
            } else if features.obi < dec!(-20) {
                IndicatorStatus::Bearish
            } else {
                IndicatorStatus::Neutral
            },
            description: if features.obi > dec!(20) {
                "买方主导市场".to_string()
            } else if features.obi < dec!(-20) {
                "卖方主导市场".to_string()
            } else {
                "买卖相对平衡".to_string()
            },
        });

        // OFI 分析
        indicators.push(KeyIndicator {
            name: "订单流不平衡 (OFI)".to_string(),
            value: format!("{:.0}", features.ofi),
            status: if features.ofi > dec!(50000) {
                IndicatorStatus::Bullish
            } else if features.ofi < dec!(-50000) {
                IndicatorStatus::Bearish
            } else {
                IndicatorStatus::Neutral
            },
            description: if features.ofi > dec!(50000) {
                "买单主动吃筹".to_string()
            } else if features.ofi < dec!(-50000) {
                "卖单主动砸盘".to_string()
            } else {
                "订单流平稳".to_string()
            },
        });

        // 斜率分析
        indicators.push(KeyIndicator {
            name: "买单斜率".to_string(),
            value: format!("{:.0}", features.slope_bid),
            status: if features.slope_bid > dec!(1000000) {
                IndicatorStatus::Bullish
            } else if features.slope_bid < dec!(-1000000) {
                IndicatorStatus::Bearish
            } else {
                IndicatorStatus::Neutral
            },
            description: if features.slope_bid > dec!(1000000) {
                "买盘加速堆积".to_string()
            } else if features.slope_bid < dec!(-1000000) {
                "买盘快速撤离".to_string()
            } else {
                "买盘平稳".to_string()
            },
        });

        // 卖单斜率分析
        indicators.push(KeyIndicator {
            name: "卖单斜率".to_string(),
            value: format!("{:.0}", features.slope_ask),
            status: if features.slope_ask < dec!(-1000000) {
                IndicatorStatus::Bearish
            } else if features.slope_ask > dec!(1000000) {
                IndicatorStatus::Bullish
            } else {
                IndicatorStatus::Neutral
            },
            description: if features.slope_ask < dec!(-1000000) {
                "卖盘加速堆积".to_string()
            } else if features.slope_ask > dec!(1000000) {
                "卖盘快速撤离".to_string()
            } else {
                "卖盘平稳".to_string()
            },
        });

        // 大单占比
        indicators.push(KeyIndicator {
            name: "最大买单占比".to_string(),
            value: format!("{:.1}%", features.max_bid_ratio),
            status: if features.max_bid_ratio > dec!(30) {
                IndicatorStatus::Warning
            } else {
                IndicatorStatus::Neutral
            },
            description: if features.max_bid_ratio > dec!(30) {
                "存在托单或吸筹".to_string()
            } else {
                "大单分散".to_string()
            },
        });

        indicators.push(KeyIndicator {
            name: "最大卖单占比".to_string(),
            value: format!("{:.1}%", features.max_ask_ratio),
            status: if features.max_ask_ratio > dec!(30) {
                IndicatorStatus::Warning
            } else {
                IndicatorStatus::Neutral
            },
            description: if features.max_ask_ratio > dec!(30) {
                "存在压盘或出货".to_string()
            } else {
                "卖单分散".to_string()
            },
        });

        // 趋势强度
        indicators.push(KeyIndicator {
            name: "趋势强度".to_string(),
            value: format!("{:.1}", features.trend_strength),
            status: if features.trend_strength > dec!(30) {
                IndicatorStatus::Bullish
            } else if features.trend_strength < dec!(-30) {
                IndicatorStatus::Bearish
            } else {
                IndicatorStatus::Neutral
            },
            description: if features.trend_strength > dec!(30) {
                "上升趋势明确".to_string()
            } else if features.trend_strength < dec!(-30) {
                "下降趋势明确".to_string()
            } else {
                "无明显趋势".to_string()
            },
        });

        // 价差分析
        indicators.push(KeyIndicator {
            name: "价差 (bps)".to_string(),
            value: format!("{:.2}", features.spread_bps),
            status: if features.spread_bps > dec!(50) {
                IndicatorStatus::Warning
            } else if features.spread_bps > dec!(20) {
                IndicatorStatus::Neutral
            } else {
                IndicatorStatus::Bullish
            },
            description: if features.spread_bps > dec!(50) {
                "价差过大，流动性不足".to_string()
            } else if features.spread_bps > dec!(20) {
                "价差正常".to_string()
            } else {
                "价差很小，流动性好".to_string()
            },
        });

        indicators
    }

    // ==================== 高级指标计算 ====================

    /// 汇总计算所有高级指标分组。
    ///
    /// 大部分指标是解释性指标，适合前端面板或分析报告，并非撮合级精确定义。
    fn calculate_advanced_metrics(
        book: &OrderBook,
        features: &OrderBookFeatures,
        current_price: Decimal,
    ) -> AdvancedMetrics {
        // 1. 流动性深度指标
        let liquidity_score = Self::calculate_liquidity_score(book, features);
        let market_depth_ratio = if features.ask_volume_depth > Decimal::ZERO {
            features.bid_volume_depth / features.ask_volume_depth
        } else {
            dec!(1.0)
        };

        let bid_ask_concentration =
            (features.bid_concentration + features.ask_concentration) / dec!(2);

        // 计算滑点
        let (slippage_buy_1k, slippage_sell_1k) =
            Self::calculate_slippage(book, current_price, dec!(1000));
        let (slippage_buy_10k, slippage_sell_10k) =
            Self::calculate_slippage(book, current_price, dec!(10000));

        // 加权价差
        let weighted_spread = features.spread
            * (dec!(1) + (features.bid_volume_depth + features.ask_volume_depth) / dec!(1000000));

        // 2. 订单流质量指标
        let total_volume = features.total_bid_volume + features.total_ask_volume;
        let aggressive_order_ratio = if features.ofi.abs() > Decimal::ZERO {
            (features.ofi.abs() / total_volume * dec!(100)).min(dec!(100))
        } else {
            dec!(0)
        };

        let order_flow_quality = if aggressive_order_ratio > dec!(30) {
            dec!(80) // 高质量订单流（主动性强）
        } else if aggressive_order_ratio > dec!(15) {
            dec!(60)
        } else {
            dec!(40)
        };

        // 3. 价格发现指标
        let microprice_efficiency = if features.spread > Decimal::ZERO {
            ((features.microprice - current_price) / features.spread).abs()
        } else {
            dec!(0)
        };

        let weighted_mid_price =
            (features.weighted_bid_price + features.weighted_ask_price) / dec!(2);

        // 价格影响
        let price_impact_buy = if features.ask_volume_depth > Decimal::ZERO {
            (features.weighted_ask_price - current_price) / current_price * dec!(100)
        } else {
            dec!(0)
        };

        let price_impact_sell = if features.bid_volume_depth > Decimal::ZERO {
            (current_price - features.weighted_bid_price) / current_price * dec!(100)
        } else {
            dec!(0)
        };

        // 4. 波动率指标
        let realized_volatility_1m = features.price_change.abs() * dec!(2); // 简化计算
        let implied_volatility = (features.spread_bps / dec!(100) * dec!(16)).min(dec!(100)); // 简化的隐含波动率

        // 恐慌指数等效
        let vix_equivalent =
            (features.spread_bps * dec!(2) + features.obi.abs() / dec!(2)).min(dec!(100));

        // 5. 市场微观结构
        let market_efficiency_ratio = if features.spread_bps > Decimal::ZERO {
            (dec!(100) / features.spread_bps).min(dec!(100))
        } else {
            dec!(50)
        };

        let information_asymmetry = (features.max_bid_ratio + features.max_ask_ratio) / dec!(2);

        // 价格聚集度检测
        let price_clustering = Self::detect_price_clustering(book);

        // 订单簿恢复力
        let order_book_resilience = if features.bid_volume_change.abs()
            + features.ask_volume_change.abs()
            > Decimal::ZERO
        {
            dec!(100)
                - (features.bid_volume_change.abs() + features.ask_volume_change.abs()) / dec!(2)
        } else {
            dec!(70)
        };

        // 6. 资金流向指标
        let net_flow_volume = features.ofi;
        let buy_flow_pressure = if total_volume > Decimal::ZERO {
            features.total_bid_volume / total_volume * dec!(100)
        } else {
            dec!(50)
        };

        let sell_flow_pressure = dec!(100) - buy_flow_pressure;

        // 7. 风险指标
        let var_95_1m = features.price_change.abs() * dec!(1.645); // 简化的VaR计算
        let tail_risk_index =
            (features.max_bid_ratio + features.max_ask_ratio) / dec!(2) * dec!(1.5);

        // 8. 市场情绪指标
        let fear_greed_index = Self::calculate_fear_greed_index(features);
        let market_sentiment = if features.obi > dec!(10) {
            dec!(70) // 贪婪
        } else if features.obi < dec!(-10) {
            dec!(30) // 恐惧
        } else {
            dec!(50) // 中性
        };

        let bull_bear_ratio = if features.ask_volume_depth > Decimal::ZERO {
            features.bid_volume_depth / features.ask_volume_depth
        } else {
            dec!(1.0)
        };

        // 9. 做市商专用指标
        let inventory_risk = (features.bid_volume_depth - features.ask_volume_depth).abs()
            / (features.bid_volume_depth + features.ask_volume_depth)
            * dec!(100);

        let spread_profitability = features.spread_bps
            * (features.bid_volume_depth + features.ask_volume_depth)
            / dec!(10000);

        let optimal_spread = (features.spread * dec!(1.2)).min(dec!(0.001));

        AdvancedMetrics {
            // 流动性指标
            liquidity_score,
            market_depth_ratio,
            bid_ask_concentration,
            depth_elasticity: dec!(50), // 需要复杂计算
            quote_slippage_buy_1k: slippage_buy_1k,
            quote_slippage_sell_1k: slippage_sell_1k,
            quote_slippage_10k: (slippage_buy_10k + slippage_sell_10k) / dec!(2),
            weighted_spread,

            // 订单流指标
            order_flow_quality,
            aggressive_order_ratio,
            passive_order_ratio: dec!(100) - aggressive_order_ratio,
            order_cancellation_rate: dec!(10), // 需要实时数据
            order_submission_rate: dec!(15),   // 需要实时数据
            trade_flow_efficiency: dec!(60),
            order_book_imbalance_velocity: features.cum_delta,

            // 价格发现指标
            price_discovery_quality: dec!(70),
            microprice_efficiency,
            weighted_mid_price,
            price_impact_buy,
            price_impact_sell,
            market_impact_cost: (price_impact_buy + price_impact_sell) / dec!(2),

            // 波动率指标
            volatility_term_structure: dec!(50),
            realized_volatility_1m,
            realized_volatility_5m: realized_volatility_1m * dec!(1.5),
            implied_volatility,
            volatility_skew: dec!(0),
            volatility_cones: dec!(50),
            vix_equivalent,

            // 微观结构指标
            market_efficiency_ratio,
            information_asymmetry,
            adverse_selection_risk: information_asymmetry / dec!(2),
            price_clustering,
            tick_utilization: dec!(70),
            order_book_resilience,
            liquidity_provider_confidence: dec!(65),
            market_maker_profitability: spread_profitability,

            // 资金流向指标
            net_flow_volume,
            buy_flow_pressure,
            sell_flow_pressure,
            flow_concentration: (buy_flow_pressure - sell_flow_pressure).abs(),
            institutional_flow_ratio: aggressive_order_ratio,
            retail_flow_ratio: dec!(100) - aggressive_order_ratio,

            // 风险指标
            var_95_1m,
            cvar_95_1m: var_95_1m * dec!(1.2),
            expected_shortfall: var_95_1m * dec!(1.1),
            tail_risk_index,
            stress_test_scenario: dec!(30),
            correlation_risk: dec!(20),
            liquidity_adjusted_var: var_95_1m * (dec!(1) + inventory_risk / dec!(100)),
            systemic_risk_index: dec!(25),

            // 情绪指标
            fear_greed_index,
            market_sentiment,
            bull_bear_ratio,
            investor_confidence: market_sentiment,
            market_momentum: features.trend_strength,

            // 做市商指标
            inventory_risk,
            spread_profitability,
            adverse_selection_cost: information_asymmetry / dec!(3),
            inventory_decay_rate: dec!(5),
            optimal_spread,
            market_making_signal: features.ofi / dec!(100000),
            liquidity_provision_cost: features.spread_bps / dec!(2),
        }
    }

    // 计算流动性评分
    fn calculate_liquidity_score(book: &OrderBook, features: &OrderBookFeatures) -> Decimal {
        let mut score = dec!(50);

        // 价差评分 (越低分越高)
        if features.spread_bps < dec!(5) {
            score += dec!(30);
        } else if features.spread_bps < dec!(10) {
            score += dec!(20);
        } else if features.spread_bps < dec!(20) {
            score += dec!(10);
        } else if features.spread_bps > dec!(50) {
            score -= dec!(20);
        }

        // 深度评分
        let total_depth = features.bid_volume_depth + features.ask_volume_depth;
        if total_depth > dec!(1000000) {
            score += dec!(30);
        } else if total_depth > dec!(500000) {
            score += dec!(20);
        } else if total_depth > dec!(100000) {
            score += dec!(10);
        } else if total_depth < dec!(50000) {
            score -= dec!(20);
        }

        // 档位数量评分
        let total_levels = book.bids.len() + book.asks.len();
        if total_levels > 1500 {
            score += dec!(20);
        } else if total_levels > 1000 {
            score += dec!(10);
        } else if total_levels < 500 {
            score -= dec!(10);
        }

        score.max(dec!(0)).min(dec!(100))
    }

    // 计算滑点
    fn calculate_slippage(
        book: &OrderBook,
        start_price: Decimal,
        amount: Decimal,
    ) -> (Decimal, Decimal) {
        let mut buy_remaining = amount;
        let mut sell_remaining = amount;
        let mut buy_total_price = Decimal::ZERO;
        let mut sell_total_price = Decimal::ZERO;
        let mut buy_total_qty = Decimal::ZERO;
        let mut sell_total_qty = Decimal::ZERO;

        // 计算买入滑点
        for (price, qty) in book.asks.iter() {
            let trade_value = price * qty;
            if trade_value >= buy_remaining {
                let trade_qty = buy_remaining / price;
                buy_total_price += trade_qty * price;
                buy_total_qty += trade_qty;
                break;
            } else {
                buy_total_price += price * qty;
                buy_total_qty += qty;
                buy_remaining -= price * qty;
            }
        }

        // 计算卖出滑点
        for (Reverse(price), qty) in book.bids.iter() {
            let trade_value = price * qty;
            if trade_value >= sell_remaining {
                let trade_qty = sell_remaining / price;
                sell_total_price += trade_qty * price;
                sell_total_qty += trade_qty;
                break;
            } else {
                sell_total_price += price * qty;
                sell_total_qty += qty;
                sell_remaining -= price * qty;
            }
        }

        let buy_slippage = if buy_total_qty > Decimal::ZERO {
            ((buy_total_price / buy_total_qty - start_price) / start_price * dec!(100)).abs()
        } else {
            dec!(100)
        };

        let sell_slippage = if sell_total_qty > Decimal::ZERO {
            ((start_price - sell_total_price / sell_total_qty) / start_price * dec!(100)).abs()
        } else {
            dec!(100)
        };

        (buy_slippage, sell_slippage)
    }

    // 检测价格聚集度
    fn detect_price_clustering(book: &OrderBook) -> Decimal {
        let mut cluster_count = 0;
        let mut last_price = Decimal::ZERO;
        let tick_size = dec!(0.0001);

        for (Reverse(price), _) in book.bids.iter().take(30) {
            if last_price > Decimal::ZERO {
                let diff = (*price - last_price).abs();
                if diff < tick_size * dec!(5) {
                    cluster_count += 1;
                }
            }
            last_price = *price;
        }

        Decimal::from(cluster_count) / dec!(30) * dec!(100)
    }

    // 计算恐惧贪婪指数
    fn calculate_fear_greed_index(features: &OrderBookFeatures) -> Decimal {
        let mut score = dec!(50);

        // OBI贡献
        if features.obi > dec!(30) {
            score += dec!(20);
        } else if features.obi > dec!(15) {
            score += dec!(10);
        } else if features.obi < dec!(-30) {
            score -= dec!(20);
        } else if features.obi < dec!(-15) {
            score -= dec!(10);
        }

        // OFI贡献
        if features.ofi > dec!(100000) {
            score += dec!(15);
        } else if features.ofi < dec!(-100000) {
            score -= dec!(15);
        }

        // 波动率贡献
        if features.price_change.abs() > dec!(2) {
            score += dec!(10);
        } else if features.price_change.abs() < dec!(0.5) {
            score -= dec!(5);
        }

        // 鲸鱼活动贡献
        if features.whale_entry {
            score += dec!(15);
        }
        if features.whale_exit {
            score -= dec!(15);
        }

        score.max(dec!(0)).min(dec!(100))
    }

    // 高级支撑阻力查找
    fn find_support_resistance_advanced(
        book: &OrderBook,
        features: &OrderBookFeatures,
        current_price: Decimal,
    ) -> (
        Vec<(Decimal, Decimal, String)>,
        Vec<(Decimal, Decimal, String)>,
    ) {
        let mut supports = Vec::new();
        let mut resistances = Vec::new();

        // 查找支撑位（买单密集区）
        let bid_levels: Vec<_> = book
            .bids
            .iter()
            .take(50)
            .map(|(Reverse(p), q)| (*p, *q))
            .collect();

        for i in 0..bid_levels.len() {
            let (price, qty) = bid_levels[i];
            let volume_ratio = qty / features.total_bid_volume * dec!(100);

            if volume_ratio > dec!(2) {
                let distance = ((current_price - price) / current_price * dec!(100)).abs();
                let strength = if volume_ratio > dec!(10) {
                    format!(
                        "🔵 强支撑 ({}%, {:.1}%)",
                        volume_ratio.round_dp(1),
                        distance.round_dp(2)
                    )
                } else if volume_ratio > dec!(5) {
                    format!(
                        "🔹 中支撑 ({}%, {:.1}%)",
                        volume_ratio.round_dp(1),
                        distance.round_dp(2)
                    )
                } else {
                    format!(
                        "⚪ 弱支撑 ({}%, {:.1}%)",
                        volume_ratio.round_dp(1),
                        distance.round_dp(2)
                    )
                };
                supports.push((price, qty, strength));
            }
        }

        // 查找阻力位（卖单密集区）
        let ask_levels: Vec<_> = book.asks.iter().take(50).map(|(p, q)| (*p, *q)).collect();

        for i in 0..ask_levels.len() {
            let (price, qty) = ask_levels[i];
            let volume_ratio = qty / features.total_ask_volume * dec!(100);

            if volume_ratio > dec!(2) {
                let distance = ((price - current_price) / current_price * dec!(100)).abs();
                let strength = if volume_ratio > dec!(10) {
                    format!(
                        "🔴 强阻力 ({}%, {:.1}%)",
                        volume_ratio.round_dp(1),
                        distance.round_dp(2)
                    )
                } else if volume_ratio > dec!(5) {
                    format!(
                        "🔸 中阻力 ({}%, {:.1}%)",
                        volume_ratio.round_dp(1),
                        distance.round_dp(2)
                    )
                } else {
                    format!(
                        "⚪ 弱阻力 ({}%, {:.1}%)",
                        volume_ratio.round_dp(1),
                        distance.round_dp(2)
                    )
                };
                resistances.push((price, qty, strength));
            }
        }

        // 按距离排序
        supports.sort_by(|a, b| {
            let dist_a = (a.0 - current_price).abs();
            let dist_b = (b.0 - current_price).abs();
            dist_a.cmp(&dist_b)
        });

        resistances.sort_by(|a, b| {
            let dist_a = (a.0 - current_price).abs();
            let dist_b = (b.0 - current_price).abs();
            dist_a.cmp(&dist_b)
        });

        (supports, resistances)
    }

    // 高级鲸鱼意图检测
    fn detect_whale_intent_advanced(
        features: &OrderBookFeatures,
        advanced: &AdvancedMetrics,
    ) -> WhaleIntent {
        // 吸筹特征
        if features.whale_bid
            && advanced.buy_flow_pressure > advanced.sell_flow_pressure + dec!(10)
            && features.bid_volume_change > dec!(5)
            && advanced.fear_greed_index < dec!(30)
        {
            return WhaleIntent::Accumulating;
        }

        // 出货特征
        if features.whale_ask
            && advanced.sell_flow_pressure > advanced.buy_flow_pressure + dec!(10)
            && features.ask_volume_change > dec!(5)
            && advanced.fear_greed_index > dec!(70)
        {
            return WhaleIntent::Distributing;
        }

        // 操纵特征
        if (features.pump_signal || features.dump_signal)
            && advanced.order_flow_quality > dec!(70)
            && advanced.price_clustering > dec!(30)
        {
            return WhaleIntent::Manipulating;
        }

        // 观望特征
        if advanced.aggressive_order_ratio < dec!(10) && advanced.inventory_risk < dec!(20) {
            return WhaleIntent::Waiting;
        }

        WhaleIntent::Unknown
    }

    // 高级预测生成
    fn generate_forecasts_advanced(
        book: &OrderBook,
        features: &OrderBookFeatures,
        advanced: &AdvancedMetrics,
        current_price: Decimal,
    ) -> (Forecast, Forecast) {
        let short_term =
            Self::generate_short_term_forecast_advanced(book, features, advanced, current_price);
        let medium_term = Self::generate_medium_term_forecast_advanced(
            features,
            advanced,
            current_price,
            &short_term,
        );

        (short_term, medium_term)
    }

    fn generate_short_term_forecast_advanced(
        book: &OrderBook,
        features: &OrderBookFeatures,
        advanced: &AdvancedMetrics,
        current_price: Decimal,
    ) -> Forecast {
        let (best_bid, best_ask) = book
            .best_bid_ask()
            .unwrap_or((current_price * dec!(0.999), current_price * dec!(1.001)));

        // 多因子评分系统
        let mut bullish_score = 0;
        let mut bearish_score = 0;

        // OBI 贡献
        if features.obi > dec!(20) {
            bullish_score += 4;
        } else if features.obi > dec!(10) {
            bullish_score += 2;
        } else if features.obi < dec!(-20) {
            bearish_score += 4;
        } else if features.obi < dec!(-10) {
            bearish_score += 2;
        }

        // OFI 贡献
        if features.ofi > dec!(100000) {
            bullish_score += 5;
        } else if features.ofi > dec!(50000) {
            bullish_score += 3;
        } else if features.ofi < dec!(-100000) {
            bearish_score += 5;
        } else if features.ofi < dec!(-50000) {
            bearish_score += 3;
        }

        // 订单流质量贡献
        if advanced.order_flow_quality > dec!(70) {
            bullish_score += 3;
        } else if advanced.order_flow_quality < dec!(30) {
            bearish_score += 3;
        }

        // 恐慌指数贡献
        if advanced.fear_greed_index > dec!(70) {
            bearish_score += 2;
        }
        // 过度贪婪
        else if advanced.fear_greed_index < dec!(30) {
            bullish_score += 2;
        } // 过度恐惧

        // 价格影响贡献
        if advanced.price_impact_buy > advanced.price_impact_sell {
            bullish_score += 2;
        } else if advanced.price_impact_sell > advanced.price_impact_buy {
            bearish_score += 2;
        }

        // 判断方向
        let direction = if bullish_score > bearish_score + 5 {
            ForecastDirection::StrongBullish
        } else if bullish_score > bearish_score + 2 {
            ForecastDirection::Bullish
        } else if bearish_score > bullish_score + 5 {
            ForecastDirection::StrongBearish
        } else if bearish_score > bullish_score + 2 {
            ForecastDirection::Bearish
        } else {
            ForecastDirection::Neutral
        };

        // 计算概率
        let base_prob = match direction {
            ForecastDirection::StrongBullish => 80,
            ForecastDirection::Bullish => 65,
            ForecastDirection::Neutral => 50,
            ForecastDirection::Bearish => 65,
            ForecastDirection::StrongBearish => 80,
        };

        // 根据流动性调整概率
        let probability = if advanced.liquidity_score > dec!(70) {
            (base_prob as f64 * 1.1) as u8
        } else if advanced.liquidity_score < dec!(30) {
            (base_prob as f64 * 0.9) as u8
        } else {
            base_prob
        }
        .min(95);

        // 计算目标位
        let (target, stop_loss) = match direction {
            ForecastDirection::Bullish | ForecastDirection::StrongBullish => {
                let target = book
                    .asks
                    .iter()
                    .skip(3)
                    .next()
                    .map(|(p, _)| *p)
                    .unwrap_or(best_ask * dec!(1.015));
                let stop = best_bid * dec!(0.99);
                (target, stop)
            }
            ForecastDirection::Bearish | ForecastDirection::StrongBearish => {
                let target = book
                    .bids
                    .iter()
                    .skip(3)
                    .next()
                    .map(|(Reverse(p), _)| *p)
                    .unwrap_or(best_bid * dec!(0.985));
                let stop = best_ask * dec!(1.01);
                (target, stop)
            }
            _ => (current_price, current_price),
        };

        // 生成理由
        let reasoning = Self::generate_reasoning_advanced(features, advanced, direction.clone());

        Forecast {
            direction,
            probability,
            target,
            stop_loss,
            time_frame: "15-30分钟".to_string(),
            reasoning,
        }
    }

    fn generate_medium_term_forecast_advanced(
        features: &OrderBookFeatures,
        advanced: &AdvancedMetrics,
        current_price: Decimal,
        short_term: &Forecast,
    ) -> Forecast {
        let direction =
            if features.trend_strength > dec!(25) && advanced.fear_greed_index > dec!(60) {
                ForecastDirection::Bullish
            } else if features.trend_strength < dec!(-25) && advanced.fear_greed_index < dec!(40) {
                ForecastDirection::Bearish
            } else {
                short_term.direction.clone()
            };

        let probability = (features.trend_strength.abs().to_u64().unwrap_or(50) as u8)
            .min(75)
            .max(35);

        let (target, stop_loss) = match direction {
            ForecastDirection::Bullish | ForecastDirection::StrongBullish => {
                (current_price * dec!(1.03), current_price * dec!(0.975))
            }
            ForecastDirection::Bearish | ForecastDirection::StrongBearish => {
                (current_price * dec!(0.97), current_price * dec!(1.025))
            }
            _ => (current_price, current_price),
        };

        Forecast {
            direction,
            probability,
            target,
            stop_loss,
            time_frame: "1-2小时".to_string(),
            reasoning: format!(
                "趋势强度{:.1}，恐惧贪婪指数{:.1}，流动性评分{:.1}",
                features.trend_strength, advanced.fear_greed_index, advanced.liquidity_score
            ),
        }
    }

    fn generate_reasoning_advanced(
        features: &OrderBookFeatures,
        advanced: &AdvancedMetrics,
        direction: ForecastDirection,
    ) -> String {
        let mut reasons = Vec::new();

        match direction {
            ForecastDirection::Bullish | ForecastDirection::StrongBullish => {
                if features.ofi > dec!(100000) {
                    reasons.push(format!("OFI {:.0} 显示买单强劲", features.ofi));
                }
                if advanced.buy_flow_pressure > advanced.sell_flow_pressure + dec!(10) {
                    reasons.push(format!("买方压力{:.1}% 占优", advanced.buy_flow_pressure));
                }
                if advanced.fear_greed_index < dec!(30) {
                    reasons.push(format!(
                        "恐惧贪婪指数{:.1} 市场过度恐惧",
                        advanced.fear_greed_index
                    ));
                }
                if advanced.order_flow_quality > dec!(70) {
                    reasons.push("订单流质量高".to_string());
                }
            }
            ForecastDirection::Bearish | ForecastDirection::StrongBearish => {
                if features.ofi < dec!(-100000) {
                    reasons.push(format!("OFI {:.0} 显示卖单强劲", features.ofi));
                }
                if advanced.sell_flow_pressure > advanced.buy_flow_pressure + dec!(10) {
                    reasons.push(format!("卖方压力{:.1}% 占优", advanced.sell_flow_pressure));
                }
                if advanced.fear_greed_index > dec!(70) {
                    reasons.push(format!(
                        "恐惧贪婪指数{:.1} 市场过度贪婪",
                        advanced.fear_greed_index
                    ));
                }
                if advanced.tail_risk_index > dec!(50) {
                    reasons.push(format!(
                        "尾部风险指数{:.1} 风险较高",
                        advanced.tail_risk_index
                    ));
                }
            }
            _ => {
                reasons.push("多空力量相对均衡".to_string());
                if advanced.liquidity_score > dec!(70) {
                    reasons.push("流动性充足".to_string());
                }
            }
        }

        reasons.join("，")
    }

    // 高级风险提示
    fn generate_risk_warnings_advanced(
        _features: &OrderBookFeatures,
        advanced: &AdvancedMetrics,
    ) -> Vec<String> {
        let mut warnings = Vec::new();

        // 流动性风险
        if advanced.liquidity_score < dec!(30) {
            warnings.push(format!(
                "⚠️ 严重流动性不足 - 评分{:.1}",
                advanced.liquidity_score
            ));
        } else if advanced.liquidity_score < dec!(50) {
            warnings.push(format!(
                "⚠️ 流动性偏低 - 评分{:.1}",
                advanced.liquidity_score
            ));
        }

        // 滑点风险
        if advanced.quote_slippage_10k > dec!(2) {
            warnings.push(format!(
                "💰 大额交易滑点高 - {:.2}%",
                advanced.quote_slippage_10k
            ));
        }

        // VaR风险
        if advanced.var_95_1m > dec!(3) {
            warnings.push(format!(
                "📊 1分钟VaR(95%) {:.2}% - 风险较高",
                advanced.var_95_1m
            ));
        }

        // 尾部风险
        if advanced.tail_risk_index > dec!(60) {
            warnings.push(format!(
                "🐉 尾部风险指数 {:.1} - 警惕极端行情",
                advanced.tail_risk_index
            ));
        }

        // 库存风险
        if advanced.inventory_risk > dec!(50) {
            warnings.push(format!(
                "📦 库存风险 {:.1}% - 做市商可能调整报价",
                advanced.inventory_risk
            ));
        }

        // 市场情绪风险
        if advanced.fear_greed_index > dec!(80) {
            warnings.push("🤑 市场极度贪婪 - 回调风险增加".to_string());
        } else if advanced.fear_greed_index < dec!(20) {
            warnings.push("😱 市场极度恐惧 - 可能超跌反弹".to_string());
        }

        // 价格操纵风险
        if advanced.price_clustering > dec!(40) {
            warnings.push(format!(
                "🎯 价格聚集度 {:.1}% - 可能存在操纵",
                advanced.price_clustering
            ));
        }

        // 系统性风险
        if advanced.systemic_risk_index > dec!(50) {
            warnings.push(format!(
                "🌍 系统性风险指数 {:.1} - 关注宏观因素",
                advanced.systemic_risk_index
            ));
        }

        warnings
    }

    // 高级操作建议
    fn generate_recommendations_advanced(
        features: &OrderBookFeatures,
        whale_intent: &WhaleIntent,
        forecast: &Forecast,
        advanced: &AdvancedMetrics,
    ) -> Vec<String> {
        let mut recs = Vec::new();

        // 基于预测的建议
        match forecast.direction {
            ForecastDirection::StrongBullish => {
                recs.push("✅ 强烈看涨，可考虑建仓做多".to_string());
                recs.push(format!("🎯 目标位: {:.6}", forecast.target));
                recs.push(format!("🛑 止损位: {:.6}", forecast.stop_loss));
                let position_size = if advanced.liquidity_score > dec!(70) {
                    "30-40"
                } else {
                    "20-30"
                };
                recs.push(format!("📊 仓位建议: {}%", position_size));
            }
            ForecastDirection::Bullish => {
                recs.push("📈 谨慎看涨，可轻仓试多".to_string());
                recs.push(format!("🎯 目标位: {:.6}", forecast.target));
                recs.push(format!("🛑 止损位: {:.6}", forecast.stop_loss));
                let position_size = if advanced.liquidity_score > dec!(70) {
                    "15-20"
                } else {
                    "10-15"
                };
                recs.push(format!("📊 仓位建议: {}%", position_size));
            }
            ForecastDirection::Neutral => {
                recs.push("⏸️ 建议观望，等待明确信号".to_string());
                if advanced.liquidity_score < dec!(40) {
                    recs.push("⚠️ 流动性较差，不适合大额交易".to_string());
                }
            }
            ForecastDirection::Bearish => {
                recs.push("📉 谨慎看跌，可轻仓试空".to_string());
                recs.push(format!("🎯 目标位: {:.6}", forecast.target));
                recs.push(format!("🛑 止损位: {:.6}", forecast.stop_loss));
                let position_size = if advanced.liquidity_score > dec!(70) {
                    "15-20"
                } else {
                    "10-15"
                };
                recs.push(format!("📊 仓位建议: {}%", position_size));
            }
            ForecastDirection::StrongBearish => {
                recs.push("❌ 强烈看跌，可考虑做空".to_string());
                recs.push(format!("🎯 目标位: {:.6}", forecast.target));
                recs.push(format!("🛑 止损位: {:.6}", forecast.stop_loss));
                let position_size = if advanced.liquidity_score > dec!(70) {
                    "30-40"
                } else {
                    "20-30"
                };
                recs.push(format!("📊 仓位建议: {}%", position_size));
            }
        }

        // 基于鲸鱼意图的建议
        match whale_intent {
            WhaleIntent::Accumulating => {
                recs.push("🐋 鲸鱼正在吸筹，可跟随主力方向分批建仓".to_string());
            }
            WhaleIntent::Distributing => {
                recs.push("🐋 鲸鱼正在出货，建议逢高减仓".to_string());
            }
            WhaleIntent::Manipulating => {
                recs.push("🎭 主力正在操纵，设置宽止损，勿追涨杀跌".to_string());
            }
            WhaleIntent::Waiting => {
                recs.push("👀 鲸鱼观望中，等待明确信号再入场".to_string());
            }
            _ => {}
        }

        // 做市商专用建议
        if advanced.spread_profitability > dec!(10) {
            recs.push(format!(
                "💰 价差盈利性{:.2}，适合做市",
                advanced.spread_profitability
            ));
        }
        if advanced.inventory_risk > dec!(40) {
            recs.push("📦 库存风险较高，建议降低持仓".to_string());
        }
        if advanced.optimal_spread > features.spread {
            recs.push(format!(
                "🎯 最优价差{:.6}，可考虑扩大挂单范围",
                advanced.optimal_spread
            ));
        }

        // 风险管理建议
        if advanced.var_95_1m > dec!(2) {
            recs.push(format!(
                "📊 基于VaR{:.2}%，建议降低杠杆",
                advanced.var_95_1m
            ));
        }
        if advanced.tail_risk_index > dec!(50) {
            recs.push("🛡️ 尾部风险较高，考虑买入期权对冲".to_string());
        }

        // 情绪相关建议
        if advanced.fear_greed_index > dec!(80) {
            recs.push("🤑 市场过度贪婪，分批止盈".to_string());
        } else if advanced.fear_greed_index < dec!(20) {
            recs.push("😱 市场过度恐惧，可考虑左侧布局".to_string());
        }

        recs
    }

    // 将报告输出到文件
    pub fn write_to_file(&self, file_path: &str) -> std::io::Result<()> {
        let content = self.format_to_string();
        let mut file = OpenOptions::new()
            .create(true)
            .append(true)
            .open(file_path)?;
        writeln!(file, "{}", content)?;
        Ok(())
    }

    // 将报告格式化为字符串
    fn format_to_string(&self) -> String {
        let mut output = String::new();

        output.push_str(&format!("\n{}", "=".repeat(120)));
        output.push_str(&format!(
            "\n📊 市场分析报告 - {}",
            self.timestamp.format("%Y-%m-%d %H:%M:%S")
        ));
        output.push_str(&format!("\n{}", "=".repeat(120)));

        // 市场概览
        output.push_str("\n\n📌 市场概览");
        output.push_str(&format!("\n{}", "-".repeat(60)));
        output.push_str(&format!("\n当前价格: {:.6}", self.price_level));

        let regime_str = match self.market_regime {
            MarketRegime::Accumulation => "📈 吸筹阶段",
            MarketRegime::Distribution => "📉 出货阶段",
            MarketRegime::MarkUp => "🚀 拉升阶段",
            MarketRegime::MarkDown => "💥 砸盘阶段",
            MarketRegime::Sideways => "➡️ 横盘整理",
            MarketRegime::Volatile => "⚡ 剧烈波动",
        };
        output.push_str(&format!(
            "\n市场状态: {} (置信度 {}%)",
            regime_str, self.confidence
        ));

        let intent_str = match self.whale_intent {
            WhaleIntent::Accumulating => "🐋 正在吸筹",
            WhaleIntent::Distributing => "🐋 正在出货",
            WhaleIntent::Manipulating => "🎭 正在操纵",
            WhaleIntent::Waiting => "👀 观望等待",
            WhaleIntent::Unknown => "❓ 意图不明",
        };
        output.push_str(&format!("\n主力意图: {}", intent_str));

        // 基础关键指标
        output.push_str("\n\n📊 基础指标分析");
        output.push_str(&format!("\n{}", "-".repeat(80)));
        for indicator in &self.key_indicators {
            let status_symbol = match indicator.status {
                IndicatorStatus::Bullish => "🟢",
                IndicatorStatus::Bearish => "🔴",
                IndicatorStatus::Neutral => "⚪",
                IndicatorStatus::Warning => "🟡",
            };
            output.push_str(&format!(
                "\n{} {}: {} - {}",
                status_symbol, indicator.name, indicator.value, indicator.description
            ));
        }

        // 1. 流动性深度指标
        output.push_str("\n\n💧 流动性深度指标");
        output.push_str(&format!("\n{}", "-".repeat(60)));
        output.push_str(&format!(
            "\n流动性综合评分: {:.1}/100",
            self.advanced_metrics.liquidity_score
        ));
        output.push_str(&format!(
            "\n深度比率(买/卖): {:.3}",
            self.advanced_metrics.market_depth_ratio
        ));
        output.push_str(&format!(
            "\n买卖盘集中度: {:.1}%",
            self.advanced_metrics.bid_ask_concentration
        ));
        output.push_str(&format!(
            "\n1kU买入滑点: {:.3}%",
            self.advanced_metrics.quote_slippage_buy_1k
        ));
        output.push_str(&format!(
            "\n1kU卖出滑点: {:.3}%",
            self.advanced_metrics.quote_slippage_sell_1k
        ));
        output.push_str(&format!(
            "\n10kU平均滑点: {:.3}%",
            self.advanced_metrics.quote_slippage_10k
        ));
        output.push_str(&format!(
            "\n加权价差: {:.6}",
            self.advanced_metrics.weighted_spread
        ));

        // 2. 订单流质量指标
        output.push_str("\n\n📈 订单流质量指标");
        output.push_str(&format!("\n{}", "-".repeat(60)));
        output.push_str(&format!(
            "\n订单流质量: {:.1}/100",
            self.advanced_metrics.order_flow_quality
        ));
        output.push_str(&format!(
            "\n主动单比例: {:.1}%",
            self.advanced_metrics.aggressive_order_ratio
        ));
        output.push_str(&format!(
            "\n被动单比例: {:.1}%",
            self.advanced_metrics.passive_order_ratio
        ));
        output.push_str(&format!(
            "\n订单流失衡速度: {:.0}",
            self.advanced_metrics.order_book_imbalance_velocity
        ));

        // 3. 价格发现指标
        output.push_str("\n\n🎯 价格发现指标");
        output.push_str(&format!("\n{}", "-".repeat(60)));
        output.push_str(&format!(
            "\n微价格效率: {:.3}",
            self.advanced_metrics.microprice_efficiency
        ));
        output.push_str(&format!(
            "\n加权中间价: {:.6}",
            self.advanced_metrics.weighted_mid_price
        ));
        output.push_str(&format!(
            "\n买入价格影响: {:.3}%",
            self.advanced_metrics.price_impact_buy
        ));
        output.push_str(&format!(
            "\n卖出价格影响: {:.3}%",
            self.advanced_metrics.price_impact_sell
        ));
        output.push_str(&format!(
            "\n市场冲击成本: {:.3}%",
            self.advanced_metrics.market_impact_cost
        ));

        // 4. 波动率指标
        output.push_str("\n\n🌊 波动率指标");
        output.push_str(&format!("\n{}", "-".repeat(60)));
        output.push_str(&format!(
            "\n1分钟已实现波动率: {:.3}%",
            self.advanced_metrics.realized_volatility_1m
        ));
        output.push_str(&format!(
            "\n5分钟已实现波动率: {:.3}%",
            self.advanced_metrics.realized_volatility_5m
        ));
        output.push_str(&format!(
            "\n隐含波动率: {:.1}%",
            self.advanced_metrics.implied_volatility
        ));
        output.push_str(&format!(
            "\n恐慌指数等效: {:.1}",
            self.advanced_metrics.vix_equivalent
        ));

        // 5. 市场微观结构
        output.push_str("\n\n🔬 市场微观结构");
        output.push_str(&format!("\n{}", "-".repeat(60)));
        output.push_str(&format!(
            "\n市场效率比率: {:.1}/100",
            self.advanced_metrics.market_efficiency_ratio
        ));
        output.push_str(&format!(
            "\n信息不对称度: {:.1}%",
            self.advanced_metrics.information_asymmetry
        ));
        output.push_str(&format!(
            "\n价格聚集度: {:.1}%",
            self.advanced_metrics.price_clustering
        ));
        output.push_str(&format!(
            "\n订单簿恢复力: {:.1}/100",
            self.advanced_metrics.order_book_resilience
        ));
        output.push_str(&format!(
            "\n做市商盈利性: {:.3}",
            self.advanced_metrics.market_maker_profitability
        ));

        // 6. 资金流向指标
        output.push_str("\n\n💰 资金流向指标");
        output.push_str(&format!("\n{}", "-".repeat(60)));
        output.push_str(&format!(
            "\n净流量: {:.0}",
            self.advanced_metrics.net_flow_volume
        ));
        output.push_str(&format!(
            "\n买入流压力: {:.1}%",
            self.advanced_metrics.buy_flow_pressure
        ));
        output.push_str(&format!(
            "\n卖出流压力: {:.1}%",
            self.advanced_metrics.sell_flow_pressure
        ));
        output.push_str(&format!(
            "\n机构流比例: {:.1}%",
            self.advanced_metrics.institutional_flow_ratio
        ));
        output.push_str(&format!(
            "\n散户流比例: {:.1}%",
            self.advanced_metrics.retail_flow_ratio
        ));

        // 7. 风险指标
        output.push_str("\n\n⚠️ 风险指标");
        output.push_str(&format!("\n{}", "-".repeat(60)));
        output.push_str(&format!(
            "\nVaR(95% 1分钟): {:.3}%",
            self.advanced_metrics.var_95_1m
        ));
        output.push_str(&format!(
            "\nCVaR(95% 1分钟): {:.3}%",
            self.advanced_metrics.cvar_95_1m
        ));
        output.push_str(&format!(
            "\n尾部风险指数: {:.1}/100",
            self.advanced_metrics.tail_risk_index
        ));
        output.push_str(&format!(
            "\n流动性调整VaR: {:.3}%",
            self.advanced_metrics.liquidity_adjusted_var
        ));
        output.push_str(&format!(
            "\n系统性风险指数: {:.1}/100",
            self.advanced_metrics.systemic_risk_index
        ));

        // 8. 市场情绪指标
        output.push_str("\n\n😊 市场情绪指标");
        output.push_str(&format!("\n{}", "-".repeat(60)));
        output.push_str(&format!(
            "\n恐惧贪婪指数: {:.1}/100",
            self.advanced_metrics.fear_greed_index
        ));
        output.push_str(&format!(
            "\n市场情绪: {:.1}/100",
            self.advanced_metrics.market_sentiment
        ));
        output.push_str(&format!(
            "\n多空比: {:.3}",
            self.advanced_metrics.bull_bear_ratio
        ));
        output.push_str(&format!(
            "\n投资者信心: {:.1}/100",
            self.advanced_metrics.investor_confidence
        ));
        output.push_str(&format!(
            "\n市场动量: {:.1}",
            self.advanced_metrics.market_momentum
        ));

        // 9. 做市商专用指标
        output.push_str("\n\n🏦 做市商专用指标");
        output.push_str(&format!("\n{}", "-".repeat(60)));
        output.push_str(&format!(
            "\n库存风险: {:.1}%",
            self.advanced_metrics.inventory_risk
        ));
        output.push_str(&format!(
            "\n价差盈利性: {:.3}",
            self.advanced_metrics.spread_profitability
        ));
        output.push_str(&format!(
            "\n逆向选择成本: {:.3}",
            self.advanced_metrics.adverse_selection_cost
        ));
        output.push_str(&format!(
            "\n最优价差: {:.6}",
            self.advanced_metrics.optimal_spread
        ));
        output.push_str(&format!(
            "\n做市信号: {:.0}",
            self.advanced_metrics.market_making_signal
        ));
        output.push_str(&format!(
            "\n流动性提供成本: {:.3}",
            self.advanced_metrics.liquidity_provision_cost
        ));

        // 支撑阻力
        output.push_str("\n\n🛡️ 支撑阻力位 (按距离排序)");
        output.push_str(&format!("\n{}", "-".repeat(60)));
        output.push_str("\n【支撑位】");
        if !self.support_levels.is_empty() {
            for (i, (price, qty, strength)) in self.support_levels.iter().take(5).enumerate() {
                output.push_str(&format!(
                    "\n  {}. {:.6} - {:.0} {}",
                    i + 1,
                    price,
                    qty.round_dp(0),
                    strength
                ));
            }
        } else {
            output.push_str("\n  暂无明显支撑位");
        }

        output.push_str("\n\n【阻力位】");
        if !self.resistance_levels.is_empty() {
            for (i, (price, qty, strength)) in self.resistance_levels.iter().take(5).enumerate() {
                output.push_str(&format!(
                    "\n  {}. {:.6} - {:.0} {}",
                    i + 1,
                    price,
                    qty.round_dp(0),
                    strength
                ));
            }
        } else {
            output.push_str("\n  暂无明显阻力位");
        }

        // 预测
        output.push_str("\n\n🎯 走势预测");
        output.push_str(&format!("\n{}", "-".repeat(80)));

        // 短期预测
        let short_dir = match self.short_term_forecast.direction {
            ForecastDirection::StrongBullish => "🚀 强烈看涨",
            ForecastDirection::Bullish => "📈 看涨",
            ForecastDirection::Neutral => "➡️ 横盘",
            ForecastDirection::Bearish => "📉 看跌",
            ForecastDirection::StrongBearish => "💥 强烈看跌",
        };
        output.push_str(&format!(
            "\n【短期 {}】",
            self.short_term_forecast.time_frame
        ));
        output.push_str(&format!("\n  方向: {}", short_dir));
        output.push_str(&format!(
            "\n  概率: {}%",
            self.short_term_forecast.probability
        ));
        output.push_str(&format!("\n  目标: {:.6}", self.short_term_forecast.target));
        output.push_str(&format!(
            "\n  止损: {:.6}",
            self.short_term_forecast.stop_loss
        ));
        output.push_str(&format!("\n  理由: {}", self.short_term_forecast.reasoning));

        // 中期预测
        let medium_dir = match self.medium_term_forecast.direction {
            ForecastDirection::StrongBullish => "🚀 强烈看涨",
            ForecastDirection::Bullish => "📈 看涨",
            ForecastDirection::Neutral => "➡️ 横盘",
            ForecastDirection::Bearish => "📉 看跌",
            ForecastDirection::StrongBearish => "💥 强烈看跌",
        };
        output.push_str(&format!(
            "\n\n【中期 {}】",
            self.medium_term_forecast.time_frame
        ));
        output.push_str(&format!("\n  方向: {}", medium_dir));
        output.push_str(&format!(
            "\n  概率: {}%",
            self.medium_term_forecast.probability
        ));
        output.push_str(&format!(
            "\n  目标: {:.6}",
            self.medium_term_forecast.target
        ));
        output.push_str(&format!(
            "\n  止损: {:.6}",
            self.medium_term_forecast.stop_loss
        ));
        output.push_str(&format!(
            "\n  理由: {}",
            self.medium_term_forecast.reasoning
        ));

        // 风险提示
        if !self.risk_warnings.is_empty() {
            output.push_str(&format!("\n\n⚠️ 风险提示 ({})", self.risk_warnings.len()));
            output.push_str(&format!("\n{}", "-".repeat(60)));
            for warning in &self.risk_warnings {
                output.push_str(&format!("\n  {}", warning));
            }
        }

        // 操作建议
        output.push_str("\n\n💡 操作建议");
        output.push_str(&format!("\n{}", "-".repeat(60)));
        for rec in &self.recommendations {
            output.push_str(&format!("\n  {}", rec));
        }

        output.push_str(&format!("\n\n{}", "=".repeat(120)));
        output.push_str("\n");

        output
    }

    // 重写display函数以包含高级指标
    pub fn display(&self) {
        println!("{}", self.format_to_string());
    }
}
