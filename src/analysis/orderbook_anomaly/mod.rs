//! 订单簿异常检测模块。
//!
//! 这里把异常检测拆成：
//! - `types`：异常类型、事件结构和配置
//! - `detector`：基于订单簿快照与变动历史执行检测

mod detector;
pub mod types;

pub use detector::OrderBookAnomalyDetector;
