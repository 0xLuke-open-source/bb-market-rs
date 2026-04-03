function filterP(q){searchQ=q.toUpperCase();renderPairList();}

function applyFavoriteSymbols(symbols,persistLocal=false){
  S.favorites=Array.isArray(symbols)?[...new Set(symbols.map(item=>String(item||'').toUpperCase()).filter(Boolean))]:[];
  if(persistLocal)saveViewPrefs();
  renderPairList();
  renderPairMini();
  renderTicker();
  refreshFavoriteButton();
}

function normalizeMarketQuickFilter(mode){
  return mode||'all';
}

function saveViewPrefs(){
  try{
    localStorage.setItem(VIEW_PREF_KEY,JSON.stringify({
      marketSort,signalWindow,marketQuickFilter,
      favorites:S.auth?.user?[]:(S.favorites||[]),
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
    if(pref.marketQuickFilter)marketQuickFilter=normalizeMarketQuickFilter(pref.marketQuickFilter);
    if(Array.isArray(pref.favorites))S.favorites=pref.favorites;
    // 刷新 / 重启后首页统一回到 BTC，不再恢复上次查看的币种。
    S.sel=null;
    window.__bbRestoredSelectedSymbol=null;
    window.__bbRestoredSelectedAt=0;
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
  marketQuickFilter=normalizeMarketQuickFilter(mode);
  document.querySelectorAll('.quickbtn').forEach(btn=>btn.classList.toggle('act',btn===el));
  saveViewPrefs();
  renderPairList();
}

function isFavorite(sym){
  return (S.favorites||[]).includes(sym);
}

async function toggleFavorite(sym=S.sel){
  if(!sym)return;
  if(!S.auth?.user){
    if(typeof openAuthModal==='function')openAuthModal('login');
    if(typeof setAuthMessage==='function')setAuthMessage('登录后才可收藏自选币种。');
    return;
  }
  try{
    let json;
    if(isFavorite(sym)){
      const res=await apiFetch(`/api/auth/favorites/${encodeURIComponent(sym)}`,{method:'DELETE'});
      json=await res.json();
    }else{
      json=await postJson(`/api/auth/favorites/${encodeURIComponent(sym)}`,{});
    }
    if(!json.ok){
      if(typeof setAuthMessage==='function')setAuthMessage(json.message||'收藏操作失败','err');
      if(!S.auth?.user && typeof openAuthModal==='function')openAuthModal('login');
      return;
    }
    applyFavoriteSymbols(json.data||[],false);
  }catch(err){
    if(err?.code==='AUTH_REQUIRED'){
      if(typeof openAuthModal==='function')openAuthModal('login');
      if(typeof setAuthMessage==='function')setAuthMessage('登录后才可收藏自选币种。');
      return;
    }
    if(typeof setAuthMessage==='function')setAuthMessage('收藏操作失败，请稍后重试。','err');
  }
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
    const keepLiveDetail=symbol.symbol===S.sel && Number(prev.__detailFreshAt) && (Date.now()-Number(prev.__detailFreshAt)<1800);
    if((!symbol.klines||Object.keys(symbol.klines).length===0)&&prev.klines)symbol.klines=prev.klines;
    if((!symbol.current_kline||Object.keys(symbol.current_kline).length===0)&&prev.current_kline)symbol.current_kline=prev.current_kline;
    if((keepLiveDetail || !symbol.big_trades || symbol.big_trades.length===0)&&prev.big_trades)symbol.big_trades=prev.big_trades;
    if((keepLiveDetail || !symbol.recent_trades || symbol.recent_trades.length===0)&&prev.recent_trades)symbol.recent_trades=prev.recent_trades;
    if((keepLiveDetail || !symbol.top_bids || symbol.top_bids.length===0)&&prev.top_bids)symbol.top_bids=prev.top_bids;
    if((keepLiveDetail || !symbol.top_asks || symbol.top_asks.length===0)&&prev.top_asks)symbol.top_asks=prev.top_asks;
    if((keepLiveDetail || ((!Number(symbol.total_bid_volume))&&Number(prev.total_bid_volume))))symbol.total_bid_volume=prev.total_bid_volume;
    if((keepLiveDetail || ((!Number(symbol.total_ask_volume))&&Number(prev.total_ask_volume))))symbol.total_ask_volume=prev.total_ask_volume;
    if((keepLiveDetail || ((!Number(symbol.spread_bps))&&Number(prev.spread_bps))))symbol.spread_bps=prev.spread_bps;
    if((keepLiveDetail || ((!Number(symbol.bid))&&Number(prev.bid))))symbol.bid=prev.bid;
    if((keepLiveDetail || ((!Number(symbol.ask))&&Number(prev.ask))))symbol.ask=prev.ask;
    if((keepLiveDetail || !Array.isArray(symbol.signal_history) || symbol.signal_history.length===0) && Array.isArray(prev.signal_history))symbol.signal_history=prev.signal_history;
    if((keepLiveDetail || !Array.isArray(symbol.factor_metrics) || symbol.factor_metrics.length===0) && Array.isArray(prev.factor_metrics))symbol.factor_metrics=prev.factor_metrics;
    if((keepLiveDetail || !Array.isArray(symbol.enterprise_metrics) || symbol.enterprise_metrics.length===0) && Array.isArray(prev.enterprise_metrics))symbol.enterprise_metrics=prev.enterprise_metrics;
    if(prev.__detailFreshAt)symbol.__detailFreshAt=prev.__detailFreshAt;
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

function hasRenderableMarketData(symbol){
  if(!symbol)return false;
  return (Number(symbol.mid)||0)>0
    || (Number(symbol.bid)||0)>0
    || (Number(symbol.ask)||0)>0
    || (Number(symbol.volume_24h)||0)>0
    || (Array.isArray(symbol.top_bids)&&symbol.top_bids.length>0)
    || (Array.isArray(symbol.top_asks)&&symbol.top_asks.length>0)
    || (Array.isArray(symbol.recent_trades)&&symbol.recent_trades.length>0);
}
function hasReadyDetailData(symbol){
  if(!symbol)return false;
  return (
    ((symbol.top_bids?.length||0)>0 && (symbol.top_asks?.length||0)>0)
    || (symbol.recent_trades?.length||0)>0
    || (symbol.big_trades?.length||0)>0
    || (
      (Number(symbol.bid)||0)>0
      && (Number(symbol.ask)||0)>0
      && (((Number(symbol.total_bid_volume)||0)>0) || ((Number(symbol.total_ask_volume)||0)>0))
    )
  );
}

const STARTUP_DEFAULT_SYMBOL='BTCUSDT';
const RESTORED_SYMBOL_GRACE_MS=1600;
const MANUAL_SELECTION_GRACE_MS=120000;

function hasStartupDefaultSymbol(){
  return (S.syms||[]).some(symbol=>symbol.symbol===STARTUP_DEFAULT_SYMBOL);
}

function startupSymbolPriority(sym){
  if(sym===STARTUP_DEFAULT_SYMBOL)return 0;
  return Number.MAX_SAFE_INTEGER;
}

function restoredSymbolExpired(sym){
  if(!sym || window.__bbRestoredSelectedSymbol!==sym)return false;
  const baseAt=Number(window.__bbRestoredSelectedAt||window.__bbBootAt||Date.now());
  return Date.now()-baseAt>=RESTORED_SYMBOL_GRACE_MS;
}
function hasManualSelectedSymbol(sym){
  if(!sym || window.__bbManualSelectedSymbol!==sym)return false;
  const selectedAt=Number(window.__bbManualSelectedAt||0);
  if(!selectedAt)return true;
  return Date.now()-selectedAt<MANUAL_SELECTION_GRACE_MS;
}

function pickPreferredSymbol(activeSymbol){
  if(activeSymbol&&hasManualSelectedSymbol(activeSymbol)){
    return activeSymbol;
  }
  const current=activeSymbol?getSymbolState(activeSymbol):null;
  const rankSymbol=symbol=>{
    if(!symbol)return -1;
    return Math.max(
      Number(symbol.pump_score)||0,
      Number(symbol.dump_score)||0,
      (symbol.whale_entry||symbol.whale_exit)?68:0,
      Number(symbol.anomaly_max_severity)||0,
      Number(symbol.anomaly_count_1m)||0
    );
  };
  const symbolSorter=(a,b)=>{
    const coreDiff=startupSymbolPriority(a?.symbol)-startupSymbolPriority(b?.symbol);
    if(coreDiff!==0)return coreDiff;
    const rankDiff=rankSymbol(b)-rankSymbol(a);
    if(rankDiff!==0)return rankDiff;
    const volDiff=(Number(b?.quote_vol_24h)||Number(b?.volume_24h)||0)-(Number(a?.quote_vol_24h)||Number(a?.volume_24h)||0);
    if(volDiff!==0)return volDiff;
    return String(a?.symbol||'').localeCompare(String(b?.symbol||''));
  };
  if(current&&hasManualSelectedSymbol(current.symbol))return current.symbol;
  if(current&&hasReadyDetailData(current))return current.symbol;
  if(current&&hasRenderableMarketData(current))return current.symbol;
  const liveCandidates=[...(S.syms||[])].filter(hasRenderableMarketData).sort(symbolSorter);
  const detailReadyCandidates=liveCandidates.filter(hasReadyDetailData);
  const startupReady=detailReadyCandidates.find(symbol=>symbol.symbol===STARTUP_DEFAULT_SYMBOL);
  if(startupReady)return startupReady.symbol;
  if(detailReadyCandidates[0])return detailReadyCandidates[0].symbol;
  const startupLive=liveCandidates.find(symbol=>symbol.symbol===STARTUP_DEFAULT_SYMBOL);
  if(startupLive)return startupLive.symbol;
  if(liveCandidates[0])return liveCandidates[0].symbol;
  const startupFallback=hasStartupDefaultSymbol()
    ?(S.syms||[]).find(symbol=>symbol.symbol===STARTUP_DEFAULT_SYMBOL)
    :null;
  if(startupFallback && (!current || restoredSymbolExpired(current.symbol) || window.__bbRestoredSelectedSymbol===current.symbol)){
    return startupFallback.symbol;
  }
  const feedSymbol=(S.feed||[])
    .map(item=>getSymbolState(item.symbol))
    .find(hasReadyDetailData)
    ?.symbol;
  if(feedSymbol)return feedSymbol;
  const hotCandidate=[...(S.syms||[])].filter(hasReadyDetailData).sort(symbolSorter)[0];
  if(hotCandidate)return hotCandidate.symbol;
  return current?.symbol||S.syms[0]?.symbol||null;
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

function upsertSymbolDetail(detail,options={}){
  if(!detail||!detail.symbol)return;
  const markDetailFresh=options.markDetailFresh!==false;
  const idx=(S.syms||[]).findIndex(s=>s.symbol===detail.symbol);
  if(idx>=0){
    const prev=S.syms[idx];
    const next={...prev,...detail};
    if((!detail.klines||Object.keys(detail.klines).length===0)&&prev.klines)next.klines=prev.klines;
    if((!detail.current_kline||Object.keys(detail.current_kline).length===0)&&prev.current_kline)next.current_kline=prev.current_kline;
    if((!detail.big_trades||detail.big_trades.length===0)&&prev.big_trades)next.big_trades=prev.big_trades;
    if((!detail.recent_trades||detail.recent_trades.length===0)&&prev.recent_trades)next.recent_trades=prev.recent_trades;
    if((!detail.top_bids||detail.top_bids.length===0)&&prev.top_bids)next.top_bids=prev.top_bids;
    if((!detail.top_asks||detail.top_asks.length===0)&&prev.top_asks)next.top_asks=prev.top_asks;
    if((!Number(detail.total_bid_volume))&&Number(prev.total_bid_volume))next.total_bid_volume=prev.total_bid_volume;
    if((!Number(detail.total_ask_volume))&&Number(prev.total_ask_volume))next.total_ask_volume=prev.total_ask_volume;
    if((!Number(detail.spread_bps))&&Number(prev.spread_bps))next.spread_bps=prev.spread_bps;
    if((!Number(detail.bid))&&Number(prev.bid))next.bid=prev.bid;
    if((!Number(detail.ask))&&Number(prev.ask))next.ask=prev.ask;
    if((!Array.isArray(detail.signal_history) || detail.signal_history.length===0) && Array.isArray(prev.signal_history))next.signal_history=prev.signal_history;
    if((!Array.isArray(detail.factor_metrics) || detail.factor_metrics.length===0) && Array.isArray(prev.factor_metrics))next.factor_metrics=prev.factor_metrics;
    if((!Array.isArray(detail.enterprise_metrics) || detail.enterprise_metrics.length===0) && Array.isArray(prev.enterprise_metrics))next.enterprise_metrics=prev.enterprise_metrics;
    if(markDetailFresh)next.__detailFreshAt=Date.now();
    else if(prev.__detailFreshAt)next.__detailFreshAt=prev.__detailFreshAt;
    S.syms[idx]=next;
  }else{
    if(markDetailFresh)detail.__detailFreshAt=Date.now();
    S.syms.push(detail);
  }
  if(S.sel===detail.symbol){
    S.selectedCache={...(S.selectedCache||{}),...detail};
  }
}

async function loadSymbolDetail(sym,renderAfter=false){
  if(!sym)return null;
  window.__bbDetailRequests=window.__bbDetailRequests||Object.create(null);
  try{
    let req=window.__bbDetailRequests[sym];
    if(!req){
      req=fetch(`/api/symbol/${encodeURIComponent(sym)}`)
        .then(r=>r.json())
        .finally(()=>{
          if(window.__bbDetailRequests?.[sym]===req){
            delete window.__bbDetailRequests[sym];
          }
        });
      window.__bbDetailRequests[sym]=req;
    }
    const detail=await req;
    if(!detail)return null;
    upsertSymbolDetail(detail,{markDetailFresh:true});
    if(renderAfter&&S.sel===sym){
      if(typeof scheduleSelectedDetailRender==='function')scheduleSelectedDetailRender(sym,true);
      else{
        renderDetail(sym);
        updOHLCV();
      }
    }
    return detail;
  }catch(_){
    return null;
  }
}
