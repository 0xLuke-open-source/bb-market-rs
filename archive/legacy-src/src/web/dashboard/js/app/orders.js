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
