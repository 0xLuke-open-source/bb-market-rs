// ── 工具 ─────────────────────────────────────────────────────────
function e(id,txt){const el=document.getElementById(id);if(el)el.textContent=txt;}
function es(id,txt,cls,color){const el=document.getElementById(id);if(!el)return;el.textContent=txt;if(cls)el.className=cls;if(color)el.style.color=color;}
function dismissAlert(time,full,type){
  S.alerts=S.alerts.filter(a=>!(a.time===time&&a.full===full&&a.type===type));
  checkAlerts();
}
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
    summary:s.status_summary,
    level:s.watch_level,
    reason:s.signal_reason,
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
  loadViewPrefs();
  syncViewControls();
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
