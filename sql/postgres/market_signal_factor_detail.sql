-- 信号因子详情表初始化脚本
-- 作用：
-- 1. 记录信号触发时每个因子的原始值、z-score、贡献分
-- 2. FK 关联 signal_performance_sample，供事后回测分析
-- 3. 每次信号触发产生多行（每个因子一行）

create schema if not exists market;

create table if not exists market.signal_factor_detail (
  id                 bigserial primary key,
  sample_id          uuid           not null,   -- FK → signal_performance_sample.sample_id
  symbol             varchar(32)    not null,
  signal_type        varchar(16)    not null,   -- 'pump' | 'dump'
  triggered_at       timestamptz    not null,
  factor_name        varchar(64)    not null,   -- 如 'ofi_zscore', 'obi_zscore', 'vol_zscore', 'whale', 'price_accel'
  raw_value          double precision not null, -- 因子原始值（如 ofi 值 12345.6）
  z_score            double precision,          -- z-score（预热期为 null）
  contribution_score double precision not null, -- 该因子对 total_score 的贡献分
  created_at         timestamptz    not null default now(),

  foreign key (sample_id) references market.signal_performance_sample(sample_id) on delete cascade
);

comment on table market.signal_factor_detail                   is '信号因子详情：记录每次信号触发时各因子的值和权重贡献';
comment on column market.signal_factor_detail.sample_id        is '关联的信号样本 ID';
comment on column market.signal_factor_detail.symbol           is '交易对，如 BTCUSDT';
comment on column market.signal_factor_detail.signal_type      is '信号类型：pump / dump';
comment on column market.signal_factor_detail.triggered_at     is '信号触发时间，与 sample_id 对应';
comment on column market.signal_factor_detail.factor_name      is '因子名称，如 ofi_zscore / obi_zscore / vol_zscore / whale / price_accel / level_break';
comment on column market.signal_factor_detail.raw_value        is '因子的原始数值，未经归一化';
comment on column market.signal_factor_detail.z_score          is '因子 z-score（预热未完成时为 null）';
comment on column market.signal_factor_detail.contribution_score is '该因子对最终评分的贡献分值';

create index if not exists idx_market_signal_factor_sample
  on market.signal_factor_detail(sample_id, factor_name);

create index if not exists idx_market_signal_factor_symbol_time
  on market.signal_factor_detail(symbol, triggered_at desc);

create index if not exists idx_market_signal_factor_name_time
  on market.signal_factor_detail(factor_name, triggered_at desc);
