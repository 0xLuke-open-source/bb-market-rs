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

use crate::web::auth::{AuthRequest, AuthService, AuthStatusResponse, SubscribeRequest};
use crate::web::spot::{
    ApiOrderRequest, ApiResponse, CancelAllRequest, CancelAllResult, OrderActionResult,
    ReplayQuery, ReplayResponse, SpotTradingService,
};
use crate::web::state::{
    AccessInfoJson, DashboardState, FullSnapshot, SharedDashboardState, SymbolJson,
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
use serde::Serialize;
use std::collections::{HashMap, HashSet};
use std::path::PathBuf;
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
const PUBLIC_DEFAULT_SYMBOL: &str = "BTCUSDT";

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
    // dashboard 是“展示域状态”。
    dashboard: SharedDashboardState,
    // spot 是“交易域服务”。
    spot: SpotTradingService,
    // auth 是“访问控制服务”。
    auth: AuthService,
}

pub async fn run_server(
    dashboard: SharedDashboardState,
    spot: SpotTradingService,
    auth: AuthService,
    port: u16,
) -> anyhow::Result<()> {
    // 这里把所有前端需要的接口集中挂到一个 Router 上。
    // Dashboard HTML 是静态内容，实时数据则通过 API / WebSocket 提供。
    let state = AppState {
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
        .route("/api/state", get(api_full_state))
        .route("/api/symbol/:symbol", get(api_symbol_state))
        .route("/api/symbols", get(api_symbol_list))
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
    Json(state.auth.plans())
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
    Ok(Json(filter_snapshot(snapshot, &access)))
}

async fn api_symbol_state(
    headers: HeaderMap,
    Path(symbol): Path<String>,
    State(state): State<AppState>,
) -> Result<Json<Option<SymbolJson>>, StatusCode> {
    let access = state.auth.status_from_headers(&headers).await;
    let dashboard = state.dashboard.read().await;
    if !can_view_symbol(&dashboard, &symbol, &access) {
        return Ok(Json(None));
    }
    Ok(Json(
        dashboard
            .get_symbol(&symbol)
            .map(strip_symbol_for_api_state),
    ))
}

async fn api_symbol_list(State(state): State<AppState>) -> Result<Json<Vec<String>>, StatusCode> {
    let dashboard = state.dashboard.read().await;
    Ok(Json(dashboard.sorted_keys.clone()))
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
    {
        let dashboard = state.dashboard.read().await;
        if !can_view_symbol(&dashboard, &symbol, &access) {
            return StatusCode::FORBIDDEN.into_response();
        }
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
            build_compact_ws_snapshot(&dashboard, trader, &access)
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
            if !can_view_symbol(&dashboard, &symbol, &access) {
                break;
            }
            let Some(detail) = dashboard
                .get_symbol(&symbol)
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

async fn require_auth(state: &AppState, headers: &HeaderMap) -> Result<(), StatusCode> {
    if state.auth.status_from_headers(headers).await.authenticated {
        Ok(())
    } else {
        Err(StatusCode::UNAUTHORIZED)
    }
}

fn filter_snapshot(mut snapshot: FullSnapshot, access: &AuthStatusResponse) -> FullSnapshot {
    let total_symbols = snapshot.symbols.len();
    if !access.authenticated {
        snapshot.trader = TraderStateJson::default();
    }
    if !access.full_access {
        if let Some(limit) = access.symbol_limit {
            let visible_keys =
                limited_visible_symbols(snapshot.symbols.iter().map(|symbol| symbol.symbol.as_str()), limit);
            let mut visible_map = std::collections::HashMap::with_capacity(snapshot.symbols.len());
            for symbol in snapshot.symbols.drain(..) {
                visible_map.insert(symbol.symbol.clone(), symbol);
            }
            snapshot.symbols = visible_keys
                .iter()
                .filter_map(|symbol| visible_map.remove(symbol))
                .collect();
        }
        let visible: std::collections::HashSet<&str> = snapshot
            .symbols
            .iter()
            .map(|symbol| symbol.symbol.as_str())
            .collect();
        snapshot
            .feed
            .retain(|entry| visible.contains(entry.symbol.as_str()));
    }
    snapshot.access = build_access_info(access, snapshot.symbols.len(), total_symbols);
    snapshot
}

fn build_access_info(
    access: &AuthStatusResponse,
    visible_symbols: usize,
    total_symbols: usize,
) -> AccessInfoJson {
    let message = if access.full_access {
        format!("已解锁全部 {} 个币种。", total_symbols)
    } else if access.authenticated {
        format!(
            "当前仅展示 {} / {} 个币种，订阅后解锁全部。",
            visible_symbols, total_symbols
        )
    } else {
        format!(
            "未登录状态下仅展示 {} / {} 个币种，登录并订阅后可查看全部。",
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

fn can_view_symbol(
    dashboard: &DashboardState,
    symbol: &str,
    access: &AuthStatusResponse,
) -> bool {
    if access.full_access {
        return true;
    }

    let limit = access.symbol_limit.unwrap_or(0);
    limited_visible_symbols(dashboard.sorted_keys.iter().map(String::as_str), limit)
        .iter()
        .any(|item| item.eq_ignore_ascii_case(symbol))
}

fn build_compact_ws_snapshot(
    dashboard: &DashboardState,
    trader: TraderStateJson,
    access: &AuthStatusResponse,
) -> CompactWsSnapshot {
    let total_symbols = dashboard.sorted_keys.len();
    let visible_keys: Vec<String> = if access.full_access {
        dashboard.sorted_keys.clone()
    } else {
        limited_visible_symbols(
            dashboard.sorted_keys.iter().map(String::as_str),
            access.symbol_limit.unwrap_or(0),
        )
    };
    let visible_set = if access.full_access {
        None
    } else {
        Some(
            visible_keys
                .iter()
                .map(String::as_str)
                .collect::<std::collections::HashSet<&str>>(),
        )
    };
    let symbols = visible_keys
        .iter()
        .filter_map(|key| dashboard.symbols.get(key))
        .map(CompactSymbolRow::from)
        .collect();
    let feed = dashboard
        .feed
        .iter()
        .filter(|entry| {
            visible_set
                .as_ref()
                .map(|set| set.contains(entry.symbol.as_str()))
                .unwrap_or(true)
        })
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

fn limited_visible_symbols<'a>(
    sorted_keys: impl IntoIterator<Item = &'a str>,
    limit: usize,
) -> Vec<String> {
    if limit == 0 {
        return Vec::new();
    }

    let mut visible = Vec::with_capacity(limit);
    let mut seen = std::collections::HashSet::with_capacity(limit);
    let mut keys: Vec<&str> = sorted_keys.into_iter().collect();

    if let Some(default_symbol) = keys
        .iter()
        .copied()
        .find(|symbol| symbol.eq_ignore_ascii_case(PUBLIC_DEFAULT_SYMBOL))
    {
        visible.push(default_symbol.to_string());
        seen.insert(default_symbol.to_ascii_uppercase());
    }

    for symbol in keys.drain(..) {
        if visible.len() >= limit {
            break;
        }
        let normalized = symbol.to_ascii_uppercase();
        if !seen.insert(normalized) {
            continue;
        }
        visible.push(symbol.to_string());
    }

    visible
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
