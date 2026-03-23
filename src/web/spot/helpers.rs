//! helpers 收纳两类逻辑：
//! 1. API 文本字段到引擎枚举的解析
//! 2. 引擎结构到前端展示结构的转换

use std::str::FromStr;

use anyhow::{anyhow, Result};
use chrono::Local;
use rust_decimal::prelude::ToPrimitive;
use rust_decimal::Decimal;

use crate::engine::{NewOrderRequest, OrderType, Side, SubmitOrderResult, TimeInForce, Trade};
use crate::web::state::{TraderOrderJson, TraderTradeJson};

use super::types::{ApiOrderRequest, OrderActionResult, ParsedOrderType};
use super::USER_ACCOUNT_ID;

pub(super) fn parse_base_asset(symbol: &str) -> Result<String> {
    symbol
        .strip_suffix("USDT")
        .map(|base| base.to_string())
        .ok_or_else(|| anyhow!("unsupported symbol: {}", symbol))
}

pub(super) fn parse_side(input: &str) -> Result<Side> {
    match input.to_ascii_lowercase().as_str() {
        "buy" => Ok(Side::Buy),
        "sell" => Ok(Side::Sell),
        _ => Err(anyhow!("invalid side: {}", input)),
    }
}

pub(super) fn parse_order_type(input: &str) -> Result<ParsedOrderType> {
    match input.to_ascii_lowercase().as_str() {
        "limit" => Ok(ParsedOrderType::Limit),
        "market" => Ok(ParsedOrderType::Market),
        "stop_limit" => Ok(ParsedOrderType::StopLimit),
        "stop_market" => Ok(ParsedOrderType::StopMarket),
        _ => Err(anyhow!("invalid order_type: {}", input)),
    }
}

pub(super) fn parse_tif(input: Option<&str>, order_type: OrderType) -> Result<TimeInForce> {
    match input
        .unwrap_or(match order_type {
            OrderType::Limit => "gtc",
            OrderType::Market => "ioc",
        })
        .to_ascii_lowercase()
        .as_str()
    {
        "gtc" => Ok(TimeInForce::Gtc),
        "ioc" => Ok(TimeInForce::Ioc),
        "fok" => Ok(TimeInForce::Fok),
        "post_only" | "postonly" => Ok(TimeInForce::PostOnly),
        other => Err(anyhow!("invalid time_in_force: {}", other)),
    }
}

pub(super) fn decimal_from_f64(value: f64, field: &str) -> Result<Decimal> {
    Decimal::from_str(&value.to_string()).map_err(|_| anyhow!("invalid {}: {}", field, value))
}

pub(super) fn now_text() -> String {
    Local::now().format("%H:%M:%S").to_string()
}

pub(super) fn now_ts() -> i64 {
    Local::now().timestamp_millis()
}

pub(super) fn trader_order_from_view(view: &crate::engine::OrderView) -> TraderOrderJson {
    TraderOrderJson {
        time: now_text(),
        order_id: view.order_id,
        symbol: view.symbol.clone(),
        side: format!("{:?}", view.side),
        order_type: format!("{:?}", view.order_type),
        time_in_force: format!("{:?}", view.time_in_force),
        price: view.price.and_then(|value| value.to_f64()),
        trigger_price: None,
        trigger_kind: None,
        quantity: view.original_qty.to_f64().unwrap_or(0.0),
        remaining_qty: view.remaining_qty.to_f64().unwrap_or(0.0),
        filled_qty: view.filled_qty.to_f64().unwrap_or(0.0),
        filled_quote_qty: view.filled_quote_qty.to_f64().unwrap_or(0.0),
        status: format!("{:?}", view.status),
    }
}

pub(super) fn order_from_submit(
    req: &NewOrderRequest,
    result: &SubmitOrderResult,
) -> TraderOrderJson {
    TraderOrderJson {
        time: now_text(),
        order_id: result.order_id,
        symbol: req.symbol.clone(),
        side: format!("{:?}", req.side),
        order_type: format!("{:?}", req.order_type),
        time_in_force: format!("{:?}", req.time_in_force),
        price: req.price.and_then(|value| value.to_f64()),
        trigger_price: None,
        trigger_kind: None,
        quantity: req
            .quantity
            .or(req.quote_quantity)
            .and_then(|value| value.to_f64())
            .unwrap_or(0.0),
        remaining_qty: result.remaining_qty.to_f64().unwrap_or(0.0),
        filled_qty: result.filled_qty.to_f64().unwrap_or(0.0),
        filled_quote_qty: result.filled_quote_qty.to_f64().unwrap_or(0.0),
        status: format!("{:?}", result.status),
    }
}

pub(super) fn trader_trade_from_trade(trade: &Trade) -> TraderTradeJson {
    let user_is_taker = trade.taker_account_id == USER_ACCOUNT_ID;
    let side = if user_is_taker {
        format!("{:?}", trade.taker_side)
    } else {
        format!("{:?}", opposite_side(trade.taker_side))
    };

    TraderTradeJson {
        time: now_text(),
        trade_id: trade.trade_id,
        symbol: trade.symbol.clone(),
        side,
        price: trade.price.to_f64().unwrap_or(0.0),
        quantity: trade.quantity.to_f64().unwrap_or(0.0),
        quote_quantity: trade.quote_quantity.to_f64().unwrap_or(0.0),
        liquidity: if user_is_taker { "Taker" } else { "Maker" }.to_string(),
    }
}

pub(super) fn summarize_result(result: &SubmitOrderResult) -> OrderActionResult {
    OrderActionResult {
        order_id: result.order_id,
        status: format!("{:?}", result.status),
        filled_qty: result.filled_qty.to_f64().unwrap_or(0.0),
        filled_quote_qty: result.filled_quote_qty.to_f64().unwrap_or(0.0),
        remaining_qty: result.remaining_qty.to_f64().unwrap_or(0.0),
        trade_count: result.trades.len(),
    }
}

pub(super) fn should_trigger_stop(req: &ApiOrderRequest, mid: Decimal) -> bool {
    let trigger = match req
        .trigger_price
        .and_then(|value| Decimal::from_str(&value.to_string()).ok())
    {
        Some(value) => value,
        None => return false,
    };
    let kind = req
        .trigger_kind
        .clone()
        .unwrap_or_else(|| "stop_loss".to_string())
        .to_ascii_lowercase();
    let side = req.side.to_ascii_lowercase();
    match (side.as_str(), kind.as_str()) {
        ("buy", "stop_loss") => mid >= trigger,
        ("buy", "take_profit") => mid <= trigger,
        ("sell", "stop_loss") => mid <= trigger,
        ("sell", "take_profit") => mid >= trigger,
        ("buy", _) => mid >= trigger,
        ("sell", _) => mid <= trigger,
        _ => false,
    }
}

fn opposite_side(side: Side) -> Side {
    match side {
        Side::Buy => Side::Sell,
        Side::Sell => Side::Buy,
    }
}

pub(super) trait SubmitOrderResultExt {
    fn status_string(&self) -> String;
}

impl SubmitOrderResultExt for SubmitOrderResult {
    fn status_string(&self) -> String {
        format!("{:?}", self.status)
    }
}
