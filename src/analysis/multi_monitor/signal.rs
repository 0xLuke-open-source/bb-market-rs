//! 多币种监控的信号适配层。
//!
//! 这里把 `multi_monitor` 与历史上已有的 `PumpDetector` 对接起来，
//! 让实时监控模块只关心“何时要分析/写出信号”，而不关心具体落盘细节。

use std::sync::atomic::{AtomicUsize, Ordering};

use rust_decimal::Decimal;

use crate::analysis::pump_detector::{PumpDetector, PumpSignal};
use crate::store::l2_book::OrderBookFeatures;

lazy_static::lazy_static! {
    static ref PUMP_DETECTOR: PumpDetector = PumpDetector::new("pump_signals.txt")
        .with_min_strength(30);
}

static SIGNAL_BATCH_COUNTER: AtomicUsize = AtomicUsize::new(0);

/// 用订单簿特征和上游算法结果生成标准化的 `PumpSignal`。
pub fn analyze_signal(
    symbol: &str,
    features: &OrderBookFeatures,
    pump_probability: u8,
    accumulation_score: u8,
    target_price: Decimal,
) -> Option<PumpSignal> {
    PUMP_DETECTOR.analyze_symbol(
        symbol,
        features,
        pump_probability,
        accumulation_score,
        target_price,
    )
}

/// 立即把单条信号写入持久化介质。
///
/// 单条写入用于已经确认触发的高价值事件，便于回溯。
pub fn write_signal(signal: &PumpSignal) {
    let _ = PUMP_DETECTOR.write_pump_signal(signal);
}

/// 批量刷出当前收集到的 top signals。
///
/// 这里每 10 轮才写一次，目的是减少高频监控场景下的磁盘写放大。
pub fn flush_signal_batch(signals: &mut Vec<PumpSignal>) {
    let count = SIGNAL_BATCH_COUNTER.fetch_add(1, Ordering::SeqCst);
    if count % 10 == 0 && !signals.is_empty() {
        let _ = PUMP_DETECTOR.write_top_signals(signals);
    }
}
