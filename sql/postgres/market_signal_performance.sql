create schema if not exists market;

create table if not exists market.signal_performance_sample (
  sample_id uuid primary key,
  symbol varchar(32) not null,
  signal_type varchar(16) not null,
  triggered_at timestamptz not null,
  trigger_price double precision not null,
  trigger_score integer not null,
  watch_level varchar(32) not null,
  signal_reason text not null,
  update_count bigint not null,
  resolved_5m boolean not null default false,
  resolved_15m boolean not null default false,
  resolved_decay boolean not null default false,
  outcome_5m_return double precision,
  outcome_5m_win boolean,
  outcome_5m_at timestamptz,
  outcome_15m_return double precision,
  outcome_15m_win boolean,
  outcome_15m_at timestamptz,
  decay_minutes double precision,
  decay_at timestamptz,
  created_at timestamptz not null default now()
);

comment on table market.signal_performance_sample is '信号质量样本库：记录后端识别出的真实信号及其 5m/15m 表现和衰减时间';
comment on column market.signal_performance_sample.signal_type is '信号类型：pump/dump';
comment on column market.signal_performance_sample.trigger_price is '信号触发时的中间价';
comment on column market.signal_performance_sample.trigger_score is '信号触发时的评分';
comment on column market.signal_performance_sample.decay_minutes is '信号从触发到失效的分钟数';

create index if not exists idx_market_signal_perf_symbol_time
  on market.signal_performance_sample(symbol, triggered_at desc);

create index if not exists idx_market_signal_perf_symbol_pending
  on market.signal_performance_sample(symbol, resolved_5m, resolved_15m, resolved_decay);
