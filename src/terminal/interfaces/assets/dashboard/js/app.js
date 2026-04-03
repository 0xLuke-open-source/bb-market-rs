function filterP(q){searchQ=q.toUpperCase();renderPairList();}

function normalizeMarketQuickFilter(mode){
  return mode==='all' ? 'key' : (mode||'key');
}

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
const MANUAL_SELECTION_GRACE_MS=20000;

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
  if(activeSymbol&&hasManualSelectedSymbol(activeSymbol))return activeSymbol;
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
    if(markDetailFresh)next.__detailFreshAt=Date.now();
    else if(prev.__detailFreshAt)next.__detailFreshAt=prev.__detailFreshAt;
    S.syms[idx]=next;
  }else{
    if(markDetailFresh)detail.__detailFreshAt=Date.now();
    S.syms.push(detail);
  }
  if(!S.sm[detail.symbol])S.sm[detail.symbol]={};
  if(Number.isFinite(Number(detail.mid)))S.sm[detail.symbol].mid=Number(detail.mid);
  if(Number.isFinite(Number(detail.obi)))S.sm[detail.symbol].obi=Number(detail.obi);
  if(Number.isFinite(Number(detail.ofi)))S.sm[detail.symbol].ofi=Number(detail.ofi);
  if(Number.isFinite(Number(detail.pump_score)))S.sm[detail.symbol].ps=Number(detail.pump_score);
  if(Number.isFinite(Number(detail.dump_score)))S.sm[detail.symbol].ds=Number(detail.dump_score);
  if(Number.isFinite(Number(detail.cvd)))S.sm[detail.symbol].cvd=Number(detail.cvd);
  if(Number.isFinite(Number(detail.taker_buy_ratio)))S.sm[detail.symbol].tbr=Number(detail.taker_buy_ratio);
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
      req=apiFetch(`/api/symbol/${encodeURIComponent(sym)}`)
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
        <span style="flex:1;font-variant-numeric:tabular-nums">${o.price!=null?fP(o.price,o.symbol):'市价'}</span>
        <span style="flex:1;font-variant-numeric:tabular-nums">${fNum(o.quantity)}</span>
        <span style="flex:1.2;color:var(--t2)">--</span>
        <span style="flex:.8;color:var(--y)">${filledPct(o)}%</span>
        <span style="flex:1;font-variant-numeric:tabular-nums">${fNum((o.price||0)*o.quantity)}</span>
        <span style="flex:1.2;color:var(--t2)">${o.trigger_price!=null?`${o.trigger_kind||'trigger'} @ ${fP(o.trigger_price,o.symbol)}`:'--'}</span>
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
        <span style="flex:1;font-variant-numeric:tabular-nums">${o.price!=null?fP(o.price,o.symbol):'市价'}</span>
        <span style="flex:1;font-variant-numeric:tabular-nums">${fNum(o.quantity)}</span>
        <span style="flex:1.2;color:var(--t2)">--</span>
        <span style="flex:.8;color:var(--y)">${filledPct(o)}%</span>
        <span style="flex:1;font-variant-numeric:tabular-nums">${fNum(o.filled_quote_qty||0)}</span>
        <span style="flex:1.2;color:var(--t2)">${o.trigger_price!=null?`${o.trigger_kind||'trigger'} @ ${fP(o.trigger_price,o.symbol)}`:o.status}</span>
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
        <span style="flex:1;font-variant-numeric:tabular-nums">${fP(t.price,t.symbol)}</span>
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
  if(side==='buy')document.getElementById('buy-price').value=fP(s.bid||sv(S.sel,'mid'),S.sel);
  else document.getElementById('sell-price').value=fP(s.ask||sv(S.sel,'mid'),S.sel);
  updateTotals();
}

function recalcQtyByPct(side,pct){
  pct=Number(pct)||0;
  if(!pct||!S.sel)return;
  if(side==='buy'){
    const price=parseFloat(document.getElementById('buy-price').value)||sv(S.sel,'mid')||1;
    const avail=getBalance('USDT').available||0;
    const qty=(avail*pct/100/price);
    document.getElementById('buy-qty').value=qty>0?fQ(qty,S.sel):'';
  }else{
    const asset=S.sel.replace('USDT','');
    const avail=getBalance(asset).available||0;
    const qty=(avail*pct/100);
    document.getElementById('sell-qty').value=qty>0?fQ(qty,S.sel):'';
  }
}

function resolvePriceInputSide(bookSide){
  const active=document.activeElement?.id;
  if(active==='buy-price')return 'buy';
  if(active==='sell-price')return 'sell';
  return bookSide==='ask'?'buy':'sell';
}

function applyBookPrice(bookSide,price){
  if(!S.sel)return;
  const side=resolvePriceInputSide(bookSide);
  const input=document.getElementById(`${side}-price`);
  if(!input)return;
  input.value=fP(price,S.sel);
  const pctInput=document.getElementById(`${side}-pct`);
  recalcQtyByPct(side,pctInput?.value||0);
  updateTotals();
}

function setBuyPct(pct){
  pct=Number(pct)||0;
  // 根据可用余额计算数量
  const price=parseFloat(document.getElementById('buy-price').value)||sv(S.sel||'','mid')||1;
  const avail=getBalance('USDT').available||0;
  const qty=(avail*pct/100/price);
  document.getElementById('buy-qty').value=qty>0?fQ(qty,S.sel):'';
  const total=qty*price;
  document.getElementById('buy-total').textContent=total.toFixed(2);
  setPctActive('buy',pct);
  document.getElementById('buy-pct').value=pct;
  updateTradeSliderTip('buy',pct);
}

function setSellPct(pct){
  pct=Number(pct)||0;
  const asset=(S.sel||'').replace('USDT','');
  const avail=getBalance(asset).available||0;
  const qty=(avail*pct/100);
  const price=parseFloat(document.getElementById('sell-price').value)||sv(S.sel||'','mid')||1;
  document.getElementById('sell-qty').value=qty>0?fQ(qty,S.sel):'';
  document.getElementById('sell-total').textContent=(qty*price).toFixed(2);
  setPctActive('sell',pct);
  document.getElementById('sell-pct').value=pct;
  updateTradeSliderTip('sell',pct);
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
  document.getElementById(id).value = fP(price,S.sel);
  updateTotals();
}

function updateTotals(){
  const bp=parseFloat(document.getElementById('buy-price').value)||sv(S.sel||'','mid')||0;
  recalcQtyByPct('buy',document.getElementById('buy-pct')?.value||0);
  const bq=parseFloat(document.getElementById('buy-qty').value)||0;
  document.getElementById('buy-total').textContent=(bp*bq).toFixed(2);
  const sp=parseFloat(document.getElementById('sell-price').value)||sv(S.sel||'','mid')||0;
  recalcQtyByPct('sell',document.getElementById('sell-pct')?.value||0);
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
function ema(sym,k,v){
  if(!S.sm[sym])S.sm[sym]={};
  if(k==='mid'){
    S.sm[sym][k]=v;
    return v;
  }
  const p=S.sm[sym][k];
  if(p===undefined){
    S.sm[sym][k]=v;
    return v;
  }
  const r=A*v+(1-A)*p;
  S.sm[sym][k]=r;
  return r;
}
function sv(sym,k){return S.sm[sym]?.[k]??0;}

function pct(v,total){return total>0?(v/total*100):0;}
function clamp(v,min,max){return Math.max(min,Math.min(max,v));}
function avg(arr){return arr.length?arr.reduce((a,b)=>a+b,0)/arr.length:0;}
function sum(arr){return arr.reduce((a,b)=>a+b,0);}
function fmtMetricValue(v,unit=''){
  if(typeof v==='string')return v;
  if(!Number.isFinite(v))return '--';
  if(unit==='%')return `${v.toFixed(1)}%`;
  if(unit==='x')return `${v.toFixed(2)}x`;
  if(unit==='bps')return `${(v/100).toFixed(2)}%`;
  if(unit==='count')return `${Math.round(v)}`;
  if(unit==='ratio')return `${v.toFixed(2)}`;
  if(unit==='compact')return fN(v);
  return `${v.toFixed(2)}${unit}`;
}
function metricTone(score,invert=false){
  const s=invert?(100-score):score;
  return s>=70?'cae-good':s>=40?'cae-warn':'cae-bad';
}
function calcReturnPct(bars){
  if(!bars||bars.length<2)return 0;
  const first=bars[0]?.o||bars[0]?.c||0;
  const last=bars[bars.length-1]?.c||0;
  return first?((last-first)/first*100):0;
}
function getBars(sym,interval,count){
  const s=S.syms.find(x=>x.symbol===sym);
  const bars=s?.klines?.[interval]||[];
  return bars.slice(Math.max(0,bars.length-count));
}
function getCurrentBar(sym,interval){
  const s=S.syms.find(x=>x.symbol===sym);
  return s?.current_kline?.[interval]||null;
}
function depthTotals(levels,n){
  return (levels||[]).slice(0,n).reduce((acc,[p,q])=>acc+((+p||0)*(+q||0)),0);
}
function depthGap(levels){
  const list=(levels||[]).slice(0,8).map(x=>+x[0]||0).filter(Boolean);
  if(list.length<3)return 0;
  const gaps=[];
  for(let i=1;i<list.length;i++){
    const prev=list[i-1],cur=list[i];
    gaps.push(Math.abs(cur-prev)/(prev||1)*10000);
  }
  return Math.max(...gaps,0);
}
function walkBookCost(levels,notionalTarget){
  let remain=notionalTarget;
  let filledQty=0,spent=0,lastPx=0;
  for(const [p,q] of (levels||[])){
    const px=+p||0,qty=+q||0;
    if(px<=0||qty<=0)continue;
    const levelNotional=px*qty;
    const take=Math.min(remain,levelNotional);
    spent+=take;
    filledQty+=take/px;
    remain-=take;
    lastPx=px;
    if(remain<=0)break;
  }
  return {spent,filledQty,remain,lastPx};
}
function ensureMetricHistory(sym,s){
  if(!S.metricH[sym])S.metricH[sym]=[];
  const bids=s.top_bids||[],asks=s.top_asks||[];
  const item={
    t:Date.now(),
    mid:sv(sym,'mid'),
    cvd:sv(sym,'cvd'),
    tbr:sv(sym,'tbr'),
    ps:sv(sym,'ps'),
    ds:sv(sym,'ds'),
    bid5:depthTotals(bids,5),
    ask5:depthTotals(asks,5),
    bid10:depthTotals(bids,10),
    ask10:depthTotals(asks,10),
    bid20:depthTotals(bids,20),
    ask20:depthTotals(asks,20),
    wallBid:s.max_bid_ratio||0,
    wallAsk:s.max_ask_ratio||0,
    spread:s.spread_bps||0,
    anomaly:s.anomaly_max_severity||0
  };
  const hist=S.metricH[sym];
  const prev=hist[hist.length-1];
  if(!prev || item.t-prev.t>4000){
    hist.push(item);
    if(hist.length>180)hist.shift();
  }else{
    hist[hist.length-1]=item;
  }
}
function metricDeltaPct(cur,prev){
  if(!prev)return 0;
  return prev?((cur-prev)/prev*100):0;
}
function buildMetricRow(name,score,value,tip,invert=false){
  const width=clamp(score,0,100);
  return `<div class="cae-row">
    <div class="cae-name">${name}</div>
    <div>
      <div class="cae-bar"><div class="cae-fill ${metricTone(score,invert)}" style="width:${width}%"></div></div>
      <div class="cae-tip">${tip}</div>
    </div>
    <div class="cae-val">${value}</div>
  </div>`;
}
function buildMetricSection(title,sub,items){
  return `<div class="cae-sec">
    <div class="cae-sec-h"><span class="cae-sec-t">${title}</span><span class="cae-sec-s">${sub}</span></div>
    <div class="cae-list">${items.map(item=>buildMetricRow(item.name,item.score,item.value,item.tip,item.invert)).join('')}</div>
  </div>`;
}
function getSignalPerf(sym){
  const list=(S.signalPerf[sym]||[]).filter(x=>x.done5||x.done15);
  const five=list.filter(x=>x.done5);
  const fifteen=list.filter(x=>x.done15);
  const win5=five.length?pct(five.filter(x=>x.win5).length,five.length):0;
  const win15=fifteen.length?pct(fifteen.filter(x=>x.win15).length,fifteen.length):0;
  const decay=avg(list.map(x=>x.decayMinutes||0));
  return {win5,win15,count5:five.length,count15:fifteen.length,decay};
}
function recordSignalPerf(sym,type,startPrice,score){
  if(!S.signalPerf[sym])S.signalPerf[sym]=[];
  S.signalPerf[sym].unshift({type,startPrice,score,createdAt:Date.now(),done5:false,done15:false,win5:false,win15:false,decayMinutes:null});
  if(S.signalPerf[sym].length>80)S.signalPerf[sym].pop();
}
function updateSignalPerfStats(){
  const now=Date.now();
  Object.entries(S.signalPerf).forEach(([sym,list])=>{
    const s=S.syms.find(x=>x.symbol===sym);
    const current=sv(sym,'mid') || s?.mid || 0;
    const levelScore=Math.max(s?.pump_score||0,s?.dump_score||0);
    list.forEach(item=>{
      const elapsedMin=(now-item.createdAt)/60000;
      const dir=item.type==='dump'?-1:1;
      if(!item.done5 && elapsedMin>=5){
        item.done5=true;
        item.win5=((current-item.startPrice)*dir)>=0;
      }
      if(!item.done15 && elapsedMin>=15){
        item.done15=true;
        item.win15=((current-item.startPrice)*dir)>=0;
      }
      if(item.decayMinutes==null && levelScore<Math.max(45,(item.score||60)-20)){
        item.decayMinutes=elapsedMin;
      }
    });
  });
}
function renderEnterpriseMetrics(sym){
  const s=S.syms.find(x=>x.symbol===sym);
  if(!s){
    S.ui.enterprise='';
    e('enterprise-metrics','');
    return;
  }
  const hist=S.metricH[sym]||[];
  const last=hist[hist.length-1]||null;
  const prev=hist[Math.max(0,hist.length-6)]||null;
  const bars1m=getBars(sym,'1m',20);
  const bars5m=getBars(sym,'5m',20);
  const bars15m=getBars(sym,'15m',20);
  const bars1h=getBars(sym,'1h',20);
  const cur1m=getCurrentBar(sym,'1m');
  const currentPx=sv(sym,'mid');
  const bigTrades=s.big_trades||[];
  const now=Date.now();
  const recentBig=bigTrades.filter(t=>now-(+t.t||0)<=60000);
  const prevBig=bigTrades.filter(t=>now-(+t.t||0)>60000 && now-(+t.t||0)<=120000);
  const bigRecentNotional=sum(recentBig.map(t=>(+t.p||0)*(+t.q||0)));
  const bigPrevNotional=sum(prevBig.map(t=>(+t.p||0)*(+t.q||0)));
  const estMinuteQuote=(s.quote_vol_24h||0)/1440;
  const largeTradeRatio=pct(bigRecentNotional,Math.max(estMinuteQuote,1));
  const continuity=100-Math.min(100,avg((hist.slice(-8)).map(x=>Math.abs((x.tbr||50)-50)*2)));
  const directionalContinuity=Math.abs(avg(hist.slice(-8).map(x=>(x.tbr||50)-50))*2);
  const tradeDensity=clamp(recentBig.length*16,0,100);
  const countSurge=prevBig.length?clamp((recentBig.length/prevBig.length)*25,0,100):clamp(recentBig.length*20,0,100);
  const amountSurge=bigPrevNotional?clamp((bigRecentNotional/bigPrevNotional)*25,0,100):clamp(bigRecentNotional/Math.max(estMinuteQuote,1)*100,0,100);
  const wallStrength=clamp(Math.max(s.max_bid_ratio||0,s.max_ask_ratio||0)*2.2,0,100);
  const cancelRatioEst=clamp(Math.abs(s.ofi_raw||0)/(Math.abs(s.ofi||0)+1)*30,0,100);
  const recoverySpeed=clamp(100-((s.spread_bps||0)*2)+Math.min(30,Math.abs(metricDeltaPct(last?.bid10||0,prev?.bid10||0))*0.3),0,100);
  const depth5Delta=metricDeltaPct((last?.bid5||0)+(last?.ask5||0),(prev?.bid5||0)+(prev?.ask5||0));
  const depth10Delta=metricDeltaPct((last?.bid10||0)+(last?.ask10||0),(prev?.bid10||0)+(prev?.ask10||0));
  const depth20Delta=metricDeltaPct((last?.bid20||0)+(last?.ask20||0),(prev?.bid20||0)+(prev?.ask20||0));
  const depthGapBps=Math.max(depthGap(s.top_bids),depthGap(s.top_asks));
  const ret1=calcReturnPct(bars1m.slice(-5));
  const ret5=calcReturnPct(bars5m.slice(-6));
  const ret15=calcReturnPct(bars15m.slice(-6));
  const ret60=calcReturnPct(bars1h.slice(-6));
  const sameDir=[ret1,ret5,ret15,ret60].filter(v=>v!==0);
  const multiTfConsistency=sameDir.length?clamp(Math.abs(sum(sameDir.map(v=>Math.sign(v))))/sameDir.length*100,0,100):0;
  const rangeNow=cur1m?((cur1m.h-cur1m.l)/(cur1m.o||1)*100):0;
  const rangeAvg=avg(bars1m.slice(-10).map(b=>((b.h-b.l)/(b.o||1)*100)));
  const volExpand=rangeAvg?clamp(rangeNow/rangeAvg*35,0,100):0;
  const prevHigh=Math.max(...bars1m.slice(-12).map(b=>b.h||0),0);
  const prevLow=Math.min(...bars1m.slice(-12).map(b=>b.l||Number.MAX_SAFE_INTEGER));
  const falseBreak=(currentPx>prevHigh && (cur1m?.c||currentPx)<prevHigh) || (currentPx<prevLow && (cur1m?.c||currentPx)>prevLow);
  const recentSpikeBase=bars1m.slice(-10)[0]?.o||currentPx;
  const recentExtreme=Math.max(...bars1m.slice(-10).map(b=>Math.abs((b.h-recentSpikeBase)/(recentSpikeBase||1))*100),0);
  const pullback=Math.abs(recentExtreme-Math.abs((currentPx-recentSpikeBase)/(recentSpikeBase||1)*100));
  const acceptance=(currentPx>=prevHigh*0.998 || currentPx<=prevLow*1.002)?clamp((s.taker_buy_ratio||50),0,100):45;
  const accumulation=((s.cvd>0?1:-1)*((s.taker_buy_ratio||50)-50)>=0 && (s.obi||0)>0)?78:((s.cvd<0 && (s.obi||0)<0)?25:52);
  const whaleFollow=s.whale_entry?clamp(((s.pump_score||0)+(s.taker_buy_ratio||50))/1.5,0,100):s.whale_exit?clamp(((s.dump_score||0)+(100-(s.taker_buy_ratio||50)))/1.5,0,100):45;
  const wallDwell=hist.length>=3?clamp(avg(hist.slice(-6).map(x=>Math.max(x.wallBid,x.wallAsk)))*2,0,100):0;
  const cvdSlope=hist.length>=2?((last?.cvd||0)-(prev?.cvd||0))/Math.max(Math.abs(prev?.cvd||0),1000)*100:0;
  const resonance=clamp((multiTfConsistency + Math.max(s.pump_score||0,s.dump_score||0))/2,0,100);
  const confirmation=(Math.abs(ret1)>0.2 && Math.abs(ret5)>0.2 && Math.sign(ret1)===Math.sign(ret5))?78:38;
  const upCount=S.syms.filter(x=>(x.change_24h_pct||0)>0).length;
  const downCount=S.syms.filter(x=>(x.change_24h_pct||0)<0).length;
  const strongShare=pct(S.syms.filter(x=>Math.max(x.pump_score||0,x.dump_score||0)>=70).length,Math.max(S.syms.length,1));
  const anomalyShare=pct(S.syms.filter(x=>(x.anomaly_max_severity||0)>=75).length,Math.max(S.syms.length,1));
  const linkage=clamp(Math.abs(upCount-downCount)/Math.max(S.syms.length,1)*100 + strongShare*0.4,0,100);
  const spreadLevel=clamp(100-(s.spread_bps||0)*2.5,0,100);
  const bookImpact=walkBookCost(s.top_asks,1000);
  const buyAvgPx=bookImpact.filledQty?bookImpact.spent/bookImpact.filledQty:currentPx;
  const slippageRisk=Math.abs(buyAvgPx-currentPx)/(currentPx||1)*10000;
  const executableDepth=clamp((1-(bookImpact.remain/1000))*100,0,100);
  const liquidityWarning=clamp(((s.spread_bps||0)*1.6)+(100-executableDepth)+(s.anomaly_max_severity||0)*0.35,0,100);
  const perf=getSignalPerf(sym);

  const sections=[
    buildMetricSection('成交结构','大单与主动成交',[
      {name:'大单成交占比',score:clamp(largeTradeRatio,0,100),value:fmtMetricValue(largeTradeRatio,'%'),tip:`最近1分钟大单成交额相当于日均每分钟成交额的 ${largeTradeRatio.toFixed(1)}%。`},
      {name:'买卖连续性',score:clamp(directionalContinuity,0,100),value:fmtMetricValue(directionalContinuity,'%'),tip:`主动买卖方向连续程度，越高说明一边更持续。`},
      {name:'短时成交密度',score:tradeDensity,value:fmtMetricValue(recentBig.length,'count'),tip:`最近1分钟捕捉到 ${recentBig.length} 笔大额成交。`},
      {name:'笔数突变',score:countSurge,value:fmtMetricValue(recentBig.length-prevBig.length,'count'),tip:`对比前1分钟，大单笔数变化更明显。`},
      {name:'成交额突变',score:amountSurge,value:fmtMetricValue(bigRecentNotional,'compact'),tip:`最近1分钟大单成交额与前1分钟对比。`}
    ]),
    buildMetricSection('盘口结构','深度与挂单质量',[
      {name:'买卖墙强度',score:wallStrength,value:fmtMetricValue(Math.max(s.max_bid_ratio||0,s.max_ask_ratio||0),'%'),tip:`大额挂单在前排深度中的占比。`},
      {name:'挂撤单比',score:cancelRatioEst,value:fmtMetricValue(cancelRatioEst,'%'),tip:`根据深度净变化和原始订单流估算，数值高说明撤改单更频繁。`,invert:true},
      {name:'恢复速度',score:recoverySpeed,value:fmtMetricValue(recoverySpeed,'%'),tip:`点差与深度恢复综合估算，越高越容易迅速回补。`},
      {name:'前5/10/20档变化',score:clamp((Math.abs(depth5Delta)+Math.abs(depth10Delta)+Math.abs(depth20Delta))/3,0,100),value:`${depth5Delta.toFixed(0)} / ${depth10Delta.toFixed(0)} / ${depth20Delta.toFixed(0)}%`,tip:`对比约 20-30 秒前的深度变化。`},
      {name:'深度断层',score:clamp(depthGapBps/2,0,100),value:fmtMetricValue(depthGapBps,'bps'),tip:`相邻档位价格跳空越大，深度断层越明显。`,invert:true}
    ]),
    buildMetricSection('价格行为','多周期价格状态',[
      {name:'多周期一致性',score:multiTfConsistency,value:fmtMetricValue(multiTfConsistency,'%'),tip:`1m / 5m / 15m / 1h 涨跌方向一致程度。`},
      {name:'波动扩张/收缩',score:volExpand,value:fmtMetricValue(volExpand,'%'),tip:`当前 1m 波动相对最近 10 根 1m 的放大程度。`},
      {name:'假突破识别',score:falseBreak?82:28,value:falseBreak?'疑似假突破':'暂未发现',tip:`刚破高/破低又收回时更需要防追单。`,invert:!falseBreak?false:true},
      {name:'回吐幅度',score:clamp(pullback*10,0,100),value:fmtMetricValue(pullback,'%'),tip:`急拉急砸后已经回吐的幅度，越大越需要谨慎。`,invert:true},
      {name:'新高/低承接',score:acceptance,value:fmtMetricValue(acceptance,'%'),tip:`新高或新低附近是否仍有主动承接。`}
    ]),
    buildMetricSection('资金痕迹','吸筹、派发与大户跟随',[
      {name:'持续吸筹/派发',score:accumulation,value:s.cvd>=0?'偏吸筹':'偏派发',tip:`结合主动买卖量差、主动买入占比和盘口失衡。`},
      {name:'大户跟随强度',score:whaleFollow,value:fmtMetricValue(whaleFollow,'%'),tip:`大户信号出现后，盘口和主动成交是否继续跟随。`},
      {name:'大单停留时间',score:wallDwell,value:fmtMetricValue(wallDwell,'%'),tip:`根据大墙连续出现历史估算，越高说明挂单停留更久。`},
      {name:'主动买卖量差斜率',score:clamp(Math.abs(cvdSlope),0,100),value:fmtMetricValue(cvdSlope,'%'),tip:`主动买卖量差增长或衰减速度。`}
    ]),
    buildMetricSection('跨周期指标','信号共振与确认',[
      {name:'1m/5m/15m/1h共振',score:resonance,value:fmtMetricValue(resonance,'%'),tip:`短中周期信号是否同时偏向同一方向。`},
      {name:'短期获中期确认',score:confirmation,value:confirmation>=60?'已确认':'待确认',tip:`短周期异动是否已得到 5m / 15m 方向确认。`}
    ]),
    buildMetricSection('市场广度','全市场同步状态',[
      {name:'上涨/下跌家数',score:pct(Math.max(upCount,downCount),Math.max(S.syms.length,1)),value:`${upCount} / ${downCount}`,tip:`当前全市场上涨家数与下跌家数。`},
      {name:'强势币占比',score:strongShare,value:fmtMetricValue(strongShare,'%'),tip:`拉升或下跌评分达到 70 分以上的币占比。`},
      {name:'异常币占比',score:anomalyShare,value:fmtMetricValue(anomalyShare,'%'),tip:`异常波动严重的币占比。`},
      {name:'板块联动强弱',score:linkage,value:fmtMetricValue(linkage,'%'),tip:`全市场方向集中度和强势币占比综合估算。`}
    ]),
    buildMetricSection('交易质量','可成交性与流动性',[
      {name:'点差水平',score:spreadLevel,value:fmtMetricValue(s.spread_bps||0,'bps'),tip:`点差越小，短线执行质量越好。`},
      {name:'深度可成交性',score:executableDepth,value:fmtMetricValue(executableDepth,'%'),tip:`按 1000 USDT 试算，盘口可立即承接的程度。`},
      {name:'滑点风险估计',score:clamp(slippageRisk*6,0,100),value:fmtMetricValue(slippageRisk,'bps'),tip:`按 1000 USDT 吃单估算的买入滑点。`,invert:true},
      {name:'流动性恶化预警',score:liquidityWarning,value:liquidityWarning>=70?'偏高':'正常',tip:`结合点差、可成交深度和异常波动的综合风险。`,invert:true}
    ]),
    buildMetricSection('信号质量','最近触发后的表现',[
      {name:'过去5分钟表现',score:perf.win5,value:perf.count5?`${perf.win5.toFixed(0)}%`:'样本少',tip:`当前页面运行期间，信号触发 5 分钟后方向正确的比例。`},
      {name:'过去15分钟表现',score:perf.win15,value:perf.count15?`${perf.win15.toFixed(0)}%`:'样本少',tip:`当前页面运行期间，信号触发 15 分钟后方向正确的比例。`},
      {name:'胜率/误报率',score:perf.win15||perf.win5,value:perf.count15?`${perf.win15.toFixed(0)} / ${(100-perf.win15).toFixed(0)}%`:'待积累',tip:`胜率越高说明信号更稳定；误报率越低越好。`},
      {name:'信号衰减速度',score:clamp(100-(perf.decay||0)*6,0,100),value:perf.decay?`${perf.decay.toFixed(1)} 分钟`:'待积累',tip:`信号从强提醒回落到普通关注的平均时间。`}
    ])
  ];
  setHtmlIfChanged('enterprise-metrics',sections.join(''),'enterprise');
}

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
  if(!S.sel)return;const s=getSymbolState(S.sel);if(!s)return;
  const ik=IVMAP[curIv]||'1m';const bars=s.klines?.[ik]||[];
  const cur=s.current_kline?.[ik];const bar=cur||(bars.length?bars[bars.length-1]:null);
  if(bar){e('ci-o',fP(bar.o));e('ci-h',fP(bar.h));e('ci-l2',fP(bar.l));
    e('ci-c',fP(bar.c));e('ci-v',fN(bar.v));e('ci-tbr',bar.tbr.toFixed(1)+'%');}
}

// ── 主渲染 ───────────────────────────────────────────────────────
function render(data){
  S.syms=mergeSymbols(data.symbols||[]);S.feed=compactSignalFeed(data.feed||[],20);
  S.trader=data.trader||{account_id:0,balances:[],open_orders:[],order_history:[],trade_history:[]};

  S.syms.forEach(s=>{
    ema(s.symbol,'obi',s.obi||0);ema(s.symbol,'ps',s.pump_score||0);
    ema(s.symbol,'ds',s.dump_score||0);ema(s.symbol,'ofi',s.ofi||0);ema(s.symbol,'mid',s.mid||0);
    if(!S.sm[s.symbol])S.sm[s.symbol]={};
    S.sm[s.symbol].cvd=s.cvd||0;S.sm[s.symbol].tbr=s.taker_buy_ratio||50;
    ensureMetricHistory(s.symbol,s);
    if(!S.cvdH[s.symbol])S.cvdH[s.symbol]=[];
    S.cvdH[s.symbol].push({t:nowT(),v:s.cvd||0});
    if(S.cvdH[s.symbol].length>HL)S.cvdH[s.symbol].shift();
    S.tr[s.symbol]=(s.recent_trades||[]).map(t=>({
      p:t.p,
      q:t.q,
      buy:!!t.buy,
      t:typeof t.t==='number'?new Date(t.t).toLocaleTimeString('zh-CN',{hour12:false}):String(t.t||'--')
    }));
  });

  const act=S.syms.filter(s=>sv(s.symbol,'ps')>=60||sv(s.symbol,'ds')>=60).length;
  e('nc',S.syms.length);e('ns2',act);
  updateSignalPerfStats();

  scheduleBottomStripRefresh();
  scheduleSlowPanelsRefresh();

  let activeSymbol=S.sel;
  if(activeSymbol){
    const selected=getSymbolState(activeSymbol);
    if(selected){
      S.selectedCache=selected;
    }else if(hasManualSelectedSymbol(activeSymbol)){
      S.selectedCache={
        symbol:activeSymbol,
        top_bids:[],
        top_asks:[],
        recent_trades:[],
        big_trades:[],
        klines:{},
        current_kline:{}
      };
    }else{
      activeSymbol=null;
      S.sel=null;
      S.selectedCache=null;
      S.detailSignal=null;
    }
  }
  const cur=pickPreferredSymbol(activeSymbol);
  if(cur){
    if(S.sel!==cur){
      if(window.__bbRestoredSelectedSymbol && cur!==window.__bbRestoredSelectedSymbol){
        window.__bbRestoredSelectedSymbol=null;
        window.__bbRestoredSelectedAt=0;
      }
      S.sel=cur;
      S.selectedCache=getSymbolState(cur);
      saveViewPrefs();
      connectSelectedSymbolStream(cur);
    }
    S.ui.detailKey='';
    if(tvSym!==('BINANCE:'+cur) || !document.getElementById('tv-widget').children.length){
      initTV(cur,curIv);
    }
    renderDetail(cur);
    if(window.__bbSelectedSymbolWsSymbol!==cur){
      connectSelectedSymbolStream(cur);
    }
    const selected=getSymbolState(cur);
    if(selected&&(
      !selected.klines
      || Object.keys(selected.klines).length===0
      || !hasReadyDetailData(selected)
    )){
      loadSymbolDetail(cur,true);
    }
  }
  updOHLCV();
  renderOrders();
}

// ── 币对列表（三区同时渲染，scard 风格）──────────────────────
function renderPairList(){
  let all=[...S.syms];
  const scoreOf=s=>Math.max(sv(s.symbol,'ps'),sv(s.symbol,'ds'));
  if(marketSort==='up'){
    all.sort((a,b)=>sv(b.symbol,'ps')-sv(a.symbol,'ps') || (b.change_24h_pct||0)-(a.change_24h_pct||0));
  }else if(marketSort==='down'){
    all.sort((a,b)=>sv(b.symbol,'ds')-sv(a.symbol,'ds') || Math.abs(b.change_24h_pct||0)-Math.abs(a.change_24h_pct||0));
  }else if(marketSort==='whale'){
    all.sort((a,b)=>(Number(b.whale_entry||b.whale_exit)-Number(a.whale_entry||a.whale_exit)) || (b.max_bid_ratio||0)-(a.max_bid_ratio||0) || scoreOf(b)-scoreOf(a));
  }else if(marketSort==='anomaly'){
    all.sort((a,b)=>(b.anomaly_max_severity||0)-(a.anomaly_max_severity||0) || (b.anomaly_count_1m||0)-(a.anomaly_count_1m||0) || scoreOf(b)-scoreOf(a));
  }else{
    all.sort((a,b)=>scoreOf(b)-scoreOf(a) || (b.anomaly_max_severity||0)-(a.anomaly_max_severity||0));
  }
  if(marketQuickFilter==='fav'){
    all=all.filter(s=>isFavorite(s.symbol));
  }else if(marketQuickFilter==='key'){
    all=all.filter(s=>['重点关注','强提醒'].includes(s.watch_level));
  }else if(marketQuickFilter==='strong'){
    all=all.filter(s=>s.watch_level==='强提醒');
  }else if(marketQuickFilter==='whale'){
    all=all.filter(s=>s.whale_entry||s.whale_exit);
  }else if(marketQuickFilter==='anomaly'){
    all=all.filter(s=>(s.anomaly_max_severity||0)>=75 || (s.anomaly_count_1m||0)>=30);
  }
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
      tagHtml=`<span class="cc-tag" style="background:var(--b-glow);color:var(--b);border:1px solid rgba(24,144,255,.2)">🐋 大户进场</span>`;
    } else if(sv(s.symbol,'ps')>=70||(s.pump_signal)){
      cls='cc-pump';
      tagHtml=`<span class="cc-tag" style="background:var(--g-dim);color:var(--g);border:1px solid rgba(14,203,129,.2)">🚀 上涨 ${Math.round(ps)}</span>`;
    } else if(sv(s.symbol,'ds')>=70||(s.dump_signal)){
      cls='cc-dump';
      tagHtml=`<span class="cc-tag" style="background:var(--r-dim);color:var(--r);border:1px solid rgba(246,70,93,.2)">📉 下跌 ${Math.round(ds)}</span>`;
    }
    const scoreBar=ps>0||ds>0?`
      <div class="cc-bars">
        <div class="pib"><div class="pbf pf-g" style="width:${Math.min(100,ps)}%"></div></div>
        <div class="pib"><div class="pbf pf-r" style="width:${Math.min(100,ds)}%"></div></div>
        <div class="pib"><div class="pbf pf-b" style="width:${Math.min(100,Math.abs(obi)*2)}%"></div></div>
      </div>`:'';
    return `<div class="coin-card ${cls}${S.sel===s.symbol?' act':''}" onclick="focusSignal('${s.symbol}','${s.watch_level||'观察'}','${(s.signal_reason||s.status_summary||'继续观察市场变化').replace(/'/g,'&#39;')}')">
      <div class="cc-h">
        <div class="cc-head-main">
          <span class="cc-sym">${sym}<span style="font-size:9px;color:var(--t3);font-weight:400">/USDT</span></span>
          <div class="cc-price-wrap">
            <span class="cc-price" style="color:${chgColor}">${fP(mid,s)}</span>
            <span class="cc-chg" style="color:${chgColor};background:${chgBg}">${chg>=0?'+':''}${chg.toFixed(2)}%</span>
          </div>
        </div>
        <div class="cc-head-actions">
          <button class="cc-fav ${isFavorite(s.symbol)?'act':''}" onclick="event.stopPropagation();toggleFavorite('${s.symbol}')">${isFavorite(s.symbol)?'★':'☆'}</button>
        </div>
      </div>
      ${tagHtml?`<div>${tagHtml}</div>`:''}
      <div class="cc-stats" style="color:var(--t3)">拉:<span style="color:${ps>=60?'var(--g)':'var(--t2)'}">${Math.round(ps)}</span> &nbsp;砸:<span style="color:${ds>=60?'var(--r)':'var(--t2)'}">${Math.round(ds)}</span> &nbsp;买卖盘失衡:<span>${obi.toFixed(1)}%</span></div>
      <div class="cc-stats" style="color:var(--t3)">${s.watch_level||'观察'} · ${s.status_summary||'继续观察市场变化'}</div>
      ${scoreBar}
    </div>`;
  };

  updateSoftStreamList('sec-sigs',sigs,{
    getKey:s=>s.symbol,
    renderItem:mkCard,
    emptyHtml:'<div class="ls-empty">暂无信号币种</div>',
    minAnimateInterval:5000,
    pauseOnHover:true
  });
  updateSoftStreamList('sec-whales',whales,{
    getKey:s=>s.symbol,
    renderItem:mkCard,
    emptyHtml:'<div class="ls-empty">暂无鲸鱼动态</div>',
    minAnimateInterval:5000,
    pauseOnHover:true
  });
  document.getElementById('sec-sig-cnt').textContent=sigs.length;
  document.getElementById('sec-whale-cnt').textContent=whales.length;
}

// ── 底部币对快选（当前选中附近5个） ─────────────────────────────
function renderPairMini(){
  const list=[...S.syms].sort((a,b)=>Math.max(sv(b.symbol,'ps'),sv(b.symbol,'ds'))-Math.max(sv(a.symbol,'ps'),sv(a.symbol,'ds'))).slice(0,5);
  setHtmlIfChangedPaused('pair-mini',list.map(s=>{
    const chg=s.change_24h_pct||0,cls=chg>=0?'pmu':'pmd';
    return `<div class="pi-mini" onclick="focusSignal('${s.symbol}','${s.watch_level||'观察'}','${(s.signal_reason||s.status_summary||'继续观察市场变化').replace(/'/g,'&#39;')}')">
      <span class="pm-sym">${s.symbol.replace('USDT','/U')}</span>
      <span class="pm-p" style="color:${chg>=0?'var(--g)':'var(--r)'}">${fP(sv(s.symbol,'mid'),s)}</span>
      <span class="pm-c ${cls}">${chg>=0?'+':''}${chg.toFixed(2)}%</span>
    </div>`;
  }).join(''),'pairMini','bottom');
}

// ── Ticker ───────────────────────────────────────────────────────
function renderTicker(){
  const top=[...S.syms].sort((a,b)=>Math.abs(b.change_24h_pct||0)-Math.abs(a.change_24h_pct||0)).slice(0,20);
  setHtmlIfChangedPaused('ticker-scroll',top.map(s=>{
    const chg=s.change_24h_pct||0,cls=chg>=0?'tbu':'tbd';
    return `<div class="tbi" onclick="focusSignal('${s.symbol}','${s.watch_level||'观察'}','${(s.signal_reason||s.status_summary||'继续观察市场变化').replace(/'/g,'&#39;')}')">
      <span class="tb-s">${s.symbol.replace('USDT','/U')}</span>
      <span class="tb-p" style="color:${chg>=0?'var(--g)':'var(--r)'}">${fP(sv(s.symbol,'mid'),s)}</span>
      <span class="tb-c ${cls}">${chg>=0?'+':''}${chg.toFixed(2)}%</span>
    </div>`;
  }).join(''),'ticker','bottom');
}

// ── 选中币种 ─────────────────────────────────────────────────────
async function selSym(sym){
  window.__bbManualSelectedSymbol=sym;
  window.__bbManualSelectedAt=Date.now();
  window.__bbRestoredSelectedSymbol=null;
  window.__bbRestoredSelectedAt=0;
  S.sel=sym;
  saveViewPrefs();
  initTV(sym,curIv);
  connectSelectedSymbolStream(sym);
  renderDetail(sym);
  renderPairList();
  await loadSymbolDetail(sym,true);
}

function buildSignalHistory(sym,signal){
  const targetType=signalTypeFromTag(signal?.tag||'');
  const feedItems=(S.feed||[])
    .filter(item=>item.symbol===sym && (!targetType || item.type===targetType))
    .map(item=>({time:item.time||'--',desc:item.desc||''}));
  const alertItems=(S.alerts||[])
    .filter(item=>item.full===sym && (!targetType || item.type===targetType))
    .map(item=>({time:item.time||'--',desc:item.desc||''}));
  const seen=new Set();
  return [...alertItems,...feedItems].filter(item=>{
    const key=`${item.time}|${item.desc}`;
    if(seen.has(key))return false;
    seen.add(key);
    return true;
  }).slice(0,10);
}

function updateSignalDetail(sym,signal){
  const s=S.syms.find(x=>x.symbol===sym);
  if(!s)return;
  const change=(s.change_24h_pct||0);
  e('signal-detail-tag',signal?.tag||'当前币种');
  e('signal-detail-text',signal?.desc||s.signal_reason||'当前没有特别突出的异常信号。');
  e('signal-detail-level',s.watch_level||'观察');
  e('signal-detail-price',fP(sv(sym,'mid'),s));
  e('signal-detail-change',`${change>=0?'+':''}${change.toFixed(2)}%`);
  e('signal-detail-tbr',`${sv(sym,'tbr').toFixed(1)}%`);
  const history=buildSignalHistory(sym,signal);
  document.getElementById('signal-detail-history').innerHTML=history.length
    ?history.map(item=>`<div class="sigdetail-item"><span class="sigdetail-time">${item.time}</span><span class="sigdetail-desc">${item.desc}</span></div>`).join('')
    :'<div class="sigdetail-empty">最近还没有同类提醒，先继续观察。</div>';
}

function focusSignal(sym,signalTag='',signalDesc=''){
  S.detailSignal={sym,tag:signalTag,desc:signalDesc};
  selSym(sym);
  updateSignalDetail(sym,S.detailSignal);
  const box=document.querySelector('.ca-summary');
  if(box){
    box.classList.add('flash');
    setTimeout(()=>box.classList.remove('flash'),1200);
  }
}

function clearCvdCanvas(){
  const canvas=document.getElementById('cvd-c');
  const ctx=canvas?.getContext?.('2d');
  if(!canvas||!ctx)return;
  ctx.clearRect(0,0,canvas.width,canvas.height);
}
function loadingRows(count=4){
  return Array.from({length:count},()=>`
    <div class="loading-row">
      <span class="loading-line w40"></span>
      <span class="loading-line w26"></span>
      <span class="loading-line w18"></span>
    </div>`).join('');
}
function loadingFactors(count=5){
  return Array.from({length:count},()=>`
    <div class="fi is-loading">
      <div class="loading-line w44"></div>
      <div class="loading-line w90"></div>
      <div class="loading-line w28 loading-right"></div>
    </div>`).join('');
}
function loadingEnterpriseSections(count=3){
  return Array.from({length:count},()=>`
    <div class="cae-sec is-loading">
      <div class="cae-sec-h">
        <span class="loading-line w36"></span>
        <span class="loading-line w22"></span>
      </div>
      <div class="cae-list">
        <div class="cae-row">
          <div class="loading-line w48"></div>
          <div class="loading-line w92"></div>
          <div class="loading-line w26 loading-right"></div>
        </div>
        <div class="cae-row">
          <div class="loading-line w42"></div>
          <div class="loading-line w86"></div>
          <div class="loading-line w20 loading-right"></div>
        </div>
      </div>
    </div>`).join('');
}
function renderDetailLoadingState(sym,s){
  const symShort=sym.replace('USDT','');
  const quoteBal=getBalance('USDT');
  const baseBal=getBalance(symShort);
  refreshFavoriteButton();
  e('nav-sym',fmtSym(sym));
  es('nav-price','--',null,'var(--t2)');
  const nc=document.getElementById('nav-chg');
  nc.textContent='同步中';
  nc.className='nav-chg';
  nc.style.cssText='color:var(--t2);background:rgba(148,163,184,.10)';
  es('nv-chg','--',null,'var(--t2)');
  e('nv-hi','--');e('nv-lo','--');e('nv-vol','--');e('nv-sp','--');e('nv-ps','--');e('nv-cvd','--');
  if(typeof updateDocumentTitle==='function')updateDocumentTitle(sym,'--',null);
  document.getElementById('buy-unit').textContent=symShort;
  document.getElementById('sell-unit').textContent=symShort;
  document.getElementById('sell-unit2').textContent=symShort;
  document.getElementById('btn-buy').textContent='买入 '+symShort;
  document.getElementById('btn-sell').textContent='卖出 '+symShort;
  e('avail-buy',fNum(quoteBal.available||0));
  document.querySelector('#trade-area .ta-side:nth-child(2) .ta-avail span:last-child').innerHTML=`${fNum(baseBal.available||0)} <span id="sell-unit2" style="color:var(--t3)">${symShort}</span>`;
  document.getElementById('ob-asks').innerHTML='';
  document.getElementById('ob-bids').innerHTML='';
  e('ob-mid','--');e('ob-bps','同步中');
  document.getElementById('or-b').style.width='50%';
  document.getElementById('or-b').textContent='买 --';
  document.getElementById('or-s').textContent='卖 --';
  e('tr-cnt','同步中');
  document.getElementById('tr-list').innerHTML='<div class="loading-empty">正在等待最新成交...</div>';
  e('rd-sym',fmtSym(sym));
  es('rd-p','--',null,'var(--t2)');
  const rc=document.getElementById('rd-c');
  rc.textContent='同步中';
  rc.style.cssText='background:rgba(148,163,184,.10);color:var(--t2)';
  e('rd-bid','--');e('rd-ask','--');e('rd-chg','--');e('rd-vol','--');e('rd-hi','--');e('rd-lo','--');e('rd-ps','--');e('rd-ds','--');
  e('rd-watch-level','同步中');
  document.getElementById('rd-watch-level').style.color='var(--t2)';
  document.getElementById('rd-watch-level').style.borderColor='rgba(148,163,184,.22)';
  e('rd-summary','正在同步实时行情，请稍候。');
  e('rd-reason','已进入该币种，正在等待首个实时价格、盘口和成交快照。');
  e('bt-cnt','...');
  document.getElementById('bt-list').innerHTML=loadingRows(5);
  e('signal-detail-tag','行情同步中');
  e('signal-detail-text','实时信号还没到达，拿到首个 live 快照后这里会展示当前最值得注意的原因。');
  e('signal-detail-level','同步中');
  e('signal-detail-price','--');
  e('signal-detail-change','--');
  e('signal-detail-tbr','--');
  document.getElementById('signal-detail-history').innerHTML='<div class="loading-empty">等待同类提醒...</div>';
  e('cvd-v','--');
  clearCvdCanvas();
  document.getElementById('rf-list').innerHTML=loadingFactors(6);
  S.ui.enterprise='';
  document.getElementById('enterprise-metrics').innerHTML=loadingEnterpriseSections(3);
}

// ── 详情 ─────────────────────────────────────────────────────────
function renderDetail(sym){
  const s=getSymbolState(sym);if(!s)return;
  if(S.sel===sym)S.selectedCache=s;
  const detailKey=selectedDetailKey(sym);
  if(S.ui.detailKey===detailKey){
    renderEnterpriseMetrics(sym);
    return;
  }
  S.ui.detailKey=detailKey;
  const mid=sv(sym,'mid'),chg=s.change_24h_pct||0;
  const gc=chg>=0?'var(--g)':'var(--r)';
  const cvd=sv(sym,'cvd'),ps=sv(sym,'ps'),ds=sv(sym,'ds');
  const obi=sv(sym,'obi'),ofi=sv(sym,'ofi'),tbr=sv(sym,'tbr');
  const symShort=sym.replace('USDT','');
  const watchLevel=s.watch_level||'观察';
  const levelColor=watchLevel==='强提醒'?'var(--r)':watchLevel==='重点关注'?'var(--y)':watchLevel==='普通关注'?'var(--b)':'var(--t2)';
  if(!hasRenderableMarketData(s)){
    renderDetailLoadingState(sym,s);
    return;
  }
  refreshFavoriteButton();

  // 顶部导航
  e('nav-sym',sym.replace('USDT','/USDT'));
  setSoftValue('nav-price',fP(mid,s),{color:gc,effect:'pulse'});
  const nc=document.getElementById('nav-chg');
  nc.textContent=(chg>=0?'+':'')+chg.toFixed(2)+'%';nc.className='nav-chg '+(chg>=0?'nup':'ndn');nc.style.cssText='';
  es('nv-chg',(chg>=0?'+':'')+chg.toFixed(2)+'%',null,gc);
  e('nv-hi',fP(s.high_24h||0,s));e('nv-lo',fP(s.low_24h||0,s));
  e('nv-vol',fN(s.quote_vol_24h||0));e('nv-sp',(s.spread_bps/100).toFixed(2)+'%');
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
  const askRows=[...asks].reverse().map(([p,q],i)=>{
    const cum=ats[asks.length-1-i];
    return {p,q,cum,width:(cum/mx*100).toFixed(0)};
  });
  const bidRows=bids.map(([p,q],i)=>{
    const cum=bts[i];
    return {p,q,cum,width:(cum/mx*100).toFixed(0)};
  });
  document.getElementById('ob-asks').innerHTML=askRows.map(row=>
    `<div class="ob-row"><div class="ob-bg bga" style="width:${row.width}%"></div>
      <span class="ob-p ap clickable" onclick="applyBookPrice('ask',${row.p})">${fP(row.p,s)}</span><span class="ob-q">${fBookNum(row.q)}</span><span class="ob-c">${fBookNum(row.cum)}</span></div>`
  ).join('');
  document.getElementById('ob-bids').innerHTML=bidRows.map(row=>
    `<div class="ob-row"><div class="ob-bg bgb" style="width:${row.width}%"></div>
      <span class="ob-p bp clickable" onclick="applyBookPrice('bid',${row.p})">${fP(row.p,s)}</span><span class="ob-q">${fBookNum(row.q)}</span><span class="ob-c">${fBookNum(row.cum)}</span></div>`
  ).join('');
  setSoftValue('ob-mid',fP(mid,s),{color:gc,effect:'pulse'});e('ob-bps',(s.spread_bps/100).toFixed(2)+'%');

  // 成交记录
  const tr=(s.recent_trades||[]).length
    ?(s.recent_trades||[]).map(t=>({
      p:t.p,
      q:t.q,
      buy:!!t.buy,
      t:typeof t.t==='number'?new Date(t.t).toLocaleTimeString('zh-CN',{hour12:false}):String(t.t||'--')
    }))
    :(S.tr[sym]||[]);
  e('tr-cnt',tr.length+' 笔');
  document.getElementById('tr-list').innerHTML=tr.slice(0,50).map(t=>`
    <div class="tr-row">
      <span style="color:${t.buy?'var(--g)':'var(--r)'};font-weight:600">${fP(t.p,s)}</span>
      <span style="color:var(--t2)">${fQ(t.q,s)}</span>
      <span style="color:var(--t3)">${t.t}</span>
    </div>`).join('');

  // 分析面板
  e('rd-sym',sym.replace('USDT','/USDT'));
  setSoftValue('rd-p',fP(mid,s),{color:gc,effect:'pulse'});
  const rc=document.getElementById('rd-c');
  rc.textContent=(chg>=0?'+':'')+chg.toFixed(2)+'%';
  rc.style.cssText=`background:${chg>=0?'rgba(14,203,129,.12)':'rgba(246,70,93,.12)'};color:${gc}`;
  setSoftValue('rd-bid',fP(s.bid||0,s),{color:'var(--g)'});
  setSoftValue('rd-ask',fP(s.ask||0,s),{color:'var(--r)'});
  es('rd-chg',(chg>=0?'+':'')+chg.toFixed(2)+'%',null,gc);
  e('rd-vol',fN(s.volume_24h||0));e('rd-hi',fP(s.high_24h||0,s));e('rd-lo',fP(s.low_24h||0,s));
  e('rd-ps',Math.round(ps));e('rd-ds',Math.round(ds));
  e('rd-watch-level',watchLevel);
  document.getElementById('rd-watch-level').style.color=levelColor;
  document.getElementById('rd-watch-level').style.borderColor=levelColor;
  e('rd-summary',s.status_summary||'当前没有明显的主导方向，先继续观察。');
  e('rd-reason',s.signal_reason||'当前没有特别突出的异常信号。');
  updateSignalDetail(sym,S.detailSignal&&S.detailSignal.sym===sym?S.detailSignal:null);
  es('cvd-v',fN(cvd),null,cvd>=0?'var(--g)':'var(--r)');
  drawCVD(sym);

  // 因子
  const factors=[
    {n:'上涨动能',v:`${Math.round(ps)}/100`,bw:Math.min(100,ps),bc:'gf',vc:ps>=60?'fg':ps>=30?'fy':'fn',tip:ps>=70?'上涨力量很强':ps>=60?'上涨信号明显':ps>=30?'略偏强':'暂不明显'},
    {n:'下跌压力',v:`${Math.round(ds)}/100`,bw:Math.min(100,ds),bc:'rf2',vc:ds>=60?'fr':ds>=30?'fy':'fn',tip:ds>=70?'下跌压力很强':ds>=60?'回落风险偏高':ds>=30?'略偏弱':'暂不明显'},
    {n:'买卖盘失衡',v:`${obi>=0?'+':''}${obi.toFixed(1)}%`,bw:Math.min(100,Math.abs(obi)*2),bc:obi>=0?'gf':'rf2',vc:obi>10?'fg':obi<-10?'fr':'fn',tip:obi>20?'买盘明显压过卖盘':obi>10?'买盘偏多':obi<-20?'卖盘明显压过买盘':obi<-10?'卖盘偏多':'买卖比较均衡'},
    {n:'主动买入占比',v:`${tbr.toFixed(1)}%`,bw:tbr,bc:tbr>60?'gf':tbr<40?'rf2':'yf',vc:tbr>60?'fg':tbr<40?'fr':'fy',tip:tbr>70?'主动买入很强':tbr>60?'偏多':tbr<30?'主动卖出很强':'偏空'},
    {n:'主动买卖量差',v:fN(cvd),bw:Math.min(100,Math.abs(cvd)/500),bc:cvd>=0?'gf':'rf2',vc:cvd>0?'fg':'fr',tip:cvd>50000?'大量净流入':cvd>0?'净买入':cvd<-50000?'大量净流出':'净卖出'},
    {n:'挂单变化强度',v:fN(ofi),bw:Math.min(100,Math.abs(ofi)/100),bc:ofi>0?'gf':'rf2',vc:ofi>3000?'fg':ofi<-3000?'fr':'fn',tip:ofi>5000?'买方挂单明显增强':ofi>2000?'买方在持续加单':ofi<-5000?'卖方挂单明显增强':'买卖挂单较平衡'},
    {n:'买卖价差',v:`${(s.spread_bps/100).toFixed(2)}%`,bw:Math.min(100,s.spread_bps*3),bc:s.spread_bps<20?'gf':'yf',vc:s.spread_bps<10?'fg':s.spread_bps<30?'fy':'fn',tip:s.spread_bps<10?'成交环境很好':s.spread_bps<20?'正常':'价差偏大'},
    {n:'大户资金',v:s.whale_entry?'进场':s.whale_exit?'离场':'观望',bw:s.whale_entry?80:s.whale_exit?60:20,bc:s.whale_entry?'gf':s.whale_exit?'rf2':'yf',vc:s.whale_entry?'fg':s.whale_exit?'fr':'fn',tip:s.whale_entry?`大单占比${s.max_bid_ratio.toFixed(1)}%`:s.whale_exit?'大户有离场迹象':'暂无明显动作'},
    {n:'异常波动',v:`${s.anomaly_count_1m}次`,bw:Math.min(100,s.anomaly_count_1m),bc:s.anomaly_count_1m>50?'rf2':'yf',vc:s.anomaly_count_1m>100?'fr':s.anomaly_count_1m>50?'fy':'fn',tip:s.anomaly_count_1m>200?'波动非常剧烈':s.anomaly_count_1m>50?'波动偏多':'整体平稳'},
  ];
  document.getElementById('rf-list').innerHTML=factors.map(f=>`
    <div class="fi"><div class="fi-n">${f.n}</div>
      <div><div class="fi-bar"><div class="fi-f ${f.bc}" style="width:${f.bw}%"></div></div>
      <div class="fi-tip">${f.tip}</div></div>
      <div class="fi-v ${f.vc}">${f.v}</div></div>`).join('');
  renderEnterpriseMetrics(sym);

  // 大单
  const bigT=s.big_trades||[];e('bt-cnt',bigT.length);
  updateSoftStreamList('bt-list',bigT,{
    scopeKey:sym,
    getKey:bt=>`${bt.t}|${bt.p}|${bt.q}|${bt.buy?1:0}`,
    enterClass:'soft-trade-drop',
    emptyHtml:'<div class="empty-p">等待大单...</div>',
    renderItem:bt=>`<div class="bt-row"><span class="btdot ${bt.buy?'db':'ds'}"></span>
      <span class="bt-dir ${bt.buy?'btu':'btd'}">${bt.buy?'主动买':'主动卖'}</span>
      <span style="color:${bt.buy?'var(--g)':'var(--r)'}">${fP(bt.p,s)}</span>
      <span style="color:var(--y);font-weight:700;margin-left:auto">${fQ(bt.q,s)}</span>
      <span style="color:var(--t3);margin-left:5px">${typeof bt.t==='number'?new Date(bt.t).toLocaleTimeString('zh-CN',{hour12:false}):bt.t}</span>
    </div>`
  });
}

// ── 信号 ─────────────────────────────────────────────────────────
function renderSigs(){
  const sigs=[];const seen=new Set();
  S.feed.slice(0,40).forEach(f=>{const k=f.time+f.symbol+f.type;if(seen.has(k))return;seen.add(k);
    if(!withinWindow(f.time))return;
    sigs.push({time:f.time,sym:f.symbol.replace('USDT',''),full:f.symbol,type:f.type,score:f.score,desc:f.desc,fresh:sigs.length<2});});
  S.syms.forEach(s=>{
    const ps=sv(s.symbol,'ps'),ds=sv(s.symbol,'ds');
    if(ps>=70&&!sigs.find(x=>x.full===s.symbol&&x.type==='pump'))
      sigs.unshift({time:'实时',sym:s.symbol.replace('USDT',''),full:s.symbol,type:'pump',score:Math.round(ps),
        desc:(s.signal_reason||`评分${Math.round(ps)} 买卖盘失衡${sv(s.symbol,'obi').toFixed(1)}% 主动买入占比${sv(s.symbol,'tbr').toFixed(0)}%`),fresh:false});
    if(ds>=70&&!sigs.find(x=>x.full===s.symbol&&x.type==='dump'))
      sigs.unshift({time:'实时',sym:s.symbol.replace('USDT',''),full:s.symbol,type:'dump',score:Math.round(ds),
        desc:(s.signal_reason||`评分${Math.round(ds)} 买卖盘失衡${sv(s.symbol,'obi').toFixed(1)}%`),fresh:false});
  });
  e('sig-cnt',sigs.length);
  const lbl={pump:'🚀 上涨异动',dump:'📉 下跌异动',whale:'🐋 大户动向',anomaly:'⚠️ 异常波动',cvd:'📊 主动买卖量差'};
  updateSoftStreamList('sig-list',sigs.slice(0,20),{
    getKey:s=>`${s.time}|${s.full}|${s.type}|${s.desc}`,
    emptyHtml:'<div class="empty-p">📡<br>等待信号<br><span style="color:var(--t3)">系统会在有明显异动时提醒</span></div>',
    minAnimateInterval:5000,
    pauseOnHover:true,
    renderItem:(s,i)=>`
      <div class="scard ${s.type}" onclick="focusSignal('${s.full}','${(lbl[s.type]||s.type).replace(/'/g,'&#39;')}','${String(s.desc||'').replace(/'/g,'&#39;')}')">
        ${i===0?'<div class="sc-new">NEW</div>':''}
        <div class="sc-h"><span class="sc-sym">${s.sym}</span><span class="sc-t">${s.time}</span></div>
        <div class="sc-tag">${lbl[s.type]||s.type}</div>
        <div class="sc-desc">${s.desc}</div>
        ${s.score!=null?`<div class="sc-score">
          <div class="sc-score-bar"><div class="sc-score-fill" style="width:${Math.min(100,s.score)}%;background:${s.type==='pump'?'var(--g)':s.type==='dump'?'var(--r)':s.type==='whale'?'var(--b)':'var(--p)'}"></div></div>
          <span class="sc-score-v">${s.score}</span>
        </div>`:''}
      </div>`
  });
}

// ── 预警 ─────────────────────────────────────────────────────────
function checkAlerts(){
  const nextAlerts=[];
  S.syms.forEach(s=>{
    const ps=sv(s.symbol,'ps'),ds=sv(s.symbol,'ds'),sym=s.symbol.replace('USDT',''),t=nowT();
    if(signalWindow!=='all'&&!withinWindow(t))return;
    if(ps>=75){
      nextAlerts.push({type:'pump',sym,full:s.symbol,tag:'🚀 上涨异动',time:t,score:ps,
        desc:(s.signal_reason||`评分${Math.round(ps)}/100 买卖盘失衡${sv(s.symbol,'obi').toFixed(1)}%`)});
    }
    if(ds>=75){
      nextAlerts.push({type:'dump',sym,full:s.symbol,tag:'📉 下跌异动',time:t,score:ds,
        desc:(s.signal_reason||`评分${Math.round(ds)}/100 买卖盘失衡${sv(s.symbol,'obi').toFixed(1)}%`)});
    }
    if(s.whale_entry){
      nextAlerts.push({type:'whale',sym,full:s.symbol,tag:'🐋 大户资金进场',time:t,score:Math.max(ps,60),
        desc:(s.signal_reason||`大单${s.max_bid_ratio.toFixed(1)}% 主动买卖量差${fN(sv(s.symbol,'cvd'))}`)});
    }
    const cvd=sv(s.symbol,'cvd');
    if(Math.abs(cvd)>50000){
      nextAlerts.push({type:'cvd',sym,full:s.symbol,tag:cvd>0?'📈 主动买入占优':'📉 主动卖出占优',time:t,score:Math.max(Math.abs(cvd)/2000,55),
        desc:(s.signal_reason||`主动买卖量差${fN(cvd)} 主动买入占比${sv(s.symbol,'tbr').toFixed(0)}%`)});
    }
  });
  const alerts=latestOnlyItems(
    nextAlerts
      .sort((a,b)=>(Number(b.score)||0)-(Number(a.score)||0))
      .map((item,index)=>({...item,fresh:index===0})),
    item=>`${item.full}|${item.type}`,
    20
  ).filter(a=>withinWindow(a.time));
  S.alerts=alerts;
  e('al-cnt',alerts.length);
  updateSoftStreamList('al-list',alerts,{
    getKey:a=>`${a.time}|${a.full}|${a.type}|${a.desc}`,
    emptyHtml:'<div class="empty-p">🔔<br>等待预警<br><span style="color:var(--t3)">出现高风险变化时会在这里提示</span></div>',
    enterClass:'soft-stream-enter',
    minAnimateInterval:5000,
    pauseOnHover:true,
    renderItem:(a,i)=>`
      <div class="scard ${a.type}" onclick="focusSignal('${a.full}','${String(a.tag||'').replace(/'/g,'&#39;')}','${String(a.desc||'').replace(/'/g,'&#39;')}')">
        ${a.fresh&&i===0?'<div class="sc-new">NEW</div>':''}
        <span class="sc-x" onclick="event.stopPropagation();dismissAlert('${a.time}','${a.full}','${a.type}');">✕</span>
        <div class="sc-h"><span class="sc-sym">${a.sym}</span><span class="sc-t">${a.time}</span></div>
        <div class="sc-tag">${a.tag}</div>
        <div class="sc-desc">${a.desc}</div>
      </div>`
  });
}

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
function setHtmlIfChangedPaused(id,html,cacheKey,pauseTargetId=id){
  const el=document.getElementById(id);
  if(!el)return;
  const pauseTarget=document.getElementById(pauseTargetId)||el;
  if(pauseTarget.dataset.hoverFreezeBound!=='1'){
    pauseTarget.dataset.hoverFreezeBound='1';
    pauseTarget.addEventListener('mouseenter',()=>{pauseTarget.dataset.hoverFrozen='1';});
    pauseTarget.addEventListener('mouseleave',()=>{
      pauseTarget.dataset.hoverFrozen='0';
      const pending=pauseTarget.__pendingHtml instanceof Map ? [...pauseTarget.__pendingHtml.values()] : [];
      pauseTarget.__pendingHtml=null;
      pending.forEach(item=>setHtmlIfChangedPaused(item.id,item.html,item.cacheKey,item.pauseTargetId));
    });
  }
  if(pauseTarget.dataset.hoverFrozen==='1' || pauseTarget.matches(':hover')){
    pauseTarget.dataset.hoverFrozen='1';
    if(!(pauseTarget.__pendingHtml instanceof Map))pauseTarget.__pendingHtml=new Map();
    pauseTarget.__pendingHtml.set(cacheKey,{id,html,cacheKey,pauseTargetId});
    return;
  }
  if(S.ui[cacheKey]===html)return;
  S.ui[cacheKey]=html;
  el.innerHTML=html;
}
function latestOnlyItems(items,getKey,limit=20){
  const list=Array.isArray(items)?items:[];
  const seen=new Set();
  const next=[];
  list.forEach((item,index)=>{
    const key=String(getKey?getKey(item,index):index);
    if(!key || seen.has(key))return;
    seen.add(key);
    next.push(item);
  });
  return limit>0?next.slice(0,limit):next;
}
function compactSignalFeed(feed,limit=20){
  return latestOnlyItems(feed,item=>`${item?.symbol||''}|${item?.type||''}`,limit);
}
function wsNum(value){
  const num=Number(value);
  return Number.isFinite(num)?num:0;
}
function inflateWsFeedRows(rows){
  return (Array.isArray(rows)?rows:[]).map(row=>{
    if(!Array.isArray(row))return row||{};
    return {
      time:String(row[0]||''),
      symbol:String(row[1]||''),
      type:String(row[2]||''),
      score:row[3]==null?null:wsNum(row[3]),
      desc:String(row[4]||'')
    };
  });
}
function inflateWsSymbolRows(rows){
  return (Array.isArray(rows)?rows:[]).map(row=>{
    if(!Array.isArray(row))return row||{};
    return {
      symbol:String(row[0]||''),
      status_summary:String(row[1]||''),
      watch_level:String(row[2]||''),
      signal_reason:String(row[3]||''),
      bid:wsNum(row[4]),
      ask:wsNum(row[5]),
      mid:wsNum(row[6]),
      spread_bps:wsNum(row[7]),
      price_precision:wsNum(row[8]),
      quantity_precision:wsNum(row[9]),
      change_24h_pct:wsNum(row[10]),
      high_24h:wsNum(row[11]),
      low_24h:wsNum(row[12]),
      volume_24h:wsNum(row[13]),
      quote_vol_24h:wsNum(row[14]),
      ofi:wsNum(row[15]),
      ofi_raw:wsNum(row[16]),
      obi:wsNum(row[17]),
      cvd:wsNum(row[18]),
      taker_buy_ratio:wsNum(row[19]),
      pump_score:wsNum(row[20]),
      dump_score:wsNum(row[21]),
      pump_signal:!!row[22],
      dump_signal:!!row[23],
      whale_entry:!!row[24],
      whale_exit:!!row[25],
      bid_eating:!!row[26],
      total_bid_volume:wsNum(row[27]),
      total_ask_volume:wsNum(row[28]),
      max_bid_ratio:wsNum(row[29]),
      max_ask_ratio:wsNum(row[30]),
      anomaly_count_1m:wsNum(row[31]),
      anomaly_max_severity:wsNum(row[32]),
      update_count:wsNum(row[33])
    };
  });
}
function normalizeWsSnapshot(payload){
  if(!payload || payload.k!=='m1')return payload;
  return {
    symbols:inflateWsSymbolRows(payload.s),
    feed:inflateWsFeedRows(payload.f),
    trader:payload.t||{account_id:0,balances:[],open_orders:[],order_history:[],trade_history:[]},
    access:payload.a||S.access,
    total_updates:wsNum(payload.u),
    uptime_secs:wsNum(payload.up)
  };
}
const WS_TEXT_DECODER=typeof TextDecoder!=='undefined'?new TextDecoder():null;
function decodeWsUnsigned64(view,offset){
  const hi=view.getUint32(offset,false);
  const lo=view.getUint32(offset+4,false);
  return hi*4294967296+lo;
}
function decodeWsSigned64(view,offset){
  const hi=view.getInt32(offset,false);
  const lo=view.getUint32(offset+4,false);
  return hi*4294967296+lo;
}
function decodeWsUtf8(bytes,start,length){
  if(!WS_TEXT_DECODER)throw new Error('TextDecoder unavailable');
  return WS_TEXT_DECODER.decode(bytes.subarray(start,start+length));
}
function decodeMsgPack(data){
  const bytes=data instanceof Uint8Array?data:new Uint8Array(data);
  const view=new DataView(bytes.buffer,bytes.byteOffset,bytes.byteLength);
  let offset=0;
  const readLen=type=>{
    if(type===0xdc){const len=view.getUint16(offset,false);offset+=2;return len;}
    if(type===0xdd){const len=view.getUint32(offset,false);offset+=4;return len;}
    if(type===0xde){const len=view.getUint16(offset,false);offset+=2;return len;}
    if(type===0xdf){const len=view.getUint32(offset,false);offset+=4;return len;}
    return 0;
  };
  const readArray=len=>{
    const arr=new Array(len);
    for(let i=0;i<len;i++)arr[i]=readValue();
    return arr;
  };
  const readMap=len=>{
    const obj={};
    for(let i=0;i<len;i++){
      const key=readValue();
      obj[String(key)]=readValue();
    }
    return obj;
  };
  const readValue=()=>{
    const type=view.getUint8(offset++);
    if(type<=0x7f)return type;
    if(type>=0xe0)return type-0x100;
    if(type>=0xa0 && type<=0xbf){
      const len=type&0x1f;
      const value=decodeWsUtf8(bytes,offset,len);
      offset+=len;
      return value;
    }
    if(type>=0x90 && type<=0x9f)return readArray(type&0x0f);
    if(type>=0x80 && type<=0x8f)return readMap(type&0x0f);
    switch(type){
      case 0xc0:return null;
      case 0xc2:return false;
      case 0xc3:return true;
      case 0xca:{const value=view.getFloat32(offset,false);offset+=4;return value;}
      case 0xcb:{const value=view.getFloat64(offset,false);offset+=8;return value;}
      case 0xcc:{const value=view.getUint8(offset);offset+=1;return value;}
      case 0xcd:{const value=view.getUint16(offset,false);offset+=2;return value;}
      case 0xce:{const value=view.getUint32(offset,false);offset+=4;return value;}
      case 0xcf:{const value=decodeWsUnsigned64(view,offset);offset+=8;return value;}
      case 0xd0:{const value=view.getInt8(offset);offset+=1;return value;}
      case 0xd1:{const value=view.getInt16(offset,false);offset+=2;return value;}
      case 0xd2:{const value=view.getInt32(offset,false);offset+=4;return value;}
      case 0xd3:{const value=decodeWsSigned64(view,offset);offset+=8;return value;}
      case 0xd9:{const len=view.getUint8(offset);offset+=1;const value=decodeWsUtf8(bytes,offset,len);offset+=len;return value;}
      case 0xda:{const len=view.getUint16(offset,false);offset+=2;const value=decodeWsUtf8(bytes,offset,len);offset+=len;return value;}
      case 0xdb:{const len=view.getUint32(offset,false);offset+=4;const value=decodeWsUtf8(bytes,offset,len);offset+=len;return value;}
      case 0xc4:{const len=view.getUint8(offset);offset+=1;const value=bytes.slice(offset,offset+len);offset+=len;return value;}
      case 0xc5:{const len=view.getUint16(offset,false);offset+=2;const value=bytes.slice(offset,offset+len);offset+=len;return value;}
      case 0xc6:{const len=view.getUint32(offset,false);offset+=4;const value=bytes.slice(offset,offset+len);offset+=len;return value;}
      case 0xdc:
      case 0xdd:return readArray(readLen(type));
      case 0xde:
      case 0xdf:return readMap(readLen(type));
      default:throw new Error(`Unsupported MessagePack type: ${type}`);
    }
  };
  return readValue();
}
function decodeWsPayload(data){
  if(typeof data==='string')return JSON.parse(data);
  if(data instanceof ArrayBuffer)return decodeMsgPack(data);
  if(ArrayBuffer.isView(data))return decodeMsgPack(new Uint8Array(data.buffer,data.byteOffset,data.byteLength));
  throw new Error('Unsupported WebSocket payload type');
}
const BOTTOM_STRIP_REFRESH_MS=10000;
function runBottomStripRefresh(){
  window.__bbBottomStripTimer=null;
  window.__bbBottomStripRenderedAt=Date.now();
  renderPairMini();
  renderTicker();
}
function scheduleBottomStripRefresh(force=false){
  if(force || !Number(window.__bbBottomStripRenderedAt||0)){
    runBottomStripRefresh();
    return;
  }
  const elapsed=Date.now()-Number(window.__bbBottomStripRenderedAt||0);
  if(elapsed>=BOTTOM_STRIP_REFRESH_MS){
    runBottomStripRefresh();
    return;
  }
  if(window.__bbBottomStripTimer)return;
  window.__bbBottomStripTimer=setTimeout(runBottomStripRefresh,BOTTOM_STRIP_REFRESH_MS-elapsed);
}
const SLOW_STREAM_MIN_MS=5000;
const SLOW_STREAM_MAX_MS=15000;
const SLOW_PANEL_TASKS={
  market:{timerKey:'__bbSlowMarketTimer',run:()=>renderPairList()},
  signals:{timerKey:'__bbSlowSignalsTimer',run:()=>renderSigs()},
  alerts:{timerKey:'__bbSlowAlertsTimer',run:()=>checkAlerts()}
};
function randomSlowPanelDelay(){
  return Math.round(SLOW_STREAM_MIN_MS+Math.random()*(SLOW_STREAM_MAX_MS-SLOW_STREAM_MIN_MS));
}
function runSlowPanelTask(name){
  const task=SLOW_PANEL_TASKS[name];
  if(!task)return;
  const timerKey=task.timerKey;
  if(window[timerKey]){
    clearTimeout(window[timerKey]);
    window[timerKey]=null;
  }
  window[`__bbSlowPanelRenderedAt_${name}`]=Date.now();
  task.run();
  scheduleSlowPanelTask(name);
}
function scheduleSlowPanelTask(name,force=false){
  const task=SLOW_PANEL_TASKS[name];
  if(!task)return;
  const timerKey=task.timerKey;
  if(force || !Number(window[`__bbSlowPanelRenderedAt_${name}`]||0)){
    runSlowPanelTask(name);
    return;
  }
  if(window[timerKey])return;
  const delay=randomSlowPanelDelay();
  window[timerKey]=setTimeout(()=>runSlowPanelTask(name),delay);
}
function runSlowPanelsRefresh(){
  runSlowPanelTask('market');
  runSlowPanelTask('signals');
  runSlowPanelTask('alerts');
}
function scheduleSlowPanelsRefresh(force=false){
  scheduleSlowPanelTask('market',force);
  scheduleSlowPanelTask('signals',force);
  scheduleSlowPanelTask('alerts',force);
}
function htmlToElement(html){
  const tpl=document.createElement('template');
  tpl.innerHTML=String(html||'').trim();
  return tpl.content.firstElementChild;
}
function animateSoftListMoves(container,prevRects){
  const nodes=[...container.children].filter(node=>node.dataset.softKey);
  nodes.forEach(node=>{
    const prevRect=prevRects.get(node.dataset.softKey);
    if(!prevRect)return;
    const nextRect=node.getBoundingClientRect();
    const deltaY=prevRect.top-nextRect.top;
    if(Math.abs(deltaY)<1)return;
    node.style.transition='none';
    node.style.transform=`translateY(${deltaY}px)`;
    node.style.opacity='.985';
    requestAnimationFrame(()=>{
      node.style.transition='transform 1.1s var(--ease-premium), opacity .7s var(--ease-premium)';
      node.style.transform='translateY(0)';
      node.style.opacity='1';
      setTimeout(()=>{
        node.style.transition='';
        node.style.transform='';
        node.style.opacity='';
      },1120);
    });
  });
}
function addSoftClass(el,className,duration=720,restart=false){
  if(!el||!className)return;
  if(restart){
    el.classList.remove(className);
    void el.offsetWidth;
  }
  el.classList.add(className);
  setTimeout(()=>el.classList.remove(className),duration);
}
function createOrderBookRow(row,side,ctx){
  const rowEl=document.createElement('div');
  rowEl.className='ob-row';
  rowEl.dataset.softKey=String(row.p);
  rowEl.innerHTML=`<div class="ob-bg ${side==='bid'?'bgb':'bga'}"></div>
    <span class="ob-p ${side==='bid'?'bp':'ap'} clickable"></span>
    <span class="ob-q"></span>
    <span class="ob-c"></span>`;
  const priceEl=rowEl.querySelector('.ob-p');
  priceEl.addEventListener('click',()=>applyBookPrice(side,row.p));
  patchOrderBookRow(rowEl,row,side,ctx,{isNew:true,force:true});
  return rowEl;
}
function patchOrderBookRow(rowEl,row,side,ctx,{isNew=false,force=false}={}){
  if(!rowEl)return;
  const sig=`${row.q}|${row.cum}|${row.width}`;
  const changed=force || rowEl.dataset.rowSig!==sig;
  rowEl.dataset.rowSig=sig;
  rowEl.dataset.softKey=String(row.p);
  const bgEl=rowEl.querySelector('.ob-bg');
  const priceEl=rowEl.querySelector('.ob-p');
  const qtyEl=rowEl.querySelector('.ob-q');
  const cumEl=rowEl.querySelector('.ob-c');
  if(priceEl){
    priceEl.textContent=fP(row.p,ctx);
    priceEl.className=`ob-p ${side==='bid'?'bp':'ap'} clickable`;
  }
  if(qtyEl)qtyEl.textContent=fBookNum(row.q);
  if(cumEl)cumEl.textContent=fBookNum(row.cum);
  if(bgEl)bgEl.style.width=`${row.width}%`;
  if(isNew){
    addSoftClass(rowEl,side==='bid'?'soft-depth-enter-bid':'soft-depth-enter-ask',520);
  }else if(changed){
    addSoftClass(rowEl,side==='bid'?'soft-depth-bid':'soft-depth-ask',480,true);
  }
}
function updateOrderBookSide(containerId,rows,side,ctx,scopeKey=''){
  const container=document.getElementById(containerId);
  if(!container)return;
  const list=Array.isArray(rows)?rows:[];
  const scopeValue=String(scopeKey||'');
  const scopeChanged=container.dataset.softScope!==scopeValue;
  const canAnimate=container.dataset.softLive==='1' && !scopeChanged;
  const prevRects=canAnimate
    ?new Map([...container.children]
      .filter(node=>node.dataset.softKey)
      .map(node=>[node.dataset.softKey,node.getBoundingClientRect()]))
    :new Map();
  const prevNodes=scopeChanged
    ?new Map()
    :new Map([...container.children]
      .filter(node=>node.dataset.softKey)
      .map(node=>[node.dataset.softKey,node]));
  if(!list.length){
    container.replaceChildren();
    container.dataset.softScope=scopeValue;
    delete container.dataset.softLive;
    return;
  }
  const frag=document.createDocumentFragment();
  list.forEach(row=>{
    const key=String(row.p);
    let rowEl=prevNodes.get(key);
    if(rowEl){
      patchOrderBookRow(rowEl,row,side,ctx);
    }else{
      rowEl=createOrderBookRow(row,side,ctx);
    }
    frag.appendChild(rowEl);
  });
  container.replaceChildren(frag);
  container.dataset.softScope=scopeValue;
  container.dataset.softLive='1';
  if(canAnimate)animateSoftListMoves(container,prevRects);
}
function updateSoftStreamList(containerId,items,options={}){
  const container=document.getElementById(containerId);
  if(!container)return;
  const {
    getKey,
    renderItem,
    emptyHtml='',
    getSignature,
    enterClass='soft-stream-enter',
    updateClass='',
    scopeKey=null,
    minAnimateInterval=0,
    pauseOnHover=false
  }=options;
  if(pauseOnHover){
    if(container.dataset.softPauseBound!=='1'){
      container.dataset.softPauseBound='1';
      container.addEventListener('mouseenter',()=>{
        container.dataset.softPaused='1';
      });
      container.addEventListener('mouseleave',()=>{
        container.dataset.softPaused='0';
        const pending=container.__softPending;
        if(pending){
          container.__softPending=null;
          updateSoftStreamList(container.id,pending.items,pending.options);
        }
      });
    }
    if(container.dataset.softPaused==='1' || container.matches(':hover')){
      container.dataset.softPaused='1';
      container.__softPending={items:Array.isArray(items)?items:[],options:{...options}};
      return;
    }
  }
  const list=Array.isArray(items)?items:[];
  const scopeValue=scopeKey==null?'':String(scopeKey);
  const scopeChanged=scopeKey!=null && container.dataset.softScope!==scopeValue;
  const now=Date.now();
  const lastAnimateAt=Number(container.dataset.softLastAnimateAt||0);
  const canAnimate=container.dataset.softLive==='1' && !scopeChanged;
  const allowAnimate=canAnimate && (!minAnimateInterval || now-lastAnimateAt>=minAnimateInterval);
  const prevRects=canAnimate
    ?new Map([...container.children]
      .filter(node=>node.dataset.softKey)
      .map(node=>[node.dataset.softKey,node.getBoundingClientRect()]))
    :new Map();
  const prevNodes=scopeChanged
    ?new Map()
    :new Map([...container.children]
      .filter(node=>node.dataset.softKey)
      .map(node=>[node.dataset.softKey,node]));

  if(!list.length){
    if(container.dataset.softEmptyHtml!==emptyHtml){
      container.innerHTML=emptyHtml;
      container.dataset.softEmptyHtml=emptyHtml;
    }
    if(scopeKey!=null)container.dataset.softScope=scopeValue;
    delete container.dataset.softLive;
    return;
  }

  const frag=document.createDocumentFragment();
  const seen=new Set();
  list.forEach((item,index)=>{
    const rawKey=getKey?getKey(item,index):index;
    const key=String(rawKey);
    if(!key||seen.has(key))return;
    seen.add(key);

    const html=renderItem(item,index);
    const sig=String(getSignature?getSignature(item,index,html):String(html||'').trim());
    const existing=prevNodes.get(key);
    let node=existing;

    if(!existing || existing.dataset.renderSig!==sig){
      node=htmlToElement(html);
      if(!node)return;
      node.dataset.softKey=key;
      node.dataset.renderSig=sig;
      if(allowAnimate && !existing){
        addSoftClass(node,enterClass,700);
      }else if(allowAnimate && existing){
        const cls=typeof updateClass==='function'?updateClass(item,index,existing):updateClass;
        if(cls)addSoftClass(node,cls,560);
      }
    }else{
      node.dataset.softKey=key;
      node.dataset.renderSig=sig;
    }

    frag.appendChild(node);
  });

  container.replaceChildren(frag);
  container.dataset.softEmptyHtml='';
  container.dataset.softLive='1';
  if(scopeKey!=null)container.dataset.softScope=scopeValue;
  if(allowAnimate){
    container.dataset.softLastAnimateAt=String(now);
    animateSoftListMoves(container,prevRects);
  }
}
function numericFromText(text){
  const match=String(text??'').replace(/,/g,'').match(/-?\d+(?:\.\d+)?/);
  return match?parseFloat(match[0]):NaN;
}
function inferSoftPulseKind(prevText,nextText,fallback='neutral'){
  const prev=numericFromText(prevText);
  const next=numericFromText(nextText);
  if(Number.isFinite(prev)&&Number.isFinite(next)){
    if(next>prev)return 'up';
    if(next<prev)return 'dn';
  }
  return fallback;
}
function pulseSoftValue(el,kind='neutral',effect='pulse'){
  const className=effect==='sheen'
    ?(kind==='up'
      ?'soft-quote-sheen-up'
      :kind==='dn'
        ?'soft-quote-sheen-dn'
        :'soft-quote-sheen-neutral')
    :(kind==='up'
      ?'soft-pulse-up'
      :kind==='dn'
        ?'soft-pulse-dn'
        :'soft-pulse-neutral');
  addSoftClass(el,className,effect==='sheen'?780:560,true);
}
function setSoftValue(target,text,options={}){
  const el=typeof target==='string'?document.getElementById(target):target;
  if(!el)return;
  const {
    cls,
    color,
    styleText,
    pulse=true,
    kind,
    fallbackKind='neutral',
    effect='pulse'
  }=options;
  const nextText=String(text??'');
  const prevText=el.dataset.softValue ?? el.textContent ?? '';
  if(cls!=null)el.className=cls;
  el.textContent=nextText;
  if(styleText!=null){
    el.style.cssText=styleText;
  }else if(color!=null){
    el.style.color=color;
  }
  if(pulse && prevText!==nextText){
    pulseSoftValue(el,kind||inferSoftPulseKind(prevText,nextText,fallbackKind),effect);
  }
  el.dataset.softValue=nextText;
}
function inferPricePrecision(v){
  const n=Math.abs(+v||0);
  return n>=1000?1:n>=10?2:n>=1?3:n>=.1?4:6;
}
function getPricePrecision(ctx=null,v=0){
  if(typeof ctx==='number'&&Number.isFinite(ctx))return Math.max(0,Math.min(12,Math.round(ctx)));
  let state=null;
  if(typeof ctx==='string'&&ctx){
    state=typeof getSymbolState==='function'?getSymbolState(ctx):null;
  }else if(ctx&&typeof ctx==='object'){
    if(Number.isFinite(ctx.price_precision))return Math.max(0,Math.min(12,Math.round(ctx.price_precision)));
    if(ctx.symbol&&typeof getSymbolState==='function')state=getSymbolState(ctx.symbol);
  }else if(S?.sel&&typeof getSymbolState==='function'){
    state=getSymbolState(S.sel);
  }
  const precision=state?.price_precision;
  if(Number.isFinite(precision))return Math.max(0,Math.min(12,Math.round(precision)));
  return inferPricePrecision(v);
}
function fP(p,ctx=null){
  const v=+p;
  if(!Number.isFinite(v))return '--';
  return trimDecimalString(v.toFixed(getPricePrecision(ctx,v)));
}
function inferQtyPrecision(v){
  const n=Math.abs(+v||0);
  if(n===0)return 0;
  return n>=1000?0:n>=100?2:n>=1?4:8;
}
function getQtyPrecision(ctx=null,v=0){
  let state=null;
  if(typeof ctx==='string'&&ctx){
    state=typeof getSymbolState==='function'?getSymbolState(ctx):null;
  }else if(ctx&&typeof ctx==='object'){
    if(Number.isFinite(ctx.quantity_precision))return Math.max(0,Math.min(12,Math.round(ctx.quantity_precision)));
    if(ctx.symbol&&typeof getSymbolState==='function')state=getSymbolState(ctx.symbol);
  }else if(S?.sel&&typeof getSymbolState==='function'){
    state=getSymbolState(S.sel);
  }
  const precision=state?.quantity_precision;
  if(Number.isFinite(precision))return Math.max(0,Math.min(12,Math.round(precision)));
  return inferQtyPrecision(v);
}
function fQ(v,ctx=null){
  const n=+v;
  if(!Number.isFinite(n))return '--';
  return trimDecimalString(n.toFixed(getQtyPrecision(ctx,n)));
}
function fBookNum(v){
  const n=+v;
  if(!Number.isFinite(n))return '--';
  const abs=Math.abs(n);
  if(abs>=1e12)return trimDecimalString((n/1e12).toFixed(2))+'T';
  if(abs>=1e9)return trimDecimalString((n/1e9).toFixed(2))+'B';
  if(abs>=1e6)return trimDecimalString((n/1e6).toFixed(2))+'M';
  if(abs>=1e3)return trimDecimalString((n/1e3).toFixed(2))+'K';
  return fQ(n);
}
function trimDecimalString(text){
  return String(text)
    .replace(/(\.\d*?[1-9])0+$/,'$1')
    .replace(/\.0+$/,'')
    .replace(/\.$/,'');
}
function fN(n){const v=+n;return Math.abs(v)>=1e9?(v/1e9).toFixed(1)+'B':Math.abs(v)>=1e6?(v/1e6).toFixed(1)+'M':Math.abs(v)>=1e3?(v/1e3).toFixed(1)+'K':v.toFixed(0);}
function fNum(n){const v=+n;return Math.abs(v)>=1000?fN(v):v.toFixed(v>=1?4:8).replace(/0+$/,'').replace(/\.$/,'');}
function nowT(){return new Date().toLocaleTimeString('zh-CN',{hour12:false,hour:'2-digit',minute:'2-digit',second:'2-digit'});}
function fmtSym(sym){return sym?sym.replace('USDT','/USDT'):'--';}
function filledPct(order){const q=+order.quantity||0;if(q<=0)return 0;return Math.round((+order.filled_qty||0)/q*100);}
function getBalance(asset){return (S.trader.balances||[]).find(b=>b.asset===asset)||{available:0,locked:0};}
function sideLabel(side){return String(side||'').toUpperCase()==='BUY'?'买':'卖';}
function sideColor(side){return String(side||'').toUpperCase()==='BUY'?'var(--g)':'var(--r)';}
function hasActiveSelectedSymbolStream(){
  const ws=window.__bbSelectedSymbolWs;
  return !!(ws && ws.readyState===WebSocket.OPEN && window.__bbSelectedSymbolWsSymbol===S.sel);
}
const DETAIL_WS_FORCE_SYNC_MS=4500;
const DETAIL_WS_STUCK_RECONNECT_MS=12000;
function selectedSymbolPollMs(){
  if(document.hidden)return 12000;
  return hasActiveSelectedSymbolStream()?DETAIL_WS_FORCE_SYNC_MS:2200;
}
function scheduleSelectedSymbolPoll(delay=selectedSymbolPollMs()){
  if(window.__bbSelectedSymbolPoller)clearTimeout(window.__bbSelectedSymbolPoller);
  window.__bbSelectedSymbolPoller=setTimeout(runSelectedSymbolPoll,delay);
}
function flushSelectedDetailRender(){
  window.__bbDetailRenderTimer=null;
  const sym=window.__bbDetailRenderPendingSym;
  if(!sym || sym!==S.sel)return;
  window.__bbDetailRenderAt=Date.now();
  renderDetail(sym);
  updOHLCV();
}
function scheduleSelectedDetailRender(sym,immediate=false){
  if(!sym || sym!==S.sel)return;
  window.__bbDetailRenderPendingSym=sym;
  if(immediate){
    if(window.__bbDetailRenderTimer){
      clearTimeout(window.__bbDetailRenderTimer);
      window.__bbDetailRenderTimer=null;
    }
    flushSelectedDetailRender();
    return;
  }
  if(window.__bbDetailRenderTimer)return;
  const elapsed=Date.now()-(window.__bbDetailRenderAt||0);
  const wait=Math.max(0,220-elapsed);
  window.__bbDetailRenderTimer=setTimeout(flushSelectedDetailRender,wait);
}
async function runSelectedSymbolPoll(){
  if(window.__bbSelectedSymbolDetailBusy){
    scheduleSelectedSymbolPoll(160);
    return;
  }
  if(!S.sel){
    scheduleSelectedSymbolPoll();
    return;
  }
  window.__bbSelectedSymbolDetailBusy=true;
  try{
    await loadSymbolDetail(S.sel,true);
  }finally{
    window.__bbSelectedSymbolDetailBusy=false;
    scheduleSelectedSymbolPoll();
  }
}
function buildSelectedSymbolWsUrl(sym){
  const proto=location.protocol==='https:'?'wss':'ws';
  return `${proto}://${location.host}/ws/symbol/${encodeURIComponent(sym)}`;
}
function detailStreamSignature(detail){
  if(!detail||!detail.symbol)return '';
  return JSON.stringify({
    sym:detail.symbol,
    uc:wsNum(detail.update_count),
    bid:wsNum(detail.bid),
    ask:wsNum(detail.ask),
    tb:wsNum(detail.total_bid_volume),
    ta:wsNum(detail.total_ask_volume),
    sb:wsNum(detail.spread_bps),
    rt:(detail.recent_trades||[]).slice(0,20).map(t=>[t.t,t.p,t.q,t.buy]),
    bids:(detail.top_bids||[]).slice(0,12),
    asks:(detail.top_asks||[]).slice(0,12)
  });
}
function connectSelectedSymbolStream(sym=S.sel){
  if(!sym)return;
  const current=window.__bbSelectedSymbolWs;
  const currentSymbol=window.__bbSelectedSymbolWsSymbol;
  if(current && currentSymbol===sym && (current.readyState===WebSocket.OPEN || current.readyState===WebSocket.CONNECTING)){
    return;
  }
  if(window.__bbSelectedSymbolWsRetry){
    clearTimeout(window.__bbSelectedSymbolWsRetry);
    window.__bbSelectedSymbolWsRetry=null;
  }
  if(current){
    try{
      current.onclose=null;
      current.close();
    }catch(_){}
  }
  const ws=new WebSocket(buildSelectedSymbolWsUrl(sym));
  ws.binaryType='arraybuffer';
  window.__bbSelectedSymbolWs=ws;
  window.__bbSelectedSymbolWsSymbol=sym;
  ws.onopen=()=>{
    window.__bbSelectedDetailSig='';
    window.__bbSelectedDetailSigAt=0;
    window.__bbSelectedDetailLagSince=0;
  };
  ws.onmessage=ev=>{
    try{
      const detail=decodeWsPayload(ev.data);
      if(!detail || detail.symbol!==S.sel)return;
      const now=Date.now();
      const prev=getSymbolState(detail.symbol);
      const prevUpdate=Number(prev?.update_count||0);
      const nextUpdate=Number(detail.update_count||0);
      const sig=detailStreamSignature(detail);
      if(sig===window.__bbSelectedDetailSig){
        if(!window.__bbSelectedDetailSigAt)window.__bbSelectedDetailSigAt=now;
      }else{
        window.__bbSelectedDetailSig=sig;
        window.__bbSelectedDetailSigAt=now;
      }
      if(prevUpdate>0 && nextUpdate>0 && nextUpdate+2<prevUpdate){
        if(!window.__bbSelectedDetailLagSince)window.__bbSelectedDetailLagSince=now;
      }else{
        window.__bbSelectedDetailLagSince=0;
      }
      upsertSymbolDetail(detail,{markDetailFresh:true});
      S.tr[detail.symbol]=(detail.recent_trades||[]).map(t=>({
        p:t.p,
        q:t.q,
        buy:!!t.buy,
        t:typeof t.t==='number'?new Date(t.t).toLocaleTimeString('zh-CN',{hour12:false}):String(t.t||'--')
      }));
      ensureMetricHistory(detail.symbol,getSymbolState(detail.symbol)||detail);
      if(!S.cvdH[detail.symbol])S.cvdH[detail.symbol]=[];
      S.cvdH[detail.symbol].push({t:nowT(),v:sv(detail.symbol,'cvd')});
      if(S.cvdH[detail.symbol].length>HL)S.cvdH[detail.symbol].shift();
      S.ui.detailKey='';
      scheduleSelectedDetailRender(detail.symbol,false);
      if(
        (window.__bbSelectedDetailSigAt && now-window.__bbSelectedDetailSigAt>=DETAIL_WS_STUCK_RECONNECT_MS)
        || (window.__bbSelectedDetailLagSince && now-window.__bbSelectedDetailLagSince>=DETAIL_WS_STUCK_RECONNECT_MS)
      ){
        window.__bbSelectedDetailSig='';
        window.__bbSelectedDetailSigAt=0;
        window.__bbSelectedDetailLagSince=0;
        try{ws.close();}catch(_){}
      }
    }catch(_){}
  };
  ws.onclose=()=>{
    if(window.__bbSelectedSymbolWs!==ws)return;
    window.__bbSelectedSymbolWs=null;
    window.__bbSelectedDetailSig='';
    window.__bbSelectedDetailSigAt=0;
    window.__bbSelectedDetailLagSince=0;
    if(window.__bbSelectedSymbolWsSymbol!==sym)return;
    window.__bbSelectedSymbolWsRetry=setTimeout(()=>{
      if(S.sel===sym)connectSelectedSymbolStream(sym);
    },800);
  };
}
function selectedDetailKey(sym){
  const s=getSymbolState(sym);if(!s)return '';
  const quoteBal=getBalance('USDT');
  const baseBal=getBalance(sym.replace('USDT',''));
  return JSON.stringify({
    sym,
    uc:s.update_count||0,
    summary:s.status_summary,
    level:s.watch_level,
    reason:s.signal_reason,
    mid:sv(sym,'mid'),
    bid:s.bid,ask:s.ask,chg:s.change_24h_pct,cvd:sv(sym,'cvd'),ps:sv(sym,'ps'),ds:sv(sym,'ds'),
    hi:s.high_24h,lo:s.low_24h,vol:s.volume_24h,qv:s.quote_vol_24h,
    obi:sv(sym,'obi'),ofi:sv(sym,'ofi'),tbr:sv(sym,'tbr'),
    tb:s.total_bid_volume,ta:s.total_ask_volume,sb:s.spread_bps,
    bb:(s.big_trades||[]).slice(0,10).map(t=>[t.t,t.p,t.q,t.buy]),
    rt:(s.recent_trades||[]).slice(0,20).map(t=>[t.t,t.p,t.q,t.buy]),
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
function updateTradeSliderTip(side,value){
  const input=document.getElementById(`${side}-pct`);
  const tip=document.getElementById(`${side}-pct-tip`);
  if(!input||!tip)return;
  const min=Number(input.min||0);
  const max=Number(input.max||100);
  const current=Number(value ?? input.value ?? 0);
  const ratio=max===min?0:(current-min)/(max-min);
  const pct=Math.max(0,Math.min(100,ratio*100));
  input.parentElement?.style.setProperty('--pct',`${pct}%`);
  tip.textContent=`${Math.round(current)}%`;
}
function bindTradeSliderTips(){
  ['buy','sell'].forEach(side=>{
    const input=document.getElementById(`${side}-pct`);
    if(!input||input.dataset.tipBound==='1')return;
    input.dataset.tipBound='1';
    const sync=()=>updateTradeSliderTip(side,input.value);
    input.addEventListener('input',sync);
    input.addEventListener('change',sync);
    sync();
  });
}
function bindTradeButtonFx(){
  document.querySelectorAll('.ta-btn').forEach(btn=>{
    if(btn.dataset.fxBound==='1')return;
    btn.dataset.fxBound='1';
    btn.addEventListener('pointerenter',()=>btn.classList.add('is-hovered'));
    btn.addEventListener('pointerleave',()=>{
      btn.classList.remove('is-hovered');
      btn.classList.remove('is-pressed');
      btn.style.setProperty('--mx','50%');
      btn.style.setProperty('--my','50%');
    });
    btn.addEventListener('pointermove',ev=>{
      const rect=btn.getBoundingClientRect();
      const x=((ev.clientX-rect.left)/Math.max(rect.width,1))*100;
      const y=((ev.clientY-rect.top)/Math.max(rect.height,1))*100;
      btn.style.setProperty('--mx',`${x.toFixed(2)}%`);
      btn.style.setProperty('--my',`${y.toFixed(2)}%`);
    });
    btn.addEventListener('pointerdown',ev=>{
      const rect=btn.getBoundingClientRect();
      const x=((ev.clientX-rect.left)/Math.max(rect.width,1))*100;
      const y=((ev.clientY-rect.top)/Math.max(rect.height,1))*100;
      btn.style.setProperty('--mx',`${x.toFixed(2)}%`);
      btn.style.setProperty('--my',`${y.toFixed(2)}%`);
      btn.classList.add('is-pressed');
      const pulse=document.createElement('span');
      pulse.className='ta-btn-ripple';
      pulse.style.setProperty('--x',`${x.toFixed(2)}%`);
      pulse.style.setProperty('--y',`${y.toFixed(2)}%`);
      btn.appendChild(pulse);
      window.setTimeout(()=>pulse.remove(),760);
    });
    btn.addEventListener('pointerup',()=>btn.classList.remove('is-pressed'));
    btn.addEventListener('pointercancel',()=>btn.classList.remove('is-pressed'));
  });
}
setInterval(()=>{e('htime',new Date().toLocaleTimeString('zh-CN',{hour12:false}));},1000);
window.addEventListener('resize',()=>{if(S.sel)drawCVD(S.sel);});

// ── WebSocket ─────────────────────────────────────────────────────
function connect(){
  const proto=location.protocol==='https:'?'wss':'ws';
  const ws=new WebSocket(`${proto}://${location.host}/ws`);
  ws.binaryType='arraybuffer';
  ws.onopen=()=>{document.getElementById('wdot').className='wdot live';e('wlbl','实时连接');};
  ws.onmessage=ev=>{try{render(normalizeWsSnapshot(decodeWsPayload(ev.data)));}catch(_){ }};
  ws.onerror=()=>{document.getElementById('wdot').className='wdot';e('wlbl','连接异常');};
  ws.onclose=()=>{document.getElementById('wdot').className='wdot';e('wlbl','重连中...');setTimeout(connect,2000);};
}
window.addEventListener('DOMContentLoaded',()=>{
  window.__bbBootAt=Date.now();
  loadViewPrefs();
  syncViewControls();
  bindTradeButtonFx();
  bindTradeSliderTips();
  document.getElementById('buy-pct').oninput=e=>setBuyPct(e.target.value);
  document.getElementById('sell-pct').oninput=e=>setSellPct(e.target.value);
  ['buy-price','buy-qty','sell-price','sell-qty'].forEach(id=>{
    const el=document.getElementById(id);
    if(el) el.addEventListener('input',()=>updateTotals());
  });
  renderOrders();
  if(typeof ensureTradingView==='function'){
    ensureTradingView().catch(()=>{});
  }
  fetch('/api/state').then(r=>r.json()).then(d=>{
    render(normalizeWsSnapshot(d));
    if(S.sel)connectSelectedSymbolStream(S.sel);
  }).catch(()=>{});
  connect();
  scheduleSelectedSymbolPoll(1200);
});
document.addEventListener('visibilitychange',()=>{
  scheduleSelectedSymbolPoll(800);
  if(S.sel)connectSelectedSymbolStream(S.sel);
});
