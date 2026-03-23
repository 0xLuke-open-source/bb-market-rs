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
    saveViewPrefs();
    if(S.sel){
      initTV(S.sel,curIv);
      await loadSymbolDetail(S.sel,true);
    }
    updOHLCV();
  };
});

// ── 左侧搜索 ─────────────────────────────────────────────────────
