// ── 主渲染 ───────────────────────────────────────────────────────
function render(data){
  const prevSelected=getSymbolState(S.sel);
  S.syms=mergeSymbols(data.symbols||[]);S.feed=data.feed||[];
  S.access=data.access||S.access;
  S.trader=data.trader||{account_id:0,balances:[],open_orders:[],order_history:[],trade_history:[]};
  if(typeof applyAccessState==='function'){
    // 访问控制信息以后端快照为准，避免 WS / /api/state 更新时把套餐状态冲掉。
    applyAccessState({...S.access,user:S.auth.user},S.auth.user);
  }

  S.syms.forEach(s=>{
    ema(s.symbol,'obi',s.obi||0);ema(s.symbol,'ps',s.pump_score||0);
    ema(s.symbol,'ds',s.dump_score||0);ema(s.symbol,'ofi',s.ofi||0);ema(s.symbol,'mid',s.mid||0);
    if(!S.sm[s.symbol])S.sm[s.symbol]={};
    S.sm[s.symbol].cvd=s.cvd||0;S.sm[s.symbol].tbr=s.taker_buy_ratio||50;
    ensureMetricHistory(s.symbol,s);
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
  updateSignalPerfStats();

  renderPairList();renderPairMini();renderTicker();renderSigs();checkAlerts();

  let activeSymbol=S.sel;
  if(activeSymbol){
    const selected=getSymbolState(activeSymbol);
    if(selected){
      S.selectedCache=selected;
    }else{
      // 刷新后本地可能保留了一个当前不可见/无权限的旧币种。
      // 这时必须回退到本次快照里真实可用的 symbol，否则右侧分析面板会一直是空态。
      activeSymbol=null;
      S.sel=null;
      S.selectedCache=null;
      S.detailSignal=null;
      S.ui.detailKey='';
    }
  }
  const cur=activeSymbol||(S.syms[0]?.symbol);
  if(cur){
    if(S.sel!==cur){
      S.sel=cur;
      S.selectedCache=getSymbolState(cur);
      saveViewPrefs();
    }
    if(tvSym!==('BINANCE:'+cur) || !document.getElementById('tv-widget').children.length){
      initTV(cur,curIv);
    }
    renderDetail(cur);
    const selected=getSymbolState(cur);
    if(selected&&(!selected.klines||Object.keys(selected.klines).length===0)){
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
            <span class="cc-price" style="color:${chgColor}">${fP(mid)}</span>
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
    return `<div class="pi-mini" onclick="focusSignal('${s.symbol}','${s.watch_level||'观察'}','${(s.signal_reason||s.status_summary||'继续观察市场变化').replace(/'/g,'&#39;')}')">
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
    return `<div class="tbi" onclick="focusSignal('${s.symbol}','${s.watch_level||'观察'}','${(s.signal_reason||s.status_summary||'继续观察市场变化').replace(/'/g,'&#39;')}')">
      <span class="tb-s">${s.symbol.replace('USDT','/U')}</span>
      <span class="tb-p" style="color:${chg>=0?'var(--g)':'var(--r)'}">${fP(sv(s.symbol,'mid'))}</span>
      <span class="tb-c ${cls}">${chg>=0?'+':''}${chg.toFixed(2)}%</span>
    </div>`;
  }).join(''),'ticker');
}

// ── 选中币种 ─────────────────────────────────────────────────────
async function selSym(sym){
  S.sel=sym;
  saveViewPrefs();
  initTV(sym,curIv);
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
  }).slice(0,5);
}

function updateSignalDetail(sym,signal){
  const s=S.syms.find(x=>x.symbol===sym);
  if(!s)return;
  const change=(s.change_24h_pct||0);
  e('signal-detail-tag',signal?.tag||'当前币种');
  e('signal-detail-text',signal?.desc||s.signal_reason||'当前没有特别突出的异常信号。');
  e('signal-detail-level',s.watch_level||'观察');
  e('signal-detail-price',fP(sv(sym,'mid')));
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

// ── 详情 ─────────────────────────────────────────────────────────
function renderDetail(sym){
  const s=getSymbolState(sym);if(!s)return;
  if(S.sel===sym)S.selectedCache=s;
  const detailKey=selectedDetailKey(sym);
  if(S.ui.detailKey===detailKey)return;
  S.ui.detailKey=detailKey;
  const mid=sv(sym,'mid'),chg=s.change_24h_pct||0;
  const gc=chg>=0?'var(--g)':'var(--r)';
  const cvd=sv(sym,'cvd'),ps=sv(sym,'ps'),ds=sv(sym,'ds');
  const obi=sv(sym,'obi'),ofi=sv(sym,'ofi'),tbr=sv(sym,'tbr');
  const symShort=sym.replace('USDT','');
  const watchLevel=s.watch_level||'观察';
  const levelColor=watchLevel==='强提醒'?'var(--r)':watchLevel==='重点关注'?'var(--y)':watchLevel==='普通关注'?'var(--b)':'var(--t2)';
  refreshFavoriteButton();

  // 顶部导航
  e('nav-sym',sym.replace('USDT','/USDT'));
  es('nav-price',fP(mid),null,gc);
  const nc=document.getElementById('nav-chg');
  nc.textContent=(chg>=0?'+':'')+chg.toFixed(2)+'%';nc.className='nav-chg '+(chg>=0?'nup':'ndn');
  if(typeof updateDocumentTitle==='function')updateDocumentTitle(sym,fP(mid),chg);
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
    {n:'买卖价差',v:`${s.spread_bps.toFixed(1)} 基点`,bw:Math.min(100,s.spread_bps*3),bc:s.spread_bps<20?'gf':'yf',vc:s.spread_bps<10?'fg':s.spread_bps<30?'fy':'fn',tip:s.spread_bps<10?'成交环境很好':s.spread_bps<20?'正常':'价差偏大'},
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
  setHtmlIfChanged(
    'sig-list',
    sigs.slice(0,20).map((s,i)=>`
      <div class="scard ${s.type}" onclick="focusSignal('${s.full}','${(lbl[s.type]||s.type).replace(/'/g,'&#39;')}','${String(s.desc||'').replace(/'/g,'&#39;')}')">
        ${i===0?'<div class="sc-new">NEW</div>':''}
        <div class="sc-h"><span class="sc-sym">${s.sym}</span><span class="sc-t">${s.time}</span></div>
        <div class="sc-tag">${lbl[s.type]||s.type}</div>
        <div class="sc-desc">${s.desc}</div>
        ${s.score!=null?`<div class="sc-score">
          <div class="sc-score-bar"><div class="sc-score-fill" style="width:${Math.min(100,s.score)}%;background:${s.type==='pump'?'var(--g)':s.type==='dump'?'var(--r)':s.type==='whale'?'var(--b)':'var(--p)'}"></div></div>
          <span class="sc-score-v">${s.score}</span>
        </div>`:''}
      </div>`).join('')||'<div class="empty-p">📡<br>等待信号<br><span style="color:var(--t3)">系统会在有明显异动时提醒</span></div>',
    'signals'
  );
}
