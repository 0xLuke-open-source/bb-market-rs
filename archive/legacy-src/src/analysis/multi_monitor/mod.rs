//! 多币种实时监控入口。
//!
//! 这个模块负责把“多个 symbol 的订单簿状态机”与“WebSocket 连接管理器”
//! 组装起来，对外暴露统一的启动与消息分发接口。

mod manager;
mod signal;
mod symbol;
mod websocket;

pub use manager::MultiSymbolMonitor;
pub use symbol::SymbolMonitor;
pub use websocket::MultiWebSocketManager;
