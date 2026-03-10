use std::collections::{BTreeMap, VecDeque};
use std::cmp::Reverse;
use std::time::{Duration, Instant};
use rust_decimal::Decimal;
use crate::codec::binance_msg::{DepthUpdate, Snapshot};
use std::str::FromStr;
use rust_decimal::prelude::ToPrimitive;
use rust_decimal_macros::dec;
use crate::analysis::algorithms::AlphaSignal::Sell;
// ==================== 采样层数据结构 ====================

// 增强的采样点，包含更多微观结构特征
#[derive(Debug, Clone)]
pub struct RichSamplePoint {
    pub timestamp: Instant,
    // 价格维度
    pub mid_price: Decimal,
    pub weighted_bid: Decimal,
    pub weighted_ask: Decimal,
    pub microprice: Decimal,

    // 订单簿维度
    pub bid_volume: Decimal,
    pub ask_volume: Decimal,
    pub obi: Decimal,              // 订单簿失衡
    pub ofi: Decimal,               // 订单流失衡
    pub spread_bps: Decimal,        // 买卖价差

    // 流动性维度
    pub bid_depth: Decimal,
    pub ask_depth: Decimal,
    pub depth_ratio: Decimal,

    // 大单维度
    pub max_bid_ratio: Decimal,
    pub max_ask_ratio: Decimal,

    // 斜率维度
    pub slope_bid: Decimal,
    pub slope_ask: Decimal,

    // 衍生特征
    pub price_pressure: Decimal,
    pub microprice_deviation: Decimal,
}

// 滚动统计（避免每次重新计算）
#[derive(Debug, Clone, Default)]
pub struct RollingStats {
    pub price_ma: Decimal,        // 移动平均
    pub price_std: Decimal,        // 标准差
    pub volume_ma: Decimal,        // 成交量均线
    pub obi_ma: Decimal,           // OBI 均线
    pub price_momentum: Decimal,    // 价格动量
    pub volume_momentum: Decimal,   // 成交量动量
    pub acceleration: Decimal,      // 加速度
}

// 采样特征
#[derive(Debug, Clone)]
pub struct HistoryManager {
    // 多级采样桶
    pub samples_raw: VecDeque<RichSamplePoint>,      // 原始流（100ms级，用于实时）
    pub samples_5s: VecDeque<RichSamplePoint>,       // 5秒级（用于微观加速）
    pub samples_1m: VecDeque<RichSamplePoint>,       // 1分钟级（用于趋势）
    pub samples_5m: VecDeque<RichSamplePoint>,       // 5分钟级（用于中期）
    pub samples_1h: VecDeque<RichSamplePoint>,       // 1小时级（用于宏观）

    // 采样计时器
    last_5s_tick: Instant,
    last_1m_tick: Instant,
    last_5m_tick: Instant,
    last_1h_tick: Instant,

    // 聚合统计（预计算加速）
    pub stats_5s: RollingStats,
    pub stats_1m: RollingStats,
    pub stats_5m: RollingStats,
    pub stats_1h: RollingStats,
}

// 趋势周期枚举
#[derive(Debug, Clone, Copy)]
pub enum TrendPeriod {
    Micro,   // 5秒级（瞬时）
    Short,   // 1分钟级（短期）
    Medium,  // 5分钟级（中期）
    Long,    // 1小时级（长期）
}

#[derive(Debug)]
#[allow(dead_code)]
pub struct OrderBook {
    pub symbol: String,
    pub last_update_id: u64,
    pub bids: BTreeMap<Reverse<Decimal>, Decimal>,
    pub asks: BTreeMap<Decimal, Decimal>,
    // 历史数据存储，用于计算变化趋势
    pub prev_bids: BTreeMap<Reverse<Decimal>, Decimal>,
    pub prev_asks: BTreeMap<Decimal, Decimal>,
    pub update_history: Vec<DepthSnapshot>,

    pub last_mid_price: Option<Decimal>,
    pub last_total_volume: Decimal,
    pub last_update_time: Instant,

    // 新增：多级采样历史管理器
    pub history: HistoryManager,
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
        }
    }
}

impl HistoryManager {
    pub fn new() -> Self {
        Self {
            // 容量配置：原始采样保存最近1000条（约100秒）
            samples_raw: VecDeque::with_capacity(1000),
            // 5秒采样保存1小时
            samples_5s: VecDeque::with_capacity(720),
            // 1分钟采样保存24小时
            samples_1m: VecDeque::with_capacity(1440),
            // 5分钟采样保存7天
            samples_5m: VecDeque::with_capacity(2016),
            // 1小时采样保存30天
            samples_1h: VecDeque::with_capacity(720),

            last_5s_tick: Instant::now(),
            last_1m_tick: Instant::now(),
            last_5m_tick: Instant::now(),
            last_1h_tick: Instant::now(),

            stats_5s: RollingStats::default(),
            stats_1m: RollingStats::default(),
            stats_5m: RollingStats::default(),
            stats_1h: RollingStats::default(),
        }
    }

    // 核心采样方法
    pub fn sample(&mut self, point: RichSamplePoint) {
        let now = Instant::now();

        // 1. 始终保存原始采样
        self.samples_raw.push_back(point.clone());
        if self.samples_raw.len() > 1000 {
            self.samples_raw.pop_front();
        }

        // 2. 5秒采样
        if now.duration_since(self.last_5s_tick) >= Duration::from_secs(5) {
            self.samples_5s.push_back(point.clone());
            if self.samples_5s.len() > 720 {
                self.samples_5s.pop_front();
            }
            Self::update_stats(&mut self.stats_5s, &self.samples_5s);
            self.last_5s_tick = now;
        }

        // 3. 1分钟采样
        if now.duration_since(self.last_1m_tick) >= Duration::from_secs(60) {
            self.samples_1m.push_back(point.clone());
            if self.samples_1m.len() > 1440 {
                self.samples_1m.pop_front();
            }
            Self::update_stats(&mut self.stats_1m, &self.samples_1m);
            self.last_1m_tick = now;
        }

        // 4. 5分钟采样
        if now.duration_since(self.last_5m_tick) >= Duration::from_secs(300) {
            self.samples_5m.push_back(point.clone());
            if self.samples_5m.len() > 2016 {
                self.samples_5m.pop_front();
            }
            Self::update_stats(&mut self.stats_5m, &self.samples_5m);
            self.last_5m_tick = now;
        }

        // 5. 1小时采样
        if now.duration_since(self.last_1h_tick) >= Duration::from_secs(3600) {
            self.samples_1h.push_back(point);
            if self.samples_1h.len() > 720 {
                self.samples_1h.pop_front();
            }
            Self::update_stats(&mut self.stats_1h, &self.samples_1h);
            self.last_1h_tick = now;
        }
    }

    // 更新滚动统计
    fn update_stats(stats: &mut RollingStats, samples: &VecDeque<RichSamplePoint>){
    // fn update_stats(&self, stats: &mut RollingStats, samples: &VecDeque<RichSamplePoint>) {
        if samples.len() < 2 { return; }

        // 计算价格移动平均
        let sum: Decimal = samples.iter().map(|s| s.mid_price).sum();
        stats.price_ma = sum / Decimal::from(samples.len());

        // 计算标准差
        let variance: Decimal = samples.iter()
            .map(|s| (s.mid_price - stats.price_ma) * (s.mid_price - stats.price_ma))
            .sum::<Decimal>() / Decimal::from(samples.len());
        stats.price_std = Decimal::from_f64_retain(variance.to_f64().unwrap_or(0.0).sqrt()).unwrap_or(Decimal::ZERO);
        // 成交量均线
        let vol_sum: Decimal = samples.iter().map(|s| s.bid_volume + s.ask_volume).sum();
        stats.volume_ma = vol_sum / Decimal::from(samples.len());

        // OBI均线
        let obi_sum: Decimal = samples.iter().map(|s| s.obi).sum();
        stats.obi_ma = obi_sum / Decimal::from(samples.len());

        // 价格动量
        if let (Some(current), Some(prev)) = (samples.back(), samples.front()) {
            stats.price_momentum = current.mid_price - prev.mid_price;
            stats.volume_momentum = (current.bid_volume + current.ask_volume) -
                (prev.bid_volume + prev.ask_volume);
        }

        // 加速度（需要至少3个点）
        if samples.len() >= 3 {
            let v1 = samples.back().unwrap();
            let v2 = samples.get(samples.len() - 2).unwrap();
            let v3 = samples.get(samples.len() - 3).unwrap();

            let speed_now = v1.mid_price - v2.mid_price;
            let speed_prev = v2.mid_price - v3.mid_price;
            stats.acceleration = speed_now - speed_prev;
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
        }
    }

    // 使用 REST API 的快照初始化
    pub fn init_from_snapshot(&mut self, snapshot: Snapshot) {
        self.last_update_id = snapshot.lastUpdateId;

        // 解析 bids
        self.bids = snapshot.bids
            .into_iter()
            .filter_map(|[p, q]| {
                if let (Ok(price), Ok(qty)) = (Decimal::from_str(&p), Decimal::from_str(&q)) {
                    Some((Reverse(price), qty))
                } else {
                    None
                }
            })
            .collect();

        // 解析 asks
        self.asks = snapshot.asks
            .into_iter()
            .filter_map(|[p, q]| {
                if let (Ok(price), Ok(qty)) = (Decimal::from_str(&p), Decimal::from_str(&q)) {
                    Some((price, qty))
                } else {
                    None
                }
            })
            .collect();

        // 初始化历史数据
        self.prev_bids = self.bids.clone();
        self.prev_asks = self.asks.clone();
    }

    pub fn apply_incremental_update(&mut self, msg: DepthUpdate) -> anyhow::Result<()> {
        if msg.last_update_id <= self.last_update_id {
            return Ok(());
        }

        // 1. 在更新前，将当前状态存入 prev，用于计算 OFI 和变化率
        self.prev_bids = self.bids.clone();
        self.prev_asks = self.asks.clone();

        // 2. 数据连续性检查
        if self.last_update_id != 0 && msg.first_update_id > self.last_update_id + 1 {
            anyhow::bail!("Data gap: expected {}, got {}", self.last_update_id + 1, msg.first_update_id);
        }

        // 3. 更新 BTreeMap (Bids/Asks)
        for bid in msg.bids {
            let price = Decimal::from_str(&bid[0])?;
            let qty = Decimal::from_str(&bid[1])?;
            if qty.is_zero() { self.bids.remove(&Reverse(price)); }
            else { self.bids.insert(Reverse(price), qty); }
        }
        for ask in msg.asks {
            let price = Decimal::from_str(&ask[0])?;
            let qty = Decimal::from_str(&ask[1])?;
            if qty.is_zero() { self.asks.remove(&price); }
            else { self.asks.insert(price, qty); }
        }

        self.last_update_id = msg.last_update_id;

        // 4. 更新历史记录与中间状态
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
        let top_bids: Vec<(Decimal, Decimal)> = self.bids
            .iter()
            .take(n)
            .map(|(Reverse(price), qty)| (*price, *qty))
            .collect();

        let top_asks: Vec<(Decimal, Decimal)> = self.asks
            .iter()
            .take(n)
            .map(|(price, qty)| (*price, *qty))
            .collect();

        (top_bids, top_asks)
    }

    // 范围查询修复版
    pub fn get_bids_in_range(&self, start: Decimal, end: Decimal) -> Vec<(Decimal, Decimal)> {
        let reverse_start = Reverse(end);
        let reverse_end = Reverse(start);
        self.bids
            .range(reverse_start..=reverse_end)
            .map(|(Reverse(p), q)| (*p, *q))
            .collect()
    }

    pub fn get_bids_from_price(&self, price: Decimal) -> Vec<(Decimal, Decimal)> {
        let reverse_start = Reverse(price);
        let reverse_end = Reverse(Decimal::MIN);
        self.bids
            .range(reverse_start..=reverse_end)
            .map(|(Reverse(p), q)| (*p, *q))
            .collect()
    }

    pub fn get_bids_to_price(&self, price: Decimal) -> Vec<(Decimal, Decimal)> {
        let reverse_start = Reverse(Decimal::MAX);
        let reverse_end = Reverse(price);
        self.bids
            .range(reverse_start..=reverse_end)
            .map(|(Reverse(p), q)| (*p, *q))
            .collect()
    }

    pub fn get_asks_in_range(&self, start: Decimal, end: Decimal) -> Vec<(Decimal, Decimal)> {
        self.asks.range(start..=end).map(|(p, q)| (*p, *q)).collect()
    }

    pub fn get_asks_from_price(&self, price: Decimal) -> Vec<(Decimal, Decimal)> {
        self.asks.range(price..=Decimal::MAX).map(|(p, q)| (*p, *q)).collect()
    }

    pub fn get_asks_to_price(&self, price: Decimal) -> Vec<(Decimal, Decimal)> {
        self.asks.range(Decimal::MIN..=price).map(|(p, q)| (*p, *q)).collect()
    }

    // 自动采样
    pub fn auto_sample(&mut self, features: &OrderBookFeatures) {
        let point = RichSamplePoint::from_features(features);
        self.history.sample(point);
    }

    // 获取指定周期的趋势强度
    pub fn get_trend_strength(&self, period: TrendPeriod) -> Decimal {
        let stats = match period {
            TrendPeriod::Micro => &self.history.stats_5s,
            TrendPeriod::Short => &self.history.stats_1m,
            TrendPeriod::Medium => &self.history.stats_5m,
            TrendPeriod::Long => &self.history.stats_1h,
        };

        // 结合价格动量和成交量动量
        stats.price_momentum * dec!(0.6) + stats.volume_momentum * dec!(0.4)
    }

    /// 计算盘口特征（保留全部30个指标逻辑）
    pub fn compute_features(&self, depth: usize) -> OrderBookFeatures {
        let (best_bid, best_ask) = self.best_bid_ask().unwrap_or((Decimal::ZERO, Decimal::ZERO));

        let top_bids: Vec<_> = self.bids.iter()
            .take(depth)
            .map(|(rev_price, qty)| (rev_price.0, *qty))
            .collect();
        let top_asks: Vec<_> = self.asks.iter()
            .take(depth)
            .map(|(price, qty)| (*price, *qty))
            .collect();

        // 1. 基础指标
        let spread = best_ask - best_bid;
        let spread_bps = if !best_bid.is_zero() {
            (spread / best_bid * Decimal::from(10000)).round_dp(2)
        } else {
            Decimal::ZERO
        };

        // 2. 微价格
        let microprice = if let (Some(bid), Some(ask)) = (top_bids.first(), top_asks.first()) {
            (bid.0 * ask.1 + ask.0 * bid.1) / (bid.1 + ask.1)
        } else {
            Decimal::ZERO
        };

        // 3. 成交量计算
        let bid_volume_depth: Decimal = top_bids.iter().map(|(_, q)| q).sum();
        let ask_volume_depth: Decimal = top_asks.iter().map(|(_, q)| q).sum();
        let total_bid_volume: Decimal = self.bids.values().sum();
        let total_ask_volume: Decimal = self.asks.values().sum();

        // 4. OFI (订单流不平衡)
        let ofi = bid_volume_depth - ask_volume_depth;

        // 5. 买卖比例
        let bid_ask_ratio = if ask_volume_depth > Decimal::ZERO {
            bid_volume_depth / ask_volume_depth
        } else {
            dec!(10.0)
        };

        // 6. 加权平均价格
        let weighted_bid_price: Decimal = if bid_volume_depth > Decimal::ZERO {
            top_bids.iter().map(|(p, q)| p * q).sum::<Decimal>() / bid_volume_depth
        } else {
            Decimal::ZERO
        };
        let weighted_ask_price: Decimal = if ask_volume_depth > Decimal::ZERO {
            top_asks.iter().map(|(p, q)| p * q).sum::<Decimal>() / ask_volume_depth
        } else {
            Decimal::ZERO
        };

        // 7. 价格压力
        let price_pressure = weighted_ask_price - weighted_bid_price;

        // 8. 深度集中度
        let bid_depth_3: Decimal = self.bids.iter().take(3).map(|(_, q)| q).sum();
        let ask_depth_3: Decimal = self.asks.iter().take(3).map(|(_, q)| q).sum();
        let bid_concentration = if total_bid_volume > Decimal::ZERO {
            bid_depth_3 / total_bid_volume * dec!(100)
        } else {
            Decimal::ZERO
        };
        let ask_concentration = if total_ask_volume > Decimal::ZERO {
            ask_depth_3 / total_ask_volume * dec!(100)
        } else {
            Decimal::ZERO
        };

        // 9. OBI
        let obi = if total_bid_volume + total_ask_volume > Decimal::ZERO {
            (total_bid_volume - total_ask_volume) / (total_bid_volume + total_ask_volume) * dec!(100)
        } else {
            Decimal::ZERO
        };

        // 10. 价格弹性
        let bid_elasticity = if top_bids.len() >= 2 {
            (0..top_bids.len()-1).map(|i| top_bids[i].0 - top_bids[i+1].0).sum::<Decimal>() / Decimal::from(top_bids.len() - 1)
        } else {
            Decimal::ZERO
        };
        let ask_elasticity = if top_asks.len() >= 2 {
            (0..top_asks.len()-1).map(|i| top_asks[i+1].0 - top_asks[i].0).sum::<Decimal>() / Decimal::from(top_asks.len() - 1)
        } else {
            Decimal::ZERO
        };

        // 11. 鲸鱼检测
        let whale_bid = Self::detect_whale(&top_bids.iter().map(|(p, q)| (p, q)).collect());
        let whale_ask = Self::detect_whale(&top_asks.iter().map(|(p, q)| (p, q)).collect());

        // 12. 大单占比
        let max_bid_qty = top_bids.iter().map(|(_, q)| q).max().unwrap_or(&Decimal::ZERO);
        let max_ask_qty = top_asks.iter().map(|(_, q)| q).max().unwrap_or(&Decimal::ZERO);
        let max_bid_ratio = if bid_volume_depth > Decimal::ZERO {
            *max_bid_qty / bid_volume_depth * dec!(100)
        } else {
            Decimal::ZERO
        };
        let max_ask_ratio = if ask_volume_depth > Decimal::ZERO {
            *max_ask_qty / ask_volume_depth * dec!(100)
        } else {
            Decimal::ZERO
        };

        // 13. 挂单变化率
        let prev_bid_volume: Decimal = self.prev_bids.values().sum();
        let prev_ask_volume: Decimal = self.prev_asks.values().sum();
        let bid_volume_change = if prev_bid_volume > Decimal::ZERO {
            ((total_bid_volume - prev_bid_volume) / prev_bid_volume * dec!(100)).round_dp(2)
        } else {
            Decimal::ZERO
        };
        let ask_volume_change = if prev_ask_volume > Decimal::ZERO {
            ((total_ask_volume - prev_ask_volume) / prev_ask_volume * dec!(100)).round_dp(2)
        } else {
            Decimal::ZERO
        };

        // 14. 价格变化率
        let price_change = if let Some(last) = self.update_history.last() {
            if last.best_bid > Decimal::ZERO {
                ((best_bid - last.best_bid) / last.best_bid * dec!(100)).round_dp(2)
            } else {
                Decimal::ZERO
            }
        } else {
            Decimal::ZERO
        };

        // 15. 累计 delta
        let cum_delta = if self.update_history.len() >= 2 {
            let first = self.update_history.first().unwrap();
            let last = self.update_history.last().unwrap();
            (last.bid_volume - last.ask_volume) - (first.bid_volume - first.ask_volume)
        } else {
            Decimal::ZERO
        };

        // 16. 斜率计算
        let slope_bid = Self::calc_slope(&top_bids.iter().map(|(p, q)| (p, q)).collect());
        let slope_ask = Self::calc_slope(&top_asks.iter().map(|(p, q)| (p, q)).collect());

        // 17. 流动性缺口
        let liquidity_gap_bid = Self::calc_liquidity_gap(&top_bids.iter().map(|(p, q)| (p, q)).collect());
        let liquidity_gap_ask = Self::calc_liquidity_gap(&top_asks.iter().map(|(p, q)| (p, q)).collect());

        // 18. 买卖压力比
        let bid_pressure_front: Decimal = self.bids.iter().take(5).map(|(_, q)| q).sum();
        let bid_pressure_back: Decimal = self.bids.iter().skip(5).take(5).map(|(_, q)| q).sum();
        let ask_pressure_front: Decimal = self.asks.iter().take(5).map(|(_, q)| q).sum();
        let ask_pressure_back: Decimal = self.asks.iter().skip(5).take(5).map(|(_, q)| q).sum();
        let bid_pressure_ratio = if bid_pressure_back > Decimal::ZERO { bid_pressure_front / bid_pressure_back } else { dec!(10.0) };
        let ask_pressure_ratio = if ask_pressure_back > Decimal::ZERO { ask_pressure_front / ask_pressure_back } else { dec!(10.0) };

        // 19. 盘口厚度
        let near_bid_thickness: Decimal = if best_bid > Decimal::ZERO {
            self.get_bids_in_range(best_bid - dec!(0.1), best_bid).iter().map(|(_, q)| q).sum()
        } else { Decimal::ZERO };
        let near_ask_thickness: Decimal = if best_ask > Decimal::ZERO {
            self.get_asks_in_range(best_ask, best_ask + dec!(0.1)).iter().map(|(_, q)| q).sum()
        } else { Decimal::ZERO };

        // 20. 价格支撑/阻力
        let support_strength = if best_bid > Decimal::ZERO {
            self.get_bids_in_range(best_bid * dec!(0.99), best_bid).iter().map(|(_, q)| q).sum::<Decimal>()
        } else { Decimal::ZERO };
        let resistance_strength = if best_ask > Decimal::ZERO {
            self.get_asks_in_range(best_ask, best_ask * dec!(1.01)).iter().map(|(_, q)| q).sum::<Decimal>()
        } else { Decimal::ZERO };

        // 21. 失衡度
        let imbalance_depth_10 = if ask_volume_depth > Decimal::ZERO { bid_volume_depth / ask_volume_depth } else { dec!(10.0) };
        let imbalance_total = if total_ask_volume > Decimal::ZERO { total_bid_volume / total_ask_volume } else { dec!(10.0) };

        // 22. 趋势强度
        let trend_strength = (obi * dec!(0.3) + bid_volume_change * dec!(0.2) + price_change * dec!(0.5))
            .max(dec!(-100)).min(dec!(100));

        // 23. 拉盘/砸盘信号
        let pump_signal = slope_bid > dec!(100000) && slope_ask < dec!(50000) && bid_concentration > dec!(30) && bid_volume_change > dec!(10) && ofi > dec!(50000);
        let dump_signal = slope_ask < dec!(-100000) && slope_bid > dec!(-50000) && ask_concentration > dec!(30) && ask_volume_change > dec!(10) && ofi < dec!(-50000);

        // 24. 鲸鱼进出场
        let whale_entry = whale_bid && bid_volume_change > dec!(20) && max_bid_ratio > dec!(40);
        let whale_exit = whale_ask && ask_volume_change > dec!(20) && max_ask_ratio > dec!(40);

        // 25. 假突破
        let fake_breakout = price_change.abs() > dec!(0.5) && (bid_volume_change.abs() + ask_volume_change.abs()) < dec!(5);

        // 26. 流动性警告
        let liquidity_warning = spread_bps > dec!(50) || (bid_volume_depth < dec!(10000) && ask_volume_depth < dec!(10000));

        // 27 & 28. 吃筹/砸盘检测
        let bid_eating = bid_volume_change > dec!(30) && ask_volume_change < dec!(-10) && price_change > Decimal::ZERO;
        let ask_eating = ask_volume_change > dec!(30) && bid_volume_change < dec!(-10) && price_change < Decimal::ZERO;

        // 29. 失衡加速
        let imbalance_acceleration = if self.update_history.len() >= 10 {
            let old_obi = if let Some(old) = self.update_history.get(self.update_history.len() - 10) {
                let old_total = old.bid_volume + old.ask_volume;
                if old_total > Decimal::ZERO { (old.bid_volume - old.ask_volume) / old_total * dec!(100) } else { Decimal::ZERO }
            } else { Decimal::ZERO };
            obi - old_obi
        } else { Decimal::ZERO };

        // 30. 价格偏离
        let mid_price = (best_bid + best_ask) / dec!(2);
        let price_deviation = if mid_price > Decimal::ZERO { (weighted_bid_price - mid_price) / mid_price * dec!(100) } else { Decimal::ZERO };

        OrderBookFeatures {
            spread, spread_bps, microprice, ofi, bid_ask_ratio, obi,
            bid_volume_depth, ask_volume_depth, total_bid_volume, total_ask_volume,
            weighted_bid_price, weighted_ask_price, price_pressure, price_change,
            bid_concentration, ask_concentration, bid_elasticity, ask_elasticity,
            whale_bid, whale_ask, max_bid_ratio, max_ask_ratio,
            bid_volume_change, ask_volume_change, cum_delta,
            slope_bid, slope_ask,
            liquidity_gap_bid, liquidity_gap_ask, bid_pressure_ratio, ask_pressure_ratio,
            near_bid_thickness, near_ask_thickness, support_strength, resistance_strength,
            imbalance_depth_10, imbalance_total, trend_strength, imbalance_acceleration, price_deviation,
            pump_signal, dump_signal, whale_entry, whale_exit, fake_breakout, liquidity_warning, bid_eating, ask_eating,
        }
    }

    fn calc_slope(entries: &Vec<(&Decimal, &Decimal)>) -> Decimal {
        if entries.len() < 2 { return Decimal::ZERO; }
        let (p1, q1) = entries.first().unwrap();
        let (p2, q2) = entries.last().unwrap();
        if (**p2 - **p1).is_zero() { Decimal::ZERO } else { (**q2 - **q1) / (**p2 - **p1) }
    }

    fn calc_liquidity_gap(entries: &Vec<(&Decimal, &Decimal)>) -> usize {
        let mut gap = 0;
        for i in 1..entries.len() {
            if (*entries[i].0 - *entries[i-1].0) > dec!(0.5) { gap += 1; }
        }
        gap
    }

    fn detect_whale(entries: &Vec<(&Decimal, &Decimal)>) -> bool {
        let total: Decimal = entries.iter().map(|(_, q)| *q).sum();
        if total.is_zero() { return false; }
        let max_order = entries.iter().map(|(_, q)| *q).max().unwrap();
        (max_order / total) > dec!(0.3)
    }
}

#[derive(Debug, Clone, Copy, Default)]
pub struct OrderBookFeatures {
    // 基础数据 (6个)
    pub spread: Decimal,
    pub spread_bps: Decimal,
    pub microprice: Decimal,
    pub ofi: Decimal,
    pub bid_ask_ratio: Decimal,
    pub obi: Decimal,

    // 成交量相关 (4个)
    pub bid_volume_depth: Decimal,
    pub ask_volume_depth: Decimal,
    pub total_bid_volume: Decimal,
    pub total_ask_volume: Decimal,

    // 价格相关 (4个)
    pub weighted_bid_price: Decimal,
    pub weighted_ask_price: Decimal,
    pub price_pressure: Decimal,
    pub price_change: Decimal,

    // 深度相关 (4个)
    pub bid_concentration: Decimal,
    pub ask_concentration: Decimal,
    pub bid_elasticity: Decimal,
    pub ask_elasticity: Decimal,

    // 大单相关 (4个)
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

    // 流动性 (7个)
    pub liquidity_gap_bid: usize,
    pub liquidity_gap_ask: usize,
    pub bid_pressure_ratio: Decimal,
    pub ask_pressure_ratio: Decimal,
    pub near_bid_thickness: Decimal,
    pub near_ask_thickness: Decimal,
    pub support_strength: Decimal,
    pub resistance_strength: Decimal,

    // 失衡指标 (5个)
    pub imbalance_depth_10: Decimal,
    pub imbalance_total: Decimal,
    pub trend_strength: Decimal,
    pub imbalance_acceleration: Decimal,
    pub price_deviation: Decimal,

    // 信号标志 (8个)
    pub pump_signal: bool,
    pub dump_signal: bool,
    pub whale_entry: bool,
    pub whale_exit: bool,
    pub fake_breakout: bool,
    pub liquidity_warning: bool,
    pub bid_eating: bool,
    pub ask_eating: bool,
}