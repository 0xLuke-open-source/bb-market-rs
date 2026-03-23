use chrono::Local;

use crate::store::l2_book::OrderBookFeatures;
use crate::web::state::FeedEntry;

pub fn build_feed_entries(
    symbol: &str,
    watch_level: &str,
    signal_reason: &str,
    features: &OrderBookFeatures,
    anomaly_max_severity: u8,
    cvd: f64,
) -> Vec<FeedEntry> {
    let mut feed_entries = Vec::new();
    let time = Local::now().format("%H:%M:%S").to_string();

    if features.pump_signal && features.pump_score >= 60 {
        feed_entries.push(FeedEntry {
            time: time.clone(),
            symbol: symbol.to_string(),
            r#type: "pump".into(),
            score: Some(features.pump_score),
            desc: format!("[{}] {}", watch_level, signal_reason),
        });
    }
    if features.dump_signal && features.dump_score >= 60 {
        feed_entries.push(FeedEntry {
            time: time.clone(),
            symbol: symbol.to_string(),
            r#type: "dump".into(),
            score: Some(features.dump_score),
            desc: format!("[{}] {}", watch_level, signal_reason),
        });
    }
    if features.whale_entry {
        feed_entries.push(FeedEntry {
            time: time.clone(),
            symbol: symbol.to_string(),
            r#type: "whale".into(),
            score: None,
            desc: format!("[{}] {}", watch_level, signal_reason),
        });
    }
    if anomaly_max_severity >= 75 {
        feed_entries.push(FeedEntry {
            time: time.clone(),
            symbol: symbol.to_string(),
            r#type: "anomaly".into(),
            score: Some(anomaly_max_severity),
            desc: format!("[{}] {}", watch_level, signal_reason),
        });
    }
    if cvd.abs() > 10000.0 {
        let direction = if cvd > 0.0 { "持续主动买入" } else { "持续主动卖出" };
        feed_entries.push(FeedEntry {
            time,
            symbol: symbol.to_string(),
            r#type: "cvd".into(),
            score: None,
            desc: format!("[{}] {}，{}", watch_level, direction, signal_reason),
        });
    }

    feed_entries
}
