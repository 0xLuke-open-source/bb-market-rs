use crate::signal_intelligence::domain::algorithms::{
    OverallSentiment, RiskLevel, TradingRecommendation,
};
use crate::signal_intelligence::domain::strategy_engine::StrategyProfile;

pub fn watch_level_label(
    strategy: &StrategyProfile,
    pump_score: u8,
    dump_score: u8,
    anomaly_max_severity: u8,
    whale_entry: bool,
) -> &'static str {
    let max_prob = strategy
        .pump_probability
        .max(strategy.dump_probability)
        .max(strategy.false_breakout_probability)
        .max(strategy.reversal_risk);
    if anomaly_max_severity >= 88
        || max_prob >= 86
        || strategy.confidence >= 82
        || strategy.phase == "点火期"
    {
        "强提醒"
    } else if whale_entry
        || anomaly_max_severity >= 75
        || strategy.confidence >= 68
        || max_prob >= 72
        || strategy.phase == "延续期"
    {
        "重点关注"
    } else if pump_score >= 60 || dump_score >= 60 || max_prob >= 58 {
        "普通关注"
    } else {
        "观察"
    }
}

pub fn status_summary(
    strategy: &StrategyProfile,
    pump_score: u8,
    dump_score: u8,
    taker_buy_ratio: f64,
    cvd: f64,
    obi: f64,
    anomaly_max_severity: u8,
    whale_entry: bool,
    whale_exit: bool,
) -> String {
    if anomaly_max_severity >= 85 {
        return "波动明显异常，先看风险再决定是否继续关注。".into();
    }
    match strategy.phase.as_str() {
        "吸筹期" if strategy.direction == "偏多" => {
            return format!(
                "买盘在吸收卖压，拉盘预期 {}%，等待点火确认。",
                strategy.pump_probability
            );
        }
        "派发期" if strategy.direction == "偏空" => {
            return format!(
                "卖压在逐步占优，砸盘预期 {}%，等待进一步下破。",
                strategy.dump_probability
            );
        }
        "点火期" if strategy.direction == "偏多" => {
            return format!(
                "盘口和主动成交开始同步转强，预计 {}-{} 秒内见方向。",
                10,
                strategy.expected_window_secs
            );
        }
        "点火期" if strategy.direction == "偏空" => {
            return format!(
                "盘口和主动卖出开始同步放大，预计 {}-{} 秒内见方向。",
                10,
                strategy.expected_window_secs
            );
        }
        "延续期" => {
            return format!(
                "多窗口方向一致，延续概率 {}%，但要盯住失效条件。",
                strategy.continuation_probability
            );
        }
        "衰减期" => {
            return format!(
                "原方向还在，但反转风险 {}%，更适合控仓而不是追单。",
                strategy.reversal_risk
            );
        }
        "假突破风险" => {
            return format!(
                "动作已出现，但承接不够，假突破风险 {}%。",
                strategy.false_breakout_probability
            );
        }
        _ => {}
    }
    if whale_exit && dump_score >= 70 {
        return "大户疑似离场，卖压偏强，短线要防回落。".into();
    }
    if whale_entry && pump_score >= 70 {
        return "大户资金有进场迹象，买盘偏强，适合重点盯住。".into();
    }
    if pump_score >= 75 && taker_buy_ratio >= 60.0 && cvd > 0.0 {
        return "主动买盘持续增强，短线偏强，可以重点观察突破。".into();
    }
    if dump_score >= 75 && taker_buy_ratio <= 40.0 && cvd < 0.0 {
        return "主动卖盘持续增强，短线偏弱，注意继续下压。".into();
    }
    if obi >= 15.0 && taker_buy_ratio >= 55.0 {
        return "买盘略占优势，当前偏强，但还需要继续确认。".into();
    }
    if obi <= -15.0 && taker_buy_ratio <= 45.0 {
        return "卖盘略占优势，当前偏弱，追涨要谨慎。".into();
    }
    "多空暂时比较均衡，先观察是否出现新的主导资金方向。".into()
}

pub fn signal_reason(
    strategy: &StrategyProfile,
    pump_score: u8,
    dump_score: u8,
    taker_buy_ratio: f64,
    cvd: f64,
    obi: f64,
    anomaly_count_1m: u32,
    whale_entry: bool,
    whale_exit: bool,
) -> String {
    let mut parts: Vec<String> = Vec::new();
    if !strategy.reasons.is_empty() {
        parts.extend(strategy.reasons.iter().take(3).cloned());
    }
    if pump_score >= 60 {
        parts.push(format!("上涨动能 {} 分", pump_score));
    }
    if dump_score >= 60 {
        parts.push(format!("下跌压力 {} 分", dump_score));
    }
    if obi.abs() >= 10.0 {
        parts.push(format!("买卖盘失衡 {:+.1}%", obi));
    }
    if taker_buy_ratio >= 60.0 {
        parts.push(format!("主动买入占比 {:.0}%", taker_buy_ratio));
    } else if taker_buy_ratio <= 40.0 {
        parts.push(format!("主动卖出占比 {:.0}%", 100.0 - taker_buy_ratio));
    }
    if cvd.abs() >= 10000.0 {
        parts.push(format!("主动买卖量差 {:+.0}", cvd));
    }
    if anomaly_count_1m >= 30 {
        parts.push(format!("1分钟异常波动 {} 次", anomaly_count_1m));
    }
    if whale_entry {
        parts.push("检测到大户资金进场".into());
    }
    if whale_exit {
        parts.push("检测到大户资金离场".into());
    }
    parts.dedup();
    if parts.is_empty() {
        "当前没有特别突出的异常信号。".into()
    } else {
        parts.join("，")
    }
}

pub fn sentiment_str(s: &OverallSentiment) -> String {
    match s {
        OverallSentiment::StrongBullish => "StrongBullish",
        OverallSentiment::Bullish => "Bullish",
        OverallSentiment::Neutral => "Neutral",
        OverallSentiment::Bearish => "Bearish",
        OverallSentiment::StrongBearish => "StrongBearish",
    }
    .into()
}

pub fn risk_str(r: &RiskLevel) -> String {
    match r {
        RiskLevel::VeryLow => "VeryLow",
        RiskLevel::Low => "Low",
        RiskLevel::Medium => "Medium",
        RiskLevel::High => "High",
        RiskLevel::VeryHigh => "VeryHigh",
    }
    .into()
}

pub fn recommendation_str(r: &TradingRecommendation) -> String {
    match r {
        TradingRecommendation::StrongBuy => "StrongBuy",
        TradingRecommendation::Buy => "Buy",
        TradingRecommendation::Neutral => "Neutral",
        TradingRecommendation::Sell => "Sell",
        TradingRecommendation::StrongSell => "StrongSell",
        TradingRecommendation::Wait => "Wait",
    }
    .into()
}
// labels 模块把内部枚举/分值转换成人类可读的短文本。
//
// 这层是纯展示语义层，不参与计算，只负责把算法输出翻译成前端标签。
