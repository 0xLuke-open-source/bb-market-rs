// src/web/server.rs — 交易员版 Dashboard
//
// 设计原则：信号优先，数字为辅
// ─ 强信号触发时：全屏弹出声音 + 大字提醒
// ─ 数据平滑：3秒滚动均值，不闪烁
// ─ 布局：左侧信号墙(最重要) + 右侧币种状态 + 底部详情

use std::sync::Arc;
use axum::{
    Router,
    extract::{State, WebSocketUpgrade},
    extract::ws::{Message, WebSocket},
    response::{Html, IntoResponse, Json},
    routing::get,
};
use tokio::time::{interval, Duration};
use tower_http::cors::CorsLayer;
use crate::web::state::SharedDashboardState;

pub async fn run_server(state: SharedDashboardState, port: u16) -> anyhow::Result<()> {
    let app = Router::new()
        .route("/", get(serve_dashboard))
        .route("/api/state", get(api_full_state))
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

async fn api_full_state(State(s): State<SharedDashboardState>) -> impl IntoResponse {
    let s = s.read().await;
    Json(s.to_full_snapshot())
}

async fn ws_handler(ws: WebSocketUpgrade, State(s): State<SharedDashboardState>) -> impl IntoResponse {
    ws.on_upgrade(move |socket| ws_loop(socket, s))
}

async fn ws_loop(mut socket: WebSocket, state: SharedDashboardState) {
    // 推送频率：2秒一次（给交易员看，不需要太快）
    let mut tick = interval(Duration::from_millis(2000));
    loop {
        tick.tick().await;
        let json = {
            let s = state.read().await;
            match serde_json::to_string(&s.to_full_snapshot()) {
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
<title>BB-Market · 交易员信号台</title>
<script src="https://cdn.jsdelivr.net/npm/echarts@5.5.0/dist/echarts.min.js"></script>
<style>
*{box-sizing:border-box;margin:0;padding:0}
html,body{height:100%;background:#060a0f;color:#e2e8f0;font-family:-apple-system,BlinkMacSystemFont,'Segoe UI',sans-serif;overflow:hidden;font-size:12px}

/* ══ 整体布局：顶栏 + 左信号 + 中市场 + 右详情 ══ */
body{display:grid;grid-template-rows:46px 1fr;grid-template-columns:380px 220px 220px 1fr;height:100vh;gap:0}

/* ── 顶栏 ── */
#hdr{grid-column:1/5;background:#0a0f1a;border-bottom:1px solid #1a2535;display:flex;align-items:center;padding:0 16px;gap:0}
.logo{font-size:16px;font-weight:800;color:#fff;letter-spacing:-.3px;margin-right:24px}
.logo em{color:#3b82f6;font-style:normal}
.hdr-divider{width:1px;height:24px;background:#1a2535;margin:0 20px}
.hstat{display:flex;flex-direction:column;align-items:center;min-width:56px}
.hstat-l{font-size:9px;color:#475569;text-transform:uppercase;letter-spacing:.06em}
.hstat-v{font-size:14px;font-weight:700;font-variant-numeric:tabular-nums;line-height:1.3}
.hdr-right{margin-left:auto;display:flex;align-items:center;gap:8px;font-size:11px;color:#475569}
.ws-dot{width:7px;height:7px;border-radius:50%;background:#ef4444;flex-shrink:0}
.ws-dot.live{background:#10b981;animation:blink 2s infinite}
@keyframes blink{0%,100%{opacity:1}50%{opacity:.35}}
#hdr-time{font-variant-numeric:tabular-nums;color:#334155}

/* ── 公共面板样式 ── */
.panel{display:flex;flex-direction:column;overflow:hidden;background:#080d16;border-right:1px solid #1a2535}
.panel-hdr{padding:7px 12px;background:#0a0f1a;border-bottom:1px solid #1a2535;font-size:10px;font-weight:700;color:#64748b;text-transform:uppercase;letter-spacing:.08em;flex-shrink:0;display:flex;align-items:center;justify-content:space-between}
.panel-hdr .count{color:#3b82f6;font-weight:800}
.panel-body{flex:1;overflow-y:auto;overflow-x:hidden}
.panel-body::-webkit-scrollbar{width:3px}
.panel-body::-webkit-scrollbar-thumb{background:#1a2535;border-radius:2px}

/* ── 左：信号墙 ── */
#sig-panel{grid-row:2;grid-column:2}

.sc{border-radius:6px;padding:10px 12px;margin:5px 6px;cursor:pointer;border:1px solid transparent;position:relative;overflow:hidden;transition:transform .12s}
.sc:hover{transform:translateX(3px)}
.sc.pump{background:#041a0f;border-color:#065f46}
.sc.dump{background:#1a0505;border-color:#7f1d1d}
.sc.whale{background:#050f1e;border-color:#1d4ed8}
.sc.anomaly{background:#0f0900;border-color:#78350f}

.sc-head{display:flex;justify-content:space-between;align-items:center;margin-bottom:4px}
.sc-sym{font-size:15px;font-weight:800;letter-spacing:-.3px}
.sc.pump  .sc-sym{color:#34d399}
.sc.dump  .sc-sym{color:#f87171}
.sc.whale .sc-sym{color:#60a5fa}
.sc.anomaly .sc-sym{color:#fbbf24}
.sc-time{font-size:9px;color:#475569;font-variant-numeric:tabular-nums}

.sc-tag{font-size:10px;font-weight:700;padding:1px 7px;border-radius:3px;margin-bottom:4px;display:inline-block}
.pump  .sc-tag{background:#064e3b;color:#6ee7b7}
.dump  .sc-tag{background:#7f1d1d;color:#fca5a5}
.whale .sc-tag{background:#1e3a8a;color:#93c5fd}
.anomaly .sc-tag{background:#451a03;color:#fcd34d}

.sc-desc{font-size:11px;color:#94a3b8;line-height:1.45;margin-bottom:5px}
.sc-desc b{color:#e2e8f0;font-weight:600}
.sc-row{display:flex;gap:10px;font-size:10px}
.sc-kv{display:flex;flex-direction:column}
.sc-kv-l{color:#475569;font-size:9px}
.sc-kv-v{font-weight:700;font-variant-numeric:tabular-nums}
.sc-bar{height:2px;border-radius:1px;margin-top:5px;transition:width .6s}
.pump .sc-bar{background:#10b981}
.dump .sc-bar{background:#ef4444}
.sc-new{position:absolute;top:6px;right:6px;background:#ef4444;color:#fff;font-size:8px;font-weight:800;padding:1px 4px;border-radius:8px;animation:fadeOut 4s forwards}
@keyframes fadeOut{0%,60%{opacity:1}100%{opacity:0;pointer-events:none}}

.empty-sig{padding:32px 12px;text-align:center;color:#1e2d42}
.empty-sig-icon{font-size:28px;margin-bottom:8px}
.empty-sig-txt{font-size:11px;color:#334155;line-height:1.6}

/* ── 中：市场总览 ── */
#market-panel{grid-row:2;grid-column:4;border-right:none}

#watch-grid{display:grid;grid-template-columns:repeat(5,1fr);gap:5px;padding:8px;overflow-y:auto;height:100%}
#watch-grid::-webkit-scrollbar{width:3px}
#watch-grid::-webkit-scrollbar-thumb{background:#1a2535;border-radius:2px}

.wc{background:#0a1020;border:1px solid #1a2535;border-radius:6px;padding:7px 9px;cursor:pointer;transition:border-color .2s,background .2s;position:relative}
.wc:hover,.wc.active{border-color:#334155;background:#0d1830}
.wc.active{border-color:#3b82f6!important}
.wc.sp{border-color:#065f46;animation:pg 2.5s infinite}
.wc.sd{border-color:#7f1d1d;animation:rd 2.5s infinite}
.wc.sw{border-color:#1d4ed8}
@keyframes pg{0%,100%{box-shadow:none}50%{box-shadow:0 0 6px rgba(16,185,129,.2)}}
@keyframes rd{0%,100%{box-shadow:none}50%{box-shadow:0 0 6px rgba(239,68,68,.2)}}

.wc-top{display:flex;justify-content:space-between;align-items:center;margin-bottom:3px}
.wc-sym{font-size:12px;font-weight:800;color:#e2e8f0}
.wc-tag{font-size:8px;font-weight:700;padding:1px 5px;border-radius:3px}
.tp{background:#065f46;color:#6ee7b7}
.td{background:#7f1d1d;color:#fca5a5}
.tw{background:#1e3a8a;color:#93c5fd}
.tn{background:#1a2535;color:#475569}

.wc-price{font-size:15px;font-weight:800;color:#f1f5f9;font-variant-numeric:tabular-nums;letter-spacing:-.4px;line-height:1.1}
.wc-chg{font-size:10px;font-weight:700;margin-top:1px}
.up{color:#10b981}.dn{color:#ef4444}.fl{color:#64748b}

/* ── 卡片底部三指标块 ── */
.wc-metrics{display:flex;gap:3px;margin-top:6px}
.wc-metric{flex:1;border-radius:4px;padding:3px 5px;display:flex;flex-direction:column;align-items:center;gap:1px;transition:background .5s}
.wc-metric-label{font-size:8px;font-weight:600;opacity:.7;letter-spacing:.02em}
.wc-metric-val{font-size:11px;font-weight:800;font-variant-numeric:tabular-nums;line-height:1}
/* 拉盘分色阶：0=暗->100=亮橙 */
.mp-0{background:#1a1400;color:#78520a}
.mp-30{background:#241a00;color:#a06010}
.mp-50{background:#2e1f00;color:#d08020}
.mp-60{background:#3d2800;color:#f59e0b}
.mp-70{background:#4a3000;color:#fbbf24}
.mp-80{background:#5a3a00;color:#fcd34d}
.mp-90{background:#6b4500;color:#fde68a}
/* 砸盘分色阶：0=暗->100=亮红 */
.md-0{background:#1a0505;color:#6b1a1a}
.md-30{background:#250707;color:#8b2020}
.md-50{background:#300a0a;color:#b83030}
.md-60{background:#420d0d;color:#dc2626}
.md-70{background:#560f0f;color:#ef4444}
.md-80{background:#6b1111;color:#f87171}
.md-90{background:#7f1d1d;color:#fca5a5}
/* OBI 双色：正=绿 负=红 */
.mo-pos-hi{background:#052e1a;color:#34d399}
.mo-pos-md{background:#041d10;color:#10b981}
.mo-pos-lo{background:#030f09;color:#6ee7b7;opacity:.7}
.mo-neg-hi{background:#2d0a0a;color:#f87171}
.mo-neg-md{background:#1a0505;color:#ef4444}
.mo-neg-lo{background:#0f0303;color:#fca5a5;opacity:.7}
.mo-flat{background:#0d1426;color:#475569}

/* ── 右：详情区（交易所风格，竖向三区域） ── */
#detail-panel{grid-row:2;grid-column:1;display:flex;flex-direction:column;background:#080d16;border-right:1px solid #1a2535}

/* 右上：选中币种信息条 */
#det-header{padding:8px 12px;background:#0a0f1a;border-bottom:1px solid #1a2535;flex-shrink:0}
.det-sym-row{display:flex;align-items:baseline;gap:8px;margin-bottom:3px}
.det-sym{font-size:20px;font-weight:800;color:#e2e8f0}
.det-price{font-size:28px;font-weight:800;font-variant-numeric:tabular-nums;letter-spacing:-.5px}
.det-chg{font-size:11px;font-weight:700}
.det-stats{display:flex;gap:12px}
.det-stat{font-size:10px}
.det-stat-l{color:#475569}
.det-stat-v{font-weight:700;font-variant-numeric:tabular-nums}

/* 右中：订单簿（交易所风格） */
#orderbook{flex:0 0 660px;border-bottom:1px solid #1a2535;display:flex;flex-direction:column;min-height:0}
.ob-hdr{padding:6px 12px;font-size:10px;font-weight:700;color:#64748b;border-bottom:1px solid #1a2535;display:flex;justify-content:space-between;flex-shrink:0}
.ob-col-hdr{display:flex;justify-content:space-between;padding:3px 12px;background:#0a0f1a;font-size:9px;color:#475569;flex-shrink:0}
.ob-asks{display:flex;flex-direction:column-reverse;overflow:hidden;height:290px;flex-shrink:0}
.ob-bids{display:flex;flex-direction:column;overflow:hidden;height:290px;flex-shrink:0}

.ob-row{display:flex;justify-content:space-between;align-items:center;padding:3px 12px;position:relative;cursor:default;transition:background .1s}
.ob-row:hover{background:rgba(255,255,255,.03)}
.ob-bg{position:absolute;top:0;bottom:0;right:0;opacity:.15;transition:width .4s}
.ask-bg{background:#ef4444}.bid-bg{background:#10b981}
.ob-price{font-size:12px;font-weight:700;font-variant-numeric:tabular-nums;position:relative;z-index:1}
.ob-ask .ob-price{color:#f87171}
.ob-bid .ob-price{color:#34d399}
.ob-qty{font-size:11px;color:#94a3b8;font-variant-numeric:tabular-nums;position:relative;z-index:1}
.ob-total{font-size:11px;color:#475569;font-variant-numeric:tabular-nums;position:relative;z-index:1}

.ob-spread{display:flex;align-items:center;justify-content:space-between;padding:4px 12px;background:#0d1426;border-top:1px solid #1a2535;border-bottom:1px solid #1a2535;flex-shrink:0}
.ob-spread-price{font-size:18px;font-weight:800;color:#e2e8f0;font-variant-numeric:tabular-nums}
.ob-spread-info{font-size:9px;color:#475569}
.ob-spread-val{font-size:10px;color:#94a3b8;font-variant-numeric:tabular-nums}

/* 右下：OFI图 + 因子 */
#det-bottom{flex:1;min-height:0;display:flex;flex-direction:column;overflow:hidden}
#ofi-section{flex:0 0 140px;border-bottom:1px solid #1a2535;padding:8px;display:flex;flex-direction:column}
.sec-title{font-size:11px;font-weight:700;color:#64748b;letter-spacing:.02em;margin-bottom:6px}
#ofi-chart{flex:1;min-height:0}

#factor-section{flex:1;overflow-y:auto;padding:8px}
#factor-section::-webkit-scrollbar{width:3px}
#factor-section::-webkit-scrollbar-thumb{background:#1a2535;border-radius:2px}

.factor-item{display:grid;grid-template-columns:80px 1fr auto;align-items:center;gap:8px;padding:5px 4px;border-bottom:1px solid #0d1426}
.factor-item:last-child{border-bottom:none}
.fi-name{font-size:10px;color:#64748b}
.fi-bar{height:4px;background:#1a2535;border-radius:2px;overflow:hidden}
.fi-bar-fill{height:100%;border-radius:2px;transition:width .7s}
.fi-bull-bar{background:#10b981}.fi-bear-bar{background:#ef4444}.fi-neut-bar{background:#475569}
.fi-val{font-size:11px;font-weight:700;font-variant-numeric:tabular-nums;text-align:right;white-space:nowrap}
.fi-bull{color:#34d399}.fi-bear{color:#f87171}.fi-neut{color:#94a3b8}

.det-actions{display:flex;gap:6px;margin-left:8px;align-items:center}
.det-btn{display:flex;align-items:center;gap:4px;padding:4px 10px;border-radius:5px;font-size:10px;font-weight:700;cursor:pointer;border:none;transition:all .15s;white-space:nowrap}
.det-btn-copy{background:#1e2d42;color:#94a3b8;border:1px solid #2d3f55}
.det-btn-copy:hover{background:#2d3f55;color:#e2e8f0}
.det-btn-copy.copied{background:#065f46;color:#34d399;border-color:#065f46}
.det-btn-trade{background:#1a3a5c;color:#60a5fa;border:1px solid #1d4ed8}
.det-btn-trade:hover{background:#1d4ed8;color:#fff}
.copy-tip{position:fixed;bottom:60px;right:20px;background:#065f46;color:#6ee7b7;font-size:11px;font-weight:700;padding:6px 14px;border-radius:6px;opacity:0;transition:opacity .2s;pointer-events:none;z-index:300}
.copy-tip.show{opacity:1}
/* ── Toast ── */
/* ── 预警列 ── */
#alert-panel{grid-row:2;grid-column:3;display:flex;flex-direction:column;overflow:hidden;background:#080d16;border-right:1px solid #1a2535}
.ac{border-radius:6px;padding:10px 12px;margin:5px 6px;border:1px solid transparent;position:relative;overflow:hidden}
.ac.pump{background:#041a0f;border-color:#065f46}
.ac.dump{background:#1a0505;border-color:#7f1d1d}
.ac.whale{background:#050f1e;border-color:#1d4ed8}
.ac-head{display:flex;justify-content:space-between;align-items:center;margin-bottom:3px}
.ac-sym{font-size:14px;font-weight:800}
.ac.pump  .ac-sym{color:#34d399}
.ac.dump  .ac-sym{color:#f87171}
.ac.whale .ac-sym{color:#60a5fa}
.ac-time{font-size:9px;color:#475569;font-variant-numeric:tabular-nums}
.ac-tag{font-size:10px;font-weight:700;padding:1px 7px;border-radius:3px;margin-bottom:3px;display:inline-block}
.pump .ac-tag{background:#064e3b;color:#6ee7b7}
.dump .ac-tag{background:#7f1d1d;color:#fca5a5}
.whale .ac-tag{background:#1e3a8a;color:#93c5fd}
.ac-detail{font-size:11px;color:#94a3b8;line-height:1.4}
.ac-detail b{color:#e2e8f0;font-weight:600}
.ac-close{position:absolute;top:7px;right:8px;font-size:11px;color:#334155;cursor:pointer;transition:color .1s}
.ac-close:hover{color:#94a3b8}
.ac-new{position:absolute;top:5px;right:24px;background:#ef4444;color:#fff;font-size:8px;font-weight:800;padding:1px 4px;border-radius:8px;animation:fadeOut 4s forwards}
@keyframes fadeOut{0%,60%{opacity:1}100%{opacity:0;pointer-events:none}}
.empty-alert{padding:32px 12px;text-align:center;color:#1e2d42}
.empty-alert-icon{font-size:28px;margin-bottom:8px}
.empty-alert-txt{font-size:11px;color:#334155;line-height:1.6}
</style>
</head>
<body>

<!-- ── 顶栏 ── -->
<header id="hdr">
  <div class="logo">BB-<em>Market</em></div>
  <div class="hdr-divider"></div>
  <div class="hstat"><div class="hstat-l">监控币种</div><div class="hstat-v" id="h-count">--</div></div>
  <div class="hdr-divider"></div>
  <div class="hstat"><div class="hstat-l">活跃信号</div><div class="hstat-v" style="color:#10b981" id="h-signals">--</div></div>
  <div class="hdr-divider"></div>
  <div class="hstat"><div class="hstat-l">鲸鱼活跃</div><div class="hstat-v" style="color:#3b82f6" id="h-whales">--</div></div>
  <div class="hdr-divider"></div>
  <div class="hstat"><div class="hstat-l">运行时长</div><div class="hstat-v" id="h-uptime">--</div></div>
  <div class="hdr-right">
    <div class="ws-dot" id="ws-dot"></div>
    <span id="ws-label">连接中</span>
    <span class="hdr-divider" style="margin:0 10px"></span>
    <span id="hdr-time">--:--:--</span>
  </div>
</header>

<!-- ── 左：信号墙 ── -->
<div id="sig-panel" class="panel">
  <div class="panel-hdr">
    📡 实时信号 <span class="count" id="sig-count">0</span>
    <span style="font-size:9px;text-transform:none;letter-spacing:0;color:#334155">评分≥60触发</span>
  </div>
  <div class="panel-body" id="signal-list">
    <div class="empty-sig">
      <div class="empty-sig-icon">📡</div>
      <div class="empty-sig-txt">等待信号...<br>评分≥60 时自动出现</div>
    </div>
  </div>
</div>

<!-- ── 中：市场总览 ── -->
<div id="market-panel" class="panel">
  <div class="panel-hdr">
    市场总览
    <span style="font-size:9px;text-transform:none;letter-spacing:0;color:#334155">点击查看盘口</span>
  </div>
  <div id="watch-grid"></div>
</div>

<!-- ── 右：详情（交易所风格） ── -->
<div id="detail-panel">

  <!-- 选中币信息 -->
  <div id="det-header">
    <div class="det-sym-row">
      <span class="det-sym" id="det-sym">-- / USDT</span>
      <span class="det-price up" id="det-price">--</span>
      <span class="det-chg up" id="det-chg">--</span>
      <div class="det-actions">
        <button class="det-btn det-btn-copy" id="btn-copy" onclick="copySym()" title="复制交易对">
          <svg width="12" height="12" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2.5"><rect x="9" y="9" width="13" height="13" rx="2"/><path d="M5 15H4a2 2 0 0 1-2-2V4a2 2 0 0 1 2-2h9a2 2 0 0 1 2 2v1"/></svg>
          复制
        </button>
        <button class="det-btn det-btn-trade" id="btn-trade" onclick="openBinance()" title="在币安交易">
          <svg width="12" height="12" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2.5"><path d="M18 13v6a2 2 0 0 1-2 2H5a2 2 0 0 1-2-2V8a2 2 0 0 1 2-2h6"/><polyline points="15 3 21 3 21 9"/><line x1="10" y1="14" x2="21" y2="3"/></svg>
          币安交易
        </button>
      </div>
    </div>
    <div class="det-stats">
      <div class="det-stat"><span class="det-stat-l">买一 </span><span class="det-stat-v up" id="det-bid">--</span></div>
      <div class="det-stat"><span class="det-stat-l">卖一 </span><span class="det-stat-v dn" id="det-ask">--</span></div>
      <div class="det-stat"><span class="det-stat-l">价差 </span><span class="det-stat-v" id="det-spread">--</span></div>
      <div class="det-stat"><span class="det-stat-l">拉盘分 </span><span class="det-stat-v" id="det-ps" style="color:#f59e0b">--</span></div>
      <div class="det-stat"><span class="det-stat-l">砸盘分 </span><span class="det-stat-v dn" id="det-ds">--</span></div>
    </div>
  </div>

  <!-- 订单簿 -->
  <div id="orderbook">
    <div class="ob-hdr">
      <span>订单簿</span>
      <span style="font-size:9px;color:#334155">实时深度</span>
    </div>
    <div class="ob-col-hdr">
      <span>价格 (USDT)</span>
      <span>数量</span>
      <span>累计量</span>
    </div>
    <!-- 卖单：从下到上价格降序（最低卖价在底部靠近中间） -->
    <div id="ob-asks" class="ob-asks"></div>
    <!-- 价差中线 -->
    <div class="ob-spread">
      <span class="ob-spread-price" id="ob-mid">--</span>
      <span class="ob-spread-info">最新成交价</span>
      <span class="ob-spread-val" id="ob-spread-bps">-- bps</span>
    </div>
    <!-- 买单：从上到下价格降序（最高买价在顶部靠近中间） -->
    <div id="ob-bids" class="ob-bids"></div>
  </div>

  <!-- OFI 走势图 + 因子 -->
  <div id="det-bottom">
    <div id="ofi-section">
      <div class="sec-title">订单流失衡 (OFI) · 正值=买方主导 · 负值=卖方主导</div>
      <div id="ofi-chart"></div>
    </div>
    <div id="factor-section">
      <div class="sec-title" style="margin-bottom:6px">信号因子解读</div>
      <div id="factor-list"></div>
    </div>
  </div>
</div>

<!-- ── 预警列 ── -->
<div id="alert-panel" class="panel">
  <div class="panel-hdr">
    🔔 预警通知 <span class="count" id="alert-count">0</span>
    <span style="font-size:9px;text-transform:none;letter-spacing:0;color:#334155">评分≥75触发</span>
  </div>
  <div class="panel-body" id="alert-list">
    <div class="empty-alert">
      <div class="empty-alert-icon">🔔</div>
      <div class="empty-alert-txt">等待预警...<br>评分≥75 或鲸鱼进场<br>自动出现</div>
    </div>
  </div>
</div>

<div class="copy-tip" id="copy-tip">✓ 已复制</div>

<script>
const S = {
  syms: [], feed: [],
  selectedSym: null,
  smoothed: {},
  ofiHistory: {},
  seenSignals: new Set(),
};
const SMOOTH_A = 0.3;
const HIST_LEN = 60;

// ── ECharts OFI ──────────────────────────────────────────────────
const ofiChart = echarts.init(document.getElementById('ofi-chart'));
ofiChart.setOption({
  backgroundColor:'transparent',
  grid:{top:4,bottom:18,left:44,right:8},
  tooltip:{trigger:'axis',backgroundColor:'#0f172a',borderColor:'#1e2d42',
    textStyle:{color:'#e2e8f0',fontSize:9},
    formatter: p => `${p[0].axisValue}<br>OFI: <b>${p[0].value}</b>`},
  xAxis:{type:'category',data:[],
    axisLabel:{color:'#475569',fontSize:8,interval:'auto'},
    axisLine:{lineStyle:{color:'#1a2535'}},splitLine:{show:false}},
  yAxis:{type:'value',
    axisLabel:{color:'#475569',fontSize:8,formatter:v=>v>=1000?(v/1000).toFixed(0)+'K':v},
    splitLine:{lineStyle:{color:'#0d1426',type:'dashed'}}},
  series:[{name:'OFI',type:'bar',data:[],barMaxWidth:10,
    itemStyle:{color:p=>p.value>=0?'rgba(16,185,129,.8)':'rgba(239,68,68,.8)',
      borderRadius:p=>p.value>=0?[2,2,0,0]:[0,0,2,2]}}]
});

// ── EMA 平滑 ────────────────────────────────────────────────────
function smooth(sym,key,val){
  if(!S.smoothed[sym])S.smoothed[sym]={};
  const p=S.smoothed[sym][key];
  if(p===undefined){S.smoothed[sym][key]=val;return val;}
  const v=SMOOTH_A*val+(1-SMOOTH_A)*p;
  S.smoothed[sym][key]=v;return v;
}
function sv(sym,key){return S.smoothed[sym]?.[key]??0;}

// ── 主渲染 ───────────────────────────────────────────────────────
function render(data){
  S.syms = data.symbols||[];
  S.feed = data.feed||[];

  S.syms.forEach(s=>{
    smooth(s.symbol,'obi',s.obi||0);
    smooth(s.symbol,'pump_score',s.pump_score||0);
    smooth(s.symbol,'dump_score',s.dump_score||0);
    smooth(s.symbol,'ofi',s.ofi||0);
    smooth(s.symbol,'mid',s.mid||0);
    if(!S.ofiHistory[s.symbol])S.ofiHistory[s.symbol]=[];
    const h=S.ofiHistory[s.symbol];
    h.push({t:new Date().toLocaleTimeString('zh-CN',{hour12:false,hour:'2-digit',minute:'2-digit',second:'2-digit'}),
            v:+sv(s.symbol,'ofi').toFixed(0)});
    if(h.length>HIST_LEN)h.shift();
  });

  // 头部
  const act=S.syms.filter(s=>sv(s.symbol,'pump_score')>=60||sv(s.symbol,'dump_score')>=60).length;
  const wh =S.syms.filter(s=>s.whale_entry||s.whale_exit).length;
  document.getElementById('h-count').textContent   = S.syms.length;
  document.getElementById('h-signals').textContent = act;
  document.getElementById('h-whales').textContent  = wh;
  document.getElementById('h-uptime').textContent  = fmtUptime(data.uptime_secs||0);

  renderSignals();
  renderGrid();
  if(S.selectedSym) renderDetail(S.selectedSym);
  else if(S.syms.length>0) renderDetail(S.syms[0].symbol);
  checkAlerts();
}

// ── 信号墙 ───────────────────────────────────────────────────────
function renderSignals(){
  const signals=[];
  const seen=new Set();
  S.feed.slice(0,30).forEach(f=>{
    const k=f.time+f.symbol+f.type;
    if(seen.has(k))return; seen.add(k);
    signals.push({time:f.time,sym:f.symbol.replace('USDT',''),fullSym:f.symbol,
      type:f.type,score:f.score,desc:f.desc,isNew:signals.length<3});
  });
  // 补实时强信号
  S.syms.forEach(s=>{
    const ps=sv(s.symbol,'pump_score'),ds=sv(s.symbol,'dump_score');
    if(ps>=70&&!signals.find(x=>x.fullSym===s.symbol&&x.type==='pump'))
      signals.unshift({time:'实时',sym:s.symbol.replace('USDT',''),fullSym:s.symbol,
        type:'pump',score:Math.round(ps),desc:`评分${Math.round(ps)} · OBI${sv(s.symbol,'obi').toFixed(1)}% · 正在拉升中`,isNew:false});
    if(ds>=70&&!signals.find(x=>x.fullSym===s.symbol&&x.type==='dump'))
      signals.unshift({time:'实时',sym:s.symbol.replace('USDT',''),fullSym:s.symbol,
        type:'dump',score:Math.round(ds),desc:`评分${Math.round(ds)} · OBI${sv(s.symbol,'obi').toFixed(1)}% · 砸盘压力大`,isNew:false});
  });

  document.getElementById('sig-count').textContent=signals.length;
  if(!signals.length){
    document.getElementById('signal-list').innerHTML=`
      <div class="empty-sig"><div class="empty-sig-icon">📡</div>
      <div class="empty-sig-txt">暂无强信号<br>评分≥60 自动出现</div></div>`;
    return;
  }

  const labels={pump:'🚀 拉盘信号',dump:'📉 砸盘信号',whale:'🐋 鲸鱼进场',anomaly:'⚠️ 异动'};
  document.getElementById('signal-list').innerHTML=signals.slice(0,15).map(s=>{
    const sym=S.syms.find(x=>x.symbol===s.fullSym);
    const price=sym?fmtPrice(sv(s.fullSym,'mid')):'';
    const obi=sym?(sv(s.fullSym,'obi')>0?'+':'')+sv(s.fullSym,'obi').toFixed(1)+'%':'';
    const score=s.score||0;
    return `<div class="sc ${s.type}" onclick="selectSym('${s.fullSym}')">
      ${s.isNew?'<div class="sc-new">NEW</div>':''}
      <div class="sc-head"><span class="sc-sym">${s.sym}</span><span class="sc-time">${s.time}</span></div>
      <div class="sc-tag">${labels[s.type]||s.type}</div>
      <div class="sc-desc">${s.desc}</div>
      ${price?`<div class="sc-row">
        <div class="sc-kv"><div class="sc-kv-l">价格</div><div class="sc-kv-v">${price}</div></div>
        <div class="sc-kv"><div class="sc-kv-l">OBI</div><div class="sc-kv-v">${obi}</div></div>
        ${score>0?`<div class="sc-kv"><div class="sc-kv-l">评分</div><div class="sc-kv-v">${score}</div></div>`:''}
      </div>`:''}
      ${score>0?`<div class="sc-bar" style="width:${Math.min(100,score)}%"></div>`:''}
    </div>`;
  }).join('');
}

// ── 市场网格 ─────────────────────────────────────────────────────
function pumpCls(v){
  if(v>=90)return'mp-90'; if(v>=80)return'mp-80'; if(v>=70)return'mp-70';
  if(v>=60)return'mp-60'; if(v>=50)return'mp-50'; if(v>=30)return'mp-30';
  return'mp-0';
}
function dumpCls(v){
  if(v>=90)return'md-90'; if(v>=80)return'md-80'; if(v>=70)return'md-70';
  if(v>=60)return'md-60'; if(v>=50)return'md-50'; if(v>=30)return'md-30';
  return'md-0';
}
function obiCls(v){
  if(v>30)return'mo-pos-hi'; if(v>10)return'mo-pos-md'; if(v>2)return'mo-pos-lo';
  if(v<-30)return'mo-neg-hi'; if(v<-10)return'mo-neg-md'; if(v<-2)return'mo-neg-lo';
  return'mo-flat';
}
function renderGrid(){
  const sorted=[...S.syms].sort((a,b)=>
    Math.max(sv(b.symbol,'pump_score'),sv(b.symbol,'dump_score'))-
    Math.max(sv(a.symbol,'pump_score'),sv(a.symbol,'dump_score')));
  document.getElementById('watch-grid').innerHTML=sorted.map(s=>{
    const sym=s.symbol.replace('USDT','');
    const ps=sv(s.symbol,'pump_score'),ds=sv(s.symbol,'dump_score');
    const obi=sv(s.symbol,'obi'),mid=sv(s.symbol,'mid');
    const chg=s.price_change_pct||0;
    const isAct=S.selectedSym===s.symbol;
    let cls=isAct?'active':'',tag='tn',tagTxt='—';
    if(s.whale_entry){cls+=' sw';tag='tw';tagTxt='🐋';}
    if(ds>=60){cls+=' sd';tag='td';tagTxt='📉';}
    if(ps>=60){cls+=' sp';tag='tp';tagTxt='🚀';}
    const chgC=chg>.05?'up':chg<-.05?'dn':'fl';
    const chgS=(chg>0?'▲':chg<0?'▼':'')+(Math.abs(chg).toFixed(2))+'%';
    const pCls=pumpCls(ps),dCls=dumpCls(ds),oCls=obiCls(obi);
    return `<div class="wc ${cls}" onclick="selectSym('${s.symbol}')">
      <div class="wc-top"><div class="wc-sym">${sym}</div><div class="wc-tag ${tag}">${tagTxt}</div></div>
      <div class="wc-price">${fmtPrice(mid)}</div>
      <div class="wc-chg ${chgC}">${chgS}</div>
      <div class="wc-metrics">
        <div class="wc-metric ${pCls}">
          <span class="wc-metric-label">拉盘</span>
          <span class="wc-metric-val">${Math.round(ps)}</span>
        </div>
        <div class="wc-metric ${dCls}">
          <span class="wc-metric-label">砸盘</span>
          <span class="wc-metric-val">${Math.round(ds)}</span>
        </div>
        <div class="wc-metric ${oCls}">
          <span class="wc-metric-label">失衡</span>
          <span class="wc-metric-val">${obi.toFixed(0)}</span>
        </div>
      </div>
    </div>`;
  }).join('');
}

// ── 右侧详情（交易所风格订单簿） ────────────────────────────────
function selectSym(sym){
  S.selectedSym=sym;
  renderDetail(sym);
  renderGrid();
}

function renderDetail(sym){
  S.selectedSym=sym;
  const s=S.syms.find(x=>x.symbol===sym);
  if(!s)return;

  const mid=sv(sym,'mid');
  const chg=s.price_change_pct||0;
  const chgC=chg>.05?'up':chg<-.05?'dn':'fl';
  const chgS=(chg>=0?'+':'')+chg.toFixed(2)+'%';

  document.getElementById('det-sym').textContent    = sym.replace('USDT','/USDT');
  document.getElementById('det-price').textContent  = fmtPrice(mid);
  document.getElementById('det-price').className    = 'det-price '+chgC;
  document.getElementById('det-chg').textContent    = chgS;
  document.getElementById('det-chg').className      = 'det-chg '+chgC;
  document.getElementById('det-bid').textContent    = fmtPrice(s.bid||0);
  document.getElementById('det-ask').textContent    = fmtPrice(s.ask||0);
  document.getElementById('det-spread').textContent = s.spread_bps.toFixed(1)+'bps';
  document.getElementById('det-ps').textContent     = Math.round(sv(sym,'pump_score'));
  document.getElementById('det-ds').textContent     = Math.round(sv(sym,'dump_score'));

  // ── 订单簿（交易所风格） ──────────────────────────────────────
  const asks = (s.top_asks||[]).slice(0,25); // 最低卖价在第一位
  const bids = (s.top_bids||[]).slice(0,25); // 最高买价在第一位

  // 计算累计量（用于背景宽度）
  let askCum=0, bidCum=0;
  const askTotals=asks.map(([p,q])=>{askCum+=q;return askCum;});
  const bidTotals=bids.map(([p,q])=>{bidCum+=q;return bidCum;});
  const maxCum=Math.max(askCum,bidCum,1);

  // 卖单（从高到低，渲染后 column-reverse 让低价贴近中间）
  const asksHtml=[...asks].reverse().map(([p,q],i)=>{
    const cum=askTotals[asks.length-1-i];
    const bgW=(cum/maxCum*100).toFixed(1);
    return `<div class="ob-row ob-ask">
      <div class="ob-bg ask-bg" style="width:${bgW}%"></div>
      <span class="ob-price">${fmtPrice(p)}</span>
      <span class="ob-qty">${fmtNum(q)}</span>
      <span class="ob-total">${fmtNum(cum)}</span>
    </div>`;
  }).join('');

  // 买单（从高到低，最高价贴近中间）
  const bidsHtml=bids.map(([p,q],i)=>{
    const cum=bidTotals[i];
    const bgW=(cum/maxCum*100).toFixed(1);
    return `<div class="ob-row ob-bid">
      <div class="ob-bg bid-bg" style="width:${bgW}%"></div>
      <span class="ob-price">${fmtPrice(p)}</span>
      <span class="ob-qty">${fmtNum(q)}</span>
      <span class="ob-total">${fmtNum(cum)}</span>
    </div>`;
  }).join('');

  document.getElementById('ob-asks').innerHTML    = asksHtml;
  document.getElementById('ob-bids').innerHTML    = bidsHtml;
  document.getElementById('ob-mid').textContent   = fmtPrice(mid);
  document.getElementById('ob-mid').className     = 'ob-spread-price '+chgC;
  document.getElementById('ob-spread-bps').textContent = s.spread_bps.toFixed(1)+' bps';

  // ── OFI 图 ──────────────────────────────────────────────────────
  const h=S.ofiHistory[sym]||[];
  ofiChart.setOption({xAxis:{data:h.map(x=>x.t)},series:[{data:h.map(x=>x.v)}]});

  // ── 因子解读（中文，人话） ───────────────────────────────────────
  const ps=sv(sym,'pump_score'),ds=sv(sym,'dump_score'),obi=sv(sym,'obi'),ofi=sv(sym,'ofi');
  const factors=[
    {
      name:'拉盘评分',
      val: Math.round(ps)+'/100',
      barW: Math.min(100,ps), barCls: ps>=60?'fi-bull-bar':ps>=30?'fi-neut-bar':'fi-neut-bar',
      valCls: ps>=60?'fi-bull':ps>=30?'fi-neut':'fi-neut',
      tip: ps>=70?'⚠️ 强烈看涨，主力大概率拉升':ps>=60?'✅ 看涨信号，建议关注':ps>=40?'→ 偏多，尚未到强信号':'→ 无明显做多信号'
    },
    {
      name:'砸盘评分',
      val: Math.round(ds)+'/100',
      barW: Math.min(100,ds), barCls: ds>=60?'fi-bear-bar':'fi-neut-bar',
      valCls: ds>=60?'fi-bear':'fi-neut',
      tip: ds>=70?'⚠️ 强烈看跌，注意风险':ds>=60?'⚠️ 砸盘压力大，谨慎做多':ds>=40?'→ 有卖压，控制仓位':'→ 无明显做空信号'
    },
    {
      name:'订单簿失衡',
      val: (obi>=0?'+':'')+obi.toFixed(1)+'%',
      barW: Math.min(100,Math.abs(obi)*2), barCls: obi>0?'fi-bull-bar':'fi-bear-bar',
      valCls: obi>10?'fi-bull':obi<-10?'fi-bear':'fi-neut',
      tip: obi>20?'买单远多于卖单，买方压倒性优势':obi>10?'买单偏多，买方占优':obi<-20?'卖单远多于买单，卖方主导':obi<-10?'卖单偏多，卖压较大':'买卖基本平衡，方向待定'
    },
    {
      name:'订单流方向',
      val: fmtNum(ofi),
      barW: Math.min(100,Math.abs(ofi)/100), barCls: ofi>0?'fi-bull-bar':'fi-bear-bar',
      valCls: ofi>3000?'fi-bull':ofi<-3000?'fi-bear':'fi-neut',
      tip: ofi>5000?'主力在大量挂买单，意图拉升':ofi>2000?'买方在增加挂单':ofi<-5000?'主力在大量挂卖单，意图砸盘':ofi<-2000?'卖方在增加挂单':'挂单双向平衡，无明显意图'
    },
    {
      name:'价差',
      val: s.spread_bps.toFixed(1)+' bps',
      barW: Math.min(100,s.spread_bps*2), barCls: s.spread_bps<20?'fi-bull-bar':'fi-neut-bar',
      valCls: s.spread_bps<10?'fi-bull':s.spread_bps<30?'fi-neut':'fi-bear',
      tip: s.spread_bps<10?'价差极窄，流动性极好，吃单成本低':s.spread_bps<20?'价差正常，流动性良好':s.spread_bps<50?'价差偏宽，流动性一般':'价差较大，流动性差，注意滑点'
    },
    {
      name:'鲸鱼动向',
      val: s.whale_entry?'进场':s.whale_exit?'离场':'观望',
      barW: s.whale_entry?80:s.whale_exit?60:20, barCls: s.whale_entry?'fi-bull-bar':s.whale_exit?'fi-bear-bar':'fi-neut-bar',
      valCls: s.whale_entry?'fi-bull':s.whale_exit?'fi-bear':'fi-neut',
      tip: s.whale_entry?`🐋 大户正在买入，大单占比${s.max_bid_ratio.toFixed(1)}%，可能预示拉升`:s.whale_exit?`🐋 大户正在卖出，注意砸盘风险`:'大户暂无明显动作，静观其变'
    },
    {
      name:'1分钟异动',
      val: s.anomaly_count_1m+'次',
      barW: Math.min(100,s.anomaly_count_1m), barCls: s.anomaly_count_1m>50?'fi-bear-bar':s.anomaly_count_1m>20?'fi-neut-bar':'fi-neut-bar',
      valCls: s.anomaly_count_1m>100?'fi-bear':s.anomaly_count_1m>50?'fi-neut':'fi-neut',
      tip: s.anomaly_count_1m>200?'⚠️ 异动极多，市场极不稳定，慎入':s.anomaly_count_1m>100?'异动较多，可能有大行情酝酿':s.anomaly_count_1m>50?'有一定异动，保持关注':'市场平稳，暂无异常'
    },
  ];

  document.getElementById('factor-list').innerHTML=factors.map(f=>`
    <div class="factor-item">
      <div class="fi-name">${f.name}</div>
      <div>
        <div class="fi-bar"><div class="fi-bar-fill ${f.barCls}" style="width:${f.barW}%"></div></div>
        <div style="font-size:9px;color:#475569;margin-top:2px;line-height:1.3">${f.tip}</div>
      </div>
      <div class="fi-val ${f.valCls}">${f.val}</div>
    </div>`).join('');
}

// ── Toast 通知 ───────────────────────────────────────────────────
function checkAlerts(){
  S.syms.forEach(s=>{
    const ps=sv(s.symbol,'pump_score'),ds=sv(s.symbol,'dump_score');
    const sym=s.symbol.replace('USDT','');
    if(ps>=75){
      const id=`pump-${s.symbol}-${Math.floor(Date.now()/30000)}`;
      if(!S.seenSignals.has(id)){S.seenSignals.add(id);
        addAlert({type:'pump',icon:'🚀',sym,tag:'🚀 拉盘强信号',
          detail:`评分<b>${Math.round(ps)}</b>/100 · OBI<b>${sv(s.symbol,'obi').toFixed(1)}%</b> · OFI ${fmtNum(sv(s.symbol,'ofi'))}`,isNew:true});}
    }
    if(ds>=75){
      const id=`dump-${s.symbol}-${Math.floor(Date.now()/30000)}`;
      if(!S.seenSignals.has(id)){S.seenSignals.add(id);
        addAlert({type:'dump',icon:'📉',sym,tag:'📉 砸盘强信号',
          detail:`评分<b>${Math.round(ds)}</b>/100 · OBI<b>${sv(s.symbol,'obi').toFixed(1)}%</b>`,isNew:true});}
    }
    if(s.whale_entry){
      const id=`whale-${s.symbol}-${Math.floor(Date.now()/60000)}`;
      if(!S.seenSignals.has(id)){S.seenSignals.add(id);
        addAlert({type:'whale',icon:'🐋',sym,tag:'🐋 鲸鱼进场',
          detail:`大单占比<b>${s.max_bid_ratio.toFixed(1)}%</b> · 买盘量 ${fmtNum(s.total_bid_volume)}`,isNew:true});}
    }
  });
}

// 预警列维护一个内存列表，最多保留30条
if(!window.S_alerts) window.S_alerts=[];
function addAlert(a){
  const t=new Date().toLocaleTimeString('zh-CN',{hour12:false,hour:'2-digit',minute:'2-digit',second:'2-digit'});
  window.S_alerts.unshift({...a,time:t});
  if(window.S_alerts.length>30) window.S_alerts.pop();
  renderAlertPanel();
}
function renderAlertPanel(){
  const list=document.getElementById('alert-list');
  const cnt=document.getElementById('alert-count');
  if(!window.S_alerts.length){
    list.innerHTML=`<div class="empty-alert"><div class="empty-alert-icon">🔔</div><div class="empty-alert-txt">等待预警...<br>评分≥75 或鲸鱼进场<br>自动出现</div></div>`;
    cnt.textContent='0';
    return;
  }
  cnt.textContent=window.S_alerts.length;
  list.innerHTML=window.S_alerts.map((a,i)=>`
    <div class="ac ${a.type}" onclick="selectSym('${a.sym}USDT')">
      ${a.isNew&&i===0?'<div class="ac-new">NEW</div>':''}
      <div class="ac-close" onclick="event.stopPropagation();window.S_alerts.splice(${i},1);renderAlertPanel()">✕</div>
      <div class="ac-head"><span class="ac-sym">${a.sym}</span><span class="ac-time">${a.time}</span></div>
      <div class="ac-tag">${a.tag}</div>
      <div class="ac-detail">${a.detail}</div>
    </div>`).join('');
}

// ── 复制 & 跳转 ──────────────────────────────────────────────────
function copySym(){
  const sym = S.selectedSym;
  if(!sym) return;
  const text = sym.replace('USDT', '_USDT');
  navigator.clipboard.writeText(text).then(()=>{
    const btn = document.getElementById('btn-copy');
    btn.classList.add('copied');
    btn.innerHTML = '<svg width="12" height="12" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2.5"><polyline points="20 6 9 17 4 12"/></svg> 已复制';
    const tip = document.getElementById('copy-tip');
    tip.textContent = '✓ 已复制 ' + text;
    tip.classList.add('show');
    setTimeout(()=>{
      btn.classList.remove('copied');
      btn.innerHTML = '<svg width="12" height="12" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2.5"><rect x="9" y="9" width="13" height="13" rx="2"/><path d="M5 15H4a2 2 0 0 1-2-2V4a2 2 0 0 1 2-2h9a2 2 0 0 1 2 2v1"/></svg> 复制';
      tip.classList.remove('show');
    }, 2000);
  }).catch(()=>{
    // 降级：选中文本
    const el = document.createElement('textarea');
    el.value = text;
    document.body.appendChild(el);
    el.select();
    document.execCommand('copy');
    document.body.removeChild(el);
  });
}

function openBinance(){
  const sym = S.selectedSym;
  if(!sym) return;
  const base = sym.replace('USDT','');
  const url = `https://www.binance.com/zh-CN/trade/${base}_USDT?type=spot`;
  window.open(url, '_blank');
}

// ── 工具 ─────────────────────────────────────────────────────────
function fmtPrice(p){
  if(!p)return '--';
  return p>=1000?p.toFixed(1):p>=10?p.toFixed(2):p>=1?p.toFixed(3):p.toFixed(4);
}
function fmtNum(n){
  const v=+n;
  return Math.abs(v)>=1e6?(v/1e6).toFixed(1)+'M':Math.abs(v)>=1e3?(v/1e3).toFixed(1)+'K':v.toFixed(0);
}
function fmtUptime(s){
  const h=Math.floor(s/3600),m=Math.floor((s%3600)/60),sec=s%60;
  return `${String(h).padStart(2,'0')}:${String(m).padStart(2,'0')}:${String(sec).padStart(2,'0')}`;
}

// 时钟
setInterval(()=>{document.getElementById('hdr-time').textContent=
  new Date().toLocaleTimeString('zh-CN',{hour12:false});},1000);

// WebSocket
function connect(){
  const ws=new WebSocket(`ws://${location.host}/ws`);
  const dot=document.getElementById('ws-dot');
  const lbl=document.getElementById('ws-label');
  ws.onopen=()=>{dot.className='ws-dot live';lbl.textContent='实时连接';};
  ws.onmessage=e=>{try{render(JSON.parse(e.data));}catch(err){console.warn(err);}};
  ws.onerror=()=>{dot.className='ws-dot';lbl.textContent='连接异常';};
  ws.onclose=()=>{dot.className='ws-dot';lbl.textContent='重连中...';setTimeout(connect,2000);};
}
window.addEventListener('DOMContentLoaded',()=>{
  window.addEventListener('resize',()=>ofiChart.resize());
  fetch('/api/state').then(r=>r.json()).then(d=>render(d)).catch(()=>{});
  connect();
});
</script>
</body>
</html>
"#;