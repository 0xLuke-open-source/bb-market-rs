// src/web/server.rs — 交易员版 Dashboard
//
// 设计原则：信号优先，数字为辅
// ─ 强信号触发时：全屏弹出声音 + 大字提醒
// ─ 数据平滑：3秒滚动均值，不闪烁
// ─ 布局：左侧信号墙(最重要) + 右侧币种状态 + 底部详情

use axum::{
    Router,
    extract::{Path, Query, State, WebSocketUpgrade},
    extract::ws::{Message, WebSocket},
    http::StatusCode,
    response::{Html, IntoResponse, Json},
    routing::{delete, get, post},
};
use tokio::time::{interval, Duration};
use std::path::PathBuf;
use tower_http::cors::CorsLayer;
use tower_http::services::ServeDir;
use crate::web::spot::{
    ApiOrderRequest, ApiResponse, CancelAllRequest, CancelAllResult, OrderActionResult, ReplayQuery,
    ReplayResponse, SpotTradingService,
};
use crate::web::state::SharedDashboardState;

#[derive(Clone)]
struct AppState {
    dashboard: SharedDashboardState,
    spot: SpotTradingService,
}

pub async fn run_server(dashboard: SharedDashboardState, spot: SpotTradingService, port: u16) -> anyhow::Result<()> {
    let state = AppState { dashboard, spot };
    let dashboard_root = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("src/web/dashboard");
    let app = Router::new()
        .route("/", get(serve_dashboard))
        .route("/favicon.ico", get(serve_favicon))
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
    Html(include_str!(concat!(env!("OUT_DIR"), "/dashboard_index.html")))
}

async fn serve_favicon() -> StatusCode { StatusCode::NO_CONTENT }

async fn api_full_state(State(state): State<AppState>) -> impl IntoResponse {
    let trader = state.spot.snapshot().await;
    let dashboard = state.dashboard.read().await;
    Json(dashboard.to_light_snapshot(trader))
}

async fn api_symbol_state(
    Path(symbol): Path<String>,
    State(state): State<AppState>,
) -> impl IntoResponse {
    let dashboard = state.dashboard.read().await;
    Json(dashboard.get_symbol(&symbol))
}

async fn api_spot_state(State(state): State<AppState>) -> impl IntoResponse {
    Json(state.spot.snapshot().await)
}

async fn api_spot_replay(
    State(state): State<AppState>,
    Query(query): Query<ReplayQuery>,
) -> impl IntoResponse {
    match state.spot.replay(query).await {
        Ok(data) => Json(ApiResponse::<ReplayResponse> {
            ok: true,
            message: "ok".into(),
            data,
        }),
        Err(err) => Json(ApiResponse::<ReplayResponse> {
            ok: false,
            message: err.to_string(),
            data: ReplayResponse::default(),
        }),
    }
}

async fn api_submit_order(
    State(state): State<AppState>,
    Json(req): Json<ApiOrderRequest>,
) -> impl IntoResponse {
    match state.spot.submit_order(req).await {
        Ok(data) => Json(ApiResponse::<OrderActionResult> {
            ok: true,
            message: "ok".into(),
            data,
        }),
        Err(err) => Json(ApiResponse::<OrderActionResult> {
            ok: false,
            message: err.to_string(),
            data: OrderActionResult::default(),
        }),
    }
}

async fn api_cancel_order(
    Path(order_id): Path<u64>,
    State(state): State<AppState>,
) -> impl IntoResponse {
    match state.spot.cancel_order(order_id).await {
        Ok(data) => Json(ApiResponse::<OrderActionResult> {
            ok: true,
            message: "ok".into(),
            data,
        }),
        Err(err) => Json(ApiResponse::<OrderActionResult> {
            ok: false,
            message: err.to_string(),
            data: OrderActionResult::default(),
        }),
    }
}

async fn api_cancel_all(
    State(state): State<AppState>,
    Json(req): Json<CancelAllRequest>,
) -> impl IntoResponse {
    match state.spot.cancel_all(req).await {
        Ok(data) => Json(ApiResponse::<CancelAllResult> {
            ok: true,
            message: "ok".into(),
            data,
        }),
        Err(err) => Json(ApiResponse::<CancelAllResult> {
            ok: false,
            message: err.to_string(),
            data: CancelAllResult::default(),
        }),
    }
}

async fn ws_handler(ws: WebSocketUpgrade, State(state): State<AppState>) -> impl IntoResponse {
    ws.on_upgrade(move |socket| ws_loop(socket, state))
}

async fn ws_loop(mut socket: WebSocket, state: AppState) {
    // 推送频率：2秒一次（给交易员看，不需要太快）
    let mut tick = interval(Duration::from_millis(2000));
    loop {
        tick.tick().await;
        let json = {
            let trader = state.spot.snapshot().await;
            let dashboard = state.dashboard.read().await;
            match serde_json::to_string(&dashboard.to_light_snapshot(trader)) {
                Ok(j) => j,
                Err(_) => continue,
            }
        };
        if socket.send(Message::Text(json.into())).await.is_err() { break; }
    }
}
