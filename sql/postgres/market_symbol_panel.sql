-- 分析面板快照初始化脚本
-- 作用：
-- 1. 创建 market.symbol_panel_snapshot，保存前端分析面板核心指标
-- 2. 便于后续做盘中分析、回放和席位级复盘
-- 3. 建立按币种、时间、update_count 检索的常用索引

create schema if not exists market;

create table if not exists market.symbol_panel_snapshot (
  snapshot_id uuid primary key,
  symbol varchar(32) not null,
  event_time timestamptz not null,
  bid double precision not null,
  ask double precision not null,
  mid double precision not null,
  spread_bps double precision not null,
  change_24h_pct double precision not null,
  high_24h double precision not null,
  low_24h double precision not null,
  volume_24h double precision not null,
  quote_vol_24h double precision not null,
  ofi double precision not null,
  ofi_raw double precision not null,
  obi double precision not null,
  trend_strength double precision not null,
  cvd double precision not null,
  taker_buy_ratio double precision not null,
  pump_score integer not null,
  dump_score integer not null,
  pump_signal boolean not null,
  dump_signal boolean not null,
  whale_entry boolean not null,
  whale_exit boolean not null,
  bid_eating boolean not null,
  total_bid_volume double precision not null,
  total_ask_volume double precision not null,
  max_bid_ratio double precision not null,
  max_ask_ratio double precision not null,
  anomaly_count_1m integer not null,
  anomaly_max_severity integer not null,
  status_summary text not null,
  watch_level varchar(32) not null,
  signal_reason text not null,
  sentiment varchar(32) not null,
  risk_level varchar(32) not null,
  recommendation varchar(64) not null,
  whale_type varchar(64) not null,
  pump_probability integer not null,
  price_precision integer not null default 0,
  quantity_precision integer not null default 0,
  snapshot_json jsonb not null default '{}'::jsonb,
  signal_history_json jsonb not null default '[]'::jsonb,
  factor_metrics_json jsonb not null default '[]'::jsonb,
  enterprise_metrics_json jsonb not null default '[]'::jsonb,
  update_count bigint not null,
  created_at timestamptz not null default now()
);

alter table market.symbol_panel_snapshot
  add column if not exists price_precision integer not null default 0;

alter table market.symbol_panel_snapshot
  add column if not exists quantity_precision integer not null default 0;

alter table market.symbol_panel_snapshot
  add column if not exists snapshot_json jsonb not null default '{}'::jsonb;

alter table market.symbol_panel_snapshot
  add column if not exists signal_history_json jsonb not null default '[]'::jsonb;

alter table market.symbol_panel_snapshot
  add column if not exists factor_metrics_json jsonb not null default '[]'::jsonb;

alter table market.symbol_panel_snapshot
  add column if not exists enterprise_metrics_json jsonb not null default '[]'::jsonb;

comment on table market.symbol_panel_snapshot is '分析面板快照表：保存前端分析面板核心指标和解释文案';
comment on column market.symbol_panel_snapshot.symbol is '交易对，如 BTCUSDT';
comment on column market.symbol_panel_snapshot.event_time is '应用生成该快照的 UTC 时间';
comment on column market.symbol_panel_snapshot.status_summary is '分析面板摘要';
comment on column market.symbol_panel_snapshot.watch_level is '关注级别';
comment on column market.symbol_panel_snapshot.signal_reason is '信号解释';
comment on column market.symbol_panel_snapshot.snapshot_json is '完整 SymbolJson 快照，包含盘口/大单/最近成交/K线等分析面板原始数据';
comment on column market.symbol_panel_snapshot.signal_history_json is '分析面板信号详情区最近历史提醒 JSONB';
comment on column market.symbol_panel_snapshot.factor_metrics_json is '分析面板信号因子区 JSONB';
comment on column market.symbol_panel_snapshot.enterprise_metrics_json is '分析面板企业级指标区 JSONB';
comment on column market.symbol_panel_snapshot.update_count is '该 symbol 已处理的深度更新计数';

create index if not exists idx_market_symbol_panel_snapshot_symbol_time
  on market.symbol_panel_snapshot(symbol, event_time desc);

create index if not exists idx_market_symbol_panel_snapshot_event_time
  on market.symbol_panel_snapshot(event_time desc);

create index if not exists idx_market_symbol_panel_snapshot_symbol_update
  on market.symbol_panel_snapshot(symbol, update_count desc);
