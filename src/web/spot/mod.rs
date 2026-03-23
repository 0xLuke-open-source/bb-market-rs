//! `web::spot` 是“前端交易 API”到“本地撮合引擎”的适配层。
//!
//! 这一层并不实现撮合算法本身，而是负责：
//! 1. 解析前端请求
//! 2. 调用 engine
//! 3. 维护订单/成交/止损单等展示态
//! 4. 落盘日志，供回放查询使用

mod core;
mod helpers;
mod service;
mod storage;
mod types;

#[cfg(test)]
mod tests;

use std::path::PathBuf;
use std::sync::Arc;

use tokio::sync::Mutex;

use self::core::TradingCore;

pub use self::types::{
    ApiOrderRequest, ApiResponse, CancelAllRequest, CancelAllResult, OrderActionResult,
    ReplayQuery, ReplayResponse,
};

const USER_ACCOUNT_ID: u64 = 900001;
const LIQUIDITY_ACCOUNT_ID: u64 = 999999;

#[derive(Clone)]
pub struct SpotTradingService {
    // 核心交易状态，所有交易相关的共享数据都放在这里。
    inner: Arc<Mutex<TradingCore>>,
    // 日志目录，保存订单/成交/余额以及 archive 回放事件。
    log_dir: Arc<PathBuf>,
}
