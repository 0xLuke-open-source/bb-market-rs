-- 最新成交表初始化脚本
-- 作用：
-- 1. 创建 market.recent_trade，保存逐笔 aggTrade 成交
-- 2. 使用 (symbol, agg_trade_id) 去重，避免重连或重复消费产生脏数据
-- 3. 建立按币种、时间检索的常用索引

create schema if not exists market;

create table if not exists market.recent_trade (
  symbol varchar(32) not null,
  agg_trade_id bigint not null,
  event_time timestamptz not null,
  trade_time timestamptz not null,
  price double precision not null,
  quantity double precision not null,
  quote_quantity double precision not null,
  is_taker_buy boolean not null,
  is_buyer_maker boolean not null,
  created_at timestamptz not null default now(),
  primary key (symbol, agg_trade_id)
);

comment on table market.recent_trade is '最新成交表：保存逐笔 aggTrade 成交记录';
comment on column market.recent_trade.symbol is '交易对，如 BTCUSDT';
comment on column market.recent_trade.agg_trade_id is 'Binance aggTrade 唯一 ID';
comment on column market.recent_trade.event_time is 'WebSocket 事件时间';
comment on column market.recent_trade.trade_time is '成交发生时间';
comment on column market.recent_trade.price is '成交价';
comment on column market.recent_trade.quantity is '成交数量（基础资产）';
comment on column market.recent_trade.quote_quantity is '成交额（price * quantity）';
comment on column market.recent_trade.is_taker_buy is '是否主动买入';
comment on column market.recent_trade.is_buyer_maker is 'Binance 原始字段 m';

create index if not exists idx_market_recent_trade_symbol_time
  on market.recent_trade(symbol, trade_time desc);

create index if not exists idx_market_recent_trade_trade_time
  on market.recent_trade(trade_time desc);
