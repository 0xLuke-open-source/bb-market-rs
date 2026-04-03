use async_trait::async_trait;

use crate::signal_intelligence::application::ports::SignalPriceSource;
use crate::terminal::application::projection::SharedDashboardState;

#[derive(Clone)]
pub struct DashboardPriceSource {
    dashboard: SharedDashboardState,
}

impl DashboardPriceSource {
    pub fn new(dashboard: SharedDashboardState) -> Self {
        Self { dashboard }
    }
}

#[async_trait]
impl SignalPriceSource for DashboardPriceSource {
    async fn current_mid_price(&self, symbol: &str) -> Option<f64> {
        let dashboard = self.dashboard.read().await;
        dashboard
            .symbols
            .get(&symbol.trim().to_ascii_uppercase())
            .map(|snapshot| snapshot.mid)
    }
}
