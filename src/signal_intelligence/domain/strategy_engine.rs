use serde::{Deserialize, Serialize};
use rust_decimal::prelude::ToPrimitive;

use crate::market_data::domain::order_book::OrderBookFeatures;
use crate::signal_intelligence::domain::algorithms::{
    AlphaSignal, ComprehensiveAnalysis, SpoofingType,
};

#[derive(Debug, Clone, Default)]
pub struct StrategyWindowContext {
    pub label: String,
    pub window_secs: u64,
    pub trade_buy_ratio: f64,
    pub cvd_delta: f64,
    pub ofi_sum: f64,
    pub price_change_bps: f64,
    pub microprice_lead_bps: f64,
    pub depth_imbalance: f64,
    pub bid_depth_change_pct: f64,
    pub ask_depth_change_pct: f64,
    pub large_trade_buy_ratio: f64,
    pub sweep_buy_ratio: f64,
    pub total_trade_notional: f64,
}

#[derive(Debug, Clone, Default)]
pub struct StrategyMarketContext {
    pub windows: Vec<StrategyWindowContext>,
    pub adaptive_ready: bool,
    pub current_ofi_zscore: Option<f64>,
    pub current_obi_zscore: Option<f64>,
    pub current_vol_zscore: Option<f64>,
    pub current_spread_zscore: Option<f64>,
    pub kline_return_1m: f64,
    pub kline_return_5m: f64,
    pub kline_return_15m: f64,
    pub kline_return_1h: f64,
    pub range_expansion_1m: f64,
    pub breakout_up: bool,
    pub breakout_down: bool,
    pub breakout_acceptance: f64,
    pub anomaly_count_1m: u32,
    pub anomaly_max_severity: u8,
    pub recent_trade_count_60s: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct StrategyWindowSignal {
    pub label: String,
    pub window_secs: u64,
    pub bias_score: f64,
    pub trade_buy_ratio: f64,
    pub cvd_delta: f64,
    pub ofi_sum: f64,
    pub price_change_bps: f64,
    pub microprice_lead_bps: f64,
    pub depth_imbalance: f64,
    pub bid_depth_change_pct: f64,
    pub ask_depth_change_pct: f64,
    pub large_trade_buy_ratio: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct StrategyProfile {
    pub phase: String,
    pub direction: String,
    pub regime: String,
    pub calibration_mode: String,
    pub confidence: u8,
    pub pump_probability: u8,
    pub dump_probability: u8,
    pub continuation_probability: u8,
    pub false_breakout_probability: u8,
    pub reversal_risk: u8,
    pub expected_window_secs: u32,
    pub invalidation: String,
    pub reasons: Vec<String>,
    pub windows: Vec<StrategyWindowSignal>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Direction {
    Bullish,
    Bearish,
    Neutral,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Phase {
    Neutral,
    Accumulation,
    Distribution,
    Ignition,
    Continuation,
    Exhaustion,
    FalseBreakout,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Regime {
    Balanced,
    Expansion,
    Squeeze,
    Volatile,
    Illiquid,
}

pub struct StrategyEngine;

impl StrategyEngine {
    pub fn analyze(
        features: &OrderBookFeatures,
        analysis: &ComprehensiveAnalysis,
        ctx: &StrategyMarketContext,
    ) -> StrategyProfile {
        let mut window_signals = ctx
            .windows
            .iter()
            .map(|window| StrategyWindowSignal {
                label: window.label.clone(),
                window_secs: window.window_secs,
                bias_score: window_bias(window),
                trade_buy_ratio: window.trade_buy_ratio,
                cvd_delta: window.cvd_delta,
                ofi_sum: window.ofi_sum,
                price_change_bps: window.price_change_bps,
                microprice_lead_bps: window.microprice_lead_bps,
                depth_imbalance: window.depth_imbalance,
                bid_depth_change_pct: window.bid_depth_change_pct,
                ask_depth_change_pct: window.ask_depth_change_pct,
                large_trade_buy_ratio: window.large_trade_buy_ratio,
            })
            .collect::<Vec<_>>();

        let short_bias = weighted_window_bias(&window_signals, &[3, 10]);
        let medium_bias = weighted_window_bias(&window_signals, &[30, 60]);
        let composite_bias = weighted_window_bias(&window_signals, &[3, 10, 30, 60]);
        let alpha_bias = alpha_bias(&analysis.alpha.signal);
        let whale_bias = whale_bias(analysis);
        let z_bias = zscore_bias(ctx);
        let spoofing_penalty = spoofing_penalty(analysis);
        let directional_edge = composite_bias + alpha_bias + whale_bias + z_bias - spoofing_penalty;
        let direction = direction_from_edge(directional_edge);
        let coherence = directional_coherence(direction, ctx);
        let regime = classify_regime(features, ctx);

        let base_pump = analysis.pump_dump.pump_probability as f64;
        let base_dump = analysis.pump_dump.dump_probability as f64;
        let directional_center = clamp(50.0 + directional_edge, 0.0, 100.0);
        let mut pump_probability = weighted_average(base_pump, directional_center, 0.35, 0.65);
        let mut dump_probability = weighted_average(base_dump, 100.0 - directional_center, 0.35, 0.65);

        if direction == Direction::Bullish {
            pump_probability += clamp(coherence * 0.10 + positive(ctx.current_ofi_zscore) * 6.0, 0.0, 16.0);
            dump_probability -= clamp(coherence * 0.04, 0.0, 8.0);
        } else if direction == Direction::Bearish {
            dump_probability += clamp(coherence * 0.10 + negative(ctx.current_ofi_zscore) * 6.0, 0.0, 16.0);
            pump_probability -= clamp(coherence * 0.04, 0.0, 8.0);
        }

        let false_breakout_probability = calc_false_breakout_probability(features, analysis, ctx, short_bias, medium_bias, coherence);
        let reversal_risk = calc_reversal_risk(features, analysis, ctx, short_bias, medium_bias, direction, false_breakout_probability);
        let continuation_probability = calc_continuation_probability(ctx, short_bias, medium_bias, coherence, direction, false_breakout_probability, reversal_risk);

        if false_breakout_probability > 65.0 {
            pump_probability -= 8.0;
            dump_probability -= 8.0;
        }
        if reversal_risk > 70.0 {
            if direction == Direction::Bullish {
                pump_probability -= 10.0;
            } else if direction == Direction::Bearish {
                dump_probability -= 10.0;
            }
        }

        pump_probability = clamp(pump_probability, 0.0, 100.0);
        dump_probability = clamp(dump_probability, 0.0, 100.0);

        let phase = classify_phase(
            direction,
            short_bias,
            medium_bias,
            pump_probability,
            dump_probability,
            continuation_probability,
            false_breakout_probability,
            reversal_risk,
            ctx,
        );
        let confidence = calc_confidence(
            analysis,
            ctx,
            direction,
            phase,
            coherence,
            pump_probability,
            dump_probability,
            false_breakout_probability,
            reversal_risk,
        );

        let expected_window_secs = match phase {
            Phase::Accumulation | Phase::Distribution => 60,
            Phase::Ignition => 20,
            Phase::Continuation => 180,
            Phase::Exhaustion | Phase::FalseBreakout => 15,
            Phase::Neutral => 45,
        };
        let invalidation = invalidation_text(direction, phase);

        let mut reasons = build_reasons(
            &window_signals,
            analysis,
            ctx,
            direction,
            phase,
            pump_probability,
            dump_probability,
            false_breakout_probability,
            reversal_risk,
            coherence,
        );
        if reasons.len() > 5 {
            reasons.truncate(5);
        }

        StrategyProfile {
            phase: phase_label(phase).to_string(),
            direction: direction_label(direction).to_string(),
            regime: regime_label(regime).to_string(),
            calibration_mode: if ctx.adaptive_ready { "自适应" } else { "冷启动" }.to_string(),
            confidence,
            pump_probability: clamp(pump_probability.round(), 0.0, 100.0) as u8,
            dump_probability: clamp(dump_probability.round(), 0.0, 100.0) as u8,
            continuation_probability: clamp(continuation_probability.round(), 0.0, 100.0) as u8,
            false_breakout_probability: clamp(false_breakout_probability.round(), 0.0, 100.0) as u8,
            reversal_risk: clamp(reversal_risk.round(), 0.0, 100.0) as u8,
            expected_window_secs,
            invalidation,
            reasons,
            windows: std::mem::take(&mut window_signals),
        }
    }
}

fn weighted_window_bias(windows: &[StrategyWindowSignal], window_secs: &[u64]) -> f64 {
    let mut score = 0.0;
    let mut weight_sum = 0.0;
    for window in windows {
        let secs = window.window_secs;
        if !window_secs.contains(&secs) {
            continue;
        }
        let weight = window_weight(secs);
        score += window.bias_score * weight;
        weight_sum += weight;
    }
    if weight_sum <= 0.0 {
        0.0
    } else {
        score / weight_sum
    }
}

fn window_bias(window: &StrategyWindowContext) -> f64 {
    let trade_edge = clamp((window.trade_buy_ratio - 50.0) * 1.1, -20.0, 20.0);
    let cvd_edge = signed_magnitude(window.cvd_delta, 7_500.0, 16.0);
    let ofi_edge = signed_magnitude(window.ofi_sum, 12_000.0, 18.0);
    let micro_edge = clamp(window.microprice_lead_bps * 2.3, -18.0, 18.0);
    let depth_edge = clamp(
        (window.bid_depth_change_pct - window.ask_depth_change_pct) * 0.45
            + window.depth_imbalance * 0.20,
        -20.0,
        20.0,
    );
    let large_edge = clamp((window.large_trade_buy_ratio - 50.0) * 0.35, -10.0, 10.0);
    let sweep_edge = clamp((window.sweep_buy_ratio - 50.0) * 0.45, -12.0, 12.0);
    let activity_scale = if window.total_trade_notional > 0.0 {
        clamp((window.total_trade_notional + 1.0).ln_1p() / 6.0, 0.65, 1.15)
    } else {
        0.65
    };
    clamp(
        (trade_edge + cvd_edge + ofi_edge + micro_edge + depth_edge + large_edge + sweep_edge)
            * activity_scale,
        -100.0,
        100.0,
    )
}

fn alpha_bias(signal: &AlphaSignal) -> f64 {
    match signal {
        AlphaSignal::StrongBuy => 14.0,
        AlphaSignal::Buy => 8.0,
        AlphaSignal::Neutral => 0.0,
        AlphaSignal::Sell => -8.0,
        AlphaSignal::StrongSell => -14.0,
    }
}

fn whale_bias(analysis: &ComprehensiveAnalysis) -> f64 {
    ((analysis.whale.accumulation_score.to_f64().unwrap_or(0.0)
        - analysis.whale.distribution_score.to_f64().unwrap_or(0.0))
        * 0.18)
        .clamp(-18.0, 18.0)
}

fn zscore_bias(ctx: &StrategyMarketContext) -> f64 {
    let bull = positive(ctx.current_ofi_zscore) * 8.0
        + positive(ctx.current_obi_zscore) * 6.0
        + positive(ctx.current_vol_zscore) * 4.0
        + negative(ctx.current_spread_zscore) * 3.0;
    let bear = negative(ctx.current_ofi_zscore) * 8.0
        + negative(ctx.current_obi_zscore) * 6.0
        + positive(ctx.current_spread_zscore) * 3.0;
    clamp(bull - bear, -22.0, 22.0)
}

fn spoofing_penalty(analysis: &ComprehensiveAnalysis) -> f64 {
    if !analysis.spoofing.detected {
        return 0.0;
    }
    match analysis.spoofing.spoofing_type {
        SpoofingType::BidSpoofing | SpoofingType::AskSpoofing | SpoofingType::Layering => 8.0,
        _ => 4.0,
    }
}

fn direction_from_edge(edge: f64) -> Direction {
    if edge > 8.0 {
        Direction::Bullish
    } else if edge < -8.0 {
        Direction::Bearish
    } else {
        Direction::Neutral
    }
}

fn directional_coherence(direction: Direction, ctx: &StrategyMarketContext) -> f64 {
    let expected = match direction {
        Direction::Bullish => 1.0,
        Direction::Bearish => -1.0,
        Direction::Neutral => 0.0,
    };
    if expected == 0.0 {
        return 45.0;
    }
    let mut votes = Vec::new();
    for value in [ctx.kline_return_1m, ctx.kline_return_5m, ctx.kline_return_15m, ctx.kline_return_1h] {
        if value.abs() > 0.0001 {
            votes.push(value.signum());
        }
    }
    if votes.is_empty() {
        return 48.0;
    }
    let aligned = votes.iter().filter(|value| (**value - expected).abs() < f64::EPSILON).count();
    clamp(aligned as f64 / votes.len() as f64 * 100.0, 0.0, 100.0)
}

fn classify_regime(features: &OrderBookFeatures, ctx: &StrategyMarketContext) -> Regime {
    let spread_bps = features.spread_bps.to_f64().unwrap_or(0.0);
    if spread_bps > 30.0 || positive(ctx.current_spread_zscore) > 1.8 {
        Regime::Illiquid
    } else if ctx.anomaly_max_severity >= 80 || ctx.range_expansion_1m > 80.0 {
        Regime::Volatile
    } else if positive(ctx.current_vol_zscore) > 1.2 && ctx.kline_return_1m.abs() > 0.002 {
        Regime::Expansion
    } else if spread_bps < 8.0 && ctx.range_expansion_1m < 40.0 {
        Regime::Squeeze
    } else {
        Regime::Balanced
    }
}

fn calc_false_breakout_probability(
    features: &OrderBookFeatures,
    analysis: &ComprehensiveAnalysis,
    ctx: &StrategyMarketContext,
    short_bias: f64,
    medium_bias: f64,
    coherence: f64,
) -> f64 {
    let mut risk = 18.0;
    if features.fake_breakout {
        risk += 25.0;
    }
    if ctx.breakout_up && short_bias < -5.0 {
        risk += 18.0;
    }
    if ctx.breakout_down && short_bias > 5.0 {
        risk += 18.0;
    }
    if (ctx.breakout_up || ctx.breakout_down) && ctx.breakout_acceptance < 55.0 {
        risk += 16.0;
    }
    if (ctx.breakout_up || ctx.breakout_down) && coherence < 50.0 {
        risk += 12.0;
    }
    if (ctx.breakout_up || ctx.breakout_down) && ctx.recent_trade_count_60s < 10 {
        risk += 8.0;
    }
    if analysis.spoofing.detected {
        risk += 10.0;
    }
    if short_bias.signum() != medium_bias.signum() && short_bias.abs() > 10.0 && medium_bias.abs() > 10.0 {
        risk += 8.0;
    }
    clamp(risk, 0.0, 100.0)
}

fn calc_reversal_risk(
    features: &OrderBookFeatures,
    analysis: &ComprehensiveAnalysis,
    ctx: &StrategyMarketContext,
    short_bias: f64,
    medium_bias: f64,
    direction: Direction,
    false_breakout_probability: f64,
) -> f64 {
    let mut risk = 12.0 + false_breakout_probability * 0.35;
    let spread_stress = positive(ctx.current_spread_zscore) * 8.0;
    risk += spread_stress;
    if ctx.anomaly_max_severity >= 75 {
        risk += 10.0;
    }
    if ctx.anomaly_count_1m >= 40 {
        risk += 6.0;
    }
    if direction == Direction::Bullish {
        if short_bias < -10.0 && medium_bias > 12.0 {
            risk += 18.0;
        }
        if features.whale_exit {
            risk += 12.0;
        }
        if features.max_ask_ratio.to_f64().unwrap_or(0.0) > features.max_bid_ratio.to_f64().unwrap_or(0.0) + 6.0 {
            risk += 10.0;
        }
    } else if direction == Direction::Bearish {
        if short_bias > 10.0 && medium_bias < -12.0 {
            risk += 18.0;
        }
        if features.whale_entry {
            risk += 12.0;
        }
        if features.max_bid_ratio.to_f64().unwrap_or(0.0) > features.max_ask_ratio.to_f64().unwrap_or(0.0) + 6.0 {
            risk += 10.0;
        }
    } else {
        risk += 6.0;
    }
    if analysis.spoofing.detected {
        risk += 6.0;
    }
    clamp(risk, 0.0, 100.0)
}

fn calc_continuation_probability(
    ctx: &StrategyMarketContext,
    short_bias: f64,
    medium_bias: f64,
    coherence: f64,
    direction: Direction,
    false_breakout_probability: f64,
    reversal_risk: f64,
) -> f64 {
    let sign = match direction {
        Direction::Bullish => 1.0,
        Direction::Bearish => -1.0,
        Direction::Neutral => 0.0,
    };
    if sign == 0.0 {
        return 32.0;
    }
    let aligned_short = (short_bias * sign).max(0.0);
    let aligned_medium = (medium_bias * sign).max(0.0);
    let mut probability = 24.0 + aligned_short * 0.55 + aligned_medium * 0.70 + coherence * 0.28;
    if ctx.range_expansion_1m > 70.0 {
        probability += 6.0;
    }
    probability -= false_breakout_probability * 0.25;
    probability -= reversal_risk * 0.22;
    clamp(probability, 0.0, 100.0)
}

#[allow(clippy::too_many_arguments)]
fn classify_phase(
    direction: Direction,
    short_bias: f64,
    medium_bias: f64,
    pump_probability: f64,
    dump_probability: f64,
    continuation_probability: f64,
    false_breakout_probability: f64,
    reversal_risk: f64,
    ctx: &StrategyMarketContext,
) -> Phase {
    if false_breakout_probability >= 72.0 {
        return Phase::FalseBreakout;
    }
    if direction == Direction::Bullish {
        if reversal_risk >= 68.0 && pump_probability >= 58.0 {
            return Phase::Exhaustion;
        }
        if pump_probability >= 68.0
            && short_bias >= 20.0
            && (ctx.breakout_up || positive(ctx.current_ofi_zscore) > 1.5)
        {
            return Phase::Ignition;
        }
        if continuation_probability >= 65.0 && short_bias >= 12.0 && medium_bias >= 10.0 {
            return Phase::Continuation;
        }
        if pump_probability >= 56.0 && short_bias >= 10.0 && medium_bias < 10.0 {
            return Phase::Accumulation;
        }
    } else if direction == Direction::Bearish {
        if reversal_risk >= 68.0 && dump_probability >= 58.0 {
            return Phase::Exhaustion;
        }
        if dump_probability >= 68.0
            && short_bias <= -20.0
            && (ctx.breakout_down || negative(ctx.current_ofi_zscore) > 1.5)
        {
            return Phase::Ignition;
        }
        if continuation_probability >= 65.0 && short_bias <= -12.0 && medium_bias <= -10.0 {
            return Phase::Continuation;
        }
        if dump_probability >= 56.0 && short_bias <= -10.0 && medium_bias > -10.0 {
            return Phase::Distribution;
        }
    }
    Phase::Neutral
}

#[allow(clippy::too_many_arguments)]
fn calc_confidence(
    analysis: &ComprehensiveAnalysis,
    ctx: &StrategyMarketContext,
    direction: Direction,
    phase: Phase,
    coherence: f64,
    pump_probability: f64,
    dump_probability: f64,
    false_breakout_probability: f64,
    reversal_risk: f64,
) -> u8 {
    let directional_gap = (pump_probability - dump_probability).abs();
    let mut confidence = 42.0 + directional_gap * 0.38 + coherence * 0.20;
    if ctx.adaptive_ready {
        confidence += 8.0;
    }
    confidence += clamp(ctx.recent_trade_count_60s as f64 * 0.18, 0.0, 6.0);
    if matches!(phase, Phase::Ignition | Phase::Continuation) {
        confidence += 6.0;
    }
    if direction == Direction::Neutral {
        confidence -= 10.0;
    }
    if analysis.spoofing.detected {
        confidence -= 6.0;
    }
    confidence -= false_breakout_probability * 0.15;
    confidence -= reversal_risk * 0.12;
    confidence -= (ctx.anomaly_count_1m.min(120) as f64) * 0.03;
    confidence -= ctx.anomaly_max_severity as f64 * 0.04;
    clamp(confidence.round(), 0.0, 100.0) as u8
}

#[allow(clippy::too_many_arguments)]
fn build_reasons(
    windows: &[StrategyWindowSignal],
    analysis: &ComprehensiveAnalysis,
    ctx: &StrategyMarketContext,
    direction: Direction,
    phase: Phase,
    pump_probability: f64,
    dump_probability: f64,
    false_breakout_probability: f64,
    reversal_risk: f64,
    coherence: f64,
) -> Vec<String> {
    let mut reasons = Vec::<(f64, String)>::new();
    if let Some(window) = windows.iter().find(|item| item.label == "10s") {
        if window.trade_buy_ratio > 57.0 {
            reasons.push((window.trade_buy_ratio - 50.0, format!("10秒主动买入占比 {:.1}%", window.trade_buy_ratio)));
        }
        if window.trade_buy_ratio < 43.0 {
            reasons.push((50.0 - window.trade_buy_ratio, format!("10秒主动卖出占比 {:.1}%", 100.0 - window.trade_buy_ratio)));
        }
        if window.microprice_lead_bps.abs() > 0.6 {
            reasons.push((window.microprice_lead_bps.abs() * 2.0, format!("10秒 microprice 领先 {:.2}bps", window.microprice_lead_bps)));
        }
        let depth_edge = window.bid_depth_change_pct - window.ask_depth_change_pct;
        if depth_edge.abs() > 6.0 {
            let label = if depth_edge > 0.0 { "买盘回补强于卖盘" } else { "卖盘回补强于买盘" };
            reasons.push((depth_edge.abs(), format!("10秒 {} ({:+.1}%)", label, depth_edge)));
        }
    }
    if positive(ctx.current_ofi_zscore) > 1.2 {
        reasons.push((positive(ctx.current_ofi_zscore) * 6.0, format!("OFI 自适应 z-score {:.2}", ctx.current_ofi_zscore.unwrap_or_default())));
    }
    if negative(ctx.current_ofi_zscore) > 1.2 {
        reasons.push((negative(ctx.current_ofi_zscore) * 6.0, format!("OFI 自适应 z-score {:.2}", ctx.current_ofi_zscore.unwrap_or_default())));
    }
    if coherence >= 65.0 {
        reasons.push((coherence * 0.2, "1m/5m/15m 方向共振".to_string()));
    }
    if ctx.breakout_up && direction == Direction::Bullish {
        reasons.push((10.0, format!("突破上沿后承接 {:.0}%", ctx.breakout_acceptance)));
    }
    if ctx.breakout_down && direction == Direction::Bearish {
        reasons.push((10.0, format!("跌破下沿后承接 {:.0}%", ctx.breakout_acceptance)));
    }
    if false_breakout_probability >= 60.0 {
        reasons.push((false_breakout_probability * 0.4, "突破后成交和盘口不同步，存在假动作风险".to_string()));
    }
    if reversal_risk >= 60.0 {
        reasons.push((reversal_risk * 0.35, "短周期出现反向流入，延续性开始衰减".to_string()));
    }
    if analysis.spoofing.detected {
        reasons.push((8.0, "盘口存在疑似 spoofing/layering".to_string()));
    }
    if analysis.whale.detected {
        let whale_score = analysis.whale.dominance_ratio.to_f64().unwrap_or(0.0);
        if whale_score > 0.0 {
            let text = if direction == Direction::Bullish {
                format!("主导买单占比 {:.1}%", whale_score)
            } else if direction == Direction::Bearish {
                format!("主导卖单占比 {:.1}%", whale_score)
            } else {
                format!("主导挂单占比 {:.1}%", whale_score)
            };
            reasons.push((whale_score * 0.3, text));
        }
    }
    reasons.sort_by(|a, b| b.0.partial_cmp(&a.0).unwrap_or(std::cmp::Ordering::Equal));
    let mut ordered = reasons.into_iter().map(|(_, text)| text).collect::<Vec<_>>();
    if ordered.is_empty() {
        ordered.push(match phase {
            Phase::Accumulation => format!("拉盘预期 {:.0}%，等待更强点火确认", pump_probability),
            Phase::Distribution => format!("砸盘预期 {:.0}%，等待更强下破确认", dump_probability),
            Phase::Ignition => "盘口和成交开始同步点火".to_string(),
            Phase::Continuation => "方向延续条件暂时成立".to_string(),
            Phase::Exhaustion => "方向仍在延续，但边际动能减弱".to_string(),
            Phase::FalseBreakout => "突破后承接不足，优先防守".to_string(),
            Phase::Neutral => "当前未形成高质量单边结构".to_string(),
        });
    }
    ordered
}

fn invalidation_text(direction: Direction, phase: Phase) -> String {
    match (direction, phase) {
        (Direction::Bullish, Phase::Accumulation) => "卖盘重新回补、3s/10s 主动买入跌破 52%、spread 扩大".to_string(),
        (Direction::Bullish, Phase::Ignition) => "ask 深度回补、OFI 回落至中性、短周期成交转空".to_string(),
        (Direction::Bullish, Phase::Continuation) => "30s/60s 偏多结构失效，或出现连续大额反向成交".to_string(),
        (Direction::Bullish, Phase::Exhaustion) => "继续拉升但成交跟不上时，优先防回吐".to_string(),
        (Direction::Bearish, Phase::Distribution) => "买盘重新承接、3s/10s 主动卖出跌破 52%、spread 扩大".to_string(),
        (Direction::Bearish, Phase::Ignition) => "bid 深度回补、OFI 回到中性、短周期成交转多".to_string(),
        (Direction::Bearish, Phase::Continuation) => "30s/60s 偏空结构失效，或出现连续大额反向扫单".to_string(),
        (Direction::Bearish, Phase::Exhaustion) => "继续下压但主动卖出衰减时，谨防反抽".to_string(),
        (_, Phase::FalseBreakout) => "突破方向无法获得成交确认时，不追单".to_string(),
        _ => "等待多窗口方向再次收敛后再动作".to_string(),
    }
}

fn phase_label(phase: Phase) -> &'static str {
    match phase {
        Phase::Neutral => "平衡观察",
        Phase::Accumulation => "吸筹期",
        Phase::Distribution => "派发期",
        Phase::Ignition => "点火期",
        Phase::Continuation => "延续期",
        Phase::Exhaustion => "衰减期",
        Phase::FalseBreakout => "假突破风险",
    }
}

fn direction_label(direction: Direction) -> &'static str {
    match direction {
        Direction::Bullish => "偏多",
        Direction::Bearish => "偏空",
        Direction::Neutral => "中性",
    }
}

fn regime_label(regime: Regime) -> &'static str {
    match regime {
        Regime::Balanced => "平衡整理",
        Regime::Expansion => "放量扩张",
        Regime::Squeeze => "收敛待变",
        Regime::Volatile => "高波动",
        Regime::Illiquid => "流动性偏弱",
    }
}

fn signed_magnitude(value: f64, scale: f64, max: f64) -> f64 {
    if value.abs() < f64::EPSILON {
        return 0.0;
    }
    let magnitude = (value.abs() / scale).ln_1p() * 8.0;
    clamp(value.signum() * magnitude, -max, max)
}

fn positive(value: Option<f64>) -> f64 {
    value.unwrap_or_default().max(0.0)
}

fn negative(value: Option<f64>) -> f64 {
    (-value.unwrap_or_default()).max(0.0)
}

fn weighted_average(a: f64, b: f64, wa: f64, wb: f64) -> f64 {
    let total = wa + wb;
    if total <= 0.0 {
        0.0
    } else {
        (a * wa + b * wb) / total
    }
}

fn window_weight(window_secs: u64) -> f64 {
    match window_secs {
        3 => 0.34,
        10 => 0.26,
        30 => 0.24,
        60 => 0.16,
        _ => 0.1,
    }
}

fn clamp(value: f64, min: f64, max: f64) -> f64 {
    value.max(min).min(max)
}
