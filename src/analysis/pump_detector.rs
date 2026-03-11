// src/analysis/pump_detector.rs
// 独立拉盘检测模块 - 不修改原有逻辑

use rust_decimal::Decimal;
use rust_decimal_macros::dec;
use std::collections::HashMap;
use std::fs::OpenOptions;
use std::io::Write;
use chrono;
use rust_decimal::prelude::ToPrimitive;
use crate::store::l2_book::OrderBookFeatures;

// 拉盘信号结构
#[derive(Debug, Clone)]
pub struct PumpSignal {
    pub timestamp: chrono::DateTime<chrono::Local>,
    pub symbol: String,
    pub strength: u8,              // 信号强度 0-100
    pub pump_probability: u8,       // 拉升概率
    pub accumulation_score: u8,     // 吸筹评分
    pub ofi: f64,                   // 订单流失衡
    pub obi: f64,                    // 订单簿失衡
    pub price: f64,                  // 当前价格
    pub target: f64,                 // 目标位
    pub bid_volume_change: f64,       // 买单量变化
    pub max_bid_ratio: f64,           // 大单占比
    pub slope_bid: f64,               // 买单斜率
    pub reasons: Vec<String>,         // 信号原因
}

// 拉盘检测器
pub struct PumpDetector {
    output_file: String,
    min_strength: u8,  // 最小强度阈值
}

impl PumpDetector {
    pub fn new(output_file: &str) -> Self {
        let output_file = format!("PumpDetector/{}", output_file);

        // 确保目录存在
        std::fs::create_dir_all("PumpDetector").unwrap_or_default();

        // 初始化文件，写入表头
        let _ = OpenOptions::new()
            .create(true)
            .write(true)
            .truncate(true)
            .open(&output_file)
            .and_then(|mut file| {
                writeln!(file, "{}", "=".repeat(150))?;
                writeln!(file, "🚀 拉盘信号实时监测报告 - {}",
                         chrono::Local::now().format("%Y-%m-%d %H:%M:%S"))?;
                writeln!(file, "{}", "=".repeat(150))?;
                writeln!(
                    file,
                    "{:<8} | {:<10} | {:<6} | {:<6} | {:<12} | {:<8} | {:<10} | {:<12} | {:<10} | {:<10} | {:<30}",
                    "时间", "币种", "强度", "概率", "OFI", "OBI%", "价格", "目标", "买单变化%", "大单比%", "信号原因"
                )?;
                writeln!(file, "{}", "-".repeat(150))
            });

        Self {
            output_file,
            min_strength: 30,  // 默认30分以上才记录
        }
    }

    // 打印当前最强的几个信号到控制台
    pub fn print_top_signals(&self, signals: &[PumpSignal], top_n: usize) {
        if signals.is_empty() {
            return;
        }

        println!("\n{}", "🔥".repeat(10));
        println!("🔥 当前最强拉盘信号 TOP {}", top_n.min(signals.len()));
        println!("{}", "🔥".repeat(10));

        for (i, signal) in signals.iter().take(top_n).enumerate() {
            let medal = if i == 0 { "🥇" } else if i == 1 { "🥈" } else if i == 2 { "🥉" } else { "  " };

            let strength_emoji = if signal.strength >= 80 {
                "🔥"
            } else if signal.strength >= 60 {
                "🚀"
            } else {
                "📈"
            };

            println!(
                "{} {} {:<10} 强度:{:>2}%{} 概率:{:>2}%  OFI:{:<8.0} OBI:{:+.1}% 目标:{:.4}",
                medal,
                signal.timestamp.format("%H:%M:%S"),
                signal.symbol,
                signal.strength,
                strength_emoji,
                signal.pump_probability,
                signal.ofi,
                signal.obi,
                signal.target
            );
        }
        println!("{}", "🔥".repeat(10));
    }

    // 设置最小强度阈值
    pub fn with_min_strength(mut self, min_strength: u8) -> Self {
        self.min_strength = min_strength;
        self
    }

    // 分析单个币种的拉盘信号
    pub fn analyze_symbol(
        &self,
        symbol: &str,
        features: &OrderBookFeatures,
        pump_probability: u8,
        accumulation_score: u8,
        target_price: Decimal,
    ) -> Option<PumpSignal> {
        let mut strength = 0;
        let mut reasons = Vec::new();

        // 获取数值
        let ofi_val = features.ofi.to_f64().unwrap_or(0.0);
        let obi_val = features.obi.to_f64().unwrap_or(0.0);
        let bid_change = features.bid_volume_change.to_f64().unwrap_or(0.0);
        let max_bid = features.max_bid_ratio.to_f64().unwrap_or(0.0);
        let slope_bid = features.slope_bid.to_f64().unwrap_or(0.0);

        let (best_bid, best_ask) = match (features.weighted_bid_price, features.weighted_ask_price) {
            (bid, ask) if bid > Decimal::ZERO && ask > Decimal::ZERO => {
                (bid.to_f64().unwrap_or(0.0), ask.to_f64().unwrap_or(0.0))
            },
            _ => (0.0, 0.0),
        };
        let current_price = (best_bid + best_ask) / 2.0;

        // 1. OFI 信号
        if features.ofi > dec!(100000) {
            strength += 25;
            reasons.push(format!("OFI={:.0}", ofi_val));
        } else if features.ofi > dec!(50000) {
            strength += 15;
            reasons.push(format!("OFI={:.0}", ofi_val));
        }

        // 2. OBI 信号
        if features.obi > dec!(30) {
            strength += 20;
            reasons.push(format!("OBI={:.1}%", obi_val));
        } else if features.obi > dec!(20) {
            strength += 12;
            reasons.push(format!("OBI={:.1}%", obi_val));
        }

        // 3. 拉盘信号标志
        if features.pump_signal {
            strength += 15;
            reasons.push("🚀PUMP".to_string());
        }

        // 4. 吃筹信号
        if features.bid_eating {
            strength += 15;
            reasons.push("🍽️EAT".to_string());
        }

        // 5. 鲸鱼进场
        if features.whale_entry {
            strength += 20;
            reasons.push("🐋WHALE".to_string());
        }

        // 6. 买单斜率
        if features.slope_bid > dec!(1000000) {
            strength += 10;
            reasons.push("SLOPE↑".to_string());
        } else if features.slope_bid > dec!(500000) {
            strength += 5;
            reasons.push("slope↑".to_string());
        }

        // 7. 大单占比
        if features.max_bid_ratio > dec!(25) {
            strength += 10;
            reasons.push(format!("BIG{}%", max_bid as i32));
        } else if features.max_bid_ratio > dec!(15) {
            strength += 5;
            reasons.push(format!("big{}%", max_bid as i32));
        }

        // 8. 买单量变化
        if features.bid_volume_change > dec!(20) {
            strength += 10;
            reasons.push(format!("VOL+{}%", bid_change as i32));
        } else if features.bid_volume_change > dec!(10) {
            strength += 5;
            reasons.push(format!("vol+{}%", bid_change as i32));
        }

        // 9. 趋势强度
        if features.trend_strength > dec!(25) {
            strength += 10;
            reasons.push(format!("TREND{:.0}", features.trend_strength));
        }

        // 10. 价差收窄
        if features.spread_bps < dec!(10) {
            strength += 5;
            reasons.push("SPREAD".to_string());
        }

        // 限制最大强度
        let strength = strength.min(100);

        if strength >= self.min_strength {
            Some(PumpSignal {
                timestamp: chrono::Local::now(),
                symbol: symbol.to_string(),
                strength: strength as u8,
                pump_probability,
                accumulation_score,
                ofi: ofi_val,
                obi: obi_val,
                price: current_price,
                target: target_price.to_f64().unwrap_or(0.0),
                bid_volume_change: bid_change,
                max_bid_ratio: max_bid,
                slope_bid,
                reasons,
            })
        } else {
            None
        }
    }

    // 写入拉盘信号到文件
    pub fn write_pump_signal(&self, signal: &PumpSignal) -> std::io::Result<()> {
        let mut file = OpenOptions::new()
            .create(true)
            .append(true)
            .open(&self.output_file)?;

        let time_str = signal.timestamp.format("%H:%M:%S");

        // 强度表情
        let strength_emoji = if signal.strength >= 80 {
            "🔥🔥"
        } else if signal.strength >= 60 {
            "🚀🚀"
        } else if signal.strength >= 40 {
            "📈📈"
        } else {
            "⬆️"
        };

        // 拼接原因（限制长度）
        let reasons_str = signal.reasons.join(" ");

        writeln!(
            file,
            "{} | {:<10} | {:<2}%{} | {:<3}% | {:<12.0} | {:<+6.1}% | {:<10.4} | {:<10.4} | {:<+6.1}% | {:<6.1}% | {}",
            time_str,
            signal.symbol,
            signal.strength,
            strength_emoji,
            signal.pump_probability,
            signal.ofi,
            signal.obi,
            signal.price,
            signal.target,
            signal.bid_volume_change,
            signal.max_bid_ratio,
            reasons_str
        )?;

        file.flush()?;
        Ok(())
    }

    // 批量写入多个信号（按强度排序）
    pub fn write_top_signals(&self, signals: &mut Vec<PumpSignal>) -> std::io::Result<()> {
        if signals.is_empty() {
            return Ok(());
        }

        // 按强度降序排序
        signals.sort_by(|a, b| b.strength.cmp(&a.strength));

        let mut file = OpenOptions::new()
            .create(true)
            .append(true)
            .open(&self.output_file)?;

        writeln!(file)?;
        writeln!(file, "{}", "=".repeat(150))?;
        writeln!(file, "📊 当前最强拉盘信号 TOP {} - {}",
                 signals.len(), chrono::Local::now().format("%H:%M:%S"))?;
        writeln!(file, "{}", "-".repeat(150))?;

        for (rank, signal) in signals.iter().enumerate() {
            let medal = if rank == 0 { "🥇" } else if rank == 1 { "🥈" } else if rank == 2 { "🥉" } else { "  " };

            writeln!(
                file,
                "{} {} | {:<10} | 强度{:>2}% | 概率{:>2}% | OFI={:<8.0} | OBI={:+.1}% | 目标:{:.4} | {}",
                medal,
                signal.timestamp.format("%H:%M:%S"),
                signal.symbol,
                signal.strength,
                signal.pump_probability,
                signal.ofi,
                signal.obi,
                signal.target,
                signal.reasons.join(" ")
            )?;
        }

        writeln!(file, "{}", "=".repeat(150))?;
        file.flush()?;
        Ok(())
    }
}