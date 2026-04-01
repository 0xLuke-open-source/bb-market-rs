-- 认证与订阅初始化脚本
-- 作用：
-- 1. 创建 identity schema
-- 2. 创建用户、会话、订阅套餐、订阅记录四张核心表
-- 3. 创建 updated_at 自动更新时间触发器
-- 4. 初始化默认订阅套餐数据
--
-- 执行方式：
-- psql -h localhost -p 5432 -U root -d bb_market -f sql/postgres/auth_identity.sql

create schema if not exists identity;

-- 通用触发器：更新数据时自动刷新 updated_at
create or replace function identity.set_updated_at()
returns trigger
language plpgsql
as $$
begin
  new.updated_at = now();
  return new;
end;
$$;

-- 用户主表：保存注册账户、密码摘要、订阅状态
create table if not exists identity.user_account (
  user_id uuid primary key,
  username varchar(32) not null unique,
  display_name varchar(64) not null,
  salt varchar(128) not null,
  password_hash varchar(128) not null,
  subscribed boolean not null default false,
  subscription_plan varchar(32),
  subscription_expires_at timestamptz,
  created_at timestamptz not null default now(),
  updated_at timestamptz not null default now(),
  last_login_at timestamptz
);

comment on table identity.user_account is '用户账户表：保存注册账户、密码哈希和订阅状态';
comment on column identity.user_account.user_id is '用户主键 UUID';
comment on column identity.user_account.username is '登录用户名，唯一';
comment on column identity.user_account.display_name is '前端展示名称';
comment on column identity.user_account.salt is '密码加盐';
comment on column identity.user_account.password_hash is '密码哈希摘要';
comment on column identity.user_account.subscribed is '是否处于订阅状态';
comment on column identity.user_account.subscription_plan is '当前订阅套餐编码';
comment on column identity.user_account.subscription_expires_at is '当前订阅到期时间';
comment on column identity.user_account.last_login_at is '最近一次登录时间';

-- 登录会话表：保存 cookie 对应的 session token
create table if not exists identity.user_session (
  session_token varchar(64) primary key,
  user_id uuid not null references identity.user_account(user_id) on delete cascade,
  username varchar(32) not null,
  created_at timestamptz not null default now(),
  expires_at timestamptz not null
);

comment on table identity.user_session is '用户会话表：保存登录 session token';
comment on column identity.user_session.session_token is '会话 token，写入浏览器 cookie';
comment on column identity.user_session.expires_at is '会话过期时间';

-- 套餐配置表：保存前台可售卖的订阅套餐
create table if not exists identity.subscription_plan (
  plan_code varchar(32) primary key,
  plan_label varchar(64) not null,
  duration_days integer not null,
  price_text varchar(64) not null,
  description text not null,
  sort_order integer not null default 0,
  active boolean not null default true,
  created_at timestamptz not null default now(),
  updated_at timestamptz not null default now()
);

comment on table identity.subscription_plan is '订阅套餐表：定义周卡、月卡、年卡等套餐';
comment on column identity.subscription_plan.plan_code is '套餐编码';
comment on column identity.subscription_plan.duration_days is '套餐有效天数';
comment on column identity.subscription_plan.active is '是否启用';

-- 用户订阅历史表：保存每次订阅激活记录
create table if not exists identity.user_subscription (
  subscription_id uuid primary key,
  user_id uuid not null references identity.user_account(user_id) on delete cascade,
  username varchar(32) not null,
  plan_code varchar(32) not null references identity.subscription_plan(plan_code),
  started_at timestamptz not null,
  expires_at timestamptz not null,
  status varchar(16) not null,
  created_at timestamptz not null default now(),
  updated_at timestamptz not null default now(),
  check (status in ('ACTIVE', 'SUPERSEDED', 'EXPIRED', 'CANCELLED'))
);

comment on table identity.user_subscription is '用户订阅记录表：保存订阅历史和状态流转';
comment on column identity.user_subscription.status is '订阅状态：ACTIVE/SUPERSEDED/EXPIRED/CANCELLED';
comment on column identity.user_subscription.expires_at is '本次订阅到期时间';

-- 用户收藏表：按账户保存自选币对
create table if not exists identity.user_favorite_symbol (
  user_id uuid not null references identity.user_account(user_id) on delete cascade,
  symbol varchar(32) not null,
  created_at timestamptz not null default now(),
  primary key (user_id, symbol)
);

comment on table identity.user_favorite_symbol is '用户自选币对表：登录后收藏的币对持久化到数据库';
comment on column identity.user_favorite_symbol.symbol is '标准化后的交易对代码，如 BTCUSDT';

-- 常用查询索引
create index if not exists idx_identity_user_session_username
  on identity.user_session(username);
create index if not exists idx_identity_user_session_expires_at
  on identity.user_session(expires_at);
create index if not exists idx_identity_user_subscription_user
  on identity.user_subscription(user_id, status, expires_at desc);
create index if not exists idx_identity_user_account_subscription
  on identity.user_account(subscription_plan, subscription_expires_at desc);
create index if not exists idx_identity_user_favorite_symbol_created
  on identity.user_favorite_symbol(user_id, created_at desc);

-- 自动更新时间触发器
drop trigger if exists trg_identity_user_account_updated_at on identity.user_account;
create trigger trg_identity_user_account_updated_at
before update on identity.user_account
for each row
execute function identity.set_updated_at();

drop trigger if exists trg_identity_subscription_plan_updated_at on identity.subscription_plan;
create trigger trg_identity_subscription_plan_updated_at
before update on identity.subscription_plan
for each row
execute function identity.set_updated_at();

drop trigger if exists trg_identity_user_subscription_updated_at on identity.user_subscription;
create trigger trg_identity_user_subscription_updated_at
before update on identity.user_subscription
for each row
execute function identity.set_updated_at();

-- 初始化默认套餐数据
insert into identity.subscription_plan (
  plan_code, plan_label, duration_days, price_text, description, sort_order, active
) values
  ('pro_week',  'PRO 周卡',  7,   '98 USDT',   '适合短期盯盘，解锁全部币种、全量信号和实时推送。', 10, true),
  ('pro_month', 'PRO 月卡',  30,  '298 USDT',  '适合日常交易使用，30 天内查看全量市场和完整快照。', 20, true),
  ('pro_year',  'PRO 年卡',  365, '1998 USDT', '适合长期使用，全年解锁全部币种和完整实时数据。', 30, true),
  ('legacy',    '历史订阅',  30,  '0 USDT',    '历史迁移订阅占位套餐，不用于前台售卖。', 999, false)
on conflict (plan_code) do update set
  plan_label = excluded.plan_label,
  duration_days = excluded.duration_days,
  price_text = excluded.price_text,
  description = excluded.description,
  sort_order = excluded.sort_order,
  active = excluded.active,
  updated_at = now();
