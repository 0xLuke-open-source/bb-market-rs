// ── 预警 ─────────────────────────────────────────────────────────
function checkAlerts(){
  S.syms.forEach(s=>{
    const ps=sv(s.symbol,'ps'),ds=sv(s.symbol,'ds'),sym=s.symbol.replace('USDT',''),t=nowT();
    if(signalWindow!=='all'&&!withinWindow(t))return;
    if(ps>=75){const id=`p-${s.symbol}-${Math.floor(Date.now()/30000)}`;
      if(!S.seen.has(id)){S.seen.add(id);S.alerts.unshift({type:'pump',sym,full:s.symbol,
        tag:'🚀 上涨异动',time:t,desc:(s.signal_reason||`评分${Math.round(ps)}/100 买卖盘失衡${sv(s.symbol,'obi').toFixed(1)}%`),fresh:true});recordSignalPerf(s.symbol,'pump',sv(s.symbol,'mid'),ps);}}
    if(ds>=75){const id=`d-${s.symbol}-${Math.floor(Date.now()/30000)}`;
      if(!S.seen.has(id)){S.seen.add(id);S.alerts.unshift({type:'dump',sym,full:s.symbol,
        tag:'📉 下跌异动',time:t,desc:(s.signal_reason||`评分${Math.round(ds)}/100 买卖盘失衡${sv(s.symbol,'obi').toFixed(1)}%`),fresh:true});recordSignalPerf(s.symbol,'dump',sv(s.symbol,'mid'),ds);}}
    if(s.whale_entry){const id=`w-${s.symbol}-${Math.floor(Date.now()/60000)}`;
      if(!S.seen.has(id)){S.seen.add(id);S.alerts.unshift({type:'whale',sym,full:s.symbol,
        tag:'🐋 大户资金进场',time:t,desc:(s.signal_reason||`大单${s.max_bid_ratio.toFixed(1)}% 主动买卖量差${fN(sv(s.symbol,'cvd'))}`),fresh:true});recordSignalPerf(s.symbol,'pump',sv(s.symbol,'mid'),Math.max(ps,60));}}
    const cvd=sv(s.symbol,'cvd');
    if(Math.abs(cvd)>50000){const id=`c-${s.symbol}-${Math.floor(Date.now()/120000)}`;
      if(!S.seen.has(id)){S.seen.add(id);S.alerts.unshift({type:'cvd',sym,full:s.symbol,
        tag:cvd>0?'📈 主动买入占优':'📉 主动卖出占优',time:t,desc:(s.signal_reason||`主动买卖量差${fN(cvd)} 主动买入占比${sv(s.symbol,'tbr').toFixed(0)}%`),fresh:true});recordSignalPerf(s.symbol,cvd>0?'pump':'dump',sv(s.symbol,'mid'),Math.max(Math.abs(cvd)/2000,55));}}
  });
  if(S.alerts.length>50)S.alerts=S.alerts.slice(0,50);
  const alerts=S.alerts.filter(a=>withinWindow(a.time)).slice(0,50);
  e('al-cnt',alerts.length);
  setHtmlIfChanged('al-list',alerts.map((a,i)=>`
    <div class="scard ${a.type}" onclick="focusSignal('${a.full}','${String(a.tag||'').replace(/'/g,'&#39;')}','${String(a.desc||'').replace(/'/g,'&#39;')}')">
      ${a.fresh&&i===0?'<div class="sc-new">NEW</div>':''}
      <span class="sc-x" onclick="event.stopPropagation();dismissAlert('${a.time}','${a.full}','${a.type}');">✕</span>
      <div class="sc-h"><span class="sc-sym">${a.sym}</span><span class="sc-t">${a.time}</span></div>
      <div class="sc-tag">${a.tag}</div>
      <div class="sc-desc">${a.desc}</div>
    </div>`).join('')||'<div class="empty-p">🔔<br>等待预警<br><span style="color:var(--t3)">出现高风险变化时会在这里提示</span></div>','alerts');
}

