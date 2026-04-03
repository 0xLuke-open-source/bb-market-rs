use axum::http::{HeaderMap, StatusCode};
use chrono::{DateTime, Utc};

use crate::identity::application::AuthStatusView;
use crate::identity::interfaces::AuthService;
use crate::instrument_catalog::application::SymbolRegistryService;
use crate::instrument_catalog::domain::VisibilityTier;
use crate::terminal::application::projection::{
    AccessInfoJson, DashboardState, FullSnapshot, TraderStateJson,
};

pub async fn require_auth(auth: &AuthService, headers: &HeaderMap) -> Result<(), StatusCode> {
    if auth.status_from_headers(headers).await.authenticated {
        Ok(())
    } else {
        Err(StatusCode::UNAUTHORIZED)
    }
}

pub async fn can_view_symbol(
    symbol_registry: &SymbolRegistryService,
    symbol: &str,
    access: &AuthStatusView,
) -> bool {
    symbol_registry
        .can_view_symbol(display_visibility_tier(access), symbol)
        .await
}

pub async fn visible_keys_for_access(
    symbol_registry: &SymbolRegistryService,
    dashboard: &DashboardState,
    access: &AuthStatusView,
) -> Vec<String> {
    symbol_registry
        .visible_symbols(display_visibility_tier(access), &dashboard.sorted_keys)
        .await
}

pub async fn filter_snapshot(
    symbol_registry: &SymbolRegistryService,
    dashboard: &DashboardState,
    mut snapshot: FullSnapshot,
    access: &AuthStatusView,
) -> FullSnapshot {
    let total_symbols = snapshot.symbols.len();
    if !access.authenticated {
        snapshot.trader = TraderStateJson::default();
    }
    let visible_keys = visible_keys_for_access(symbol_registry, dashboard, access).await;
    let visible_key_set: std::collections::HashSet<&str> =
        visible_keys.iter().map(String::as_str).collect();
    let mut visible_map = std::collections::HashMap::with_capacity(snapshot.symbols.len());
    for symbol in snapshot.symbols.drain(..) {
        visible_map.insert(symbol.symbol.clone(), symbol);
    }
    snapshot.symbols = visible_keys
        .iter()
        .filter_map(|symbol| visible_map.remove(symbol))
        .collect();
    snapshot
        .feed
        .retain(|entry| visible_key_set.contains(entry.symbol.as_str()));
    snapshot.access = build_access_info(access, snapshot.symbols.len(), total_symbols);
    snapshot
}

pub fn build_access_info(
    access: &AuthStatusView,
    visible_symbols: usize,
    total_symbols: usize,
) -> AccessInfoJson {
    let message = if visible_symbols >= total_symbols {
        format!("当前展示全部 {} 个可用币种。", total_symbols)
    } else {
        format!(
            "当前按系统展示状态输出 {} / {} 个币种。",
            visible_symbols, total_symbols
        )
    };

    AccessInfoJson {
        authenticated: access.authenticated,
        subscribed: access.subscribed,
        full_access: access.full_access,
        visible_symbols,
        total_symbols,
        symbol_limit: access.symbol_limit,
        subscription_plan: access.subscription_plan.clone(),
        subscription_expires_at: access.subscription_expires_at.clone(),
        message,
    }
}

pub fn display_visibility_tier(access: &AuthStatusView) -> VisibilityTier {
    if !access.authenticated {
        return VisibilityTier::Public;
    }
    if access.full_access {
        if let Some(plan) = &access.subscription_plan {
            return VisibilityTier::Plan(plan.clone());
        }
    }
    VisibilityTier::Member
}

pub fn parse_query_millis(value: Option<i64>) -> Result<Option<DateTime<Utc>>, StatusCode> {
    value
        .map(|ts| DateTime::<Utc>::from_timestamp_millis(ts).ok_or(StatusCode::BAD_REQUEST))
        .transpose()
}
