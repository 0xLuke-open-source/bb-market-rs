//! 多币种监控调度器。
//!
//! `MultiSymbolMonitor` 是运行时的中心注册表：
//! - 启动时接收上游已经选定好的 symbol 列表
//! - 为每个 symbol 创建独立的 `SymbolMonitor`
//! - 收到流数据后按 symbol 分发到对应状态机
//! - 定时汇总每个 symbol 的拉盘/砸盘信号

use std::collections::HashMap;
use std::sync::Arc;

use tokio::sync::Mutex;
use tokio::time::Duration;

use crate::market_data::application::ports::{BigTradeSink, OrderBookTickSink, RecentTradeSink};
use crate::market_data::application::runtime::signal;
use crate::market_data::application::runtime::SymbolMonitor;
use crate::market_data::domain::stream::StreamMsg;
use crate::signal_intelligence::domain::pump_detector::PumpDetector;

/// 全局多币种监控器。
///
/// 外层用 `Arc<Mutex<...>>` 是因为：
/// - WebSocket 任务会并发写入不同 symbol 的监控状态
/// - 信号扫描任务也会周期性读取并修改内部统计
/// - 这里优先追求实现直接和状态集中，而不是做更细粒度的 lock-free 拆分
pub struct MultiSymbolMonitor {
    pub monitors: Arc<Mutex<HashMap<String, Arc<Mutex<SymbolMonitor>>>>>,
    pub report_interval: Duration,
    trade_persistence: Option<Arc<dyn RecentTradeSink>>,
    big_trade_persistence: Option<Arc<dyn BigTradeSink>>,
    /// 订单簿档位变化持久化（撤单/新增/修改 >=5%）
    orderbook_tick_persistence: Option<Arc<dyn OrderBookTickSink>>,
    /// 跨 symbol 共享的 PumpDetector（维护每个 symbol 的信号状态机）
    pump_detector: Arc<Mutex<PumpDetector>>,
}

impl MultiSymbolMonitor {
    /// 创建空监控器。
    ///
    /// `report_interval` 控制订单簿采样与部分统计的刷新频率，
    /// 避免每一笔深度更新都触发较重的历史采样。
    pub fn new(
        report_interval_secs: u64,
        trade_persistence: Option<Arc<dyn RecentTradeSink>>,
        big_trade_persistence: Option<Arc<dyn BigTradeSink>>,
    ) -> Self {
        Self {
            monitors: Arc::new(Mutex::new(HashMap::new())),
            report_interval: Duration::from_secs(report_interval_secs),
            trade_persistence,
            big_trade_persistence,
            orderbook_tick_persistence: None,
            pump_detector: Arc::new(Mutex::new(
                PumpDetector::new("pump_signals.txt").with_min_strength(30),
            )),
        }
    }

    /// 注入订单簿变化持久化服务（可选，在 main.rs 中初始化后调用）
    pub fn set_orderbook_tick_sink(&mut self, svc: Arc<dyn OrderBookTickSink>) {
        self.orderbook_tick_persistence = Some(svc);
    }

    /// 为每个 symbol 初始化独立监控状态。
    ///
    /// 每个 `SymbolMonitor` 持有该交易对自己的订单簿、K 线缓存、
    /// 成交统计与信号冷却状态，互不影响。
    pub async fn init_monitors(&self, symbols: Vec<String>) {
        let mut monitors = self.monitors.lock().await;
        for symbol in symbols {
            let normalized = symbol.trim().to_ascii_uppercase();
            if normalized.is_empty() {
                continue;
            }
            monitors.insert(
                normalized.clone(),
                Arc::new(Mutex::new(SymbolMonitor::new(&normalized))),
            );
        }
    }

    /// 按 symbol 分发一条流消息。
    ///
    /// 这里不直接在全局层做业务判断，而是把状态更新下沉到
    /// `SymbolMonitor`，让每个交易对自己维护完整上下文。
    ///
    /// Depth 消息处理后：
    /// - 取走本帧产出的 OrderChangeEvent
    /// - 转入 AnomalyDetector.record_change_batch（已接通撤单检测）
    /// - 提交到 OrderBookTickPersistenceService 入库
    pub async fn handle_msg(&self, symbol: &str, msg: StreamMsg) -> anyhow::Result<()> {
        let route_symbol = match &msg {
            StreamMsg::Depth(update) => update.symbol.to_ascii_uppercase(),
            StreamMsg::Trade(trade) => trade.symbol.to_ascii_uppercase(),
            StreamMsg::Ticker(ticker) => ticker.symbol.to_ascii_uppercase(),
            StreamMsg::Kline(kline) => kline.symbol.to_ascii_uppercase(),
        };
        let task_symbol = symbol.to_ascii_uppercase();
        let monitor = {
            let monitors = self.monitors.lock().await;
            monitors.get(route_symbol.as_str()).cloned()
        };
        if monitor.is_none() && !route_symbol.eq_ignore_ascii_case(&task_symbol) {
            return Ok(());
        }
        let Some(monitor) = monitor else {
            return Ok(());
        };

        let mut guard = monitor.lock().await;
        match msg {
            StreamMsg::Depth(update) => {
                guard.handle_depth_update(update, self.report_interval)?;

                // 取走本帧有意义的档位变化事件
                let change_events = guard.book.take_change_events();
                if !change_events.is_empty() {
                    // 接通撤单 / 激增检测
                    guard.anomaly_detector.record_change_batch(&change_events);

                    // 持久化到 market.orderbook_tick
                    if let Some(tick_svc) = &self.orderbook_tick_persistence {
                        tick_svc.persist_orderbook_changes(change_events);
                    }
                }
            }
            StreamMsg::Trade(trade) => {
                let big_trade = guard.apply_trade(&trade);
                if let Some(persistence) = &self.trade_persistence {
                    persistence.persist_agg_trade(&trade);
                }
                if let (Some(persistence), Some(event)) = (&self.big_trade_persistence, big_trade) {
                    persistence.persist_big_trade(&trade, event.threshold_qty);
                }
            }
            StreamMsg::Ticker(ticker) => guard.apply_ticker(&ticker),
            StreamMsg::Kline(kline) => guard.apply_kline(&kline),
        }

        Ok(())
    }

    pub async fn get_monitor(&self, symbol: &str) -> Option<Arc<Mutex<SymbolMonitor>>> {
        self.monitors
            .lock()
            .await
            .get(&symbol.to_ascii_uppercase())
            .cloned()
    }

    /// 扫描所有 symbol 的拉盘/砸盘信号。
    ///
    /// 这个方法通常由上层定时任务驱动。它只收集本轮触发的信号，
    /// 最终由 `signal::flush_signal_batch` 统一落盘，降低频繁 IO。
    pub async fn detect_pump_signals(&self) -> anyhow::Result<()> {
        let monitors: Vec<Arc<Mutex<SymbolMonitor>>> = {
            let guard = self.monitors.lock().await;
            guard.values().cloned().collect()
        };

        let mut signals = Vec::new();
        let mut pump_detector = self.pump_detector.lock().await;
        for monitor in monitors {
            let mut guard = monitor.lock().await;
            if let Some(signal) = guard.detect_pump_signal(&mut pump_detector) {
                signals.push(signal);
            }
        }

        signal::flush_signal_batch(&mut signals);
        Ok(())
    }
}
