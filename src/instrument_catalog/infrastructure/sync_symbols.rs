use std::collections::{HashMap, HashSet};
use std::fs;
use std::sync::Arc;

use anyhow::{Context, Result};
use async_trait::async_trait;
use reqwest::Client;
use serde::Deserialize;
use tokio::sync::OnceCell;
use tokio::sync::RwLock;
use tokio::time::Duration;

use crate::instrument_catalog::application::ports::{
    SymbolRegistryAdminPort, SymbolRegistryQueryPort,
};
use crate::instrument_catalog::domain::{SymbolPrecision, SymbolVisibility, VisibilityTier};
use crate::shared::postgres::PgPool;

const VISIBILITY_REFRESH_SECS: u64 = 15;
static SYMBOL_REGISTRY_SCHEMA_READY: OnceCell<()> = OnceCell::const_new();

#[derive(Debug, Deserialize)]
struct ExchangeInfo {
    symbols: Vec<ExchangeSymbol>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct ExchangeSymbol {
    symbol: String,
    base_asset: String,
    quote_asset: String,
    status: String,
    #[serde(default)]
    base_asset_precision: Option<u32>,
    #[serde(default)]
    quote_precision: Option<u32>,
    #[serde(default)]
    quote_asset_precision: Option<u32>,
    #[serde(default)]
    filters: Vec<ExchangeFilter>,
}

#[derive(Debug, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
struct ExchangeFilter {
    filter_type: String,
    #[serde(default)]
    tick_size: Option<String>,
    #[serde(default)]
    step_size: Option<String>,
}

#[derive(Clone)]
pub struct PostgresSymbolRegistryAdapter {
    pool: Arc<PgPool>,
    visibility: Arc<RwLock<SymbolVisibility>>,
    precisions: Arc<RwLock<HashMap<String, SymbolPrecision>>>,
}

pub async fn sync_usdt_symbols(client: &Client, pool: Arc<PgPool>) -> Result<usize> {
    ensure_schema(pool.clone()).await?;

    println!("🔄 开始同步 USDT 交易对到 PostgreSQL...");
    let url = "https://api.binance.com/api/v3/exchangeInfo";
    let response = client
        .get(url)
        .send()
        .await
        .context("Failed to send request to exchangeInfo")?;

    if !response.status().is_success() {
        anyhow::bail!("HTTP error: {}", response.status());
    }

    let exchange_info: ExchangeInfo = response
        .json()
        .await
        .context("Failed to parse exchangeInfo JSON")?;

    let mut symbols: Vec<ExchangeSymbol> = exchange_info
        .symbols
        .into_iter()
        .filter(|s| s.quote_asset.eq_ignore_ascii_case("USDT"))
        .collect();
    symbols.sort_by(|a, b| a.symbol.cmp(&b.symbol));

    let trading_count = symbols
        .iter()
        .filter(|symbol| symbol.status.eq_ignore_ascii_case("TRADING"))
        .count();

    let mut pooled = pool.acquire().await?;
    let tx = pooled.client_mut().transaction().await?;
    let stmt = tx
        .prepare(
            "insert into market.symbol_registry (
                symbol, base_asset, quote_asset, exchange_status,
                price_precision, quantity_precision, enabled,
                visible_public, visible_member, visible_subscriber,
                created_at, updated_at
            ) values (
                $1, $2, $3, $4,
                $5, $6, $7,
                $8, $9, $10,
                now(), now()
            )
            on conflict (symbol) do update set
                base_asset = excluded.base_asset,
                quote_asset = excluded.quote_asset,
                exchange_status = excluded.exchange_status,
                price_precision = excluded.price_precision,
                quantity_precision = excluded.quantity_precision,
                updated_at = now()",
        )
        .await?;

    for symbol in &symbols {
        let normalized = normalize_symbol(&symbol.symbol);
        let precision = precision_from_exchange_symbol(symbol);
        let default_visible = symbol.status.eq_ignore_ascii_case("TRADING");
        tx.execute(
            &stmt,
            &[
                &normalized,
                &symbol.base_asset,
                &symbol.quote_asset,
                &symbol.status,
                &(precision.price_precision as i32),
                &(precision.quantity_precision as i32),
                &default_visible,
                &default_visible,
                &default_visible,
                &default_visible,
            ],
        )
        .await?;
    }

    tx.commit().await?;
    println!(
        "✅ 已同步 {} 个 USDT 交易对到 PostgreSQL，其中 TRADING 状态 {} 个。",
        symbols.len(),
        trading_count
    );
    Ok(symbols.len())
}

pub async fn sync_symbols_from_file(pool: Arc<PgPool>, file_path: &str) -> Result<usize> {
    ensure_schema(pool.clone()).await?;

    let raw = fs::read_to_string(file_path)
        .with_context(|| format!("failed to read symbol file: {}", file_path))?;
    let symbols = parse_symbol_file(&raw);
    if symbols.is_empty() {
        anyhow::bail!("symbol file is empty: {}", file_path);
    }

    let mut pooled = pool.acquire().await?;
    let tx = pooled.client_mut().transaction().await?;
    let stmt = tx
        .prepare(
            "insert into market.symbol_registry (
                symbol, base_asset, quote_asset, exchange_status,
                price_precision, quantity_precision, enabled,
                visible_public, visible_member, visible_subscriber,
                created_at, updated_at
            ) values (
                $1, $2, 'USDT', 'TRADING',
                0, 0, true,
                true, true, true,
                now(), now()
            )
            on conflict (symbol) do update set
                base_asset = excluded.base_asset,
                quote_asset = excluded.quote_asset,
                exchange_status = excluded.exchange_status,
                price_precision = case
                    when excluded.price_precision > 0 then excluded.price_precision
                    else market.symbol_registry.price_precision
                end,
                quantity_precision = case
                    when excluded.quantity_precision > 0 then excluded.quantity_precision
                    else market.symbol_registry.quantity_precision
                end,
                enabled = true,
                visible_public = true,
                visible_member = true,
                visible_subscriber = true,
                updated_at = now()",
        )
        .await?;

    for base_asset in &symbols {
        let symbol = normalize_symbol(base_asset);
        tx.execute(&stmt, &[&symbol, base_asset]).await?;
    }

    tx.commit().await?;
    println!(
        "✅ 已从文件 {} 同步 {} 个币种到 PostgreSQL。",
        file_path,
        symbols.len()
    );
    Ok(symbols.len())
}

impl PostgresSymbolRegistryAdapter {
    pub async fn new(pool: Arc<PgPool>) -> Result<Self> {
        ensure_schema(pool.clone()).await?;
        let (visibility_snapshot, precision_snapshot) =
            load_registry_snapshot(pool.clone()).await?;
        let visibility = Arc::new(RwLock::new(visibility_snapshot));
        let precisions = Arc::new(RwLock::new(precision_snapshot));
        let service = Self {
            pool,
            visibility: visibility.clone(),
            precisions: precisions.clone(),
        };

        let refresh_pool = service.pool.clone();
        tokio::spawn(async move {
            let mut tick = tokio::time::interval(Duration::from_secs(VISIBILITY_REFRESH_SECS));
            loop {
                tick.tick().await;
                match load_registry_snapshot(refresh_pool.clone()).await {
                    Ok((visibility_snapshot, precision_snapshot)) => {
                        *visibility.write().await = visibility_snapshot;
                        *precisions.write().await = precision_snapshot;
                    }
                    Err(err) => {
                        eprintln!("symbol visibility refresh error: {}", err);
                    }
                }
            }
        });

        Ok(service)
    }
}

#[async_trait]
impl SymbolRegistryQueryPort for PostgresSymbolRegistryAdapter {
    async fn load_enabled_symbols(&self, max: usize) -> Result<Vec<String>> {
        load_enabled_symbols(self.pool.clone(), max).await
    }

    async fn visible_symbols(&self, tier: VisibilityTier, candidates: &[String]) -> Vec<String> {
        let visibility = self.visibility.read().await;
        let empty = HashSet::new();
        let allow: &HashSet<String> = match &tier {
            VisibilityTier::Public => &visibility.public_symbols,
            VisibilityTier::Member => &visibility.member_symbols,
            VisibilityTier::Plan(code) => visibility.plan_symbols.get(code).unwrap_or(&empty),
        };

        candidates
            .iter()
            .filter(|symbol| allow.contains(symbol.as_str()))
            .cloned()
            .collect()
    }

    async fn can_view_symbol(&self, tier: VisibilityTier, symbol: &str) -> bool {
        let visibility = self.visibility.read().await;
        let normalized = normalize_symbol(symbol);
        let empty = HashSet::new();
        let allow: &HashSet<String> = match &tier {
            VisibilityTier::Public => &visibility.public_symbols,
            VisibilityTier::Member => &visibility.member_symbols,
            VisibilityTier::Plan(code) => visibility.plan_symbols.get(code).unwrap_or(&empty),
        };
        allow.contains(&normalized)
    }

    async fn symbol_precision(&self, symbol: &str) -> Option<SymbolPrecision> {
        let precisions = self.precisions.read().await;
        precisions.get(&normalize_symbol(symbol)).copied()
    }

    async fn symbol_precisions(&self, symbols: &[String]) -> HashMap<String, SymbolPrecision> {
        let precisions = self.precisions.read().await;
        symbols
            .iter()
            .filter_map(|symbol| {
                precisions
                    .get(&normalize_symbol(symbol))
                    .copied()
                    .map(|precision| (normalize_symbol(symbol), precision))
            })
            .collect()
    }
}

#[async_trait]
impl SymbolRegistryAdminPort for PostgresSymbolRegistryAdapter {
    async fn sync_usdt_symbols(&self, client: &Client) -> Result<usize> {
        sync_usdt_symbols(client, self.pool.clone()).await
    }

    async fn sync_symbols_from_file(&self, file_path: &str) -> Result<usize> {
        sync_symbols_from_file(self.pool.clone(), file_path).await
    }

    async fn set_symbols_enabled(&self, symbols: &[String], enabled: bool) -> Result<u64> {
        set_symbols_enabled(self.pool.clone(), symbols, enabled).await
    }

    async fn set_symbols_visibility(&self, symbols: &[String], visible: bool) -> Result<u64> {
        set_symbols_visibility(self.pool.clone(), symbols, visible).await
    }

    async fn set_all_symbols_visibility(&self, visible: bool) -> Result<u64> {
        set_all_symbols_visibility(self.pool.clone(), visible).await
    }
}

pub async fn load_enabled_symbols(pool: Arc<PgPool>, max: usize) -> Result<Vec<String>> {
    ensure_schema(pool.clone()).await?;
    let client = pool.acquire().await?;
    let limit = i64::try_from(max).unwrap_or(i64::MAX);
    let rows = client
        .client()
        .query(
            "select symbol
               from market.symbol_registry
              where enabled = true
                and exchange_status = 'TRADING'
              order by symbol asc
              limit $1",
            &[&limit],
        )
        .await?;
    Ok(rows
        .into_iter()
        .map(|row| row.get::<_, String>(0))
        .collect())
}

async fn load_registry_snapshot(
    pool: Arc<PgPool>,
) -> Result<(SymbolVisibility, HashMap<String, SymbolPrecision>)> {
    ensure_schema(pool.clone()).await?;
    let client = pool.acquire().await?;

    // 加载 public / member 可见性及精度
    let rows = client
        .client()
        .query(
            "select
                symbol,
                visible_public,
                visible_member,
                price_precision,
                quantity_precision
               from market.symbol_registry
              where enabled = true
                and exchange_status = 'TRADING'",
            &[],
        )
        .await?;

    let mut visibility = SymbolVisibility::default();
    let mut precisions = HashMap::new();
    for row in rows {
        let symbol: String = row.get("symbol");
        if row.get::<_, bool>("visible_public") {
            visibility.public_symbols.insert(symbol.clone());
        }
        if row.get::<_, bool>("visible_member") {
            visibility.member_symbols.insert(symbol.clone());
        }
        precisions.insert(
            symbol,
            SymbolPrecision {
                price_precision: i32::max(row.get::<_, i32>("price_precision"), 0) as u32,
                quantity_precision: i32::max(row.get::<_, i32>("quantity_precision"), 0) as u32,
            },
        );
    }

    // 加载套餐币种关联
    let plan_rows = client
        .client()
        .query(
            "select a.plan_code, a.symbol
               from market.symbol_plan_access a
               join market.symbol_registry r on r.symbol = a.symbol
              where r.enabled = true
                and r.exchange_status = 'TRADING'
              order by a.plan_code, a.symbol",
            &[],
        )
        .await?;

    for row in plan_rows {
        let plan_code: String = row.get("plan_code");
        let symbol: String = row.get("symbol");
        visibility
            .plan_symbols
            .entry(plan_code)
            .or_default()
            .insert(symbol);
    }

    Ok((visibility, precisions))
}

pub async fn set_symbols_enabled(
    pool: Arc<PgPool>,
    symbols: &[String],
    enabled: bool,
) -> Result<u64> {
    ensure_schema(pool.clone()).await?;
    let normalized = normalize_symbol_list(symbols);
    if normalized.is_empty() {
        return Ok(0);
    }

    let mut pooled = pool.acquire().await?;
    let tx = pooled.client_mut().transaction().await?;
    let stmt = tx
        .prepare(
            "update market.symbol_registry
                set enabled = $2,
                    updated_at = now()
              where symbol = $1",
        )
        .await?;

    let mut affected = 0_u64;
    for symbol in normalized {
        affected += tx.execute(&stmt, &[&symbol, &enabled]).await?;
    }
    tx.commit().await?;
    Ok(affected)
}

pub async fn set_symbols_visibility(
    pool: Arc<PgPool>,
    symbols: &[String],
    visible: bool,
) -> Result<u64> {
    ensure_schema(pool.clone()).await?;
    let normalized = normalize_symbol_list(symbols);
    if normalized.is_empty() {
        return Ok(0);
    }

    let mut pooled = pool.acquire().await?;
    let tx = pooled.client_mut().transaction().await?;
    let stmt = tx
        .prepare(
            "update market.symbol_registry
                set visible_public = $2,
                    visible_member = $2,
                    updated_at = now()
              where symbol = $1",
        )
        .await?;

    let mut affected = 0_u64;
    for symbol in normalized {
        affected += tx.execute(&stmt, &[&symbol, &visible]).await?;
    }
    tx.commit().await?;
    Ok(affected)
}

pub async fn set_all_symbols_visibility(pool: Arc<PgPool>, visible: bool) -> Result<u64> {
    ensure_schema(pool.clone()).await?;
    let client = pool.acquire().await?;
    let affected = client
        .client()
        .execute(
            "update market.symbol_registry
                set visible_public = $1,
                    visible_member = $1,
                    updated_at = now()
              where enabled = true
                and exchange_status = 'TRADING'",
            &[&visible],
        )
        .await?;
    Ok(affected)
}

pub async fn ensure_schema(pool: Arc<PgPool>) -> Result<()> {
    SYMBOL_REGISTRY_SCHEMA_READY
        .get_or_try_init(|| async move {
            let client = pool.acquire().await?;
            client
                .client()
                .batch_execute(include_str!(
                    "../../../sql/postgres/market_symbol_registry.sql"
                ))
                .await?;
            Ok::<(), anyhow::Error>(())
        })
        .await?;
    Ok(())
}

fn normalize_symbol(raw: &str) -> String {
    let symbol = raw.trim().to_ascii_uppercase();
    if symbol.ends_with("USDT") {
        symbol
    } else {
        format!("{}USDT", symbol)
    }
}

fn normalize_symbol_list(symbols: &[String]) -> Vec<String> {
    symbols
        .iter()
        .map(|symbol| normalize_symbol(symbol))
        .collect::<HashSet<_>>()
        .into_iter()
        .collect()
}

fn parse_symbol_file(raw: &str) -> Vec<String> {
    raw.lines()
        .map(str::trim)
        .filter(|line| !line.is_empty())
        .map(|line| line.to_ascii_uppercase())
        .collect::<HashSet<_>>()
        .into_iter()
        .collect()
}

fn precision_from_exchange_symbol(symbol: &ExchangeSymbol) -> SymbolPrecision {
    let mut price_precision = 0_u32;
    let mut quantity_precision = 0_u32;

    for filter in &symbol.filters {
        if filter.filter_type == "PRICE_FILTER" {
            if let Some(tick_size) = filter.tick_size.as_deref() {
                price_precision = decimal_precision(tick_size);
            }
        } else if filter.filter_type == "LOT_SIZE" {
            if let Some(step_size) = filter.step_size.as_deref() {
                quantity_precision = decimal_precision(step_size);
            }
        }
    }

    if price_precision == 0 {
        price_precision = symbol
            .quote_precision
            .or(symbol.quote_asset_precision)
            .unwrap_or(0);
    }
    if quantity_precision == 0 {
        quantity_precision = symbol.base_asset_precision.unwrap_or(0);
    }

    SymbolPrecision {
        price_precision,
        quantity_precision,
    }
}

fn decimal_precision(raw: &str) -> u32 {
    let trimmed = raw.trim();
    let Some((_, frac)) = trimmed.split_once('.') else {
        return 0;
    };
    frac.trim_end_matches('0').len() as u32
}
