// src/web/server.rs — 交易员版 Dashboard
//
// 这个模块只负责“对外提供接口”，不负责计算业务数据。
// 它依赖两个上游对象：
// 1. DashboardState：分析后的展示快照
// 2. SpotTradingService：本地下单 / 撤单 / 回放服务
//
// 设计原则：信号优先，数字为辅
// ─ 强信号触发时：全屏弹出声音 + 大字提醒
// ─ 数据平滑：3秒滚动均值，不闪烁
// ─ 布局：左侧信号墙(最重要) + 右侧币种状态 + 底部详情

use crate::analysis::multi_monitor::MultiSymbolMonitor;
use crate::market::big_trade::{BigTradeHistoryRecord, BigTradeQueryService, BigTradeStatsRecord};
use crate::market::panel::{
    SignalPerformanceSampleRecord, SymbolPanelPersistenceService, SymbolPanelQueryService,
    SymbolPanelSnapshotRecord,
};
use crate::market::trade::RecentTradeQueryService;
use crate::symbols::sync_symbols::{SymbolRegistryService, VisibilityTier};
use crate::web::bridge::build_symbol_detail;
use crate::web::auth::{AuthRequest, AuthService, AuthStatusResponse, SubscribeRequest};
use crate::web::spot::{
    ApiOrderRequest, ApiResponse, CancelAllRequest, CancelAllResult, OrderActionResult,
    ReplayQuery, ReplayResponse, SpotTradingService,
};
use crate::web::state::{
    AccessInfoJson, BigTradeJson, DashboardState, FullSnapshot, SharedDashboardState, SymbolJson,
    TraderStateJson,
};
use axum::{
    extract::ws::{Message, WebSocket},
    extract::{Path, Query, State, WebSocketUpgrade},
    http::{header, HeaderMap, HeaderValue, StatusCode},
    response::{Html, IntoResponse, Json, Response},
    routing::{delete, get, post},
    Router,
};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::{HashMap, HashSet};
use std::path::PathBuf;
use std::sync::Arc;
use tokio::time::{interval, Duration, MissedTickBehavior};
use tower_http::cors::CorsLayer;
use tower_http::services::ServeDir;

const WS_PUSH_INTERVAL_MS: u64 = 900;
const WS_PUSH_IDLE_INTERVAL_MS: u64 = 1800;
const WS_FULL_SYNC_INTERVAL_SECS: u64 = 12;
const WS_SYMBOL_PUSH_INTERVAL_MS: u64 = 260;
const WS_SYMBOL_FULL_SYNC_INTERVAL_SECS: u64 = 4;
const WS_FEED_LIMIT: usize = 32;
const WS_SYMBOL_BOOK_DEPTH_LIMIT: usize = 18;
const WS_SYMBOL_BIG_TRADES_LIMIT: usize = 16;
const WS_SYMBOL_RECENT_TRADES_LIMIT: usize = 60;
const WS_TRADER_OPEN_ORDERS_LIMIT: usize = 64;
const WS_TRADER_HISTORY_LIMIT: usize = 40;
const API_SYMBOL_BOOK_DEPTH_LIMIT: usize = 18;
const API_SYMBOL_BIG_TRADES_LIMIT: usize = 16;
const API_SYMBOL_RECENT_TRADES_LIMIT: usize = 80;
const API_SYMBOL_KLINES_LIMIT: usize = 240;
#[derive(Debug, Serialize)]
struct CompactWsSnapshot {
    #[serde(rename = "k")]
    kind: &'static str,
    #[serde(rename = "u")]
    total_updates: u64,
    #[serde(rename = "up")]
    uptime_secs: u64,
    #[serde(rename = "a")]
    access: AccessInfoJson,
    #[serde(rename = "t")]
    trader: TraderStateJson,
    #[serde(rename = "f")]
    feed: Vec<CompactFeedRow>,
    #[serde(rename = "s")]
    symbols: Vec<CompactSymbolRow>,
}

#[derive(Debug, Clone, PartialEq, Serialize)]
struct CompactFeedRow(String, String, String, Option<u8>, String, i64);

#[derive(Debug, Clone, PartialEq, Serialize)]
struct CompactSymbolRow(
    String,
    String,
    String,
    String,
    f64,
    f64,
    f64,
    f64,
    u32,
    u32,
    f64,
    f64,
    f64,
    f64,
    f64,
    f64,
    f64,
    f64,
    f64,
    f64,
    u8,
    u8,
    bool,
    bool,
    bool,
    bool,
    bool,
    f64,
    f64,
    f64,
    f64,
    u32,
    u8,
    u64,
);

#[derive(Debug, Serialize)]
struct CompactWsDelta {
    #[serde(rename = "k")]
    kind: &'static str,
    #[serde(rename = "u")]
    total_updates: u64,
    #[serde(rename = "up")]
    uptime_secs: u64,
    #[serde(rename = "a", skip_serializing_if = "Option::is_none")]
    access: Option<AccessInfoJson>,
    #[serde(rename = "t", skip_serializing_if = "Option::is_none")]
    trader: Option<TraderStateJson>,
    #[serde(rename = "f", skip_serializing_if = "Vec::is_empty")]
    feed: Vec<CompactFeedRow>,
    #[serde(rename = "s", skip_serializing_if = "Vec::is_empty")]
    symbols: Vec<CompactSymbolRow>,
    #[serde(rename = "rm", skip_serializing_if = "Vec::is_empty")]
    removed_symbols: Vec<String>,
}

#[derive(Clone)]
struct AppState {
    monitor: Arc<MultiSymbolMonitor>,
    symbol_registry: SymbolRegistryService,
    panel_runtime: SymbolPanelPersistenceService,
    panel_query: SymbolPanelQueryService,
    recent_trade_query: RecentTradeQueryService,
    big_trade_query: BigTradeQueryService,
    // dashboard 是“展示域状态”。
    dashboard: SharedDashboardState,
    // spot 是“交易域服务”。
    spot: SpotTradingService,
    // auth 是“访问控制服务”。
    auth: AuthService,
}

pub async fn run_server(
    monitor: Arc<MultiSymbolMonitor>,
    symbol_registry: SymbolRegistryService,
    panel_runtime: SymbolPanelPersistenceService,
    panel_query: SymbolPanelQueryService,
    recent_trade_query: RecentTradeQueryService,
    big_trade_query: BigTradeQueryService,
    dashboard: SharedDashboardState,
    spot: SpotTradingService,
    auth: AuthService,
    port: u16,
) -> anyhow::Result<()> {
    // 这里把所有前端需要的接口集中挂到一个 Router 上。
    // Dashboard HTML 是静态内容，实时数据则通过 API / WebSocket 提供。
    let state = AppState {
        monitor,
        symbol_registry,
        panel_runtime,
        panel_query,
        recent_trade_query,
        big_trade_query,
        dashboard,
        spot,
        auth,
    };
    let dashboard_root = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("src/web/dashboard");
    let app = Router::new()
        .route("/", get(serve_dashboard))
        .route("/favicon.ico", get(serve_favicon))
        .route("/api/auth/me", get(api_auth_me))
        .route("/api/auth/register", post(api_auth_register))
        .route("/api/auth/login", post(api_auth_login))
        .route("/api/auth/plans", get(api_auth_plans))
        .route("/api/auth/subscribe", post(api_auth_subscribe))
        .route("/api/auth/logout", post(api_auth_logout))
        .route("/api/auth/favorites", get(api_auth_favorites))
        .route(
            "/api/auth/favorites/:symbol",
            post(api_auth_favorite_add).delete(api_auth_favorite_remove),
        )
        .route("/api/state", get(api_full_state))
        .route("/api/symbol/:symbol", get(api_symbol_state))
        .route("/api/symbols", get(api_symbol_list))
        .route("/api/big-trades/:symbol", get(api_big_trade_history))
        .route("/api/big-trades/stats/:symbol", get(api_big_trade_stats))
        .route("/api/panel/perf/:symbol", get(api_panel_perf_history))
        .route("/api/panel/:symbol", get(api_panel_history))
        .route("/api/trades/:symbol", get(api_recent_trades))
        .route("/api/spot/state", get(api_spot_state))
        .route("/api/spot/replay", get(api_spot_replay))
        .route("/api/spot/order", post(api_submit_order))
        .route("/api/spot/order/:order_id", delete(api_cancel_order))
        .route("/api/spot/cancel_all", post(api_cancel_all))
        .route("/ws", get(ws_handler))
        .route("/ws/symbol/:symbol", get(ws_symbol_handler))
        .nest_service("/static", ServeDir::new(dashboard_root))
        .layer(CorsLayer::permissive())
        .with_state(state);

    let addr = format!("0.0.0.0:{}", port);
    println!("🌐 交易员 Dashboard: http://127.0.0.1:{}", port);
    let listener = tokio::net::TcpListener::bind(&addr).await?;
    axum::serve(listener, app).await?;
    Ok(())
}

async fn serve_dashboard() -> Html<&'static str> {
    Html(include_str!(concat!(
        env!("OUT_DIR"),
        "/dashboard_index.html"
    )))
}

async fn serve_favicon() -> StatusCode {
    StatusCode::NO_CONTENT
}

async fn api_auth_me(headers: HeaderMap, State(state): State<AppState>) -> impl IntoResponse {
    Json(state.auth.me_from_headers(&headers).await)
}

async fn api_auth_plans(State(state): State<AppState>) -> impl IntoResponse {
    Json(state.auth.plans().await)
}

async fn api_auth_register(
    State(state): State<AppState>,
    Json(req): Json<AuthRequest>,
) -> impl IntoResponse {
    match state.auth.register(req).await {
        Ok((data, token)) => json_with_cookie(
            StatusCode::OK,
            AuthService::session_cookie(&token),
            true,
            "ok",
            data,
        ),
        Err(err) => json_response(
            StatusCode::BAD_REQUEST,
            false,
            &err.to_string(),
            AuthStatusResponse::default(),
        ),
    }
}

async fn api_auth_login(
    State(state): State<AppState>,
    Json(req): Json<AuthRequest>,
) -> impl IntoResponse {
    match state.auth.login(req).await {
        Ok((data, token)) => json_with_cookie(
            StatusCode::OK,
            AuthService::session_cookie(&token),
            true,
            "ok",
            data,
        ),
        Err(err) => json_response(
            StatusCode::UNAUTHORIZED,
            false,
            &err.to_string(),
            AuthStatusResponse::default(),
        ),
    }
}

async fn api_auth_logout(headers: HeaderMap, State(state): State<AppState>) -> impl IntoResponse {
    if let Some(token) = state.auth.session_token_from_headers(&headers) {
        state.auth.logout(&token).await;
    }
    json_with_cookie(
        StatusCode::OK,
        AuthService::clear_cookie(),
        true,
        "ok",
        AuthStatusResponse::default(),
    )
}

async fn api_auth_favorites(headers: HeaderMap, State(state): State<AppState>) -> impl IntoResponse {
    let Some(token) = state.auth.session_token_from_headers(&headers) else {
        return json_response(
            StatusCode::UNAUTHORIZED,
            false,
            "请先登录后再查看收藏",
            Vec::<String>::new(),
        );
    };
    match state.auth.favorite_symbols(&token).await {
        Ok(symbols) => json_response(StatusCode::OK, true, "ok", symbols),
        Err(err) => json_response(
            StatusCode::BAD_REQUEST,
            false,
            &err.to_string(),
            Vec::<String>::new(),
        ),
    }
}

async fn api_auth_favorite_add(
    headers: HeaderMap,
    Path(symbol): Path<String>,
    State(state): State<AppState>,
) -> impl IntoResponse {
    let Some(token) = state.auth.session_token_from_headers(&headers) else {
        return json_response(
            StatusCode::UNAUTHORIZED,
            false,
            "请先登录后再收藏",
            Vec::<String>::new(),
        );
    };
    match state.auth.add_favorite_symbol(&token, &symbol).await {
        Ok(symbols) => json_response(StatusCode::OK, true, "收藏成功", symbols),
        Err(err) => json_response(
            StatusCode::BAD_REQUEST,
            false,
            &err.to_string(),
            Vec::<String>::new(),
        ),
    }
}

async fn api_auth_favorite_remove(
    headers: HeaderMap,
    Path(symbol): Path<String>,
    State(state): State<AppState>,
) -> impl IntoResponse {
    let Some(token) = state.auth.session_token_from_headers(&headers) else {
        return json_response(
            StatusCode::UNAUTHORIZED,
            false,
            "请先登录后再操作收藏",
            Vec::<String>::new(),
        );
    };
    match state.auth.remove_favorite_symbol(&token, &symbol).await {
        Ok(symbols) => json_response(StatusCode::OK, true, "已取消收藏", symbols),
        Err(err) => json_response(
            StatusCode::BAD_REQUEST,
            false,
            &err.to_string(),
            Vec::<String>::new(),
        ),
    }
}

async fn api_auth_subscribe(
    headers: HeaderMap,
    State(state): State<AppState>,
    Json(req): Json<SubscribeRequest>,
) -> impl IntoResponse {
    let Some(token) = state.auth.session_token_from_headers(&headers) else {
        return json_response(
            StatusCode::UNAUTHORIZED,
            false,
            "请先登录后再订阅",
            AuthStatusResponse::default(),
        );
    };
    match state.auth.subscribe(&token, &req.plan_code).await {
        Ok(data) => json_response(StatusCode::OK, true, "订阅已激活", data),
        Err(err) => json_response(
            StatusCode::BAD_REQUEST,
            false,
            &err.to_string(),
            AuthStatusResponse::default(),
        ),
    }
}

async fn api_full_state(
    headers: HeaderMap,
    State(state): State<AppState>,
) -> Result<Json<FullSnapshot>, StatusCode> {
    let access = state.auth.status_from_headers(&headers).await;
    let trader = state.spot.snapshot().await;
    let dashboard = state.dashboard.read().await;
    let snapshot = dashboard.to_light_snapshot(trader);
    Ok(Json(filter_snapshot(&state, &dashboard, snapshot, &access).await))
}

async fn api_symbol_state(
    headers: HeaderMap,
    Path(symbol): Path<String>,
    State(state): State<AppState>,
) -> Result<Json<Option<SymbolJson>>, StatusCode> {
    let access = state.auth.status_from_headers(&headers).await;
    let dashboard = state.dashboard.read().await;
    if !can_view_symbol(&state, &symbol, &access).await {
        return Ok(Json(None));
    }
    drop(dashboard);

    Ok(Json(
        load_symbol_detail(&state, &symbol)
            .await
            .map(strip_symbol_for_api_state),
    ))
}

async fn api_symbol_list(
    headers: HeaderMap,
    State(state): State<AppState>,
) -> Result<Json<Vec<String>>, StatusCode> {
    let access = state.auth.status_from_headers(&headers).await;
    let dashboard = state.dashboard.read().await;
    Ok(Json(visible_keys_for_access(&state, &dashboard, &access).await))
}

#[derive(Debug, Deserialize, Default)]
struct TradesQuery {
    limit: Option<usize>,
}

#[derive(Debug, Deserialize, Default)]
struct PanelHistoryQuery {
    limit: Option<usize>,
    from: Option<i64>,
    to: Option<i64>,
}

#[derive(Debug, Serialize)]
struct BigTradeHistoryItemJson {
    symbol: String,
    agg_trade_id: i64,
    event_time: String,
    event_ts: i64,
    trade_time: String,
    trade_ts: i64,
    price: f64,
    quantity: f64,
    quote_quantity: f64,
    threshold_quantity: f64,
    is_taker_buy: bool,
    is_buyer_maker: bool,
}

#[derive(Debug, Serialize)]
struct BigTradeStatsJson {
    symbol: String,
    total_count: i64,
    buy_count: i64,
    sell_count: i64,
    total_quote_quantity: f64,
    buy_quote_quantity: f64,
    sell_quote_quantity: f64,
    buy_ratio: f64,
    sell_ratio: f64,
    avg_quote_quantity: f64,
    max_quote_quantity: f64,
    avg_threshold_quantity: f64,
    first_trade_time: Option<String>,
    first_trade_ts: Option<i64>,
    last_trade_time: Option<String>,
    last_trade_ts: Option<i64>,
}

#[derive(Debug, Serialize)]
struct PanelSnapshotJson {
    event_ts: i64,
    event_time: String,
    bid: f64,
    ask: f64,
    mid: f64,
    spread_bps: f64,
    change_24h_pct: f64,
    high_24h: f64,
    low_24h: f64,
    volume_24h: f64,
    quote_vol_24h: f64,
    ofi: f64,
    ofi_raw: f64,
    obi: f64,
    trend_strength: f64,
    cvd: f64,
    taker_buy_ratio: f64,
    pump_score: i32,
    dump_score: i32,
    pump_signal: bool,
    dump_signal: bool,
    whale_entry: bool,
    whale_exit: bool,
    bid_eating: bool,
    total_bid_volume: f64,
    total_ask_volume: f64,
    max_bid_ratio: f64,
    max_ask_ratio: f64,
    anomaly_count_1m: i32,
    anomaly_max_severity: i32,
    status_summary: String,
    watch_level: String,
    signal_reason: String,
    sentiment: String,
    risk_level: String,
    recommendation: String,
    whale_type: String,
    pump_probability: i32,
    price_precision: i32,
    quantity_precision: i32,
    snapshot: Value,
    signal_history: Value,
    factor_metrics: Value,
    enterprise_metrics: Value,
    update_count: i64,
}

#[derive(Debug, Serialize)]
struct SignalPerformanceSampleJson {
    sample_id: String,
    symbol: String,
    signal_type: String,
    triggered_at: String,
    triggered_ts: i64,
    trigger_price: f64,
    trigger_score: i32,
    watch_level: String,
    signal_reason: String,
    update_count: i64,
    resolved_5m: bool,
    resolved_15m: bool,
    resolved_decay: bool,
    outcome_5m_return: Option<f64>,
    outcome_5m_win: Option<bool>,
    outcome_5m_at: Option<String>,
    outcome_5m_ts: Option<i64>,
    outcome_15m_return: Option<f64>,
    outcome_15m_win: Option<bool>,
    outcome_15m_at: Option<String>,
    outcome_15m_ts: Option<i64>,
    decay_minutes: Option<f64>,
    decay_at: Option<String>,
    decay_ts: Option<i64>,
    created_at: String,
    created_ts: i64,
}

async fn api_recent_trades(
    headers: HeaderMap,
    Path(symbol): Path<String>,
    Query(query): Query<TradesQuery>,
    State(state): State<AppState>,
) -> Result<Json<Vec<BigTradeJson>>, StatusCode> {
    let access = state.auth.status_from_headers(&headers).await;
    let normalized_symbol = symbol.trim().to_ascii_uppercase();
    if !can_view_symbol(&state, &normalized_symbol, &access).await {
        return Err(StatusCode::FORBIDDEN);
    }

    let limit = query.limit.unwrap_or(120).clamp(1, 500);
    let trades = state
        .recent_trade_query
        .load_recent_trades(&normalized_symbol, limit)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    Ok(Json(trades))
}

async fn api_big_trade_history(
    headers: HeaderMap,
    Path(symbol): Path<String>,
    Query(query): Query<PanelHistoryQuery>,
    State(state): State<AppState>,
) -> Result<Json<Vec<BigTradeHistoryItemJson>>, StatusCode> {
    let access = state.auth.status_from_headers(&headers).await;
    let normalized_symbol = symbol.trim().to_ascii_uppercase();
    if !can_view_symbol(&state, &normalized_symbol, &access).await {
        return Err(StatusCode::FORBIDDEN);
    }

    let limit = query.limit.unwrap_or(120).clamp(1, 500);
    let from = parse_query_millis(query.from)?;
    let to = parse_query_millis(query.to)?;
    if let (Some(from_ts), Some(to_ts)) = (from.as_ref(), to.as_ref()) {
        if from_ts > to_ts {
            return Err(StatusCode::BAD_REQUEST);
        }
    }

    let records = state
        .big_trade_query
        .load_big_trades(&normalized_symbol, limit, from, to)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    Ok(Json(
        records
            .into_iter()
            .map(big_trade_history_item_json_from_record)
            .collect(),
    ))
}

async fn api_big_trade_stats(
    headers: HeaderMap,
    Path(symbol): Path<String>,
    Query(query): Query<PanelHistoryQuery>,
    State(state): State<AppState>,
) -> Result<Json<BigTradeStatsJson>, StatusCode> {
    let access = state.auth.status_from_headers(&headers).await;
    let normalized_symbol = symbol.trim().to_ascii_uppercase();
    if !can_view_symbol(&state, &normalized_symbol, &access).await {
        return Err(StatusCode::FORBIDDEN);
    }

    let from = parse_query_millis(query.from)?;
    let to = parse_query_millis(query.to)?;
    if let (Some(from_ts), Some(to_ts)) = (from.as_ref(), to.as_ref()) {
        if from_ts > to_ts {
            return Err(StatusCode::BAD_REQUEST);
        }
    }

    let stats = state
        .big_trade_query
        .load_big_trade_stats(&normalized_symbol, from, to)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    Ok(Json(big_trade_stats_json_from_record(stats)))
}

async fn api_panel_history(
    headers: HeaderMap,
    Path(symbol): Path<String>,
    Query(query): Query<PanelHistoryQuery>,
    State(state): State<AppState>,
) -> Result<Json<Vec<PanelSnapshotJson>>, StatusCode> {
    let access = state.auth.status_from_headers(&headers).await;
    let normalized_symbol = symbol.trim().to_ascii_uppercase();
    if !can_view_symbol(&state, &normalized_symbol, &access).await {
        return Err(StatusCode::FORBIDDEN);
    }

    let limit = query.limit.unwrap_or(120).clamp(1, 500);
    let from = parse_query_millis(query.from)?;
    let to = parse_query_millis(query.to)?;
    if let (Some(from_ts), Some(to_ts)) = (from.as_ref(), to.as_ref()) {
        if from_ts > to_ts {
            return Err(StatusCode::BAD_REQUEST);
        }
    }
    let snapshots = state
        .panel_query
        .load_recent_snapshots(&normalized_symbol, limit, from, to)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    Ok(Json(
        snapshots
            .into_iter()
            .map(panel_snapshot_json_from_record)
            .collect(),
    ))
}

async fn api_panel_perf_history(
    headers: HeaderMap,
    Path(symbol): Path<String>,
    Query(query): Query<TradesQuery>,
    State(state): State<AppState>,
) -> Result<Json<Vec<SignalPerformanceSampleJson>>, StatusCode> {
    let access = state.auth.status_from_headers(&headers).await;
    let normalized_symbol = symbol.trim().to_ascii_uppercase();
    if !can_view_symbol(&state, &normalized_symbol, &access).await {
        return Err(StatusCode::FORBIDDEN);
    }

    let limit = query.limit.unwrap_or(120).clamp(1, 500);
    let samples = state
        .panel_query
        .load_signal_perf_samples(&normalized_symbol, limit)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    Ok(Json(
        samples
            .into_iter()
            .map(signal_perf_sample_json_from_record)
            .collect(),
    ))
}

async fn api_spot_state(
    headers: HeaderMap,
    State(state): State<AppState>,
) -> Result<Json<TraderStateJson>, StatusCode> {
    require_auth(&state, &headers).await?;
    Ok(Json(state.spot.snapshot().await))
}

async fn api_spot_replay(
    headers: HeaderMap,
    State(state): State<AppState>,
    Query(query): Query<ReplayQuery>,
) -> impl IntoResponse {
    if require_auth(&state, &headers).await.is_err() {
        return json_response(
            StatusCode::UNAUTHORIZED,
            false,
            "请先登录",
            ReplayResponse::default(),
        );
    }
    match state.spot.replay(query).await {
        Ok(data) => json_response(StatusCode::OK, true, "ok", data),
        Err(err) => json_response(
            StatusCode::BAD_REQUEST,
            false,
            &err.to_string(),
            ReplayResponse::default(),
        ),
    }
}

async fn api_submit_order(
    headers: HeaderMap,
    State(state): State<AppState>,
    Json(req): Json<ApiOrderRequest>,
) -> impl IntoResponse {
    if require_auth(&state, &headers).await.is_err() {
        return json_response(
            StatusCode::UNAUTHORIZED,
            false,
            "请先登录",
            OrderActionResult::default(),
        );
    }
    match state.spot.submit_order(req).await {
        Ok(data) => json_response(StatusCode::OK, true, "ok", data),
        Err(err) => json_response(
            StatusCode::BAD_REQUEST,
            false,
            &err.to_string(),
            OrderActionResult::default(),
        ),
    }
}

async fn api_cancel_order(
    headers: HeaderMap,
    Path(order_id): Path<u64>,
    State(state): State<AppState>,
) -> impl IntoResponse {
    if require_auth(&state, &headers).await.is_err() {
        return json_response(
            StatusCode::UNAUTHORIZED,
            false,
            "请先登录",
            OrderActionResult::default(),
        );
    }
    match state.spot.cancel_order(order_id).await {
        Ok(data) => json_response(StatusCode::OK, true, "ok", data),
        Err(err) => json_response(
            StatusCode::BAD_REQUEST,
            false,
            &err.to_string(),
            OrderActionResult::default(),
        ),
    }
}

async fn api_cancel_all(
    headers: HeaderMap,
    State(state): State<AppState>,
    Json(req): Json<CancelAllRequest>,
) -> impl IntoResponse {
    if require_auth(&state, &headers).await.is_err() {
        return json_response(
            StatusCode::UNAUTHORIZED,
            false,
            "请先登录",
            CancelAllResult::default(),
        );
    }
    match state.spot.cancel_all(req).await {
        Ok(data) => json_response(StatusCode::OK, true, "ok", data),
        Err(err) => json_response(
            StatusCode::BAD_REQUEST,
            false,
            &err.to_string(),
            CancelAllResult::default(),
        ),
    }
}

async fn ws_handler(
    ws: WebSocketUpgrade,
    headers: HeaderMap,
    State(state): State<AppState>,
) -> impl IntoResponse {
    let access = state.auth.status_from_headers(&headers).await;
    ws.on_upgrade(move |socket| ws_loop(socket, state, access))
}

async fn ws_symbol_handler(
    ws: WebSocketUpgrade,
    headers: HeaderMap,
    Path(symbol): Path<String>,
    State(state): State<AppState>,
) -> impl IntoResponse {
    let access = state.auth.status_from_headers(&headers).await;
    let symbol = symbol.trim().to_ascii_uppercase();
    if !can_view_symbol(&state, &symbol, &access).await {
        return StatusCode::FORBIDDEN.into_response();
    }
    ws.on_upgrade(move |socket| ws_symbol_loop(socket, state, access, symbol))
}

fn compact_feed_row_key(row: &CompactFeedRow) -> String {
    format!("{}|{}|{}|{}|{}", row.5, row.1, row.2, row.0, row.4)
}

async fn ws_loop(mut socket: WebSocket, state: AppState, access: AuthStatusResponse) {
    // 全市场流采用“首包全量 + 后续增量 + 定期全量校准”策略。
    // 有变化时推增量，无变化时自动降频，减少前端无效刷新与网络开销。
    let mut poll_ms = WS_PUSH_INTERVAL_MS;
    let mut idle_rounds: u32 = 0;
    let mut last_full_sync = std::time::Instant::now()
        .checked_sub(Duration::from_secs(WS_FULL_SYNC_INTERVAL_SECS))
        .unwrap_or_else(std::time::Instant::now);
    let mut last_symbols: HashMap<String, CompactSymbolRow> = HashMap::new();
    let mut last_feed_keys: HashSet<String> = HashSet::new();
    let mut last_access_json = String::new();
    let mut last_trader_json = String::new();
    loop {
        let snapshot = {
            let trader = state.spot.snapshot().await;
            let dashboard = state.dashboard.read().await;
            build_compact_ws_snapshot(&state, &dashboard, trader, &access).await
        };

        let access_json = serde_json::to_string(&snapshot.access).unwrap_or_default();
        let trader_json = serde_json::to_string(&snapshot.trader).unwrap_or_default();
        let access_changed = access_json != last_access_json;
        let trader_changed = trader_json != last_trader_json;

        let mut current_symbols = HashMap::with_capacity(snapshot.symbols.len());
        for row in &snapshot.symbols {
            current_symbols.insert(row.0.clone(), row.clone());
        }

        let changed_symbols: Vec<CompactSymbolRow> = current_symbols
            .iter()
            .filter_map(|(symbol, row)| {
                if last_symbols.get(symbol) == Some(row) {
                    None
                } else {
                    Some(row.clone())
                }
            })
            .collect();
        let removed_symbols: Vec<String> = last_symbols
            .keys()
            .filter(|symbol| !current_symbols.contains_key(*symbol))
            .cloned()
            .collect();

        let current_feed_keys: HashSet<String> =
            snapshot.feed.iter().map(compact_feed_row_key).collect();
        let feed_delta: Vec<CompactFeedRow> = snapshot
            .feed
            .iter()
            .filter_map(|row| {
                let key = compact_feed_row_key(row);
                if last_feed_keys.contains(&key) {
                    None
                } else {
                    Some(row.clone())
                }
            })
            .collect();

        let force_full =
            last_symbols.is_empty() || last_full_sync.elapsed().as_secs() >= WS_FULL_SYNC_INTERVAL_SECS;
        let has_delta = !changed_symbols.is_empty()
            || !removed_symbols.is_empty()
            || !feed_delta.is_empty()
            || access_changed
            || trader_changed;

        let mut sent = false;
        if force_full {
            if let Some(payload) = encode_ws_binary(&snapshot) {
                if socket.send(Message::Binary(payload.into())).await.is_err() {
                    break;
                }
                sent = true;
                last_full_sync = std::time::Instant::now();
            }
        } else if has_delta {
            let delta = CompactWsDelta {
                kind: "m2",
                total_updates: snapshot.total_updates,
                uptime_secs: snapshot.uptime_secs,
                access: if access_changed {
                    Some(snapshot.access.clone())
                } else {
                    None
                },
                trader: if trader_changed {
                    Some(snapshot.trader.clone())
                } else {
                    None
                },
                feed: feed_delta,
                symbols: changed_symbols,
                removed_symbols,
            };
            if let Some(payload) = encode_ws_binary(&delta) {
                if socket.send(Message::Binary(payload.into())).await.is_err() {
                    break;
                }
                sent = true;
            }
        }

        last_symbols = current_symbols;
        last_feed_keys = current_feed_keys;
        last_access_json = access_json;
        last_trader_json = trader_json;

        if sent {
            idle_rounds = 0;
            poll_ms = WS_PUSH_INTERVAL_MS;
        } else {
            idle_rounds = idle_rounds.saturating_add(1);
            if idle_rounds >= 3 {
                poll_ms = WS_PUSH_IDLE_INTERVAL_MS;
            }
        }
        tokio::time::sleep(Duration::from_millis(poll_ms)).await;
    }
}

async fn ws_symbol_loop(
    mut socket: WebSocket,
    state: AppState,
    access: AuthStatusResponse,
    symbol: String,
) {
    let mut tick = interval(Duration::from_millis(WS_SYMBOL_PUSH_INTERVAL_MS));
    tick.set_missed_tick_behavior(MissedTickBehavior::Skip);
    let mut last_payload = Vec::new();
    let mut last_full_sync = std::time::Instant::now()
        .checked_sub(Duration::from_secs(WS_SYMBOL_FULL_SYNC_INTERVAL_SECS))
        .unwrap_or_else(std::time::Instant::now);
    loop {
        let payload = {
            let dashboard = state.dashboard.read().await;
            if !can_view_symbol(&state, &symbol, &access).await {
                break;
            }
            drop(dashboard);

            let Some(detail) = load_symbol_detail(&state, &symbol)
                .await
                .map(strip_symbol_for_detail_stream)
            else {
                continue;
            };
            match encode_ws_binary(&detail) {
                Some(bytes) => bytes,
                None => continue,
            }
        };
        let force_full = last_payload.is_empty()
            || last_full_sync.elapsed().as_secs() >= WS_SYMBOL_FULL_SYNC_INTERVAL_SECS;
        if force_full || payload != last_payload {
            if socket.send(Message::Binary(payload.clone().into())).await.is_err() {
                break;
            }
            last_payload = payload;
            last_full_sync = std::time::Instant::now();
        }
        tick.tick().await;
    }
}

fn encode_ws_binary<T: Serialize>(value: &T) -> Option<Vec<u8>> {
    rmp_serde::to_vec_named(value).ok()
}

async fn load_symbol_detail(state: &AppState, symbol: &str) -> Option<SymbolJson> {
    let monitor = state.monitor.get_monitor(symbol).await?;
    let mut guard = monitor.lock().await;
    let mut detail = build_symbol_detail(symbol, &mut guard);
    drop(guard);
    state.symbol_registry.apply_symbol_precision(&mut detail).await;

    let signal_history = {
        let dashboard = state.dashboard.read().await;
        dashboard
            .feed
            .iter()
            .filter(|entry| entry.symbol == symbol)
            .take(20)
            .cloned()
            .collect::<Vec<_>>()
    };
    state
        .panel_runtime
        .decorate_live_snapshot(&mut detail, signal_history)
        .await;
    Some(detail)
}

async fn require_auth(state: &AppState, headers: &HeaderMap) -> Result<(), StatusCode> {
    if state.auth.status_from_headers(headers).await.authenticated {
        Ok(())
    } else {
        Err(StatusCode::UNAUTHORIZED)
    }
}

async fn filter_snapshot(
    state: &AppState,
    dashboard: &DashboardState,
    mut snapshot: FullSnapshot,
    access: &AuthStatusResponse,
) -> FullSnapshot {
    let total_symbols = snapshot.symbols.len();
    if !access.authenticated {
        snapshot.trader = TraderStateJson::default();
    }
    let visible_keys = visible_keys_for_access(state, dashboard, access).await;
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

fn build_access_info(
    access: &AuthStatusResponse,
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

async fn can_view_symbol(state: &AppState, symbol: &str, access: &AuthStatusResponse) -> bool {
    state
        .symbol_registry
        .can_view_symbol(display_visibility_tier(access), symbol)
        .await
}

async fn build_compact_ws_snapshot(
    state: &AppState,
    dashboard: &DashboardState,
    trader: TraderStateJson,
    access: &AuthStatusResponse,
) -> CompactWsSnapshot {
    let total_symbols = dashboard.sorted_keys.len();
    let visible_keys = visible_keys_for_access(state, dashboard, access).await;
    let visible_set = visible_keys
        .iter()
        .map(String::as_str)
        .collect::<std::collections::HashSet<&str>>();
    let symbols = visible_keys
        .iter()
        .filter_map(|key| dashboard.symbols.get(key))
        .map(CompactSymbolRow::from)
        .collect();
    let feed = dashboard
        .feed
        .iter()
        .filter(|entry| visible_set.contains(entry.symbol.as_str()))
        .take(WS_FEED_LIMIT)
        .map(CompactFeedRow::from)
        .collect();

    CompactWsSnapshot {
        kind: "m1",
        total_updates: dashboard.total_updates,
        uptime_secs: dashboard.start_time.elapsed().as_secs(),
        access: build_access_info(access, visible_keys.len(), total_symbols),
        trader: compact_trader_for_ws(trader, access.authenticated),
        feed,
        symbols,
    }
}

async fn visible_keys_for_access(
    state: &AppState,
    dashboard: &DashboardState,
    access: &AuthStatusResponse,
) -> Vec<String> {
    state
        .symbol_registry
        .visible_symbols(display_visibility_tier(access), &dashboard.sorted_keys)
        .await
}

fn display_visibility_tier(access: &AuthStatusResponse) -> VisibilityTier {
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

fn panel_snapshot_json_from_record(record: SymbolPanelSnapshotRecord) -> PanelSnapshotJson {
    PanelSnapshotJson {
        event_ts: record.event_time.timestamp_millis(),
        event_time: record.event_time.to_rfc3339(),
        bid: record.bid,
        ask: record.ask,
        mid: record.mid,
        spread_bps: record.spread_bps,
        change_24h_pct: record.change_24h_pct,
        high_24h: record.high_24h,
        low_24h: record.low_24h,
        volume_24h: record.volume_24h,
        quote_vol_24h: record.quote_vol_24h,
        ofi: record.ofi,
        ofi_raw: record.ofi_raw,
        obi: record.obi,
        trend_strength: record.trend_strength,
        cvd: record.cvd,
        taker_buy_ratio: record.taker_buy_ratio,
        pump_score: record.pump_score,
        dump_score: record.dump_score,
        pump_signal: record.pump_signal,
        dump_signal: record.dump_signal,
        whale_entry: record.whale_entry,
        whale_exit: record.whale_exit,
        bid_eating: record.bid_eating,
        total_bid_volume: record.total_bid_volume,
        total_ask_volume: record.total_ask_volume,
        max_bid_ratio: record.max_bid_ratio,
        max_ask_ratio: record.max_ask_ratio,
        anomaly_count_1m: record.anomaly_count_1m,
        anomaly_max_severity: record.anomaly_max_severity,
        status_summary: record.status_summary,
        watch_level: record.watch_level,
        signal_reason: record.signal_reason,
        sentiment: record.sentiment,
        risk_level: record.risk_level,
        recommendation: record.recommendation,
        whale_type: record.whale_type,
        pump_probability: record.pump_probability,
        price_precision: record.price_precision,
        quantity_precision: record.quantity_precision,
        snapshot: parse_json_value(&record.snapshot_json),
        signal_history: parse_json_value(&record.signal_history_json),
        factor_metrics: parse_json_value(&record.factor_metrics_json),
        enterprise_metrics: parse_json_value(&record.enterprise_metrics_json),
        update_count: record.update_count,
    }
}

fn signal_perf_sample_json_from_record(
    record: SignalPerformanceSampleRecord,
) -> SignalPerformanceSampleJson {
    SignalPerformanceSampleJson {
        sample_id: record.sample_id.to_string(),
        symbol: record.symbol,
        signal_type: record.signal_type,
        triggered_at: record.triggered_at.to_rfc3339(),
        triggered_ts: record.triggered_at.timestamp_millis(),
        trigger_price: record.trigger_price,
        trigger_score: record.trigger_score,
        watch_level: record.watch_level,
        signal_reason: record.signal_reason,
        update_count: record.update_count,
        resolved_5m: record.resolved_5m,
        resolved_15m: record.resolved_15m,
        resolved_decay: record.resolved_decay,
        outcome_5m_return: record.outcome_5m_return,
        outcome_5m_win: record.outcome_5m_win,
        outcome_5m_at: record.outcome_5m_at.map(|value| value.to_rfc3339()),
        outcome_5m_ts: record.outcome_5m_at.map(|value| value.timestamp_millis()),
        outcome_15m_return: record.outcome_15m_return,
        outcome_15m_win: record.outcome_15m_win,
        outcome_15m_at: record.outcome_15m_at.map(|value| value.to_rfc3339()),
        outcome_15m_ts: record.outcome_15m_at.map(|value| value.timestamp_millis()),
        decay_minutes: record.decay_minutes,
        decay_at: record.decay_at.map(|value| value.to_rfc3339()),
        decay_ts: record.decay_at.map(|value| value.timestamp_millis()),
        created_at: record.created_at.to_rfc3339(),
        created_ts: record.created_at.timestamp_millis(),
    }
}

fn big_trade_history_item_json_from_record(record: BigTradeHistoryRecord) -> BigTradeHistoryItemJson {
    BigTradeHistoryItemJson {
        symbol: record.symbol,
        agg_trade_id: record.agg_trade_id,
        event_time: record.event_time.to_rfc3339(),
        event_ts: record.event_time.timestamp_millis(),
        trade_time: record.trade_time.to_rfc3339(),
        trade_ts: record.trade_time.timestamp_millis(),
        price: record.price,
        quantity: record.quantity,
        quote_quantity: record.quote_quantity,
        threshold_quantity: record.threshold_quantity,
        is_taker_buy: record.is_taker_buy,
        is_buyer_maker: record.is_buyer_maker,
    }
}

fn big_trade_stats_json_from_record(record: BigTradeStatsRecord) -> BigTradeStatsJson {
    let total = record.total_count.max(0) as f64;
    let buy_ratio = if total > 0.0 {
        record.buy_count.max(0) as f64 / total * 100.0
    } else {
        0.0
    };
    let sell_ratio = if total > 0.0 {
        record.sell_count.max(0) as f64 / total * 100.0
    } else {
        0.0
    };
    BigTradeStatsJson {
        symbol: record.symbol,
        total_count: record.total_count,
        buy_count: record.buy_count,
        sell_count: record.sell_count,
        total_quote_quantity: record.total_quote_quantity,
        buy_quote_quantity: record.buy_quote_quantity,
        sell_quote_quantity: record.sell_quote_quantity,
        buy_ratio,
        sell_ratio,
        avg_quote_quantity: record.avg_quote_quantity,
        max_quote_quantity: record.max_quote_quantity,
        avg_threshold_quantity: record.avg_threshold_quantity,
        first_trade_time: record.first_trade_time.map(|value| value.to_rfc3339()),
        first_trade_ts: record.first_trade_time.map(|value| value.timestamp_millis()),
        last_trade_time: record.last_trade_time.map(|value| value.to_rfc3339()),
        last_trade_ts: record.last_trade_time.map(|value| value.timestamp_millis()),
    }
}

fn parse_query_millis(value: Option<i64>) -> Result<Option<DateTime<Utc>>, StatusCode> {
    value
        .map(|ts| DateTime::<Utc>::from_timestamp_millis(ts).ok_or(StatusCode::BAD_REQUEST))
        .transpose()
}

fn parse_json_value(raw: &str) -> Value {
    serde_json::from_str(raw).unwrap_or(Value::Null)
}

fn compact_trader_for_ws(mut trader: TraderStateJson, authenticated: bool) -> TraderStateJson {
    if !authenticated {
        return TraderStateJson::default();
    }
    if trader.open_orders.len() > WS_TRADER_OPEN_ORDERS_LIMIT {
        trader.open_orders.truncate(WS_TRADER_OPEN_ORDERS_LIMIT);
    }
    if trader.order_history.len() > WS_TRADER_HISTORY_LIMIT {
        trader.order_history.truncate(WS_TRADER_HISTORY_LIMIT);
    }
    if trader.trade_history.len() > WS_TRADER_HISTORY_LIMIT {
        trader.trade_history.truncate(WS_TRADER_HISTORY_LIMIT);
    }
    trader
}

fn trim_tail<T>(items: &mut Vec<T>, limit: usize) {
    if items.len() > limit {
        items.drain(0..items.len() - limit);
    }
}

fn strip_symbol_for_api_state(mut symbol: SymbolJson) -> SymbolJson {
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
        symbol.recent_trades.truncate(API_SYMBOL_RECENT_TRADES_LIMIT);
    }
    for bars in symbol.klines.values_mut() {
        trim_tail(bars, API_SYMBOL_KLINES_LIMIT);
    }
    symbol
}

fn strip_symbol_for_detail_stream(mut symbol: SymbolJson) -> SymbolJson {
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

impl From<&crate::web::state::FeedEntry> for CompactFeedRow {
    fn from(entry: &crate::web::state::FeedEntry) -> Self {
        Self(
            entry.time.clone(),
            entry.symbol.clone(),
            entry.r#type.clone(),
            entry.score,
            entry.desc.clone(),
            entry.ts,
        )
    }
}

impl From<&SymbolJson> for CompactSymbolRow {
    fn from(symbol: &SymbolJson) -> Self {
        Self(
            symbol.symbol.clone(),
            symbol.status_summary.clone(),
            symbol.watch_level.clone(),
            symbol.signal_reason.clone(),
            symbol.bid,
            symbol.ask,
            symbol.mid,
            symbol.spread_bps,
            symbol.price_precision,
            symbol.quantity_precision,
            symbol.change_24h_pct,
            symbol.high_24h,
            symbol.low_24h,
            symbol.volume_24h,
            symbol.quote_vol_24h,
            symbol.ofi,
            symbol.ofi_raw,
            symbol.obi,
            symbol.cvd,
            symbol.taker_buy_ratio,
            symbol.pump_score,
            symbol.dump_score,
            symbol.pump_signal,
            symbol.dump_signal,
            symbol.whale_entry,
            symbol.whale_exit,
            symbol.bid_eating,
            symbol.total_bid_volume,
            symbol.total_ask_volume,
            symbol.max_bid_ratio,
            symbol.max_ask_ratio,
            symbol.anomaly_count_1m,
            symbol.anomaly_max_severity,
            symbol.update_count,
        )
    }
}

fn json_response<T>(status: StatusCode, ok: bool, message: &str, data: T) -> Response
where
    T: serde::Serialize,
{
    (
        status,
        Json(ApiResponse {
            ok,
            message: message.to_string(),
            data,
        }),
    )
        .into_response()
}

fn json_with_cookie<T>(
    status: StatusCode,
    cookie: String,
    ok: bool,
    message: &str,
    data: T,
) -> Response
where
    T: serde::Serialize,
{
    let mut response = json_response(status, ok, message, data);
    if let Ok(value) = HeaderValue::from_str(&cookie) {
        response.headers_mut().insert(header::SET_COOKIE, value);
    }
    response
}
