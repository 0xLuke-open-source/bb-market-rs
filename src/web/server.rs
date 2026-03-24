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
    AccessInfoJson, FullSnapshot, SharedDashboardState, SymbolJson, TraderStateJson,
};
use axum::{
    extract::ws::{Message, WebSocket},
    extract::{Path, Query, State, WebSocketUpgrade},
    http::{header, HeaderMap, HeaderValue, StatusCode},
    response::{Html, IntoResponse, Json, Response},
    routing::{delete, get, post},
    Router,
};
use std::path::PathBuf;
use tokio::time::{interval, Duration};
use tower_http::cors::CorsLayer;
use tower_http::services::ServeDir;

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
        .route("/api/spot/state", get(api_spot_state))
        .route("/api/spot/replay", get(api_spot_replay))
        .route("/api/spot/order", post(api_submit_order))
        .route("/api/spot/order/:order_id", delete(api_cancel_order))
        .route("/api/spot/cancel_all", post(api_cancel_all))
        .route("/ws", get(ws_handler))
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
    Ok(Json(dashboard.get_symbol(&symbol)))
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

async fn ws_loop(mut socket: WebSocket, state: AppState, access: AuthStatusResponse) {
    // 推送频率：2秒一次（给交易员看，不需要太快）
    let mut tick = interval(Duration::from_millis(2000));
    loop {
        tick.tick().await;
        let json = {
            // 每次推送都重新抓取一份轻量快照，避免前端维护复杂增量状态。
            let trader = state.spot.snapshot().await;
            let dashboard = state.dashboard.read().await;
            let snapshot = filter_snapshot(dashboard.to_light_snapshot(trader), &access);
            match serde_json::to_string(&snapshot) {
                Ok(j) => j,
                Err(_) => continue,
            }
        };
        if socket.send(Message::Text(json.into())).await.is_err() {
            break;
        }
    }
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
            snapshot.symbols.truncate(limit);
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
    dashboard: &crate::web::state::DashboardState,
    symbol: &str,
    access: &AuthStatusResponse,
) -> bool {
    if access.full_access {
        return true;
    }

    let limit = access.symbol_limit.unwrap_or(0);
    dashboard
        .sorted_keys
        .iter()
        .take(limit)
        .any(|item| item.eq_ignore_ascii_case(symbol))
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
