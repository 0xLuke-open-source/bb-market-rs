use std::cmp::Reverse;
use std::collections::{HashMap, VecDeque};

use chrono::{DateTime, Local};
use rust_decimal::prelude::ToPrimitive;
use rust_decimal::Decimal;
use rust_decimal_macros::dec;

use crate::store::l2_book::{OrderBook, OrderBookFeatures};

use super::types::{AnomalyConfig, AnomalyEvent, AnomalyStats, AnomalyType, ChangeType, OrderSide};

pub struct OrderBookAnomalyDetector {
    snapshot_history: VecDeque<AnomalySnapshot>,
    order_history: VecDeque<OrderChange>,
    config: AnomalyConfig,
    stats: AnomalyStats,
    recent_anomalies: VecDeque<AnomalyEvent>,
}

#[derive(Debug, Clone)]
struct AnomalySnapshot {
    timestamp: DateTime<Local>,
    best_bid: Decimal,
    best_ask: Decimal,
    mid_price: Decimal,
    bid_volume: Decimal,
    ask_volume: Decimal,
    bid_depth_10: Decimal,
    ask_depth_10: Decimal,
    total_bid_orders: usize,
    total_ask_orders: usize,
    large_bid_count: u32,
    large_ask_count: u32,
}

#[derive(Debug, Clone)]
struct OrderChange {
    timestamp: DateTime<Local>,
    price: Decimal,
    quantity: Decimal,
    side: OrderSide,
    change_type: ChangeType,
    prev_quantity: Option<Decimal>,
}

impl OrderBookAnomalyDetector {
    pub fn new() -> Self {
        Self::with_config(AnomalyConfig::default())
    }

    pub fn with_config(config: AnomalyConfig) -> Self {
        Self {
            snapshot_history: VecDeque::with_capacity(600),
            order_history: VecDeque::with_capacity(10000),
            config,
            stats: AnomalyStats::default(),
            recent_anomalies: VecDeque::with_capacity(100),
        }
    }

    pub fn detect(&mut self, book: &OrderBook, features: &OrderBookFeatures) -> Vec<AnomalyEvent> {
        let mut anomalies = Vec::new();

        self.update_snapshot(book);
        anomalies.extend(self.detect_mega_orders(book, features));
        anomalies.extend(self.detect_rapid_cancellations());
        anomalies.extend(self.detect_price_spikes());
        anomalies.extend(self.detect_liquidity_drops(features));
        anomalies.extend(self.detect_depth_gaps(book));
        anomalies.extend(self.detect_order_surge());
        anomalies.extend(self.detect_imbalance_spikes(features));
        anomalies.extend(self.detect_whale_walls(book, features));
        anomalies.extend(self.detect_complex_patterns());

        for anomaly in &anomalies {
            self.update_stats(anomaly);
            self.recent_anomalies.push_back(anomaly.clone());
        }

        self.cleanup_history();
        anomalies
    }

    fn detect_mega_orders(&self, book: &OrderBook, features: &OrderBookFeatures) -> Vec<AnomalyEvent> {
        let mut anomalies = Vec::new();
        let total_bid_depth = features.bid_volume_depth;
        let total_ask_depth = features.ask_volume_depth;

        for (Reverse(price), qty) in book.bids.iter().take(10) {
            if total_bid_depth > Decimal::ZERO {
                let percentage = qty / total_bid_depth * dec!(100);
                if percentage > self.config.mega_bid_threshold {
                    anomalies.push(AnomalyEvent {
                        timestamp: Local::now(),
                        anomaly_type: AnomalyType::MegaBid,
                        severity: (percentage.to_u64().unwrap_or(0) / 2) as u8,
                        confidence: 85,
                        price_level: Some(*price),
                        side: Some(OrderSide::Bid),
                        size: Some(*qty),
                        percentage: Some(percentage),
                        duration_ms: None,
                        frequency: None,
                        price_impact: None,
                        volume_impact: Some(percentage),
                        description: format!("超大买单 {:.0} USDT ({:.1}% 深度)", qty, percentage),
                        details: HashMap::new(),
                    });
                }
            }
        }

        for (price, qty) in book.asks.iter().take(10) {
            if total_ask_depth > Decimal::ZERO {
                let percentage = qty / total_ask_depth * dec!(100);
                if percentage > self.config.mega_ask_threshold {
                    anomalies.push(AnomalyEvent {
                        timestamp: Local::now(),
                        anomaly_type: AnomalyType::MegaAsk,
                        severity: (percentage.to_u64().unwrap_or(0) / 2) as u8,
                        confidence: 85,
                        price_level: Some(*price),
                        side: Some(OrderSide::Ask),
                        size: Some(*qty),
                        percentage: Some(percentage),
                        duration_ms: None,
                        frequency: None,
                        price_impact: None,
                        volume_impact: Some(percentage),
                        description: format!("超大卖单 {:.0} USDT ({:.1}% 深度)", qty, percentage),
                        details: HashMap::new(),
                    });
                }
            }
        }

        anomalies
    }

    fn detect_rapid_cancellations(&mut self) -> Vec<AnomalyEvent> {
        let mut anomalies = Vec::new();
        let now = Local::now();
        let mut cancel_counts: HashMap<(Decimal, OrderSide), (u32, DateTime<Local>)> = HashMap::new();

        for change in self.order_history.iter().rev().take(100) {
            if change.change_type == ChangeType::Cancel {
                let time_diff = (now - change.timestamp).num_milliseconds();
                if time_diff <= self.config.rapid_cancel_ms as i64 {
                    let key = (change.price, change.side.clone());
                    let entry = cancel_counts.entry(key).or_insert((0, change.timestamp));
                    entry.0 += 1;
                }
            }
        }

        for ((price, side), (count, first_time)) in cancel_counts {
            if count >= 5 {
                let duration = (now - first_time).num_milliseconds();
                let frequency = count as f64 / (duration as f64 / 1000.0);
                let severity = (count * 10).min(100) as u8;

                anomalies.push(AnomalyEvent {
                    timestamp: now,
                    anomaly_type: if count >= self.config.mass_cancel_threshold as u32 {
                        AnomalyType::MassCancellation
                    } else {
                        AnomalyType::RapidCancellation
                    },
                    severity,
                    confidence: 75,
                    price_level: Some(price),
                    side: Some(side.clone()),
                    size: None,
                    percentage: None,
                    duration_ms: Some(duration as u64),
                    frequency: Some(frequency),
                    price_impact: None,
                    volume_impact: None,
                    description: format!(
                        "{:?} 在 {:.6} 快速撤销 {} 次 (频率: {:.1}次/秒)",
                        side, price, count, frequency
                    ),
                    details: HashMap::new(),
                });
            }
        }

        anomalies
    }

    fn detect_price_spikes(&self) -> Vec<AnomalyEvent> {
        let mut anomalies = Vec::new();
        if self.snapshot_history.len() < 2 {
            return anomalies;
        }

        let current = self.snapshot_history.back().expect("snapshot exists");
        let previous = self
            .snapshot_history
            .get(self.snapshot_history.len() - 2)
            .expect("previous snapshot exists");

        let price_change_bps = if previous.mid_price > Decimal::ZERO {
            ((current.mid_price - previous.mid_price) / previous.mid_price * dec!(10000)).abs()
        } else {
            Decimal::ZERO
        };

        if price_change_bps > self.config.price_spike_bps {
            let is_up = current.mid_price > previous.mid_price;
            anomalies.push(AnomalyEvent {
                timestamp: Local::now(),
                anomaly_type: AnomalyType::PriceSpike,
                severity: (price_change_bps.to_u64().unwrap_or(0) / 5) as u8,
                confidence: 90,
                price_level: Some(current.mid_price),
                side: Some(if is_up { OrderSide::Bid } else { OrderSide::Ask }),
                size: None,
                percentage: None,
                duration_ms: None,
                frequency: None,
                price_impact: Some(price_change_bps / dec!(100)),
                volume_impact: None,
                description: format!(
                    "价格{} {:.2}bps ({:.4} -> {:.4})",
                    if is_up { "飙升" } else { "暴跌" },
                    price_change_bps,
                    previous.mid_price,
                    current.mid_price
                ),
                details: HashMap::new(),
            });
        }

        anomalies
    }

    fn detect_liquidity_drops(&self, features: &OrderBookFeatures) -> Vec<AnomalyEvent> {
        let mut anomalies = Vec::new();
        if self.snapshot_history.len() < 10 {
            return anomalies;
        }

        let current_depth = features.bid_volume_depth + features.ask_volume_depth;
        let avg_depth: Decimal = self
            .snapshot_history
            .iter()
            .rev()
            .skip(1)
            .take(10)
            .map(|snapshot| snapshot.bid_volume + snapshot.ask_volume)
            .sum::<Decimal>()
            / Decimal::from(10);

        if avg_depth > Decimal::ZERO {
            let drop_percentage = (avg_depth - current_depth) / avg_depth * dec!(100);
            if drop_percentage > self.config.liquidity_drop_threshold {
                anomalies.push(AnomalyEvent {
                    timestamp: Local::now(),
                    anomaly_type: AnomalyType::LiquidityDrop,
                    severity: (drop_percentage.to_u64().unwrap_or(0) / 2) as u8,
                    confidence: 80,
                    price_level: None,
                    side: None,
                    size: None,
                    percentage: Some(drop_percentage),
                    duration_ms: None,
                    frequency: None,
                    price_impact: None,
                    volume_impact: Some(drop_percentage),
                    description: format!(
                        "流动性骤降 {:.1}% (平均: {:.0}, 当前: {:.0})",
                        drop_percentage, avg_depth, current_depth
                    ),
                    details: HashMap::new(),
                });
            }
        }

        anomalies
    }

    fn detect_depth_gaps(&self, book: &OrderBook) -> Vec<AnomalyEvent> {
        let mut anomalies = Vec::new();
        let (best_bid, best_ask) = match book.best_bid_ask() {
            Some(levels) => levels,
            None => return anomalies,
        };

        let mut last_price = best_bid;
        for (Reverse(price), _) in book.bids.iter().take(10).skip(1) {
            let gap_bps = (last_price - *price) / last_price * dec!(10000);
            if gap_bps > self.config.depth_gap_bps {
                anomalies.push(AnomalyEvent {
                    timestamp: Local::now(),
                    anomaly_type: AnomalyType::DepthGap,
                    severity: (gap_bps.to_u64().unwrap_or(0) / 4) as u8,
                    confidence: 85,
                    price_level: Some(*price),
                    side: Some(OrderSide::Bid),
                    size: None,
                    percentage: Some(gap_bps / dec!(100)),
                    duration_ms: None,
                    frequency: None,
                    price_impact: Some(gap_bps / dec!(100)),
                    volume_impact: None,
                    description: format!(
                        "买单深度缺口 {:.1}bps ({:.6} -> {:.6})",
                        gap_bps, *price, last_price
                    ),
                    details: HashMap::new(),
                });
                break;
            }
            last_price = *price;
        }

        let mut last_price = best_ask;
        for (price, _) in book.asks.iter().take(10).skip(1) {
            let gap_bps = (*price - last_price) / last_price * dec!(10000);
            if gap_bps > self.config.depth_gap_bps {
                anomalies.push(AnomalyEvent {
                    timestamp: Local::now(),
                    anomaly_type: AnomalyType::DepthGap,
                    severity: (gap_bps.to_u64().unwrap_or(0) / 4) as u8,
                    confidence: 85,
                    price_level: Some(*price),
                    side: Some(OrderSide::Ask),
                    size: None,
                    percentage: Some(gap_bps / dec!(100)),
                    duration_ms: None,
                    frequency: None,
                    price_impact: Some(gap_bps / dec!(100)),
                    volume_impact: None,
                    description: format!(
                        "卖单深度缺口 {:.1}bps ({:.6} -> {:.6})",
                        gap_bps, last_price, *price
                    ),
                    details: HashMap::new(),
                });
                break;
            }
            last_price = *price;
        }

        anomalies
    }

    fn detect_order_surge(&mut self) -> Vec<AnomalyEvent> {
        let mut anomalies = Vec::new();
        let now = Local::now();
        if self.order_history.len() < 100 {
            return anomalies;
        }

        let recent_count = self
            .order_history
            .iter()
            .rev()
            .take_while(|order| (now - order.timestamp).num_milliseconds() < 1000)
            .count();
        let prev_count = self
            .order_history
            .iter()
            .rev()
            .skip_while(|order| (now - order.timestamp).num_milliseconds() < 1000)
            .take_while(|order| (now - order.timestamp).num_milliseconds() < 6000)
            .count();

        if prev_count > 0 {
            let surge_ratio = recent_count as f64 / (prev_count as f64 / 5.0);
            if surge_ratio > self.config.order_surge_threshold {
                anomalies.push(AnomalyEvent {
                    timestamp: now,
                    anomaly_type: AnomalyType::OrderFlowSurge,
                    severity: (surge_ratio * 20.0).min(100.0) as u8,
                    confidence: 80,
                    price_level: None,
                    side: None,
                    size: None,
                    percentage: None,
                    duration_ms: Some(1000),
                    frequency: Some(recent_count as f64),
                    price_impact: None,
                    volume_impact: None,
                    description: format!("订单流激增 {:.1}倍 ({} 单/秒)", surge_ratio, recent_count),
                    details: HashMap::new(),
                });
            }
        }

        anomalies
    }

    fn detect_imbalance_spikes(&self, features: &OrderBookFeatures) -> Vec<AnomalyEvent> {
        let mut anomalies = Vec::new();
        let imbalance = features.obi.abs();

        if imbalance > self.config.imbalance_spike_threshold {
            anomalies.push(AnomalyEvent {
                timestamp: Local::now(),
                anomaly_type: AnomalyType::ImbalanceSpike,
                severity: (imbalance.to_u64().unwrap_or(0) / 2) as u8,
                confidence: 90,
                price_level: None,
                side: Some(if features.obi > Decimal::ZERO {
                    OrderSide::Bid
                } else {
                    OrderSide::Ask
                }),
                size: None,
                percentage: Some(imbalance),
                duration_ms: None,
                frequency: None,
                price_impact: None,
                volume_impact: Some(imbalance),
                description: format!("订单簿失衡尖峰 {:.1}%", imbalance),
                details: HashMap::new(),
            });
        }

        anomalies
    }

    fn detect_whale_walls(&self, book: &OrderBook, features: &OrderBookFeatures) -> Vec<AnomalyEvent> {
        let mut anomalies = Vec::new();
        if let Some(last) = self.recent_anomalies.back() {
            if last.anomaly_type == AnomalyType::WhaleWall
                && (Local::now() - last.timestamp).num_milliseconds() < 10000
            {
                return anomalies;
            }
        }

        let mut bid_wall_count = 0;
        let mut bid_wall_volume = Decimal::ZERO;
        for (Reverse(_price), qty) in book.bids.iter().take(5) {
            if qty > &(features.total_bid_volume * dec!(0.1)) {
                bid_wall_count += 1;
                bid_wall_volume += qty;
            }
        }

        if bid_wall_count >= self.config.whale_wall_min_orders {
            anomalies.push(AnomalyEvent {
                timestamp: Local::now(),
                anomaly_type: AnomalyType::WhaleWall,
                severity: (bid_wall_count * 20) as u8,
                confidence: 85,
                price_level: None,
                side: Some(OrderSide::Bid),
                size: Some(bid_wall_volume),
                percentage: Some(bid_wall_volume / features.total_bid_volume * dec!(100)),
                duration_ms: None,
                frequency: None,
                price_impact: None,
                volume_impact: None,
                description: format!("买单鲸鱼墙: {} 个大单, 总 {:.0} USDT", bid_wall_count, bid_wall_volume),
                details: HashMap::new(),
            });
        }

        let mut ask_wall_count = 0;
        let mut ask_wall_volume = Decimal::ZERO;
        for (_price, qty) in book.asks.iter().take(5) {
            if qty > &(features.total_ask_volume * dec!(0.1)) {
                ask_wall_count += 1;
                ask_wall_volume += qty;
            }
        }

        if ask_wall_count >= self.config.whale_wall_min_orders {
            anomalies.push(AnomalyEvent {
                timestamp: Local::now(),
                anomaly_type: AnomalyType::WhaleWall,
                severity: (ask_wall_count * 20) as u8,
                confidence: 85,
                price_level: None,
                side: Some(OrderSide::Ask),
                size: Some(ask_wall_volume),
                percentage: Some(ask_wall_volume / features.total_ask_volume * dec!(100)),
                duration_ms: None,
                frequency: None,
                price_impact: None,
                volume_impact: None,
                description: format!("卖单鲸鱼墙: {} 个大单, 总 {:.0} USDT", ask_wall_count, ask_wall_volume),
                details: HashMap::new(),
            });
        }

        anomalies
    }

    fn detect_complex_patterns(&self) -> Vec<AnomalyEvent> {
        let mut anomalies = Vec::new();

        if let Some(mega_order) = self
            .recent_anomalies
            .iter()
            .find(|anomaly| matches!(anomaly.anomaly_type, AnomalyType::MegaBid | AnomalyType::MegaAsk))
        {
            if let Some(cancel) = self
                .recent_anomalies
                .iter()
                .find(|anomaly| matches!(anomaly.anomaly_type, AnomalyType::RapidCancellation))
            {
                if (cancel.timestamp - mega_order.timestamp).num_milliseconds() < 2000 {
                    anomalies.push(AnomalyEvent {
                        timestamp: Local::now(),
                        anomaly_type: AnomalyType::Spoofing,
                        severity: mega_order.severity.max(cancel.severity),
                        confidence: 70,
                        price_level: mega_order.price_level,
                        side: mega_order.side.clone(),
                        size: mega_order.size,
                        percentage: mega_order.percentage,
                        duration_ms: Some(
                            (cancel.timestamp - mega_order.timestamp).num_milliseconds() as u64
                        ),
                        frequency: None,
                        price_impact: None,
                        volume_impact: None,
                        description: format!(
                            "疑似Spoofing: 大单出现后快速撤销 (间隔{}ms)",
                            (cancel.timestamp - mega_order.timestamp).num_milliseconds()
                        ),
                        details: HashMap::new(),
                    });
                }
            }
        }

        if let Some(liquidity_drop) = self
            .recent_anomalies
            .iter()
            .find(|anomaly| anomaly.anomaly_type == AnomalyType::LiquidityDrop)
        {
            if let Some(price_spike) = self
                .recent_anomalies
                .iter()
                .find(|anomaly| anomaly.anomaly_type == AnomalyType::PriceSpike)
            {
                if (price_spike.timestamp - liquidity_drop.timestamp).num_milliseconds() < 3000 {
                    anomalies.push(AnomalyEvent {
                        timestamp: Local::now(),
                        anomaly_type: AnomalyType::ComplexPattern,
                        severity: liquidity_drop.severity.max(price_spike.severity),
                        confidence: 65,
                        price_level: price_spike.price_level,
                        side: None,
                        size: None,
                        percentage: None,
                        duration_ms: None,
                        frequency: None,
                        price_impact: price_spike.price_impact,
                        volume_impact: liquidity_drop.volume_impact,
                        description: "流动性骤降后价格尖峰 - 可能的大户操纵".to_string(),
                        details: HashMap::new(),
                    });
                }
            }
        }

        anomalies
    }

    fn update_snapshot(&mut self, book: &OrderBook) {
        let (best_bid, best_ask) = book.best_bid_ask().unwrap_or((Decimal::ZERO, Decimal::ZERO));
        let snapshot = AnomalySnapshot {
            timestamp: Local::now(),
            best_bid,
            best_ask,
            mid_price: (best_bid + best_ask) / dec!(2),
            bid_volume: book.bids.values().sum(),
            ask_volume: book.asks.values().sum(),
            bid_depth_10: book.bids.iter().take(10).map(|(_, qty)| qty).sum(),
            ask_depth_10: book.asks.iter().take(10).map(|(_, qty)| qty).sum(),
            total_bid_orders: book.bids.len(),
            total_ask_orders: book.asks.len(),
            large_bid_count: book.bids.iter().filter(|(_, qty)| **qty > dec!(10000)).count() as u32,
            large_ask_count: book.asks.iter().filter(|(_, qty)| **qty > dec!(10000)).count() as u32,
        };
        self.snapshot_history.push_back(snapshot);
    }

    pub(crate) fn record_order_change(
        &mut self,
        price: Decimal,
        quantity: Decimal,
        side: OrderSide,
        change_type: ChangeType,
        prev_quantity: Option<Decimal>,
    ) {
        self.order_history.push_back(OrderChange {
            timestamp: Local::now(),
            price,
            quantity,
            side,
            change_type,
            prev_quantity,
        });
    }

    fn update_stats(&mut self, anomaly: &AnomalyEvent) {
        self.stats.total_events += 1;
        let count = self
            .stats
            .events_by_type
            .entry(anomaly.anomaly_type.clone())
            .or_insert(0);
        *count += 1;

        self.stats.avg_severity = (self.stats.avg_severity * (self.stats.total_events - 1) as f64
            + anomaly.severity as f64)
            / self.stats.total_events as f64;
        self.stats.max_severity = self.stats.max_severity.max(anomaly.severity);

        let now = Local::now();
        self.stats.last_minute_count = self
            .recent_anomalies
            .iter()
            .filter(|anomaly| (now - anomaly.timestamp).num_seconds() < 60)
            .count() as u32;
        self.stats.last_hour_count = self
            .recent_anomalies
            .iter()
            .filter(|anomaly| (now - anomaly.timestamp).num_seconds() < 3600)
            .count() as u32;
    }

    fn cleanup_history(&mut self) {
        let now = Local::now();
        let window_secs = self.config.history_window_secs as i64;

        while let Some(snapshot) = self.snapshot_history.front() {
            if (now - snapshot.timestamp).num_seconds() > window_secs {
                self.snapshot_history.pop_front();
            } else {
                break;
            }
        }

        while let Some(order) = self.order_history.front() {
            if (now - order.timestamp).num_seconds() > window_secs {
                self.order_history.pop_front();
            } else {
                break;
            }
        }

        while let Some(anomaly) = self.recent_anomalies.front() {
            if (now - anomaly.timestamp).num_seconds() > 3600 {
                self.recent_anomalies.pop_front();
            } else {
                break;
            }
        }
    }

    pub fn get_recent_anomalies(&self, limit: usize) -> Vec<AnomalyEvent> {
        self.recent_anomalies.iter().rev().take(limit).cloned().collect()
    }

    pub fn get_anomalies_by_type(&self, anomaly_type: AnomalyType, limit: usize) -> Vec<AnomalyEvent> {
        self.recent_anomalies
            .iter()
            .filter(|anomaly| anomaly.anomaly_type == anomaly_type)
            .rev()
            .take(limit)
            .cloned()
            .collect()
    }

    pub fn get_stats(&self) -> &AnomalyStats {
        &self.stats
    }

    pub fn print_summary(&self) {
        println!("\n{}", "⚠️".repeat(30));
        println!("📊 订单簿异动统计 - {}", Local::now().format("%Y-%m-%d %H:%M:%S"));
        println!("{}", "⚠️".repeat(30));

        println!("\n📈 总体统计:");
        println!("  总异动数: {}", self.stats.total_events);
        println!("  平均严重程度: {:.1}", self.stats.avg_severity);
        println!("  最大严重程度: {}", self.stats.max_severity);
        println!("  最近1分钟: {}", self.stats.last_minute_count);
        println!("  最近1小时: {}", self.stats.last_hour_count);

        println!("\n📋 异动类型分布:");
        let mut sorted: Vec<_> = self.stats.events_by_type.iter().collect();
        sorted.sort_by(|left, right| right.1.cmp(left.1));
        for (anomaly_type, count) in sorted.iter().take(10) {
            let percentage = **count as f64 / self.stats.total_events as f64 * 100.0;
            println!("  {:?}: {} ({:.1}%)", anomaly_type, count, percentage);
        }

        println!("\n🔥 最近异动 (最新5条):");
        for anomaly in self.recent_anomalies.iter().rev().take(5) {
            let severity_emoji = if anomaly.severity >= 80 {
                "🔥"
            } else if anomaly.severity >= 60 {
                "⚠️"
            } else {
                "📌"
            };
            let type_str = format!("{:?}", anomaly.anomaly_type);
            let display_type = if type_str.len() > 10 {
                &type_str[..10]
            } else {
                &type_str
            };

            println!(
                "  {} [{}] {} - 严重度:{}% {}",
                anomaly.timestamp.format("%H:%M:%S"),
                severity_emoji,
                display_type,
                anomaly.severity,
                anomaly.description
            );
        }

        println!("{}", "⚠️".repeat(30));
    }
}
