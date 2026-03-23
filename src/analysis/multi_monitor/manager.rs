use std::collections::HashMap;
use std::fs::File;
use std::io::{BufRead, BufReader};
use std::sync::Arc;

use tokio::sync::Mutex;
use tokio::time::Duration;

use crate::analysis::multi_monitor::signal;
use crate::analysis::multi_monitor::SymbolMonitor;
use crate::codec::binance_msg::StreamMsg;

pub struct MultiSymbolMonitor {
    pub monitors: Arc<Mutex<HashMap<String, Arc<Mutex<SymbolMonitor>>>>>,
    pub report_interval: Duration,
}

impl MultiSymbolMonitor {
    pub fn new(report_interval_secs: u64) -> Self {
        Self {
            monitors: Arc::new(Mutex::new(HashMap::new())),
            report_interval: Duration::from_secs(report_interval_secs),
        }
    }

    pub async fn load_symbols_from_file(
        &self,
        path: &str,
        max: usize,
    ) -> anyhow::Result<Vec<String>> {
        let file = File::open(path)?;
        let reader = BufReader::new(file);
        let mut symbols = Vec::new();

        for line in reader.lines() {
            let symbol = line?.trim().to_string();
            if !symbol.is_empty() && symbol != "币安人生" {
                symbols.push(format!("{}USDT", symbol));
                if symbols.len() >= max {
                    break;
                }
            }
        }

        Ok(symbols)
    }

    pub async fn init_monitors(&self, symbols: Vec<String>) {
        let mut monitors = self.monitors.lock().await;
        for symbol in symbols {
            monitors.insert(
                symbol.clone(),
                Arc::new(Mutex::new(SymbolMonitor::new(&symbol))),
            );
        }
    }

    pub async fn handle_msg(&self, symbol: &str, msg: StreamMsg) -> anyhow::Result<()> {
        let monitor = {
            let monitors = self.monitors.lock().await;
            monitors.get(symbol).cloned()
        };
        let Some(monitor) = monitor else {
            return Ok(());
        };

        let mut guard = monitor.lock().await;
        match msg {
            StreamMsg::Depth(update) => {
                guard.handle_depth_update(update, self.report_interval)?;
            }
            StreamMsg::Trade(trade) => guard.apply_trade(&trade),
            StreamMsg::Ticker(ticker) => guard.apply_ticker(&ticker),
            StreamMsg::Kline(kline) => guard.apply_kline(&kline),
        }

        Ok(())
    }

    pub async fn get_active_symbols(&self) -> Vec<String> {
        self.monitors.lock().await.keys().cloned().collect()
    }

    pub async fn detect_pump_signals(&self) -> anyhow::Result<()> {
        let monitors: Vec<Arc<Mutex<SymbolMonitor>>> = {
            let guard = self.monitors.lock().await;
            guard.values().cloned().collect()
        };

        let mut signals = Vec::new();
        for monitor in monitors {
            let mut guard = monitor.lock().await;
            if let Some(signal) = guard.detect_pump_signal() {
                signals.push(signal);
            }
        }

        signal::flush_signal_batch(&mut signals);
        Ok(())
    }
}
