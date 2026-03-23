use std::sync::Arc;

use tokio::sync::mpsc;
use tokio::time::{sleep, Duration};

use crate::analysis::multi_monitor::MultiSymbolMonitor;
use crate::codec::binance_msg::StreamMsg;

pub struct MultiWebSocketManager {
    monitors: Arc<MultiSymbolMonitor>,
    tasks: tokio::task::JoinSet<()>,
}

impl MultiWebSocketManager {
    pub fn new(monitors: Arc<MultiSymbolMonitor>) -> Self {
        Self {
            monitors,
            tasks: tokio::task::JoinSet::new(),
        }
    }

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
                            let _ = crate::client::websocket::run_client(&ws_symbol, tx.clone()).await;
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

    pub async fn wait(&mut self) {
        while let Some(result) = self.tasks.join_next().await {
            let _ = result;
        }
    }
}
