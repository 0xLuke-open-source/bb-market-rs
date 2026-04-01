-- 大单事件表初始化脚本
-- 作用：
-- 1. 创建 market.big_trade_event，保存被判定为“大单”的成交事件
-- 2. 使用 (symbol, agg_trade_id) 去重，避免重连和重复消费产生脏数据
-- 3. 保留阈值字段，方便后续做大单识别口径分析

create schema if not exists market;

create table if not exists market.big_trade_event (
  symbol varchar(32) not null,
  agg_trade_id bigint not null,
  event_time timestamptz not null,
  trade_time timestamptz not null,
  price double precision not null,
  quantity double precision not null,
  quote_quantity double precision not null,
  threshold_quantity double precision not null,
  is_taker_buy boolean not null,
  is_buyer_maker boolean not null,
  created_at timestamptz not null default now(),
  primary key (symbol, agg_trade_id)
);

comment on table market.big_trade_event is '大单事件表：保存被实时识别为大单的成交事件';
comment on column market.big_trade_event.symbol is '交易对，如 BTCUSDT';
comment on column market.big_trade_event.agg_trade_id is 'Binance aggTrade 唯一 ID';
comment on column market.big_trade_event.event_time is 'WebSocket 事件时间';
comment on column market.big_trade_event.trade_time is '成交发生时间';
comment on column market.big_trade_event.price is '成交价';
comment on column market.big_trade_event.quantity is '成交数量（基础资产）';
comment on column market.big_trade_event.quote_quantity is '成交额（price * quantity）';
comment on column market.big_trade_event.threshold_quantity is '当前大单识别阈值数量';
comment on column market.big_trade_event.is_taker_buy is '是否主动买入';
comment on column market.big_trade_event.is_buyer_maker is 'Binance 原始字段 m';

create index if not exists idx_market_big_trade_event_symbol_time
  on market.big_trade_event(symbol, trade_time desc);

create index if not exists idx_market_big_trade_event_trade_time
  on market.big_trade_event(trade_time desc);
