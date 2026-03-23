//! bridge 层负责把 monitor/analysis 中的运行时状态转换成前端快照。
//!
//! 这层的职责很关键，因为上游状态是“算法视角”，下游前端需要的是“展示视角”。
//! 同时它还顺带把盘口前几档同步给本地撮合引擎，为交易面板提供可成交深度。

mod feed;
mod labels;
mod snapshot;

use std::sync::Arc;
use tokio::time::{interval, Duration};

use crate::analysis::multi_monitor::{MultiSymbolMonitor, SymbolMonitor};
use crate::web::spot::SpotTradingService;
use crate::web::state::SharedDashboardState;

use self::snapshot::build_bridge_update;

pub async fn run_bridge(
    monitor: Arc<MultiSymbolMonitor>,
    dash: SharedDashboardState,
    spot: SpotTradingService,
    refresh_ms: u64,
) {
    // bridge 是一个纯轮询任务：
    // 1. 扫描所有 symbol monitor
    // 2. 生成 SymbolJson + FeedEntry
    // 3. 同步给 dashboard state
    // 4. 把盘口同步给 spot 模块作为流动性
    let mut tick = interval(Duration::from_millis(refresh_ms));
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
            let update = build_bridge_update(&symbol, &mut guard);
            spot.sync_liquidity(&symbol, &update.top_bids_raw, &update.top_asks_raw)
                .await
                .ok();

            drop(guard);

            let mut ds = dash.write().await;
            ds.upsert(update.snapshot);
            for entry in update.feed_entries {
                ds.push_feed(entry);
            }
        }
    }
}
