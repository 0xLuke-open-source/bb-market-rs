use std::sync::Arc;

use crate::instrument_catalog::application::SymbolRegistryService;
use crate::instrument_catalog::domain::SymbolPrecision;
use crate::market_data::application::runtime::MultiSymbolMonitor;
use crate::terminal::application::bridge::build_symbol_detail;
use crate::terminal::application::projection::{SharedDashboardState, SymbolJson};
use crate::terminal::application::query::TerminalQueryService;

const WS_SYMBOL_BOOK_DEPTH_LIMIT: usize = 18;
const WS_SYMBOL_BIG_TRADES_LIMIT: usize = 16;
const WS_SYMBOL_RECENT_TRADES_LIMIT: usize = 60;
const API_SYMBOL_BOOK_DEPTH_LIMIT: usize = 18;
const API_SYMBOL_BIG_TRADES_LIMIT: usize = 16;
const API_SYMBOL_RECENT_TRADES_LIMIT: usize = 80;
const API_SYMBOL_KLINES_LIMIT: usize = 240;

pub async fn load_symbol_detail(
    monitor: &Arc<MultiSymbolMonitor>,
    symbol_registry: &SymbolRegistryService,
    queries: &TerminalQueryService,
    dashboard: &SharedDashboardState,
    symbol: &str,
) -> Option<SymbolJson> {
    let monitor = monitor.get_monitor(symbol).await?;
    let mut guard = monitor.lock().await;
    let mut detail = build_symbol_detail(symbol, &mut guard);
    drop(guard);
    if let Some(precision) = symbol_registry.symbol_precision(&detail.symbol).await {
        apply_symbol_precision(&mut detail, precision);
    }

    let signal_history = {
        let dashboard = dashboard.read().await;
        dashboard
            .feed
            .iter()
            .filter(|entry| entry.symbol == symbol)
            .take(20)
            .cloned()
            .collect::<Vec<_>>()
    };
    queries
        .decorate_live_snapshot(&mut detail, signal_history)
        .await;
    Some(detail)
}

pub fn strip_symbol_for_api_state(mut symbol: SymbolJson) -> SymbolJson {
    if symbol.top_bids.len() > API_SYMBOL_BOOK_DEPTH_LIMIT {
        symbol.top_bids.truncate(API_SYMBOL_BOOK_DEPTH_LIMIT);
    }
    if symbol.top_asks.len() > API_SYMBOL_BOOK_DEPTH_LIMIT {
        symbol.top_asks.truncate(API_SYMBOL_BOOK_DEPTH_LIMIT);
    }
    if symbol.big_trades.len() > API_SYMBOL_BIG_TRADES_LIMIT {
        symbol.big_trades.truncate(API_SYMBOL_BIG_TRADES_LIMIT);
    }
    if symbol.recent_trades.len() > API_SYMBOL_RECENT_TRADES_LIMIT {
        symbol
            .recent_trades
            .truncate(API_SYMBOL_RECENT_TRADES_LIMIT);
    }
    for bars in symbol.klines.values_mut() {
        trim_tail(bars, API_SYMBOL_KLINES_LIMIT);
    }
    symbol
}

pub fn strip_symbol_for_detail_stream(mut symbol: SymbolJson) -> SymbolJson {
    symbol.klines.clear();
    if symbol.top_bids.len() > WS_SYMBOL_BOOK_DEPTH_LIMIT {
        symbol.top_bids.truncate(WS_SYMBOL_BOOK_DEPTH_LIMIT);
    }
    if symbol.top_asks.len() > WS_SYMBOL_BOOK_DEPTH_LIMIT {
        symbol.top_asks.truncate(WS_SYMBOL_BOOK_DEPTH_LIMIT);
    }
    if symbol.big_trades.len() > WS_SYMBOL_BIG_TRADES_LIMIT {
        symbol.big_trades.truncate(WS_SYMBOL_BIG_TRADES_LIMIT);
    }
    if symbol.recent_trades.len() > WS_SYMBOL_RECENT_TRADES_LIMIT {
        symbol.recent_trades.truncate(WS_SYMBOL_RECENT_TRADES_LIMIT);
    }
    symbol
}

fn apply_symbol_precision(symbol: &mut SymbolJson, precision: SymbolPrecision) {
    if precision.price_precision > 0 {
        symbol.price_precision = precision.price_precision;
    }
    if precision.quantity_precision > 0 {
        symbol.quantity_precision = precision.quantity_precision;
    }
}

fn trim_tail<T>(items: &mut Vec<T>, limit: usize) {
    if items.len() > limit {
        items.drain(0..items.len() - limit);
    }
}
