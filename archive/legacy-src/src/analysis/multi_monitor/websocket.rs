//! 多连接 WebSocket 管理器。
//!
//! 这个模块负责为每个 symbol 拉起一个长期运行的消费任务，并在连接断开后重连。
//! 业务状态不保存在这里，而是统一交给 `MultiSymbolMonitor`。

use std::sync::Arc;

use tokio::sync::mpsc;
use tokio::time::{sleep, Duration};

use crate::analysis::multi_monitor::MultiSymbolMonitor;
use crate::codec::binance_msg::StreamMsg;

/// 多 symbol WebSocket 生命周期管理器。
pub struct MultiWebSocketManager {
    monitors: Arc<MultiSymbolMonitor>,
    tasks: tokio::task::JoinSet<()>,
}

impl MultiWebSocketManager {
    /// 创建新的连接管理器。
    pub fn new(monitors: Arc<MultiSymbolMonitor>) -> Self {
        Self {
            monitors,
            tasks: tokio::task::JoinSet::new(),
        }
    }

    /// 为所有 symbol 启动独立的重连循环。
    ///
    /// 每个任务内部又拆成两层：
    /// - 一层 `run_client` 负责持续从 Binance 拉流
    /// - 一层消费 `mpsc`，把流消息投递到监控器
    ///
    /// 当内部 channel 断开时会整体重建，确保连接异常后能自恢复。
    pub async fn start_all(&mut self, symbols: Vec<String>) {
        for symbol in symbols {
            let monitors = self.monitors.clone();
            let task_symbol = symbol.clone();

            self.tasks.spawn(async move {
                loop {
                    let (tx, mut rx) = mpsc::channel::<StreamMsg>(2000);

                    let ws_symbol = task_symbol.clone();
                    let ws_task = tokio::spawn(async move {
                        loop {
                            let _ =
                                crate::client::websocket::run_client(&ws_symbol, tx.clone()).await;
                            sleep(Duration::from_secs(5)).await;
                        }
                    });

                    while let Some(msg) = rx.recv().await {
                        let _ = monitors.handle_msg(&task_symbol, msg).await;
                    }

                    ws_task.abort();
                    sleep(Duration::from_secs(5)).await;
                }
            });

            sleep(Duration::from_millis(300)).await;
        }
    }

    /// 阻塞等待所有后台任务结束。
    ///
    /// 正常运行时这些任务几乎不会自然退出，这个方法更多用于测试或进程收尾。
    pub async fn wait(&mut self) {
        while let Some(result) = self.tasks.join_next().await {
            let _ = result;
        }
    }
}
