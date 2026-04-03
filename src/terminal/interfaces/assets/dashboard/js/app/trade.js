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
      priceLabel.textContent=side==='buy'?'买入价格':'卖出价格';
      autofillTradeForm(side);
    }
    stopBox.classList.toggle('show',isStop);
  });
  updateTradePrecisionUI(S.sel);
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
  document.getElementById('buy-total').textContent=fQuote(total);
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
  document.getElementById('sell-total').textContent=fQuote(qty*price);
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
  updateTradePrecisionUI(S.sel);
  updateTotals();
}

function updateTotals(){
  const bp=normalizePriceValue(parseFloat(document.getElementById('buy-price').value)||sv(S.sel||'','mid')||0,S.sel);
  recalcQtyByPct('buy',document.getElementById('buy-pct')?.value||0);
  const bq=normalizeQtyValue(parseFloat(document.getElementById('buy-qty').value)||0,S.sel);
  document.getElementById('buy-total').textContent=fQuote(bp*bq);
  const sp=normalizePriceValue(parseFloat(document.getElementById('sell-price').value)||sv(S.sel||'','mid')||0,S.sel);
  recalcQtyByPct('sell',document.getElementById('sell-pct')?.value||0);
  const sq=normalizeQtyValue(parseFloat(document.getElementById('sell-qty').value)||0,S.sel);
  document.getElementById('sell-total').textContent=fQuote(sp*sq);
}

// ── 真实下单 ────────────────────────────────────────────────────
async function doTrade(side){
  if(!S.sel)return;
  const priceId=side==='buy'?'buy-price':'sell-price';
  const qtyId=side==='buy'?'buy-qty':'sell-qty';
  const priceInput=document.getElementById(priceId);
  const qtyInput=document.getElementById(qtyId);
  const price=normalizeTradeFieldInput(priceInput,'price',S.sel) ?? normalizePriceValue(sv(S.sel,'mid')||0,S.sel);
  const qty=normalizeTradeFieldInput(qtyInput,'qty',S.sel) ?? 0;
  if(!qty||qty<=0){alert('请输入有效数量');return;}
  const triggerInput=tradeType===2?document.getElementById(`${side}-trigger-price`):null;
  const triggerPrice=tradeType===2?(normalizeTradeFieldInput(triggerInput,'price',S.sel) ?? null):null;
  const payload={
    symbol:S.sel,
    side,
    order_type:tradeType===2?(price?'stop_limit':'stop_market'):(tradeType===1?'market':'limit'),
    time_in_force:tradeType===1?'ioc':'gtc',
    price,
    quantity:qty,
    trigger_price: triggerPrice,
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
  const res=await apiFetch(`/api/spot/order/${orderId}`,{method:'DELETE'});
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
