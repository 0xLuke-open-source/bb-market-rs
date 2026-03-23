use std::sync::atomic::{AtomicUsize, Ordering};

use rust_decimal::Decimal;

use crate::analysis::pump_detector::{PumpDetector, PumpSignal};
use crate::store::l2_book::OrderBookFeatures;

lazy_static::lazy_static! {
    static ref PUMP_DETECTOR: PumpDetector = PumpDetector::new("pump_signals.txt")
        .with_min_strength(30);
}

static SIGNAL_BATCH_COUNTER: AtomicUsize = AtomicUsize::new(0);

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

pub fn write_signal(signal: &PumpSignal) {
    let _ = PUMP_DETECTOR.write_pump_signal(signal);
}

pub fn flush_signal_batch(signals: &mut Vec<PumpSignal>) {
    let count = SIGNAL_BATCH_COUNTER.fetch_add(1, Ordering::SeqCst);
    if count % 10 == 0 && !signals.is_empty() {
        let _ = PUMP_DETECTOR.write_top_signals(signals);
    }
}
