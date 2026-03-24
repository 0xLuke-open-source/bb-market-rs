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
function normalizeSymbolKeyword(keyword=''){
  return String(keyword||'').toUpperCase().replace(/\s+/g,'').replace(/[\/_-]/g,'');
}
function syncMarketSearchInput(keyword=''){
  const input=document.getElementById('srch');
  if(input)input.value=keyword;
}
function findSymbolByKeyword(keyword=''){
  const normalized=normalizeSymbolKeyword(keyword);
  if(!normalized)return null;
  const list=S.syms||[];
  const exact=list.find(item=>item.symbol===normalized || item.symbol===`${normalized}USDT`);
  if(exact)return exact.symbol;
  const startsWith=list.find(item=>item.symbol.startsWith(normalized));
  if(startsWith)return startsWith.symbol;
  const baseMatch=list.find(item=>item.symbol.replace(/USDT$/,'')===normalized);
  if(baseMatch)return baseMatch.symbol;
  const fuzzy=list.find(item=>item.symbol.includes(normalized));
  return fuzzy?fuzzy.symbol:null;
}
async function searchTopSymbol(){
  const input=document.getElementById('site-search-input');
  const raw=input?.value||'';
  const keyword=normalizeSymbolKeyword(raw);
  if(!keyword)return;
  const matched=findSymbolByKeyword(keyword);
  if(typeof switchSitePage==='function')switchSitePage('home');
  if(typeof filterP==='function'){
    filterP(keyword);
    syncMarketSearchInput(keyword);
  }
  if(matched && typeof selSym==='function'){
    await selSym(matched);
    if(input)input.value=matched.replace('USDT','');
    return;
  }
  const tip=document.getElementById('ctip');
  if(tip){
    tip.textContent=`未找到 ${keyword}，已按关键词筛选`;
    tip.classList.add('show');
    setTimeout(()=>tip.classList.remove('show'),1800);
  }
}
function updateDocumentTitle(sym='',price='--',change=null){
  const page=S.site?.page||'home';
  const pageTitles={
    home:'BB-Market',
    ai:'AI盯盘',
    vip:'VIP服务',
    ads:'广告',
    feedback:'产品反馈与建议',
    rebate:'超级返佣',
    invite:'邀请奖励',
    plaza:'广场',
    blog:'博客',
    help:'帮助中心',
    announcements:'公告',
    news:'新闻中心',
    community:'社区',
    agreement:'服务协议',
    privacy:'隐私说明',
    about:'关于我们'
  };
  if(page!=='home'){
    document.title=`${pageTitles[page]||'BB-Market'} - BB-Market`;
    return;
  }
  if(!sym){
    document.title='BB-Market';
    return;
  }
  const changeText=typeof change==='number'&&!Number.isNaN(change)?` ${change>=0?'+':''}${change.toFixed(2)}%`:'';
  document.title=`${fmtSym(sym)} ${price||'--'}${changeText}`;
}
function filledPct(order){const q=+order.quantity||0;if(q<=0)return 0;return Math.round((+order.filled_qty||0)/q*100);}
function getBalance(asset){return (S.trader.balances||[]).find(b=>b.asset===asset)||{available:0,locked:0};}
function sideLabel(side){return String(side||'').toUpperCase()==='BUY'?'买':'卖';}
function sideColor(side){return String(side||'').toUpperCase()==='BUY'?'var(--g)':'var(--r)';}
function selectedDetailKey(sym){
  const s=getSymbolState(sym);if(!s)return '';
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
async function apiFetch(url,options={}){
  const res=await fetch(url,options);
  if(res.status===401){
    if(typeof handleAuthExpired==='function')handleAuthExpired();
    const err=new Error('AUTH_REQUIRED');
    err.code='AUTH_REQUIRED';
    throw err;
  }
  return res;
}
async function postJson(url,payload){
  const res=await apiFetch(url,{method:'POST',headers:{'Content-Type':'application/json'},body:JSON.stringify(payload)});
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
  try{S.trader=await apiFetch('/api/spot/state').then(r=>r.json());}catch(_){}
}
async function openReplay(){
  const atTs=prompt('输入要回放的毫秒时间戳，留空则读取最近归档事件：','');
  if(atTs===null)return;
  const q=atTs.trim()?`?at_ts=${encodeURIComponent(atTs.trim())}&limit=50`:'?limit=50';
  let res;
  try{
    res=await apiFetch('/api/spot/replay'+q).then(r=>r.json());
  }catch(err){
    if(err&&err.code==='AUTH_REQUIRED')return;
    alert('回放失败');
    return;
  }
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
window.addEventListener('DOMContentLoaded',()=>{
  const input=document.getElementById('site-search-input');
  if(input){
    input.addEventListener('keydown',ev=>{
      if(ev.key==='Enter'){
        ev.preventDefault();
        searchTopSymbol();
      }
    });
  }
});

// ── WebSocket ─────────────────────────────────────────────────────
function connect(){
  if(S.auth.ws){
    try{
      S.auth.ws.onclose=null;
      S.auth.ws.close();
    }catch(_){}
  }
  const proto=location.protocol==='https:'?'wss':'ws';
  const ws=new WebSocket(`${proto}://${location.host}/ws`);
  S.auth.ws=ws;
  ws.onopen=()=>{document.getElementById('wdot').className='wdot live';e('wlbl','实时连接');};
  ws.onmessage=ev=>{try{render(JSON.parse(ev.data));}catch(_){ }};
  ws.onerror=()=>{document.getElementById('wdot').className='wdot';e('wlbl','连接异常');};
  ws.onclose=()=>{
    if(S.auth.ws!==ws || !S.auth.appReady)return;
    document.getElementById('wdot').className='wdot';
    // WebSocket 现在对游客开放，断线后也要继续自动重连，
    // 否则公开预览模式会在一次掉线后完全停更。
    e('wlbl',S.auth.user?'重连中...':'公开预览重连中...');
    setTimeout(()=>{
      if(S.auth.ws===ws && S.auth.appReady)connect();
    },2000);
  };
}

function connectionIdleLabel(){
  return S.auth.user?'连接中':'公开预览';
}

function stopDashboardApp(){
  S.auth.appReady=false;
  if(S.auth.detailPoller){
    clearInterval(S.auth.detailPoller);
    S.auth.detailPoller=null;
  }
  if(S.auth.ws){
    try{S.auth.ws.close();}catch(_){}
    S.auth.ws=null;
  }
  document.getElementById('wdot').className='wdot';
  e('wlbl',connectionIdleLabel());
}

function startDashboardApp(){
  if(S.auth.appReady)return;
  if(!S.auth.domBound){
    loadViewPrefs();
    syncViewControls();
    document.getElementById('buy-pct').oninput=e=>setBuyPct(e.target.value);
    document.getElementById('sell-pct').oninput=e=>setSellPct(e.target.value);
    ['buy-price','buy-qty','sell-price','sell-qty'].forEach(id=>{
      const el=document.getElementById(id);
      if(el) el.addEventListener('input',()=>updateTotals());
    });
    S.auth.domBound=true;
  }
  renderOrders();
  e('wlbl',connectionIdleLabel());
  apiFetch('/api/state').then(r=>r.json()).then(d=>render(d)).catch(()=>{});
  connect();
  S.auth.detailPoller=setInterval(()=>{ if(S.sel) loadSymbolDetail(S.sel,true); },5000);
  S.auth.appReady=true;
}

window.addEventListener('DOMContentLoaded',()=>{
  if(typeof bootAuth==='function')bootAuth();
});
