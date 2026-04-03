use std::collections::HashMap;

use anyhow::Result;
use async_trait::async_trait;
use reqwest::Client;

use crate::instrument_catalog::domain::{SymbolPrecision, VisibilityTier};

#[async_trait]
pub trait SymbolRegistryQueryPort: Send + Sync {
    async fn load_enabled_symbols(&self, max: usize) -> Result<Vec<String>>;
    async fn visible_symbols(&self, tier: VisibilityTier, candidates: &[String]) -> Vec<String>;
    async fn can_view_symbol(&self, tier: VisibilityTier, symbol: &str) -> bool;
    async fn symbol_precision(&self, symbol: &str) -> Option<SymbolPrecision>;
    async fn symbol_precisions(&self, symbols: &[String]) -> HashMap<String, SymbolPrecision>;
}

#[async_trait]
pub trait SymbolRegistryAdminPort: Send + Sync {
    async fn sync_usdt_symbols(&self, client: &Client) -> Result<usize>;
    async fn sync_symbols_from_file(&self, file_path: &str) -> Result<usize>;
    async fn set_symbols_enabled(&self, symbols: &[String], enabled: bool) -> Result<u64>;
    async fn set_symbols_visibility(&self, symbols: &[String], visible: bool) -> Result<u64>;
    async fn set_all_symbols_visibility(&self, visible: bool) -> Result<u64>;
}
