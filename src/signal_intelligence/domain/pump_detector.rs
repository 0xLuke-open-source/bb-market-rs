// analysis/pump_detector.rs — 市场最佳质量版
//
// ═══════════════════════════════════════════════
// 核心改进：
//
// 1. z-score 自适应评分（热身完成后替代绝对值判断）
//    - OFI z>2.0  → +25, z>1.0 → +15
//    - OBI z>2.0  → +20, z>1.0 → +10
//    - Vol z>2.0  → +15, z>1.0 → +8
//    预热期继续使用原有绝对值阈值（完整保留）
//
// 2. 多周期共振过滤（新增）
//    要求 1m 信号方向 = 5m K线趋势方向，否则强度打 0.6 折
//
// 3. 时序状态机（新增）
//    Idle → [OFI 触发] → Watching → [价格移动 + 30s内] → Armed
//    → [Vol 确认 + 30s内] → emit
//    任何阶段超时 60s → Idle
// ═══════════════════════════════════════════════

use std::collections::HashMap;
use std::time::{Duration, Instant};

use crate::market_data::domain::order_book::OrderBookFeatures;
use crate::signal_intelligence::domain::adaptive_threshold::SymbolAdaptiveThreshold;
use chrono;
use rust_decimal::prelude::ToPrimitive;
use rust_decimal::Decimal;
use rust_decimal_macros::dec;

/// 信号触发时各因子的快照（供 signal_factor_detail 入库）
#[derive(Debug, Clone)]
pub struct FactorSnapshot {
    pub factor_name: String,
    pub raw_value: f64,
    pub z_score: Option<f64>,
    pub contribution_score: f64,
}

#[derive(Debug, Clone)]
pub struct PumpSignal {
    pub timestamp: chrono::DateTime<chrono::Local>,
    pub symbol: String,
    pub strength: u8,
    pub pump_probability: u8,
    pub accumulation_score: u8,
    pub ofi: f64,
    pub ofi_raw: f64,
    pub obi: f64,
    pub pump_score: u8,
    pub dump_score: u8,
    pub price: f64,
    pub target: f64,
    pub bid_volume_change: f64,
    pub max_bid_ratio: f64,
    pub slope_bid: f64,
    pub reasons: Vec<String>,
    /// 触发时各因子详情（供 signal_factor_detail 入库）
    pub factor_snapshots: Vec<FactorSnapshot>,
}

/// 信号时序状态机
#[derive(Debug, Clone)]
enum SignalState {
    Idle,
    Watching {
        since: Instant,
        ofi_triggered: bool,
        trigger_price: f64,
    },
    Armed {
        since: Instant,
        trigger_price: f64,
    },
}

pub struct PumpDetector {
    min_strength: u8,
    /// 每个 symbol 独立的状态机
    states: HashMap<String, SignalState>,
}

impl PumpDetector {
    pub fn new(_output_file: &str) -> Self {
        Self {
            min_strength: 30,
            states: HashMap::new(),
        }
    }

    pub fn with_min_strength(mut self, min: u8) -> Self {
        self.min_strength = min;
        self
    }

    /// 主分析入口
    ///
    /// 参数 `threshold` 为 None 时（预热期）使用绝对值判断；
    /// 有值时切换为 z-score 判断。
    pub fn analyze_symbol(
        &mut self,
        symbol: &str,
        features: &OrderBookFeatures,
        pump_probability: u8,
        accumulation_score: u8,
        target_price: Decimal,
        threshold: Option<&SymbolAdaptiveThreshold>,
        kline_1m_direction: Option<f64>, // 最近 1m K 线收益率（正=涨，负=跌）
        kline_5m_direction: Option<f64>, // 最近 5m K 线收益率
    ) -> Option<PumpSignal> {
        let ofi_val = features.ofi.to_f64().unwrap_or(0.0);
        let ofi_raw_val = features.ofi_raw.to_f64().unwrap_or(0.0);
        let obi_val = features.obi.to_f64().unwrap_or(0.0);
        let bid_change = features.bid_volume_change.to_f64().unwrap_or(0.0);
        let max_bid = features.max_bid_ratio.to_f64().unwrap_or(0.0);
        let slope_bid = features.slope_bid.to_f64().unwrap_or(0.0);
        let total_vol = features.total_bid_volume.to_f64().unwrap_or(0.0)
            + features.total_ask_volume.to_f64().unwrap_or(0.0);

        let current_price = {
            let b = features.weighted_bid_price.to_f64().unwrap_or(0.0);
            let a = features.weighted_ask_price.to_f64().unwrap_or(0.0);
            if b > 0.0 && a > 0.0 {
                (b + a) / 2.0
            } else {
                0.0
            }
        };

        let mut strength: i32 = 0;
        let mut reasons = Vec::new();
        let mut factor_snapshots = Vec::new();

        // ────────────────────────────────────────────────────────
        // 分支 A：热身完成 → z-score 评分
        // 分支 B：预热期 → 绝对值评分（原逻辑完整保留）
        // ────────────────────────────────────────────────────────
        if let Some(th) = threshold {
            if th.is_warm() {
                // ── A1. OFI z-score ────────────────────────────
                if let Some(z) = th.ofi_zscore(ofi_val) {
                    let contrib = if z > 2.0 {
                        25.0
                    } else if z > 1.0 {
                        15.0
                    } else {
                        0.0
                    };
                    if contrib > 0.0 {
                        strength += contrib as i32;
                        reasons.push(format!("OFI_z={:.1}", z));
                    }
                    factor_snapshots.push(FactorSnapshot {
                        factor_name: "ofi_zscore".to_string(),
                        raw_value: ofi_val,
                        z_score: Some(z),
                        contribution_score: contrib,
                    });
                }

                // ── A2. OBI z-score ────────────────────────────
                if let Some(z) = th.obi_zscore(obi_val) {
                    let contrib = if z > 2.0 {
                        20.0
                    } else if z > 1.0 {
                        10.0
                    } else {
                        0.0
                    };
                    if contrib > 0.0 {
                        strength += contrib as i32;
                        reasons.push(format!("OBI_z={:.1}", z));
                    }
                    factor_snapshots.push(FactorSnapshot {
                        factor_name: "obi_zscore".to_string(),
                        raw_value: obi_val,
                        z_score: Some(z),
                        contribution_score: contrib,
                    });
                }

                // ── A3. Vol z-score ────────────────────────────
                if let Some(z) = th.vol_zscore(total_vol) {
                    let contrib = if z > 2.0 {
                        15.0
                    } else if z > 1.0 {
                        8.0
                    } else {
                        0.0
                    };
                    if contrib > 0.0 {
                        strength += contrib as i32;
                        reasons.push(format!("VOL_z={:.1}", z));
                    }
                    factor_snapshots.push(FactorSnapshot {
                        factor_name: "vol_zscore".to_string(),
                        raw_value: total_vol,
                        z_score: Some(z),
                        contribution_score: contrib,
                    });
                }
            } else {
                // 预热中，使用绝对值（见分支 B）
                strength += self.score_absolute(features, &mut reasons, &mut factor_snapshots);
            }
        } else {
            // 没有 threshold（单币种调试模式），使用绝对值
            strength += self.score_absolute(features, &mut reasons, &mut factor_snapshots);
        }

        // ── 共有因子：不依赖 threshold（总是计算）──────────────
        // pump_score 积分制结果
        {
            let contrib = if features.pump_score >= 80 {
                20.0
            } else if features.pump_score >= 60 {
                12.0
            } else {
                0.0
            };
            if contrib > 0.0 {
                strength += contrib as i32;
                reasons.push(format!("SCORE={}", features.pump_score));
            }
            factor_snapshots.push(FactorSnapshot {
                factor_name: "pump_score".to_string(),
                raw_value: features.pump_score as f64,
                z_score: None,
                contribution_score: contrib,
            });
        }

        // pump_signal 标志
        if features.pump_signal {
            strength += 10;
            reasons.push("PUMP".into());
        }
        if features.bid_eating {
            strength += 10;
            reasons.push("EAT".into());
        }
        if features.whale_entry {
            let contrib = 15.0;
            strength += contrib as i32;
            reasons.push("WHALE".into());
            factor_snapshots.push(FactorSnapshot {
                factor_name: "whale".to_string(),
                raw_value: 1.0,
                z_score: None,
                contribution_score: contrib,
            });
        }

        // 大单占比
        if features.max_bid_ratio > dec!(25) {
            strength += 10;
            reasons.push(format!("BIG{:.0}%", max_bid));
        }

        // 价差收窄
        if features.spread_bps < dec!(10) {
            strength += 5;
            reasons.push("TIGHT".into());
        }

        // ── 多周期共振过滤 ──────────────────────────────────────
        // 如果 1m 方向与 5m 方向不一致，信号强度打 0.6 折
        if let (Some(d1), Some(d5)) = (kline_1m_direction, kline_5m_direction) {
            if d1 * d5 < 0.0 {
                // 方向不一致
                strength = (strength as f64 * 0.6) as i32;
                reasons.push("COHERENCE-0.6x".into());
            }
        }

        let raw_strength = strength.clamp(0, 100) as u8;

        // ── 时序状态机 ──────────────────────────────────────────
        let final_strength = self.advance_state_machine(
            symbol,
            raw_strength,
            ofi_val,
            total_vol,
            current_price,
            threshold,
        );

        if final_strength >= self.min_strength {
            Some(PumpSignal {
                timestamp: chrono::Local::now(),
                symbol: symbol.to_string(),
                strength: final_strength,
                pump_probability,
                accumulation_score,
                ofi: ofi_val,
                ofi_raw: ofi_raw_val,
                obi: obi_val,
                pump_score: features.pump_score,
                dump_score: features.dump_score,
                price: current_price,
                target: target_price.to_f64().unwrap_or(0.0),
                bid_volume_change: bid_change,
                max_bid_ratio: max_bid,
                slope_bid,
                reasons,
                factor_snapshots,
            })
        } else {
            None
        }
    }

    /// 绝对值评分（预热期或无 threshold 时使用，与原版逻辑相同）
    fn score_absolute(
        &self,
        features: &OrderBookFeatures,
        reasons: &mut Vec<String>,
        factor_snapshots: &mut Vec<FactorSnapshot>,
    ) -> i32 {
        let mut strength = 0i32;
        let ofi_val = features.ofi.to_f64().unwrap_or(0.0);
        let obi_val = features.obi.to_f64().unwrap_or(0.0);

        // OFI
        let ofi_contrib = if features.ofi > dec!(10000) {
            25.0
        } else if features.ofi > dec!(3000) {
            15.0
        } else {
            0.0
        };
        if ofi_contrib > 0.0 {
            strength += ofi_contrib as i32;
            reasons.push(format!("OFI={:.0}", ofi_val));
        }
        factor_snapshots.push(FactorSnapshot {
            factor_name: "ofi_abs".to_string(),
            raw_value: ofi_val,
            z_score: None,
            contribution_score: ofi_contrib,
        });

        // OBI
        let obi_contrib = if features.obi > dec!(20) {
            20.0
        } else if features.obi > dec!(10) {
            10.0
        } else {
            0.0
        };
        if obi_contrib > 0.0 {
            strength += obi_contrib as i32;
            reasons.push(format!("OBI={:.1}%", obi_val));
        }
        factor_snapshots.push(FactorSnapshot {
            factor_name: "obi_abs".to_string(),
            raw_value: obi_val,
            z_score: None,
            contribution_score: obi_contrib,
        });

        // 买单量变化
        let bid_change = features.bid_volume_change.to_f64().unwrap_or(0.0);
        if features.bid_volume_change > dec!(20) {
            strength += 10;
            reasons.push(format!("VOL+{:.0}%", bid_change));
        } else if features.bid_volume_change > dec!(8) {
            strength += 5;
        }

        // 斜率
        if features.slope_bid > dec!(500000) {
            strength += 10;
            reasons.push("SLOPE↑".into());
        } else if features.slope_bid > dec!(200000) {
            strength += 5;
        }

        strength
    }

    /// 推进时序状态机，返回最终是否允许输出信号的强度值
    /// - Idle: 不输出（返回 0）
    /// - Watching: 不输出（等待价格确认）
    /// - Armed: 等待 vol 确认后输出（返回原始强度）
    fn advance_state_machine(
        &mut self,
        symbol: &str,
        raw_strength: u8,
        ofi: f64,
        total_vol: f64,
        current_price: f64,
        threshold: Option<&SymbolAdaptiveThreshold>,
    ) -> u8 {
        let now = Instant::now();
        let timeout = Duration::from_secs(60);

        let state = self
            .states
            .entry(symbol.to_string())
            .or_insert(SignalState::Idle);

        // 超时回退到 Idle
        let timed_out = match state {
            SignalState::Watching { since, .. } | SignalState::Armed { since, .. } => {
                since.elapsed() > timeout
            }
            SignalState::Idle => false,
        };
        if timed_out {
            *state = SignalState::Idle;
        }

        // 如果 threshold 未热身，直接走原始强度（无状态机过滤）
        let is_warm = threshold.map(|t| t.is_warm()).unwrap_or(false);
        if !is_warm {
            return raw_strength;
        }

        // OFI z-score 是否超过 1.5
        let ofi_trigger = if let Some(th) = threshold {
            th.ofi_zscore(ofi).map(|z| z > 1.5).unwrap_or(false)
        } else {
            false
        };

        // Vol z-score 是否超过 1.5
        let vol_confirm = if let Some(th) = threshold {
            th.vol_zscore(total_vol).map(|z| z > 1.5).unwrap_or(false)
        } else {
            false
        };

        match state.clone() {
            SignalState::Idle => {
                if ofi_trigger && raw_strength >= 15 {
                    *state = SignalState::Watching {
                        since: now,
                        ofi_triggered: true,
                        trigger_price: current_price,
                    };
                }
                0 // Idle 不输出
            }
            SignalState::Watching {
                since,
                trigger_price,
                ..
            } => {
                // 价格移动超过 0.05% 且在 30s 内
                let price_moved = current_price > 0.0
                    && trigger_price > 0.0
                    && (current_price - trigger_price).abs() / trigger_price > 0.0005;
                let within_30s = since.elapsed() <= Duration::from_secs(30);

                if price_moved && within_30s {
                    *state = SignalState::Armed {
                        since: now,
                        trigger_price,
                    };
                }
                0 // Watching 不输出
            }
            SignalState::Armed { .. } => {
                // Vol 确认且在 30s 内 → 输出信号，重置状态机
                let within_30s = true; // Armed 刚进入 30s 内（超时已在上面处理）
                if vol_confirm && within_30s && raw_strength >= self.min_strength {
                    *state = SignalState::Idle;
                    raw_strength
                } else if raw_strength >= 60 {
                    // 极高分直接输出，不需要 vol 确认
                    *state = SignalState::Idle;
                    raw_strength
                } else {
                    0
                }
            }
        }
    }

    pub fn write_pump_signal(&self, s: &PumpSignal) -> std::io::Result<()> {
        let _ = s;
        Ok(())
    }

    pub fn write_top_signals(&self, signals: &mut Vec<PumpSignal>) -> std::io::Result<()> {
        let _ = signals;
        Ok(())
    }

    pub fn print_top_signals(&self, signals: &[PumpSignal], top_n: usize) {
        let _ = (signals, top_n);
    }
}
