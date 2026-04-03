//! bridge 层负责把 monitor/analysis 中的运行时状态转换成前端快照。
//!
//! 这层的职责很关键，因为上游状态是“算法视角”，下游前端需要的是“展示视角”。
//! 同时它还顺带把盘口前几档同步给本地撮合引擎，为交易面板提供可成交深度。

mod feed;
mod labels;
mod snapshot;

use std::sync::Arc;
use tokio::time::{interval, Duration, Instant};

use crate::execution::application::spot::SpotTradingService;
use crate::instrument_catalog::application::SymbolRegistryService;
use crate::instrument_catalog::domain::SymbolPrecision;
use crate::market_data::application::runtime::{MultiSymbolMonitor, SymbolMonitor};
use crate::terminal::application::cache::persist_dashboard_cache;
use crate::terminal::application::dto::OrderBookSnapshotRecord;
use crate::terminal::application::ports::{OrderBookSnapshotSink, SymbolPanelSnapshotStore};
use crate::terminal::application::projection::{SharedDashboardState, SymbolJson};

pub use self::snapshot::build_symbol_detail;
use self::snapshot::{build_bridge_update, build_panel_persistence_snapshot};

const DASHBOARD_CACHE_PATH: &str = "logs/dashboard-cache.json";
const DASHBOARD_CACHE_FLUSH_SECS: u64 = 5;

pub async fn run_bridge(
    monitor: Arc<MultiSymbolMonitor>,
    symbol_registry: SymbolRegistryService,
    dash: SharedDashboardState,
    spot: SpotTradingService,
    orderbook_persistence: Arc<dyn OrderBookSnapshotSink>,
    panel_persistence: Arc<dyn SymbolPanelSnapshotStore>,
    refresh_ms: u64,
) {
    // bridge 是一个纯轮询任务：
    // 1. 扫描所有 symbol monitor
    // 2. 生成 SymbolJson + FeedEntry
    // 3. 同步给 dashboard state
    // 4. 把盘口同步给 spot 模块作为流动性
    let mut tick = interval(Duration::from_millis(refresh_ms));
    let mut last_persist = Instant::now() - Duration::from_secs(DASHBOARD_CACHE_FLUSH_SECS);
    loop {
        tick.tick().await;

        let arcs: Vec<(String, Arc<tokio::sync::Mutex<SymbolMonitor>>)> = {
            let monitors = monitor.monitors.lock().await;
            monitors
                .iter()
                .map(|(k, v)| (k.clone(), v.clone()))
                .collect()
        };

        for (symbol, arc) in arcs {
            let mut guard = arc.lock().await;
            let mut update = build_bridge_update(&symbol, &mut guard);
            if let Some(precision) = symbol_registry
                .symbol_precision(&update.snapshot.symbol)
                .await
            {
                apply_symbol_precision(&mut update.snapshot, precision);
            }
            let orderbook_snapshot = OrderBookSnapshotRecord::from_snapshot(
                &update.snapshot,
                &update.top_bids_raw,
                &update.top_asks_raw,
            );
            let panel_snapshot = build_panel_persistence_snapshot(&update.snapshot, &guard);
            spot.sync_liquidity(&symbol, &update.top_bids_raw, &update.top_asks_raw)
                .await
                .ok();

            drop(guard);
            orderbook_persistence.submit_snapshot(orderbook_snapshot);

            let mut ds = dash.write().await;
            ds.upsert(update.snapshot);
            for entry in update.feed_entries {
                ds.push_feed(entry);
            }
            let signal_history = ds
                .feed
                .iter()
                .filter(|entry| entry.symbol == symbol)
                .take(20)
                .cloned()
                .collect::<Vec<_>>();
            drop(ds);

            panel_persistence.submit_snapshot(&panel_snapshot, signal_history);
        }

        if last_persist.elapsed() >= Duration::from_secs(DASHBOARD_CACHE_FLUSH_SECS) {
            if let Err(err) = persist_dashboard_cache(&dash, DASHBOARD_CACHE_PATH).await {
                eprintln!("dashboard cache persist error: {}", err);
            }
            last_persist = Instant::now();
        }
    }
}

fn apply_symbol_precision(symbol: &mut SymbolJson, precision: SymbolPrecision) {
    if precision.price_precision > 0 {
        symbol.price_precision = precision.price_precision;
    }
    if precision.quantity_precision > 0 {
        symbol.quantity_precision = precision.quantity_precision;
    }
}
