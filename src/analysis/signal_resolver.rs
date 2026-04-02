//! 信号事后回填服务
//!
//! 每 30s 查询 resolved_5m=false 且触发超过 5m 的记录，用当前价格填充 outcome_5m_*。
//! 每 60s 同理填充 outcome_15m_*。
//! 当前价格从 SharedDashboardState 中读取（不需要外部 API 调用）。
//!
//! 此服务是对 panel.rs 中 resolve_signal_samples 的补充：
//! - panel.rs 中的解析依赖 bridge 传入的 snapshot.mid，在 bridge 运行时自动触发
//! - SignalResolver 是独立定时任务，覆盖 bridge 不活跃期间的未解析样本

use std::sync::Arc;

use anyhow::Result;
use chrono::Utc;
use tokio::time::{interval, Duration};

use crate::postgres::PgPool;
use crate::web::state::SharedDashboardState;

pub struct SignalResolver {
    pool: Arc<PgPool>,
}

impl SignalResolver {
    pub fn new(pool: Arc<PgPool>) -> Self {
        Self { pool }
    }

    /// 填充 5m 和 15m 未解析的信号样本
    /// 返回本轮解析条数
    pub async fn resolve_pending(&self, state: &SharedDashboardState) -> Result<usize> {
        let client = self.pool.acquire().await?;

        // 查询所有未完全解析的样本
        let rows = client
            .client()
            .query(
                "select
                    sample_id, symbol, signal_type, triggered_at, trigger_price,
                    resolved_5m, resolved_15m, resolved_decay
                 from market.signal_performance_sample
                where resolved_5m = false or resolved_15m = false or resolved_decay = false
                order by triggered_at asc
                limit 200",
                &[],
            )
            .await?;

        let now = Utc::now();
        let mut count = 0usize;

        for row in rows {
            let sample_id: uuid::Uuid = row.get("sample_id");
            let symbol: String = row.get("symbol");
            let signal_type: String = row.get("signal_type");
            let triggered_at: chrono::DateTime<Utc> = row.get("triggered_at");
            let trigger_price: f64 = row.get("trigger_price");
            let resolved_5m: bool = row.get("resolved_5m");
            let resolved_15m: bool = row.get("resolved_15m");
            let resolved_decay: bool = row.get("resolved_decay");

            // 从 DashboardState 取当前价格
            let current_price = {
                let ds = state.read().await;
                ds.symbols
                    .get(&symbol)
                    .map(|s| s.mid)
                    .unwrap_or(0.0)
            };
            if current_price <= 0.0 {
                continue;
            }

            let elapsed_secs = (now - triggered_at).num_seconds();
            if elapsed_secs < 0 {
                continue;
            }

            // 5m 回填
            if !resolved_5m && elapsed_secs >= 5 * 60 {
                let ret = calc_return(trigger_price, current_price, &signal_type);
                let win = is_win(&signal_type, ret);
                client
                    .client()
                    .execute(
                        "update market.signal_performance_sample
                            set resolved_5m = true,
                                outcome_5m_return = $2,
                                outcome_5m_win = $3,
                                outcome_5m_at = $4
                          where sample_id = $1",
                        &[&sample_id, &ret, &win, &now],
                    )
                    .await?;
                count += 1;
            }

            // 15m 回填
            if !resolved_15m && elapsed_secs >= 15 * 60 {
                let ret = calc_return(trigger_price, current_price, &signal_type);
                let win = is_win(&signal_type, ret);
                client
                    .client()
                    .execute(
                        "update market.signal_performance_sample
                            set resolved_15m = true,
                                outcome_15m_return = $2,
                                outcome_15m_win = $3,
                                outcome_15m_at = $4
                          where sample_id = $1",
                        &[&sample_id, &ret, &win, &now],
                    )
                    .await?;
                count += 1;
            }

            // decay 回填：超过 60min 且仍未 decay，强制标记
            if !resolved_decay && elapsed_secs >= 60 * 60 {
                let decay_minutes = elapsed_secs as f64 / 60.0;
                client
                    .client()
                    .execute(
                        "update market.signal_performance_sample
                            set resolved_decay = true,
                                decay_minutes = $2,
                                decay_at = $3
                          where sample_id = $1",
                        &[&sample_id, &decay_minutes, &now],
                    )
                    .await?;
                count += 1;
            }
        }

        Ok(count)
    }
}

/// 启动信号回填后台任务
pub fn spawn_signal_resolver(resolver: SignalResolver, state: SharedDashboardState) {
    tokio::spawn(async move {
        let mut tick_30s = interval(Duration::from_secs(31));
        loop {
            tick_30s.tick().await;
            match resolver.resolve_pending(&state).await {
                Ok(n) if n > 0 => {
                    eprintln!("[signal_resolver] resolved {} signal samples", n);
                }
                Err(e) => {
                    eprintln!("[signal_resolver] error: {}", e);
                }
                _ => {}
            }
        }
    });
}

// ── 内部辅助函数 ─────────────────────────────────────────────────

fn calc_return(trigger_price: f64, current_price: f64, signal_type: &str) -> f64 {
    if trigger_price <= 0.0 {
        return 0.0;
    }
    let raw = (current_price - trigger_price) / trigger_price * 100.0;
    if signal_type == "dump" {
        -raw
    } else {
        raw
    }
}

fn is_win(signal_type: &str, ret: f64) -> bool {
    if signal_type == "dump" {
        ret > 0.0
    } else {
        ret > 0.0
    }
}
