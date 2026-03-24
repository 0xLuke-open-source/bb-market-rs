function filterP(q){searchQ=q.toUpperCase();renderPairList();}

function saveViewPrefs(){
  try{
    localStorage.setItem(VIEW_PREF_KEY,JSON.stringify({
      marketSort,signalWindow,marketQuickFilter,
      favorites:S.favorites||[],
      selectedSymbol:S.sel||null,
      selectedInterval:curIv||'60'
    }));
  }catch(_){}
}

function loadViewPrefs(){
  try{
    const raw=localStorage.getItem(VIEW_PREF_KEY);
    if(!raw)return;
    const pref=JSON.parse(raw);
    if(pref.marketSort)marketSort=pref.marketSort;
    if(pref.signalWindow)signalWindow=pref.signalWindow;
    if(pref.marketQuickFilter)marketQuickFilter=pref.marketQuickFilter;
    if(Array.isArray(pref.favorites))S.favorites=pref.favorites;
    if(pref.selectedSymbol)S.sel=pref.selectedSymbol;
    if(pref.selectedInterval)curIv=pref.selectedInterval;
  }catch(_){}
}

function syncViewControls(){
  document.querySelectorAll('.sortbtn').forEach(btn=>btn.classList.toggle('act',btn.dataset.sort===marketSort));
  document.querySelectorAll('.filterbtn').forEach(btn=>btn.classList.toggle('act',btn.dataset.window===signalWindow));
  document.querySelectorAll('.quickbtn').forEach(btn=>btn.classList.toggle('act',btn.dataset.filter===marketQuickFilter));
  document.querySelectorAll('.kt[data-iv]').forEach(btn=>btn.classList.toggle('act',btn.dataset.iv===curIv));
}

function setMarketSort(mode,el){
  marketSort=mode;
  document.querySelectorAll('.sortbtn').forEach(btn=>btn.classList.toggle('act',btn===el));
  saveViewPrefs();
  renderPairList();
}

function setSignalWindow(windowKey,el){
  signalWindow=windowKey;
  document.querySelectorAll('.filterbtn').forEach(btn=>btn.classList.toggle('act',btn===el));
  saveViewPrefs();
  renderSigs();
  checkAlerts();
}

function setMarketQuickFilter(mode,el){
  marketQuickFilter=mode;
  document.querySelectorAll('.quickbtn').forEach(btn=>btn.classList.toggle('act',btn===el));
  saveViewPrefs();
  renderPairList();
}

function isFavorite(sym){
  return (S.favorites||[]).includes(sym);
}

function toggleFavorite(sym=S.sel){
  if(!sym)return;
  if(isFavorite(sym)){
    S.favorites=(S.favorites||[]).filter(x=>x!==sym);
  }else{
    S.favorites=[...(S.favorites||[]),sym];
  }
  saveViewPrefs();
  renderPairList();
  renderPairMini();
  renderTicker();
  refreshFavoriteButton();
}

function refreshFavoriteButton(){
  const btn=document.getElementById('rbfav');
  if(!btn||!S.sel)return;
  const act=isFavorite(S.sel);
  btn.textContent=act?'★ 已加入自选':'☆ 加入自选';
  btn.classList.toggle('act',act);
}

function mergeSymbols(next){
  const prevMap=new Map((S.syms||[]).map(s=>[s.symbol,s]));
  return (next||[]).map(symbol=>{
    const prev=prevMap.get(symbol.symbol);
    if(!prev)return symbol;
    if((!symbol.klines||Object.keys(symbol.klines).length===0)&&prev.klines)symbol.klines=prev.klines;
    if((!symbol.current_kline||Object.keys(symbol.current_kline).length===0)&&prev.current_kline)symbol.current_kline=prev.current_kline;
    if((!symbol.big_trades||symbol.big_trades.length===0)&&prev.big_trades)symbol.big_trades=prev.big_trades;
    if((!symbol.recent_trades||symbol.recent_trades.length===0)&&prev.recent_trades)symbol.recent_trades=prev.recent_trades;
    return symbol;
  });
}

function getSymbolState(sym){
  if(!sym)return null;
  const current=(S.syms||[]).find(x=>x.symbol===sym);
  if(current)return current;
  if(S.selectedCache&&S.selectedCache.symbol===sym)return S.selectedCache;
  return null;
}

function signalTypeFromTag(tag=''){
  const t=String(tag||'').toLowerCase();
  if(t.includes('上涨'))return 'pump';
  if(t.includes('下跌'))return 'dump';
  if(t.includes('大户'))return 'whale';
  if(t.includes('异常'))return 'anomaly';
  if(t.includes('主动买卖量差')||t.includes('主动买入')||t.includes('主动卖出'))return 'cvd';
  return '';
}

function parseFeedAgeMinutes(timeText){
  if(timeText==='实时')return 0;
  if(!timeText||!timeText.includes(':'))return Number.MAX_SAFE_INTEGER;
  const now=new Date();
  const [h,m,s]=(timeText||'').split(':').map(v=>parseInt(v,10)||0);
  const dt=new Date(now.getFullYear(),now.getMonth(),now.getDate(),h,m,s||0,0);
  let diff=(now.getTime()-dt.getTime())/60000;
  if(diff<0)diff+=24*60;
  return diff;
}

function withinWindow(timeText){
  if(signalWindow==='all')return true;
  const age=parseFeedAgeMinutes(timeText);
  if(signalWindow==='5m')return age<=5;
  if(signalWindow==='15m')return age<=15;
  if(signalWindow==='60m')return age<=60;
  return true;
}

function upsertSymbolDetail(detail){
  if(!detail||!detail.symbol)return;
  const idx=(S.syms||[]).findIndex(s=>s.symbol===detail.symbol);
  if(idx>=0)S.syms[idx]={...S.syms[idx],...detail};
  else S.syms.push(detail);
  if(S.sel===detail.symbol){
    S.selectedCache={...(S.selectedCache||{}),...detail};
  }
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
