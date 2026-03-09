use std::collections::BTreeMap;
use std::cmp::Reverse;
use rust_decimal::Decimal;
use crate::codec::binance_msg::{DepthUpdate, Snapshot};
use std::str::FromStr;
use rust_decimal_macros::dec;

#[derive(Debug)]
#[allow(dead_code)]
pub struct OrderBook {
    pub symbol: String,
    pub last_update_id: u64,
    pub bids: BTreeMap<Reverse<Decimal>, Decimal>,
    pub asks: BTreeMap<Decimal, Decimal>,
}

impl OrderBook {
    pub fn new(symbol: &str) -> Self {
        Self {
            symbol: symbol.to_string(),
            last_update_id: 0,
            bids: BTreeMap::new(),
            asks: BTreeMap::new(),
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
    }

    pub fn apply_incremental_update(&mut self, msg: DepthUpdate) -> anyhow::Result<()> {
        if msg.last_update_id <= self.last_update_id {
            return Ok(());
        }

        // 首次对齐逻辑：U <= lastUpdateId+1 且 u >= lastUpdateId+1
        if self.last_update_id != 0 && msg.first_update_id > self.last_update_id + 1 {
            anyhow::bail!("Data gap: expected {}, got {}", self.last_update_id + 1, msg.first_update_id);
        }

        // 处理 bids
        for bid in msg.bids {
            let price = Decimal::from_str(&bid[0])?;
            let qty = Decimal::from_str(&bid[1])?;

            if qty.is_zero() {
                self.bids.remove(&Reverse(price));
            } else {
                self.bids.insert(Reverse(price), qty);
            }
        }

        // 处理 asks
        for ask in msg.asks {
            let price = Decimal::from_str(&ask[0])?;
            let qty = Decimal::from_str(&ask[1])?;

            if qty.is_zero() {
                self.asks.remove(&price);
            } else {
                self.asks.insert(price, qty);
            }
        }

        self.last_update_id = msg.last_update_id;
        Ok(())
    }

    pub fn best_bid_ask(&self) -> Option<(Decimal, Decimal)> {
        let b = self.bids.keys().next()?.0;
        let a = *self.asks.keys().next()?;
        Some((b, a))
    }

    // 获取前 N 个 bids 和 asks
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

    /// 计算盘口特征
    pub fn compute_features(&self, depth: usize) -> OrderBookFeatures {
        // 解开 Reverse
        let top_bids: Vec<_> = self.bids.iter()
            .take(depth)
            .map(|(rev_price, qty)| (&rev_price.0, qty))
            .collect();
        let top_asks: Vec<_> = self.asks.iter().take(depth).collect();

        let slope_bid = Self::calc_slope(&top_bids);
        let slope_ask = Self::calc_slope(&top_asks);

        let liquidity_gap_bid = Self::calc_liquidity_gap(&top_bids);
        let liquidity_gap_ask = Self::calc_liquidity_gap(&top_asks);

        let whale_bid = Self::detect_whale(&top_bids);
        let whale_ask = Self::detect_whale(&top_asks);

        let (spread, spread_bps) = if let (Some(bid), Some(ask)) = (top_bids.first(), top_asks.first()) {
            let s = ask.0 - bid.0;
            let s_bps = if !bid.0.is_zero() { (s / bid.0 * Decimal::from(10000)).round_dp(2) } else { Decimal::ZERO };
            (s, s_bps)
        } else {
            (Decimal::ZERO, Decimal::ZERO)
        };

        let microprice = if let (Some(bid), Some(ask)) = (top_bids.first(), top_asks.first()) {
            (bid.0 * ask.1 + ask.0 * bid.1) / (bid.1 + ask.1)
        } else {
            Decimal::ZERO
        };

        let buy_volume: Decimal = top_bids.iter().map(|(_, q)| *q).sum();
        let sell_volume: Decimal = top_asks.iter().map(|(_, q)| *q).sum();
        let ofi = buy_volume - sell_volume;
        let bid_ask_ratio = if sell_volume.is_zero() { Decimal::ZERO } else { buy_volume / sell_volume };

        let pump_flag = ofi > dec!(1.5) * sell_volume;
        let dump_flag = ofi < -dec!(1.5) * buy_volume;

        OrderBookFeatures {
            slope_bid,
            slope_ask,
            liquidity_gap_bid,
            liquidity_gap_ask,
            whale_bid,
            whale_ask,
            spread,
            spread_bps,
            microprice,
            ofi,
            bid_ask_ratio,
            pump_flag,
            dump_flag,
        }
    }

    fn calc_slope(entries: &Vec<(&Decimal, &Decimal)>) -> Decimal {
        if entries.len() < 2 { return Decimal::ZERO; }
        let (p1, q1) = entries.first().unwrap();
        let (p2, q2) = entries.last().unwrap();
        (**q2 - **q1) / (**p2 - **p1)
    }

    fn calc_liquidity_gap(entries: &Vec<(&Decimal, &Decimal)>) -> usize {
        let mut gap = 0;
        for i in 1..entries.len() {
            if (*entries[i].0 - *entries[i-1].0) >dec!(0.5) {
                gap += 1;
            }
        }
        gap
    }

    fn detect_whale(entries: &Vec<(&Decimal, &Decimal)>) -> bool {
        let total: Decimal = entries.iter().map(|(_, q)| *q).sum();
        if total.is_zero() { return false; }
        let max_order = entries.iter().map(|(_, q)| *q).max().unwrap();
        (max_order / total) > dec!(0.3) // 单笔占比 >30%
    }



}


#[derive(Debug, Clone, Copy)]
pub struct OrderBookFeatures {
    pub slope_bid: Decimal,
    pub slope_ask: Decimal,
    pub liquidity_gap_bid: usize,
    pub liquidity_gap_ask: usize,
    pub whale_bid: bool,
    pub whale_ask: bool,
    pub spread: Decimal,
    pub spread_bps: Decimal,
    pub microprice: Decimal,
    pub ofi: Decimal,
    pub bid_ask_ratio: Decimal,
    pub pump_flag: bool,
    pub dump_flag: bool,
}