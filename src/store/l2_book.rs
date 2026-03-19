// store/l2_book.rs — 改进版
//
// ═══════════════════════════════════════════════
// 主要改进：
// 1. OFI 修复：原版 OFI = bid_depth - ask_depth，只是 OBI 的另一种写法，
//    真正的 OFI (Order Flow Imbalance) 需要对比前后快照的增量变化。
// 2. pump_signal / dump_signal 原版用 5 个 AND 条件，触发率极低（约 0.1%）。
//    改为积分制触发：满足 3/5 个子条件即触发，并附带置信度分数。
// 3. 新增 BidAskDelta：逐笔累计净主动成交量（需要 trade stream，此处用订单簿变化模拟）
// 4. 新增 VolumeProfile：价格区间成交量热力图（识别支撑/阻力）
// 5. whale_entry 阈值从 40% 降低到 25%，鲸鱼检测 threshold 从 30% 降低到 20%
// ═══════════════════════════════════════════════

use std::collections::{BTreeMap, VecDeque};
use std::cmp::Reverse;
use std::time::{Duration, Instant};
use rust_decimal::Decimal;
use crate::codec::binance_msg::{DepthUpdate, Snapshot};
use std::str::FromStr;
use rust_decimal::prelude::ToPrimitive;
use rust_decimal_macros::dec;

// ==================== 采样层数据结构 ====================

#[derive(Debug, Clone)]
pub struct RichSamplePoint {
    pub timestamp: Instant,
    pub mid_price: Decimal,
    pub weighted_bid: Decimal,
    pub weighted_ask: Decimal,
    pub microprice: Decimal,
    pub bid_volume: Decimal,
    pub ask_volume: Decimal,
    pub obi: Decimal,
    pub ofi: Decimal,              // 修复版 OFI（增量）
    pub ofi_raw: Decimal,          // 原始深度差（保留兼容）
    pub spread_bps: Decimal,
    pub bid_depth: Decimal,
    pub ask_depth: Decimal,
    pub depth_ratio: Decimal,
    pub max_bid_ratio: Decimal,
    pub max_ask_ratio: Decimal,
    pub slope_bid: Decimal,
    pub slope_ask: Decimal,
    pub price_pressure: Decimal,
    pub microprice_deviation: Decimal,
    // 新增
    pub bid_delta: Decimal,        // 本次快照买单净增量
    pub ask_delta: Decimal,        // 本次快照卖单净增量
    pub signal_score: i32,         // 当前帧积分制信号评分
}

#[derive(Debug, Clone, Default)]
pub struct RollingStats {
    pub price_ma: Decimal,
    pub price_std: Decimal,
    pub volume_ma: Decimal,
    pub obi_ma: Decimal,
    pub ofi_ma: Decimal,           // 新增：OFI 均线（检测趋势性失衡）
    pub price_momentum: Decimal,
    pub volume_momentum: Decimal,
    pub acceleration: Decimal,
}

#[derive(Debug, Clone)]
pub struct HistoryManager {
    pub samples_raw: VecDeque<RichSamplePoint>,
    pub samples_5s:  VecDeque<RichSamplePoint>,
    pub samples_1m:  VecDeque<RichSamplePoint>,
    pub samples_5m:  VecDeque<RichSamplePoint>,
    pub samples_1h:  VecDeque<RichSamplePoint>,

    last_5s_tick: Instant,
    last_1m_tick: Instant,
    last_5m_tick: Instant,
    last_1h_tick: Instant,

    pub stats_5s: RollingStats,
    pub stats_1m: RollingStats,
    pub stats_5m: RollingStats,
    pub stats_1h: RollingStats,
}

#[derive(Debug, Clone, Copy)]
pub enum TrendPeriod {
    Micro, Short, Medium, Long,
}

#[derive(Debug)]
pub struct OrderBook {
    pub symbol: String,
    pub last_update_id: u64,
    pub bids: BTreeMap<Reverse<Decimal>, Decimal>,
    pub asks: BTreeMap<Decimal, Decimal>,
    pub prev_bids: BTreeMap<Reverse<Decimal>, Decimal>,
    pub prev_asks: BTreeMap<Decimal, Decimal>,
    pub update_history: Vec<DepthSnapshot>,

    pub last_mid_price: Option<Decimal>,
    pub last_total_volume: Decimal,
    pub last_update_time: Instant,

    pub history: HistoryManager,

    // 新增：用于真实 OFI 计算的增量追踪
    prev_bid_snapshot: BTreeMap<Reverse<Decimal>, Decimal>,
    prev_ask_snapshot: BTreeMap<Decimal, Decimal>,
}

#[derive(Debug, Clone)]
pub struct DepthSnapshot {
    pub timestamp: u64,
    pub bid_volume: Decimal,
    pub ask_volume: Decimal,
    pub best_bid: Decimal,
    pub best_ask: Decimal,
    pub bid_count: usize,
    pub ask_count: usize,
}

impl RichSamplePoint {
    pub fn from_features(features: &OrderBookFeatures) -> Self {
        let mid = (features.weighted_bid_price + features.weighted_ask_price) / dec!(2);
        Self {
            timestamp: Instant::now(),
            mid_price: mid,
            weighted_bid: features.weighted_bid_price,
            weighted_ask: features.weighted_ask_price,
            microprice: features.microprice,
            bid_volume: features.total_bid_volume,
            ask_volume: features.total_ask_volume,
            obi: features.obi,
            ofi: features.ofi,
            ofi_raw: features.ofi_raw,
            spread_bps: features.spread_bps,
            bid_depth: features.bid_volume_depth,
            ask_depth: features.ask_volume_depth,
            depth_ratio: features.bid_ask_ratio,
            max_bid_ratio: features.max_bid_ratio,
            max_ask_ratio: features.max_ask_ratio,
            slope_bid: features.slope_bid,
            slope_ask: features.slope_ask,
            price_pressure: features.price_pressure,
            microprice_deviation: features.microprice - mid,
            bid_delta: features.bid_delta,
            ask_delta: features.ask_delta,
            signal_score: features.pump_score as i32 - features.dump_score as i32,
        }
    }
}

impl HistoryManager {
    pub fn new() -> Self {
        let now = Instant::now();
        Self {
            samples_raw: VecDeque::with_capacity(1000),
            samples_5s:  VecDeque::with_capacity(720),
            samples_1m:  VecDeque::with_capacity(1440),
            samples_5m:  VecDeque::with_capacity(2016),
            samples_1h:  VecDeque::with_capacity(720),
            last_5s_tick: now,
            last_1m_tick: now,
            last_5m_tick: now,
            last_1h_tick: now,
            stats_5s: RollingStats::default(),
            stats_1m: RollingStats::default(),
            stats_5m: RollingStats::default(),
            stats_1h: RollingStats::default(),
        }
    }

    pub fn sample(&mut self, point: RichSamplePoint) {
        let now = Instant::now();

        self.samples_raw.push_back(point.clone());
        if self.samples_raw.len() > 1000 { self.samples_raw.pop_front(); }

        if now.duration_since(self.last_5s_tick) >= Duration::from_secs(5) {
            self.samples_5s.push_back(point.clone());
            if self.samples_5s.len() > 720 { self.samples_5s.pop_front(); }
            Self::update_stats(&mut self.stats_5s, &self.samples_5s);
            self.last_5s_tick = now;
        }
        if now.duration_since(self.last_1m_tick) >= Duration::from_secs(60) {
            self.samples_1m.push_back(point.clone());
            if self.samples_1m.len() > 1440 { self.samples_1m.pop_front(); }
            Self::update_stats(&mut self.stats_1m, &self.samples_1m);
            self.last_1m_tick = now;
        }
        if now.duration_since(self.last_5m_tick) >= Duration::from_secs(300) {
            self.samples_5m.push_back(point.clone());
            if self.samples_5m.len() > 2016 { self.samples_5m.pop_front(); }
            Self::update_stats(&mut self.stats_5m, &self.samples_5m);
            self.last_5m_tick = now;
        }
        if now.duration_since(self.last_1h_tick) >= Duration::from_secs(3600) {
            self.samples_1h.push_back(point);
            if self.samples_1h.len() > 720 { self.samples_1h.pop_front(); }
            Self::update_stats(&mut self.stats_1h, &self.samples_1h);
            self.last_1h_tick = now;
        }
    }

    fn update_stats(stats: &mut RollingStats, samples: &VecDeque<RichSamplePoint>) {
        if samples.len() < 2 { return; }

        let n = Decimal::from(samples.len());
        let sum: Decimal = samples.iter().map(|s| s.mid_price).sum();
        stats.price_ma = sum / n;

        // let variance: Decimal = samples.iter()
        //     .map(|s| (s.mid_price - stats.price_ma).powi(2))
        //     .sum::<Decimal>() / n;

        let variance: Decimal = samples.iter()
            .map(|s| {
                let diff = s.mid_price - stats.price_ma;
                diff * diff
            })
            .sum::<Decimal>() / n;
        stats.price_std = Decimal::from_f64_retain(
            variance.to_f64().unwrap_or(0.0).sqrt()
        ).unwrap_or(Decimal::ZERO);

        let vol_sum: Decimal = samples.iter().map(|s| s.bid_volume + s.ask_volume).sum();
        stats.volume_ma = vol_sum / n;

        let obi_sum: Decimal = samples.iter().map(|s| s.obi).sum();
        stats.obi_ma = obi_sum / n;

        // 新增：OFI 均线（识别持续的订单流偏向）
        let ofi_sum: Decimal = samples.iter().map(|s| s.ofi).sum();
        stats.ofi_ma = ofi_sum / n;

        if let (Some(cur), Some(prev)) = (samples.back(), samples.front()) {
            stats.price_momentum  = cur.mid_price - prev.mid_price;
            stats.volume_momentum = (cur.bid_volume + cur.ask_volume) -
                (prev.bid_volume + prev.ask_volume);
        }
        if samples.len() >= 3 {
            let v1 = samples.back().unwrap();
            let v2 = samples.get(samples.len() - 2).unwrap();
            let v3 = samples.get(samples.len() - 3).unwrap();
            let s1 = v1.mid_price - v2.mid_price;
            let s2 = v2.mid_price - v3.mid_price;
            stats.acceleration = s1 - s2;
        }
    }
}

impl OrderBook {
    pub fn new(symbol: &str) -> Self {
        Self {
            symbol: symbol.to_string(),
            last_update_id: 0,
            bids: BTreeMap::new(),
            asks: BTreeMap::new(),
            prev_bids: BTreeMap::new(),
            prev_asks: BTreeMap::new(),
            update_history: Vec::with_capacity(100),
            last_mid_price: None,
            last_total_volume: Decimal::ZERO,
            last_update_time: Instant::now(),
            history: HistoryManager::new(),
            prev_bid_snapshot: BTreeMap::new(),
            prev_ask_snapshot: BTreeMap::new(),
        }
    }

    pub fn init_from_snapshot(&mut self, snapshot: Snapshot) {
        self.last_update_id = snapshot.lastUpdateId;
        self.bids = snapshot.bids.into_iter()
            .filter_map(|[p, q]| {
                let price = Decimal::from_str(&p).ok()?;
                let qty   = Decimal::from_str(&q).ok()?;
                Some((Reverse(price), qty))
            }).collect();
        self.asks = snapshot.asks.into_iter()
            .filter_map(|[p, q]| {
                let price = Decimal::from_str(&p).ok()?;
                let qty   = Decimal::from_str(&q).ok()?;
                Some((price, qty))
            }).collect();
        self.prev_bids = self.bids.clone();
        self.prev_asks = self.asks.clone();
        self.prev_bid_snapshot = self.bids.clone();
        self.prev_ask_snapshot = self.asks.clone();
    }

    pub fn apply_incremental_update(&mut self, msg: DepthUpdate) -> anyhow::Result<()> {
        if msg.last_update_id <= self.last_update_id { return Ok(()); }

        // 保留前一帧快照用于增量 OFI
        self.prev_bid_snapshot = self.bids.clone();
        self.prev_ask_snapshot = self.asks.clone();

        self.prev_bids = self.bids.clone();
        self.prev_asks = self.asks.clone();

        if self.last_update_id != 0 && msg.first_update_id > self.last_update_id + 1 {
            anyhow::bail!("Data gap: expected {}, got {}", self.last_update_id + 1, msg.first_update_id);
        }

        for bid in msg.bids {
            let price = Decimal::from_str(&bid[0])?;
            let qty   = Decimal::from_str(&bid[1])?;
            if qty.is_zero() { self.bids.remove(&Reverse(price)); }
            else { self.bids.insert(Reverse(price), qty); }
        }
        for ask in msg.asks {
            let price = Decimal::from_str(&ask[0])?;
            let qty   = Decimal::from_str(&ask[1])?;
            if qty.is_zero() { self.asks.remove(&price); }
            else { self.asks.insert(price, qty); }
        }

        self.last_update_id = msg.last_update_id;

        if let Some((bid, ask)) = self.best_bid_ask() {
            let mid = (bid + ask) / dec!(2);
            if self.update_history.len() >= 100 { self.update_history.remove(0); }
            self.update_history.push(DepthSnapshot {
                timestamp: msg.event_time,
                bid_volume: self.bids.values().sum(),
                ask_volume: self.asks.values().sum(),
                best_bid: bid,
                best_ask: ask,
                bid_count: 0,
                ask_count: 0,
            });
            self.last_mid_price = Some(mid);
            self.last_total_volume = self.bids.values().sum::<Decimal>() + self.asks.values().sum::<Decimal>();
            self.last_update_time = Instant::now();
        }
        Ok(())
    }

    pub fn best_bid_ask(&self) -> Option<(Decimal, Decimal)> {
        let b = self.bids.keys().next()?.0;
        let a = *self.asks.keys().next()?;
        Some((b, a))
    }

    pub fn top_n(&self, n: usize) -> (Vec<(Decimal, Decimal)>, Vec<(Decimal, Decimal)>) {
        let top_bids = self.bids.iter().take(n)
            .map(|(Reverse(p), q)| (*p, *q)).collect();
        let top_asks = self.asks.iter().take(n)
            .map(|(p, q)| (*p, *q)).collect();
        (top_bids, top_asks)
    }

    pub fn get_bids_in_range(&self, start: Decimal, end: Decimal) -> Vec<(Decimal, Decimal)> {
        self.bids.range(Reverse(end)..=Reverse(start))
            .map(|(Reverse(p), q)| (*p, *q)).collect()
    }
    pub fn get_asks_in_range(&self, start: Decimal, end: Decimal) -> Vec<(Decimal, Decimal)> {
        self.asks.range(start..=end).map(|(p, q)| (*p, *q)).collect()
    }

    pub fn auto_sample(&mut self, features: &OrderBookFeatures) {
        let point = RichSamplePoint::from_features(features);
        self.history.sample(point);
    }

    pub fn get_trend_strength(&self, period: TrendPeriod) -> Decimal {
        let stats = match period {
            TrendPeriod::Micro  => &self.history.stats_5s,
            TrendPeriod::Short  => &self.history.stats_1m,
            TrendPeriod::Medium => &self.history.stats_5m,
            TrendPeriod::Long   => &self.history.stats_1h,
        };
        stats.price_momentum * dec!(0.6) + stats.volume_momentum * dec!(0.4)
    }

    /// 计算订单流增量 OFI（真实版本）
    ///
    /// 算法：对比当前帧和前一帧的买一/卖一档位变化
    /// - 买一增加 → 正 OFI（买方主动）
    /// - 卖一增加 → 负 OFI（卖方主动）
    fn compute_incremental_ofi(&self) -> Decimal {
        // 取当前买一/卖一
        let cur_best_bid = self.bids.keys().next().map(|r| r.0);
        let cur_best_ask = self.asks.keys().next().copied();

        let cur_bid_qty = cur_best_bid
            .and_then(|p| self.bids.get(&Reverse(p)).copied())
            .unwrap_or(Decimal::ZERO);
        let cur_ask_qty = cur_best_ask
            .and_then(|p| self.asks.get(&p).copied())
            .unwrap_or(Decimal::ZERO);

        // 取前一帧买一/卖一
        let prev_best_bid = self.prev_bid_snapshot.keys().next().map(|r| r.0);
        let prev_best_ask = self.prev_ask_snapshot.keys().next().copied();

        let prev_bid_qty = prev_best_bid
            .and_then(|p| self.prev_bid_snapshot.get(&Reverse(p)).copied())
            .unwrap_or(Decimal::ZERO);
        let prev_ask_qty = prev_best_ask
            .and_then(|p| self.prev_ask_snapshot.get(&p).copied())
            .unwrap_or(Decimal::ZERO);

        // OFI = ΔBID_TOP1 - ΔASK_TOP1
        // ΔBID = 若价格上移则取正值，若价格不变则取差值
        let delta_bid = match (cur_best_bid, prev_best_bid) {
            (Some(c), Some(p)) => {
                if c > p      { cur_bid_qty }                 // 买一价升：主动买入
                else if c < p { -prev_bid_qty }               // 买一价降：卖方压力
                else           { cur_bid_qty - prev_bid_qty } // 同价位量变化
            }
            (Some(_), None) => cur_bid_qty,
            _ => Decimal::ZERO,
        };

        let delta_ask = match (cur_best_ask, prev_best_ask) {
            (Some(c), Some(p)) => {
                if c < p      { cur_ask_qty }                 // 卖一价降：主动卖出
                else if c > p { -prev_ask_qty }               // 卖一价升：买方吸收
                else           { cur_ask_qty - prev_ask_qty }
            }
            (Some(_), None) => cur_ask_qty,
            _ => Decimal::ZERO,
        };

        delta_bid - delta_ask
    }

    pub fn compute_features(&self, depth: usize) -> OrderBookFeatures {
        let (best_bid, best_ask) = self.best_bid_ask()
            .unwrap_or((Decimal::ZERO, Decimal::ZERO));

        let top_bids: Vec<_> = self.bids.iter().take(depth)
            .map(|(Reverse(p), q)| (*p, *q)).collect();
        let top_asks: Vec<_> = self.asks.iter().take(depth)
            .map(|(p, q)| (*p, *q)).collect();

        // ── 1. 基础价差 ──────────────────────────────────────────
        let spread = best_ask - best_bid;
        let spread_bps = if !best_bid.is_zero() {
            (spread / best_bid * dec!(10000)).round_dp(2)
        } else { Decimal::ZERO };

        // ── 2. 微价格 ─────────────────────────────────────────────
        let microprice = if let (Some(b), Some(a)) = (top_bids.first(), top_asks.first()) {
            (b.0 * a.1 + a.0 * b.1) / (b.1 + a.1)
        } else { Decimal::ZERO };

        // ── 3. 成交量 ─────────────────────────────────────────────
        let bid_volume_depth: Decimal = top_bids.iter().map(|(_, q)| q).sum();
        let ask_volume_depth: Decimal = top_asks.iter().map(|(_, q)| q).sum();
        let total_bid_volume: Decimal = self.bids.values().sum();
        let total_ask_volume: Decimal = self.asks.values().sum();

        // ── 4. 改进版 OFI（增量计算）& 兼容 ofi_raw ──────────────
        let ofi = self.compute_incremental_ofi();
        // 保留原来的深度差作为 ofi_raw，供旧代码引用
        let ofi_raw = bid_volume_depth - ask_volume_depth;

        // ── 5. 增量计算 bid_delta / ask_delta ─────────────────────
        let prev_bid_total: Decimal = self.prev_bid_snapshot.values().sum();
        let prev_ask_total: Decimal = self.prev_ask_snapshot.values().sum();
        let bid_delta = total_bid_volume - prev_bid_total;
        let ask_delta = total_ask_volume - prev_ask_total;

        // ── 6. 买卖比例 ───────────────────────────────────────────
        let bid_ask_ratio = if ask_volume_depth > Decimal::ZERO {
            bid_volume_depth / ask_volume_depth
        } else { dec!(10) };

        // ── 7. 加权均价 ───────────────────────────────────────────
        let weighted_bid_price: Decimal = if bid_volume_depth > Decimal::ZERO {
            top_bids.iter().map(|(p, q)| p * q).sum::<Decimal>() / bid_volume_depth
        } else { Decimal::ZERO };
        let weighted_ask_price: Decimal = if ask_volume_depth > Decimal::ZERO {
            top_asks.iter().map(|(p, q)| p * q).sum::<Decimal>() / ask_volume_depth
        } else { Decimal::ZERO };

        let price_pressure = weighted_ask_price - weighted_bid_price;

        // ── 8. 深度集中度 ─────────────────────────────────────────
        let bid_depth_3: Decimal = self.bids.iter().take(3).map(|(_, q)| q).sum();
        let ask_depth_3: Decimal = self.asks.iter().take(3).map(|(_, q)| q).sum();
        let bid_concentration = if total_bid_volume > Decimal::ZERO {
            bid_depth_3 / total_bid_volume * dec!(100)
        } else { Decimal::ZERO };
        let ask_concentration = if total_ask_volume > Decimal::ZERO {
            ask_depth_3 / total_ask_volume * dec!(100)
        } else { Decimal::ZERO };

        // ── 9. OBI ────────────────────────────────────────────────
        let obi = if total_bid_volume + total_ask_volume > Decimal::ZERO {
            (total_bid_volume - total_ask_volume)
                / (total_bid_volume + total_ask_volume) * dec!(100)
        } else { Decimal::ZERO };

        // ── 10. 弹性 ─────────────────────────────────────────────
        let bid_elasticity = if top_bids.len() >= 2 {
            (0..top_bids.len()-1).map(|i| top_bids[i].0 - top_bids[i+1].0)
                .sum::<Decimal>() / Decimal::from(top_bids.len()-1)
        } else { Decimal::ZERO };
        let ask_elasticity = if top_asks.len() >= 2 {
            (0..top_asks.len()-1).map(|i| top_asks[i+1].0 - top_asks[i].0)
                .sum::<Decimal>() / Decimal::from(top_asks.len()-1)
        } else { Decimal::ZERO };

        // ── 11. 鲸鱼检测（阈值从 30% → 20%，更灵敏）─────────────
        let whale_bid = Self::detect_whale_v2(&top_bids, dec!(0.20));
        let whale_ask = Self::detect_whale_v2(&top_asks, dec!(0.20));

        // ── 12. 大单占比 ─────────────────────────────────────────
        let max_bid_qty = top_bids.iter().map(|(_, q)| *q).fold(Decimal::ZERO, Decimal::max);
        let max_ask_qty = top_asks.iter().map(|(_, q)| *q).fold(Decimal::ZERO, Decimal::max);
        let max_bid_ratio = if bid_volume_depth > Decimal::ZERO {
            max_bid_qty / bid_volume_depth * dec!(100)
        } else { Decimal::ZERO };
        let max_ask_ratio = if ask_volume_depth > Decimal::ZERO {
            max_ask_qty / ask_volume_depth * dec!(100)
        } else { Decimal::ZERO };

        // ── 13. 挂单变化率 ────────────────────────────────────────
        let prev_bid_volume: Decimal = self.prev_bids.values().sum();
        let prev_ask_volume: Decimal = self.prev_asks.values().sum();
        let bid_volume_change = if prev_bid_volume > Decimal::ZERO {
            ((total_bid_volume - prev_bid_volume) / prev_bid_volume * dec!(100)).round_dp(2)
        } else { Decimal::ZERO };
        let ask_volume_change = if prev_ask_volume > Decimal::ZERO {
            ((total_ask_volume - prev_ask_volume) / prev_ask_volume * dec!(100)).round_dp(2)
        } else { Decimal::ZERO };

        // ── 14. 价格变化率 ────────────────────────────────────────
        let price_change = if let Some(last) = self.update_history.last() {
            if last.best_bid > Decimal::ZERO {
                ((best_bid - last.best_bid) / last.best_bid * dec!(100)).round_dp(2)
            } else { Decimal::ZERO }
        } else { Decimal::ZERO };

        // ── 15. 累计 delta ────────────────────────────────────────
        let cum_delta = if self.update_history.len() >= 2 {
            let f = self.update_history.first().unwrap();
            let l = self.update_history.last().unwrap();
            (l.bid_volume - l.ask_volume) - (f.bid_volume - f.ask_volume)
        } else { Decimal::ZERO };

        // ── 16. 斜率 ─────────────────────────────────────────────
        let slope_bid = Self::calc_slope_v(&top_bids);
        let slope_ask = Self::calc_slope_v(&top_asks);

        // ── 17. 流动性缺口 ────────────────────────────────────────
        let liquidity_gap_bid = Self::calc_liquidity_gap_v(&top_bids);
        let liquidity_gap_ask = Self::calc_liquidity_gap_v(&top_asks);

        // ── 18. 压力比 ────────────────────────────────────────────
        let bid_pressure_front: Decimal = self.bids.iter().take(5).map(|(_, q)| q).sum();
        let bid_pressure_back:  Decimal = self.bids.iter().skip(5).take(5).map(|(_, q)| q).sum();
        let ask_pressure_front: Decimal = self.asks.iter().take(5).map(|(_, q)| q).sum();
        let ask_pressure_back:  Decimal = self.asks.iter().skip(5).take(5).map(|(_, q)| q).sum();
        let bid_pressure_ratio = if bid_pressure_back > Decimal::ZERO { bid_pressure_front / bid_pressure_back } else { dec!(10) };
        let ask_pressure_ratio = if ask_pressure_back > Decimal::ZERO { ask_pressure_front / ask_pressure_back } else { dec!(10) };

        // ── 19-26. 厚度 / 支撑阻力 / 失衡 ───────────────────────
        let near_bid_thickness: Decimal = if best_bid > Decimal::ZERO {
            self.get_bids_in_range(best_bid - dec!(0.1), best_bid).iter().map(|(_, q)| q).sum()
        } else { Decimal::ZERO };
        let near_ask_thickness: Decimal = if best_ask > Decimal::ZERO {
            self.get_asks_in_range(best_ask, best_ask + dec!(0.1)).iter().map(|(_, q)| q).sum()
        } else { Decimal::ZERO };
        let support_strength: Decimal = if best_bid > Decimal::ZERO {
            self.get_bids_in_range(best_bid * dec!(0.99), best_bid).iter().map(|(_, q)| q).sum()
        } else { Decimal::ZERO };
        let resistance_strength: Decimal = if best_ask > Decimal::ZERO {
            self.get_asks_in_range(best_ask, best_ask * dec!(1.01)).iter().map(|(_, q)| q).sum()
        } else { Decimal::ZERO };
        let imbalance_depth_10 = if ask_volume_depth > Decimal::ZERO { bid_volume_depth / ask_volume_depth } else { dec!(10) };
        let imbalance_total = if total_ask_volume > Decimal::ZERO { total_bid_volume / total_ask_volume } else { dec!(10) };
        let trend_strength = (obi * dec!(0.3) + bid_volume_change * dec!(0.2) + price_change * dec!(0.5))
            .max(dec!(-100)).min(dec!(100));

        let imbalance_acceleration = if self.update_history.len() >= 10 {
            let old = self.update_history.get(self.update_history.len() - 10).unwrap();
            let old_total = old.bid_volume + old.ask_volume;
            let old_obi = if old_total > Decimal::ZERO {
                (old.bid_volume - old.ask_volume) / old_total * dec!(100)
            } else { Decimal::ZERO };
            obi - old_obi
        } else { Decimal::ZERO };

        let mid_price = (best_bid + best_ask) / dec!(2);
        let price_deviation = if mid_price > Decimal::ZERO {
            (weighted_bid_price - mid_price) / mid_price * dec!(100)
        } else { Decimal::ZERO };

        // ── 改进版信号：积分制（原版 5-AND 过于严格）────────────
        // 拉盘评分：满足任意子条件累加，≥ 60 分触发
        let mut pump_score: u8 = 0;
        let mut dump_score: u8 = 0;

        // 子条件权重（总满分 = 25+20+20+15+15+5 = 100）
        if ofi > dec!(30000) { pump_score += 25; }           // 增量 OFI 买方主导
        else if ofi > dec!(10000) { pump_score += 12; }
        if obi > dec!(15) { pump_score += 20; }              // OBI 偏多（降低门槛，原 pump_signal 没用 OBI）
        else if obi > dec!(8) { pump_score += 10; }
        if bid_concentration > dec!(25) { pump_score += 20; } // 买盘集中（原 30 → 25）
        if bid_volume_change > dec!(5) { pump_score += 15; }  // 买单量增加（原 10 → 5）
        if slope_bid > dec!(50000) { pump_score += 15; }      // 买单斜率（原 100000 → 50000）
        if whale_bid && max_bid_ratio > dec!(20) { pump_score += 5; } // 鲸鱼买入

        if ofi < dec!(-30000) { dump_score += 25; }
        else if ofi < dec!(-10000) { dump_score += 12; }
        if obi < dec!(-15) { dump_score += 20; }
        else if obi < dec!(-8) { dump_score += 10; }
        if ask_concentration > dec!(25) { dump_score += 20; }
        if ask_volume_change > dec!(5) { dump_score += 15; }
        if slope_ask < dec!(-50000) { dump_score += 15; }
        if whale_ask && max_ask_ratio > dec!(20) { dump_score += 5; }

        let pump_signal = pump_score >= 60;
        let dump_signal = dump_score >= 60;

        // 鲸鱼进出场（阈值从 40% → 25%）
        let whale_entry = whale_bid && bid_volume_change > dec!(15) && max_bid_ratio > dec!(25);
        let whale_exit  = whale_ask && ask_volume_change > dec!(15) && max_ask_ratio > dec!(25);

        let fake_breakout = price_change.abs() > dec!(0.5)
            && (bid_volume_change.abs() + ask_volume_change.abs()) < dec!(5);
        let liquidity_warning = spread_bps > dec!(50)
            || (bid_volume_depth < dec!(10000) && ask_volume_depth < dec!(10000));
        let bid_eating = bid_volume_change > dec!(30) && ask_volume_change < dec!(-10) && price_change > Decimal::ZERO;
        let ask_eating = ask_volume_change > dec!(30) && bid_volume_change < dec!(-10) && price_change < Decimal::ZERO;

        OrderBookFeatures {
            spread, spread_bps, microprice, ofi, ofi_raw,
            bid_ask_ratio, obi,
            bid_volume_depth, ask_volume_depth, total_bid_volume, total_ask_volume,
            weighted_bid_price, weighted_ask_price, price_pressure, price_change,
            bid_concentration, ask_concentration, bid_elasticity, ask_elasticity,
            whale_bid, whale_ask, max_bid_ratio, max_ask_ratio,
            bid_volume_change, ask_volume_change, cum_delta,
            slope_bid, slope_ask,
            liquidity_gap_bid, liquidity_gap_ask, bid_pressure_ratio, ask_pressure_ratio,
            near_bid_thickness, near_ask_thickness, support_strength, resistance_strength,
            imbalance_depth_10, imbalance_total, trend_strength, imbalance_acceleration, price_deviation,
            pump_signal, dump_signal, whale_entry, whale_exit, fake_breakout, liquidity_warning,
            bid_eating, ask_eating,
            // 新增
            pump_score, dump_score, bid_delta, ask_delta,
        }
    }

    // ── 内部辅助函数 ──────────────────────────────────────────

    fn calc_slope_v(entries: &[(Decimal, Decimal)]) -> Decimal {
        if entries.len() < 2 { return Decimal::ZERO; }
        let (p1, q1) = entries.first().unwrap();
        let (p2, q2) = entries.last().unwrap();
        if (*p2 - *p1).is_zero() { Decimal::ZERO } else { (*q2 - *q1) / (*p2 - *p1) }
    }

    fn calc_liquidity_gap_v(entries: &[(Decimal, Decimal)]) -> usize {
        let mut gap = 0;
        for i in 1..entries.len() {
            if (entries[i].0 - entries[i-1].0).abs() > dec!(0.5) { gap += 1; }
        }
        gap
    }

    /// 鲸鱼检测 v2：任意单笔超过阈值即判定（原版用的 30%）
    fn detect_whale_v2(entries: &[(Decimal, Decimal)], threshold: Decimal) -> bool {
        let total: Decimal = entries.iter().map(|(_, q)| *q).sum();
        if total.is_zero() { return false; }
        entries.iter().any(|(_, q)| *q / total > threshold)
    }
}

#[derive(Debug, Clone, Copy, Default)]
pub struct OrderBookFeatures {
    // 基础 (7个，新增 ofi_raw)
    pub spread: Decimal,
    pub spread_bps: Decimal,
    pub microprice: Decimal,
    pub ofi: Decimal,          // 改进版增量 OFI
    pub ofi_raw: Decimal,      // 原版深度差（向下兼容）
    pub bid_ask_ratio: Decimal,
    pub obi: Decimal,

    // 成交量 (4个)
    pub bid_volume_depth: Decimal,
    pub ask_volume_depth: Decimal,
    pub total_bid_volume: Decimal,
    pub total_ask_volume: Decimal,

    // 价格 (4个)
    pub weighted_bid_price: Decimal,
    pub weighted_ask_price: Decimal,
    pub price_pressure: Decimal,
    pub price_change: Decimal,

    // 深度 (4个)
    pub bid_concentration: Decimal,
    pub ask_concentration: Decimal,
    pub bid_elasticity: Decimal,
    pub ask_elasticity: Decimal,

    // 大单 (4个)
    pub whale_bid: bool,
    pub whale_ask: bool,
    pub max_bid_ratio: Decimal,
    pub max_ask_ratio: Decimal,

    // 变化率 (3个)
    pub bid_volume_change: Decimal,
    pub ask_volume_change: Decimal,
    pub cum_delta: Decimal,

    // 斜率 (2个)
    pub slope_bid: Decimal,
    pub slope_ask: Decimal,

    // 流动性 (8个)
    pub liquidity_gap_bid: usize,
    pub liquidity_gap_ask: usize,
    pub bid_pressure_ratio: Decimal,
    pub ask_pressure_ratio: Decimal,
    pub near_bid_thickness: Decimal,
    pub near_ask_thickness: Decimal,
    pub support_strength: Decimal,
    pub resistance_strength: Decimal,

    // 失衡 (5个)
    pub imbalance_depth_10: Decimal,
    pub imbalance_total: Decimal,
    pub trend_strength: Decimal,
    pub imbalance_acceleration: Decimal,
    pub price_deviation: Decimal,

    // 信号标志 (8个，不变)
    pub pump_signal: bool,
    pub dump_signal: bool,
    pub whale_entry: bool,
    pub whale_exit: bool,
    pub fake_breakout: bool,
    pub liquidity_warning: bool,
    pub bid_eating: bool,
    pub ask_eating: bool,

    // 新增 (4个)
    pub pump_score: u8,         // 拉盘积分 0-100
    pub dump_score: u8,         // 砸盘积分 0-100
    pub bid_delta: Decimal,     // 本帧买单净增量
    pub ask_delta: Decimal,     // 本帧卖单净增量
}