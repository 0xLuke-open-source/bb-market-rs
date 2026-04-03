-- 自适应阈值快照表初始化脚本
-- 作用：
-- 1. 每 5 分钟记录每个 symbol 的 4h 滚动窗口统计基准
-- 2. 用于后期离线分析信号触发时的市场统计背景
-- 3. 表内数据可按需保留（推荐 7 天，节省空间）

create schema if not exists market;

create table if not exists market.adaptive_threshold (
  id              bigserial primary key,
  symbol          varchar(32)      not null,
  window_end_at   timestamptz      not null,
  sample_count    integer          not null,   -- 有效样本数（达到 720 才热身完成）
  is_warm         boolean          not null,   -- sample_count >= 720（约 1h）

  ofi_mean        double precision not null,
  ofi_std         double precision not null,
  obi_mean        double precision not null,
  obi_std         double precision not null,
  vol_mean        double precision not null,
  vol_std         double precision not null,
  bid_vol_mean    double precision not null,
  bid_vol_std     double precision not null,
  spread_mean     double precision not null,
  spread_std      double precision not null,

  created_at      timestamptz      not null default now()
);

comment on table market.adaptive_threshold                  is '自适应阈值快照：每 5 分钟记录每个币种的 4h 滚动统计基准';
comment on column market.adaptive_threshold.symbol          is '交易对，如 BTCUSDT';
comment on column market.adaptive_threshold.window_end_at  is '该快照对应的统计窗口结束时间';
comment on column market.adaptive_threshold.sample_count   is '当前滚动窗口内的有效样本数量';
comment on column market.adaptive_threshold.is_warm        is '是否已完成预热（sample_count >= 720，约 1 小时）';
comment on column market.adaptive_threshold.ofi_mean       is '订单流失衡均值';
comment on column market.adaptive_threshold.ofi_std        is '订单流失衡标准差';
comment on column market.adaptive_threshold.obi_mean       is '订单簿失衡均值';
comment on column market.adaptive_threshold.obi_std        is '订单簿失衡标准差';
comment on column market.adaptive_threshold.vol_mean       is '成交量均值';
comment on column market.adaptive_threshold.vol_std        is '成交量标准差';
comment on column market.adaptive_threshold.bid_vol_mean   is '买盘量均值';
comment on column market.adaptive_threshold.bid_vol_std    is '买盘量标准差';
comment on column market.adaptive_threshold.spread_mean    is '价差均值（bps）';
comment on column market.adaptive_threshold.spread_std     is '价差标准差（bps）';

create index if not exists idx_market_adaptive_threshold_symbol_time
  on market.adaptive_threshold(symbol, window_end_at desc);

create index if not exists idx_market_adaptive_threshold_symbol_warm
  on market.adaptive_threshold(symbol, is_warm, window_end_at desc);
