-- 币种注册表初始化脚本
-- 作用：
-- 1. 创建 market.symbol_registry，保存全部 USDT 币种
-- 2. 通过 enabled 控制是否参与监控
-- 3. 保留交易所原始状态 exchange_status，便于运维排查

create schema if not exists market;

create or replace function market.set_updated_at()
returns trigger
language plpgsql
as $$
begin
  new.updated_at = now();
  return new;
end;
$$;

create table if not exists market.symbol_registry (
  symbol varchar(32) primary key,
  base_asset varchar(32) not null,
  quote_asset varchar(16) not null,
  exchange_status varchar(16) not null,
  price_precision integer not null default 0,
  quantity_precision integer not null default 0,
  enabled boolean not null default true,
  visible_public boolean not null default false,
  visible_member boolean not null default false,
  visible_subscriber boolean not null default true,
  created_at timestamptz not null default now(),
  updated_at timestamptz not null default now()
);

-- 旧数据库字段补全（必须在 COMMENT 之前执行，否则 COMMENT 会因列不存在而中止整个 batch）
alter table market.symbol_registry
  add column if not exists price_precision integer not null default 0;

alter table market.symbol_registry
  add column if not exists quantity_precision integer not null default 0;

alter table market.symbol_registry
  add column if not exists visible_public boolean not null default false;

alter table market.symbol_registry
  add column if not exists visible_member boolean not null default false;

alter table market.symbol_registry
  add column if not exists visible_subscriber boolean not null default true;

comment on table market.symbol_registry is '币种注册表：保存全部 USDT 交易对及启用状态';
comment on column market.symbol_registry.symbol is '标准交易对代码，如 BTCUSDT';
comment on column market.symbol_registry.base_asset is '基础币种，如 BTC';
comment on column market.symbol_registry.quote_asset is '计价币种，如 USDT';
comment on column market.symbol_registry.exchange_status is '交易所返回的原始状态，如 TRADING';
comment on column market.symbol_registry.price_precision is '交易所价格精度，优先由 PRICE_FILTER.tickSize 推导';
comment on column market.symbol_registry.quantity_precision is '交易所数量精度，优先由 LOT_SIZE.stepSize 推导';
comment on column market.symbol_registry.enabled is '本系统是否启用该币种监控';
comment on column market.symbol_registry.visible_public is '未登录用户是否可见';
comment on column market.symbol_registry.visible_member is '已登录未订阅用户是否可见';
comment on column market.symbol_registry.visible_subscriber is '已订阅用户是否可见';

create index if not exists idx_market_symbol_registry_enabled
  on market.symbol_registry(enabled, exchange_status, symbol);

drop trigger if exists trg_market_symbol_registry_updated_at on market.symbol_registry;
create trigger trg_market_symbol_registry_updated_at
before update on market.symbol_registry
for each row
execute function market.set_updated_at();

-- 套餐与币种的多对多关联表
-- 每行表示某个套餐可以访问某个币种
-- 示例：INSERT INTO market.symbol_plan_access VALUES ('BTCUSDT', 'basic');
create table if not exists market.symbol_plan_access (
  symbol    varchar(32) not null,
  plan_code varchar(32) not null,
  created_at timestamptz not null default now(),
  primary key (symbol, plan_code),
  foreign key (symbol) references market.symbol_registry(symbol) on delete cascade
);

comment on table market.symbol_plan_access is '套餐币种权限表：定义每个订阅套餐可访问的币种';
comment on column market.symbol_plan_access.symbol    is '币种代码，如 BTCUSDT';
comment on column market.symbol_plan_access.plan_code is '套餐代码，与 auth.subscription_plan.code 对应';

create index if not exists idx_symbol_plan_access_plan_code
  on market.symbol_plan_access(plan_code, symbol);
