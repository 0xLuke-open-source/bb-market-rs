use async_trait::async_trait;

#[async_trait]
pub trait SignalPriceSource: Send + Sync {
    async fn current_mid_price(&self, symbol: &str) -> Option<f64>;
}
