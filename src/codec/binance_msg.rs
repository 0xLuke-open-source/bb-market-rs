use serde::{Deserialize};
use rust_decimal::Decimal;

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
    pub bids: Vec<[String; 2]>, // 价格, 数量 - Binance 发送的是字符串数组，只有两个元素
    #[serde(rename = "a")]
    pub asks: Vec<[String; 2]>, // 价格, 数量 - Binance 发送的是字符串数组，只有两个元素
}

// 用于 REST API 获取的初始快照结构
#[derive(Debug, Deserialize)]
#[allow(non_snake_case)]
pub struct Snapshot {
    pub lastUpdateId: u64,
    pub bids: Vec<[String; 2]>, // REST API 也返回字符串
    pub asks: Vec<[String; 2]>,
}