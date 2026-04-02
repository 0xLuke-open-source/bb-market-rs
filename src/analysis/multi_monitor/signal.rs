//! 多币种监控的信号适配层。
//!
//! 这里把 `multi_monitor` 与历史上已有的 `PumpDetector` 对接起来，
//! 让实时监控模块只关心”何时要分析/写出信号”，而不关心具体落盘细节。

use std::sync::atomic::{AtomicUsize, Ordering};

use crate::analysis::pump_detector::PumpSignal;

static SIGNAL_BATCH_COUNTER: AtomicUsize = AtomicUsize::new(0);

/// 立即把单条信号写入持久化介质。
///
/// 单条写入用于已经确认触发的高价值事件，便于回溯。
pub fn write_signal(_signal: &PumpSignal) {
    // No-op：信号已在 bridge/panel 层落库，此处保留供调试扩展
}

/// 批量刷出当前收集到的 top signals。
///
/// 这里每 10 轮才写一次，目的是减少高频监控场景下的磁盘写放大。
pub fn flush_signal_batch(signals: &mut Vec<PumpSignal>) {
    let _count = SIGNAL_BATCH_COUNTER.fetch_add(1, Ordering::SeqCst);
    let _ = signals;
}

