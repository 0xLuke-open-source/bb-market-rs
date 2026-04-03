mod bootstrap;
mod execution;
mod identity;
mod instrument_catalog;
mod market_data;
mod shared;
mod signal_intelligence;
mod terminal;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    bootstrap::main::run().await
}
