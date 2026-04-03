-- 异动检测事件历史表初始化脚本
-- 作用：
-- 1. 持久化 OrderBookAnomalyDetector 检测到的异动事件（替代内存 VecDeque）
-- 2. 支持按类型、严重度、时间检索
-- 3. 为后期策略回测提供异动事件上下文

create schema if not exists market;

create table if not exists market.anomaly_event (
  id           bigserial primary key,
  symbol       varchar(32)      not null,
  detected_at  timestamptz      not null,
  anomaly_type varchar(64)      not null,   -- 如 'RapidCancellation', 'OrderSurge', 'WallAppear', 'Spoofing' 等
  severity     varchar(16)      not null,   -- 'low' | 'medium' | 'high' | 'critical'
  confidence   double precision not null,   -- 置信度 0.0~1.0
  price_level  double precision,            -- 关联价格档位（可为 null）
  side         varchar(4),                  -- 'bid' | 'ask' | null（非方向性事件）
  size_qty     double precision,            -- 触发事件的挂单量（可为 null）
  percentage   double precision,            -- 相对指标，如撤单占比 0.0~1.0
  duration_ms  integer,                     -- 事件持续时间（毫秒，可为 null）
  description  text             not null,   -- 人类可读描述
  created_at   timestamptz      not null default now()
);

comment on table market.anomaly_event              is '异动检测事件历史：持久化 anomaly detector 检测到的各类市场异常';
comment on column market.anomaly_event.symbol      is '交易对，如 BTCUSDT';
comment on column market.anomaly_event.detected_at is '检测到异动的 UTC 时间';
comment on column market.anomaly_event.anomaly_type is '异动类型，与 detector 中 AnomalyType 枚举对应';
comment on column market.anomaly_event.severity    is '严重等级：low/medium/high/critical';
comment on column market.anomaly_event.confidence  is '检测置信度 [0, 1]';
comment on column market.anomaly_event.price_level is '相关价格档位（可为 null）';
comment on column market.anomaly_event.side        is '方向性事件：bid / ask（无方向时为 null）';
comment on column market.anomaly_event.size_qty    is '触发事件的挂单数量（可为 null）';
comment on column market.anomaly_event.percentage  is '相对占比指标（0~1），如撤单量占总挂单量的比例';
comment on column market.anomaly_event.duration_ms is '事件持续时长（毫秒），如大单驻留时间';
comment on column market.anomaly_event.description is '事件的文字描述，供人工快速审查';

create index if not exists idx_market_anomaly_event_symbol_time
  on market.anomaly_event(symbol, detected_at desc);

create index if not exists idx_market_anomaly_event_type_time
  on market.anomaly_event(anomaly_type, detected_at desc);

create index if not exists idx_market_anomaly_event_symbol_type
  on market.anomaly_event(symbol, anomaly_type, detected_at desc);

create index if not exists idx_market_anomaly_event_severity
  on market.anomaly_event(severity, detected_at desc);
