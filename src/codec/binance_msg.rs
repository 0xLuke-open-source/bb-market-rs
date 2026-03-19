// src/codec/binance_msg.rs
// 币安 WebSocket 所有消息类型定义

use serde::{Deserialize, Serialize};
use rust_decimal::Decimal;

// ── 原有：增量订单簿 ──────────────────────────────────────────────
#[allow(dead_code)]
#[derive(Debug, Deserialize, Clone)]
pub struct DepthUpdate {
    #[serde(rename = "e")]
    pub event_type: String,
    #[serde(rename = "E")]
    pub event_time: u64,
    #[serde(rename = "s")]
    pub symbol: String,
    #[serde(rename = "U")]
    pub first_update_id: u64,
    #[serde(rename = "u")]
    pub last_update_id: u64,
    #[serde(rename = "b")]
    pub bids: Vec<[String; 2]>,
    #[serde(rename = "a")]
    pub asks: Vec<[String; 2]>,
}

// ── 新增：归集成交（aggTrade） ───────────────────────────────────
// m=true  → 买方是 Maker → 主动卖出（Taker Sell）
// m=false → 买方是 Taker → 主动买入（Taker Buy）
#[allow(dead_code)]
#[derive(Debug, Deserialize, Clone)]
pub struct AggTrade {
    #[serde(rename = "e")]
    pub event_type: String,
    #[serde(rename = "E")]
    pub event_time: u64,
    #[serde(rename = "s")]
    pub symbol: String,
    #[serde(rename = "a")]
    pub agg_trade_id: u64,
    #[serde(rename = "p")]
    pub price: String,
    #[serde(rename = "q")]
    pub qty: String,
    #[serde(rename = "T")]
    pub trade_time: u64,
    /// true = 主动卖出；false = 主动买入
    #[serde(rename = "m")]
    pub is_buyer_maker: bool,
}

impl AggTrade {
    pub fn price_decimal(&self) -> Decimal {
        self.price.parse().unwrap_or(Decimal::ZERO)
    }
    pub fn qty_decimal(&self) -> Decimal {
        self.qty.parse().unwrap_or(Decimal::ZERO)
    }
    /// 主动买入量（正值），主动卖出量（负值）—— 用于累计 CVD
    pub fn delta(&self) -> Decimal {
        let q = self.qty_decimal();
        if self.is_buyer_maker { -q } else { q }
    }
    pub fn is_taker_buy(&self) -> bool {
        !self.is_buyer_maker
    }
}

// ── 新增：精简 24h Ticker（miniTicker） ──────────────────────────
#[allow(dead_code)]
#[derive(Debug, Deserialize, Clone)]
pub struct MiniTicker {
    #[serde(rename = "e")]
    pub event_type: String,
    #[serde(rename = "E")]
    pub event_time: u64,
    #[serde(rename = "s")]
    pub symbol: String,
    /// 最新成交价
    #[serde(rename = "c")]
    pub close: String,
    /// 24h 开盘价
    #[serde(rename = "o")]
    pub open: String,
    /// 24h 最高
    #[serde(rename = "h")]
    pub high: String,
    /// 24h 最低
    #[serde(rename = "l")]
    pub low: String,
    /// 24h 成交量（基础资产）
    #[serde(rename = "v")]
    pub volume: String,
    /// 24h 成交额（USDT）
    #[serde(rename = "q")]
    pub quote_volume: String,
}

impl MiniTicker {
    pub fn close_f64(&self) -> f64 { self.close.parse().unwrap_or(0.0) }
    pub fn open_f64(&self)  -> f64 { self.open.parse().unwrap_or(0.0) }
    pub fn high_f64(&self)  -> f64 { self.high.parse().unwrap_or(0.0) }
    pub fn low_f64(&self)   -> f64 { self.low.parse().unwrap_or(0.0) }
    pub fn volume_f64(&self) -> f64 { self.volume.parse().unwrap_or(0.0) }
    pub fn quote_volume_f64(&self) -> f64 { self.quote_volume.parse().unwrap_or(0.0) }
    pub fn change_pct(&self) -> f64 {
        let o = self.open_f64();
        let c = self.close_f64();
        if o == 0.0 { 0.0 } else { (c - o) / o * 100.0 }
    }
}

// ── 新增：1m K线（kline） ────────────────────────────────────────
#[allow(dead_code)]
#[derive(Debug, Deserialize, Clone)]
pub struct KlineEvent {
    #[serde(rename = "e")]
    pub event_type: String,
    #[serde(rename = "E")]
    pub event_time: u64,
    #[serde(rename = "s")]
    pub symbol: String,
    #[serde(rename = "k")]
    pub kline: KlineData,
}

#[allow(dead_code)]
#[derive(Debug, Deserialize, Clone)]
pub struct KlineData {
    /// K线间隔 e.g. "1m","5m","1h"
    #[serde(rename = "i")]
    pub interval: String,
    /// K线开盘时间
    #[serde(rename = "t")]
    pub open_time: u64,
    /// K线收盘时间
    #[serde(rename = "T")]
    pub close_time: u64,
    #[serde(rename = "o")]
    pub open: String,
    #[serde(rename = "c")]
    pub close: String,
    #[serde(rename = "h")]
    pub high: String,
    #[serde(rename = "l")]
    pub low: String,
    /// 成交量（基础资产）
    #[serde(rename = "v")]
    pub volume: String,
    /// 成交额（USDT）
    #[serde(rename = "q")]
    pub quote_volume: String,
    /// 成交笔数
    #[serde(rename = "n")]
    pub trades: u64,
    /// 是否已完结
    #[serde(rename = "x")]
    pub is_closed: bool,
    /// 主动买入成交量
    #[serde(rename = "V")]
    pub taker_buy_volume: String,
    /// 主动买入成交额
    #[serde(rename = "Q")]
    pub taker_buy_quote_volume: String,
}

impl KlineEvent {
    /// 从 kline.interval 直接获取周期字符串
    pub fn kline_interval(&self) -> String {
        self.kline.interval.clone()
    }
}

impl KlineData {
    pub fn open_f64(&self)  -> f64 { self.open.parse().unwrap_or(0.0) }
    pub fn close_f64(&self) -> f64 { self.close.parse().unwrap_or(0.0) }
    pub fn high_f64(&self)  -> f64 { self.high.parse().unwrap_or(0.0) }
    pub fn low_f64(&self)   -> f64 { self.low.parse().unwrap_or(0.0) }
    pub fn volume_f64(&self) -> f64 { self.volume.parse().unwrap_or(0.0) }
    pub fn taker_buy_volume_f64(&self) -> f64 { self.taker_buy_volume.parse().unwrap_or(0.0) }
    /// 主动买入占比 0-100
    pub fn taker_buy_ratio(&self) -> f64 {
        let v = self.volume_f64();
        if v == 0.0 { 50.0 } else { self.taker_buy_volume_f64() / v * 100.0 }
    }
    /// 涨跌幅 %
    pub fn change_pct(&self) -> f64 {
        let o = self.open_f64();
        let c = self.close_f64();
        if o == 0.0 { 0.0 } else { (c - o) / o * 100.0 }
    }
}

// ── 组合 stream 消息（/stream?streams=...）的外层包装 ────────────
#[derive(Debug, Deserialize, Clone)]
pub struct CombinedMessage {
    pub stream: String,
    pub data: serde_json::Value,
}

// ── 统一消息枚举（发送到 channel 的类型）────────────────────────
#[derive(Debug, Clone)]
pub enum StreamMsg {
    Depth(DepthUpdate),
    Trade(AggTrade),
    Ticker(MiniTicker),
    Kline(KlineEvent),
}

impl CombinedMessage {
    /// 解析 data 字段为具体消息类型
    pub fn parse(self) -> Option<StreamMsg> {
        let s = &self.stream;
        if s.contains("@depth") {
            serde_json::from_value::<DepthUpdate>(self.data).ok().map(StreamMsg::Depth)
        } else if s.contains("@aggTrade") {
            serde_json::from_value::<AggTrade>(self.data).ok().map(StreamMsg::Trade)
        } else if s.contains("@miniTicker") {
            serde_json::from_value::<MiniTicker>(self.data).ok().map(StreamMsg::Ticker)
        } else if s.contains("@kline_") {
            serde_json::from_value::<KlineEvent>(self.data).ok().map(StreamMsg::Kline)
        } else {
            None
        }
    }
}

// ── REST API 快照 ────────────────────────────────────────────────
#[derive(Debug, Deserialize)]
#[allow(non_snake_case)]
pub struct Snapshot {
    pub lastUpdateId: u64,
    pub bids: Vec<[String; 2]>,
    pub asks: Vec<[String; 2]>,
}