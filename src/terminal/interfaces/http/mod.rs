// src/terminal/interfaces/http/mod.rs — 交易员版 Dashboard
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

use crate::execution::application::spot::{
    ApiOrderRequest, ApiResponse, CancelAllRequest, CancelAllResult, OrderActionResult,
    ReplayQuery, ReplayResponse, SpotTradingService,
};
use crate::identity::application::AuthStatusView;
use crate::identity::interfaces::{AuthRequest, AuthService, AuthStatusResponse, SubscribeRequest};
use crate::instrument_catalog::application::SymbolRegistryService;
use crate::market_data::application::runtime::MultiSymbolMonitor;
use crate::terminal::application::http_access::{
    can_view_symbol, filter_snapshot, parse_query_millis, require_auth, visible_keys_for_access,
};
use crate::terminal::application::http_dto::{
    big_trade_history_item_json_from_record, big_trade_stats_json_from_record,
    panel_snapshot_json_from_record, signal_perf_sample_json_from_record, BigTradeHistoryItemJson,
    BigTradeStatsJson, PanelSnapshotJson, SalesProofOverviewJson, SignalPerformanceSampleJson,
    sales_proof_overview_json_from_record,
};
use crate::terminal::application::projection::{
    BigTradeJson, FullSnapshot, SharedDashboardState, SymbolJson, TraderStateJson,
};
use crate::terminal::application::query::TerminalQueryService;
use crate::terminal::application::symbol_view::{
    load_symbol_detail, strip_symbol_for_api_state, strip_symbol_for_detail_stream,
};
use crate::terminal::application::ws::{
    build_compact_ws_snapshot, compact_feed_row_key, encode_ws_binary, CompactFeedRow,
    CompactSymbolRow, CompactWsDelta,
};
use axum::{
    extract::ws::{Message, WebSocket},
    extract::{Path, Query, State, WebSocketUpgrade},
    http::{header, HeaderMap, HeaderValue, StatusCode},
    response::{Html, IntoResponse, Json, Response},
    routing::{delete, get, post},
    Router,
};
use serde::Deserialize;
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

#[derive(Clone)]
struct AppState {
    monitor: Arc<MultiSymbolMonitor>,
    symbol_registry: SymbolRegistryService,
    queries: TerminalQueryService,
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
    queries: TerminalQueryService,
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
        queries,
        dashboard,
        spot,
        auth,
    };
    let dashboard_root =
        PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("src/terminal/interfaces/assets/dashboard");
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
        .route("/api/proof/overview", get(api_sales_proof_overview))
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

async fn api_auth_favorites(
    headers: HeaderMap,
    State(state): State<AppState>,
) -> impl IntoResponse {
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
    Ok(Json(
        filter_snapshot(&state.symbol_registry, &dashboard, snapshot, &access).await,
    ))
}

async fn api_symbol_state(
    headers: HeaderMap,
    Path(symbol): Path<String>,
    State(state): State<AppState>,
) -> Result<Json<Option<SymbolJson>>, StatusCode> {
    let access = state.auth.status_from_headers(&headers).await;
    let dashboard = state.dashboard.read().await;
    if !can_view_symbol(&state.symbol_registry, &symbol, &access).await {
        return Ok(Json(None));
    }
    drop(dashboard);

    Ok(Json(
        load_symbol_detail(
            &state.monitor,
            &state.symbol_registry,
            &state.queries,
            &state.dashboard,
            &symbol,
        )
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
    Ok(Json(
        visible_keys_for_access(&state.symbol_registry, &dashboard, &access).await,
    ))
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

#[derive(Debug, Deserialize, Default)]
struct SalesProofQuery {
    days: Option<i64>,
    top: Option<usize>,
}

async fn api_recent_trades(
    headers: HeaderMap,
    Path(symbol): Path<String>,
    Query(query): Query<TradesQuery>,
    State(state): State<AppState>,
) -> Result<Json<Vec<BigTradeJson>>, StatusCode> {
    let access = state.auth.status_from_headers(&headers).await;
    let normalized_symbol = symbol.trim().to_ascii_uppercase();
    if !can_view_symbol(&state.symbol_registry, &normalized_symbol, &access).await {
        return Err(StatusCode::FORBIDDEN);
    }

    let limit = query.limit.unwrap_or(120).clamp(1, 500);
    let trades = state
        .queries
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
    if !can_view_symbol(&state.symbol_registry, &normalized_symbol, &access).await {
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
        .queries
        .load_big_trade_history(&normalized_symbol, limit, from, to)
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
    if !can_view_symbol(&state.symbol_registry, &normalized_symbol, &access).await {
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
        .queries
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
    if !can_view_symbol(&state.symbol_registry, &normalized_symbol, &access).await {
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
        .queries
        .load_panel_history(&normalized_symbol, limit, from, to)
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
    if !can_view_symbol(&state.symbol_registry, &normalized_symbol, &access).await {
        return Err(StatusCode::FORBIDDEN);
    }

    let limit = query.limit.unwrap_or(120).clamp(1, 500);
    let samples = state
        .queries
        .load_signal_perf_history(&normalized_symbol, limit)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    Ok(Json(
        samples
            .into_iter()
            .map(signal_perf_sample_json_from_record)
            .collect(),
    ))
}

async fn api_sales_proof_overview(
    Query(query): Query<SalesProofQuery>,
    State(state): State<AppState>,
) -> Result<Json<SalesProofOverviewJson>, StatusCode> {
    let window_days = query.days.unwrap_or(30).clamp(1, 180);
    let top_symbols_limit = query.top.unwrap_or(8).clamp(1, 20);
    let overview = state
        .queries
        .load_sales_proof_overview(window_days, top_symbols_limit)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    Ok(Json(sales_proof_overview_json_from_record(overview)))
}

async fn api_spot_state(
    headers: HeaderMap,
    State(state): State<AppState>,
) -> Result<Json<TraderStateJson>, StatusCode> {
    require_auth(&state.auth, &headers).await?;
    Ok(Json(state.spot.snapshot().await))
}

async fn api_spot_replay(
    headers: HeaderMap,
    State(state): State<AppState>,
    Query(query): Query<ReplayQuery>,
) -> impl IntoResponse {
    if require_auth(&state.auth, &headers).await.is_err() {
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
    if require_auth(&state.auth, &headers).await.is_err() {
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
    if require_auth(&state.auth, &headers).await.is_err() {
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
    if require_auth(&state.auth, &headers).await.is_err() {
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
    if !can_view_symbol(&state.symbol_registry, &symbol, &access).await {
        return StatusCode::FORBIDDEN.into_response();
    }
    ws.on_upgrade(move |socket| ws_symbol_loop(socket, state, access, symbol))
}

async fn ws_loop(mut socket: WebSocket, state: AppState, access: AuthStatusView) {
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
            build_compact_ws_snapshot(&state.symbol_registry, &dashboard, trader, &access).await
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

        let force_full = last_symbols.is_empty()
            || last_full_sync.elapsed().as_secs() >= WS_FULL_SYNC_INTERVAL_SECS;
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
    access: AuthStatusView,
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
            if !can_view_symbol(&state.symbol_registry, &symbol, &access).await {
                break;
            }
            drop(dashboard);

            let Some(detail) = load_symbol_detail(
                &state.monitor,
                &state.symbol_registry,
                &state.queries,
                &state.dashboard,
                &symbol,
            )
            .await
            .map(strip_symbol_for_detail_stream) else {
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
            if socket
                .send(Message::Binary(payload.clone().into()))
                .await
                .is_err()
            {
                break;
            }
            last_payload = payload;
            last_full_sync = std::time::Instant::now();
        }
        tick.tick().await;
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
