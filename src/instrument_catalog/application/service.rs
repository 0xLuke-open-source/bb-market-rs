use std::collections::HashMap;
use std::sync::Arc;

use anyhow::Result;
use reqwest::Client;

use crate::instrument_catalog::application::ports::{
    SymbolRegistryAdminPort, SymbolRegistryQueryPort,
};
use crate::instrument_catalog::domain::{SymbolPrecision, VisibilityTier};

#[derive(Clone)]
pub struct SymbolRegistryService {
    inner: Arc<dyn SymbolRegistryQueryPort>,
}

impl SymbolRegistryService {
    pub fn new(inner: Arc<dyn SymbolRegistryQueryPort>) -> Self {
        Self { inner }
    }

    pub async fn load_enabled_symbols(&self, max: usize) -> Result<Vec<String>> {
        self.inner.load_enabled_symbols(max).await
    }

    pub async fn visible_symbols(
        &self,
        tier: VisibilityTier,
        candidates: &[String],
    ) -> Vec<String> {
        self.inner.visible_symbols(tier, candidates).await
    }

    pub async fn can_view_symbol(&self, tier: VisibilityTier, symbol: &str) -> bool {
        self.inner.can_view_symbol(tier, symbol).await
    }

    pub async fn symbol_precision(&self, symbol: &str) -> Option<SymbolPrecision> {
        self.inner.symbol_precision(symbol).await
    }

    pub async fn symbol_precisions(&self, symbols: &[String]) -> HashMap<String, SymbolPrecision> {
        self.inner.symbol_precisions(symbols).await
    }
}

#[derive(Clone)]
pub struct SymbolRegistryAdminService {
    inner: Arc<dyn SymbolRegistryAdminPort>,
}

impl SymbolRegistryAdminService {
    pub fn new(inner: Arc<dyn SymbolRegistryAdminPort>) -> Self {
        Self { inner }
    }

    pub async fn sync_usdt_symbols(&self, client: &Client) -> Result<usize> {
        self.inner.sync_usdt_symbols(client).await
    }

    pub async fn sync_symbols_from_file(&self, file_path: &str) -> Result<usize> {
        self.inner.sync_symbols_from_file(file_path).await
    }

    pub async fn set_symbols_enabled(&self, symbols: &[String], enabled: bool) -> Result<u64> {
        self.inner.set_symbols_enabled(symbols, enabled).await
    }

    pub async fn set_symbols_visibility(&self, symbols: &[String], visible: bool) -> Result<u64> {
        self.inner.set_symbols_visibility(symbols, visible).await
    }

    pub async fn set_all_symbols_visibility(&self, visible: bool) -> Result<u64> {
        self.inner.set_all_symbols_visibility(visible).await
    }
}
