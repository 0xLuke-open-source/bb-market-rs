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
  if(!payload)return payload;
  if(payload.k==='m1'){
    return {
      symbols:inflateWsSymbolRows(payload.s),
      feed:inflateWsFeedRows(payload.f),
      trader:payload.t||{account_id:0,balances:[],open_orders:[],order_history:[],trade_history:[]},
      access:payload.a||S.access,
      total_updates:wsNum(payload.u),
      uptime_secs:wsNum(payload.up)
    };
  }
  if(payload.k==='m2'){
    return {
      __delta:true,
      symbols:inflateWsSymbolRows(payload.s),
      removedSymbols:(Array.isArray(payload.rm)?payload.rm:[]).map(sym=>String(sym||'')),
      feedDelta:inflateWsFeedRows(payload.f),
      trader:payload.t||null,
      access:payload.a||null,
      total_updates:wsNum(payload.u),
      uptime_secs:wsNum(payload.up)
    };
  }
  return payload;
}
const WS_TEXT_DECODER=typeof TextDecoder!=='undefined'?new TextDecoder():null;
const WS_STALE_MS=25000;
const DETAIL_WS_STALE_MS=10000;
const DETAIL_WS_FORCE_SYNC_MS=4500;
const DETAIL_WS_STUCK_RECONNECT_MS=12000;
const WS_HEALTH_CHECK_MS=2000;
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
    if(type>=0x90 && type<=0x9f){
      const len=type&0x0f;
      return readArray(len);
    }
    if(type>=0x80 && type<=0x8f){
      const len=type&0x0f;
      return readMap(len);
    }
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
  return readValue();
}
function decodeWsPayload(data){
  if(typeof data==='string')return JSON.parse(data);
  if(data instanceof ArrayBuffer)return decodeMsgPack(data);
  if(ArrayBuffer.isView(data))return decodeMsgPack(new Uint8Array(data.buffer,data.byteOffset,data.byteLength));
  throw new Error('Unsupported WebSocket payload type');
}
function markWsAlive(channel='main'){
  const now=Date.now();
  if(channel==='detail')S.auth.detailWsLastAt=now;
  else S.auth.wsLastAt=now;
}
function scheduleWsHealthCheck(delay=WS_HEALTH_CHECK_MS){
  if(S.auth.wsHealthTimer)clearTimeout(S.auth.wsHealthTimer);
  S.auth.wsHealthTimer=setTimeout(runWsHealthCheck,delay);
}
function runWsHealthCheck(){
  S.auth.wsHealthTimer=null;
  if(!S.auth.appReady)return;
  const now=Date.now();
  const mainWs=S.auth.ws;
  if(mainWs && mainWs.readyState===WebSocket.OPEN && now-Number(S.auth.wsLastAt||0)>WS_STALE_MS){
    try{mainWs.close();}catch(_){}
  }
  const detailWs=S.auth.detailWs;
  if(
    detailWs
    && detailWs.readyState===WebSocket.OPEN
    && S.auth.detailWsSymbol===S.sel
    && now-Number(S.auth.detailWsLastAt||0)>DETAIL_WS_STALE_MS
  ){
    try{detailWs.close();}catch(_){}
  }
  if(S.auth.appReady)scheduleWsHealthCheck();
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
    pauseOnHover=false,
    animate=true
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
      const flushPending=()=>{
        const pending=container.__softPending;
        if(pending){
          container.__softPending=null;
          updateSoftStreamList(container.id,pending.items,pending.options);
        }
      };
      const releasePointerHold=(defer=false)=>{
        if(container.dataset.softPointerHold!=='1')return;
        container.dataset.softPointerHold='0';
        if(container.__softPointerReleaseTimer){
          clearTimeout(container.__softPointerReleaseTimer);
          container.__softPointerReleaseTimer=null;
        }
        if(defer){
          // click 在 pointerup 之后触发；延后一拍再刷列表，避免“点 A 命中重排后的 B”。
          container.__softPointerReleaseTimer=setTimeout(()=>{
            container.__softPointerReleaseTimer=null;
            flushPending();
          },0);
          return;
        }
        flushPending();
      };
      container.addEventListener('pointerdown',()=>{
        container.dataset.softPointerHold='1';
      });
      container.addEventListener('pointerup',()=>releasePointerHold(true));
      container.addEventListener('pointercancel',()=>releasePointerHold(false));
      container.addEventListener('lostpointercapture',()=>releasePointerHold(true));
    }
    if(container.dataset.softPaused==='1' || container.dataset.softPointerHold==='1' || container.matches(':hover')){
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
  const canAnimate=animate && container.dataset.softLive==='1' && !scopeChanged;
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
function normalizeSymbolKeyword(keyword=''){
  return String(keyword||'').toUpperCase().replace(/\s+/g,'').replace(/[\/_-]/g,'');
}
function syncMarketSearchInput(keyword=''){
  const input=document.getElementById('srch');
  if(input)input.value=keyword;
}
async function ensureSymbolUniverse(){
  if((S.symbolUniverse||[]).length)return S.symbolUniverse;
  try{
    const symbols=await apiFetch('/api/symbols').then(r=>r.json());
    S.symbolUniverse=Array.isArray(symbols)?symbols:[];
  }catch(_){
    S.symbolUniverse=(S.syms||[]).map(item=>item.symbol);
  }
  return S.symbolUniverse;
}
function findSymbolByKeyword(keyword=''){
  const normalized=normalizeSymbolKeyword(keyword);
  if(!normalized)return null;
  const liveList=(S.syms||[]).map(item=>item.symbol);
  const universe=[...new Set([...(S.symbolUniverse||[]),...liveList])];
  const exact=universe.find(symbol=>symbol===normalized || symbol===`${normalized}USDT`);
  if(exact)return exact;
  const baseMatch=universe.find(symbol=>symbol.replace(/USDT$/,'')===normalized);
  if(baseMatch)return baseMatch;
  const startsWith=universe.find(symbol=>symbol.startsWith(normalized) || symbol.replace(/USDT$/,'').startsWith(normalized));
  if(startsWith)return startsWith;
  const fuzzy=universe.find(symbol=>symbol.includes(normalized) || symbol.replace(/USDT$/,'').includes(normalized));
  return fuzzy||null;
}
async function searchTopSymbol(){
  const input=document.getElementById('site-search-input');
  const raw=input?.value||'';
  const keyword=normalizeSymbolKeyword(raw);
  if(!keyword)return;
  await ensureSymbolUniverse();
  const matched=findSymbolByKeyword(keyword);
  if(typeof switchSitePage==='function')switchSitePage('ai');
  if(typeof filterP==='function'){
    filterP(keyword);
    syncMarketSearchInput(keyword);
  }
  if(matched && typeof selSym==='function'){
    await selSym(matched);
    if(input)input.value=matched.replace('USDT','');
    return;
  }
  const tip=document.getElementById('ctip');
  if(tip){
    tip.textContent=`未找到 ${keyword}，已按关键词筛选`;
    tip.classList.add('show');
    setTimeout(()=>tip.classList.remove('show'),1800);
  }
}
function updateDocumentTitle(sym='',price='--',change=null){
  const page=S.site?.page||'home';
  const pageTitles={
    home:'BB-Market',
    ai:'AI盯盘',
    vip:'VIP服务',
    ads:'广告',
    feedback:'产品反馈与建议',
    rebate:'超级返佣',
    invite:'邀请奖励',
    plaza:'广场',
    blog:'博客',
    help:'帮助中心',
    announcements:'公告',
    news:'新闻中心',
    community:'社区',
    agreement:'服务协议',
    privacy:'隐私说明',
    about:'关于我们'
  };
  if(page!=='ai'){
    if(page==='home'){
      document.title='BB-Market - 实时交易智能中枢';
      return;
    }
    document.title=`${pageTitles[page]||'BB-Market'} - BB-Market`;
    return;
  }
  if(!sym){
    document.title='AI盯盘 - BB-Market';
    return;
  }
  const changeText=typeof change==='number'&&!Number.isNaN(change)?` ${change>=0?'+':''}${change.toFixed(2)}%`:'';
  document.title=`${fmtSym(sym)} ${price||'--'}${changeText}`;
}
function filledPct(order){const q=+order.quantity||0;if(q<=0)return 0;return Math.round((+order.filled_qty||0)/q*100);}
function getBalance(asset){return (S.trader.balances||[]).find(b=>b.asset===asset)||{available:0,locked:0};}
function sideLabel(side){return String(side||'').toUpperCase()==='BUY'?'买':'卖';}
function sideColor(side){return String(side||'').toUpperCase()==='BUY'?'var(--g)':'var(--r)';}
function getActivePanelReplay(sym=S.sel){
  const replay=S.panelReplay||{};
  if(!replay.active||!replay.snapshot)return null;
  if(sym&&replay.symbol&&replay.symbol!==sym)return null;
  return replay.snapshot;
}
function selectedDetailKey(sym){
  const s=getSymbolState(sym);if(!s)return '';
  const quoteBal=getBalance('USDT');
  const baseBal=getBalance(sym.replace('USDT',''));
  const replay=getActivePanelReplay(sym);
  return JSON.stringify({
    sym,
    replay:replay?{ts:replay.event_ts||replay.event_time||'',uc:replay.update_count||0}:{},
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
async function apiFetch(url,options={}){
  const res=await fetch(url,options);
  if(res.status===401){
    if(typeof handleAuthExpired==='function')handleAuthExpired();
    const err=new Error('AUTH_REQUIRED');
    err.code='AUTH_REQUIRED';
    throw err;
  }
  return res;
}
async function postJson(url,payload){
  const res=await apiFetch(url,{method:'POST',headers:{'Content-Type':'application/json'},body:JSON.stringify(payload)});
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
  try{S.trader=await apiFetch('/api/spot/state').then(r=>r.json());}catch(_){}
}
async function openReplay(){
  const atTs=prompt('输入要回放的毫秒时间戳，留空则读取最近归档事件：','');
  if(atTs===null)return;
  const q=atTs.trim()?`?at_ts=${encodeURIComponent(atTs.trim())}&limit=50`:'?limit=50';
  let res;
  try{
    res=await apiFetch('/api/spot/replay'+q).then(r=>r.json());
  }catch(err){
    if(err&&err.code==='AUTH_REQUIRED')return;
    alert('回放失败');
    return;
  }
  if(!res.ok){alert(res.message||'回放失败');return;}
  S.trader=res.data.snapshot||S.trader;
  renderOrders();
  const lines=(res.data.events||[]).slice(0,10).map(e=>`${e.seq} | ${e.kind} | ${e.summary}`);
  alert(`已载入回放快照\n事件数: ${(res.data.events||[]).length}\n`+(lines.length?('\n最近事件:\n'+lines.join('\n')):''));
}
function escapePanelReplayHtml(value){
  return String(value??'')
    .replace(/&/g,'&amp;')
    .replace(/</g,'&lt;')
    .replace(/>/g,'&gt;')
    .replace(/"/g,'&quot;')
    .replace(/'/g,'&#39;');
}
function panelHistoryModal(){
  return document.getElementById('panel-history-modal');
}
function bigTradeHistoryModal(){
  return document.getElementById('big-trade-history-modal');
}
function ensureFloatingModal(modal){
  if(!modal)return null;
  if(modal.parentElement!==document.body){
    document.body.appendChild(modal);
  }
  return modal;
}
function panelHistoryStatus(text,color='var(--t3)'){
  const el=document.getElementById('panel-history-status');
  if(!el)return;
  el.textContent=text;
  el.style.color=color;
}
function bigTradeHistoryStatus(text,color='var(--t3)'){
  const el=document.getElementById('big-trade-history-status');
  if(!el)return;
  el.textContent=text;
  el.style.color=color;
}
function formatPanelHistoryInput(ts){
  const dt=new Date(Number(ts||Date.now()));
  const pad=v=>String(v).padStart(2,'0');
  return `${dt.getFullYear()}-${pad(dt.getMonth()+1)}-${pad(dt.getDate())}T${pad(dt.getHours())}:${pad(dt.getMinutes())}`;
}
function formatPanelHistoryLabel(value){
  const dt=new Date(value);
  if(!Number.isFinite(dt.getTime()))return String(value||'--');
  return dt.toLocaleString('zh-CN',{hour12:false});
}
function parsePanelHistoryInput(id){
  const el=document.getElementById(id);
  const raw=String(el?.value||'').trim();
  if(!raw)return null;
  const ts=new Date(raw).getTime();
  return Number.isFinite(ts)?ts:null;
}
function renderBigTradeHistoryStats(){
  const el=document.getElementById('big-trade-history-stats');
  if(!el)return;
  const stats=S.bigTradeHistory?.stats;
  if(S.bigTradeHistory?.loading){
    el.innerHTML='';
    return;
  }
  if(!stats){
    el.innerHTML='';
    return;
  }
  const buyRatio=Number(stats.buy_ratio||0);
  const sellRatio=Number(stats.sell_ratio||0);
  el.innerHTML=`<div style="display:grid;grid-template-columns:repeat(4,minmax(0,1fr));gap:10px">
    <div style="padding:12px 14px;border-radius:14px;border:1px solid rgba(148,163,184,.12);background:rgba(255,255,255,.02)">
      <div style="font-size:11px;color:var(--t3)">大单总数</div>
      <div style="margin-top:6px;font-size:20px;font-weight:800;color:var(--t1)">${stats.total_count||0}</div>
    </div>
    <div style="padding:12px 14px;border-radius:14px;border:1px solid rgba(14,203,129,.16);background:rgba(14,203,129,.06)">
      <div style="font-size:11px;color:var(--t3)">主动买 / 占比</div>
      <div style="margin-top:6px;font-size:18px;font-weight:800;color:var(--g)">${stats.buy_count||0} / ${buyRatio.toFixed(1)}%</div>
      <div style="margin-top:4px;font-size:11px;color:var(--t3)">买入金额 ${fN(stats.buy_quote_quantity||0)}</div>
    </div>
    <div style="padding:12px 14px;border-radius:14px;border:1px solid rgba(246,70,93,.16);background:rgba(246,70,93,.06)">
      <div style="font-size:11px;color:var(--t3)">主动卖 / 占比</div>
      <div style="margin-top:6px;font-size:18px;font-weight:800;color:var(--r)">${stats.sell_count||0} / ${sellRatio.toFixed(1)}%</div>
      <div style="margin-top:4px;font-size:11px;color:var(--t3)">卖出金额 ${fN(stats.sell_quote_quantity||0)}</div>
    </div>
    <div style="padding:12px 14px;border-radius:14px;border:1px solid rgba(148,163,184,.12);background:rgba(255,255,255,.02)">
      <div style="font-size:11px;color:var(--t3)">总额 / 均额 / 最大</div>
      <div style="margin-top:6px;font-size:16px;font-weight:800;color:var(--t1)">${fN(stats.total_quote_quantity||0)}</div>
      <div style="margin-top:4px;font-size:11px;color:var(--t3)">均额 ${fN(stats.avg_quote_quantity||0)} · 最大 ${fN(stats.max_quote_quantity||0)}</div>
      <div style="margin-top:4px;font-size:11px;color:var(--t3)">平均阈值 ${fQ(stats.avg_threshold_quantity||0,getSymbolState(S.sel)||null)}</div>
    </div>
  </div>`;
}
function renderBigTradeHistoryList(){
  const list=document.getElementById('big-trade-history-list');
  if(!list)return;
  const items=Array.isArray(S.bigTradeHistory?.items)?S.bigTradeHistory.items:[];
  if(S.bigTradeHistory?.loading){
    list.innerHTML='<div class="loading-empty">正在查询历史大单...</div>';
    return;
  }
  if(S.bigTradeHistory?.error){
    list.innerHTML=`<div class="loading-empty" style="color:var(--r)">${escapePanelReplayHtml(S.bigTradeHistory.error)}</div>`;
    return;
  }
  if(!items.length){
    list.innerHTML='<div class="loading-empty">当前区间没有独立入库的大单事件。</div>';
    return;
  }
  const ctx=getSymbolState(S.sel)||null;
  list.innerHTML=items.map(item=>{
    const buy=!!item.is_taker_buy;
    return `<div style="display:grid;grid-template-columns:minmax(0,1.3fr) minmax(0,.9fr) minmax(0,1fr) minmax(0,.9fr);gap:12px;align-items:center;padding:12px 14px;margin-bottom:10px;border-radius:14px;border:1px solid rgba(148,163,184,.12);background:rgba(255,255,255,.02)">
      <div style="min-width:0">
        <div style="display:flex;gap:10px;align-items:center;flex-wrap:wrap">
          <span style="font-size:12px;font-weight:800;color:${buy?'var(--g)':'var(--r)'}">${buy?'主动买大单':'主动卖大单'}</span>
          <span style="font-size:12px;color:var(--t1)">${escapePanelReplayHtml(item.trade_time_label||'--')}</span>
          <span style="font-size:11px;color:var(--t3)">ID ${escapePanelReplayHtml(item.agg_trade_id)}</span>
        </div>
        <div style="margin-top:6px;font-size:12px;color:var(--t3)">阈值 ${escapePanelReplayHtml(fQ(Number(item.threshold_quantity||0),ctx))} · BuyerMaker ${item.is_buyer_maker?'是':'否'}</div>
      </div>
      <div style="font-size:12px;color:var(--t3)">
        <div>价格</div>
        <div style="margin-top:5px;font-size:15px;font-weight:800;color:${buy?'var(--g)':'var(--r)'}">${escapePanelReplayHtml(fP(Number(item.price||0),ctx))}</div>
      </div>
      <div style="font-size:12px;color:var(--t3)">
        <div>数量</div>
        <div style="margin-top:5px;font-size:15px;font-weight:800;color:var(--t1)">${escapePanelReplayHtml(fQ(Number(item.quantity||0),ctx))}</div>
      </div>
      <div style="font-size:12px;color:var(--t3)">
        <div>成交额</div>
        <div style="margin-top:5px;font-size:15px;font-weight:800;color:var(--y)">${escapePanelReplayHtml(fN(Number(item.quote_quantity||0)))}</div>
      </div>
    </div>`;
  }).join('');
}
function setBigTradeHistoryPreset(minutes){
  const end=Date.now();
  const start=end-Math.max(1,Number(minutes)||120)*60000;
  const from=document.getElementById('big-trade-history-from');
  const to=document.getElementById('big-trade-history-to');
  if(from)from.value=formatPanelHistoryInput(start);
  if(to)to.value=formatPanelHistoryInput(end);
}
async function loadBigTradeHistory(){
  if(!S.sel){
    alert('请先选择币种');
    return;
  }
  const limitInput=document.getElementById('big-trade-history-limit');
  const limit=Math.max(1,Math.min(500,Number(limitInput?.value||120)||120));
  const from=parsePanelHistoryInput('big-trade-history-from');
  const to=parsePanelHistoryInput('big-trade-history-to');
  if(from!=null && to!=null && from>to){
    bigTradeHistoryStatus('开始时间不能大于结束时间','var(--r)');
    return;
  }
  S.bigTradeHistory.loading=true;
  S.bigTradeHistory.error='';
  S.bigTradeHistory.items=[];
  S.bigTradeHistory.stats=null;
  S.bigTradeHistory.symbol=S.sel;
  S.bigTradeHistory.range={from:from||0,to:to||0,limit};
  renderBigTradeHistoryStats();
  renderBigTradeHistoryList();
  bigTradeHistoryStatus(`正在查询 ${S.sel} 历史大单...`);
  const params=new URLSearchParams();
  params.set('limit',String(limit));
  if(from!=null)params.set('from',String(from));
  if(to!=null)params.set('to',String(to));
  try{
    const [itemsRes,statsRes]=await Promise.all([
      apiFetch(`/api/big-trades/${encodeURIComponent(S.sel)}?${params.toString()}`),
      apiFetch(`/api/big-trades/stats/${encodeURIComponent(S.sel)}?${params.toString()}`)
    ]);
    if(!itemsRes.ok || !statsRes.ok){
      const code=!itemsRes.ok?itemsRes.status:statsRes.status;
      const msg=code===403?'当前账号无权查看该币种历史大单':code===400?'时间区间参数不合法':'历史大单查询失败';
      throw new Error(msg);
    }
    const [items,stats]=await Promise.all([itemsRes.json(),statsRes.json()]);
    S.bigTradeHistory.items=(Array.isArray(items)?items:[]).map(item=>({
      ...item,
      trade_time_label:formatPanelHistoryLabel(item?.trade_time||item?.trade_ts||'')
    }));
    S.bigTradeHistory.stats=stats||null;
    bigTradeHistoryStatus(`已加载 ${S.bigTradeHistory.items.length} 条 ${S.sel} 历史大单。`,S.bigTradeHistory.items.length?'var(--t2)':'var(--t3)');
  }catch(err){
    S.bigTradeHistory.error=String(err?.message||'历史大单查询失败');
    bigTradeHistoryStatus(S.bigTradeHistory.error,'var(--r)');
  }finally{
    S.bigTradeHistory.loading=false;
    renderBigTradeHistoryStats();
    renderBigTradeHistoryList();
  }
}
function openBigTradeHistory(){
  if(!S.sel){
    alert('请先选择币种');
    return;
  }
  const modal=ensureFloatingModal(bigTradeHistoryModal());
  if(!modal)return;
  modal.style.display='flex';
  document.body.style.overflow='hidden';
  const subtitle=document.getElementById('big-trade-history-subtitle');
  if(subtitle)subtitle.textContent=`${S.sel} 大单历史查询。返回独立大单事件明细和统计分析。`;
  const range=S.bigTradeHistory?.range||{};
  if(!range.from || !range.to){
    setBigTradeHistoryPreset(120);
  }else{
    const from=document.getElementById('big-trade-history-from');
    const to=document.getElementById('big-trade-history-to');
    if(from)from.value=formatPanelHistoryInput(range.from);
    if(to)to.value=formatPanelHistoryInput(range.to);
  }
  const limitInput=document.getElementById('big-trade-history-limit');
  if(limitInput)limitInput.value=String(range.limit||120);
  renderBigTradeHistoryStats();
  renderBigTradeHistoryList();
  if(!Array.isArray(S.bigTradeHistory?.items) || !S.bigTradeHistory.items.length || S.bigTradeHistory.symbol!==S.sel){
    loadBigTradeHistory();
  }else{
    bigTradeHistoryStatus(`已载入 ${S.bigTradeHistory.items.length} 条 ${S.sel} 历史大单。`);
  }
}
function closeBigTradeHistory(){
  const modal=bigTradeHistoryModal();
  if(!modal)return;
  modal.style.display='none';
  document.body.style.overflow='';
}
function syncPanelReplayUi(){
  const clearBtn=document.getElementById('panel-history-clear');
  const historyBtn=document.getElementById('rb-history');
  const replay=getActivePanelReplay();
  if(clearBtn)clearBtn.style.display=replay?'inline-flex':'none';
  if(historyBtn)historyBtn.textContent=replay?'历史回放中':'历史回放';
}
function setPanelHistoryPreset(minutes){
  const end=Date.now();
  const start=end-Math.max(1,Number(minutes)||120)*60000;
  const from=document.getElementById('panel-history-from');
  const to=document.getElementById('panel-history-to');
  if(from)from.value=formatPanelHistoryInput(start);
  if(to)to.value=formatPanelHistoryInput(end);
}
function renderPanelHistoryList(){
  const list=document.getElementById('panel-history-list');
  if(!list)return;
  const history=Array.isArray(S.panelReplay?.history)?S.panelReplay.history:[];
  const replay=getActivePanelReplay();
  if(S.panelReplay?.loading){
    list.innerHTML='<div class="loading-empty">正在查询历史快照...</div>';
    return;
  }
  if(S.panelReplay?.error){
    list.innerHTML=`<div class="loading-empty" style="color:var(--r)">${escapePanelReplayHtml(S.panelReplay.error)}</div>`;
    return;
  }
  if(!history.length){
    list.innerHTML='<div class="loading-empty">当前区间没有分析面板快照。</div>';
    return;
  }
  list.innerHTML=history.map((item,index)=>{
    const active=replay && ((replay.event_ts||0)===(item.event_ts||0)) && replay.update_count===item.update_count;
    const change=Number(item.change_24h_pct||0);
    const score=Math.max(Number(item.pump_score||0),Number(item.dump_score||0));
    const tone=change>=0?'var(--g)':'var(--r)';
    return `<div style="display:grid;grid-template-columns:minmax(0,1.45fr) minmax(0,1fr) auto;gap:12px;align-items:center;padding:14px 16px;margin-bottom:10px;border-radius:14px;border:1px solid ${active?'rgba(14,203,129,.32)':'rgba(148,163,184,.12)'};background:${active?'rgba(14,203,129,.08)':'rgba(255,255,255,.02)'}">
      <div style="min-width:0">
        <div style="display:flex;gap:10px;align-items:center;flex-wrap:wrap">
          <span style="font-size:13px;font-weight:700;color:var(--t1)">${escapePanelReplayHtml(item.event_time||'--')}</span>
          <span style="font-size:12px;color:${tone}">${change>=0?'+':''}${change.toFixed(2)}%</span>
          <span style="font-size:12px;color:var(--t3)">更新 ${escapePanelReplayHtml(item.update_count)}</span>
          ${active?'<span style="font-size:12px;color:var(--g)">当前回放</span>':''}
        </div>
        <div style="margin-top:7px;font-size:13px;color:var(--t2);line-height:1.55">${escapePanelReplayHtml(item.signal_reason||item.status_summary||'--')}</div>
      </div>
      <div style="display:grid;grid-template-columns:repeat(2,minmax(0,1fr));gap:8px 12px;font-size:12px;color:var(--t3)">
        <span>价格 <b style="color:var(--t1)">${escapePanelReplayHtml(fP(Number(item.mid||0),getSymbolState(S.sel)||{price_precision:4}))}</b></span>
        <span>级别 <b style="color:var(--t1)">${escapePanelReplayHtml(item.watch_level||'观察')}</b></span>
        <span>拉盘 <b style="color:var(--g)">${escapePanelReplayHtml(item.pump_score)}</b></span>
        <span>砸盘 <b style="color:var(--r)">${escapePanelReplayHtml(item.dump_score)}</b></span>
        <span>CVD <b style="color:${Number(item.cvd||0)>=0?'var(--g)':'var(--r)'}">${escapePanelReplayHtml(fN(Number(item.cvd||0)))}</b></span>
        <span>主导分 <b style="color:var(--t1)">${escapePanelReplayHtml(score)}</b></span>
      </div>
      <div style="display:flex;gap:8px;align-items:center">
        <button class="cbtn" onclick="applyPanelReplay(${index})">${active?'重新定位':'回放'}</button>
      </div>
    </div>`;
  }).join('');
}
function applyPanelReplay(index){
  const history=Array.isArray(S.panelReplay?.history)?S.panelReplay.history:[];
  const snapshot=history[index];
  if(!snapshot||!S.sel)return;
  S.panelReplay.active=true;
  S.panelReplay.symbol=S.sel;
  S.panelReplay.snapshot=snapshot;
  syncPanelReplayUi();
  renderPanelHistoryList();
  renderDetail(S.sel);
  panelHistoryStatus(`已回放 ${S.sel} ${snapshot.event_time||''}`,'var(--g)');
}
function clearPanelReplay(){
  if(!S.panelReplay)return;
  S.panelReplay.active=false;
  S.panelReplay.symbol='';
  S.panelReplay.snapshot=null;
  syncPanelReplayUi();
  renderPanelHistoryList();
  if(S.sel)renderDetail(S.sel);
  panelHistoryStatus('已退出回放，分析面板恢复实时视图。');
}
async function loadPanelHistory(){
  if(!S.sel){
    alert('请先选择币种');
    return;
  }
  const limitInput=document.getElementById('panel-history-limit');
  const limit=Math.max(1,Math.min(500,Number(limitInput?.value||120)||120));
  const from=parsePanelHistoryInput('panel-history-from');
  const to=parsePanelHistoryInput('panel-history-to');
  if(from!=null && to!=null && from>to){
    panelHistoryStatus('开始时间不能大于结束时间','var(--r)');
    return;
  }
  S.panelReplay.loading=true;
  S.panelReplay.error='';
  S.panelReplay.history=[];
  S.panelReplay.range={from:from||0,to:to||0,limit};
  syncPanelReplayUi();
  renderPanelHistoryList();
  panelHistoryStatus(`正在查询 ${S.sel} 历史快照...`);
  const params=new URLSearchParams();
  params.set('limit',String(limit));
  if(from!=null)params.set('from',String(from));
  if(to!=null)params.set('to',String(to));
  try{
    const res=await apiFetch(`/api/panel/${encodeURIComponent(S.sel)}?${params.toString()}`);
    if(!res.ok){
      const msg=res.status===403?'当前账号无权查看该币种历史快照':res.status===400?'时间区间参数不合法':'历史快照查询失败';
      throw new Error(msg);
    }
    const data=await res.json();
    S.panelReplay.history=(Array.isArray(data)?data:[]).map(item=>({
      ...item,
      event_time:formatPanelHistoryLabel(item?.event_time||item?.event_ts||'')
    }));
    S.panelReplay.error='';
    panelHistoryStatus(`已加载 ${S.panelReplay.history.length} 条 ${S.sel} 分析快照。`,S.panelReplay.history.length?'var(--t2)':'var(--t3)');
  }catch(err){
    S.panelReplay.error=String(err?.message||'历史快照查询失败');
    panelHistoryStatus(S.panelReplay.error,'var(--r)');
  }finally{
    S.panelReplay.loading=false;
    renderPanelHistoryList();
  }
}
function openPanelHistory(){
  if(!S.sel){
    alert('请先选择币种');
    return;
  }
  const modal=panelHistoryModal();
  if(!modal)return;
  modal.style.display='flex';
  document.body.style.overflow='hidden';
  const subtitle=document.getElementById('panel-history-subtitle');
  if(subtitle)subtitle.textContent=`${S.sel} 分析快照回放。只回放分析区字段，不覆盖实时盘口、最新成交与 K 线。`;
  const range=S.panelReplay?.range||{};
  if(!range.from || !range.to){
    setPanelHistoryPreset(120);
  }else{
    const from=document.getElementById('panel-history-from');
    const to=document.getElementById('panel-history-to');
    if(from)from.value=formatPanelHistoryInput(range.from);
    if(to)to.value=formatPanelHistoryInput(range.to);
  }
  const limitInput=document.getElementById('panel-history-limit');
  if(limitInput)limitInput.value=String(range.limit||120);
  syncPanelReplayUi();
  renderPanelHistoryList();
  if(!Array.isArray(S.panelReplay?.history) || !S.panelReplay.history.length || S.panelReplay.symbol!==S.sel){
    S.panelReplay.symbol=S.sel;
    loadPanelHistory();
  }else{
    panelHistoryStatus(`已载入 ${S.panelReplay.history.length} 条 ${S.sel} 历史快照。`);
  }
}
function closePanelHistory(){
  const modal=panelHistoryModal();
  if(!modal)return;
  modal.style.display='none';
  document.body.style.overflow='';
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
window.addEventListener('DOMContentLoaded',()=>{
  bindTradeButtonFx();
  bindTradeSliderTips();
  syncPanelReplayUi();
  const historyModal=panelHistoryModal();
  if(historyModal){
    historyModal.addEventListener('click',ev=>{
      if(ev.target===historyModal)closePanelHistory();
    });
  }
  document.addEventListener('keydown',ev=>{
    if(ev.key==='Escape' && historyModal?.style.display==='block')closePanelHistory();
  });
  const input=document.getElementById('site-search-input');
  if(input){
    input.addEventListener('keydown',ev=>{
      if(ev.key==='Enter'){
        ev.preventDefault();
        searchTopSymbol();
      }
    });
  }
});

// ── WebSocket ─────────────────────────────────────────────────────
function connect(){
  if(S.auth.ws){
    try{
      S.auth.ws.onclose=null;
      S.auth.ws.close();
    }catch(_){}
  }
  const proto=location.protocol==='https:'?'wss':'ws';
  const ws=new WebSocket(`${proto}://${location.host}/ws`);
  ws.binaryType='arraybuffer';
  S.auth.ws=ws;
  ws.onopen=()=>{
    markWsAlive('main');
    document.getElementById('wdot').className='wdot live';
    e('wlbl','实时连接');
  };
  ws.onmessage=ev=>{
    try{
      markWsAlive('main');
      render(normalizeWsSnapshot(decodeWsPayload(ev.data)));
    }catch(_){ }
  };
  ws.onerror=()=>{
    document.getElementById('wdot').className='wdot';
    e('wlbl','连接异常');
    try{ws.close();}catch(_){}
  };
  ws.onclose=()=>{
    if(S.auth.ws!==ws || !S.auth.appReady)return;
    S.auth.wsLastAt=0;
    document.getElementById('wdot').className='wdot';
    // WebSocket 现在对游客开放，断线后也要继续自动重连，
    // 否则公开预览模式会在一次掉线后完全停更。
    e('wlbl',S.auth.user?'重连中...':'公开预览重连中...');
    setTimeout(()=>{
      if(S.auth.ws===ws && S.auth.appReady)connect();
    },2000);
  };
}

function connectionIdleLabel(){
  return S.auth.user?'连接中':'公开预览';
}

function hasActiveSelectedSymbolStream(){
  const ws=S.auth.detailWs;
  return !!(ws && ws.readyState===WebSocket.OPEN && S.auth.detailWsSymbol===S.sel);
}
function selectedSymbolPollMs(){
  if(document.hidden)return 12000;
  return hasActiveSelectedSymbolStream()?DETAIL_WS_FORCE_SYNC_MS:2200;
}

function scheduleSelectedSymbolPoll(delay=selectedSymbolPollMs()){
  if(S.auth.detailPoller){
    clearTimeout(S.auth.detailPoller);
  }
  S.auth.detailPoller=setTimeout(runSelectedSymbolPoll,delay);
}

async function runSelectedSymbolPoll(){
  if(!S.auth.appReady)return;
  if(S.auth.detailBusy){
    scheduleSelectedSymbolPoll(160);
    return;
  }
  if(!S.sel){
    scheduleSelectedSymbolPoll();
    return;
  }
  S.auth.detailBusy=true;
  try{
    await loadSymbolDetail(S.sel,true);
  }finally{
    S.auth.detailBusy=false;
    if(S.auth.appReady)scheduleSelectedSymbolPoll();
  }
}

function buildSelectedSymbolWsUrl(sym){
  const proto=location.protocol==='https:'?'wss':'ws';
  return `${proto}://${location.host}/ws/symbol/${encodeURIComponent(sym)}`;
}

function connectSelectedSymbolStream(sym=S.sel){
  if(!S.auth.appReady||!sym)return;
  if(S.auth.detailWsSymbol===sym && S.auth.detailWs && (S.auth.detailWs.readyState===WebSocket.OPEN || S.auth.detailWs.readyState===WebSocket.CONNECTING)){
    return;
  }
  if(S.auth.detailWsRetry){
    clearTimeout(S.auth.detailWsRetry);
    S.auth.detailWsRetry=null;
  }
  if(S.auth.detailWs){
    try{
      S.auth.detailWs.onclose=null;
      S.auth.detailWs.close();
    }catch(_){}
  }
  const ws=new WebSocket(buildSelectedSymbolWsUrl(sym));
  ws.binaryType='arraybuffer';
  S.auth.detailWs=ws;
  S.auth.detailWsSymbol=sym;
  ws.onopen=()=>{
    markWsAlive('detail');
    S.auth.detailWsSig='';
    S.auth.detailWsSigAt=0;
    S.auth.detailWsLagSince=0;
  };
  ws.onmessage=ev=>{
    try{
      markWsAlive('detail');
      const detail=decodeWsPayload(ev.data);
      if(!detail||detail.symbol!==S.sel)return;
      const now=Date.now();
      const prev=getSymbolState(detail.symbol);
      const prevUpdate=Number(prev?.update_count||0);
      const nextUpdate=Number(detail.update_count||0);
      const sig=detailStreamSignature(detail);
      if(sig===S.auth.detailWsSig){
        if(!S.auth.detailWsSigAt)S.auth.detailWsSigAt=now;
      }else{
        S.auth.detailWsSig=sig;
        S.auth.detailWsSigAt=now;
      }
      if(prevUpdate>0 && nextUpdate>0 && nextUpdate+2<prevUpdate){
        if(!S.auth.detailWsLagSince)S.auth.detailWsLagSince=now;
      }else{
        S.auth.detailWsLagSince=0;
      }
      upsertSymbolDetail(detail);
      S.tr[detail.symbol]=(detail.recent_trades||[]).map(t=>({
        p:t.p,
        q:t.q,
        buy:!!t.buy,
        t:typeof t.t==='number'?new Date(t.t).toLocaleTimeString('zh-CN',{hour12:false}):String(t.t||'--')
      }));
      S.ui.detailKey='';
      renderDetail(detail.symbol);
      updOHLCV();
      if(
        (S.auth.detailWsSigAt && now-S.auth.detailWsSigAt>=DETAIL_WS_STUCK_RECONNECT_MS)
        || (S.auth.detailWsLagSince && now-S.auth.detailWsLagSince>=DETAIL_WS_STUCK_RECONNECT_MS)
      ){
        S.auth.detailWsSig='';
        S.auth.detailWsSigAt=0;
        S.auth.detailWsLagSince=0;
        try{ws.close();}catch(_){}
      }
    }catch(_){}
  };
  ws.onerror=()=>{try{ws.close();}catch(_){}};
  ws.onclose=()=>{
    if(S.auth.detailWs!==ws || !S.auth.appReady)return;
    S.auth.detailWs=null;
    S.auth.detailWsLastAt=0;
    S.auth.detailWsSig='';
    S.auth.detailWsSigAt=0;
    S.auth.detailWsLagSince=0;
    if(S.auth.detailWsSymbol!==sym)return;
    S.auth.detailWsRetry=setTimeout(()=>{
      if(S.auth.appReady && S.sel===sym)connectSelectedSymbolStream(sym);
    },800);
  };
}

function stopDashboardApp(){
  S.auth.appReady=false;
  if(S.auth.detailPoller){
    clearTimeout(S.auth.detailPoller);
    S.auth.detailPoller=null;
  }
  if(S.auth.detailWsRetry){
    clearTimeout(S.auth.detailWsRetry);
    S.auth.detailWsRetry=null;
  }
  if(S.auth.detailWs){
    try{
      S.auth.detailWs.close();
    }catch(_){}
    S.auth.detailWs=null;
  }
  S.auth.detailWsSymbol='';
  S.auth.wsLastAt=0;
  S.auth.detailWsLastAt=0;
  S.auth.detailBusy=false;
  if(S.auth.wsHealthTimer){
    clearTimeout(S.auth.wsHealthTimer);
    S.auth.wsHealthTimer=null;
  }
  if(S.auth.ws){
    try{S.auth.ws.close();}catch(_){}
    S.auth.ws=null;
  }
  document.getElementById('wdot').className='wdot';
  e('wlbl',connectionIdleLabel());
}

function startDashboardApp(){
  if(S.auth.appReady)return;
  window.__bbBootAt=Date.now();
  if(!S.auth.domBound){
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
    S.auth.domBound=true;
  }
  renderOrders();
  e('wlbl',connectionIdleLabel());
  ensureSymbolUniverse();
  if(typeof ensureTradingView==='function'){
    ensureTradingView().catch(()=>{});
  }
  apiFetch('/api/state').then(r=>r.json()).then(d=>render(normalizeWsSnapshot(d))).catch(()=>{});
  connect();
  S.auth.appReady=true;
  scheduleWsHealthCheck();
  scheduleSelectedSymbolPoll(220);
  if(S.sel)connectSelectedSymbolStream(S.sel);
}

window.addEventListener('DOMContentLoaded',()=>{
  if(typeof bootAuth==='function')bootAuth();
});
document.addEventListener('visibilitychange',()=>{
  if(S.auth.appReady)scheduleSelectedSymbolPoll(120);
});
