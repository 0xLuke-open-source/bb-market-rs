// src/web/server.rs
//
// Axum Web 服务器 + 完整终版 Dashboard（实时数据版）
// ─ GET  /          → 内嵌 HTML，单文件零部署
// ─ GET  /api/state → 全量 JSON 快照（首屏）
// ─ GET  /ws        → WebSocket 推送，每 500ms 全量更新

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
    println!("🌐 Dashboard: http://127.0.0.1:{}", port);

    let listener = tokio::net::TcpListener::bind(&addr).await?;
    axum::serve(listener, app).await?;
    Ok(())
}

async fn serve_dashboard() -> Html<&'static str> {
    Html(DASHBOARD_HTML)
}

async fn api_full_state(State(state): State<SharedDashboardState>) -> impl IntoResponse {
    let s = state.read().await;
    Json(s.to_full_snapshot())
}

async fn ws_handler(ws: WebSocketUpgrade, State(state): State<SharedDashboardState>) -> impl IntoResponse {
    ws.on_upgrade(move |socket| ws_loop(socket, state))
}

async fn ws_loop(mut socket: WebSocket, state: SharedDashboardState) {
    let mut tick = interval(Duration::from_millis(500));
    loop {
        tick.tick().await;
        let json = {
            let s = state.read().await;
            match serde_json::to_string(&s.to_full_snapshot()) {
                Ok(j) => j,
                Err(_) => continue,
            }
        };
        if socket.send(Message::Text(json.into())).await.is_err() {
            break;
        }
    }
}

// ─────────────────────────────────────────────────────────────────
// 完整 HTML（终版样式 + 实时 WebSocket 数据绑定）
// ─────────────────────────────────────────────────────────────────
const DASHBOARD_HTML: &str = r#"<!DOCTYPE html>
<html lang="zh-CN" class="dark">
<head>
<meta charset="UTF-8">
<meta name="viewport" content="width=device-width, initial-scale=1.0">
<title>QUANT-INTEL PRO · 极速高频微结构终端</title>
<script src="https://cdn.tailwindcss.com"></script>
<script src="https://cdn.jsdelivr.net/npm/echarts@5.5.0/dist/echarts.min.js"></script>
<link href="https://fonts.googleapis.com/css2?family=JetBrains+Mono:wght@400;700;900&family=Inter:wght@400;600;900&display=swap" rel="stylesheet">
<script>
tailwind.config = {
  darkMode: 'class',
  theme: { extend: { colors: { gray: { 850:'#111827', 900:'#0b0e14', 950:'#05070a' } },
    fontFamily: { mono:['"JetBrains Mono"','monospace'], sans:['Inter','system-ui','sans-serif'] } } }
}
</script>
<style>
*{margin:0;padding:0;box-sizing:border-box}
body{background:#05070a;color:#e5e7eb;height:100vh;overflow:hidden;font-family:'Inter',sans-serif;padding:.75rem;display:flex;flex-direction:column}
.card{background:#0f172a;border:1px solid #1e293b;border-radius:.5rem;transition:all .2s cubic-bezier(.4,0,.2,1);position:relative;overflow:hidden}
.card:hover{border-color:#334155;box-shadow:0 0 20px rgba(0,0,0,.4)}
.glow-pump{box-shadow:inset 0 0 12px rgba(16,185,129,.1);border-left:4px solid #10b981;animation:pulse-green 2s infinite}
.glow-dump{box-shadow:inset 0 0 12px rgba(239,68,68,.1);border-left:4px solid #ef4444;animation:pulse-red 2s infinite}
@keyframes pulse-green{0%,100%{border-left-color:#10b981}50%{border-left-color:#059669}}
@keyframes pulse-red{0%,100%{border-left-color:#ef4444}50%{border-left-color:#dc2626}}
.metric-value{font-family:'JetBrains Mono';letter-spacing:-1px}
.no-scrollbar::-webkit-scrollbar{display:none}
::-webkit-scrollbar{width:4px;height:4px}
::-webkit-scrollbar-track{background:#1e2532;border-radius:10px}
::-webkit-scrollbar-thumb{background:#3e4a5f;border-radius:10px}
.badge-pump{background:rgba(16,185,129,.2);color:#10b981;border:1px solid rgba(16,185,129,.3)}
.badge-dump{background:rgba(239,68,68,.2);color:#ef4444;border:1px solid rgba(239,68,68,.3)}
.badge-sideways{background:rgba(156,163,175,.2);color:#9ca3af;border:1px solid rgba(156,163,175,.3)}
</style>
</head>
<body class="flex flex-col space-y-3">

<!-- ── Header ── -->
<header class="flex justify-between items-end px-2 flex-shrink-0">
  <div class="flex flex-col">
    <h1 class="text-2xl font-black italic tracking-tighter bg-gradient-to-r from-blue-400 via-indigo-400 to-emerald-400 bg-clip-text text-transparent">
      QUANT-INTEL <span class="text-white">PRO</span>
    </h1>
    <div class="flex items-center gap-3 text-[10px] text-gray-500 font-mono mt-1">
      <span class="flex items-center gap-1"><span id="ws-dot" class="w-1.5 h-1.5 rounded-full bg-yellow-500 animate-ping"></span> <span id="ws-label">CONNECTING</span></span>
      <span>|</span><span>RUST_BACKEND</span><span>|</span>
      <span>SYMBOLS: <span id="hdr-count" class="text-gray-300">--</span></span>
      <span>|</span><span>UPDATES: <span id="hdr-updates" class="text-gray-300">--</span></span>
      <span>|</span><span>UP: <span id="hdr-uptime" class="text-gray-300">--</span></span>
    </div>
  </div>
  <div class="flex gap-6 items-center">
    <div class="text-right">
      <span class="text-[10px] text-gray-500 uppercase font-bold tracking-widest">Top Pump Score</span>
      <div class="flex items-center gap-2">
        <span id="hdr-top-pump" class="text-2xl font-black text-emerald-400 metric-value">--</span>
        <div class="w-12 h-1.5 bg-gray-800 rounded-full overflow-hidden">
          <div id="hdr-pump-bar" class="bg-emerald-500 h-full" style="width:0%"></div>
        </div>
      </div>
    </div>
    <div class="h-8 w-px bg-gray-800"></div>
    <div class="text-right">
      <span class="text-[10px] text-gray-500 uppercase font-bold tracking-widest">Avg OBI</span>
      <div class="flex items-center gap-2">
        <span id="hdr-avg-obi" class="text-2xl font-black metric-value">--</span>
      </div>
    </div>
  </div>
</header>

<!-- ── 市场大脑三卡 ── -->
<section class="grid grid-cols-3 gap-3 flex-shrink-0">
  <!-- 综合强度 -->
  <div id="brain-card" class="card p-4 relative overflow-hidden glow-pump">
    <div class="text-[10px] text-gray-500 uppercase tracking-widest font-bold">市场评分 · 综合强度</div>
    <div class="flex items-end justify-between mt-1">
      <div>
        <span id="brain-score" class="text-4xl font-black text-emerald-400 metric-value">--</span>
        <span class="text-gray-500 ml-1 text-sm">/100</span>
      </div>
      <span id="brain-badge" class="badge-pump px-3 py-1 text-xs rounded-full font-bold">等待数据</span>
    </div>
    <div id="brain-tags" class="flex gap-3 mt-2 text-[0.65rem] text-gray-400">
      <span>初始化中</span>
    </div>
  </div>

  <!-- 概率分布 -->
  <div class="card p-4">
    <div class="text-[10px] text-gray-500 mb-2 flex items-center font-bold tracking-widest">📊 市场概率 · 综合信号</div>
    <div class="flex justify-between text-xs font-medium mb-1">
      <span class="text-emerald-400">🚀 拉盘 <span id="prob-pump" class="text-base font-black ml-1">--%</span></span>
      <span class="text-red-400">📉 砸盘 <span id="prob-dump" class="text-base font-black ml-1">--%</span></span>
      <span class="text-gray-400">⚖️ 中性 <span id="prob-neutral" class="text-base font-black ml-1">--%</span></span>
    </div>
    <div class="w-full h-3 bg-gray-800 rounded-full flex overflow-hidden mt-1">
      <div id="bar-pump" class="bg-emerald-500 h-3 transition-all duration-500" style="width:0%"></div>
      <div id="bar-dump" class="bg-red-500 h-3 transition-all duration-500" style="width:0%"></div>
      <div id="bar-neutral" class="bg-gray-400 h-3 transition-all duration-500" style="width:100%"></div>
    </div>
    <div class="grid grid-cols-3 gap-2 mt-3 text-[0.6rem] text-gray-400">
      <div>OBI: <span id="prob-obi" class="text-white">--</span></div>
      <div>鲸鱼: <span id="prob-whale" class="text-white">--</span> 只</div>
      <div>异动: <span id="prob-anom" class="text-white">--</span> 次</div>
    </div>
  </div>

  <!-- 风险状态 -->
  <div class="card p-4">
    <div class="text-[10px] text-gray-500 mb-2 font-bold tracking-widest">⚠️ 市场风险 · 信号质量</div>
    <div class="flex items-center justify-between">
      <span id="risk-label" class="font-black text-lg text-yellow-400">--</span>
      <span id="risk-badge" class="text-xs bg-yellow-500/20 text-yellow-400 px-2 py-0.5 rounded-full">--</span>
    </div>
    <div class="mt-2 space-y-1 text-xs">
      <div class="flex justify-between">
        <span class="text-gray-400">最高拉盘分</span>
        <span id="risk-pump" class="text-emerald-400">--</span>
      </div>
      <div class="flex justify-between">
        <span class="text-gray-400">最高砸盘分</span>
        <span id="risk-dump" class="text-red-400">--</span>
      </div>
    </div>
    <div class="w-full bg-gray-700 h-1.5 rounded-full mt-2">
      <div id="risk-bar" class="bg-yellow-400 h-1.5 rounded-full transition-all duration-500" style="width:0%"></div>
    </div>
    <p id="risk-tip" class="text-[0.55rem] text-gray-500 mt-2">等待数据...</p>
  </div>
</section>

<!-- ── 快速状态条（动态渲染，最多6个） ── -->
<section id="ticker-row" class="grid grid-cols-6 gap-2 flex-shrink-0">
  <!-- 由 JS 动态填充 -->
</section>

<!-- ── 主图表区 ── -->
<section class="grid grid-cols-12 gap-3 flex-1 min-h-0">
  <!-- 雷达图 -->
  <div class="card col-span-3 p-3 flex flex-col h-full">
    <h3 class="text-[9px] font-bold text-gray-500 uppercase tracking-widest mb-2">因子共振 · 多空雷达</h3>
    <div id="radarChart" class="flex-1 w-full min-h-0"></div>
    <div id="radar-legend" class="flex justify-around text-[0.5rem] text-gray-500 mt-1"></div>
  </div>

  <!-- 微价格 + 信号矩阵 -->
  <div class="card col-span-6 p-3 flex flex-col relative h-full">
    <div class="absolute top-3 left-4 z-10">
      <h2 class="text-base font-black text-white flex items-center gap-2">
        <span id="chart-sym">--</span>
        <span id="chart-badge" class="text-[0.55rem] bg-emerald-500/10 text-emerald-400 px-2 py-0.5 rounded">--</span>
      </h2>
      <div class="text-xl font-black metric-value mt-0.5">
        <span id="chart-price">--</span>
        <span class="text-xs text-gray-500 ml-2 font-normal">OBI: <span id="chart-obi">--</span></span>
      </div>
    </div>
    <div id="lineChart" class="flex-1 w-full"></div>
  </div>

  <!-- 多空 + AI 快照 -->
  <div class="card col-span-3 p-3 flex flex-col h-full">
    <h3 class="text-[9px] font-bold text-gray-500 uppercase tracking-widest mb-1 text-center">Taker Orderflow Bias</h3>
    <div id="gaugeChart" class="h-20 w-full"></div>
    <div class="grid grid-cols-2 gap-2 mt-2">
      <div class="bg-gray-900 p-2 rounded border-b-2 border-emerald-500">
        <span class="text-[8px] text-gray-500 block uppercase font-bold">买盘总量</span>
        <span id="side-bid" class="text-sm font-black text-emerald-400">--</span>
      </div>
      <div class="bg-gray-900 p-2 rounded border-b-2 border-red-500">
        <span class="text-[8px] text-gray-500 block uppercase font-bold">卖盘总量</span>
        <span id="side-ask" class="text-sm font-black text-red-400">--</span>
      </div>
    </div>
    <div class="mt-3 p-2 bg-emerald-500/5 border border-emerald-500/20 rounded-lg">
      <div class="flex justify-between items-center mb-1">
        <span class="text-[9px] font-black text-emerald-400 uppercase">Pump Prediction</span>
        <span id="pump-pred-level" class="text-[8px] font-black text-emerald-400">--</span>
      </div>
      <p id="pump-pred-text" class="text-[9px] text-gray-400 italic leading-relaxed">等待数据...</p>
    </div>
    <div class="mt-2 flex justify-between text-[0.55rem] text-gray-500">
      <span>多/空: <span id="side-ratio" class="text-white">--</span></span>
      <span>OFI: <span id="side-ofi" class="text-white">--</span></span>
    </div>
  </div>
</section>

<!-- ── AI 解读 + 信号确认 ── -->
<section class="grid grid-cols-2 gap-3 flex-shrink-0">
  <div class="card p-4">
    <div class="flex items-center gap-2 mb-2">
      <span class="text-lg">🧠</span>
      <span class="text-xs font-bold text-gray-300 uppercase tracking-wider">AI 市场解读 · 实时人话</span>
      <span class="text-[0.45rem] bg-blue-500/20 text-blue-300 px-2 py-0.5 rounded-full">500ms 更新</span>
    </div>
    <div class="text-xs text-gray-200 leading-relaxed space-y-1">
      <p id="ai-summary">等待数据连接...</p>
      <div id="ai-bullets" class="text-gray-400 text-xs"></div>
      <div id="ai-conclusion" class="mt-2 p-2 bg-gray-800/60 rounded border-l-4 border-emerald-400 hidden">
        👉 <span id="ai-conclusion-text" class="text-emerald-400 font-black text-xs"></span>
      </div>
    </div>
  </div>

  <div class="card p-4">
    <div class="flex items-center gap-2 mb-2">
      <span class="text-lg">📡</span>
      <span class="text-xs font-bold text-gray-300 uppercase tracking-wider">信号确认 · 因子一致性</span>
    </div>
    <div id="signal-factors" class="space-y-2 text-xs"></div>
    <div id="signal-conclusion" class="mt-3 p-2 bg-gray-800/60 rounded text-center hidden">
      <span id="signal-conclusion-text" class="font-bold text-xs"></span>
    </div>
    <div class="grid grid-cols-3 gap-1 mt-2 text-[0.45rem] text-gray-500 border-t border-gray-700 pt-2">
      <span>🚀 拉盘: OFI+ &amp; 主动买 &amp; 鲸鱼流入</span>
      <span>📉 砸盘: OFI- &amp; 主动卖 &amp; 鲸鱼流出</span>
      <span>⚖️ 横盘: 低波动 &amp; 平衡</span>
    </div>
  </div>
</section>

<!-- ── 异动日志 ── -->
<section class="card w-full flex flex-col flex-shrink-0" style="height:18%">
  <div class="bg-gray-900/80 px-4 py-2 border-b border-gray-800 flex justify-between items-center">
    <div class="flex items-center gap-4">
      <h2 class="text-[9px] font-black text-gray-400 uppercase tracking-widest flex items-center gap-2">
        <span class="w-2 h-2 bg-red-500 rounded-full animate-pulse"></span> Anomaly Intelligence Log
      </h2>
      <div class="h-3 w-px bg-gray-700"></div>
      <span id="log-pump-count" class="text-[8px] text-emerald-400 font-mono">PUMP: 0</span>
      <span id="log-dump-count" class="text-[8px] text-red-400 font-mono">DUMP: 0</span>
    </div>
    <div class="text-[9px] font-mono text-gray-500">SYSTEM_TIME: <span id="clock" class="text-gray-300">00:00:00.000</span></div>
  </div>
  <div class="flex-1 overflow-y-auto no-scrollbar">
    <table class="w-full text-[9px] font-mono">
      <thead class="bg-gray-950 text-gray-500 sticky top-0 uppercase text-[8px]">
        <tr>
          <th class="px-3 py-1.5 text-left">TS</th>
          <th class="px-3 py-1.5 text-left">Symbol</th>
          <th class="px-3 py-1.5 text-left">Type</th>
          <th class="px-3 py-1.5 text-left">Exp.</th>
          <th class="px-3 py-1.5 text-center">Score</th>
          <th class="px-3 py-1.5 text-left">Message</th>
        </tr>
      </thead>
      <tbody id="logBody" class="divide-y divide-gray-900"></tbody>
    </table>
  </div>
</section>

<div class="flex justify-between items-center text-[0.4rem] text-gray-700 mt-0.5 flex-shrink-0">
  <span>HFT 决策终端 v4.0 · Rust后端 · WebSocket实时推送</span>
  <span>🚀 拉盘  📉 砸盘  ⚖️ 横盘  🐋 鲸鱼  ⚠️ 异动</span>
</div>

<script>
// ── 图表实例 ──────────────────────────────────────────────────────
const radar = echarts.init(document.getElementById('radarChart'));
const line  = echarts.init(document.getElementById('lineChart'));
const gauge = echarts.init(document.getElementById('gaugeChart'));

// 初始化雷达
radar.setOption({
  backgroundColor:'transparent',
  tooltip:{backgroundColor:'#1f2a3a',borderColor:'#3e4c64',textStyle:{color:'#eee',fontSize:9}},
  radar:{
    indicator:[
      {name:'OFI动能',max:100},{name:'拉盘概率',max:100},{name:'吸筹评分',max:100},
      {name:'流动性',max:100},{name:'鲸鱼活跃',max:100},{name:'风险指数',max:100}
    ],
    center:['50%','50%'],radius:'65%',
    axisName:{color:'#6b7a8f',fontSize:8,fontWeight:'bold'},
    splitLine:{lineStyle:{color:'rgba(255,255,255,.05)'}},
    splitArea:{show:false}
  },
  series:[{type:'radar',data:[]}]
});

// 初始化折线
line.setOption({
  backgroundColor:'transparent',
  grid:{top:'30%',bottom:'12%',left:'5%',right:'4%'},
  tooltip:{trigger:'axis',backgroundColor:'#1f2a3a',borderColor:'#3e4c64',textStyle:{color:'#eee',fontSize:9}},
  legend:{bottom:0,textStyle:{color:'#9ca3af',fontSize:8},itemWidth:8},
  xAxis:{type:'category',data:[],axisLine:{lineStyle:{color:'#1e293b'}},axisLabel:{fontSize:8,color:'#9ca3af'}},
  yAxis:[
    {type:'value',scale:true,splitLine:{lineStyle:{color:'#111827'}},axisLabel:{fontSize:8,color:'#9ca3af'}},
    {type:'value',splitLine:{show:false},axisLabel:{fontSize:8,color:'#9ca3af'}}
  ],
  series:[
    {name:'Microprice',type:'line',smooth:true,symbol:'none',data:[],
     lineStyle:{width:2,color:'#3b82f6'},
     areaStyle:{color:new echarts.graphic.LinearGradient(0,0,0,1,
       [{offset:0,color:'rgba(59,130,246,.2)'},{offset:1,color:'transparent'}])}},
    {name:'OFI',type:'bar',yAxisIndex:1,data:[],
     itemStyle:{color:(p)=>p.value>=0?'#10b981':'#ef4444'},barWidth:'40%'}
  ]
});

// 初始化仪表盘
gauge.setOption({
  series:[{type:'gauge',startAngle:180,endAngle:0,min:-100,max:100,
    progress:{show:true,width:8,itemStyle:{color:'#10b981'}},
    axisLine:{lineStyle:{width:8,color:[[1,'#1e293b']]}},
    pointer:{show:false},axisTick:{show:false},splitLine:{show:false},
    axisLabel:{show:false},detail:{show:false},data:[{value:0}]}]
});

// ── 历史数据（折线图用）────────────────────────────────────────────
const history = { times:[], prices:[], ofis:[], maxLen:30 };

function pushHistory(price, ofi) {
  const t = new Date().toLocaleTimeString('zh-CN',{hour12:false,hour:'2-digit',minute:'2-digit',second:'2-digit'});
  history.times.push(t); history.prices.push(+price.toFixed(4)); history.ofis.push(+ofi.toFixed(0));
  if (history.times.length > history.maxLen) {
    history.times.shift(); history.prices.shift(); history.ofis.shift();
  }
}

// ── 主渲染函数 ────────────────────────────────────────────────────
function render(data) {
  const syms = data.symbols || [];
  const feed = data.feed || [];

  // ── Header ─────────────────────────────────────────────────────
  document.getElementById('hdr-count').textContent   = syms.length;
  document.getElementById('hdr-updates').textContent = fmtNum(data.total_updates);
  document.getElementById('hdr-uptime').textContent  = fmtUptime(data.uptime_secs);

  const topPump = syms.reduce((a,b)=>(b.pump_score||0)>(a.pump_score||0)?b:a, {pump_score:0});
  document.getElementById('hdr-top-pump').textContent = topPump.symbol
    ? `${topPump.symbol.replace('USDT','')} ${topPump.pump_score}`
    : '--';
  document.getElementById('hdr-pump-bar').style.width = (topPump.pump_score||0) + '%';

  const avgObi = syms.length ? (syms.reduce((s,x)=>s+(x.obi||0),0)/syms.length) : 0;
  const obiEl = document.getElementById('hdr-avg-obi');
  obiEl.textContent = (avgObi>0?'+':'') + avgObi.toFixed(1) + '%';
  obiEl.className = 'text-2xl font-black metric-value ' + (avgObi>5?'text-emerald-400':avgObi<-5?'text-red-400':'text-blue-400');

  // ── 三卡：综合强度 ──────────────────────────────────────────────
  const pumpCount  = syms.filter(s=>s.pump_score>=60).length;
  const dumpCount  = syms.filter(s=>s.dump_score>=60).length;
  const whaleCount = syms.filter(s=>s.whale_entry||s.whale_exit).length;
  const totalAnom  = syms.reduce((s,x)=>s+(x.anomaly_count_1m||0),0);
  const maxPump    = syms.length ? Math.max(...syms.map(s=>s.pump_score||0)) : 0;
  const maxDump    = syms.length ? Math.max(...syms.map(s=>s.dump_score||0)) : 0;

  // 综合强度：基于 OBI、拉盘信号、鲸鱼活跃
  const brainScore = Math.min(100, Math.round(
    Math.abs(avgObi)*1.2 + pumpCount*8 + whaleCount*5 + maxPump*0.3
  ));
  const brainEl = document.getElementById('brain-score');
  brainEl.textContent = brainScore;
  const brainCard = document.getElementById('brain-card');
  const brainBadge = document.getElementById('brain-badge');
  const brainTags = document.getElementById('brain-tags');
  if (pumpCount > dumpCount) {
    brainEl.className = 'text-4xl font-black text-emerald-400 metric-value';
    brainCard.className = 'card p-4 relative overflow-hidden glow-pump';
    brainBadge.className = 'badge-pump px-3 py-1 text-xs rounded-full font-bold';
    brainBadge.textContent = pumpCount >= 3 ? '🚀 强势拉盘' : '📈 偏多';
    brainTags.innerHTML = `<span>买方主导</span><span>·</span><span>${pumpCount}只币拉升</span><span>·</span><span>鲸鱼${whaleCount>0?'活跃':'观望'}</span>`;
  } else if (dumpCount > pumpCount) {
    brainEl.className = 'text-4xl font-black text-red-400 metric-value';
    brainCard.className = 'card p-4 relative overflow-hidden glow-dump';
    brainBadge.className = 'badge-dump px-3 py-1 text-xs rounded-full font-bold';
    brainBadge.textContent = dumpCount >= 3 ? '📉 强势砸盘' : '🔻 偏空';
    brainTags.innerHTML = `<span>卖方主导</span><span>·</span><span>${dumpCount}只币下跌</span>`;
  } else {
    brainEl.className = 'text-4xl font-black text-gray-300 metric-value';
    brainCard.className = 'card p-4 relative overflow-hidden';
    brainBadge.className = 'badge-sideways px-3 py-1 text-xs rounded-full font-bold';
    brainBadge.textContent = '⚖️ 多空平衡';
    brainTags.innerHTML = `<span>震荡整理</span><span>·</span><span>等待方向</span>`;
  }

  // 概率分布
  const total = syms.length || 1;
  const pPump = Math.round(pumpCount / total * 100);
  const pDump = Math.round(dumpCount / total * 100);
  const pNeut = 100 - pPump - pDump;
  document.getElementById('prob-pump').textContent    = pPump + '%';
  document.getElementById('prob-dump').textContent    = pDump + '%';
  document.getElementById('prob-neutral').textContent = Math.max(0,pNeut) + '%';
  document.getElementById('bar-pump').style.width    = pPump + '%';
  document.getElementById('bar-dump').style.width    = pDump + '%';
  document.getElementById('bar-neutral').style.width = Math.max(0,pNeut) + '%';
  document.getElementById('prob-obi').textContent    = (avgObi>0?'+':'') + avgObi.toFixed(1) + '%';
  document.getElementById('prob-whale').textContent  = whaleCount;
  document.getElementById('prob-anom').textContent   = totalAnom;

  // 风险卡
  const riskScore = Math.min(100, Math.round(totalAnom/30 + maxDump*0.5 + whaleCount*8));
  const riskEl = document.getElementById('risk-label');
  const riskBadge = document.getElementById('risk-badge');
  const riskTip = document.getElementById('risk-tip');
  if (riskScore >= 70) {
    riskEl.textContent = '高风险'; riskEl.className = 'font-black text-lg text-red-400';
    riskBadge.textContent = '极端波动'; riskBadge.className = 'text-xs bg-red-500/20 text-red-400 px-2 py-0.5 rounded-full';
    riskTip.textContent = '市场剧烈波动，建议轻仓或观望';
  } else if (riskScore >= 40) {
    riskEl.textContent = '中等风险'; riskEl.className = 'font-black text-lg text-yellow-400';
    riskBadge.textContent = '注意控仓'; riskBadge.className = 'text-xs bg-yellow-500/20 text-yellow-400 px-2 py-0.5 rounded-full';
    riskTip.textContent = '异动较多，谨慎追单，等待确认';
  } else {
    riskEl.textContent = '低风险'; riskEl.className = 'font-black text-lg text-emerald-400';
    riskBadge.textContent = '市场平稳'; riskBadge.className = 'text-xs bg-emerald-500/20 text-emerald-400 px-2 py-0.5 rounded-full';
    riskTip.textContent = '行情平稳，可适当参与';
  }
  document.getElementById('risk-pump').textContent = maxPump;
  document.getElementById('risk-dump').textContent = maxDump;
  document.getElementById('risk-bar').style.width  = riskScore + '%';

  // ── 快速状态条 ─────────────────────────────────────────────────
  const tickerRow = document.getElementById('ticker-row');
  const top6 = [...syms].sort((a,b)=>(b.pump_score||0)-(a.pump_score||0)).slice(0,6);
  tickerRow.innerHTML = top6.map(s => {
    const sym = s.symbol.replace('USDT','');
    const chg = s.price_change_pct || 0;
    const chgStr = (chg>0?'▲ ':'▼ ') + Math.abs(chg).toFixed(2) + '%';
    const borderCls = s.pump_signal ? 'border-l-emerald-500' : s.dump_signal ? 'border-l-red-500' : 'border-l-gray-600';
    const chgCls = chg > 0 ? 'text-emerald-400' : 'text-red-400';
    const sigLabel = s.pump_signal ? '🚀拉' : s.dump_signal ? '🔻砸' : s.whale_entry ? '🐋鲸' : '—';
    const glowCls = s.pump_score >= 60 ? 'glow-pump' : s.dump_score >= 60 ? 'glow-dump' : '';
    return `<div class="card p-2 flex flex-col items-center border-l-4 ${borderCls} ${glowCls}">
      <div class="w-full flex justify-between text-[8px] font-bold text-gray-500"><span>${sym}</span><span class="${chgCls}">${sigLabel}</span></div>
      <span class="text-base font-black text-white metric-value">${fmtPrice(s.mid||0)}</span>
      <div class="text-[9px] font-bold ${chgCls} mt-0.5">${chgStr} <span class="text-gray-500">P:${s.pump_score}</span></div>
    </div>`;
  }).join('');

  // ── 选取最强币做详情 ───────────────────────────────────────────
  const topSym = top6[0];
  if (topSym) {
    pushHistory(topSym.mid||0, topSym.ofi||0);

    document.getElementById('chart-sym').textContent = topSym.symbol.replace('USDT','') + '/USDT';
    document.getElementById('chart-price').textContent = fmtPrice(topSym.mid||0);
    const obiVal = (topSym.obi||0);
    const obiChartEl = document.getElementById('chart-obi');
    obiChartEl.textContent = (obiVal>0?'+':'') + obiVal.toFixed(1) + '%';
    obiChartEl.style.color = obiVal>10?'#10b981':obiVal<-10?'#ef4444':'#9ca3af';

    const badge = document.getElementById('chart-badge');
    if (topSym.sentiment === 'StrongBullish' || topSym.sentiment === 'Bullish') {
      badge.textContent = 'BULLISH STRUCTURE'; badge.className = 'text-[.55rem] bg-emerald-500/10 text-emerald-400 px-2 py-0.5 rounded';
    } else if (topSym.sentiment === 'StrongBearish' || topSym.sentiment === 'Bearish') {
      badge.textContent = 'BEARISH STRUCTURE'; badge.className = 'text-[.55rem] bg-red-500/10 text-red-400 px-2 py-0.5 rounded';
    } else {
      badge.textContent = 'NEUTRAL'; badge.className = 'text-[.55rem] bg-gray-500/10 text-gray-400 px-2 py-0.5 rounded';
    }

    line.setOption({
      xAxis:{data:[...history.times]},
      series:[{data:[...history.prices]},{data:[...history.ofis]}]
    });

    // 仪表盘：用 OBI 驱动
    gauge.setOption({
      series:[{data:[{value: Math.min(100,Math.max(-100,(topSym.obi||0)*2))}],
        progress:{itemStyle:{color:(topSym.obi||0)>0?'#10b981':'#ef4444'}}}]
    });

    document.getElementById('side-bid').textContent = fmtNum(topSym.total_bid_volume||0);
    document.getElementById('side-ask').textContent = fmtNum(topSym.total_ask_volume||0);
    const ratio = topSym.total_ask_volume ? (topSym.total_bid_volume/topSym.total_ask_volume).toFixed(2) : '--';
    document.getElementById('side-ratio').textContent = ratio;
    document.getElementById('side-ofi').textContent = fmtNum(topSym.ofi||0);

    // Pump Prediction
    const pp = topSym.pump_probability || 0;
    const ppLevel = document.getElementById('pump-pred-level');
    const ppText  = document.getElementById('pump-pred-text');
    ppLevel.textContent = pp >= 70 ? 'HIGH' : pp >= 40 ? 'MED' : 'LOW';
    ppLevel.style.color = pp >= 70 ? '#10b981' : pp >= 40 ? '#f59e0b' : '#9ca3af';
    ppText.textContent = `${topSym.symbol.replace('USDT','')} 拉升概率 ${pp}%。` +
      (topSym.pump_signal ? ' 检测到有效拉盘信号，' : '') +
      (topSym.whale_entry ? ' 鲸鱼进场确认，' : '') +
      `OBI ${(topSym.obi||0).toFixed(1)}%，建议${pp>=60?'关注做多机会':'等待方向明确'}。`;
  }

  // ── 雷达图（最多4个币种） ──────────────────────────────────────
  const colors = ['#10b981','#ef4444','#3b82f6','#f59e0b'];
  const radarSyms = top6.slice(0,4);
  radar.setOption({
    series:[{type:'radar',
      data: radarSyms.map((s,i)=>({
        name: s.symbol.replace('USDT',''),
        value:[
          Math.min(100, 50 + (s.ofi||0)/100),
          s.pump_probability || 0,
          Math.min(100, 50 + (s.obi||0)),
          Math.min(100, 100 - (s.spread_bps||0)*2),
          s.whale_entry ? 80 : 20,
          Math.min(100, s.anomaly_max_severity || 0)
        ],
        lineStyle:{color:colors[i],width:1.5},
        itemStyle:{color:colors[i]},
        areaStyle:{color:colors[i],opacity:.12}
      }))
    }]
  });
  document.getElementById('radar-legend').innerHTML = radarSyms.map((s,i)=>
    `<span class="flex items-center gap-1"><span class="w-1.5 h-1.5 rounded-full" style="background:${colors[i]}"></span>${s.symbol.replace('USDT','')}</span>`
  ).join('');

  // ── AI 解读 ─────────────────────────────────────────────────────
  const aiSummary = document.getElementById('ai-summary');
  const aiBullets = document.getElementById('ai-bullets');
  const aiConc    = document.getElementById('ai-conclusion');
  const aiConcTxt = document.getElementById('ai-conclusion-text');

  if (syms.length === 0) {
    aiSummary.textContent = '等待数据连接...';
  } else {
    const domSentiment = pumpCount > dumpCount ? '强势拉盘阶段' : dumpCount > pumpCount ? '下行压力阶段' : '震荡整理阶段';
    const domColor = pumpCount > dumpCount ? 'text-emerald-400' : dumpCount > pumpCount ? 'text-red-400' : 'text-gray-300';
    aiSummary.innerHTML = `当前市场处于 <span class="${domColor} font-black">${domSentiment}</span>，OBI均值 ${(avgObi>0?'+':'')+avgObi.toFixed(1)}%。`;

    const bullets = [];
    if (whaleCount > 0) bullets.push(`• 检测到 ${whaleCount} 个品种鲸鱼活动`);
    if (pumpCount > 0)  bullets.push(`• ${pumpCount} 个品种触发拉盘信号（评分≥60）`);
    if (dumpCount > 0)  bullets.push(`• ${dumpCount} 个品种砸盘压力较大`);
    if (totalAnom > 50) bullets.push(`• 全市场异动事件 ${totalAnom} 次，注意风险`);
    aiBullets.innerHTML = bullets.join('<br>');

    aiConc.classList.remove('hidden');
    aiConc.className = 'mt-2 p-2 bg-gray-800/60 rounded border-l-4 ' + (pumpCount>dumpCount ? 'border-emerald-400' : dumpCount>pumpCount ? 'border-red-400' : 'border-gray-500');
    aiConcTxt.textContent = pumpCount > dumpCount
      ? `短线偏多概率较高 (${pPump}%)，关注拉盘品种，控制仓位`
      : dumpCount > pumpCount
        ? `市场偏空 (${pDump}%)，谨慎做多，等待企稳`
        : '多空平衡，建议观望等待方向突破';
  }

  // ── 信号确认 ────────────────────────────────────────────────────
  const sfEl = document.getElementById('signal-factors');
  const scEl = document.getElementById('signal-conclusion');
  const sctEl = document.getElementById('signal-conclusion-text');
  if (topSym) {
    const rows = [
      ['OFI 动能',  (topSym.ofi||0)>3000 ? `✔ 强 (${fmtNum(topSym.ofi)})` : `↔ 弱 (${fmtNum(topSym.ofi)})`, (topSym.ofi||0)>3000 ? 'text-emerald-400' : 'text-gray-400', '买方吃单'],
      ['OBI 失衡',  (topSym.obi||0)>10 ? `✔ 买超 ${(topSym.obi||0).toFixed(1)}%` : `↔ ${(topSym.obi||0).toFixed(1)}%`, (topSym.obi||0)>10 ? 'text-emerald-400' : 'text-gray-400', 'OBI'],
      ['鲸鱼活动',  topSym.whale_entry ? '✔ 鲸鱼进场' : topSym.whale_exit ? '⬇ 鲸鱼离场' : '○ 无', topSym.whale_entry ? 'text-emerald-400' : topSym.whale_exit ? 'text-red-400' : 'text-gray-500', '大单追踪'],
      ['拉盘评分',  `${topSym.pump_score||0}/100`, (topSym.pump_score||0)>=60 ? 'text-amber-400 font-black' : 'text-gray-400', '综合评分'],
      ['砸盘压力',  `${topSym.dump_score||0}/100`, (topSym.dump_score||0)>=60 ? 'text-red-400 font-black' : 'text-gray-400', '反向风险'],
    ];
    sfEl.innerHTML = rows.map(([k,v,c,note])=>
      `<div class="flex justify-between items-center"><span class="text-gray-400">${k}</span><span class="${c} font-medium">${v}</span><span class="text-[.5rem] text-gray-500">${note}</span></div>`
    ).join('');

    scEl.classList.remove('hidden');
    const bullish = (topSym.pump_score||0) >= 60 && (topSym.obi||0) > 5;
    sctEl.className = 'font-bold text-xs ' + (bullish ? 'text-emerald-400' : 'text-yellow-400');
    sctEl.textContent = bullish
      ? `✅ 信号一致：${topSym.symbol.replace('USDT','')} 看涨有效 (强度 ${topSym.pump_score})`
      : `⚠️ 信号混合：等待确认，谨慎操作`;
  }

  // ── 异动日志 ────────────────────────────────────────────────────
  let pumpFeedCount = 0, dumpFeedCount = 0;
  const logBody = document.getElementById('logBody');
  const rows = feed.slice(0,20).map(f => {
    const typeLabel = f.type === 'pump' ? '🚀 拉盘' : f.type === 'dump' ? '📉 砸盘' : f.type === 'whale' ? '🐋 鲸鱼' : '⚠️ 异动';
    const typeColor = f.type === 'pump' ? 'text-emerald-400' : f.type === 'dump' ? 'text-red-400' : f.type === 'whale' ? 'text-blue-400' : 'text-yellow-400';
    const typeBg   = f.type === 'pump' ? 'bg-emerald-500/10 text-emerald-400' : f.type === 'dump' ? 'bg-red-500/10 text-red-400' : f.type === 'whale' ? 'bg-blue-500/10 text-blue-400' : 'bg-yellow-500/10 text-yellow-400';
    if (f.type === 'pump') pumpFeedCount++;
    if (f.type === 'dump') dumpFeedCount++;
    return `<tr class="hover:bg-white/5">
      <td class="px-3 py-1.5 text-gray-500">${f.time}</td>
      <td class="px-3 py-1.5 font-black text-white">${f.symbol.replace('USDT','')}</td>
      <td class="px-3 py-1.5"><span class="${typeBg} px-1 rounded text-[8px]">${f.type.toUpperCase()}</span></td>
      <td class="px-3 py-1.5 font-bold ${typeColor}">${typeLabel}</td>
      <td class="px-3 py-1.5 text-center text-yellow-500">${f.score != null ? f.score : '-'}</td>
      <td class="px-3 py-1.5 text-gray-400 italic">${f.desc}</td>
    </tr>`;
  });
  logBody.innerHTML = rows.join('') || '<tr><td colspan="6" class="px-3 py-3 text-gray-600 text-center">等待信号...</td></tr>';
  document.getElementById('log-pump-count').textContent = `PUMP: ${String(pumpFeedCount).padStart(2,'0')}`;
  document.getElementById('log-dump-count').textContent = `DUMP: ${String(dumpFeedCount).padStart(2,'0')}`;
}

// ── 工具函数 ──────────────────────────────────────────────────────
function fmtNum(n) {
  const v = +n;
  if (Math.abs(v) >= 1000000) return (v/1000000).toFixed(1)+'M';
  if (Math.abs(v) >= 1000)    return (v/1000).toFixed(1)+'K';
  return v.toFixed(0);
}
function fmtUptime(s) {
  const h=Math.floor(s/3600), m=Math.floor((s%3600)/60), sec=s%60;
  return `${String(h).padStart(2,'0')}:${String(m).padStart(2,'0')}:${String(sec).padStart(2,'0')}`;
}
function fmtPrice(p) {
  return p >= 100 ? p.toFixed(2) : p >= 1 ? p.toFixed(3) : p.toFixed(4);
}

// ── 时钟 ─────────────────────────────────────────────────────────
setInterval(() => {
  document.getElementById('clock').innerText = new Date().toISOString().split('T')[1].slice(0,12);
}, 50);

// ── WebSocket ─────────────────────────────────────────────────────
function connectWS() {
  const wsUrl = `ws://${location.host}/ws`;
  const ws = new WebSocket(wsUrl);
  const dot   = document.getElementById('ws-dot');
  const label = document.getElementById('ws-label');

  ws.onopen = () => {
    dot.className   = 'w-1.5 h-1.5 rounded-full bg-emerald-500 animate-ping';
    label.textContent = 'ENGINE_ACTIVE';
  };
  ws.onmessage = (e) => {
    try { render(JSON.parse(e.data)); } catch(err) { console.warn(err); }
  };
  ws.onerror = () => {
    dot.className   = 'w-1.5 h-1.5 rounded-full bg-yellow-500';
    label.textContent = 'RECONNECTING';
  };
  ws.onclose = () => {
    dot.className   = 'w-1.5 h-1.5 rounded-full bg-red-500';
    label.textContent = 'DISCONNECTED';
    setTimeout(connectWS, 2000);
  };
}

// 首屏快照 + 建立 WS
window.addEventListener('DOMContentLoaded', () => {
  window.addEventListener('resize', () => { radar.resize(); line.resize(); gauge.resize(); });
  fetch('/api/state').then(r=>r.json()).then(d=>render(d)).catch(()=>{});
  connectWS();
});
</script>
</body>
</html>"#;