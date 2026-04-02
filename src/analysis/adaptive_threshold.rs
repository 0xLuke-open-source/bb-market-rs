//! 自适应阈值模块
//!
//! 每个 symbol 独立维护一个 4h 滚动窗口，按约 5s 采样一次（由 bridge 调用）。
//! 热身完成（≥ 720 个样本，约 1 小时）后，向上游提供 z-score 查询。
//! 热身前返回 None，上游继续使用绝对值阈值。

use std::collections::VecDeque;

use crate::store::l2_book::OrderBookFeatures;

/// 最大样本数（4h × 720 samples/h = 2880）
const MAX_SAMPLES: usize = 2880;
/// 热身所需最小样本数（约 1h）
const WARM_MIN_SAMPLES: usize = 720;

/// 单个 symbol 的自适应阈值，保存最近 4h 的滚动统计
#[derive(Debug, Clone)]
pub struct SymbolAdaptiveThreshold {
    ofi_samples: VecDeque<f64>,
    obi_samples: VecDeque<f64>,
    vol_samples: VecDeque<f64>,     // total_bid_volume + total_ask_volume
    bid_vol_samples: VecDeque<f64>, // total_bid_volume
    spread_samples: VecDeque<f64>,  // spread_bps
}

impl SymbolAdaptiveThreshold {
    pub fn new() -> Self {
        Self {
            ofi_samples: VecDeque::with_capacity(MAX_SAMPLES),
            obi_samples: VecDeque::with_capacity(MAX_SAMPLES),
            vol_samples: VecDeque::with_capacity(MAX_SAMPLES),
            bid_vol_samples: VecDeque::with_capacity(MAX_SAMPLES),
            spread_samples: VecDeque::with_capacity(MAX_SAMPLES),
        }
    }

    /// 每帧调用，推入新的特征样本
    pub fn push(&mut self, features: &OrderBookFeatures) {
        let total_vol = features
            .total_bid_volume
            .to_f64()
            .unwrap_or(0.0)
            + features.total_ask_volume.to_f64().unwrap_or(0.0);

        push_capped(&mut self.ofi_samples, features.ofi.to_f64().unwrap_or(0.0));
        push_capped(&mut self.obi_samples, features.obi.to_f64().unwrap_or(0.0));
        push_capped(&mut self.vol_samples, total_vol);
        push_capped(
            &mut self.bid_vol_samples,
            features.total_bid_volume.to_f64().unwrap_or(0.0),
        );
        push_capped(
            &mut self.spread_samples,
            features.spread_bps.to_f64().unwrap_or(0.0),
        );
    }

    /// 是否热身完成（样本数 >= 720）
    pub fn is_warm(&self) -> bool {
        self.ofi_samples.len() >= WARM_MIN_SAMPLES
    }

    /// 当前样本数
    pub fn sample_count(&self) -> usize {
        self.ofi_samples.len()
    }

    /// OFI z-score，热身前返回 None
    pub fn ofi_zscore(&self, val: f64) -> Option<f64> {
        if !self.is_warm() {
            return None;
        }
        Some(zscore(&self.ofi_samples, val))
    }

    /// OBI z-score，热身前返回 None
    pub fn obi_zscore(&self, val: f64) -> Option<f64> {
        if !self.is_warm() {
            return None;
        }
        Some(zscore(&self.obi_samples, val))
    }

    /// 总成交量 z-score（total_bid_volume + total_ask_volume），热身前返回 None
    pub fn vol_zscore(&self, val: f64) -> Option<f64> {
        if !self.is_warm() {
            return None;
        }
        Some(zscore(&self.vol_samples, val))
    }

    /// 买盘量 z-score，热身前返回 None
    pub fn bid_vol_zscore(&self, val: f64) -> Option<f64> {
        if !self.is_warm() {
            return None;
        }
        Some(zscore(&self.bid_vol_samples, val))
    }

    /// spread z-score，热身前返回 None
    pub fn spread_zscore(&self, val: f64) -> Option<f64> {
        if !self.is_warm() {
            return None;
        }
        Some(zscore(&self.spread_samples, val))
    }

    /// 生成供入库的快照
    pub fn snapshot(&self) -> AdaptiveThresholdSnapshot {
        let (ofi_mean, ofi_std) = mean_std(&self.ofi_samples);
        let (obi_mean, obi_std) = mean_std(&self.obi_samples);
        let (vol_mean, vol_std) = mean_std(&self.vol_samples);
        let (bid_vol_mean, bid_vol_std) = mean_std(&self.bid_vol_samples);
        let (spread_mean, spread_std) = mean_std(&self.spread_samples);
        AdaptiveThresholdSnapshot {
            sample_count: self.ofi_samples.len() as i32,
            is_warm: self.is_warm(),
            ofi_mean,
            ofi_std,
            obi_mean,
            obi_std,
            vol_mean,
            vol_std,
            bid_vol_mean,
            bid_vol_std,
            spread_mean,
            spread_std,
        }
    }
}

impl Default for SymbolAdaptiveThreshold {
    fn default() -> Self {
        Self::new()
    }
}

/// 供入库的快照（无时间字段，由持久化层填充 window_end_at）
#[derive(Debug, Clone)]
pub struct AdaptiveThresholdSnapshot {
    pub sample_count: i32,
    pub is_warm: bool,
    pub ofi_mean: f64,
    pub ofi_std: f64,
    pub obi_mean: f64,
    pub obi_std: f64,
    pub vol_mean: f64,
    pub vol_std: f64,
    pub bid_vol_mean: f64,
    pub bid_vol_std: f64,
    pub spread_mean: f64,
    pub spread_std: f64,
}

// ── 内部辅助函数 ────────────────────────────────────────────────────

/// Decimal → f64 trait，便于 push 时调用
trait ToF64 {
    fn to_f64(self) -> Option<f64>;
}

impl ToF64 for rust_decimal::Decimal {
    fn to_f64(self) -> Option<f64> {
        use rust_decimal::prelude::ToPrimitive;
        rust_decimal::prelude::ToPrimitive::to_f64(&self)
    }
}

fn push_capped(buf: &mut VecDeque<f64>, val: f64) {
    if buf.len() >= MAX_SAMPLES {
        buf.pop_front();
    }
    buf.push_back(val);
}

fn mean_std(buf: &VecDeque<f64>) -> (f64, f64) {
    if buf.is_empty() {
        return (0.0, 0.0);
    }
    let n = buf.len() as f64;
    let mean = buf.iter().sum::<f64>() / n;
    let var = buf.iter().map(|x| (x - mean).powi(2)).sum::<f64>() / n;
    (mean, var.sqrt())
}

fn zscore(buf: &VecDeque<f64>, val: f64) -> f64 {
    let (mean, std) = mean_std(buf);
    if std < 1e-12 {
        return 0.0;
    }
    (val - mean) / std
}
