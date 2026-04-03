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

function clamp(v,min,max){return Math.max(min,Math.min(max,v));}
function metricTone(score,invert=false){
  const s=invert?(100-score):score;
  return s>=70?'cae-good':s>=40?'cae-warn':'cae-bad';
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
function metricSource(sym,sourceOverride=null){
  if(sourceOverride)return sourceOverride;
  const s=S.syms.find(x=>x.symbol===sym);
  if(!s)return null;
  return typeof resolvePanelRenderState==='function'?resolvePanelRenderState(sym,s):s;
}
function normalizeMetricRows(items){
  return (Array.isArray(items)?items:[]).map(item=>({
    name:String(item?.name||'--'),
    score:Number.isFinite(Number(item?.score))?Number(item.score):0,
    value:item?.value==null?'--':String(item.value),
    tip:String(item?.tip||''),
    invert:!!item?.invert
  }));
}
function normalizeMetricSections(raw){
  return (Array.isArray(raw)?raw:[])
    .map(section=>({
      title:String(section?.title||''),
      subtitle:String(section?.subtitle||''),
      items:normalizeMetricRows(section?.items)
    }))
    .filter(section=>section.title && section.items.length);
}
function factorToneView(tone){
  if(tone==='fg')return {bar:'gf',value:'fg'};
  if(tone==='fr')return {bar:'rf2',value:'fr'};
  if(tone==='fy')return {bar:'yf',value:'fy'};
  return {bar:'yf',value:'fn'};
}
function normalizeFactorMetrics(raw){
  return (Array.isArray(raw)?raw:[]).map(item=>{
    const tone=factorToneView(String(item?.tone||''));
    const score=Number.isFinite(Number(item?.score))?Number(item.score):0;
    return {
      name:String(item?.name||'--'),
      value:item?.value==null?'--':String(item.value),
      score:Math.max(0,Math.min(100,score)),
      tip:String(item?.tip||''),
      barClass:tone.bar,
      valueClass:tone.value
    };
  });
}
function renderFactorMetrics(source){
  const factors=normalizeFactorMetrics(source?.factor_metrics);
  if(!factors.length){
    document.getElementById('rf-list').innerHTML='<div class="loading-empty">后端指标加载中...</div>';
    return;
  }
  document.getElementById('rf-list').innerHTML=factors.map(item=>`
    <div class="fi"><div class="fi-n">${item.name}</div>
      <div><div class="fi-bar"><div class="fi-f ${item.barClass}" style="width:${item.score}%"></div></div>
      <div class="fi-tip">${item.tip}</div></div>
      <div class="fi-v ${item.valueClass}">${item.value}</div></div>`).join('');
}
function renderEnterpriseMetrics(sym,sourceOverride=null){
  const source=metricSource(sym,sourceOverride);
  if(!source){
    S.ui.enterprise='';
    e('enterprise-metrics','');
    return;
  }
  const sections=normalizeMetricSections(source.enterprise_metrics);
  if(!sections.length){
    S.ui.enterprise='';
    document.getElementById('enterprise-metrics').innerHTML='<div class="loading-empty">后端指标加载中...</div>';
    return;
  }
  setHtmlIfChanged('enterprise-metrics',sections.map(section=>buildMetricSection(section.title,section.subtitle,section.items)).join(''),'enterprise');
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
