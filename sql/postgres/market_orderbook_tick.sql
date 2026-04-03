-- 订单簿变化事件表初始化脚本
-- 作用：
-- 1. 只记录"有意义"的变化：qty 归零（撤单）、新增档位、qty 变化 >= 5% 的修改
-- 2. 保留最近 14 天数据，后台定时清理
-- 3. 作为 detect_rapid_cancellations / detect_order_surge 的数据源

create schema if not exists market;

create table if not exists market.orderbook_tick (
  id           bigserial primary key,
  symbol       varchar(32)   not null,
  event_time   timestamptz   not null,
  side         varchar(4)    not null,   -- 'bid' | 'ask'
  price        double precision not null,
  qty_before   double precision not null default 0.0,
  qty_after    double precision not null default 0.0,
  change_type  varchar(8)    not null,   -- 'add' | 'cancel' | 'modify'
  created_at   timestamptz   not null default now()
);

comment on table market.orderbook_tick            is '订单簿逐档有意义变化：仅记录新增、撤单、qty 变化 >=5% 的档位，保留 14 天';
comment on column market.orderbook_tick.symbol     is '交易对，如 BTCUSDT';
comment on column market.orderbook_tick.event_time is '该变化对应的 Binance 事件时间（非写库时间）';
comment on column market.orderbook_tick.side       is '买盘 bid / 卖盘 ask';
comment on column market.orderbook_tick.price      is '档位价格';
comment on column market.orderbook_tick.qty_before is '变化前挂单量（新增档位时为 0）';
comment on column market.orderbook_tick.qty_after  is '变化后挂单量（撤单时为 0）';
comment on column market.orderbook_tick.change_type is '变化类型：add=新增档位 / cancel=撤单 / modify=数量变化>=5%';

-- 按币种 + 时间检索（供撤单检测）
create index if not exists idx_market_orderbook_tick_symbol_time
  on market.orderbook_tick(symbol, event_time desc);

-- 按 change_type 筛选撤单
create index if not exists idx_market_orderbook_tick_symbol_type_time
  on market.orderbook_tick(symbol, change_type, event_time desc);

-- 自动清理：保留最近 14 天
-- 由 signal_resolver 或专属清理任务调用，也可设置 pg_cron
-- delete from market.orderbook_tick where created_at < now() - interval '14 days';
