
const S={syms:[],feed:[],sel:null,selectedCache:null,sm:{},cvdH:{},metricH:{},signalPerf:{},seen:new Set(),alerts:[],tr:{},detailSignal:null,favorites:[],
  access:{authenticated:false,subscribed:false,full_access:false,visible_symbols:0,total_symbols:0,symbol_limit:null,subscription_plan:null,subscription_expires_at:null,message:''},
  trader:{account_id:0,balances:[],open_orders:[],order_history:[],trade_history:[]},
  auth:{user:null,ready:false,appReady:false,domBound:false,ws:null,detailPoller:null,plans:[],selectedPlan:'pro_month'},
  site:{page:'home'},
  ui:{pairAll:'',pairSig:'',pairWhale:'',pairMini:'',ticker:'',signals:'',alerts:'',detailKey:''}};
const A=0.25,HL=60;
let curIv='60',tvSym='',oaTabMode=0,tradeType=0,searchQ='';
let marketSort='focus',signalWindow='all',marketQuickFilter='all';
let tvLoadingPromise=null;
const VIEW_PREF_KEY='bb_market_view_prefs_v1';
const IVMAP={'1':'1m','3':'3m','5':'5m','15':'15m','30':'30m',
  '60':'1h','120':'2h','240':'4h','360':'6h','480':'8h','720':'12h',
  'D':'1d','3D':'3d','W':'1w','M':'1M'};

// ── TradingView ──────────────────────────────────────────────────
