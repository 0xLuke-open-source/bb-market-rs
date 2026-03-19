// analysis/pump_detector.rs — 改进版
//
// ═══════════════════════════════════════════════
// 主要改进：
//
// 1. OFI 阈值适配新版增量 OFI
//    原版 OFI 是 bid_depth - ask_depth（量级 10万+）
//    新版 OFI 是增量变化（量级 小得多，通常 -5000 ~ +5000）
//    → 调整阈值：100000→30000，50000→10000
//
// 2. 新增 pump_score / dump_score 字段利用
//    features 中已有积分制评分，直接用来提权
//
// 3. 新增 ofi_direction 字段区分增量 OFI 方向
//    正 OFI = 买方主动，负 OFI = 卖方主动
//
// 4. 信号强度计算改用加权组合，避免单指标爆分
// ═══════════════════════════════════════════════

use rust_decimal::Decimal;
use rust_decimal_macros::dec;
use std::fs::OpenOptions;
use std::io::Write;
use chrono;
use rust_decimal::prelude::ToPrimitive;
use crate::store::l2_book::OrderBookFeatures;

#[derive(Debug, Clone)]
pub struct PumpSignal {
    pub timestamp:          chrono::DateTime<chrono::Local>,
    pub symbol:             String,
    pub strength:           u8,
    pub pump_probability:   u8,
    pub accumulation_score: u8,
    pub ofi:                f64,   // 增量 OFI（新版）
    pub ofi_raw:            f64,   // 深度差 OFI（原版）
    pub obi:                f64,
    pub pump_score:         u8,    // 积分制拉盘评分
    pub dump_score:         u8,
    pub price:              f64,
    pub target:             f64,
    pub bid_volume_change:  f64,
    pub max_bid_ratio:      f64,
    pub slope_bid:          f64,
    pub reasons:            Vec<String>,
}

pub struct PumpDetector {
    output_file: String,
    min_strength: u8,
}

impl PumpDetector {
    pub fn new(output_file: &str) -> Self {
        let output_file = format!("PumpDetector/{}", output_file);
        std::fs::create_dir_all("PumpDetector").unwrap_or_default();

        let _ = OpenOptions::new()
            .create(true).write(true).truncate(true)
            .open(&output_file)
            .and_then(|mut f| {
                writeln!(f, "{}", "=".repeat(160))?;
                writeln!(f, "🚀 拉盘信号实时监测报告 - {}",
                         chrono::Local::now().format("%Y-%m-%d %H:%M:%S"))?;
                writeln!(f, "{}", "=".repeat(160))?;
                writeln!(f,
                         "{:<8} | {:<10} | {:<6} | {:<6} | {:<6} | {:<6} | {:<8} | {:<8} | {:<10} | {:<12} | {:<10} | {:<10} | {:<30}",
                         "时间", "币种", "强度", "概率", "P分", "D分", "OFI增量", "OBI%",
                         "价格", "目标", "买单变%", "大单比%", "信号原因"
                )?;
                writeln!(f, "{}", "-".repeat(160))
            });

        Self { output_file, min_strength: 30 }
    }

    pub fn with_min_strength(mut self, min: u8) -> Self {
        self.min_strength = min;
        self
    }

    pub fn analyze_symbol(
        &self,
        symbol:             &str,
        features:           &OrderBookFeatures,
        pump_probability:   u8,
        accumulation_score: u8,
        target_price:       Decimal,
    ) -> Option<PumpSignal> {
        let mut strength: i32 = 0;
        let mut reasons = Vec::new();

        let ofi_val      = features.ofi.to_f64().unwrap_or(0.0);
        let ofi_raw_val  = features.ofi_raw.to_f64().unwrap_or(0.0);
        let obi_val      = features.obi.to_f64().unwrap_or(0.0);
        let bid_change   = features.bid_volume_change.to_f64().unwrap_or(0.0);
        let max_bid      = features.max_bid_ratio.to_f64().unwrap_or(0.0);
        let slope_bid    = features.slope_bid.to_f64().unwrap_or(0.0);

        let (best_bid, best_ask) = match (features.weighted_bid_price, features.weighted_ask_price) {
            (b, a) if b > Decimal::ZERO && a > Decimal::ZERO => {
                (b.to_f64().unwrap_or(0.0), a.to_f64().unwrap_or(0.0))
            }
            _ => (0.0, 0.0),
        };
        let current_price = (best_bid + best_ask) / 2.0;

        // ── 1. 增量 OFI（主力信号，新版）────────────────────────
        // 阈值相比原版大幅降低（原 50000/100000 → 3000/10000）
        if features.ofi > dec!(10000) {
            strength += 25;
            reasons.push(format!("OFI↑={:.0}", ofi_val));
        } else if features.ofi > dec!(3000) {
            strength += 15;
            reasons.push(format!("ofi↑={:.0}", ofi_val));
        }

        // ── 2. OBI（全局深度不平衡）──────────────────────────────
        if features.obi > dec!(20) {
            strength += 20;
            reasons.push(format!("OBI={:.1}%", obi_val));
        } else if features.obi > dec!(10) {
            strength += 10;
            reasons.push(format!("obi={:.1}%", obi_val));
        }

        // ── 3. 积分制拉盘评分（直接复用 l2_book 的计算结果）───
        if features.pump_score >= 80 {
            strength += 20;
            reasons.push(format!("SCORE={}", features.pump_score));
        } else if features.pump_score >= 60 {
            strength += 12;
            reasons.push(format!("score={}", features.pump_score));
        }

        // ── 4. 原有标志信号（兼容原逻辑）──────────────────────
        if features.pump_signal {
            strength += 10;
            reasons.push("🚀PUMP".into());
        }
        if features.bid_eating {
            strength += 10;
            reasons.push("🍽️EAT".into());
        }
        if features.whale_entry {
            strength += 15;
            reasons.push("🐋WHALE".into());
        }

        // ── 5. 斜率（阈值降低）────────────────────────────────
        if features.slope_bid > dec!(500000) {
            strength += 10;
            reasons.push("SLOPE↑".into());
        } else if features.slope_bid > dec!(200000) {
            strength += 5;
        }

        // ── 6. 大单占比 ────────────────────────────────────────
        if features.max_bid_ratio > dec!(25) {
            strength += 10;
            reasons.push(format!("BIG{:.0}%", max_bid));
        } else if features.max_bid_ratio > dec!(15) {
            strength += 5;
        }

        // ── 7. 买单量变化 ──────────────────────────────────────
        if features.bid_volume_change > dec!(20) {
            strength += 10;
            reasons.push(format!("VOL+{:.0}%", bid_change));
        } else if features.bid_volume_change > dec!(8) {
            strength += 5;
            reasons.push(format!("vol+{:.0}%", bid_change));
        }

        // ── 8. 价差收窄（流动性好） ────────────────────────────
        if features.spread_bps < dec!(10) {
            strength += 5;
            reasons.push("TIGHT".into());
        }

        let strength = strength.min(100) as u8;

        if strength >= self.min_strength {
            Some(PumpSignal {
                timestamp:          chrono::Local::now(),
                symbol:             symbol.to_string(),
                strength,
                pump_probability,
                accumulation_score,
                ofi:                ofi_val,
                ofi_raw:            ofi_raw_val,
                obi:                obi_val,
                pump_score:         features.pump_score,
                dump_score:         features.dump_score,
                price:              current_price,
                target:             target_price.to_f64().unwrap_or(0.0),
                bid_volume_change:  bid_change,
                max_bid_ratio:      max_bid,
                slope_bid,
                reasons,
            })
        } else {
            None
        }
    }

    pub fn write_pump_signal(&self, s: &PumpSignal) -> std::io::Result<()> {
        let mut file = OpenOptions::new()
            .create(true).append(true)
            .open(&self.output_file)?;

        let emoji = if s.strength >= 80 { "🔥🔥" }
        else if s.strength >= 60 { "🚀🚀" }
        else if s.strength >= 40 { "📈📈" }
        else { "⬆️" };

        writeln!(file,
                 "{} | {:<10} | {:<2}%{} | {:<3}% | {:<3} | {:<3} | {:<8.0} | {:<+6.1}% | {:<10.4} | {:<10.4} | {:<+6.1}% | {:<6.1}% | {}",
                 s.timestamp.format("%H:%M:%S"),
                 s.symbol,
                 s.strength, emoji,
                 s.pump_probability,
                 s.pump_score, s.dump_score,
                 s.ofi,
                 s.obi,
                 s.price, s.target,
                 s.bid_volume_change, s.max_bid_ratio,
                 s.reasons.join(" ")
        )?;
        file.flush()
    }

    pub fn write_top_signals(&self, signals: &mut Vec<PumpSignal>) -> std::io::Result<()> {
        if signals.is_empty() { return Ok(()); }
        signals.sort_by(|a, b| b.strength.cmp(&a.strength));

        let mut file = OpenOptions::new()
            .create(true).append(true)
            .open(&self.output_file)?;

        writeln!(file)?;
        writeln!(file, "{}", "=".repeat(160))?;
        writeln!(file, "📊 当前最强拉盘信号 TOP {} - {}",
                 signals.len(), chrono::Local::now().format("%H:%M:%S"))?;
        writeln!(file, "{}", "-".repeat(160))?;

        for (rank, s) in signals.iter().enumerate() {
            let medal = if rank == 0 { "🥇" } else if rank == 1 { "🥈" } else if rank == 2 { "🥉" } else { "  " };
            writeln!(file,
                     "{} {} | {:<10} | 强度{:>2}% | 概率{:>2}% | P:{} D:{} | OFI={:<8.0} | OBI={:+.1}% | 目标:{:.4} | {}",
                     medal,
                     s.timestamp.format("%H:%M:%S"),
                     s.symbol, s.strength, s.pump_probability,
                     s.pump_score, s.dump_score,
                     s.ofi, s.obi, s.target,
                     s.reasons.join(" ")
            )?;
        }
        writeln!(file, "{}", "=".repeat(160))?;
        file.flush()
    }

    pub fn print_top_signals(&self, signals: &[PumpSignal], top_n: usize) {
        if signals.is_empty() { return; }
        println!("\n{}", "🔥".repeat(10));
        println!("🔥 当前最强拉盘信号 TOP {}", top_n.min(signals.len()));
        println!("{}", "🔥".repeat(10));

        for (i, s) in signals.iter().take(top_n).enumerate() {
            let medal = if i == 0 { "🥇" } else if i == 1 { "🥈" } else if i == 2 { "🥉" } else { "  " };
            let emoji = if s.strength >= 80 { "🔥" } else if s.strength >= 60 { "🚀" } else { "📈" };
            println!(
                "{} {} {:<10} 强度:{:>2}%{} 概率:{:>2}%  OFI(增量):{:<8.0} OBI:{:+.1}% P:{} 目标:{:.4}",
                medal,
                s.timestamp.format("%H:%M:%S"),
                s.symbol, s.strength, emoji, s.pump_probability,
                s.ofi, s.obi, s.pump_score, s.target
            );
        }
        println!("{}", "🔥".repeat(10));
    }
}