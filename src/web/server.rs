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
<title>BB-Market</title>
<style>
*{box-sizing:border-box;margin:0;padding:0}
:root{
  --bg0:#0b0e11;--bg1:#161a1e;--bg2:#1e2329;--bg3:#2b3139;
  --bd:#2b3139;--bd2:#3d4451;
  --t1:#eaecef;--t2:#848e9c;--t3:#5e6673;
  --g:#0ecb81;--r:#f6465d;--y:#f0b90b;--b:#1890ff;--p:#c084fc;
}
html,body{height:100%;background:var(--bg0);color:var(--t1);
  font-family:-apple-system,BlinkMacSystemFont,'Segoe UI',sans-serif;
  font-size:12px;overflow:hidden;line-height:1.4}
body{display:flex;flex-direction:column;height:100vh}

/* ══ 顶部导航 ══ */
#nav{height:42px;background:var(--bg1);border-bottom:1px solid var(--bd);
  display:flex;align-items:center;padding:0 12px;gap:0;flex-shrink:0;z-index:20}
.logo{font-size:16px;font-weight:900;color:var(--t1);letter-spacing:-.3px;margin-right:14px}
.logo em{color:var(--y);font-style:normal}
.nav-sym{display:flex;align-items:center;gap:8px;margin-right:10px}
.nav-sym-name{font-size:15px;font-weight:800}
.nav-price{font-size:19px;font-weight:800;font-variant-numeric:tabular-nums}
.nav-chg{font-size:11px;font-weight:700;padding:2px 6px;border-radius:3px}
.nup{color:var(--g);background:rgba(14,203,129,.12)}.ndn{color:var(--r);background:rgba(246,70,93,.12)}
.nav-stats{display:flex;gap:12px;align-items:center}
.ns{display:flex;flex-direction:column;gap:0}
.ns-l{font-size:9px;color:var(--t3);white-space:nowrap}
.ns-v{font-size:11px;font-weight:600;font-variant-numeric:tabular-nums}
.ndiv{width:1px;height:18px;background:var(--bd);margin:0 8px;flex-shrink:0}
.nav-r{margin-left:auto;display:flex;align-items:center;gap:7px;font-size:11px;color:var(--t2)}
.wdot{width:7px;height:7px;border-radius:50%;background:var(--r);flex-shrink:0}
.wdot.live{background:var(--g);animation:blink 2s infinite}
@keyframes blink{0%,100%{opacity:1}50%{opacity:.35}}

/* ══ K线周期行 ══ */
#ktabs{height:34px;background:var(--bg1);border-bottom:1px solid var(--bd);
  display:flex;align-items:center;padding:0 8px;gap:0;flex-shrink:0}
.kt{padding:3px 7px;font-size:11px;font-weight:600;color:var(--t2);cursor:pointer;
  border-radius:3px;border-bottom:2px solid transparent;white-space:nowrap}
.kt:hover{color:var(--t1);background:var(--bg2)}.kt.act{color:var(--y);border-bottom-color:var(--y)}
.ktd{width:1px;height:14px;background:var(--bd);margin:0 3px}
.ktv{display:flex;gap:10px;align-items:center;margin-left:8px;font-size:10px;color:var(--t2);flex-shrink:0}
.co{display:flex;gap:2px}.co-l{color:var(--t3)}

/* ══ 主体：5列布局 ══ */
/* 左：成交记录 | 中左：订单簿 | 中间：图表+交易 | 中右：分析 | 右：信号+预警 */
#app{flex:1;display:grid;
  grid-template-columns:160px 180px 1fr 240px 240px;
  grid-template-rows:1fr;
  overflow:hidden;min-height:0}

/* ══ 通用面板样式 ══ */
.panel{background:var(--bg1);border-right:1px solid var(--bd);display:flex;flex-direction:column;overflow:hidden}
.ph{display:flex;justify-content:space-between;align-items:center;
  padding:5px 8px;border-bottom:1px solid var(--bd);flex-shrink:0}
.ph-ttl{font-size:11px;font-weight:700}
.ph-sub{font-size:10px;color:var(--t2)}
.ph-cnt{color:var(--y);font-weight:800;font-size:11px}
.ps{flex:1;overflow-y:auto;min-height:0}
.ps::-webkit-scrollbar{width:2px}
.ps::-webkit-scrollbar-thumb{background:var(--bd)}

/* ══ Col1：最新成交 ══ */
#col-trades{grid-column:1}
.tr-col{display:flex;justify-content:space-between;padding:2px 8px;
  background:var(--bg0);font-size:9px;color:var(--t3);flex-shrink:0}
.tr-row{display:flex;justify-content:space-between;align-items:center;
  padding:2px 8px;font-size:10px;font-variant-numeric:tabular-nums;border-bottom:1px solid rgba(43,49,57,.25)}
.tr-row:hover{background:var(--bg2)}

/* ══ Col2：订单簿 ══ */
#col-ob{grid-column:2}
.ob-col{display:flex;justify-content:space-between;padding:2px 8px;
  background:var(--bg0);font-size:9px;color:var(--t3);flex-shrink:0}
.ob-asks{display:flex;flex-direction:column-reverse;overflow:hidden;flex:1}
.ob-bids{display:flex;flex-direction:column;overflow:hidden;flex:1}
.ob-row{display:flex;justify-content:space-between;align-items:center;
  padding:2px 8px;position:relative;cursor:default}
.ob-row:hover{background:var(--bg2)}
.ob-bg{position:absolute;top:0;bottom:0;right:0;opacity:.1}
.bga{background:var(--r)}.bgb{background:var(--g)}
.ob-p{font-size:11px;font-weight:600;font-variant-numeric:tabular-nums;position:relative;z-index:1}
.ap{color:var(--r)}.bp{color:var(--g)}
.ob-q{font-size:10px;color:var(--t2);position:relative;z-index:1;font-variant-numeric:tabular-nums}
.ob-c{font-size:9px;color:var(--t3);position:relative;z-index:1;font-variant-numeric:tabular-nums}
.ob-mid{display:flex;align-items:center;justify-content:space-between;
  padding:4px 8px;background:var(--bg2);border-top:1px solid var(--bd);border-bottom:1px solid var(--bd);flex-shrink:0}
.ob-mid-p{font-size:14px;font-weight:800;font-variant-numeric:tabular-nums}
.ob-bps{font-size:10px;color:var(--t2)}
/* 买卖比例条 */
.ob-ratio{display:flex;height:4px;margin:3px 8px;border-radius:2px;overflow:hidden;flex-shrink:0}
.or-b{background:var(--g);transition:width .8s}.or-s{background:var(--r);flex:1}
.ob-ratio-txt{display:flex;justify-content:space-between;padding:0 8px 3px;font-size:9px;flex-shrink:0}

/* ══ Col3：图表+交易 ══ */
#col-main{grid-column:3;display:flex;flex-direction:column;overflow:hidden;background:var(--bg0)}
#tv-area{flex:1 1 0;min-height:180px;border-bottom:1px solid var(--bd);position:relative;overflow:hidden}
#tv-widget{width:100%;height:100%}
.tv-loading{width:100%;height:100%;display:flex;align-items:center;justify-content:center;color:var(--t3);font-size:11px}

/* 现货交易区 */
#trade-area{flex:0 0 auto;border-bottom:1px solid var(--bd);background:var(--bg1)}
.ta-tabs{display:flex;border-bottom:1px solid var(--bd);padding:0 8px}
.tatab{padding:7px 10px;font-size:11px;font-weight:600;color:var(--t2);cursor:pointer;
  border-bottom:2px solid transparent;white-space:nowrap}
.tatab.act{color:var(--y);border-bottom-color:var(--y)}
.ta-types{display:flex;gap:2px;padding:6px 8px 4px;border-bottom:1px solid var(--bd);flex-shrink:0}
.ttype{padding:3px 10px;border-radius:3px;font-size:11px;font-weight:600;color:var(--t2);
  cursor:pointer;background:transparent}
.ttype.act{color:var(--y);background:var(--bg2)}
.ta-form{display:grid;grid-template-columns:1fr 1fr;gap:8px;padding:8px}
.ta-side{display:flex;flex-direction:column;gap:5px}
.ta-label{font-size:10px;color:var(--t2);margin-bottom:1px}
.ta-input-row{display:flex;align-items:center;background:var(--bg2);border:1px solid var(--bd);border-radius:4px;overflow:hidden;transition:border-color .15s}
.ta-input-row:focus-within{border-color:var(--y)}
.ta-input-row input{flex:1;background:transparent;border:none;color:var(--t1);padding:6px 8px;font-size:12px;outline:none;font-variant-numeric:tabular-nums;width:0}
.ta-input-row span{padding:0 8px;font-size:11px;color:var(--t2);white-space:nowrap;border-left:1px solid var(--bd)}
.ta-input-row .bbo-btn{padding:2px 6px;font-size:9px;font-weight:700;color:var(--y);cursor:pointer;
  border-left:1px solid var(--bd)}
.ta-slider{margin:2px 0}
.ta-slider input{width:100%;accent-color:var(--y)}
.ta-info{display:flex;justify-content:space-between;font-size:10px;color:var(--t2)}
.ta-avail{font-size:10px;color:var(--t2);display:flex;justify-content:space-between;padding:2px 0}
.ta-btn{width:100%;padding:9px;border-radius:4px;font-size:13px;font-weight:700;cursor:pointer;border:none;transition:opacity .15s}
.ta-btn:hover{opacity:.88}
.tb-buy{background:var(--g);color:#000}.tb-sell{background:var(--r);color:#fff}
.ta-fee{font-size:10px;color:var(--t3);text-align:center;margin-top:3px}
.ta-stopsl{display:flex;align-items:center;gap:4px;font-size:10px;color:var(--t2);padding:2px 0}
.ta-stopsl input[type=checkbox]{accent-color:var(--y)}

/* 委托记录区 */
#orders-area{flex:0 0 auto;display:flex;flex-direction:column;overflow:hidden}
.oa-tabs{display:flex;border-bottom:1px solid var(--bd);padding:0 8px;background:var(--bg1);flex-shrink:0}
.oatab{padding:6px 10px;font-size:11px;font-weight:600;color:var(--t2);cursor:pointer;
  border-bottom:2px solid transparent;white-space:nowrap}
.oatab.act{color:var(--y);border-bottom-color:var(--y)}
.oa-hdr{display:flex;align-items:center;padding:3px 8px;background:var(--bg1);
  border-bottom:1px solid var(--bd);font-size:9px;color:var(--t3);flex-shrink:0;gap:0}
.oa-col{padding:2px 4px;white-space:nowrap}
.oa-list{height:100px;overflow-y:auto;background:var(--bg0)}
.oa-list::-webkit-scrollbar{width:2px}
.oa-list::-webkit-scrollbar-thumb{background:var(--bd)}
.oa-row{display:flex;align-items:center;padding:3px 8px;font-size:10px;
  border-bottom:1px solid rgba(43,49,57,.25);gap:0}
.oa-row:hover{background:var(--bg2)}
.oa-empty{padding:16px;text-align:center;color:var(--t3);font-size:11px}
.oa-cancel-all{margin-left:auto;padding:2px 8px;font-size:9px;font-weight:700;
  color:var(--r);background:transparent;border:1px solid var(--r);border-radius:3px;cursor:pointer}
.oa-cancel-all:hover{background:rgba(246,70,93,.1)}

/* ══ Col4：分析（展开，不用点击） ══ */
#col-analysis{grid-column:4;background:var(--bg1);border-right:1px solid var(--bd);
  display:flex;flex-direction:column;overflow:hidden}
.ca-scroll{flex:1;overflow-y:auto}
.ca-scroll::-webkit-scrollbar{width:3px}
.ca-scroll::-webkit-scrollbar-thumb{background:var(--bd)}
.ca-price{padding:10px 10px 7px;border-bottom:1px solid var(--bd)}
.cap-r1{display:flex;align-items:baseline;gap:6px;margin-bottom:5px;flex-wrap:wrap}
.cap-sym{font-size:13px;font-weight:800}
.cap-p{font-size:22px;font-weight:800;font-variant-numeric:tabular-nums;letter-spacing:-.5px}
.cap-c{font-size:11px;font-weight:700;padding:2px 6px;border-radius:3px}
.cap-btns{display:flex;gap:4px;margin-top:6px}
.cbtn{padding:4px 9px;border-radius:4px;font-size:11px;font-weight:700;cursor:pointer;
  border:none;display:flex;align-items:center;gap:3px;white-space:nowrap}
.ccp{background:var(--bg2);color:var(--t2);border:1px solid var(--bd)}
.ccp:hover{color:var(--t1)}.cbn{background:rgba(24,144,255,.12);color:var(--b);border:1px solid var(--b)}
.cbn:hover{background:var(--b);color:#fff}
.cap-stats{display:flex;flex-wrap:wrap}
.cst{flex:0 0 50%;padding:2px 0;font-size:11px}
.cst-l{color:var(--t3)}.cst-v{font-weight:700;font-variant-numeric:tabular-nums}
.ca-cvd{padding:7px 10px;border-bottom:1px solid var(--bd)}
.cvd-hdr{display:flex;justify-content:space-between;align-items:center;margin-bottom:4px}
.cvd-ttl{font-size:10px;font-weight:700;color:var(--t2)}
.cvd-v{font-size:13px;font-weight:800;font-variant-numeric:tabular-nums}
#cvd-c{width:100%;height:44px}
.ca-fac{padding:7px 10px;border-bottom:1px solid var(--bd)}
.caf-ttl{font-size:10px;font-weight:700;color:var(--t2);margin-bottom:5px}
.fi{display:grid;grid-template-columns:68px 1fr 48px;align-items:center;gap:3px;
  padding:3px 0;border-bottom:1px solid rgba(43,49,57,.4)}
.fi:last-child{border:none}
.fi-n{font-size:10px;color:var(--t3)}
.fi-bar{height:3px;background:var(--bg0);border-radius:2px;overflow:hidden}
.fi-f{height:100%;border-radius:2px;transition:width .6s}
.gf{background:var(--g)}.rf2{background:var(--r)}.yf{background:var(--y)}
.fi-v{font-size:11px;font-weight:700;text-align:right;font-variant-numeric:tabular-nums}
.fg{color:var(--g)}.fr{color:var(--r)}.fy{color:var(--y)}.fn{color:var(--t2)}
.fi-tip{font-size:9px;color:var(--t3);margin-top:1px}
.ca-bt-hdr{display:flex;justify-content:space-between;align-items:center;
  padding:5px 10px;border-bottom:1px solid var(--bd);font-size:10px;font-weight:700;color:var(--t2)}
.ca-bt-cnt{color:var(--y);font-weight:800}
.bt-row{display:flex;align-items:center;gap:5px;padding:4px 10px;font-size:11px;
  font-variant-numeric:tabular-nums;border-bottom:1px solid rgba(43,49,57,.25)}
.bt-row:hover{background:var(--bg2)}
.btdot{width:5px;height:5px;border-radius:50%;flex-shrink:0}
.db{background:var(--g)}.ds{background:var(--r)}
.bt-dir{font-weight:700;width:42px}.btu{color:var(--g)}.btd{color:var(--r)}

/* ══ Col5：信号+预警（展开，不点击） ══ */
#col-alerts{grid-column:5;background:var(--bg1);display:flex;flex-direction:column;overflow:hidden;border-left:1px solid var(--bd)}
/* 右栏顶部：两栏标题 */
.ra-header{display:flex;border-bottom:1px solid var(--bd);flex-shrink:0;height:32px;align-items:stretch}
.ra-sec-hdr{flex:1;display:flex;align-items:center;justify-content:space-between;
  padding:0 10px;font-size:11px;font-weight:700;color:var(--t2);border-right:1px solid var(--bd)}
.ra-sec-hdr:last-child{border-right:none}
.ra-cnt{color:var(--y);font-weight:800;font-size:12px}
/* 信号+预警并排 */
.ra-body{flex:1;display:grid;grid-template-columns:1fr 1fr;overflow:hidden;min-height:0}
.ra-col{display:flex;flex-direction:column;overflow:hidden;border-right:1px solid var(--bd)}
.ra-col:last-child{border-right:none}
.ra-list{flex:1;overflow-y:auto}
.ra-list::-webkit-scrollbar{width:2px}
.ra-list::-webkit-scrollbar-thumb{background:var(--bd)}
.scard{padding:7px 8px;border-bottom:1px solid var(--bd);cursor:pointer;
  transition:background .1s;position:relative;border-left:3px solid transparent}
.scard:hover{background:var(--bg2)}
.scard.pump{border-left-color:var(--g)}.scard.dump{border-left-color:var(--r)}
.scard.whale{border-left-color:var(--b)}.scard.cvd{border-left-color:var(--p)}
.sc-h{display:flex;justify-content:space-between;align-items:center;margin-bottom:2px}
.sc-sym{font-size:12px;font-weight:800}
.pump .sc-sym{color:var(--g)}.dump .sc-sym{color:var(--r)}.whale .sc-sym{color:var(--b)}.cvd .sc-sym{color:var(--p)}
.sc-t{font-size:9px;color:var(--t3)}
.sc-tag{font-size:9px;font-weight:700;padding:1px 5px;border-radius:3px;display:inline-block;margin-bottom:2px}
.pump .sc-tag{background:rgba(14,203,129,.12);color:var(--g)}
.dump .sc-tag{background:rgba(246,70,93,.12);color:var(--r)}
.whale .sc-tag{background:rgba(24,144,255,.12);color:var(--b)}
.cvd .sc-tag{background:rgba(192,132,252,.12);color:var(--p)}
.sc-desc{font-size:10px;color:var(--t2);line-height:1.4}
.sc-bar{height:2px;border-radius:1px;margin-top:3px}
.sc-new{position:absolute;top:5px;right:6px;background:var(--r);color:#fff;
  font-size:7px;font-weight:800;padding:1px 3px;border-radius:6px;animation:fo 4s forwards}
@keyframes fo{0%,60%{opacity:1}100%{opacity:0;pointer-events:none}}
.sc-x{position:absolute;top:5px;right:6px;font-size:10px;color:var(--t3);cursor:pointer}
.sc-x:hover{color:var(--t1)}
.empty-p{padding:14px 8px;text-align:center;color:var(--t3);font-size:10px;line-height:1.8}

/* ══ 底部：左侧币对列表 + 右侧ticker ══ */
#bottom{height:26px;background:var(--bg1);border-top:1px solid var(--bd);
  display:flex;align-items:center;flex-shrink:0;overflow:hidden}
.pair-mini-list{display:flex;align-items:center;padding:0 8px;gap:12px;overflow:hidden;
  border-right:1px solid var(--bd);height:100%;flex-shrink:0;width:340px}
.pi-mini{display:flex;gap:4px;align-items:center;cursor:pointer;white-space:nowrap;flex-shrink:0}
.pi-mini:hover .pm-sym{color:var(--t1)}
.pm-sym{font-size:10px;color:var(--t2);font-weight:600}
.pm-p{font-size:10px;font-variant-numeric:tabular-nums;font-weight:700}
.pm-c{font-size:9px;font-weight:700;padding:0 3px;border-radius:2px}
.pmu{color:var(--g);background:rgba(14,203,129,.1)}.pmd{color:var(--r);background:rgba(246,70,93,.1)}
#ticker-scroll{flex:1;display:flex;align-items:center;padding:0 8px;gap:14px;overflow:hidden}
.tbi{display:flex;gap:4px;align-items:center;white-space:nowrap;cursor:pointer;flex-shrink:0}
.tb-s{font-size:10px;color:var(--t2);font-weight:600}
.tb-p{font-size:10px;font-variant-numeric:tabular-nums;font-weight:700}
.tb-c{font-size:9px;font-weight:700;padding:0 3px;border-radius:2px}
.tbu{color:var(--g);background:rgba(14,203,129,.1)}.tbd{color:var(--r);background:rgba(246,70,93,.1)}

/* 左侧搜索面板（内嵌在col-trades上方） */
.left-top{padding:5px 7px;border-bottom:1px solid var(--bd);flex-shrink:0}
.left-top input{width:100%;background:var(--bg2);border:1px solid var(--bd);border-radius:3px;
  color:var(--t1);padding:3px 7px;font-size:11px;outline:none}
.left-top input:focus{border-color:var(--y)}
.left-tabs{display:flex;border-bottom:1px solid var(--bd);flex-shrink:0}
.ltab{flex:1;text-align:center;padding:5px 0;font-size:10px;font-weight:600;color:var(--t2);
  cursor:pointer;border-bottom:2px solid transparent}
.ltab.act{color:var(--y);border-bottom-color:var(--y)}
.pl-hdr{display:flex;justify-content:space-between;padding:2px 7px;
  font-size:9px;color:var(--t3);border-bottom:1px solid var(--bd);flex-shrink:0}
.pair-list{flex:1;overflow-y:auto}
.pair-list::-webkit-scrollbar{width:2px}
.pair-list::-webkit-scrollbar-thumb{background:var(--bd)}
.pi{display:flex;flex-direction:column;padding:4px 7px;cursor:pointer;
  border-bottom:1px solid rgba(43,49,57,.4);transition:background .1s}
.pi:hover,.pi.act{background:var(--bg2)}
.pi-r1{display:flex;justify-content:space-between}
.pi-sym{font-size:11px;font-weight:700}
.pi-p{font-size:11px;font-weight:700;font-variant-numeric:tabular-nums}
.pi-r2{display:flex;justify-content:space-between;margin-top:1px}
.pi-sub{font-size:9px;color:var(--t2)}.pi-c{font-size:10px;font-weight:700}
.cup{color:var(--g)}.cdn{color:var(--r)}.cfl{color:var(--t2)}
.pi-bars{display:flex;gap:1px;margin-top:2px}
.pib{flex:1;height:2px;background:var(--bg0);border-radius:1px;overflow:hidden}
.pbf{height:100%;border-radius:1px;transition:width .5s}
.pf-g{background:var(--g)}.pf-r{background:var(--r)}.pf-b{background:var(--b)}

/* copy tip */
.ctip{position:fixed;bottom:36px;left:50%;transform:translateX(-50%);
  background:rgba(14,203,129,.92);color:#000;font-size:11px;font-weight:700;
  padding:5px 14px;border-radius:5px;opacity:0;transition:opacity .2s;pointer-events:none;z-index:200}
.ctip.show{opacity:1}
</style>
</head>
<body>

<!-- ══ 顶部导航 ══ -->
<div id="nav">
  <div class="logo">BB-<em>Market</em></div>
  <div class="ndiv"></div>
  <div class="nav-sym">
    <span class="nav-sym-name" id="nav-sym">--/USDT</span>
    <span class="nav-price cup" id="nav-price">--</span>
    <span class="nav-chg nup" id="nav-chg">--</span>
  </div>
  <div class="nav-stats">
    <div class="ns"><div class="ns-l">24h涨跌</div><div class="ns-v" id="nv-chg">--</div></div>
    <div class="ns"><div class="ns-l">24h最高</div><div class="ns-v cup" id="nv-hi">--</div></div>
    <div class="ns"><div class="ns-l">24h最低</div><div class="ns-v cdn" id="nv-lo">--</div></div>
    <div class="ns"><div class="ns-l">成交额(USDT)</div><div class="ns-v" id="nv-vol">--</div></div>
    <div class="ns"><div class="ns-l">价差</div><div class="ns-v" id="nv-sp">--</div></div>
    <div class="ns"><div class="ns-l">CVD</div><div class="ns-v" id="nv-cvd">--</div></div>
    <div class="ns"><div class="ns-l">拉盘分</div><div class="ns-v fy" id="nv-ps">--</div></div>
  </div>
  <div class="nav-r">
    <div class="wdot" id="wdot"></div><span id="wlbl">连接中</span>
    <div class="ndiv"></div>
    <span id="htime"></span>
    <div class="ndiv"></div>
    <span>监控<b id="nc" style="color:var(--y);margin-left:3px">--</b></span>
    <span>活跃<b id="ns2" style="color:var(--g);margin-left:3px">--</b></span>
  </div>
</div>

<!-- ══ K线周期行 ══ -->
<div id="ktabs">
  <div class="kt act" data-iv="1">1m</div>
  <div class="kt" data-iv="3">3m</div>
  <div class="kt" data-iv="5">5m</div>
  <div class="kt" data-iv="15">15m</div>
  <div class="kt" data-iv="30">30m</div>
  <div class="ktd"></div>
  <div class="kt" data-iv="60">1h</div>
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
    <span class="co"><span class="co-l">开 </span><span id="ci-o">--</span></span>
    <span class="co"><span class="co-l">高 </span><span id="ci-h" style="color:var(--g)">--</span></span>
    <span class="co"><span class="co-l">低 </span><span id="ci-l2" style="color:var(--r)">--</span></span>
    <span class="co"><span class="co-l">收 </span><span id="ci-c">--</span></span>
    <span class="co"><span class="co-l">量 </span><span id="ci-v" style="color:var(--y)">--</span></span>
    <span class="co"><span class="co-l">买占 </span><span id="ci-tbr" style="color:var(--b)">--%</span></span>
  </div>
</div>

<!-- ══ 主体 5列 ══ -->
<div id="app">

  <!-- Col1：币对列表 + 最新成交 -->
  <div class="panel" style="grid-column:1;flex-direction:column">
    <div class="left-tabs">
      <div class="ltab act" onclick="ltab(0,this)">全部</div>
      <div class="ltab" onclick="ltab(1,this)">信号</div>
      <div class="ltab" onclick="ltab(2,this)">🐋</div>
    </div>
    <div class="left-top"><input type="text" placeholder="搜索..." id="srch" oninput="filterP(this.value)"></div>
    <div class="pl-hdr"><span>币对</span><span>价格/涨跌</span></div>
    <div class="pair-list" id="pair-list"></div>
  </div>

  <!-- Col2：订单簿 -->
  <div class="panel" style="grid-column:2">
    <div class="ph"><span class="ph-ttl">订单簿</span><span class="ph-sub">深度</span></div>
    <div class="ob-col"><span>价格(USDT)</span><span>数量</span><span>累计</span></div>
    <div id="ob-asks" class="ob-asks"></div>
    <div class="ob-mid">
      <span class="ob-mid-p" id="ob-mid">--</span>
      <span style="font-size:10px;color:var(--t2)">当前价</span>
      <span class="ob-bps" id="ob-bps">--</span>
    </div>
    <div id="ob-bids" class="ob-bids"></div>
    <div class="ob-ratio"><div class="or-b" id="or-b" style="width:50%"></div><div class="or-s"></div></div>
    <div class="ob-ratio-txt">
      <span id="or-bt" style="color:var(--g)">买 50%</span>
      <span id="or-st" style="color:var(--r)">卖 50%</span>
    </div>
  </div>

  <!-- Col3：图表 + 交易 + 委托记录 -->
  <div id="col-main">
    <div id="tv-area">
      <div id="tv-widget"><div class="tv-loading">⏳ TradingView 加载中...</div></div>
    </div>

    <!-- 现货交易表单 -->
    <div id="trade-area">
      <div class="ta-tabs">
        <div class="tatab act">现货</div>
        <div class="tatab">全仓</div>
        <div class="tatab">逐仓</div>
        <div class="tatab">网格</div>
        <span style="margin-left:auto;font-size:10px;color:var(--t2);align-self:center">% 手续费等级</span>
      </div>
      <div class="ta-types">
        <div class="ttype act" onclick="setType(0,this)">限价</div>
        <div class="ttype" onclick="setType(1,this)">市价</div>
        <div class="ttype" onclick="setType(2,this)">限价止盈止损</div>
      </div>
      <div class="ta-form">
        <!-- 买入侧 -->
        <div class="ta-side">
          <div class="ta-avail">
            <span style="color:var(--t2)">可用</span>
            <span style="color:var(--t1)"><span id="avail-buy">3,921.63</span> USDT</span>
          </div>
          <div>
            <div class="ta-label">价格</div>
            <div class="ta-input-row">
              <input type="number" id="buy-price" placeholder="0.00" step="any">
              <span>USDT</span>
              <span class="bbo-btn" onclick="setBBO('buy')">BBO</span>
            </div>
          </div>
          <div>
            <div class="ta-label">数量</div>
            <div class="ta-input-row">
              <input type="number" id="buy-qty" placeholder="0">
              <span id="buy-unit">--</span>
            </div>
          </div>
          <div class="ta-slider"><input type="range" min="0" max="100" value="0" id="buy-pct" oninput="setBuyPct(this.value)"></div>
          <div class="ta-stopsl"><input type="checkbox" id="buy-sl"><label for="buy-sl">止盈/止损</label></div>
          <div class="ta-info">
            <span style="color:var(--t3)">成交额</span>
            <span style="color:var(--t2)"><span id="buy-total">0</span> USDT &nbsp; 最少5</span>
          </div>
          <div class="ta-info" style="margin-top:2px">
            <span style="color:var(--t3)">预估手续费</span>
            <span style="color:var(--t3)">-- USDT</span>
          </div>
          <button class="ta-btn tb-buy" onclick="doTrade('buy')" id="btn-buy">买入 --</button>
        </div>
        <!-- 卖出侧 -->
        <div class="ta-side">
          <div class="ta-avail">
            <span style="color:var(--t2)">可用</span>
            <span style="color:var(--t1)">0 <span id="sell-unit2">--</span></span>
          </div>
          <div>
            <div class="ta-label">价格</div>
            <div class="ta-input-row">
              <input type="number" id="sell-price" placeholder="0.00" step="any">
              <span>USDT</span>
              <span class="bbo-btn" onclick="setBBO('sell')">BBO</span>
            </div>
          </div>
          <div>
            <div class="ta-label">数量</div>
            <div class="ta-input-row">
              <input type="number" id="sell-qty" placeholder="0">
              <span id="sell-unit">--</span>
            </div>
          </div>
          <div class="ta-slider"><input type="range" min="0" max="100" value="0" id="sell-pct"></div>
          <div class="ta-stopsl"><input type="checkbox" id="sell-sl"><label for="sell-sl">止盈/止损</label></div>
          <div class="ta-info">
            <span style="color:var(--t3)">成交额</span>
            <span style="color:var(--t2)"><span id="sell-total">0</span> USDT &nbsp; 最少5</span>
          </div>
          <div class="ta-info" style="margin-top:2px">
            <span style="color:var(--t3)">预估手续费</span>
            <span style="color:var(--t3)">-- USDT</span>
          </div>
          <button class="ta-btn tb-sell" onclick="doTrade('sell')" id="btn-sell">卖出 --</button>
        </div>
      </div>
    </div>

    <!-- 最新成交记录（在交易表单下方） -->
    <div style="flex:0 0 auto;border-top:1px solid var(--bd);display:flex;flex-direction:column">
      <div class="ph" style="background:var(--bg1)"><span class="ph-ttl">最新成交</span><span class="ph-sub" id="tr-cnt">--</span></div>
      <div class="tr-col" style="background:var(--bg0)"><span>价格(USDT)</span><span>数量</span><span>时间</span></div>
      <div style="height:80px;overflow-y:auto" id="tr-list"></div>
    </div>

    <!-- 委托记录区 -->
    <div id="orders-area">
      <div class="oa-tabs">
        <div class="oatab act" onclick="oaTab(0,this)">当前委托(0)</div>
        <div class="oatab" onclick="oaTab(1,this)">历史委托</div>
        <div class="oatab" onclick="oaTab(2,this)">历史成交</div>
        <div class="oatab" onclick="oaTab(3,this)">持有币种</div>
        <div class="oatab" onclick="oaTab(4,this)">机器人</div>
        <button class="oa-cancel-all" id="cancel-all-btn" onclick="cancelAll()">全撤</button>
      </div>
      <!-- 表头 -->
      <div class="oa-hdr" id="oa-hdr">
        <span class="oa-col" style="flex:1.2">日期</span>
        <span class="oa-col" style="flex:1">交易对</span>
        <span class="oa-col" style="flex:.8">类型</span>
        <span class="oa-col" style="flex:.6">方向</span>
        <span class="oa-col" style="flex:1">价格</span>
        <span class="oa-col" style="flex:1">数量</span>
        <span class="oa-col" style="flex:1.2">单笔冰山单</span>
        <span class="oa-col" style="flex:.8">完成度</span>
        <span class="oa-col" style="flex:1">金额</span>
        <span class="oa-col" style="flex:1.2">触发条件</span>
        <span class="oa-col" style="flex:.6">SOR</span>
        <span class="oa-col" style="flex:.8">止盈/止损</span>
      </div>
      <div class="oa-list" id="oa-list">
        <div class="oa-empty">暂无当前委托。</div>
      </div>
    </div>
  </div>

  <!-- Col4：分析（展开，无需点击） -->
  <div id="col-analysis">
    <div class="ph" style="height:32px"><span class="ph-ttl" style="font-size:12px">📊 分析</span></div>
    <div class="ca-scroll">
      <div class="ca-price">
        <div class="cap-r1">
          <span class="cap-sym" id="rd-sym">--</span>
          <span class="cap-p" id="rd-p">--</span>
          <span class="cap-c" id="rd-c">--</span>
        </div>
        <div class="cap-btns">
          <button class="cbtn ccp" id="rbcp" onclick="copySym()">📋 复制</button>
          <button class="cbtn cbn" onclick="openBN()">🔗 币安交易</button>
        </div>
        <div class="cap-stats">
          <div class="cst"><span class="cst-l">买一 </span><span class="cst-v cup" id="rd-bid">--</span></div>
          <div class="cst"><span class="cst-l">卖一 </span><span class="cst-v cdn" id="rd-ask">--</span></div>
          <div class="cst"><span class="cst-l">24h涨跌 </span><span class="cst-v" id="rd-chg">--</span></div>
          <div class="cst"><span class="cst-l">24h量 </span><span class="cst-v" id="rd-vol">--</span></div>
          <div class="cst"><span class="cst-l">24h高 </span><span class="cst-v cup" id="rd-hi">--</span></div>
          <div class="cst"><span class="cst-l">24h低 </span><span class="cst-v cdn" id="rd-lo">--</span></div>
          <div class="cst"><span class="cst-l">拉盘分 </span><span class="cst-v fy" id="rd-ps">--</span></div>
          <div class="cst"><span class="cst-l">砸盘分 </span><span class="cst-v cdn" id="rd-ds">--</span></div>
        </div>
      </div>
      <div class="ca-cvd">
        <div class="cvd-hdr"><span class="cvd-ttl">CVD 累计成交量差</span><span class="cvd-v" id="cvd-v">--</span></div>
        <canvas id="cvd-c"></canvas>
      </div>
      <div class="ca-fac">
        <div class="caf-ttl">信号因子解读</div>
        <div id="rf-list"></div>
      </div>
      <div class="ca-bt-hdr"><span>近期大单</span><span class="ca-bt-cnt" id="bt-cnt">0</span></div>
      <div id="bt-list"></div>
    </div>
  </div>

  <!-- Col5：信号+预警（展开，左右并排） -->
  <div id="col-alerts">
    <div class="ra-header">
      <div class="ra-sec-hdr"><span>📡 实时信号</span><span class="ra-cnt" id="sig-cnt">0</span></div>
      <div class="ra-sec-hdr"><span>🔔 预警通知</span><span class="ra-cnt" id="al-cnt">0</span></div>
    </div>
    <div class="ra-body">
      <div class="ra-col">
        <div class="ra-list" id="sig-list"><div class="empty-p">📡 等待信号...</div></div>
      </div>
      <div class="ra-col">
        <div class="ra-list" id="al-list"><div class="empty-p">🔔 等待预警...</div></div>
      </div>
    </div>
  </div>

</div>

<!-- ══ 底部：币对快选 + Ticker ══ -->
<div id="bottom">
  <div class="pair-mini-list" id="pair-mini"></div>
  <div id="ticker-scroll"></div>
</div>

<div class="ctip" id="ctip"></div>

<script type="text/javascript" src="https://s3.tradingview.com/tv.js"></script>
<script>
const S={syms:[],feed:[],sel:null,sm:{},cvdH:{},seen:new Set(),alerts:[],tr:{},orders:[]};
const A=0.25,HL=60;
let curIv='1',tvSym='',ltabMode=0,oaTabMode=0,tradeType=0,searchQ='';
const IVMAP={'1':'1m','3':'3m','5':'5m','15':'15m','30':'30m',
  '60':'1h','120':'2h','240':'4h','360':'6h','480':'8h','720':'12h',
  'D':'1d','3D':'3d','W':'1w','M':'1M'};

// ── TradingView ──────────────────────────────────────────────────
function initTV(symbol,iv){
  const s='BINANCE:'+symbol.replace('USDT','USDT');
  if(tvSym===s&&curIv===iv)return;
  tvSym=s;curIv=iv;
  const el=document.getElementById('tv-widget');
  el.innerHTML='';
  if(typeof TradingView==='undefined'){
    el.innerHTML='<div class="tv-loading">⏳ TradingView 加载中...</div>';return;
  }
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
  t.onclick=()=>{
    document.querySelectorAll('.kt[data-iv]').forEach(x=>x.classList.remove('act'));
    t.classList.add('act');curIv=t.dataset.iv;
    if(S.sel)initTV(S.sel,curIv);
    updOHLCV();
  };
});

// ── 左侧 Tab ─────────────────────────────────────────────────────
function ltab(i,el){ltabMode=i;document.querySelectorAll('.ltab').forEach(t=>t.classList.remove('act'));el.classList.add('act');renderPairList();}
function filterP(q){searchQ=q.toUpperCase();renderPairList();}

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

  if(oaTabMode===3){ // 持有币种
    hdr.innerHTML=`<span class="oa-col" style="flex:1">币种</span><span class="oa-col" style="flex:1">可用数量</span><span class="oa-col" style="flex:1">冻结数量</span><span class="oa-col" style="flex:1">折合BTC</span>`;
    list.innerHTML=S.sel?`<div class="oa-row"><span style="flex:1;font-weight:700">${S.sel.replace('USDT','')}</span><span style="flex:1;color:var(--t2)">0.00</span><span style="flex:1;color:var(--t2)">0.00</span><span style="flex:1;color:var(--t2)">--</span></div>`:'<div class="oa-empty">暂无持仓。</div>';
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
    list.innerHTML=S.orders.length?S.orders.map(o=>`
      <div class="oa-row">
        <span style="flex:1.2;color:var(--t2)">${o.time}</span>
        <span style="flex:1;font-weight:700">${o.sym}</span>
        <span style="flex:.8;color:var(--t2)">限价</span>
        <span style="flex:.6;color:${o.side==='买'?'var(--g)':'var(--r)'}">●${o.side}</span>
        <span style="flex:1;font-variant-numeric:tabular-nums">${fP(o.price)}</span>
        <span style="flex:1;font-variant-numeric:tabular-nums">${fN(o.qty)}</span>
        <span style="flex:1.2;color:var(--t2)">--</span>
        <span style="flex:.8;color:var(--y)">${o.filled}%</span>
        <span style="flex:1;font-variant-numeric:tabular-nums">${fN(o.price*o.qty)}</span>
        <span style="flex:1.2;color:var(--t2)">--</span>
        <span style="flex:.6;color:var(--t2)">--</span>
        <span style="flex:.8;color:var(--t2)">--</span>
      </div>`).join('')
    :'<div class="oa-empty">暂无当前委托。</div>';
  } else if(oaTabMode===1){
    list.innerHTML='<div class="oa-empty">暂无历史委托。</div>';
  } else if(oaTabMode===2){
    list.innerHTML='<div class="oa-empty">暂无历史成交。</div>';
  }
}

// ── 交易类型切换 ─────────────────────────────────────────────────
function setType(i,el){tradeType=i;document.querySelectorAll('.ttype').forEach(t=>t.classList.remove('act'));el.classList.add('act');}

// ── BBO 填价 ────────────────────────────────────────────────────
function setBBO(side){
  if(!S.sel)return;
  const s=S.syms.find(x=>x.symbol===S.sel);if(!s)return;
  if(side==='buy')document.getElementById('buy-price').value=fP(s.bid||sv(S.sel,'mid'));
  else document.getElementById('sell-price').value=fP(s.ask||sv(S.sel,'mid'));
}

function setBuyPct(pct){
  // 根据可用余额计算数量（示意）
  const price=parseFloat(document.getElementById('buy-price').value)||sv(S.sel||'','mid')||1;
  const avail=3921.63;
  const qty=(avail*pct/100/price);
  document.getElementById('buy-qty').value=qty>0?qty.toFixed(0):'';
  const total=qty*price;
  document.getElementById('buy-total').textContent=total.toFixed(2);
}

// ── 模拟下单 ────────────────────────────────────────────────────
function doTrade(side){
  if(!S.sel)return;
  const priceId=side==='buy'?'buy-price':'sell-price';
  const qtyId=side==='buy'?'buy-qty':'sell-qty';
  const price=parseFloat(document.getElementById(priceId).value)||sv(S.sel,'mid');
  const qty=parseFloat(document.getElementById(qtyId).value)||0;
  if(!qty||qty<=0){alert('请输入有效数量');return;}
  const sym=S.sel.replace('USDT','');
  const order={time:nowT(),sym:sym+'/USDT',side:side==='buy'?'买':'卖',price,qty,filled:0,id:Date.now()};
  S.orders.unshift(order);
  document.querySelectorAll('.oatab').forEach(t=>t.classList.remove('act'));
  document.querySelectorAll('.oatab')[0].classList.add('act');
  document.querySelectorAll('.oatab')[0].textContent=`当前委托(${S.orders.length})`;
  oaTabMode=0;
  renderOrders();
  // 清空表单
  document.getElementById(priceId).value='';
  document.getElementById(qtyId).value='';
}

function cancelAll(){
  S.orders=[];
  document.querySelectorAll('.oatab')[0].textContent='当前委托(0)';
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
  S.syms=data.symbols||[];S.feed=data.feed||[];

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
  if(cur){if(!S.sel){S.sel=cur;initTV(cur,curIv);}renderDetail(cur);}
  updOHLCV();
}

// ── 币对列表 ─────────────────────────────────────────────────────
function renderPairList(){
  let list=[...S.syms].sort((a,b)=>Math.max(sv(b.symbol,'ps'),sv(b.symbol,'ds'))-Math.max(sv(a.symbol,'ps'),sv(a.symbol,'ds')));
  if(ltabMode===1)list=list.filter(s=>sv(s.symbol,'ps')>=60||sv(s.symbol,'ds')>=60);
  if(ltabMode===2)list=list.filter(s=>s.whale_entry||s.whale_exit);
  if(searchQ)list=list.filter(s=>s.symbol.includes(searchQ));
  document.getElementById('pair-list').innerHTML=list.map(s=>{
    const sym=s.symbol.replace('USDT','');
    const ps=sv(s.symbol,'ps'),ds=sv(s.symbol,'ds'),obi=sv(s.symbol,'obi');
    const mid=sv(s.symbol,'mid'),chg=s.change_24h_pct||0;
    const cc=chg>.05?'cup':chg<-.05?'cdn':'cfl';
    return `<div class="pi${S.sel===s.symbol?' act':''}" onclick="selSym('${s.symbol}')">
      <div class="pi-r1"><span class="pi-sym">${sym}<span style="font-size:9px;color:var(--t3)">/U</span></span>
      <span class="pi-p" style="color:${chg>=0?'var(--g)':'var(--r)'}">${fP(mid)}</span></div>
      <div class="pi-r2"><span class="pi-sub">${s.pump_signal?'🚀':s.whale_entry?'🐋':''}P:${Math.round(ps)}</span>
      <span class="pi-c ${cc}">${chg>=0?'▲':'▼'}${Math.abs(chg).toFixed(2)}%</span></div>
      <div class="pi-bars">
        <div class="pib"><div class="pbf pf-g" style="width:${Math.min(100,ps)}%"></div></div>
        <div class="pib"><div class="pbf pf-r" style="width:${Math.min(100,ds)}%"></div></div>
        <div class="pib"><div class="pbf pf-b" style="width:${Math.min(100,Math.abs(obi)*2)}%"></div></div>
      </div>
    </div>`;
  }).join('');
}

// ── 底部币对快选（当前选中附近5个） ─────────────────────────────
function renderPairMini(){
  const list=[...S.syms].sort((a,b)=>Math.max(sv(b.symbol,'ps'),sv(b.symbol,'ds'))-Math.max(sv(a.symbol,'ps'),sv(a.symbol,'ds'))).slice(0,5);
  document.getElementById('pair-mini').innerHTML=list.map(s=>{
    const chg=s.change_24h_pct||0,cls=chg>=0?'pmu':'pmd';
    return `<div class="pi-mini" onclick="selSym('${s.symbol}')">
      <span class="pm-sym">${s.symbol.replace('USDT','/U')}</span>
      <span class="pm-p" style="color:${chg>=0?'var(--g)':'var(--r)'}">${fP(sv(s.symbol,'mid'))}</span>
      <span class="pm-c ${cls}">${chg>=0?'+':''}${chg.toFixed(2)}%</span>
    </div>`;
  }).join('');
}

// ── Ticker ───────────────────────────────────────────────────────
function renderTicker(){
  const top=[...S.syms].sort((a,b)=>Math.abs(b.change_24h_pct||0)-Math.abs(a.change_24h_pct||0)).slice(0,20);
  document.getElementById('ticker-scroll').innerHTML=top.map(s=>{
    const chg=s.change_24h_pct||0,cls=chg>=0?'tbu':'tbd';
    return `<div class="tbi" onclick="selSym('${s.symbol}')">
      <span class="tb-s">${s.symbol.replace('USDT','/U')}</span>
      <span class="tb-p" style="color:${chg>=0?'var(--g)':'var(--r)'}">${fP(sv(s.symbol,'mid'))}</span>
      <span class="tb-c ${cls}">${chg>=0?'+':''}${chg.toFixed(2)}%</span>
    </div>`;
  }).join('');
}

// ── 选中币种 ─────────────────────────────────────────────────────
function selSym(sym){S.sel=sym;initTV(sym,curIv);renderDetail(sym);renderPairList();}

// ── 详情 ─────────────────────────────────────────────────────────
function renderDetail(sym){
  const s=S.syms.find(x=>x.symbol===sym);if(!s)return;
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
  e('nv-vol',fN(s.quote_vol_24h||0));e('nv-sp',s.spread_bps.toFixed(1)+'bps');
  e('nv-ps',Math.round(ps));es('nv-cvd',fN(cvd),null,cvd>=0?'var(--g)':'var(--r)');

  // 交易表单更新
  document.getElementById('buy-unit').textContent=symShort;
  document.getElementById('sell-unit').textContent=symShort;
  document.getElementById('sell-unit2').textContent=symShort;
  document.getElementById('btn-buy').textContent='买入 '+symShort;
  document.getElementById('btn-sell').textContent='卖出 '+symShort;
  // 自动填入当前价
  if(!document.getElementById('buy-price').value)
    document.getElementById('buy-price').value=fP(mid);
  if(!document.getElementById('sell-price').value)
    document.getElementById('sell-price').value=fP(mid);

  // 买卖比例条
  const totalBid=s.total_bid_volume||0,totalAsk=s.total_ask_volume||0;
  const tot=totalBid+totalAsk||1;
  const bPct=(totalBid/tot*100).toFixed(0);
  document.getElementById('or-b').style.width=bPct+'%';
  document.getElementById('or-bt').textContent='买 '+bPct+'%';
  document.getElementById('or-st').textContent='卖 '+(100-+bPct)+'%';

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
  es('ob-mid',fP(mid),null,gc);e('ob-bps',s.spread_bps.toFixed(1)+' bps');

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
    {n:'主动买入',v:`${tbr.toFixed(1)}%`,bw:tbr,bc:tbr>60?'gf':tbr<40?'rf2':'yf',vc:tbr>60?'fg':tbr<40?'fr':'fy',tip:tbr>70?'Taker强势买入':tbr>60?'偏多':tbr<30?'强势卖出':'偏空'},
    {n:'CVD累计',v:fN(cvd),bw:Math.min(100,Math.abs(cvd)/500),bc:cvd>=0?'gf':'rf2',vc:cvd>0?'fg':'fr',tip:cvd>50000?'大量净流入':cvd>0?'净买入':cvd<-50000?'大量净流出':'净卖出'},
    {n:'OFI方向',v:fN(ofi),bw:Math.min(100,Math.abs(ofi)/100),bc:ofi>0?'gf':'rf2',vc:ofi>3000?'fg':ofi<-3000?'fr':'fn',tip:ofi>5000?'主力买单拉升意图':ofi>2000?'买方增加挂单':ofi<-5000?'主力卖单砸盘意图':'挂单平衡'},
    {n:'价差',v:`${s.spread_bps.toFixed(1)}bps`,bw:Math.min(100,s.spread_bps*3),bc:s.spread_bps<20?'gf':'yf',vc:s.spread_bps<10?'fg':s.spread_bps<30?'fy':'fn',tip:s.spread_bps<10?'流动性极好':s.spread_bps<20?'正常':'偏差'},
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
        desc:`评分${Math.round(ps)} OBI${sv(s.symbol,'obi').toFixed(1)}% 买占${sv(s.symbol,'tbr').toFixed(0)}%`,fresh:false});
    if(ds>=70&&!sigs.find(x=>x.full===s.symbol&&x.type==='dump'))
      sigs.unshift({time:'实时',sym:s.symbol.replace('USDT',''),full:s.symbol,type:'dump',score:Math.round(ds),
        desc:`评分${Math.round(ds)} OBI${sv(s.symbol,'obi').toFixed(1)}%`,fresh:false});
  });
  e('sig-cnt',sigs.length);
  const lbl={pump:'🚀 拉盘',dump:'📉 砸盘',whale:'🐋 鲸鱼',anomaly:'⚠️ 异动',cvd:'📊 CVD'};
  document.getElementById('sig-list').innerHTML=sigs.slice(0,20).map((s,i)=>`
    <div class="scard ${s.type}" onclick="selSym('${s.full}')">
      ${i===0?'<div class="sc-new">NEW</div>':''}
      <div class="sc-h"><span class="sc-sym">${s.sym}</span><span class="sc-t">${s.time}</span></div>
      <div class="sc-tag">${lbl[s.type]||s.type}</div>
      <div class="sc-desc">${s.desc}</div>
      ${s.score!=null?`<div class="sc-bar" style="width:${Math.min(100,s.score)}%;background:${s.type==='pump'?'var(--g)':'var(--r)'}"></div>`:''}
    </div>`).join('')||'<div class="empty-p">📡 等待信号<br>评分≥70 触发</div>';
}

// ── 预警 ─────────────────────────────────────────────────────────
function checkAlerts(){
  S.syms.forEach(s=>{
    const ps=sv(s.symbol,'ps'),ds=sv(s.symbol,'ds'),sym=s.symbol.replace('USDT',''),t=nowT();
    if(ps>=75){const id=`p-${s.symbol}-${Math.floor(Date.now()/30000)}`;
      if(!S.seen.has(id)){S.seen.add(id);S.alerts.unshift({type:'pump',sym,full:s.symbol,
        tag:'🚀 拉盘',time:t,desc:`评分${Math.round(ps)}/100 OBI${sv(s.symbol,'obi').toFixed(1)}%`,fresh:true});}}
    if(ds>=75){const id=`d-${s.symbol}-${Math.floor(Date.now()/30000)}`;
      if(!S.seen.has(id)){S.seen.add(id);S.alerts.unshift({type:'dump',sym,full:s.symbol,
        tag:'📉 砸盘',time:t,desc:`评分${Math.round(ds)}/100 OBI${sv(s.symbol,'obi').toFixed(1)}%`,fresh:true});}}
    if(s.whale_entry){const id=`w-${s.symbol}-${Math.floor(Date.now()/60000)}`;
      if(!S.seen.has(id)){S.seen.add(id);S.alerts.unshift({type:'whale',sym,full:s.symbol,
        tag:'🐋 鲸鱼进场',time:t,desc:`大单${s.max_bid_ratio.toFixed(1)}% CVD${fN(sv(s.symbol,'cvd'))}`,fresh:true});}}
    const cvd=sv(s.symbol,'cvd');
    if(Math.abs(cvd)>50000){const id=`c-${s.symbol}-${Math.floor(Date.now()/120000)}`;
      if(!S.seen.has(id)){S.seen.add(id);S.alerts.unshift({type:'cvd',sym,full:s.symbol,
        tag:cvd>0?'📈 CVD买入':'📉 CVD卖出',time:t,desc:`CVD${fN(cvd)} 买占${sv(s.symbol,'tbr').toFixed(0)}%`,fresh:true});}}
  });
  if(S.alerts.length>50)S.alerts=S.alerts.slice(0,50);
  e('al-cnt',S.alerts.length);
  document.getElementById('al-list').innerHTML=S.alerts.map((a,i)=>`
    <div class="scard ${a.type}" onclick="selSym('${a.full}')">
      ${a.fresh&&i===0?'<div class="sc-new">NEW</div>':''}
      <span class="sc-x" onclick="event.stopPropagation();S.alerts.splice(${i},1);checkAlerts()">✕</span>
      <div class="sc-h"><span class="sc-sym">${a.sym}</span><span class="sc-t">${a.time}</span></div>
      <div class="sc-tag">${a.tag}</div>
      <div class="sc-desc">${a.desc}</div>
    </div>`).join('')||'<div class="empty-p">🔔 等待预警<br>评分≥75 触发</div>';
}

// ── 工具 ─────────────────────────────────────────────────────────
function e(id,txt){const el=document.getElementById(id);if(el)el.textContent=txt;}
function es(id,txt,cls,color){const el=document.getElementById(id);if(!el)return;el.textContent=txt;if(cls)el.className=cls;if(color)el.style.color=color;}
function fP(p){if(!p)return '--';return p>=1000?p.toFixed(1):p>=10?p.toFixed(2):p>=1?p.toFixed(3):p>=.1?p.toFixed(4):p.toFixed(6);}
function fN(n){const v=+n;return Math.abs(v)>=1e9?(v/1e9).toFixed(1)+'B':Math.abs(v)>=1e6?(v/1e6).toFixed(1)+'M':Math.abs(v)>=1e3?(v/1e3).toFixed(1)+'K':v.toFixed(0);}
function nowT(){return new Date().toLocaleTimeString('zh-CN',{hour12:false,hour:'2-digit',minute:'2-digit',second:'2-digit'});}
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
  ws.onmessage=ev=>{try{render(JSON.parse(ev.data));}catch(err){console.warn(err);}};
  ws.onerror=()=>{document.getElementById('wdot').className='wdot';e('wlbl','连接异常');};
  ws.onclose=()=>{document.getElementById('wdot').className='wdot';e('wlbl','重连中...');setTimeout(connect,2000);};
}
window.addEventListener('DOMContentLoaded',()=>{
  renderOrders();
  fetch('/api/state').then(r=>r.json()).then(d=>render(d)).catch(()=>{});
  connect();
});
</script>
</body>
</html>
"#;