// ── EMA ──────────────────────────────────────────────────────────
function ema(sym,k,v){if(!S.sm[sym])S.sm[sym]={};const p=S.sm[sym][k];if(p===undefined){S.sm[sym][k]=v;return v;}const r=A*v+(1-A)*p;S.sm[sym][k]=r;return r;}
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
  if(!s){e('enterprise-metrics','');return;}
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
      {name:'大单成交占比',score:clamp(largeTradeRatio,0,100),value:fmtMetricValue(largeTradeRatio,'%'),tip:`最近1分钟大单成交额，相对日均每分钟成交额的占比。`},
      {name:'买卖连续性',score:clamp(directionalContinuity,0,100),value:fmtMetricValue(directionalContinuity,'%'),tip:`主动买卖方向是否持续偏向单边，越高说明延续性越强。`},
      {name:'短时成交密度',score:tradeDensity,value:fmtMetricValue(recentBig.length,'count'),tip:`最近1分钟捕捉到 ${recentBig.length} 笔大额成交。`},
      {name:'笔数突变',score:countSurge,value:fmtMetricValue(recentBig.length-prevBig.length,'count'),tip:`对比前1分钟，大额成交笔数的变化幅度。`},
      {name:'成交额突变',score:amountSurge,value:fmtMetricValue(bigRecentNotional,'compact'),tip:`对比前1分钟，大额成交额的变化幅度。`}
    ]),
    buildMetricSection('盘口结构','深度与挂单质量',[
      {name:'买卖墙强度',score:wallStrength,value:fmtMetricValue(Math.max(s.max_bid_ratio||0,s.max_ask_ratio||0),'%'),tip:`前排深度中大额挂单的占比，越高说明墙体越明显。`},
      {name:'挂撤单比',score:cancelRatioEst,value:fmtMetricValue(cancelRatioEst,'%'),tip:`根据深度净变化估算撤单与改单的活跃程度。`,invert:true},
      {name:'恢复速度',score:recoverySpeed,value:fmtMetricValue(recoverySpeed,'%'),tip:`盘口深度被打掉后，重新回补的速度。`},
      {name:'前5/10/20档变化',score:clamp((Math.abs(depth5Delta)+Math.abs(depth10Delta)+Math.abs(depth20Delta))/3,0,100),value:`${depth5Delta.toFixed(0)} / ${depth10Delta.toFixed(0)} / ${depth20Delta.toFixed(0)}%`,tip:`对比约 20 到 30 秒前，前排深度的变化情况。`},
      {name:'深度断层',score:clamp(depthGapBps/2,0,100),value:fmtMetricValue(depthGapBps,'bps'),tip:`相邻档位之间的跳空程度，越高说明深度越不连续。`,invert:true}
    ]),
    buildMetricSection('价格行为','多周期价格状态',[
      {name:'多周期一致性',score:multiTfConsistency,value:fmtMetricValue(multiTfConsistency,'%'),tip:`1m、5m、15m、1h 几个周期的方向一致程度。`},
      {name:'波动扩张/收缩',score:volExpand,value:fmtMetricValue(volExpand,'%'),tip:`当前 1m 波动相对最近 10 根 1m 的放大程度。`},
      {name:'假突破识别',score:falseBreak?82:28,value:falseBreak?'疑似假突破':'暂未发现',tip:`破位后快速收回时，追单风险通常会更高。`,invert:!falseBreak?false:true},
      {name:'回吐幅度',score:clamp(pullback*10,0,100),value:fmtMetricValue(pullback,'%'),tip:`急拉或急砸之后，价格已经回吐的幅度。`,invert:true},
      {name:'新高/低承接',score:acceptance,value:fmtMetricValue(acceptance,'%'),tip:`接近新高或新低时，是否仍有主动资金承接。`}
    ]),
    buildMetricSection('资金痕迹','吸筹、派发与大户跟随',[
      {name:'持续吸筹/派发',score:accumulation,value:s.cvd>=0?'偏吸筹':'偏派发',tip:`结合 CVD、买入占比和盘口失衡做出的综合判断。`},
      {name:'大户跟随强度',score:whaleFollow,value:fmtMetricValue(whaleFollow,'%'),tip:`大户信号出现后，盘口和主动成交是否继续跟随。`},
      {name:'大单停留时间',score:wallDwell,value:fmtMetricValue(wallDwell,'%'),tip:`大墙挂单在前排停留的时长估算。`},
      {name:'主动买卖量差斜率',score:clamp(Math.abs(cvdSlope),0,100),value:fmtMetricValue(cvdSlope,'%'),tip:`主动买卖量差的增长或衰减速度。`}
    ]),
    buildMetricSection('跨周期指标','信号共振与确认',[
      {name:'1m/5m/15m/1h共振',score:resonance,value:fmtMetricValue(resonance,'%'),tip:`短中周期信号是否同时偏向同一方向。`},
      {name:'短期获中期确认',score:confirmation,value:confirmation>=60?'已确认':'待确认',tip:`短周期异动是否已经获得 5m / 15m 的方向确认。`}
    ]),
    buildMetricSection('市场广度','全市场同步状态',[
      {name:'上涨/下跌家数',score:pct(Math.max(upCount,downCount),Math.max(S.syms.length,1)),value:`${upCount} / ${downCount}`,tip:`当前全市场上涨家数与下跌家数的对比。`},
      {name:'强势币占比',score:strongShare,value:fmtMetricValue(strongShare,'%'),tip:`评分较高币种在当前市场中的占比。`},
      {name:'异常币占比',score:anomalyShare,value:fmtMetricValue(anomalyShare,'%'),tip:`异常波动严重的币种，在当前市场中的占比。`},
      {name:'板块联动强弱',score:linkage,value:fmtMetricValue(linkage,'%'),tip:`市场方向集中度与强势币占比的综合估算。`}
    ]),
    buildMetricSection('交易质量','可成交性与流动性',[
      {name:'点差水平',score:spreadLevel,value:fmtMetricValue(s.spread_bps||0,'bps'),tip:`点差越小，短线执行环境通常越友好。`},
      {name:'深度可成交性',score:executableDepth,value:fmtMetricValue(executableDepth,'%'),tip:`按 1000 USDT 吃单试算，盘口可立即承接的程度。`},
      {name:'滑点风险估计',score:clamp(slippageRisk*6,0,100),value:fmtMetricValue(slippageRisk,'bps'),tip:`按 1000 USDT 吃单估算出来的滑点水平。`,invert:true},
      {name:'流动性恶化预警',score:liquidityWarning,value:liquidityWarning>=70?'偏高':'正常',tip:`结合点差、可成交深度和异常波动得出的综合风险。`,invert:true}
    ]),
    buildMetricSection('信号质量','最近触发后的表现',[
      {name:'过去5分钟表现',score:perf.win5,value:perf.count5?`${perf.win5.toFixed(0)}%`:'样本少',tip:`信号触发后 5 分钟内，方向判断的正确率。`},
      {name:'过去15分钟表现',score:perf.win15,value:perf.count15?`${perf.win15.toFixed(0)}%`:'样本少',tip:`信号触发后 15 分钟内，方向判断的正确率。`},
      {name:'胜率/误报率',score:perf.win15||perf.win5,value:perf.count15?`${perf.win15.toFixed(0)} / ${(100-perf.win15).toFixed(0)}%`:'待积累',tip:`胜率越高说明更稳定，误报率越低说明噪音更少。`},
      {name:'信号衰减速度',score:clamp(100-(perf.decay||0)*6,0,100),value:perf.decay?`${perf.decay.toFixed(1)} 分钟`:'待积累',tip:`信号从强提醒回落到普通关注所需的平均时间。`}
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
