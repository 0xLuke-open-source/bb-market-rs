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
    let mut tick = interval(Duration::from_millis(refresh_ms));
    loop {
        tick.tick().await;

        let arcs: Vec<(String, Arc<tokio::sync::Mutex<SymbolMonitor>>)> = {
            let monitors = monitor.monitors.lock().await;
            monitors.iter().map(|(k, v)| (k.clone(), v.clone())).collect()
        };

        for (symbol, arc) in arcs {
            let mut guard = arc.lock().await;
            let update = build_bridge_update(&symbol, &mut guard);
            spot.sync_liquidity(&symbol, &update.top_bids_raw, &update.top_asks_raw).await.ok();

            drop(guard);

            let mut ds = dash.write().await;
            ds.upsert(update.snapshot);
            for entry in update.feed_entries {
                ds.push_feed(entry);
            }
        }
    }
}
