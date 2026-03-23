// src/web/bridge.rs — 完整版，含 CVD/Ticker/Kline/BigTrade
use std::sync::Arc;
use tokio::time::{interval, Duration};
use rust_decimal::prelude::ToPrimitive;
use chrono::Local;

use crate::analysis::algorithms::{OverallSentiment, RiskLevel, TradingRecommendation};
use crate::analysis::multi_monitor::{MultiSymbolMonitor, SymbolMonitor};
use crate::web::spot::SpotTradingService;
use crate::web::state::{
    SharedDashboardState, SymbolJson, FeedEntry, KlineJson, BigTradeJson,
};

fn watch_level_label(pump_score: u8, dump_score: u8, anomaly_max_severity: u8, whale_entry: bool) -> &'static str {
    if anomaly_max_severity >= 85 || pump_score >= 85 || dump_score >= 85 {
        "强提醒"
    } else if whale_entry || anomaly_max_severity >= 75 || pump_score >= 75 || dump_score >= 75 {
        "重点关注"
    } else if pump_score >= 60 || dump_score >= 60 {
        "普通关注"
    } else {
        "观察"
    }
}

fn status_summary(
    pump_score: u8,
    dump_score: u8,
    taker_buy_ratio: f64,
    cvd: f64,
    obi: f64,
    anomaly_max_severity: u8,
    whale_entry: bool,
    whale_exit: bool,
) -> String {
    if anomaly_max_severity >= 85 {
        return "波动明显异常，先看风险再决定是否继续关注。".into();
    }
    if whale_entry && pump_score >= 70 {
        return "大户资金有进场迹象，买盘偏强，适合重点盯住。".into();
    }
    if whale_exit && dump_score >= 70 {
        return "大户疑似离场，卖压偏强，短线要防回落。".into();
    }
    if pump_score >= 75 && taker_buy_ratio >= 60.0 && cvd > 0.0 {
        return "主动买盘持续增强，短线偏强，可以重点观察突破。".into();
    }
    if dump_score >= 75 && taker_buy_ratio <= 40.0 && cvd < 0.0 {
        return "主动卖盘持续增强，短线偏弱，注意继续下压。".into();
    }
    if obi >= 15.0 && taker_buy_ratio >= 55.0 {
        return "买盘略占优势，当前偏强，但还需要继续确认。".into();
    }
    if obi <= -15.0 && taker_buy_ratio <= 45.0 {
        return "卖盘略占优势，当前偏弱，追涨要谨慎。".into();
    }
    "多空暂时比较均衡，先观察是否出现新的主导资金方向。".into()
}

fn signal_reason(
    pump_score: u8,
    dump_score: u8,
    taker_buy_ratio: f64,
    cvd: f64,
    obi: f64,
    anomaly_count_1m: u32,
    whale_entry: bool,
    whale_exit: bool,
) -> String {
    let mut parts: Vec<String> = Vec::new();
    if pump_score >= 60 {
        parts.push(format!("上涨动能 {} 分", pump_score));
    }
    if dump_score >= 60 {
        parts.push(format!("下跌压力 {} 分", dump_score));
    }
    if obi.abs() >= 10.0 {
        parts.push(format!("买卖盘失衡 {:+.1}%", obi));
    }
    if taker_buy_ratio >= 60.0 {
        parts.push(format!("主动买入占比 {:.0}%", taker_buy_ratio));
    } else if taker_buy_ratio <= 40.0 {
        parts.push(format!("主动卖出占比 {:.0}%", 100.0 - taker_buy_ratio));
    }
    if cvd.abs() >= 10000.0 {
        parts.push(format!("主动买卖量差 {:+.0}", cvd));
    }
    if anomaly_count_1m >= 30 {
        parts.push(format!("1分钟异常波动 {} 次", anomaly_count_1m));
    }
    if whale_entry {
        parts.push("检测到大户资金进场".into());
    }
    if whale_exit {
        parts.push("检测到大户资金离场".into());
    }
    if parts.is_empty() {
        "当前没有特别突出的异常信号。".into()
    } else {
        parts.join("，")
    }
}

pub async fn run_bridge(
    monitor:    Arc<MultiSymbolMonitor>,
    dash:       SharedDashboardState,
    spot:       SpotTradingService,
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

            // ── 块1：从 book 取数据，块结束释放借用 ───────────────
            let (features, top_bids_raw, top_asks_raw, mid, _) = {
                let f = guard.book.compute_features(10);
                let (b, a) = guard.book.top_n(25);
                let mid = (f.weighted_bid_price + f.weighted_ask_price)
                    .to_f64().unwrap_or(0.0) / 2.0;
                (f, b, a, mid, 0.0f64)
            };

            spot.sync_liquidity(&symbol, &top_bids_raw, &top_asks_raw).await.ok();

            // ── 块2：anomaly stats（返回引用，立即 copy）──────────
            let (anom_count_1m, anom_max_severity) = {
                let s = guard.anomaly_detector.get_stats();
                (s.last_minute_count, s.max_severity)
            };

            let update_count = guard.update_count;

            // ── 块3：market_intel analyze（Option::take 避免借用冲突）
            let analysis = {
                let mut intel = guard.market_intel.take().unwrap();
                let result = intel.analyze(&guard.book, &features);
                guard.market_intel = Some(intel);
                result
            };

            // ── 成交流字段（直接 copy）────────────────────────────
            let cvd             = guard.cvd.to_f64().unwrap_or(0.0);
            let taker_buy_ratio = guard.taker_buy_ratio;
            let change_24h_pct  = guard.change_24h_pct;
            let high_24h        = guard.price_24h_high;
            let low_24h         = guard.price_24h_low;
            let volume_24h      = guard.volume_24h;
            let quote_vol_24h   = guard.quote_vol_24h;

            // ── 多周期K线 ─────────────────────────────────────────
            let klines: std::collections::HashMap<String, Vec<KlineJson>> = guard.klines.iter()
                .map(|(interval, bars)| {
                    let v: Vec<KlineJson> = bars.iter().map(|k| KlineJson {
                        interval: k.interval.clone(),
                        t: k.open_time, o: k.open, h: k.high, l: k.low, c: k.close,
                        v: k.volume, tbr: k.taker_buy_ratio,
                    }).collect();
                    (interval.clone(), v)
                }).collect();
            let current_kline: std::collections::HashMap<String, KlineJson> = guard.current_kline.iter()
                .map(|(interval, k)| (interval.clone(), KlineJson {
                    interval: k.interval.clone(),
                    t: k.open_time, o: k.open, h: k.high, l: k.low, c: k.close,
                    v: k.volume, tbr: k.taker_buy_ratio,
                })).collect();

            // ── 大单（最近10条）───────────────────────────────────
            let big_trades: Vec<BigTradeJson> = guard.big_trades.iter().rev().take(10)
                .map(|bt| BigTradeJson { t: bt.time_ms, p: bt.price, q: bt.qty, buy: bt.is_buy })
                .collect();

            let watch_level = watch_level_label(
                features.pump_score,
                features.dump_score,
                anom_max_severity,
                features.whale_entry,
            ).to_string();
            let status_summary = status_summary(
                features.pump_score,
                features.dump_score,
                taker_buy_ratio,
                cvd,
                features.obi.to_f64().unwrap_or(0.0),
                anom_max_severity,
                features.whale_entry,
                features.whale_exit,
            );
            let signal_reason = signal_reason(
                features.pump_score,
                features.dump_score,
                taker_buy_ratio,
                cvd,
                features.obi.to_f64().unwrap_or(0.0),
                anom_count_1m,
                features.whale_entry,
                features.whale_exit,
            );

            // ── 组装 SymbolJson ───────────────────────────────────
            let snap = SymbolJson {
                symbol: symbol.clone(),
                status_summary,
                watch_level: watch_level.clone(),
                signal_reason: signal_reason.clone(),
                bid:  features.weighted_bid_price.to_f64().unwrap_or(0.0),
                ask:  features.weighted_ask_price.to_f64().unwrap_or(0.0),
                mid,
                spread_bps:       features.spread_bps.to_f64().unwrap_or(0.0),
                change_24h_pct, high_24h, low_24h, volume_24h, quote_vol_24h,
                ofi:              features.ofi.to_f64().unwrap_or(0.0),
                ofi_raw:          features.ofi_raw.to_f64().unwrap_or(0.0),
                obi:              features.obi.to_f64().unwrap_or(0.0),
                trend_strength:   features.trend_strength.to_f64().unwrap_or(0.0),
                cvd, taker_buy_ratio,
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
                top_bids: top_bids_raw.iter().take(12).map(|(p,q)| [p.to_f64().unwrap_or(0.0), q.to_f64().unwrap_or(0.0)]).collect(),
                top_asks: top_asks_raw.iter().take(12).map(|(p,q)| [p.to_f64().unwrap_or(0.0), q.to_f64().unwrap_or(0.0)]).collect(),
                anomaly_count_1m: anom_count_1m,
                anomaly_max_severity: anom_max_severity,
                sentiment:      sentinel_str(&analysis.overall_sentiment),
                risk_level:     risk_str(&analysis.risk_level),
                recommendation: rec_str(&analysis.trading_recommendation),
                whale_type:     format!("{:?}", analysis.whale.whale_type),
                pump_probability: analysis.pump_dump.pump_probability,
                klines, current_kline, big_trades,
                update_count,
            };

            // ── Feed 条目 ─────────────────────────────────────────
            let mut feed_entries: Vec<FeedEntry> = Vec::new();
            let t = Local::now().format("%H:%M:%S").to_string();
            let sym_short = symbol.replace("USDT", "");

            if features.pump_signal && features.pump_score >= 60 {
                feed_entries.push(FeedEntry {
                    time: t.clone(), symbol: symbol.clone(), r#type: "pump".into(),
                    score: Some(features.pump_score),
                    desc: format!("[{}] {}", watch_level, signal_reason),
                });
            }
            if features.dump_signal && features.dump_score >= 60 {
                feed_entries.push(FeedEntry {
                    time: t.clone(), symbol: symbol.clone(), r#type: "dump".into(),
                    score: Some(features.dump_score),
                    desc: format!("[{}] {}", watch_level, signal_reason),
                });
            }
            if features.whale_entry {
                feed_entries.push(FeedEntry {
                    time: t.clone(), symbol: symbol.clone(), r#type: "whale".into(),
                    score: None,
                    desc: format!("[{}] {}", watch_level, signal_reason),
                });
            }
            if anom_max_severity >= 75 {
                feed_entries.push(FeedEntry {
                    time: t.clone(), symbol: symbol.clone(), r#type: "anomaly".into(),
                    score: Some(anom_max_severity),
                    desc: format!("[{}] {}", watch_level, signal_reason),
                });
            }
            // CVD 大幅偏离预警
            if cvd.abs() > 10000.0 {
                let dir = if cvd > 0.0 { "持续主动买入" } else { "持续主动卖出" };
                feed_entries.push(FeedEntry {
                    time: t.clone(), symbol: symbol.clone(), r#type: "cvd".into(),
                    score: None,
                    desc: format!("[{}] {}，{}", watch_level, dir, signal_reason),
                });
            }

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
