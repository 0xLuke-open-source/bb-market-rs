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
    getKey:a=>`${a.full}|${a.type}`,
    emptyHtml:'<div class="empty-p">🔔<br>等待预警<br><span style="color:var(--t3)">出现高风险变化时会在这里提示</span></div>',
    enterClass:'soft-stream-enter',
    minAnimateInterval:5000,
    pauseOnHover:true,
    renderItem:(a,i)=>`
      <div class="scard ${a.type}" onclick='${focusSignalCall(a.full,a.tag||'',a.desc||'')}'>
        ${a.fresh&&i===0?'<div class="sc-new">NEW</div>':''}
        <span class="sc-x" onclick="event.stopPropagation();dismissAlert('${a.time}','${a.full}','${a.type}');">✕</span>
        <div class="sc-h"><span class="sc-sym">${a.sym}</span><span class="sc-t">${a.time}</span></div>
        <div class="sc-tag">${a.tag}</div>
        <div class="sc-desc">${a.desc}</div>
      </div>`
  });
}
