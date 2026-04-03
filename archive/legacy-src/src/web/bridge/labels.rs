use crate::signal_intelligence::domain::algorithms::{
    OverallSentiment, RiskLevel, TradingRecommendation,
};

pub fn watch_level_label(
    pump_score: u8,
    dump_score: u8,
    anomaly_max_severity: u8,
    whale_entry: bool,
) -> &'static str {
    if anomaly_max_severity >= 85 || pump_score >= 85 || dump_score >= 85 {
        "强提醒"
    } else if whale_entry || anomaly_max_severity >= 75 || pump_score >= 75 || dump_score >= 75 {
        "重点关注"
    } else if pump_score >= 60 || dump_score >= 60 {
        "普通关注"
    } else {
        "观察"
    }
}

pub fn status_summary(
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
    if whale_entry && pump_score >= 70 {
        return "大户资金有进场迹象，买盘偏强，适合重点盯住。".into();
    }
    if whale_exit && dump_score >= 70 {
        return "大户疑似离场，卖压偏强，短线要防回落。".into();
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
