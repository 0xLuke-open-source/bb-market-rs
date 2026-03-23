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
    inner: Arc<Mutex<TradingCore>>,
    log_dir: Arc<PathBuf>,
}
