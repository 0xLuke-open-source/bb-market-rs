-- 订单簿快照初始化脚本
-- 作用：
-- 1. 创建 market schema
-- 2. 创建订单簿快照表，保存 bridge 层扁平化后的盘口快照
-- 3. 建立按币种、时间、update_count 检索的常用索引
--
-- 执行方式：
-- psql -h localhost -p 5432 -U root -d bb_market -f sql/postgres/market_order_book.sql

create schema if not exists market;

create table if not exists market.order_book_snapshot (
  snapshot_id uuid primary key,
  symbol varchar(32) not null,
  event_time timestamptz not null,
  bid double precision not null,
  ask double precision not null,
  mid double precision not null,
  spread_bps double precision not null,
  total_bid_volume double precision not null,
  total_ask_volume double precision not null,
  ofi double precision not null,
  ofi_raw double precision not null,
  obi double precision not null,
  trend_strength double precision not null,
  cvd double precision not null,
  taker_buy_ratio double precision not null,
  price_precision integer not null,
  quantity_precision integer not null,
  bid_depth jsonb not null,
  ask_depth jsonb not null,
  update_count bigint not null,
  created_at timestamptz not null default now()
);

comment on table market.order_book_snapshot is '订单簿快照表：保存 bridge 层按节流策略落库的盘口快照';
comment on column market.order_book_snapshot.symbol is '交易对，如 BTCUSDT';
comment on column market.order_book_snapshot.event_time is '应用生成该快照的 UTC 时间';
comment on column market.order_book_snapshot.bid is '当前最佳买价';
comment on column market.order_book_snapshot.ask is '当前最佳卖价';
comment on column market.order_book_snapshot.mid is '买一卖一中间价';
comment on column market.order_book_snapshot.spread_bps is '盘口价差，单位 bps';
comment on column market.order_book_snapshot.total_bid_volume is '统计窗口内买盘总量';
comment on column market.order_book_snapshot.total_ask_volume is '统计窗口内卖盘总量';
comment on column market.order_book_snapshot.ofi is '订单流失衡指标';
comment on column market.order_book_snapshot.ofi_raw is '原始深度差';
comment on column market.order_book_snapshot.obi is '订单簿失衡指标';
comment on column market.order_book_snapshot.trend_strength is '订单簿趋势强度';
comment on column market.order_book_snapshot.cvd is '累计主动成交量差';
comment on column market.order_book_snapshot.taker_buy_ratio is '主动买成交占比';
comment on column market.order_book_snapshot.price_precision is '价格精度';
comment on column market.order_book_snapshot.quantity_precision is '数量精度';
comment on column market.order_book_snapshot.bid_depth is '买盘前 25 档深度 JSONB';
comment on column market.order_book_snapshot.ask_depth is '卖盘前 25 档深度 JSONB';
comment on column market.order_book_snapshot.update_count is '内存订单簿已处理的深度更新计数';

create index if not exists idx_market_order_book_snapshot_symbol_time
  on market.order_book_snapshot(symbol, event_time desc);

create index if not exists idx_market_order_book_snapshot_event_time
  on market.order_book_snapshot(event_time desc);

create index if not exists idx_market_order_book_snapshot_symbol_update
  on market.order_book_snapshot(symbol, update_count desc);
