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
use tower_http::cors::CorsLayer;
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
        .layer(CorsLayer::permissive())
        .with_state(state);

    let addr = format!("0.0.0.0:{}", port);
    println!("🌐 交易员 Dashboard: http://127.0.0.1:{}", port);
    let listener = tokio::net::TcpListener::bind(&addr).await?;
    axum::serve(listener, app).await?;
    Ok(())
}

async fn serve_dashboard() -> Html<&'static str> { Html(HTML) }

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
const HTML: &str = r#"<!DOCTYPE html>
<html lang="zh-CN">
<head>
<meta charset="UTF-8">
<meta name="viewport" content="width=device-width,initial-scale=1.0">
<title>BB-Market</title>
<style>
*{box-sizing:border-box;margin:0;padding:0}
:root{
  --bg0:#0b0e11;--bg1:#161a1e;--bg2:#1e2329;--bg3:#2b3139;
  --bd:#242930;--bd2:#3d4451;
  --t1:#eaecef;--t2:#848e9c;--t3:#5e6673;
  --g:#0ecb81;--r:#f6465d;--y:#f0b90b;--b:#1890ff;--p:#c084fc;
  --g-dim:rgba(14,203,129,.08);--r-dim:rgba(246,70,93,.08);
  --g-glow:rgba(14,203,129,.18);--r-glow:rgba(246,70,93,.18);
  --y-glow:rgba(240,185,11,.15);--b-glow:rgba(24,144,255,.12);
}
html,body{height:100%;background:var(--bg0);color:var(--t1);
  font-family:'SF Pro Text',-apple-system,BlinkMacSystemFont,'Segoe UI',sans-serif;
  font-size:12px;overflow:hidden;line-height:1.4}
body{display:flex;flex-direction:column;height:100vh}

/* ══ 顶部导航 ══ */
#nav{height:44px;background:var(--bg1);border-bottom:1px solid var(--bd);
  display:flex;align-items:center;padding:0 14px;gap:0;flex-shrink:0;z-index:20}
.logo{font-size:15px;font-weight:900;color:var(--t1);letter-spacing:-.4px;margin-right:16px;
  display:flex;align-items:center;gap:5px}
.logo-icon{width:22px;height:22px;border-radius:6px;background:linear-gradient(135deg,#f0b90b,#d97706);
  display:flex;align-items:center;justify-content:center;font-size:11px;font-weight:900;color:#000;flex-shrink:0}
.logo em{color:var(--y);font-style:normal}
.nav-sym{display:flex;align-items:center;gap:7px;margin-right:12px}
.nav-sym-name{font-size:14px;font-weight:800;color:var(--t2)}
.nav-price{font-size:20px;font-weight:800;font-variant-numeric:tabular-nums;letter-spacing:-.5px}
.nav-chg{font-size:11px;font-weight:700;padding:2px 7px;border-radius:4px}
.nup{color:var(--g);background:var(--g-dim)}.ndn{color:var(--r);background:var(--r-dim)}
.nav-stats{display:flex;gap:0;align-items:stretch;height:100%}
.ns{display:flex;flex-direction:column;justify-content:center;gap:1px;padding:0 10px;
  border-left:1px solid var(--bd)}
.ns-l{font-size:9px;color:var(--t3);white-space:nowrap;text-transform:uppercase;letter-spacing:.3px}
.ns-v{font-size:11px;font-weight:700;font-variant-numeric:tabular-nums}
.ndiv{width:1px;height:20px;background:var(--bd);margin:0 10px;flex-shrink:0;align-self:center}
.nav-r{margin-left:auto;display:flex;align-items:center;gap:8px;font-size:11px;color:var(--t2)}
.ws-badge{display:flex;align-items:center;gap:5px;padding:3px 9px;border-radius:12px;
  background:var(--bg2);border:1px solid var(--bd)}
.wdot{width:6px;height:6px;border-radius:50%;background:var(--r);flex-shrink:0}
.wdot.live{background:var(--g);box-shadow:0 0 6px var(--g);animation:blink 2.5s infinite}
@keyframes blink{0%,100%{opacity:1;box-shadow:0 0 6px var(--g)}50%{opacity:.5;box-shadow:none}}
.stat-pill{padding:3px 8px;border-radius:10px;background:var(--bg2);border:1px solid var(--bd);
  font-size:10px;display:flex;align-items:center;gap:4px}

/* ══ K线周期行 ══ */
#ktabs{height:32px;background:var(--bg1);border-bottom:1px solid var(--bd);
  display:flex;align-items:center;padding:0 10px;gap:0;flex-shrink:0}
.kt{padding:2px 8px;font-size:11px;font-weight:600;color:var(--t3);cursor:pointer;
  border-radius:4px;margin:0 1px;transition:all .12s;white-space:nowrap}
.kt:hover{color:var(--t1);background:var(--bg2)}
.kt.act{color:var(--y);background:var(--y-glow)}
.ktd{width:1px;height:12px;background:var(--bd);margin:0 4px;align-self:center}
.ktv{display:flex;gap:12px;align-items:center;margin-left:10px;font-size:10px;color:var(--t2);
  flex-shrink:0;padding-left:10px;border-left:1px solid var(--bd)}
.co{display:flex;gap:3px;align-items:center}.co-l{color:var(--t3);font-size:9px}

/* ══ 主体：4列布局 ══ */
/* 左：币对(三区展开) | 中：图表+交易 | 中右：信号+预警 | 右：分析+订单簿 */
#app{flex:1;display:grid;
  grid-template-columns:240px 1fr 490px 310px;
  grid-template-rows:1fr;
  overflow:hidden;min-height:0}

/* ══ 通用面板样式 ══ */
.panel{background:var(--bg1);border-right:1px solid var(--bd);display:flex;flex-direction:column;overflow:hidden}
.ph{display:flex;justify-content:space-between;align-items:center;
  padding:6px 10px;border-bottom:1px solid var(--bd);flex-shrink:0;height:32px}
.ph-ttl{font-size:13px;font-weight:800;color:var(--t1)}
.ph-sub{font-size:11px;color:var(--t3)}
.ph-cnt{color:var(--y);font-weight:800;font-size:12px}
.ps{flex:1;overflow-y:auto;min-height:0}
.ps::-webkit-scrollbar{width:3px}
.ps::-webkit-scrollbar-thumb{background:var(--bd2);border-radius:2px}

/* ══ Col1：左侧三区展开 ══ */
.pair-scroll{overflow-y:auto;min-height:0}
.pair-scroll::-webkit-scrollbar{width:3px}
.pair-scroll::-webkit-scrollbar-thumb{background:var(--bd2);border-radius:2px}
/* 区域标题 */
.ls-sec-hdr{display:flex;justify-content:space-between;align-items:center;
  padding:4px 9px;background:var(--bg0);border-bottom:1px solid var(--bd);
  border-top:1px solid var(--bd);flex-shrink:0;position:sticky;top:0;z-index:2}
.ls-sec-hdr:first-child{border-top:none}
.ls-sec-ttl{font-size:11px;font-weight:700;text-transform:uppercase;letter-spacing:.4px;color:var(--t3)}
.ls-sec-cnt{font-size:12px;font-weight:800}
.ls-all .ls-sec-cnt{color:var(--t2)}
.ls-sig .ls-sec-cnt{color:var(--g)}
.ls-whale .ls-sec-cnt{color:var(--b)}
.ls-sig .ls-sec-ttl{color:var(--g)}.ls-whale .ls-sec-ttl{color:var(--b)}
.ls-empty{padding:10px 8px;text-align:center;color:var(--t3);font-size:11px}
/* 信号+鲸鱼并排区域（旧，保留兼容） */
.ls-body{display:grid;grid-template-columns:1fr 1fr;height:38%;overflow:hidden;min-height:0;border-top:1px solid var(--bd);flex-shrink:0}
.ls-col{display:flex;flex-direction:column;overflow:hidden;border-right:1px solid var(--bd)}
.ls-col:last-child{border-right:none}
.ls-col-hdr{display:flex;justify-content:space-between;align-items:center;
  padding:4px 8px;background:var(--bg0);border-bottom:1px solid var(--bd);flex-shrink:0}
.ls-col-list{flex:1;overflow-y:auto;min-height:0}
.ls-col-list::-webkit-scrollbar{width:3px}
.ls-col-list::-webkit-scrollbar-thumb{background:var(--bd2);border-radius:2px}
/* 三段式竖排区块 */
.ls-vsec{display:flex;flex-direction:column;overflow:hidden;border-top:1px solid var(--bd)}
.ls-vsec:first-of-type{border-top:none}
.ls-vsec-list{flex:1;overflow-y:auto;min-height:0}
.ls-vsec-list::-webkit-scrollbar{width:3px}
.ls-vsec-list::-webkit-scrollbar-thumb{background:var(--bd2);border-radius:2px}

/* ══ 成交记录（保留样式供成交列表使用）══ */
.tr-col{display:flex;justify-content:space-between;padding:3px 8px;
  background:var(--bg0);font-size:9px;color:var(--t3);flex-shrink:0;
  border-bottom:1px solid var(--bd)}
.tr-row{display:flex;justify-content:space-between;align-items:center;
  padding:2px 8px;font-size:10px;font-variant-numeric:tabular-nums;
  border-bottom:1px solid rgba(36,41,48,.6)}
.tr-row:hover{background:var(--bg2)}

/* ══ 订单簿（嵌入右侧面板底部）══ */
.ob-col{display:flex;justify-content:space-between;padding:3px 8px;
  background:var(--bg0);font-size:11px;color:var(--t3);flex-shrink:0;
  border-bottom:1px solid var(--bd);text-transform:uppercase;letter-spacing:.2px}
.ob-asks{display:flex;flex-direction:column-reverse;overflow:hidden;flex:1}
.ob-bids{display:flex;flex-direction:column;overflow:hidden;flex:1}
.ob-row{display:flex;justify-content:space-between;align-items:center;
  padding:2px 8px;position:relative;cursor:default}
.ob-row:hover{background:rgba(255,255,255,.025)}
.ob-bg{position:absolute;top:0;bottom:0;right:0;opacity:.12;transition:width .3s}
.bga{background:var(--r)}.bgb{background:var(--g)}
.ob-p{font-size:11px;font-weight:700;font-variant-numeric:tabular-nums;position:relative;z-index:1}
.ap{color:var(--r)}.bp{color:var(--g)}
.ob-q{font-size:10px;color:var(--t2);position:relative;z-index:1;font-variant-numeric:tabular-nums}
.ob-c{font-size:9px;color:var(--t3);position:relative;z-index:1;font-variant-numeric:tabular-nums}
.ob-mid{display:flex;align-items:center;justify-content:space-between;
  padding:5px 8px;background:var(--bg2);border-top:1px solid var(--bd);border-bottom:1px solid var(--bd);
  flex-shrink:0}
.ob-mid-p{font-size:15px;font-weight:800;font-variant-numeric:tabular-nums}
.ob-bps{font-size:10px;color:var(--t2);background:var(--bg3);padding:1px 5px;border-radius:3px}
/* 买卖比例条 — 币安风格 */
.ob-ratio{display:flex;height:22px;margin:6px 8px 2px;border-radius:4px;overflow:hidden;flex-shrink:0;background:var(--bg0)}
.or-b{background:linear-gradient(90deg,rgba(14,203,129,.85),rgba(14,203,129,.6));transition:width .8s;
  display:flex;align-items:center;justify-content:flex-start;padding-left:8px;
  font-size:11px;font-weight:700;color:#fff;white-space:nowrap;overflow:hidden;min-width:0}
.or-s{background:linear-gradient(90deg,rgba(246,70,93,.6),rgba(246,70,93,.85));flex:1;
  display:flex;align-items:center;justify-content:flex-end;padding-right:8px;
  font-size:11px;font-weight:700;color:#fff;white-space:nowrap;overflow:hidden;min-width:0}
.ob-ratio-txt{display:none}

/* ══ 右侧综合面板 ══ */
#col-right{grid-column:4;display:flex;flex-direction:column;overflow:hidden;background:var(--bg1);border-left:1px solid var(--bd)}
.cr-analysis{flex:1 1 0;overflow:hidden;display:flex;flex-direction:column;border-bottom:2px solid var(--bd)}
.cr-ob{flex:0 0 auto;height:420px;display:flex;flex-direction:column;overflow:hidden}

/* ══ Col3：图表+交易 ══ */
#col-main{grid-column:3;display:flex;flex-direction:column;overflow:hidden;background:var(--bg0)}
#tv-area{flex:1 1 0;min-height:180px;border-bottom:1px solid var(--bd);position:relative;overflow:hidden}
#tv-widget{width:100%;height:100%}
.tv-loading{width:100%;height:100%;display:flex;align-items:center;justify-content:center;
  color:var(--t3);font-size:11px;flex-direction:column;gap:8px}

/* 现货交易区 */
#trade-area{flex:0 0 auto;border-bottom:1px solid var(--bd);background:var(--bg1)}
.ta-tabs{display:flex;border-bottom:1px solid var(--bd);padding:0 8px}
.tatab{padding:7px 10px;font-size:11px;font-weight:600;color:var(--t3);cursor:pointer;
  border-bottom:2px solid transparent;white-space:nowrap;transition:color .12s}
.tatab:hover{color:var(--t2)}
.tatab.act{color:var(--y);border-bottom-color:var(--y)}
.ta-types{display:flex;gap:3px;padding:6px 8px 5px;border-bottom:1px solid var(--bd);flex-shrink:0}
.ttype{padding:3px 10px;border-radius:4px;font-size:11px;font-weight:600;color:var(--t3);
  cursor:pointer;background:transparent;transition:all .12s}
.ttype:hover{color:var(--t2)}
.ttype.act{color:var(--y);background:var(--y-glow);border:1px solid rgba(240,185,11,.2)}
.ta-form{display:grid;grid-template-columns:1fr 1fr;gap:8px;padding:8px}
.ta-side{display:flex;flex-direction:column;gap:5px}
.ta-label{font-size:9px;color:var(--t3);margin-bottom:1px;text-transform:uppercase;letter-spacing:.3px}
.ta-input-row{display:flex;align-items:center;background:var(--bg2);border:1px solid var(--bd);
  border-radius:5px;overflow:hidden;transition:border-color .15s}
.ta-input-row:focus-within{border-color:rgba(240,185,11,.5);box-shadow:0 0 0 2px var(--y-glow)}
.ta-input-row input{flex:1;background:transparent;border:none;color:var(--t1);padding:6px 8px;
  font-size:12px;outline:none;font-variant-numeric:tabular-nums;width:0}
.ta-input-row span{padding:0 8px;font-size:10px;color:var(--t3);white-space:nowrap;border-left:1px solid var(--bd)}
.ta-input-row .bbo-btn{padding:2px 7px;font-size:9px;font-weight:700;color:var(--y);cursor:pointer;
  border-left:1px solid var(--bd);transition:background .12s}
.ta-input-row .bbo-btn:hover{background:var(--y-glow)}
.ta-slider{margin:1px 0}
.ta-slider input{width:100%;accent-color:var(--y)}
.ta-pcts{display:flex;gap:6px;margin:5px 0 2px}
.ta-pct{flex:1;padding:4px 0;border-radius:4px;border:1px solid var(--bd);background:var(--bg2);color:var(--t2);font-size:10px;font-weight:700;cursor:pointer;transition:all .12s}
.ta-pct:hover,.ta-pct.act{color:var(--y);border-color:rgba(240,185,11,.35);background:var(--y-glow)}
.ta-extra{display:none;gap:6px;margin-top:6px}
.ta-extra.show{display:grid;grid-template-columns:1fr 1fr}
.ta-info{display:flex;justify-content:space-between;font-size:10px;color:var(--t2)}
.ta-avail{font-size:10px;color:var(--t2);display:flex;justify-content:space-between;padding:2px 0}
.ta-btn{width:100%;padding:9px;border-radius:5px;font-size:13px;font-weight:700;cursor:pointer;
  border:none;transition:all .15s;letter-spacing:.2px}
.ta-btn:hover{filter:brightness(1.1);transform:translateY(-1px)}
.ta-btn:active{transform:translateY(0)}
.tb-buy{background:linear-gradient(135deg,var(--g),#06a86a);color:#000}
.tb-sell{background:linear-gradient(135deg,var(--r),#c9313f);color:#fff}
.ta-fee{font-size:10px;color:var(--t3);text-align:center;margin-top:3px}
.ta-stopsl{display:flex;align-items:center;gap:4px;font-size:10px;color:var(--t2);padding:2px 0}
.ta-stopsl input[type=checkbox]{accent-color:var(--y)}

/* 委托记录区 */
#orders-area{flex:0 0 auto;display:flex;flex-direction:column;overflow:hidden}
.oa-tabs{display:flex;border-bottom:1px solid var(--bd);padding:0 8px;background:var(--bg1);flex-shrink:0}
.oatab{padding:6px 9px;font-size:11px;font-weight:600;color:var(--t3);cursor:pointer;
  border-bottom:2px solid transparent;white-space:nowrap;transition:color .12s}
.oatab:hover{color:var(--t2)}
.oatab.act{color:var(--y);border-bottom-color:var(--y)}
.oa-hdr{display:flex;align-items:center;padding:3px 8px;background:var(--bg1);
  border-bottom:1px solid var(--bd);font-size:9px;color:var(--t3);flex-shrink:0;
  text-transform:uppercase;letter-spacing:.2px}
.oa-col{padding:2px 4px;white-space:nowrap}
.oa-list{height:100px;overflow-y:auto;background:var(--bg0)}
.oa-list::-webkit-scrollbar{width:3px}
.oa-list::-webkit-scrollbar-thumb{background:var(--bd2);border-radius:2px}
.oa-row{display:flex;align-items:center;padding:3px 8px;font-size:10px;
  border-bottom:1px solid rgba(36,41,48,.6)}
.oa-row:hover{background:var(--bg2)}
.oa-empty{padding:16px;text-align:center;color:var(--t3);font-size:11px}
.oa-cancel-all{margin-left:auto;padding:2px 10px;font-size:9px;font-weight:700;
  color:var(--r);background:transparent;border:1px solid rgba(246,70,93,.4);border-radius:4px;cursor:pointer;
  transition:all .12s}
.oa-cancel-all:hover{background:var(--r-dim);border-color:var(--r)}

/* ══ Col4（右侧综合）→ 分析部分 ══ */
#col-analysis{display:flex;flex-direction:column;overflow:hidden;flex:1 1 0}
.ca-scroll{flex:1;overflow-y:auto}
.ca-scroll::-webkit-scrollbar{width:3px}
.ca-scroll::-webkit-scrollbar-thumb{background:var(--bd2);border-radius:2px}
.ca-price{padding:10px 10px 8px;border-bottom:1px solid var(--bd)}
.cap-r1{display:flex;align-items:baseline;gap:6px;margin-bottom:6px;flex-wrap:wrap}
.cap-sym{font-size:12px;font-weight:800;color:var(--t2)}
.cap-p{font-size:22px;font-weight:800;font-variant-numeric:tabular-nums;letter-spacing:-.5px}
.cap-c{font-size:11px;font-weight:700;padding:2px 7px;border-radius:4px}
.cap-btns{display:flex;gap:5px;margin-top:7px}
.cbtn{padding:4px 10px;border-radius:5px;font-size:11px;font-weight:600;cursor:pointer;
  border:1px solid var(--bd);display:flex;align-items:center;gap:3px;white-space:nowrap;transition:all .12s}
.ccp{background:var(--bg2);color:var(--t2)}
.ccp:hover{color:var(--t1);border-color:var(--bd2)}
.cbn{background:var(--b-glow);color:var(--b);border-color:rgba(24,144,255,.3)}
.cbn:hover{background:var(--b);color:#fff;border-color:var(--b)}
.cap-stats{display:grid;grid-template-columns:1fr 1fr;gap:1px;margin-top:6px;
  background:var(--bd);border-radius:6px;overflow:hidden}
.cst{padding:4px 8px;font-size:11px;background:var(--bg1)}
.cst:hover{background:var(--bg2)}
.cst-l{color:var(--t3);font-size:10px;display:block;margin-bottom:1px;text-transform:uppercase;letter-spacing:.2px}
.cst-v{font-weight:700;font-variant-numeric:tabular-nums;font-size:11px}
.ca-cvd{padding:8px 10px;border-bottom:1px solid var(--bd)}
.ca-summary{padding:10px;border-bottom:1px solid var(--bd);background:linear-gradient(180deg,rgba(240,185,11,.06),rgba(240,185,11,.01))}
.cas-h{display:flex;align-items:center;gap:8px;margin-bottom:6px}
.cas-lvl{padding:2px 8px;border-radius:999px;font-size:10px;font-weight:800;background:var(--bg2);border:1px solid var(--bd);color:var(--y)}
.cas-title{font-size:11px;color:var(--t3);text-transform:uppercase;letter-spacing:.3px}
.cas-main{font-size:13px;font-weight:700;color:var(--t1);line-height:1.55}
.cas-reason{font-size:11px;color:var(--t2);line-height:1.55;margin-top:6px}
.cvd-hdr{display:flex;justify-content:space-between;align-items:center;margin-bottom:5px}
.cvd-ttl{font-size:12px;font-weight:700;color:var(--t3);text-transform:uppercase;letter-spacing:.3px}
.cvd-v{font-size:14px;font-weight:800;font-variant-numeric:tabular-nums}
#cvd-c{width:100%;height:48px;border-radius:4px}
.ca-fac{padding:7px 10px;border-bottom:1px solid var(--bd)}
.caf-ttl{font-size:11px;font-weight:700;color:var(--t3);margin-bottom:6px;text-transform:uppercase;letter-spacing:.4px}
.fi{display:grid;grid-template-columns:70px 1fr 56px;align-items:center;gap:4px;
  padding:4px 0;border-bottom:1px solid rgba(36,41,48,.5)}
.fi:last-child{border:none}
.fi-n{font-size:11px;color:var(--t2)}
.fi-bar{height:4px;background:var(--bg0);border-radius:2px;overflow:hidden}
.fi-f{height:100%;border-radius:2px;transition:width .5s ease-out}
.gf{background:linear-gradient(90deg,var(--g),rgba(14,203,129,.6))}
.rf2{background:linear-gradient(90deg,var(--r),rgba(246,70,93,.6))}
.yf{background:linear-gradient(90deg,var(--y),rgba(240,185,11,.6))}
.bf{background:linear-gradient(90deg,var(--b),rgba(24,144,255,.6))}
.fi-v{font-size:11px;font-weight:700;text-align:right;font-variant-numeric:tabular-nums}
.fg{color:var(--g)}.fr{color:var(--r)}.fy{color:var(--y)}.fn{color:var(--t2)}.fb{color:var(--b)}
.fi-tip{font-size:9px;color:var(--t3);margin-top:1px;line-height:1.3}
.ca-bt-hdr{display:flex;justify-content:space-between;align-items:center;
  padding:6px 10px;border-bottom:1px solid var(--bd);font-size:11px;font-weight:700;color:var(--t3);
  text-transform:uppercase;letter-spacing:.3px}
.ca-bt-cnt{color:var(--y);font-weight:800;font-size:12px}
.bt-row{display:flex;align-items:center;gap:5px;padding:4px 10px;font-size:10px;
  font-variant-numeric:tabular-nums;border-bottom:1px solid rgba(36,41,48,.5)}
.bt-row:hover{background:var(--bg2)}
.btdot{width:6px;height:6px;border-radius:50%;flex-shrink:0}
.db{background:var(--g);box-shadow:0 0 4px var(--g)}.ds{background:var(--r);box-shadow:0 0 4px var(--r)}
.bt-dir{font-weight:700;width:44px}.btu{color:var(--g)}.btd{color:var(--r)}

/* ══ Col3：信号+预警 ══ */
#col-alerts{grid-column:3;background:var(--bg1);display:flex;flex-direction:column;overflow:hidden;
  border-left:1px solid var(--bd);border-right:1px solid var(--bd)}
.ra-header{display:flex;border-bottom:1px solid var(--bd);flex-shrink:0;height:33px;align-items:stretch}
.ra-sec-hdr{flex:1;display:flex;align-items:center;justify-content:space-between;
  padding:0 10px;font-size:12px;font-weight:700;color:var(--t3);border-right:1px solid var(--bd);
  text-transform:uppercase;letter-spacing:.3px}
.ra-sec-hdr:last-child{border-right:none}
.ra-cnt{color:var(--y);font-weight:800;font-size:14px}
.ra-body{flex:1;display:grid;grid-template-columns:1fr 1fr;overflow:hidden;min-height:0}
.ra-col{display:flex;flex-direction:column;overflow:hidden;border-right:1px solid var(--bd)}
.ra-col:last-child{border-right:none}
.ra-list{flex:1;overflow-y:auto}
.ra-list::-webkit-scrollbar{width:3px}
.ra-list::-webkit-scrollbar-thumb{background:var(--bd2);border-radius:2px}
.scard{padding:7px 8px 6px;border-bottom:1px solid var(--bd);cursor:pointer;
  transition:background .1s;position:relative;border-left:3px solid transparent}
.scard:hover{background:rgba(255,255,255,.02)}
.scard.pump{border-left-color:var(--g)}.scard.dump{border-left-color:var(--r)}
.scard.whale{border-left-color:var(--b)}.scard.cvd{border-left-color:var(--p)}
.scard.anomaly{border-left-color:var(--y)}
.sc-h{display:flex;justify-content:space-between;align-items:center;margin-bottom:3px}
.sc-sym{font-size:13px;font-weight:800}
.pump .sc-sym{color:var(--g)}.dump .sc-sym{color:var(--r)}
.whale .sc-sym{color:var(--b)}.cvd .sc-sym{color:var(--p)}.anomaly .sc-sym{color:var(--y)}
.sc-t{font-size:10px;color:var(--t3)}
.sc-tag{font-size:10px;font-weight:700;padding:2px 7px;border-radius:10px;display:inline-block;margin-bottom:3px}
.pump .sc-tag{background:var(--g-dim);color:var(--g);border:1px solid rgba(14,203,129,.2)}
.dump .sc-tag{background:var(--r-dim);color:var(--r);border:1px solid rgba(246,70,93,.2)}
.whale .sc-tag{background:var(--b-glow);color:var(--b);border:1px solid rgba(24,144,255,.2)}
.cvd .sc-tag{background:rgba(192,132,252,.08);color:var(--p);border:1px solid rgba(192,132,252,.2)}
.anomaly .sc-tag{background:var(--y-glow);color:var(--y);border:1px solid rgba(240,185,11,.2)}
.sc-desc{font-size:10px;color:var(--t2);line-height:1.4}
.sc-score{display:flex;align-items:center;gap:4px;margin-top:4px}
.sc-score-bar{flex:1;height:2px;border-radius:1px;background:var(--bg0);overflow:hidden}
.sc-score-fill{height:100%;border-radius:1px}
.sc-score-v{font-size:10px;font-weight:700;color:var(--t3)}
.sc-new{position:absolute;top:5px;right:6px;color:#fff;
  font-size:7px;font-weight:800;padding:1px 4px;border-radius:8px;animation:fo 4s forwards}
.pump .sc-new,.dump .sc-new{background:var(--r)}.whale .sc-new{background:var(--b)}
@keyframes fo{0%,60%{opacity:1}100%{opacity:0;pointer-events:none}}
.sc-x{position:absolute;top:5px;right:6px;font-size:10px;color:var(--t3);cursor:pointer;
  width:16px;height:16px;display:flex;align-items:center;justify-content:center;border-radius:50%;
  transition:all .1s}
.sc-x:hover{color:var(--t1);background:var(--bg3)}
.empty-p{padding:20px 8px;text-align:center;color:var(--t3);font-size:10px;line-height:2}

/* ══ 底部 ══ */
#bottom{height:28px;background:var(--bg1);border-top:1px solid var(--bd);
  display:flex;align-items:center;flex-shrink:0;overflow:hidden}
.pair-mini-list{display:flex;align-items:center;padding:0 10px;gap:14px;overflow:hidden;
  border-right:1px solid var(--bd);height:100%;flex-shrink:0;width:240px}
.pi-mini{display:flex;gap:4px;align-items:center;cursor:pointer;white-space:nowrap;flex-shrink:0;
  padding:2px 4px;border-radius:3px;transition:background .1s}
.pi-mini:hover{background:var(--bg2)}
.pm-sym{font-size:10px;color:var(--t2);font-weight:600}
.pm-p{font-size:10px;font-variant-numeric:tabular-nums;font-weight:700}
.pm-c{font-size:9px;font-weight:700;padding:0 4px;border-radius:3px}
.pmu{color:var(--g);background:var(--g-dim)}.pmd{color:var(--r);background:var(--r-dim)}
#ticker-scroll{flex:1;display:flex;align-items:center;padding:0 8px;gap:16px;overflow:hidden}
.tbi{display:flex;gap:4px;align-items:center;white-space:nowrap;cursor:pointer;flex-shrink:0;
  padding:2px 4px;border-radius:3px;transition:background .1s}
.tbi:hover{background:var(--bg2)}
.tb-s{font-size:10px;color:var(--t2);font-weight:600}
.tb-p{font-size:10px;font-variant-numeric:tabular-nums;font-weight:700}
.tb-c{font-size:9px;font-weight:700;padding:0 4px;border-radius:3px}
.tbu{color:var(--g);background:var(--g-dim)}.tbd{color:var(--r);background:var(--r-dim)}

/* ══ 左侧币对列表 ══ */
.left-top{padding:5px 7px;border-bottom:1px solid var(--bd);flex-shrink:0}
.srch-wrap{position:relative}
.srch-wrap::before{content:'⌕';position:absolute;left:8px;top:50%;transform:translateY(-50%);
  color:var(--t3);font-size:13px;pointer-events:none}
.left-top input{width:100%;background:var(--bg2);border:1px solid var(--bd);border-radius:5px;
  color:var(--t1);padding:4px 7px 4px 24px;font-size:11px;outline:none;transition:border-color .15s}
.left-top input:focus{border-color:rgba(240,185,11,.4)}
/* 币对卡片 — 和信号卡保持一致风格 */
.coin-card{padding:7px 8px 6px;border-bottom:1px solid var(--bd);cursor:pointer;
  transition:background .1s;position:relative;border-left:3px solid var(--bd2)}
.coin-card:hover,.coin-card.act{background:rgba(255,255,255,.025)}
.coin-card.act{border-left-color:var(--y)}
.coin-card.cc-pump{border-left-color:var(--g)}
.coin-card.cc-dump{border-left-color:var(--r)}
.coin-card.cc-whale{border-left-color:var(--b)}
.cc-h{display:flex;justify-content:space-between;align-items:center;margin-bottom:3px}
.cc-sym{font-size:12px;font-weight:800;color:var(--t1)}
.cc-pump .cc-sym{color:var(--g)}.cc-dump .cc-sym{color:var(--r)}.cc-whale .cc-sym{color:var(--b)}
.cc-price-wrap{display:flex;align-items:center;gap:4px}
.cc-price{font-size:11px;font-weight:700;font-variant-numeric:tabular-nums}
.cc-chg{font-size:9px;font-weight:700;padding:1px 5px;border-radius:8px}
.cc-tag{font-size:9px;font-weight:700;padding:1px 6px;border-radius:10px;
  display:inline-block;margin-bottom:3px}
.cc-stats{font-size:10px;color:var(--t2);line-height:1.5;margin-top:1px}
.cc-bars{display:flex;gap:2px;margin-top:4px}
.cup{color:var(--g)}.cdn{color:var(--r)}.cfl{color:var(--t2)}
.pib{flex:1;height:2px;background:var(--bg0);border-radius:1px;overflow:hidden}
.pbf{height:100%;border-radius:1px;transition:width .5s ease-out}
.pf-g{background:var(--g)}.pf-r{background:var(--r)}.pf-b{background:var(--b)}

/* copy tip */
.ctip{position:fixed;bottom:38px;left:50%;transform:translateX(-50%);
  background:rgba(14,203,129,.95);color:#000;font-size:11px;font-weight:700;
  padding:5px 16px;border-radius:20px;opacity:0;transition:opacity .2s;pointer-events:none;z-index:200}
.ctip.show{opacity:1}

/* 价格变化闪烁 */
@keyframes flash-g{0%{background:var(--g-dim)}100%{background:transparent}}
@keyframes flash-r{0%{background:var(--r-dim)}100%{background:transparent}}
.price-flash-up{animation:flash-g .4s ease-out}
.price-flash-dn{animation:flash-r .4s ease-out}
</style>
</head>
<body>

<!-- ══ 顶部导航 ══ -->
<div id="nav">
  <div class="logo">
    <div class="logo-icon">B</div>
    BB-<em>Market</em>
  </div>
  <div class="ndiv"></div>
  <div class="nav-sym">
    <span class="nav-sym-name" id="nav-sym">--/USDT</span>
    <span class="nav-price cup" id="nav-price">--</span>
    <span class="nav-chg nup" id="nav-chg">--</span>
  </div>
  <div class="nav-stats">
    <div class="ns"><div class="ns-l">24h 涨跌</div><div class="ns-v" id="nv-chg">--</div></div>
    <div class="ns"><div class="ns-l">24h 最高</div><div class="ns-v cup" id="nv-hi">--</div></div>
    <div class="ns"><div class="ns-l">24h 最低</div><div class="ns-v cdn" id="nv-lo">--</div></div>
    <div class="ns"><div class="ns-l">成交额 USDT</div><div class="ns-v" id="nv-vol">--</div></div>
    <div class="ns"><div class="ns-l">价差</div><div class="ns-v" id="nv-sp">--</div></div>
    <div class="ns"><div class="ns-l">主动买卖量差</div><div class="ns-v" id="nv-cvd">--</div></div>
    <div class="ns"><div class="ns-l">拉盘评分</div><div class="ns-v fy" id="nv-ps">--</div></div>
  </div>
  <div class="nav-r">
    <div class="ws-badge">
      <div class="wdot" id="wdot"></div><span id="wlbl">连接中</span>
    </div>
    <div class="ndiv"></div>
    <span id="htime" style="font-variant-numeric:tabular-nums"></span>
    <div class="ndiv"></div>
    <div class="stat-pill">监控 <b style="color:var(--y)" id="nc">--</b></div>
    <div class="stat-pill">活跃 <b style="color:var(--g)" id="ns2">--</b></div>
    <div class="stat-pill" style="cursor:pointer" onclick="openReplay()">回放</div>
  </div>
</div>

<!-- ══ K线周期行 ══ -->
<div id="ktabs">
  <div class="kt" data-iv="1">1m</div>
  <div class="kt" data-iv="3">3m</div>
  <div class="kt" data-iv="5">5m</div>
  <div class="kt" data-iv="15">15m</div>
  <div class="kt" data-iv="30">30m</div>
  <div class="ktd"></div>
  <div class="kt act" data-iv="60">1h</div>
  <div class="kt" data-iv="120">2h</div>
  <div class="kt" data-iv="240">4h</div>
  <div class="kt" data-iv="360">6h</div>
  <div class="kt" data-iv="480">8h</div>
  <div class="kt" data-iv="720">12h</div>
  <div class="ktd"></div>
  <div class="kt" data-iv="D">1d</div>
  <div class="kt" data-iv="3D">3d</div>
  <div class="kt" data-iv="W">1w</div>
  <div class="kt" data-iv="M">1M</div>
  <div class="ktd"></div>
  <div class="ktv">
    <span class="co"><span class="co-l">开</span><span id="ci-o">--</span></span>
    <span class="co"><span class="co-l">高</span><span id="ci-h" style="color:var(--g)">--</span></span>
    <span class="co"><span class="co-l">低</span><span id="ci-l2" style="color:var(--r)">--</span></span>
    <span class="co"><span class="co-l">收</span><span id="ci-c">--</span></span>
    <span class="co"><span class="co-l">量</span><span id="ci-v" style="color:var(--y)">--</span></span>
    <span class="co"><span class="co-l">买占</span><span id="ci-tbr" style="color:var(--b)">--%</span></span>
  </div>
</div>

<!-- ══ 主体 4列 ══ -->
<div id="app">

  <!-- Col1：币对列表（鲸鱼上 / 信号中 / 全部下） -->
  <div class="panel" style="grid-column:1;flex-direction:column">
    <div class="ph"><span class="ph-ttl">市场</span><span style="font-size:9px;color:var(--t3);text-transform:uppercase;letter-spacing:.3px">MARKET</span></div>
    <div class="left-top">
      <div class="srch-wrap">
        <input type="text" placeholder="搜索币对..." id="srch" oninput="filterP(this.value)">
      </div>
    </div>
    <!-- 鲸鱼区域（上，固定高度） -->
    <div class="ls-vsec" style="flex:0 0 22%">
      <div class="ls-sec-hdr ls-whale" style="border-top:none">
        <span class="ls-sec-ttl">🐋 鲸鱼</span>
        <span class="ls-sec-cnt" id="sec-whale-cnt">0</span>
      </div>
      <div class="ls-vsec-list" id="sec-whales"><div class="ls-empty">等待鲸鱼...</div></div>
    </div>
    <!-- 信号区域（中，固定高度） -->
    <div class="ls-vsec" style="flex:0 0 28%">
      <div class="ls-sec-hdr ls-sig">
        <span class="ls-sec-ttl">📡 信号</span>
        <span class="ls-sec-cnt" id="sec-sig-cnt">0</span>
      </div>
      <div class="ls-vsec-list" id="sec-sigs"><div class="ls-empty">等待信号...</div></div>
    </div>
    <!-- 全部区域（下，剩余空间） -->
    <div class="ls-vsec" style="flex:1;min-height:0">
      <div class="ls-sec-hdr ls-all">
        <span class="ls-sec-ttl">全部</span>
        <span class="ls-sec-cnt" id="sec-all-cnt">0</span>
      </div>
      <div class="ls-vsec-list" id="sec-all" style="flex:1"></div>
    </div>
  </div>

  <!-- Col2：图表 + 交易 + 委托 -->
  <div id="col-main" style="grid-column:2">
    <div id="tv-area">
      <div id="tv-widget"><div class="tv-loading"><span style="font-size:18px">📈</span><span>TradingView 加载中...</span></div></div>
    </div>

    <!-- 现货交易表单 -->
    <div id="trade-area">
      <div class="ta-tabs">
        <div class="tatab act">现货</div>
        <div class="tatab">全仓</div>
        <div class="tatab">逐仓</div>
        <div class="tatab">网格</div>
        <span style="margin-left:auto;font-size:9px;color:var(--t3);align-self:center;text-transform:uppercase;letter-spacing:.3px">手续费等级</span>
      </div>
      <div class="ta-types">
        <div class="ttype act" onclick="setType(0,this)">限价</div>
        <div class="ttype" onclick="setType(1,this)">市价</div>
        <div class="ttype" onclick="setType(2,this)">止盈止损</div>
      </div>
      <div class="ta-form">
        <!-- 买入侧 -->
        <div class="ta-side">
          <div class="ta-avail">
            <span style="color:var(--t3);font-size:9px;text-transform:uppercase;letter-spacing:.2px">可用余额</span>
            <span style="color:var(--t1)"><span id="avail-buy">3,921.63</span> <span style="color:var(--t3)">USDT</span></span>
          </div>
          <div>
            <div class="ta-label">买入价格</div>
            <div class="ta-input-row">
              <input type="number" id="buy-price" placeholder="0.00" step="any">
              <span>USDT</span>
              <span class="bbo-btn" onclick="setBBO('buy')">BBO</span>
            </div>
          </div>
          <div>
            <div class="ta-label">买入数量</div>
            <div class="ta-input-row">
              <input type="number" id="buy-qty" placeholder="0">
              <span id="buy-unit">--</span>
            </div>
          </div>
          <div class="ta-pcts">
            <button class="ta-pct" onclick="setTradePct('buy',20,this)">20%</button>
            <button class="ta-pct" onclick="setTradePct('buy',40,this)">40%</button>
            <button class="ta-pct" onclick="setTradePct('buy',50,this)">50%</button>
            <button class="ta-pct" onclick="setTradePct('buy',80,this)">80%</button>
            <button class="ta-pct" onclick="setTradePct('buy',100,this)">100%</button>
          </div>
          <div class="ta-slider"><input type="range" min="0" max="100" value="0" id="buy-pct" oninput="setBuyPct(this.value)"></div>
          <div class="ta-extra" id="buy-stop-box">
            <div>
              <div class="ta-label">触发价</div>
              <div class="ta-input-row"><input type="number" id="buy-trigger-price" placeholder="0.00" step="any"><span>USDT</span></div>
            </div>
            <div>
              <div class="ta-label">触发类型</div>
              <div class="ta-input-row">
                <select id="buy-trigger-kind" style="flex:1;background:transparent;border:none;color:var(--t1);padding:6px 8px;outline:none">
                  <option value="stop_loss">止损</option>
                  <option value="take_profit">止盈</option>
                </select>
              </div>
            </div>
          </div>
          <div class="ta-info">
            <span style="color:var(--t3)">成交额</span>
            <span style="color:var(--t2)"><span id="buy-total">0</span> USDT</span>
          </div>
          <button class="ta-btn tb-buy" onclick="doTrade('buy')" id="btn-buy">买入 --</button>
        </div>
        <!-- 卖出侧 -->
        <div class="ta-side">
          <div class="ta-avail">
            <span style="color:var(--t3);font-size:9px;text-transform:uppercase;letter-spacing:.2px">可用余额</span>
            <span style="color:var(--t1)">0 <span id="sell-unit2" style="color:var(--t3)">--</span></span>
          </div>
          <div>
            <div class="ta-label">卖出价格</div>
            <div class="ta-input-row">
              <input type="number" id="sell-price" placeholder="0.00" step="any">
              <span>USDT</span>
              <span class="bbo-btn" onclick="setBBO('sell')">BBO</span>
            </div>
          </div>
          <div>
            <div class="ta-label">卖出数量</div>
            <div class="ta-input-row">
              <input type="number" id="sell-qty" placeholder="0">
              <span id="sell-unit">--</span>
            </div>
          </div>
          <div class="ta-pcts">
            <button class="ta-pct" onclick="setTradePct('sell',20,this)">20%</button>
            <button class="ta-pct" onclick="setTradePct('sell',40,this)">40%</button>
            <button class="ta-pct" onclick="setTradePct('sell',50,this)">50%</button>
            <button class="ta-pct" onclick="setTradePct('sell',80,this)">80%</button>
            <button class="ta-pct" onclick="setTradePct('sell',100,this)">100%</button>
          </div>
          <div class="ta-slider"><input type="range" min="0" max="100" value="0" id="sell-pct"></div>
          <div class="ta-extra" id="sell-stop-box">
            <div>
              <div class="ta-label">触发价</div>
              <div class="ta-input-row"><input type="number" id="sell-trigger-price" placeholder="0.00" step="any"><span>USDT</span></div>
            </div>
            <div>
              <div class="ta-label">触发类型</div>
              <div class="ta-input-row">
                <select id="sell-trigger-kind" style="flex:1;background:transparent;border:none;color:var(--t1);padding:6px 8px;outline:none">
                  <option value="stop_loss">止损</option>
                  <option value="take_profit">止盈</option>
                </select>
              </div>
            </div>
          </div>
          <div class="ta-info">
            <span style="color:var(--t3)">成交额</span>
            <span style="color:var(--t2)"><span id="sell-total">0</span> USDT</span>
          </div>
          <button class="ta-btn tb-sell" onclick="doTrade('sell')" id="btn-sell">卖出 --</button>
        </div>
      </div>
    </div>

    <!-- 最新成交记录 -->
    <div style="flex:0 0 auto;border-top:1px solid var(--bd);display:flex;flex-direction:column">
      <div class="ph" style="background:var(--bg1)">
        <span class="ph-ttl">最新成交</span>
        <span class="ph-sub" id="tr-cnt">--</span>
      </div>
      <div class="tr-col"><span>价格 (USDT)</span><span>数量</span><span>时间</span></div>
      <div style="height:80px;overflow-y:auto" id="tr-list"></div>
    </div>

    <!-- 委托记录 -->
    <div id="orders-area">
      <div class="oa-tabs">
        <div class="oatab act" onclick="oaTab(0,this)">当前委托(0)</div>
        <div class="oatab" onclick="oaTab(1,this)">历史委托</div>
        <div class="oatab" onclick="oaTab(2,this)">历史成交</div>
        <div class="oatab" onclick="oaTab(3,this)">持仓</div>
        <div class="oatab" onclick="oaTab(4,this)">机器人</div>
        <button class="oa-cancel-all" id="cancel-all-btn" onclick="cancelAll()">全撤</button>
      </div>
      <div class="oa-hdr" id="oa-hdr">
        <span class="oa-col" style="flex:1.2">日期</span>
        <span class="oa-col" style="flex:1">交易对</span>
        <span class="oa-col" style="flex:.8">类型</span>
        <span class="oa-col" style="flex:.6">方向</span>
        <span class="oa-col" style="flex:1">价格</span>
        <span class="oa-col" style="flex:1">数量</span>
        <span class="oa-col" style="flex:1.2">冰山单</span>
        <span class="oa-col" style="flex:.8">完成度</span>
        <span class="oa-col" style="flex:1">金额</span>
        <span class="oa-col" style="flex:1.2">触发条件</span>
        <span class="oa-col" style="flex:.6">SOR</span>
        <span class="oa-col" style="flex:.8">止盈/止损</span>
      </div>
      <div class="oa-list" id="oa-list">
        <div class="oa-empty">暂无当前委托</div>
      </div>
    </div>
  </div>

  <!-- Col3：信号 + 预警 -->
  <div id="col-alerts">
    <div class="ra-header">
      <div class="ra-sec-hdr"><span>实时信号</span><span class="ra-cnt" id="sig-cnt">0</span></div>
      <div class="ra-sec-hdr"><span>预警通知</span><span class="ra-cnt" id="al-cnt">0</span></div>
    </div>
    <div class="ra-body">
      <div class="ra-col">
        <div class="ra-list" id="sig-list"><div class="empty-p">📡<br>等待信号<br><span style="color:var(--t3)">评分 ≥ 70 触发</span></div></div>
      </div>
      <div class="ra-col">
        <div class="ra-list" id="al-list"><div class="empty-p">🔔<br>等待预警<br><span style="color:var(--t3)">评分 ≥ 75 触发</span></div></div>
      </div>
    </div>
  </div>

  <!-- Col4：右侧综合面板（分析在上 + 订单簿在下） -->
  <div id="col-right">
    <!-- 分析面板（上，弹性伸缩） -->
    <div class="cr-analysis">
      <div id="col-analysis">
        <div class="ph" style="height:33px;flex-shrink:0">
          <span class="ph-ttl">分析面板</span>
          <span style="font-size:9px;color:var(--t3);text-transform:uppercase;letter-spacing:.3px">ANALYTICS</span>
        </div>
        <div class="ca-scroll">
          <div class="ca-price">
            <div class="cap-r1">
              <span class="cap-sym" id="rd-sym">--</span>
              <span class="cap-p" id="rd-p">--</span>
              <span class="cap-c" id="rd-c">--</span>
            </div>
            <div class="cap-btns">
              <button class="cbtn ccp" id="rbcp" onclick="copySym()">复制代码</button>
              <button class="cbtn cbn" onclick="openBN()">币安交易</button>
            </div>
            <div class="cap-stats">
              <div class="cst"><span class="cst-l">买一</span><span class="cst-v cup" id="rd-bid">--</span></div>
              <div class="cst"><span class="cst-l">卖一</span><span class="cst-v cdn" id="rd-ask">--</span></div>
              <div class="cst"><span class="cst-l">24h 涨跌</span><span class="cst-v" id="rd-chg">--</span></div>
              <div class="cst"><span class="cst-l">24h 成交量</span><span class="cst-v" id="rd-vol">--</span></div>
              <div class="cst"><span class="cst-l">24h 高</span><span class="cst-v cup" id="rd-hi">--</span></div>
              <div class="cst"><span class="cst-l">24h 低</span><span class="cst-v cdn" id="rd-lo">--</span></div>
              <div class="cst"><span class="cst-l">拉盘评分</span><span class="cst-v fy" id="rd-ps">--</span></div>
              <div class="cst"><span class="cst-l">砸盘评分</span><span class="cst-v cdn" id="rd-ds">--</span></div>
            </div>
          </div>
          <div class="ca-summary">
            <div class="cas-h">
              <span class="cas-lvl" id="rd-watch-level">观察</span>
              <span class="cas-title">盯盘结论</span>
            </div>
            <div class="cas-main" id="rd-summary">等待市场数据...</div>
            <div class="cas-reason" id="rd-reason">有新信号时，这里会直接告诉你为什么触发。</div>
          </div>
          <div class="ca-cvd">
            <div class="cvd-hdr">
              <span class="cvd-ttl">主动买卖量差</span>
              <span class="cvd-v" id="cvd-v">--</span>
            </div>
            <canvas id="cvd-c"></canvas>
          </div>
          <div class="ca-fac">
            <div class="caf-ttl">信号因子</div>
            <div id="rf-list"></div>
          </div>
          <div class="ca-bt-hdr">
            <span>近期大单</span>
            <span class="ca-bt-cnt" id="bt-cnt">0</span>
          </div>
          <div id="bt-list"></div>
        </div>
      </div>
    </div>
    <!-- 订单簿（下，固定高度） -->
    <div class="cr-ob">
      <div class="ph" style="height:32px;flex-shrink:0">
        <span class="ph-ttl">订单簿</span>
        <span class="ph-sub" style="font-size:9px;text-transform:uppercase;letter-spacing:.3px">DEPTH</span>
      </div>
      <div class="ob-col"><span>价格 (USDT)</span><span>数量</span><span>累计</span></div>
      <div id="ob-asks" class="ob-asks"></div>
      <div class="ob-mid">
        <span class="ob-mid-p" id="ob-mid">--</span>
        <span style="font-size:9px;color:var(--t3);text-transform:uppercase;letter-spacing:.2px">当前价</span>
        <span class="ob-bps" id="ob-bps">--</span>
      </div>
      <div id="ob-bids" class="ob-bids"></div>
      <div class="ob-ratio"><div class="or-b" id="or-b" style="width:50%">买 50%</div><div class="or-s" id="or-s">卖 50%</div></div>
      <div class="ob-ratio-txt">
        <span id="or-bt" style="color:var(--g)">买 50%</span>
        <span id="or-st" style="color:var(--r)">卖 50%</span>
      </div>
    </div>
  </div>

</div>

<!-- ══ 底部 ══ -->
<div id="bottom">
  <div class="pair-mini-list" id="pair-mini"></div>
  <div id="ticker-scroll"></div>
</div>

<div class="ctip" id="ctip"></div>

<script>
const S={syms:[],feed:[],sel:null,sm:{},cvdH:{},seen:new Set(),alerts:[],tr:{},
  trader:{account_id:0,balances:[],open_orders:[],order_history:[],trade_history:[]},
  ui:{pairAll:'',pairSig:'',pairWhale:'',pairMini:'',ticker:'',signals:'',alerts:'',detailKey:''}};
const A=0.25,HL=60;
let curIv='60',tvSym='',oaTabMode=0,tradeType=0,searchQ='';
let tvLoadingPromise=null;
const IVMAP={'1':'1m','3':'3m','5':'5m','15':'15m','30':'30m',
  '60':'1h','120':'2h','240':'4h','360':'6h','480':'8h','720':'12h',
  'D':'1d','3D':'3d','W':'1w','M':'1M'};

// ── TradingView ──────────────────────────────────────────────────
function ensureTradingView(){
  if(typeof TradingView!=='undefined')return Promise.resolve(true);
  if(tvLoadingPromise)return tvLoadingPromise;
  tvLoadingPromise=new Promise(resolve=>{
    const sc=document.createElement('script');
    sc.src='https://s3.tradingview.com/tv.js';
    sc.async=true;
    sc.onload=()=>resolve(true);
    sc.onerror=()=>resolve(false);
    document.head.appendChild(sc);
  });
  return tvLoadingPromise;
}

async function initTV(symbol,iv){
  const s='BINANCE:'+symbol;
  if(tvSym===s&&curIv===iv)return;
  tvSym=s;curIv=iv;
  const el=document.getElementById('tv-widget');
  if(typeof TradingView==='undefined'){
    el.innerHTML='<div class="tv-loading">⏳ TradingView 加载中...</div>';
    const ok=await ensureTradingView();
    if(!ok){
      el.innerHTML='<div class="tv-loading">K 线组件加载失败</div>';
      return;
    }
    if(tvSym!==s||curIv!==iv)return;
  }
  el.innerHTML='';
  new TradingView.widget({
    container_id:'tv-widget',symbol:s,interval:iv,
    timezone:'Asia/Shanghai',theme:'dark',style:'1',locale:'zh_CN',
    toolbar_bg:'#161a1e',enable_publishing:false,allow_symbol_change:false,
    hide_side_toolbar:false,hide_legend:false,save_image:false,
    studies:['RSI@tv-basicstudies','MACD@tv-basicstudies'],
    width:'100%',height:'100%',backgroundColor:'#0b0e11',gridColor:'rgba(43,49,57,.4)',
  });
}

// ── K线周期 Tab ──────────────────────────────────────────────────
document.querySelectorAll('.kt[data-iv]').forEach(t=>{
  t.onclick=async ()=>{
    document.querySelectorAll('.kt[data-iv]').forEach(x=>x.classList.remove('act'));
    t.classList.add('act');curIv=t.dataset.iv;
    if(S.sel){
      initTV(S.sel,curIv);
      await loadSymbolDetail(S.sel,true);
    }
    updOHLCV();
  };
});

// ── 左侧搜索 ─────────────────────────────────────────────────────
function filterP(q){searchQ=q.toUpperCase();renderPairList();}

function mergeSymbols(next){
  const prevMap=new Map((S.syms||[]).map(s=>[s.symbol,s]));
  return (next||[]).map(symbol=>{
    const prev=prevMap.get(symbol.symbol);
    if(!prev)return symbol;
    if((!symbol.klines||Object.keys(symbol.klines).length===0)&&prev.klines)symbol.klines=prev.klines;
    if((!symbol.current_kline||Object.keys(symbol.current_kline).length===0)&&prev.current_kline)symbol.current_kline=prev.current_kline;
    if((!symbol.big_trades||symbol.big_trades.length===0)&&prev.big_trades)symbol.big_trades=prev.big_trades;
    return symbol;
  });
}

function upsertSymbolDetail(detail){
  if(!detail||!detail.symbol)return;
  const idx=(S.syms||[]).findIndex(s=>s.symbol===detail.symbol);
  if(idx>=0)S.syms[idx]={...S.syms[idx],...detail};
  else S.syms.push(detail);
}

async function loadSymbolDetail(sym,renderAfter=false){
  if(!sym)return null;
  try{
    const detail=await fetch(`/api/symbol/${encodeURIComponent(sym)}`).then(r=>r.json());
    if(!detail)return null;
    upsertSymbolDetail(detail);
    if(renderAfter&&S.sel===sym){
      renderDetail(sym);
      updOHLCV();
    }
    return detail;
  }catch(_){
    return null;
  }
}

// ── 委托记录 Tab ─────────────────────────────────────────────────
function oaTab(i,el){
  oaTabMode=i;
  document.querySelectorAll('.oatab').forEach(t=>t.classList.remove('act'));el.classList.add('act');
  renderOrders();
}

function renderOrders(){
  const hdr=document.getElementById('oa-hdr');
  const list=document.getElementById('oa-list');
  const cancelBtn=document.getElementById('cancel-all-btn');
  const tabs=document.querySelectorAll('.oatab');
  if(tabs[0]) tabs[0].textContent=`当前委托(${(S.trader.open_orders||[]).length})`;

  if(oaTabMode===3){ // 持有币种
    hdr.innerHTML=`<span class="oa-col" style="flex:1">资产</span><span class="oa-col" style="flex:1">可用数量</span><span class="oa-col" style="flex:1">冻结数量</span><span class="oa-col" style="flex:1">总量</span>`;
    list.innerHTML=(S.trader.balances||[]).length?(S.trader.balances||[]).map(b=>`
      <div class="oa-row">
        <span style="flex:1;font-weight:700">${b.asset}</span>
        <span style="flex:1;color:var(--t2)">${fNum(b.available)}</span>
        <span style="flex:1;color:var(--t2)">${fNum(b.locked)}</span>
        <span style="flex:1;color:var(--t2)">${fNum((b.available||0)+(b.locked||0))}</span>
      </div>`).join(''):'<div class="oa-empty">暂无持仓。</div>';
    cancelBtn.style.display='none';return;
  }
  if(oaTabMode===4){ // 机器人
    hdr.innerHTML=`<span class="oa-col">策略</span>`;
    list.innerHTML='<div class="oa-empty">暂无运行中的机器人。</div>';
    cancelBtn.style.display='none';return;
  }

  cancelBtn.style.display=oaTabMode===0?'block':'none';
  const cols=['日期','交易对','类型','方向','价格','数量','单笔冰山单','完成度','金额','触发条件','SOR','止盈/止损'];
  const widths=[1.2,1,.8,.6,1,1,1.2,.8,1,1.2,.6,.8];
  hdr.innerHTML=cols.map((c,i)=>`<span class="oa-col" style="flex:${widths[i]}">${c}</span>`).join('');

  if(oaTabMode===0){
    list.innerHTML=(S.trader.open_orders||[]).length?(S.trader.open_orders||[]).map(o=>`
      <div class="oa-row">
        <span style="flex:1.2;color:var(--t2)">${o.time}</span>
        <span style="flex:1;font-weight:700">${fmtSym(o.symbol)}</span>
        <span style="flex:.8;color:var(--t2)">${o.order_type}</span>
        <span style="flex:.6;color:${sideColor(o.side)}">●${sideLabel(o.side)}</span>
        <span style="flex:1;font-variant-numeric:tabular-nums">${o.price!=null?fP(o.price):'市价'}</span>
        <span style="flex:1;font-variant-numeric:tabular-nums">${fNum(o.quantity)}</span>
        <span style="flex:1.2;color:var(--t2)">--</span>
        <span style="flex:.8;color:var(--y)">${filledPct(o)}%</span>
        <span style="flex:1;font-variant-numeric:tabular-nums">${fNum((o.price||0)*o.quantity)}</span>
        <span style="flex:1.2;color:var(--t2)">${o.trigger_price!=null?`${o.trigger_kind||'trigger'} @ ${fP(o.trigger_price)}`:'--'}</span>
        <span style="flex:.6;color:var(--t2)">--</span>
        <span style="flex:.8;color:var(--t2)"><button class="oa-cancel-all" style="margin:0;padding:1px 8px" onclick="cancelOrder(${o.order_id})">撤单</button></span>
      </div>`).join('')
    :'<div class="oa-empty">暂无当前委托。</div>';
  } else if(oaTabMode===1){
    list.innerHTML=(S.trader.order_history||[]).length?(S.trader.order_history||[]).map(o=>`
      <div class="oa-row">
        <span style="flex:1.2;color:var(--t2)">${o.time}</span>
        <span style="flex:1;font-weight:700">${fmtSym(o.symbol)}</span>
        <span style="flex:.8;color:var(--t2)">${o.order_type}</span>
        <span style="flex:.6;color:${sideColor(o.side)}">${sideLabel(o.side)}</span>
        <span style="flex:1;font-variant-numeric:tabular-nums">${o.price!=null?fP(o.price):'市价'}</span>
        <span style="flex:1;font-variant-numeric:tabular-nums">${fNum(o.quantity)}</span>
        <span style="flex:1.2;color:var(--t2)">--</span>
        <span style="flex:.8;color:var(--y)">${filledPct(o)}%</span>
        <span style="flex:1;font-variant-numeric:tabular-nums">${fNum(o.filled_quote_qty||0)}</span>
        <span style="flex:1.2;color:var(--t2)">${o.trigger_price!=null?`${o.trigger_kind||'trigger'} @ ${fP(o.trigger_price)}`:o.status}</span>
        <span style="flex:.6;color:var(--t2)">${o.time_in_force}</span>
        <span style="flex:.8;color:var(--t2)">--</span>
      </div>`).join(''):'<div class="oa-empty">暂无历史委托。</div>';
  } else if(oaTabMode===2){
    list.innerHTML=(S.trader.trade_history||[]).length?(S.trader.trade_history||[]).map(t=>`
      <div class="oa-row">
        <span style="flex:1.2;color:var(--t2)">${t.time}</span>
        <span style="flex:1;font-weight:700">${fmtSym(t.symbol)}</span>
        <span style="flex:.8;color:var(--t2)">${t.liquidity}</span>
        <span style="flex:.6;color:${sideColor(t.side)}">${sideLabel(t.side)}</span>
        <span style="flex:1;font-variant-numeric:tabular-nums">${fP(t.price)}</span>
        <span style="flex:1;font-variant-numeric:tabular-nums">${fNum(t.quantity)}</span>
        <span style="flex:1.2;color:var(--t2)">--</span>
        <span style="flex:.8;color:var(--y)">100%</span>
        <span style="flex:1;font-variant-numeric:tabular-nums">${fNum(t.quote_quantity)}</span>
        <span style="flex:1.2;color:var(--t2)">成交#${t.trade_id}</span>
        <span style="flex:.6;color:var(--t2)">--</span>
        <span style="flex:.8;color:var(--t2)">--</span>
      </div>`).join(''):'<div class="oa-empty">暂无历史成交。</div>';
  }
}

// ── 交易类型切换 ─────────────────────────────────────────────────
function setType(i,el){
  tradeType=i;
  document.querySelectorAll('.ttype').forEach(t=>t.classList.remove('act'));
  el.classList.add('act');
  const isMarket=i===1, isStop=i===2;
  ['buy','sell'].forEach(side=>{
    const priceInput=document.getElementById(`${side}-price`);
    const priceLabel=priceInput.parentElement.parentElement.querySelector('.ta-label');
    const stopBox=document.getElementById(`${side}-stop-box`);
    if(isMarket){
      priceInput.disabled=true;
      priceInput.placeholder='按市价成交';
      priceLabel.textContent=side==='buy'?'参考价格':'参考价格';
    }else{
      priceInput.disabled=false;
      priceInput.placeholder='0.00';
      priceLabel.textContent=side==='buy'?'买入价格':'卖出价格';
      autofillTradeForm(side);
    }
    stopBox.classList.toggle('show',isStop);
  });
}

// ── BBO 填价 ────────────────────────────────────────────────────
function setBBO(side){
  if(!S.sel)return;
  const s=S.syms.find(x=>x.symbol===S.sel);if(!s)return;
  if(side==='buy')document.getElementById('buy-price').value=fP(s.bid||sv(S.sel,'mid'));
  else document.getElementById('sell-price').value=fP(s.ask||sv(S.sel,'mid'));
  updateTotals();
}

function setBuyPct(pct){
  // 根据可用余额计算数量
  const price=parseFloat(document.getElementById('buy-price').value)||sv(S.sel||'','mid')||1;
  const avail=getBalance('USDT').available||0;
  const qty=(avail*pct/100/price);
  document.getElementById('buy-qty').value=qty>0?qty.toFixed(6):'';
  const total=qty*price;
  document.getElementById('buy-total').textContent=total.toFixed(2);
  setPctActive('buy',pct);
  document.getElementById('buy-pct').value=pct;
}

function setSellPct(pct){
  const asset=(S.sel||'').replace('USDT','');
  const avail=getBalance(asset).available||0;
  const qty=(avail*pct/100);
  const price=parseFloat(document.getElementById('sell-price').value)||sv(S.sel||'','mid')||1;
  document.getElementById('sell-qty').value=qty>0?qty.toFixed(6):'';
  document.getElementById('sell-total').textContent=(qty*price).toFixed(2);
  setPctActive('sell',pct);
  document.getElementById('sell-pct').value=pct;
}

function setTradePct(side,pct,el){
  if(side==='buy')setBuyPct(pct); else setSellPct(pct);
}

function setPctActive(side,pct){
  document.querySelectorAll(`#trade-area .ta-side ${side==='buy'?'.ta-pcts:first-of-type':''}`);
  const box=document.querySelectorAll('.ta-side')[side==='buy'?0:1].querySelectorAll('.ta-pct');
  box.forEach(btn=>btn.classList.toggle('act',btn.textContent===`${pct}%`));
}

function autofillTradeForm(side){
  if(!S.sel)return;
  const s=S.syms.find(x=>x.symbol===S.sel); if(!s)return;
  const price = tradeType===0 ? (side==='buy' ? (s.bid||sv(S.sel,'mid')) : (s.ask||sv(S.sel,'mid'))) : sv(S.sel,'mid');
  const id = side==='buy'?'buy-price':'sell-price';
  document.getElementById(id).value = fP(price);
  updateTotals();
}

function updateTotals(){
  const bp=parseFloat(document.getElementById('buy-price').value)||sv(S.sel||'','mid')||0;
  const bq=parseFloat(document.getElementById('buy-qty').value)||0;
  document.getElementById('buy-total').textContent=(bp*bq).toFixed(2);
  const sp=parseFloat(document.getElementById('sell-price').value)||sv(S.sel||'','mid')||0;
  const sq=parseFloat(document.getElementById('sell-qty').value)||0;
  document.getElementById('sell-total').textContent=(sp*sq).toFixed(2);
}

// ── 真实下单 ────────────────────────────────────────────────────
async function doTrade(side){
  if(!S.sel)return;
  const priceId=side==='buy'?'buy-price':'sell-price';
  const qtyId=side==='buy'?'buy-qty':'sell-qty';
  const price=parseFloat(document.getElementById(priceId).value)||sv(S.sel,'mid');
  const qty=parseFloat(document.getElementById(qtyId).value)||0;
  if(!qty||qty<=0){alert('请输入有效数量');return;}
  const payload={
    symbol:S.sel,
    side,
    order_type:tradeType===2?(price?'stop_limit':'stop_market'):(tradeType===1?'market':'limit'),
    time_in_force:tradeType===1?'ioc':'gtc',
    price,
    quantity:qty,
    trigger_price: tradeType===2 ? (parseFloat(document.getElementById(`${side}-trigger-price`).value)||null) : null,
    trigger_kind: tradeType===2 ? document.getElementById(`${side}-trigger-kind`).value : null
  };
  if(tradeType===2 && !payload.trigger_price){alert('请输入触发价');return;}
  const res=await postJson('/api/spot/order',payload);
  if(!res.ok){alert(res.message||'下单失败');return;}
  await refreshSpotState();
  document.querySelectorAll('.oatab').forEach(t=>t.classList.remove('act'));
  document.querySelectorAll('.oatab')[0].classList.add('act');
  oaTabMode=0;
  renderOrders();
  document.getElementById(qtyId).value='';
  if(tradeType===2){
    document.getElementById(`${side}-trigger-price`).value='';
  }
  autofillTradeForm(side);
}

async function cancelOrder(orderId){
  const res=await fetch(`/api/spot/order/${orderId}`,{method:'DELETE'});
  const json=await res.json();
  if(!json.ok){alert(json.message||'撤单失败');return;}
  await refreshSpotState();
  renderOrders();
}

async function cancelAll(){
  const res=await postJson('/api/spot/cancel_all',{symbol:S.sel||null});
  if(!res.ok){alert(res.message||'全撤失败');return;}
  await refreshSpotState();
  renderOrders();
}

// ── EMA ──────────────────────────────────────────────────────────
function ema(sym,k,v){if(!S.sm[sym])S.sm[sym]={};const p=S.sm[sym][k];if(p===undefined){S.sm[sym][k]=v;return v;}const r=A*v+(1-A)*p;S.sm[sym][k]=r;return r;}
function sv(sym,k){return S.sm[sym]?.[k]??0;}

// ── CVD Canvas ───────────────────────────────────────────────────
function drawCVD(sym){
  const c=document.getElementById('cvd-c');const ctx=c.getContext('2d');
  const w=c.width=c.offsetWidth||230,h=c.height=44;ctx.clearRect(0,0,w,h);
  const data=(S.cvdH[sym]||[]).map(x=>x.v);if(data.length<2)return;
  const mn=Math.min(...data),mx=Math.max(...data),rng=mx-mn||1;
  const ty=v=>h-3-(v-mn)/rng*(h-6);const step=w/(data.length-1);
  ctx.beginPath();data.forEach((v,i)=>i===0?ctx.moveTo(0,ty(v)):ctx.lineTo(i*step,ty(v)));
  const pos=data[data.length-1]>=0;
  ctx.strokeStyle=pos?'#0ecb81':'#f6465d';ctx.lineWidth=1.5;ctx.stroke();
  ctx.lineTo(w,h);ctx.lineTo(0,h);ctx.closePath();
  const g=ctx.createLinearGradient(0,0,0,h);
  g.addColorStop(0,pos?'rgba(14,203,129,.18)':'rgba(246,70,93,.18)');g.addColorStop(1,'transparent');
  ctx.fillStyle=g;ctx.fill();
  if(mn<0&&mx>0){const zy=ty(0);ctx.beginPath();ctx.moveTo(0,zy);ctx.lineTo(w,zy);ctx.strokeStyle='rgba(255,255,255,.07)';ctx.lineWidth=.5;ctx.stroke();}
}

// ── OHLCV ────────────────────────────────────────────────────────
function updOHLCV(){
  if(!S.sel)return;const s=S.syms.find(x=>x.symbol===S.sel);if(!s)return;
  const ik=IVMAP[curIv]||'1m';const bars=s.klines?.[ik]||[];
  const cur=s.current_kline?.[ik];const bar=cur||(bars.length?bars[bars.length-1]:null);
  if(bar){e('ci-o',fP(bar.o));e('ci-h',fP(bar.h));e('ci-l2',fP(bar.l));
    e('ci-c',fP(bar.c));e('ci-v',fN(bar.v));e('ci-tbr',bar.tbr.toFixed(1)+'%');}
}

// ── 主渲染 ───────────────────────────────────────────────────────
function render(data){
  S.syms=mergeSymbols(data.symbols||[]);S.feed=data.feed||[];
  S.trader=data.trader||{account_id:0,balances:[],open_orders:[],order_history:[],trade_history:[]};

  S.syms.forEach(s=>{
    ema(s.symbol,'obi',s.obi||0);ema(s.symbol,'ps',s.pump_score||0);
    ema(s.symbol,'ds',s.dump_score||0);ema(s.symbol,'ofi',s.ofi||0);ema(s.symbol,'mid',s.mid||0);
    if(!S.sm[s.symbol])S.sm[s.symbol]={};
    S.sm[s.symbol].cvd=s.cvd||0;S.sm[s.symbol].tbr=s.taker_buy_ratio||50;
    if(!S.cvdH[s.symbol])S.cvdH[s.symbol]=[];
    S.cvdH[s.symbol].push({t:nowT(),v:s.cvd||0});
    if(S.cvdH[s.symbol].length>HL)S.cvdH[s.symbol].shift();
    if(!S.tr[s.symbol])S.tr[s.symbol]=[];
    const mid=sv(s.symbol,'mid');
    if(mid>0){const buy=Math.random()>0.45,qty=+(Math.random()*60000+500).toFixed(0);
      S.tr[s.symbol].unshift({p:mid,q:qty,buy,t:nowT()});
      if(S.tr[s.symbol].length>60)S.tr[s.symbol].pop();}
  });

  const act=S.syms.filter(s=>sv(s.symbol,'ps')>=60||sv(s.symbol,'ds')>=60).length;
  e('nc',S.syms.length);e('ns2',act);

  renderPairList();renderPairMini();renderTicker();renderSigs();checkAlerts();

  const cur=S.sel||(S.syms[0]?.symbol);
  if(cur){
    if(!S.sel){S.sel=cur;initTV(cur,curIv);}
    renderDetail(cur);
    const selected=S.syms.find(x=>x.symbol===cur);
    if(selected&&(!selected.klines||Object.keys(selected.klines).length===0)){
      loadSymbolDetail(cur,true);
    }
  }
  updOHLCV();
  renderOrders();
}

// ── 币对列表（三区同时渲染，scard 风格）──────────────────────
function renderPairList(){
  let all=[...S.syms].sort((a,b)=>Math.max(sv(b.symbol,'ps'),sv(b.symbol,'ds'))-Math.max(sv(a.symbol,'ps'),sv(a.symbol,'ds')));
  if(searchQ) all=all.filter(s=>s.symbol.includes(searchQ));
  const sigs=all.filter(s=>sv(s.symbol,'ps')>=60||sv(s.symbol,'ds')>=60);
  const whales=all.filter(s=>s.whale_entry||s.whale_exit);

  const mkCard=(s)=>{
    const sym=s.symbol.replace('USDT','');
    const ps=sv(s.symbol,'ps'),ds=sv(s.symbol,'ds'),obi=sv(s.symbol,'obi');
    const mid=sv(s.symbol,'mid'),chg=s.change_24h_pct||0;
    const chgColor=chg>=0?'var(--g)':'var(--r)';
    const chgBg=chg>=0?'var(--g-dim)':'var(--r-dim)';
    // 决定卡片类型
    let cls='',tagHtml='';
    if(s.whale_entry){
      cls='cc-whale';
      tagHtml=`<span class="cc-tag" style="background:var(--b-glow);color:var(--b);border:1px solid rgba(24,144,255,.2)">🐋 鲸鱼进场</span>`;
    } else if(sv(s.symbol,'ps')>=70||(s.pump_signal)){
      cls='cc-pump';
      tagHtml=`<span class="cc-tag" style="background:var(--g-dim);color:var(--g);border:1px solid rgba(14,203,129,.2)">🚀 拉盘 ${Math.round(ps)}</span>`;
    } else if(sv(s.symbol,'ds')>=70||(s.dump_signal)){
      cls='cc-dump';
      tagHtml=`<span class="cc-tag" style="background:var(--r-dim);color:var(--r);border:1px solid rgba(246,70,93,.2)">📉 砸盘 ${Math.round(ds)}</span>`;
    }
    const scoreBar=ps>0||ds>0?`
      <div class="cc-bars">
        <div class="pib"><div class="pbf pf-g" style="width:${Math.min(100,ps)}%"></div></div>
        <div class="pib"><div class="pbf pf-r" style="width:${Math.min(100,ds)}%"></div></div>
        <div class="pib"><div class="pbf pf-b" style="width:${Math.min(100,Math.abs(obi)*2)}%"></div></div>
      </div>`:'';
    return `<div class="coin-card ${cls}${S.sel===s.symbol?' act':''}" onclick="selSym('${s.symbol}')">
      <div class="cc-h">
        <span class="cc-sym">${sym}<span style="font-size:9px;color:var(--t3);font-weight:400">/USDT</span></span>
        <div class="cc-price-wrap">
          <span class="cc-price" style="color:${chgColor}">${fP(mid)}</span>
          <span class="cc-chg" style="color:${chgColor};background:${chgBg}">${chg>=0?'+':''}${chg.toFixed(2)}%</span>
        </div>
      </div>
      ${tagHtml?`<div>${tagHtml}</div>`:''}
      <div class="cc-stats" style="color:var(--t3)">拉:<span style="color:${ps>=60?'var(--g)':'var(--t2)'}">${Math.round(ps)}</span> &nbsp;砸:<span style="color:${ds>=60?'var(--r)':'var(--t2)'}">${Math.round(ds)}</span> &nbsp;买卖盘失衡:<span>${obi.toFixed(1)}%</span></div>
      ${scoreBar}
    </div>`;
  };

  setHtmlIfChanged('sec-all',all.map(mkCard).join(''),'pairAll');
  setHtmlIfChanged('sec-sigs',sigs.length
    ?sigs.map(mkCard).join('')
    :'<div class="ls-empty">暂无信号币种</div>','pairSig');
  setHtmlIfChanged('sec-whales',whales.length
    ?whales.map(mkCard).join('')
    :'<div class="ls-empty">暂无鲸鱼动态</div>','pairWhale');
  document.getElementById('sec-all-cnt').textContent=all.length;
  document.getElementById('sec-sig-cnt').textContent=sigs.length;
  document.getElementById('sec-whale-cnt').textContent=whales.length;
}

// ── 底部币对快选（当前选中附近5个） ─────────────────────────────
function renderPairMini(){
  const list=[...S.syms].sort((a,b)=>Math.max(sv(b.symbol,'ps'),sv(b.symbol,'ds'))-Math.max(sv(a.symbol,'ps'),sv(a.symbol,'ds'))).slice(0,5);
  setHtmlIfChanged('pair-mini',list.map(s=>{
    const chg=s.change_24h_pct||0,cls=chg>=0?'pmu':'pmd';
    return `<div class="pi-mini" onclick="selSym('${s.symbol}')">
      <span class="pm-sym">${s.symbol.replace('USDT','/U')}</span>
      <span class="pm-p" style="color:${chg>=0?'var(--g)':'var(--r)'}">${fP(sv(s.symbol,'mid'))}</span>
      <span class="pm-c ${cls}">${chg>=0?'+':''}${chg.toFixed(2)}%</span>
    </div>`;
  }).join(''),'pairMini');
}

// ── Ticker ───────────────────────────────────────────────────────
function renderTicker(){
  const top=[...S.syms].sort((a,b)=>Math.abs(b.change_24h_pct||0)-Math.abs(a.change_24h_pct||0)).slice(0,20);
  setHtmlIfChanged('ticker-scroll',top.map(s=>{
    const chg=s.change_24h_pct||0,cls=chg>=0?'tbu':'tbd';
    return `<div class="tbi" onclick="selSym('${s.symbol}')">
      <span class="tb-s">${s.symbol.replace('USDT','/U')}</span>
      <span class="tb-p" style="color:${chg>=0?'var(--g)':'var(--r)'}">${fP(sv(s.symbol,'mid'))}</span>
      <span class="tb-c ${cls}">${chg>=0?'+':''}${chg.toFixed(2)}%</span>
    </div>`;
  }).join(''),'ticker');
}

// ── 选中币种 ─────────────────────────────────────────────────────
async function selSym(sym){
  S.sel=sym;
  initTV(sym,curIv);
  renderDetail(sym);
  renderPairList();
  await loadSymbolDetail(sym,true);
}

// ── 详情 ─────────────────────────────────────────────────────────
function renderDetail(sym){
  const s=S.syms.find(x=>x.symbol===sym);if(!s)return;
  const detailKey=selectedDetailKey(sym);
  if(S.ui.detailKey===detailKey)return;
  S.ui.detailKey=detailKey;
  const mid=sv(sym,'mid'),chg=s.change_24h_pct||0;
  const gc=chg>=0?'var(--g)':'var(--r)';
  const cvd=sv(sym,'cvd'),ps=sv(sym,'ps'),ds=sv(sym,'ds');
  const obi=sv(sym,'obi'),ofi=sv(sym,'ofi'),tbr=sv(sym,'tbr');
  const symShort=sym.replace('USDT','');

  // 顶部导航
  e('nav-sym',sym.replace('USDT','/USDT'));
  es('nav-price',fP(mid),null,gc);
  const nc=document.getElementById('nav-chg');
  nc.textContent=(chg>=0?'+':'')+chg.toFixed(2)+'%';nc.className='nav-chg '+(chg>=0?'nup':'ndn');
  es('nv-chg',(chg>=0?'+':'')+chg.toFixed(2)+'%',null,gc);
  e('nv-hi',fP(s.high_24h||0));e('nv-lo',fP(s.low_24h||0));
  e('nv-vol',fN(s.quote_vol_24h||0));e('nv-sp',s.spread_bps.toFixed(1)+' 基点');
  e('nv-ps',Math.round(ps));es('nv-cvd',fN(cvd),null,cvd>=0?'var(--g)':'var(--r)');

  // 交易表单更新
  document.getElementById('buy-unit').textContent=symShort;
  document.getElementById('sell-unit').textContent=symShort;
  document.getElementById('sell-unit2').textContent=symShort;
  document.getElementById('btn-buy').textContent='买入 '+symShort;
  document.getElementById('btn-sell').textContent='卖出 '+symShort;
  const quoteBal=getBalance('USDT');
  const baseBal=getBalance(symShort);
  e('avail-buy',fNum(quoteBal.available||0));
  document.querySelector('#trade-area .ta-side:nth-child(2) .ta-avail span:last-child').innerHTML=`${fNum(baseBal.available||0)} <span id="sell-unit2" style="color:var(--t3)">${symShort}</span>`;
  if(tradeType!==1){
    autofillTradeForm('buy');
    autofillTradeForm('sell');
  }

  // 买卖比例条
  const totalBid=s.total_bid_volume||0,totalAsk=s.total_ask_volume||0;
  const tot=totalBid+totalAsk||1;
  const bPct=(totalBid/tot*100).toFixed(0);
  const sPct=(100-+bPct);
  document.getElementById('or-b').style.width=bPct+'%';
  document.getElementById('or-b').textContent='买 '+bPct+'%';
  document.getElementById('or-s').textContent='卖 '+sPct+'%';
  updateTotals();

  // 订单簿
  const asks=(s.top_asks||[]).slice(0,12),bids=(s.top_bids||[]).slice(0,12);
  let ac=0,bc=0;
  const ats=asks.map(([p,q])=>{ac+=q;return ac;});
  const bts=bids.map(([p,q])=>{bc+=q;return bc;});
  const mx=Math.max(ac,bc,1);
  document.getElementById('ob-asks').innerHTML=[...asks].reverse().map(([p,q],i)=>{
    const cum=ats[asks.length-1-i];
    return `<div class="ob-row"><div class="ob-bg bga" style="width:${(cum/mx*100).toFixed(0)}%"></div>
      <span class="ob-p ap">${fP(p)}</span><span class="ob-q">${fN(q)}</span><span class="ob-c">${fN(cum)}</span></div>`;
  }).join('');
  document.getElementById('ob-bids').innerHTML=bids.map(([p,q],i)=>{
    const cum=bts[i];
    return `<div class="ob-row"><div class="ob-bg bgb" style="width:${(cum/mx*100).toFixed(0)}%"></div>
      <span class="ob-p bp">${fP(p)}</span><span class="ob-q">${fN(q)}</span><span class="ob-c">${fN(cum)}</span></div>`;
  }).join('');
  es('ob-mid',fP(mid),null,gc);e('ob-bps',s.spread_bps.toFixed(1)+' 基点');

  // 成交记录
  const tr=S.tr[sym]||[];
  e('tr-cnt',tr.length+' 笔');
  document.getElementById('tr-list').innerHTML=tr.slice(0,50).map(t=>`
    <div class="tr-row">
      <span style="color:${t.buy?'var(--g)':'var(--r)'};font-weight:600">${fP(t.p)}</span>
      <span style="color:var(--t2)">${fN(t.q)}</span>
      <span style="color:var(--t3)">${t.t}</span>
    </div>`).join('');

  // 分析面板
  e('rd-sym',sym.replace('USDT','/USDT'));
  es('rd-p',fP(mid),null,gc);
  const rc=document.getElementById('rd-c');
  rc.textContent=(chg>=0?'+':'')+chg.toFixed(2)+'%';
  rc.style.cssText=`background:${chg>=0?'rgba(14,203,129,.12)':'rgba(246,70,93,.12)'};color:${gc}`;
  e('rd-bid',fP(s.bid||0));e('rd-ask',fP(s.ask||0));
  es('rd-chg',(chg>=0?'+':'')+chg.toFixed(2)+'%',null,gc);
  e('rd-vol',fN(s.volume_24h||0));e('rd-hi',fP(s.high_24h||0));e('rd-lo',fP(s.low_24h||0));
  e('rd-ps',Math.round(ps));e('rd-ds',Math.round(ds));
  es('cvd-v',fN(cvd),null,cvd>=0?'var(--g)':'var(--r)');
  drawCVD(sym);

  // 因子
  const factors=[
    {n:'拉盘评分',v:`${Math.round(ps)}/100`,bw:Math.min(100,ps),bc:'gf',vc:ps>=60?'fg':ps>=30?'fy':'fn',tip:ps>=70?'强烈看涨':ps>=60?'看涨信号':ps>=30?'偏多':'无信号'},
    {n:'砸盘评分',v:`${Math.round(ds)}/100`,bw:Math.min(100,ds),bc:'rf2',vc:ds>=60?'fr':ds>=30?'fy':'fn',tip:ds>=70?'强烈看跌':ds>=60?'砸盘压力大':'无砸盘'},
    {n:'订单簿失衡',v:`${obi>=0?'+':''}${obi.toFixed(1)}%`,bw:Math.min(100,Math.abs(obi)*2),bc:obi>=0?'gf':'rf2',vc:obi>10?'fg':obi<-10?'fr':'fn',tip:obi>20?'买方压倒优势':obi>10?'买单偏多':obi<-20?'卖方主导':obi<-10?'卖单偏多':'买卖平衡'},
    {n:'主动买入占比',v:`${tbr.toFixed(1)}%`,bw:tbr,bc:tbr>60?'gf':tbr<40?'rf2':'yf',vc:tbr>60?'fg':tbr<40?'fr':'fy',tip:tbr>70?'主动买入很强':tbr>60?'偏多':tbr<30?'主动卖出很强':'偏空'},
    {n:'主动买卖量差',v:fN(cvd),bw:Math.min(100,Math.abs(cvd)/500),bc:cvd>=0?'gf':'rf2',vc:cvd>0?'fg':'fr',tip:cvd>50000?'大量净流入':cvd>0?'净买入':cvd<-50000?'大量净流出':'净卖出'},
    {n:'挂单变化强度',v:fN(ofi),bw:Math.min(100,Math.abs(ofi)/100),bc:ofi>0?'gf':'rf2',vc:ofi>3000?'fg':ofi<-3000?'fr':'fn',tip:ofi>5000?'买方挂单明显增强':ofi>2000?'买方在持续加单':ofi<-5000?'卖方挂单明显增强':'买卖挂单较平衡'},
    {n:'买卖价差',v:`${s.spread_bps.toFixed(1)} 基点`,bw:Math.min(100,s.spread_bps*3),bc:s.spread_bps<20?'gf':'yf',vc:s.spread_bps<10?'fg':s.spread_bps<30?'fy':'fn',tip:s.spread_bps<10?'成交环境很好':s.spread_bps<20?'正常':'价差偏大'},
    {n:'鲸鱼',v:s.whale_entry?'进场':s.whale_exit?'离场':'观望',bw:s.whale_entry?80:s.whale_exit?60:20,bc:s.whale_entry?'gf':s.whale_exit?'rf2':'yf',vc:s.whale_entry?'fg':s.whale_exit?'fr':'fn',tip:s.whale_entry?`大单占比${s.max_bid_ratio.toFixed(1)}%`:s.whale_exit?'大户离场':'暂无动作'},
    {n:'1m异动',v:`${s.anomaly_count_1m}次`,bw:Math.min(100,s.anomaly_count_1m),bc:s.anomaly_count_1m>50?'rf2':'yf',vc:s.anomaly_count_1m>100?'fr':s.anomaly_count_1m>50?'fy':'fn',tip:s.anomaly_count_1m>200?'极不稳定':s.anomaly_count_1m>50?'较多异动':'平稳'},
  ];
  document.getElementById('rf-list').innerHTML=factors.map(f=>`
    <div class="fi"><div class="fi-n">${f.n}</div>
      <div><div class="fi-bar"><div class="fi-f ${f.bc}" style="width:${f.bw}%"></div></div>
      <div class="fi-tip">${f.tip}</div></div>
      <div class="fi-v ${f.vc}">${f.v}</div></div>`).join('');

  // 大单
  const bigT=s.big_trades||[];e('bt-cnt',bigT.length);
  document.getElementById('bt-list').innerHTML=bigT.length
    ?bigT.map(bt=>`<div class="bt-row"><span class="btdot ${bt.buy?'db':'ds'}"></span>
      <span class="bt-dir ${bt.buy?'btu':'btd'}">${bt.buy?'主动买':'主动卖'}</span>
      <span style="color:${bt.buy?'var(--g)':'var(--r)'}">${fP(bt.p)}</span>
      <span style="color:var(--y);font-weight:700;margin-left:auto">${fN(bt.q)}</span>
      <span style="color:var(--t3);margin-left:5px">${typeof bt.t==='number'?new Date(bt.t).toLocaleTimeString('zh-CN',{hour12:false}):bt.t}</span>
    </div>`).join('')
    :'<div class="empty-p">等待大单...</div>';
}

// ── 信号 ─────────────────────────────────────────────────────────
function renderSigs(){
  const sigs=[];const seen=new Set();
  S.feed.slice(0,40).forEach(f=>{const k=f.time+f.symbol+f.type;if(seen.has(k))return;seen.add(k);
    sigs.push({time:f.time,sym:f.symbol.replace('USDT',''),full:f.symbol,type:f.type,score:f.score,desc:f.desc,fresh:sigs.length<2});});
  S.syms.forEach(s=>{
    const ps=sv(s.symbol,'ps'),ds=sv(s.symbol,'ds');
    if(ps>=70&&!sigs.find(x=>x.full===s.symbol&&x.type==='pump'))
      sigs.unshift({time:'实时',sym:s.symbol.replace('USDT',''),full:s.symbol,type:'pump',score:Math.round(ps),
        desc:`评分${Math.round(ps)} 买卖盘失衡${sv(s.symbol,'obi').toFixed(1)}% 主动买入占比${sv(s.symbol,'tbr').toFixed(0)}%`,fresh:false});
    if(ds>=70&&!sigs.find(x=>x.full===s.symbol&&x.type==='dump'))
      sigs.unshift({time:'实时',sym:s.symbol.replace('USDT',''),full:s.symbol,type:'dump',score:Math.round(ds),
        desc:`评分${Math.round(ds)} 买卖盘失衡${sv(s.symbol,'obi').toFixed(1)}%`,fresh:false});
  });
  e('sig-cnt',sigs.length);
  const lbl={pump:'🚀 拉盘',dump:'📉 砸盘',whale:'🐋 鲸鱼',anomaly:'⚠️ 异动',cvd:'📊 主动买卖量差'};
  setHtmlIfChanged(
    'sig-list',
    sigs.slice(0,20).map((s,i)=>`
      <div class="scard ${s.type}" onclick="selSym('${s.full}')">
        ${i===0?'<div class="sc-new">NEW</div>':''}
        <div class="sc-h"><span class="sc-sym">${s.sym}</span><span class="sc-t">${s.time}</span></div>
        <div class="sc-tag">${lbl[s.type]||s.type}</div>
        <div class="sc-desc">${s.desc}</div>
        ${s.score!=null?`<div class="sc-score">
          <div class="sc-score-bar"><div class="sc-score-fill" style="width:${Math.min(100,s.score)}%;background:${s.type==='pump'?'var(--g)':s.type==='dump'?'var(--r)':s.type==='whale'?'var(--b)':'var(--p)'}"></div></div>
          <span class="sc-score-v">${s.score}</span>
        </div>`:''}
      </div>`).join('')||'<div class="empty-p">📡<br>等待信号<br><span style="color:var(--t3)">评分 ≥ 70 触发</span></div>',
    'signals'
  );
}

// ── 预警 ─────────────────────────────────────────────────────────
function checkAlerts(){
  S.syms.forEach(s=>{
    const ps=sv(s.symbol,'ps'),ds=sv(s.symbol,'ds'),sym=s.symbol.replace('USDT',''),t=nowT();
    if(ps>=75){const id=`p-${s.symbol}-${Math.floor(Date.now()/30000)}`;
      if(!S.seen.has(id)){S.seen.add(id);S.alerts.unshift({type:'pump',sym,full:s.symbol,
        tag:'🚀 拉盘',time:t,desc:`评分${Math.round(ps)}/100 买卖盘失衡${sv(s.symbol,'obi').toFixed(1)}%`,fresh:true});}}
    if(ds>=75){const id=`d-${s.symbol}-${Math.floor(Date.now()/30000)}`;
      if(!S.seen.has(id)){S.seen.add(id);S.alerts.unshift({type:'dump',sym,full:s.symbol,
        tag:'📉 砸盘',time:t,desc:`评分${Math.round(ds)}/100 买卖盘失衡${sv(s.symbol,'obi').toFixed(1)}%`,fresh:true});}}
    if(s.whale_entry){const id=`w-${s.symbol}-${Math.floor(Date.now()/60000)}`;
      if(!S.seen.has(id)){S.seen.add(id);S.alerts.unshift({type:'whale',sym,full:s.symbol,
        tag:'🐋 鲸鱼进场',time:t,desc:`大单${s.max_bid_ratio.toFixed(1)}% 主动买卖量差${fN(sv(s.symbol,'cvd'))}`,fresh:true});}}
    const cvd=sv(s.symbol,'cvd');
    if(Math.abs(cvd)>50000){const id=`c-${s.symbol}-${Math.floor(Date.now()/120000)}`;
      if(!S.seen.has(id)){S.seen.add(id);S.alerts.unshift({type:'cvd',sym,full:s.symbol,
        tag:cvd>0?'📈 主动买入占优':'📉 主动卖出占优',time:t,desc:`主动买卖量差${fN(cvd)} 主动买入占比${sv(s.symbol,'tbr').toFixed(0)}%`,fresh:true});}}
  });
  if(S.alerts.length>50)S.alerts=S.alerts.slice(0,50);
  e('al-cnt',S.alerts.length);
  setHtmlIfChanged('al-list',S.alerts.map((a,i)=>`
    <div class="scard ${a.type}" onclick="selSym('${a.full}')">
      ${a.fresh&&i===0?'<div class="sc-new">NEW</div>':''}
      <span class="sc-x" onclick="event.stopPropagation();S.alerts.splice(${i},1);checkAlerts()">✕</span>
      <div class="sc-h"><span class="sc-sym">${a.sym}</span><span class="sc-t">${a.time}</span></div>
      <div class="sc-tag">${a.tag}</div>
      <div class="sc-desc">${a.desc}</div>
    </div>`).join('')||'<div class="empty-p">🔔<br>等待预警<br><span style="color:var(--t3)">评分 ≥ 75 触发</span></div>','alerts');
}

// ── 工具 ─────────────────────────────────────────────────────────
function e(id,txt){const el=document.getElementById(id);if(el)el.textContent=txt;}
function es(id,txt,cls,color){const el=document.getElementById(id);if(!el)return;el.textContent=txt;if(cls)el.className=cls;if(color)el.style.color=color;}
function setHtmlIfChanged(id,html,cacheKey){
  if(S.ui[cacheKey]===html)return;
  S.ui[cacheKey]=html;
  const el=document.getElementById(id);
  if(el)el.innerHTML=html;
}
function fP(p){if(!p)return '--';return p>=1000?p.toFixed(1):p>=10?p.toFixed(2):p>=1?p.toFixed(3):p>=.1?p.toFixed(4):p.toFixed(6);}
function fN(n){const v=+n;return Math.abs(v)>=1e9?(v/1e9).toFixed(1)+'B':Math.abs(v)>=1e6?(v/1e6).toFixed(1)+'M':Math.abs(v)>=1e3?(v/1e3).toFixed(1)+'K':v.toFixed(0);}
function fNum(n){const v=+n;return Math.abs(v)>=1000?fN(v):v.toFixed(v>=1?4:8).replace(/0+$/,'').replace(/\.$/,'');}
function nowT(){return new Date().toLocaleTimeString('zh-CN',{hour12:false,hour:'2-digit',minute:'2-digit',second:'2-digit'});}
function fmtSym(sym){return sym?sym.replace('USDT','/USDT'):'--';}
function filledPct(order){const q=+order.quantity||0;if(q<=0)return 0;return Math.round((+order.filled_qty||0)/q*100);}
function getBalance(asset){return (S.trader.balances||[]).find(b=>b.asset===asset)||{available:0,locked:0};}
function sideLabel(side){return String(side||'').toUpperCase()==='BUY'?'买':'卖';}
function sideColor(side){return String(side||'').toUpperCase()==='BUY'?'var(--g)':'var(--r)';}
function selectedDetailKey(sym){
  const s=(S.syms||[]).find(x=>x.symbol===sym);if(!s)return '';
  const quoteBal=getBalance('USDT');
  const baseBal=getBalance(sym.replace('USDT',''));
  return JSON.stringify({
    sym,
    mid:sv(sym,'mid'),
    bid:s.bid,ask:s.ask,chg:s.change_24h_pct,cvd:sv(sym,'cvd'),ps:sv(sym,'ps'),ds:sv(sym,'ds'),
    obi:sv(sym,'obi'),ofi:sv(sym,'ofi'),tbr:sv(sym,'tbr'),
    tb:s.total_bid_volume,ta:s.total_ask_volume,sb:s.spread_bps,
    bb:(s.big_trades||[]).slice(0,10).map(t=>[t.t,t.p,t.q,t.buy]),
    bids:(s.top_bids||[]).slice(0,12),asks:(s.top_asks||[]).slice(0,12),
    trader:[quoteBal.available,baseBal.available,tradeType]
  });
}
async function postJson(url,payload){
  const res=await fetch(url,{method:'POST',headers:{'Content-Type':'application/json'},body:JSON.stringify(payload)});
  return res.json();
}
window.addEventListener('error',ev=>{
  const file=String(ev.filename||'');
  const msg=String(ev.message||'');
  if(file.includes('inpage.js')||file.includes('pageProvider.js')||msg.includes('Cannot redefine property: ethereum')){
    ev.preventDefault();
  }
});
window.addEventListener('unhandledrejection',ev=>{
  const msg=String((ev.reason&&ev.reason.message)||ev.reason||'');
  const stack=String((ev.reason&&ev.reason.stack)||'');
  if(msg.includes('Origin not allowed')||stack.includes('inpage.js')||stack.includes('pageProvider.js')){
    ev.preventDefault();
  }
});
async function refreshSpotState(){
  try{S.trader=await fetch('/api/spot/state').then(r=>r.json());}catch(_){}
}
async function openReplay(){
  const atTs=prompt('输入要回放的毫秒时间戳，留空则读取最近归档事件：','');
  if(atTs===null)return;
  const q=atTs.trim()?`?at_ts=${encodeURIComponent(atTs.trim())}&limit=50`:'?limit=50';
  const res=await fetch('/api/spot/replay'+q).then(r=>r.json());
  if(!res.ok){alert(res.message||'回放失败');return;}
  S.trader=res.data.snapshot||S.trader;
  renderOrders();
  const lines=(res.data.events||[]).slice(0,10).map(e=>`${e.seq} | ${e.kind} | ${e.summary}`);
  alert(`已载入回放快照\n事件数: ${(res.data.events||[]).length}\n`+(lines.length?('\n最近事件:\n'+lines.join('\n')):''));
}
function copySym(){
  if(!S.sel)return;const t=S.sel.replace('USDT','_USDT');
  navigator.clipboard.writeText(t).then(()=>{
    document.getElementById('rbcp').textContent='✅ 已复制';
    const tip=document.getElementById('ctip');tip.textContent='✓ '+t;tip.classList.add('show');
    setTimeout(()=>{document.getElementById('rbcp').textContent='📋 复制';tip.classList.remove('show');},2000);});
}
function openBN(){if(!S.sel)return;window.open(`https://www.binance.com/zh-CN/trade/${S.sel.replace('USDT','_USDT')}?type=spot`,'_blank');}
setInterval(()=>{e('htime',new Date().toLocaleTimeString('zh-CN',{hour12:false}));},1000);
window.addEventListener('resize',()=>{if(S.sel)drawCVD(S.sel);});

// ── WebSocket ─────────────────────────────────────────────────────
function connect(){
  const ws=new WebSocket(`ws://${location.host}/ws`);
  ws.onopen=()=>{document.getElementById('wdot').className='wdot live';e('wlbl','实时连接');};
  ws.onmessage=ev=>{try{render(JSON.parse(ev.data));}catch(_){ }};
  ws.onerror=()=>{document.getElementById('wdot').className='wdot';e('wlbl','连接异常');};
  ws.onclose=()=>{document.getElementById('wdot').className='wdot';e('wlbl','重连中...');setTimeout(connect,2000);};
}
window.addEventListener('DOMContentLoaded',()=>{
  document.getElementById('buy-pct').oninput=e=>setBuyPct(e.target.value);
  document.getElementById('sell-pct').oninput=e=>setSellPct(e.target.value);
  ['buy-price','buy-qty','sell-price','sell-qty'].forEach(id=>{
    const el=document.getElementById(id);
    if(el) el.addEventListener('input',()=>updateTotals());
  });
  renderOrders();
  fetch('/api/state').then(r=>r.json()).then(d=>render(d)).catch(()=>{});
  connect();
  setInterval(()=>{ if(S.sel) loadSymbolDetail(S.sel,true); },5000);
});
</script>
</body>
</html>
"#;
