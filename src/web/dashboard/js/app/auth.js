// ── 登录 / 订阅 ───────────────────────────────────────────────────
// 这一组函数只负责“用户访问态”：
// 1. 登录 / 注册 / 退出
// 2. 本地订阅套餐选择
// 3. 把后端返回的访问权限同步到界面组件
function setAuthMessage(msg='',kind=''){
  const el=document.getElementById('auth-msg');
  if(!el)return;
  el.textContent=msg;
  el.className=`auth-msg${kind?` ${kind}`:''}`;
}

function openAuthModal(mode='login'){
  document.getElementById('auth-overlay')?.classList.add('show');
  switchAuthMode(mode);
}

function closeAuthModal(){
  document.getElementById('auth-overlay')?.classList.remove('show');
  setAuthMessage('');
}

function switchAuthMode(mode){
  const loginMode=mode!=='register';
  document.getElementById('auth-tab-login')?.classList.toggle('act',loginMode);
  document.getElementById('auth-tab-register')?.classList.toggle('act',!loginMode);
  document.getElementById('auth-login-form')?.classList.toggle('show',loginMode);
  document.getElementById('auth-register-form')?.classList.toggle('show',!loginMode);
  setAuthMessage('');
}

function setHeroTrialMessage(msg='',kind=''){
  const el=document.getElementById('home-hero-form-msg');
  if(!el)return;
  el.textContent=msg;
  el.className=`home-hero-form-msg${kind?` ${kind}`:''}`;
}

function renderSubscriptionPlans(){
  const root=document.getElementById('sub-plan-list');
  if(!root)return;
  const plans=S.auth.plans||[];
  root.innerHTML=plans.map(plan=>`
    <div class="sub-plan ${S.auth.selectedPlan===plan.code?'act':''}" onclick="selectSubscriptionPlan('${plan.code}')">
      <div class="sub-plan-top">
        <div>
          <div class="sub-plan-name">${plan.label}</div>
          <div class="sub-plan-days">${plan.days} 天有效期</div>
        </div>
      </div>
      <div class="sub-plan-price">${plan.price_text}</div>
      <div class="sub-plan-desc">${plan.description}</div>
    </div>
  `).join('');
}

function selectSubscriptionPlan(code){
  S.auth.selectedPlan=code;
  renderSubscriptionPlans();
}

function applyAccessState(authStatus,userOverride=null){
  const prevAccessSignature=[
    S.access.authenticated,
    S.access.subscribed,
    S.access.full_access,
    S.access.subscription_plan,
    S.access.subscription_expires_at,
    S.auth.user?.username||''
  ].join('|');
  // 后端有两类返回：
  // 1. /api/auth/* 返回 AuthStatusResponse
  // 2. /api/state /ws 返回 access 快照
  // 这里统一兼容两种结构，避免字段在多次渲染后被意外清空。
  const user=userOverride??authStatus?.user??null;
  S.auth.user=user;
  S.auth.ready=true;
  S.access.authenticated=!!authStatus?.authenticated;
  S.access.subscribed=!!authStatus?.subscribed;
  S.access.full_access=!!authStatus?.full_access;
  S.access.visible_symbols=authStatus?.visible_symbols??S.access.visible_symbols??0;
  S.access.total_symbols=authStatus?.total_symbols??S.access.total_symbols??0;
  S.access.symbol_limit=authStatus?.symbol_limit??S.access.symbol_limit??null;
  if(authStatus&&Object.prototype.hasOwnProperty.call(authStatus,'subscription_plan')){
    S.access.subscription_plan=authStatus.subscription_plan??null;
  }
  if(authStatus&&Object.prototype.hasOwnProperty.call(authStatus,'subscription_expires_at')){
    S.access.subscription_expires_at=authStatus.subscription_expires_at??null;
  }
  if(authStatus&&Object.prototype.hasOwnProperty.call(authStatus,'message')){
    S.access.message=authStatus.message||'';
  }

  const userLabel=document.getElementById('auth-user');
  const openBtn=document.getElementById('auth-open');
  const subscribeBtn=document.getElementById('auth-subscribe');
  const logoutBtn=document.getElementById('auth-logout');
  const subTitle=document.querySelector('.sub-plans-sub');

  if(userLabel){
    if(user){
      userLabel.textContent=user.display_name||user.username;
      userLabel.title=S.access.full_access
        ?`已解锁全部 ${S.access.total_symbols||0} 个币种`
        :'已登录，可订阅解锁全部币种';
    }else{
      const visible=S.access.visible_symbols||0;
      const total=S.access.total_symbols||0;
      userLabel.textContent=total?`访客 · ${visible}/${total}`:'访客';
      userLabel.title=S.access.message||'未登录状态下仅展示部分币种';
    }
  }
  if(openBtn)openBtn.style.display=user?'none':'inline-flex';
  if(logoutBtn)logoutBtn.style.display=user?'inline-flex':'none';

  const canSubscribe=!!user && !authStatus?.subscribed;
  if(subscribeBtn){
    subscribeBtn.style.display=canSubscribe?'inline-flex':'none';
    subscribeBtn.textContent='订阅';
  }
  if(subTitle){
    if(S.access.full_access && S.access.subscription_expires_at){
      subTitle.textContent=`当前套餐：${S.access.subscription_plan||'pro'}，到期时间：${new Date(S.access.subscription_expires_at).toLocaleString('zh-CN',{hour12:false})}`;
    }else if(S.access.full_access){
      subTitle.textContent=`当前套餐：${S.access.subscription_plan||'legacy'}，已处于长期解锁状态`;
    }else{
      subTitle.textContent='订阅成功后解锁全部币种与完整实时推送';
    }
  }

  if(authStatus?.full_access){
    closeAuthModal();
  }

  const nextAccessSignature=[
    S.access.authenticated,
    S.access.subscribed,
    S.access.full_access,
    S.access.subscription_plan,
    S.access.subscription_expires_at,
    S.auth.user?.username||''
  ].join('|');
  if(prevAccessSignature!==nextAccessSignature && typeof refreshSitePage==='function'){
    refreshSitePage();
  }
}

function handleAuthExpired(){
  if(typeof applyFavoriteSymbols==='function')applyFavoriteSymbols([],false);
  applyAccessState({authenticated:false,subscribed:false,full_access:false,symbol_limit:10,user:null});
  setAuthMessage('登录状态已失效，请重新登录。','err');
  openAuthModal('login');
  connect();
  apiFetch('/api/state').then(r=>r.json()).then(d=>render(d)).catch(()=>{});
}

async function requestAuthJson(url,payload){
  const res=await fetch(url,{
    method:'POST',
    headers:{'Content-Type':'application/json'},
    body:JSON.stringify(payload)
  });
  const json=await res.json();
  return {res,json};
}

async function login(ev){
  if(ev)ev.preventDefault();
  const username=document.getElementById('login-username').value.trim();
  const password=document.getElementById('login-password').value;
  setAuthMessage('');
  const {json}=await requestAuthJson('/api/auth/login',{username,password});
  if(!json.ok){
    setAuthMessage(json.message||'登录失败','err');
    return;
  }
  applyAccessState(json.data,json.data.user||null);
  await syncFavoriteSymbolsFromServer();
  closeAuthModal();
  connect();
  apiFetch('/api/state').then(r=>r.json()).then(d=>render(d)).catch(()=>{});
}

async function register(ev){
  if(ev)ev.preventDefault();
  const username=document.getElementById('register-username').value.trim();
  const display_name=document.getElementById('register-display-name').value.trim();
  const password=document.getElementById('register-password').value;
  const password2=document.getElementById('register-password-confirm').value;
  if(password!==password2){
    setAuthMessage('两次输入的密码不一致。','err');
    return;
  }
  setAuthMessage('');
  const {json}=await requestAuthJson('/api/auth/register',{username,password,display_name});
  if(!json.ok){
    setAuthMessage(json.message||'注册失败','err');
    return;
  }
  applyAccessState(json.data,json.data.user||null);
  await syncFavoriteSymbolsFromServer();
  setAuthMessage('注册成功，当前账户尚未订阅，仍只显示部分币种。');
  closeAuthModal();
  connect();
  apiFetch('/api/state').then(r=>r.json()).then(d=>render(d)).catch(()=>{});
}

function openHeroLogin(){
  const heroUsername=document.getElementById('hero-trial-username')?.value.trim();
  openAuthModal('login');
  if(heroUsername){
    const loginInput=document.getElementById('login-username');
    if(loginInput)loginInput.value=heroUsername;
  }
}

async function submitHeroTrial(ev){
  if(ev)ev.preventDefault();
  const username=document.getElementById('hero-trial-username')?.value.trim()||'';
  const display_name=document.getElementById('hero-trial-display-name')?.value.trim()||'';
  const password=document.getElementById('hero-trial-password')?.value||'';
  if(!username){
    setHeroTrialMessage('请输入用户名。','err');
    return;
  }
  if(password.length<6){
    setHeroTrialMessage('密码至少 6 位。','err');
    return;
  }
  setHeroTrialMessage('');
  const {json}=await requestAuthJson('/api/auth/register',{username,password,display_name});
  if(!json.ok){
    setHeroTrialMessage(json.message||'创建试用账户失败。','err');
    return;
  }
  applyAccessState(json.data,json.data.user||null);
  await syncFavoriteSymbolsFromServer();
  setHeroTrialMessage('账户已创建，正在进入 AI 盯盘。','ok');
  connect();
  apiFetch('/api/state').then(r=>r.json()).then(d=>render(d)).catch(()=>{});
  switchSitePage('ai');
}

async function subscribeNow(){
  if(!S.auth.user){
    openAuthModal('login');
    setAuthMessage('请先登录，再订阅解锁全部币种。');
    return;
  }
  if(!S.auth.selectedPlan){
    setAuthMessage('当前没有可用套餐，请稍后重试。','err');
    openAuthModal('login');
    return;
  }
  const res=await postJson('/api/auth/subscribe',{plan_code:S.auth.selectedPlan});
  if(!res.ok){
    alert(res.message||'订阅失败');
    return;
  }
  applyAccessState(res.data,res.data.user||S.auth.user);
  connect();
  apiFetch('/api/state').then(r=>r.json()).then(d=>render(d)).catch(()=>{});
}

async function syncFavoriteSymbolsFromServer(){
  if(!S.auth.user){
    if(typeof applyFavoriteSymbols==='function')applyFavoriteSymbols([],false);
    return;
  }
  try{
    const res=await apiFetch('/api/auth/favorites');
    const json=await res.json();
    if(json.ok){
      if(typeof applyFavoriteSymbols==='function')applyFavoriteSymbols(json.data||[],false);
      return;
    }
  }catch(_){}
  if(typeof applyFavoriteSymbols==='function')applyFavoriteSymbols([],false);
}

async function logout(){
  try{
    await fetch('/api/auth/logout',{method:'POST'});
  }catch(_){}
  if(typeof applyFavoriteSymbols==='function')applyFavoriteSymbols([],false);
  applyAccessState({authenticated:false,subscribed:false,full_access:false,symbol_limit:10,user:null});
  connect();
  apiFetch('/api/state').then(r=>r.json()).then(d=>render(d)).catch(()=>{});
}

async function bootAuth(){
  document.getElementById('auth-login-form')?.addEventListener('submit',login);
  document.getElementById('auth-register-form')?.addEventListener('submit',register);
  switchAuthMode('login');
  startDashboardApp();
  if(typeof initPortal==='function')initPortal();

  try{
    S.auth.plans=await fetch('/api/auth/plans').then(r=>r.json());
  }catch(_){
    S.auth.plans=[];
  }
  if(S.auth.plans.length){
    S.auth.selectedPlan=S.auth.plans[1]?.code||S.auth.plans[0].code;
  }
  renderSubscriptionPlans();

  try{
    const res=await fetch('/api/auth/me');
    const data=await res.json();
    applyAccessState(data,data.user||null);
    await syncFavoriteSymbolsFromServer();
  }catch(_){
    if(typeof applyFavoriteSymbols==='function')applyFavoriteSymbols([],false);
    applyAccessState({authenticated:false,subscribed:false,full_access:false,symbol_limit:10,user:null});
  }
}
