// src/web/bridge.rs — 终极修复版
//
// 所有借用冲突根因汇总：
//
// 1. guard.book.history.samples_raw.get(1)  → &RichSamplePoint → 持有 guard.book 借用
// 2. guard.anomaly_detector.get_stats()     → &AnomalyStats    → 持有 guard 借用  ← 本次根因
//
// get_stats() 返回 &AnomalyStats（引用），anom_stats 变量持有这个引用直到最后使用。
// 在 anom_stats 引用存活期间，guard.market_intel.take() 试图可变借用 guard，冲突。
//
// 修法：把 &AnomalyStats 里需要的字段立刻 copy 成值类型，引用随即释放。

use std::sync::Arc;
use tokio::time::{interval, Duration};
use rust_decimal::prelude::ToPrimitive;
use chrono::Local;

use crate::analysis::algorithms::{OverallSentiment, RiskLevel, TradingRecommendation};
use crate::analysis::multi_monitor::{MultiSymbolMonitor, SymbolMonitor};
use crate::web::state::{SharedDashboardState, SymbolJson, FeedEntry};

pub async fn run_bridge(
    monitor:    Arc<MultiSymbolMonitor>,
    dash:       SharedDashboardState,
    refresh_ms: u64,
) {
    let mut tick = interval(Duration::from_millis(refresh_ms));
    loop {
        tick.tick().await;

        let arcs: Vec<(String, Arc<tokio::sync::Mutex<SymbolMonitor>>)> = {
            let monitors = monitor.monitors.lock().await;
            monitors.iter().map(|(k, v)| (k.clone(), v.clone())).collect()
        };

        for (symbol, arc) in arcs {
            let mut guard = arc.lock().await;

            // ── 块1：所有 guard.book 访问，块结束借用释放 ──────────
            let (features, top_bids_raw, top_asks_raw, mid, price_change_pct) = {
                let features = guard.book.compute_features(10);
                let (top_bids_raw, top_asks_raw) = guard.book.top_n(12);

                let mid = (features.weighted_bid_price + features.weighted_ask_price)
                    .to_f64().unwrap_or(0.0) / 2.0;

                let prev_mid: f64 = guard.book.history.samples_raw
                    .get(1)
                    .and_then(|s| s.mid_price.to_f64())
                    .unwrap_or(mid);

                let price_change_pct = if prev_mid != 0.0 {
                    (mid - prev_mid) / prev_mid * 100.0
                } else { 0.0 };

                (features, top_bids_raw, top_asks_raw, mid, price_change_pct)
            }; // ← guard.book 借用释放

            // ── 块2：get_stats() 返回引用，立即 copy 字段值，引用释放 ──
            //    关键：不能 let anom_stats = guard.anomaly_detector.get_stats();
            //    因为 anom_stats 是 &AnomalyStats，会持有 guard 的借用直到最后使用。
            //    改为立即解构成 Copy 的值类型。
            let (anom_count_1m, anom_max_severity) = {
                let s = guard.anomaly_detector.get_stats();
                (s.last_minute_count, s.max_severity)
            }; // ← &AnomalyStats 引用在此释放，guard 借用结束

            let update_count = guard.update_count; // u64，Copy，无借用

            // ── 块3：take() + analyze() + put back，此时 guard 无任何借用 ──
            let analysis = {
                let mut intel = guard.market_intel.take().unwrap();
                let result = intel.analyze(&guard.book, &features);
                guard.market_intel = Some(intel);
                result
            };

            // ── 组装 JSON ────────────────────────────────────────────
            let snap = SymbolJson {
                symbol: symbol.clone(),
                bid:  features.weighted_bid_price.to_f64().unwrap_or(0.0),
                ask:  features.weighted_ask_price.to_f64().unwrap_or(0.0),
                mid,
                spread_bps:       features.spread_bps.to_f64().unwrap_or(0.0),
                price_change_pct,
                ofi:              features.ofi.to_f64().unwrap_or(0.0),
                ofi_raw:          features.ofi_raw.to_f64().unwrap_or(0.0),
                obi:              features.obi.to_f64().unwrap_or(0.0),
                trend_strength:   features.trend_strength.to_f64().unwrap_or(0.0),
                pump_score:       features.pump_score,
                dump_score:       features.dump_score,
                pump_signal:      features.pump_signal,
                dump_signal:      features.dump_signal,
                whale_entry:      features.whale_entry,
                whale_exit:       features.whale_exit,
                bid_eating:       features.bid_eating,
                total_bid_volume: features.total_bid_volume.to_f64().unwrap_or(0.0),
                total_ask_volume: features.total_ask_volume.to_f64().unwrap_or(0.0),
                max_bid_ratio:    features.max_bid_ratio.to_f64().unwrap_or(0.0),
                max_ask_ratio:    features.max_ask_ratio.to_f64().unwrap_or(0.0),
                top_bids: top_bids_raw.iter()
                    .map(|(p, q)| [p.to_f64().unwrap_or(0.0), q.to_f64().unwrap_or(0.0)])
                    .collect(),
                top_asks: top_asks_raw.iter()
                    .map(|(p, q)| [p.to_f64().unwrap_or(0.0), q.to_f64().unwrap_or(0.0)])
                    .collect(),
                anomaly_count_1m:     anom_count_1m,
                anomaly_max_severity: anom_max_severity,
                sentiment:      sentinel_str(&analysis.overall_sentiment),
                risk_level:     risk_str(&analysis.risk_level),
                recommendation: rec_str(&analysis.trading_recommendation),
                whale_type:     format!("{:?}", analysis.whale.whale_type),
                pump_probability: analysis.pump_dump.pump_probability,
                update_count,
            };

            // ── 生成 Feed ────────────────────────────────────────────
            let mut feed_entries: Vec<FeedEntry> = Vec::new();
            let t = Local::now().format("%H:%M:%S").to_string();

            if features.pump_signal && features.pump_score >= 60 {
                feed_entries.push(FeedEntry {
                    time: t.clone(), symbol: symbol.clone(),
                    r#type: "pump".into(), score: Some(features.pump_score),
                    desc: format!("拉盘评分{}分  OBI:{:+.1}%  OFI:{:.0}",
                                  features.pump_score,
                                  features.obi.to_f64().unwrap_or(0.0),
                                  features.ofi.to_f64().unwrap_or(0.0)),
                });
            }
            if features.dump_signal && features.dump_score >= 60 {
                feed_entries.push(FeedEntry {
                    time: t.clone(), symbol: symbol.clone(),
                    r#type: "dump".into(), score: Some(features.dump_score),
                    desc: format!("砸盘评分{}分  OBI:{:+.1}%",
                                  features.dump_score,
                                  features.obi.to_f64().unwrap_or(0.0)),
                });
            }
            if features.whale_entry {
                feed_entries.push(FeedEntry {
                    time: t.clone(), symbol: symbol.clone(),
                    r#type: "whale".into(), score: None,
                    desc: format!("🐋 鲸鱼进场  买单大单占比{:.1}%",
                                  features.max_bid_ratio.to_f64().unwrap_or(0.0)),
                });
            }
            if anom_max_severity >= 75 {
                feed_entries.push(FeedEntry {
                    time: t.clone(), symbol: symbol.clone(),
                    r#type: "anomaly".into(), score: Some(anom_max_severity),
                    desc: format!("严重异动 sev:{} 1分钟{}次",
                                  anom_max_severity, anom_count_1m),
                });
            }

            // ── 释放锁，写入 DashboardState ──────────────────────────
            drop(guard);
            let mut ds = dash.write().await;
            ds.upsert(snap);
            for entry in feed_entries { ds.push_feed(entry); }
        }
    }
}

fn sentinel_str(s: &OverallSentiment) -> String {
    match s {
        OverallSentiment::StrongBullish => "StrongBullish",
        OverallSentiment::Bullish       => "Bullish",
        OverallSentiment::Neutral       => "Neutral",
        OverallSentiment::Bearish       => "Bearish",
        OverallSentiment::StrongBearish => "StrongBearish",
    }.into()
}
fn risk_str(r: &RiskLevel) -> String {
    match r {
        RiskLevel::VeryLow  => "VeryLow",
        RiskLevel::Low      => "Low",
        RiskLevel::Medium   => "Medium",
        RiskLevel::High     => "High",
        RiskLevel::VeryHigh => "VeryHigh",
    }.into()
}
fn rec_str(r: &TradingRecommendation) -> String {
    match r {
        TradingRecommendation::StrongBuy  => "StrongBuy",
        TradingRecommendation::Buy        => "Buy",
        TradingRecommendation::Neutral    => "Neutral",
        TradingRecommendation::Sell       => "Sell",
        TradingRecommendation::StrongSell => "StrongSell",
        TradingRecommendation::Wait       => "Wait",
    }.into()
}