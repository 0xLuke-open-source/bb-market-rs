mod client;
mod codec;
mod store;
mod analysis;

use std::io;
use crate::codec::binance_msg::Snapshot;
use crate::store::l2_book::{OrderBook, OrderBookFeatures};
use colored::Colorize;
use reqwest::Client;
use rust_decimal::Decimal;
use std::io::Write;
use std::time::Duration;
use const_format::concatcp;
use rust_decimal::prelude::ToPrimitive;
use rust_decimal_macros::dec;
use tokio::sync::mpsc;
use tokio::time::Instant;
use crate::analysis::MarketAnalysis;

const COIN: &str = "DOGS";
const SYMBOL: &str = concatcp!(COIN, "USDT");  // 这个支持 const 变量

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    rustls::crypto::ring::default_provider()
        .install_default()
        .expect("Failed to install rustls crypto provider");
    tracing_subscriber::fmt::init();

    let (tx, mut rx) = mpsc::channel(2000);
    let mut book = OrderBook::new(SYMBOL);

    // 记录连接开始时间
    let _connection_start = Instant::now(); // 添加下划线前缀消除警告
    let max_connection_duration = Duration::from_secs(23 * 60 * 60); // 23小时，提前重连

    // 1. 启动 WebSocket 采集流
    let symbol_task = SYMBOL.to_string();
    let tx_clone = tx.clone();

    // 连接管理任务
    tokio::spawn(async move {
        loop {
            let start_time = Instant::now();

            match client::websocket::run_client(&symbol_task, tx_clone.clone()).await {
                Ok(()) => {
                    println!("WebSocket client exited normally");
                }
                Err(e) => {
                    eprintln!("WebSocket Error: {}", e);
                }
            }

            // 检查是否接近24小时限制
            if start_time.elapsed() >= max_connection_duration {
                println!("Connection duration approaching 24h limit, forcing reconnect");
            }

            // 重连前等待，避免频繁连接
            tokio::time::sleep(Duration::from_secs(5)).await;
        }
    });

    println!("Step 1: WebSocket client started");

    // 2. 给 WebSocket 一些时间建立连接
    tokio::time::sleep(Duration::from_secs(2)).await;

    // 3. 获取初始快照 (REST API)
    println!("Step 2: Fetching snapshot from REST API...");

    let client = Client::builder().timeout(Duration::from_secs(15)).build()?;

    let snapshot = fetch_snapshot_with_retry(&client, SYMBOL, 5).await?;

    // 4. 初始化 OrderBook
    book.init_from_snapshot(snapshot);
    println!("Step 3: Snapshot initialized. ID: {}", book.last_update_id);

    // 显示初始订单簿深度
    let (top_bids, top_asks) = book.top_n(5);
    println!("Top 5 Bids: {:?}", top_bids);
    println!("Top 5 Asks: {:?}", top_asks);

    // 5. 处理更新
    let mut last_print = Instant::now();
    let print_interval = Duration::from_millis(100);

    // 新增：定时报告
    let mut last_report = Instant::now();
    let report_interval = Duration::from_secs(30); // 30秒报告一次

    // 新增：可选，保存报告到文件
    let save_to_file = true; // 设置为 true 则保存到文件
    let report_file = "market_analysis.log";


    let mut update_count = 0;

    while let Some(msg) = rx.recv().await {
        if let Err(e) = book.apply_incremental_update(msg) {
            eprintln!("\nUpdate Error: {}", e);
            // 这里可以触发重新同步
            break;
        }

        update_count += 1;

        // 限制打印频率，避免刷屏
        if last_print.elapsed() >= print_interval {
            if let Some((bid, ask)) = book.best_bid_ask() {
                let spread = ask - bid;
                let spread_bps = if !bid.is_zero() {
                    (spread / bid * Decimal::from(10000)).round_dp(2)
                } else {
                    Decimal::from(0)
                };

                let features = book.compute_features(10);

                // 选择你喜欢的显示模式
                // display_orderbook(&book, &features, update_count); // 方案1：完整表格
                // display_compact(&book, &features, update_count);    // 方案2：紧凑表格
                // display_single_line(&book, &features, update_count);    // 方案3：单行实时更新
                io::stdout().flush()?;
            }
            last_print = Instant::now();
        }

        // 新增：每30秒生成一次分析报告
        if last_report.elapsed() >= report_interval {
            if let Some((bid, ask)) = book.best_bid_ask() {
                let features = book.compute_features(10);
                let analysis = MarketAnalysis::new(&book, &features);
                analysis.display();

                // 可选：保存到文件
                if save_to_file {
                    if let Err(e) = analysis.save_to_file(report_file) {
                        eprintln!("Failed to save report: {}", e);
                    }
                }
            }
            last_report = Instant::now();
        }
    }

    Ok(())
}

async fn fetch_snapshot_with_retry(
    client: &Client,
    symbol: &str,
    max_retries: u32,
) -> anyhow::Result<Snapshot> {
    let endpoints = [
        "https://api.binance.com",
        "https://api1.binance.com",
        "https://api2.binance.com",
        "https://api3.binance.com",
    ];

    for retry in 0..max_retries {
        for &base_url in &endpoints {
            let url = format!("{}/api/v3/depth?symbol={}&limit=1000", base_url, symbol);

            match client.get(&url).send().await {
                Ok(resp) => {
                    if resp.status().is_success() {
                        match resp.json::<Snapshot>().await {
                            Ok(snapshot) => {
                                println!("✅ Snapshot fetched from {}", base_url);
                                return Ok(snapshot);
                            }
                            Err(e) => {
                                eprintln!("Parse error from {}: {}", base_url, e);
                            }
                        }
                    } else {
                        eprintln!("HTTP error from {}: {}", base_url, resp.status());
                    }
                }
                Err(e) => {
                    eprintln!("Connection error to {}: {}", base_url, e);
                }
            }
        }

        if retry < max_retries - 1 {
            let wait_time = Duration::from_secs(2u64.pow(retry + 1));
            println!("Retrying in {}s...", wait_time.as_secs());
            tokio::time::sleep(wait_time).await;
        }
    }

    anyhow::bail!("Failed to fetch snapshot after {} retries", max_retries)
}

// 在 main.rs 中添加这个函数
fn display_orderbook(book: &OrderBook, features: &OrderBookFeatures, update_count: u64) {
    // 清除屏幕并定位到左上角
    print!("\x1B[2J\x1B[1;1H");

    // 获取最佳买卖价
    let (bid, ask) = book
        .best_bid_ask()
        .unwrap_or((Decimal::ZERO, Decimal::ZERO));

    // 标题 - 带警告色
    let title = if features.pump_signal {
        "🚀🚀🚀 拉盘预警! 🚀🚀🚀".to_string()
    } else if features.dump_signal {
        "📉📉📉 砸盘预警! 📉📉📉".to_string()
    } else if features.whale_entry {
        "🐋🐋🐋 鲸鱼进场! 🐋🐋🐋".to_string()
    } else if features.whale_exit {
        "🐋🐋🐋 鲸鱼离场! 🐋🐋🐋".to_string()
    } else if features.liquidity_warning {
        "⚠️⚠️⚠️ 流动性危机! ⚠️⚠️⚠️".to_string()
    } else {
        format!("Binance {} Order Book Monitor (专业版)", book.symbol)
    };

    println!("╔══════════════════════════════════════════════════════════════════════════════════════════════════════════╗");
    println!("║ {:^104} ║", title);
    println!("╠══════════════════════════════════════════════════════════════════════════════════════════════════════════╣");

    // ==================== 第1行：基础价格数据 ====================
    println!("║ 💰 价格数据                                 | 📊 成交量数据                                               ║");
    println!("╠─────────────────────────────────────────────┼──────────────────────────────────────────────────────────║");
    println!(
        "║  Bid: {:>20} {}    | 总买单量: {:>20.6} {:<4}                   ║",
        format!("{}", bid),
        if features.whale_bid { "🐋" } else { "  " },
        features.total_bid_volume,
        COIN
    );
    println!(
        "║  Ask: {:>20} {}    | 总卖单量: {:>20.6} {:<4}                   ║",
        format!("{}", ask),
        if features.whale_ask { "🐋" } else { "  " },
        features.total_ask_volume,
        COIN
    );
    println!(
        "║  Spread: {:>18} ({:>8} bps) | 前10档买单: {:>18.6} {:<4}                   ║",
        features.spread.to_string(),
        features.spread_bps.to_string(),
        features.bid_volume_depth,
        COIN
    );
    println!(
        "║  微价格: {:>19}     | 前10档卖单: {:>18.6} {:<4}                   ║",
        features.microprice.to_string(),
        features.ask_volume_depth,
        COIN
    );

    // ==================== 第2行：失衡指标 ====================
    println!("╠─────────────────────────────────────────────┼──────────────────────────────────────────────────────────║");
    println!("║ ⚖️ 失衡指标                                 | 📈 趋势指标                                                 ║");
    println!("╠─────────────────────────────────────────────┼──────────────────────────────────────────────────────────║");

    println!(
        "║  OBI: {:>+20} {} | 趋势强度: {:>+20}                              ║",
        features.obi.to_string(),
        if features.obi > dec!(30) { "🚀" } else if features.obi < dec!(-30) { "📉" } else { "➡️" },
        features.trend_strength.to_string()
    );
    println!(
        "║  OFI: {:>+20} {} | 价格变化: {:>+20}%                              ║",
        features.ofi.to_string(),
        if features.ofi > dec!(50000) { "📈" } else if features.ofi < dec!(-50000) { "📉" } else { "➡️" },
        features.price_change.to_string()
    );
    println!(
        "║  Bid/Ask Ratio: {:>17}    | 累计Delta: {:>+20}                              ║",
        features.bid_ask_ratio.to_string(),
        features.cum_delta.to_string()
    );
    println!(
        "║  10档失衡: {:>19}    | 总量失衡: {:>20}                              ║",
        features.imbalance_depth_10.to_string(),
        features.imbalance_total.to_string()
    );

    // ==================== 第3行：深度集中度 ====================
    println!("╠─────────────────────────────────────────────┼──────────────────────────────────────────────────────────║");
    println!("║ 🎯 深度集中度                               | 📉 价格压力                                                 ║");
    println!("╠─────────────────────────────────────────────┼──────────────────────────────────────────────────────────║");
    println!(
        "║  买单集中度(前3): {:>15}%    | 加权买价: {:>20}                              ║",
        features.bid_concentration.round_dp(2).to_string(),
        features.weighted_bid_price.round_dp(6).to_string()
    );
    println!(
        "║  卖单集中度(前3): {:>15}%    | 加权卖价: {:>20}                              ║",
        features.ask_concentration.round_dp(2).to_string(),
        features.weighted_ask_price.round_dp(6).to_string()
    );
    println!(
        "║  最大买单占比: {:>17}%    | 价格压力: {:>+20}                              ║",
        features.max_bid_ratio.round_dp(2).to_string(),
        features.price_pressure.round_dp(6).to_string()
    );
    println!(
        "║  最大卖单占比: {:>17}%    | 价格弹性(B/A): {:>10}/{:>10}                    ║",
        features.max_ask_ratio.round_dp(2).to_string(),
        features.bid_elasticity.round_dp(6).to_string(),
        features.ask_elasticity.round_dp(6).to_string()
    );

    // ==================== 第4行：变化率 ====================
    println!("╠─────────────────────────────────────────────┼──────────────────────────────────────────────────────────║");
    println!("║ 🔄 变化率                                   | 📊 买卖压力                                                 ║");
    println!("╠─────────────────────────────────────────────┼──────────────────────────────────────────────────────────║");

    println!(
        "║  买单变化: {:>+19}% {} | 买单压力比: {:>19}                              ║",
        features.bid_volume_change.to_string(),
        if features.bid_volume_change > dec!(20) { "🚀" } else { "" },
        features.bid_pressure_ratio.round_dp(4).to_string()
    );
    println!(
        "║  卖单变化: {:>+19}% {} | 卖单压力比: {:>19}                              ║",
        features.ask_volume_change.to_string(),
        if features.ask_volume_change > dec!(20) { "📉" } else { "" },
        features.ask_pressure_ratio.round_dp(4).to_string()
    );
    println!(
        "║  近端买单厚度: {:>18}    | 支撑强度: {:>20}                              ║",
        features.near_bid_thickness.round_dp(6).to_string(),
        features.support_strength.round_dp(6).to_string()
    );
    println!(
        "║  近端卖单厚度: {:>18}    | 阻力强度: {:>20}                              ║",
        features.near_ask_thickness.round_dp(6).to_string(),
        features.resistance_strength.round_dp(6).to_string()
    );

    // ==================== 第5行：斜率 ====================
    println!("╠─────────────────────────────────────────────┼──────────────────────────────────────────────────────────║");
    println!("║ 📐 斜率                                     | ⚡ 流动性缺口                                               ║");
    println!("╠─────────────────────────────────────────────┼──────────────────────────────────────────────────────────║");
    println!(
        "║  买单斜率: {:>+20} {} | 买单缺口: {:>20}处                              ║",
        features.slope_bid.round_dp(6).to_string(),
        if features.slope_bid > dec!(100000) { "📈" } else { "" },
        features.liquidity_gap_bid
    );
    println!(
        "║  卖单斜率: {:>+20} {} | 卖单缺口: {:>20}处                              ║",
        features.slope_ask.round_dp(6).to_string(),
        if features.slope_ask < dec!(-100000) { "📉" } else { "" },
        features.liquidity_gap_ask
    );

    // ==================== 第6行：信号预警 ====================
    println!("╠══════════════════════════════════════════════════════════════════════════════════════════════════════════╣");
    println!("║ 🚨 实时信号预警                                                                                          ║");
    println!("╠══════════════════════════════════════════════════════════════════════════════════════════════════════════╣");

    // 收集所有激活的信号
    let mut signals = Vec::new();
    if features.pump_signal { signals.push("🚀 拉盘信号"); }
    if features.dump_signal { signals.push("📉 砸盘信号"); }
    if features.whale_entry { signals.push("🐋 鲸鱼进场"); }
    if features.whale_exit { signals.push("🐋 鲸鱼离场"); }
    if features.bid_eating { signals.push("🍽️ 买单吃筹"); }
    if features.ask_eating { signals.push("💥 卖单砸盘"); }
    if features.fake_breakout { signals.push("🎭 假突破"); }
    if features.liquidity_warning { signals.push("⚠️ 流动性危机"); }

    if signals.is_empty() {
        println!("║  {:^104}  ║", "暂无异常信号");
    } else {
        // 每行显示4个信号
        for chunk in signals.chunks(4) {
            let mut line = String::from("║  ");
            for signal in chunk {
                line.push_str(&format!("{:<25}", signal));
            }
            line.push_str("  ║");
            println!("{}", line);
        }
    }

    println!("╠══════════════════════════════════════════════════════════════════════════════════════════════════════════╣");
    println!("║  📋 Top 5 订单簿深度                                                                                    ║");
    println!("╠══════════════════════════════════════════════════════════════════════════════════════════════════════════╣");

    // Top 5 订单簿
    println!("┌───────────────┬───────────────┬───────────────┬───────────────┐");
    println!("│     Bids       │     Amount    │     Asks       │     Amount    │");
    println!("├───────────────┼───────────────┼───────────────┼───────────────┤");

    let (top_bids, top_asks) = book.top_n(5);
    for i in 0..5 {
        let (bid_price, bid_qty) = if i < top_bids.len() {
            top_bids[i]
        } else {
            (Decimal::ZERO, Decimal::ZERO)
        };

        let (ask_price, ask_qty) = if i < top_asks.len() {
            top_asks[i]
        } else {
            (Decimal::ZERO, Decimal::ZERO)
        };

        // 标记异常大单
        let bid_marker = if bid_qty > features.total_bid_volume * dec!(0.2) { "🐋" } else { "" };
        let ask_marker = if ask_qty > features.total_ask_volume * dec!(0.2) { "🐋" } else { "" };

        println!(
            "│ {:>13.6}{:<2} │ {:>11.6} {:<4} │ {:>13.6}{:<2} │ {:>11.6} {:<4} │",
            bid_price, bid_marker, bid_qty, COIN, ask_price, ask_marker, ask_qty, COIN
        );
    }
    println!("└───────────────┴───────────────┴───────────────┴───────────────┘");

    // ==================== 买卖力量对比图 ====================
    let total_bid_vol = features.total_bid_volume;
    let total_ask_vol = features.total_ask_volume;
    let total_vol = total_bid_vol + total_ask_vol;

    let bid_ratio = if total_vol > Decimal::ZERO {
        (total_bid_vol / total_vol * dec!(100)).round_dp(1)
    } else {
        Decimal::ZERO
    };

    let ask_ratio = dec!(100) - bid_ratio;

    println!(
        "\n📊 买卖力量对比 [买单:{:.1}% 卖单:{:.1}%]  |  更新次数: {}",
        bid_ratio, ask_ratio, update_count
    );

    let bar_len = 50;
    let filled = (bid_ratio / dec!(100) * Decimal::from(bar_len)).round().to_u64().unwrap_or(0) as usize;
    let empty = bar_len - filled;

    print!("Bids ");
    for _ in 0..filled {
        print!("█");
    }
    for _ in 0..empty {
        print!("░");
    }
    println!(" Asks");

    // ==================== 市场状态总结 ====================
    println!("\n📌 市场状态总结:");

    // 根据各项指标综合判断
    if features.pump_signal {
        println!("   🔴 检测到强烈拉盘信号！买单斜率陡峭，大单集中，可能即将上涨");
    } else if features.dump_signal {
        println!("   🔴 检测到强烈砸盘信号！卖单斜率陡峭，大单抛压，可能即将下跌");
    } else if features.whale_entry {
        println!("   🟡 鲸鱼正在进场！有大资金在建立仓位，密切关注");
    } else if features.whale_exit {
        println!("   🟡 鲸鱼正在离场！有大资金在撤退，注意风险");
    } else if features.bid_eating {
        println!("   🟢 买单正在主动吃筹！买方力量强劲，价格可能上涨");
    } else if features.ask_eating {
        println!("   🔴 卖单正在主动砸盘！卖方力量强劲，价格可能下跌");
    } else if features.bid_volume_change > dec!(30) {
        println!("   🟢 买单量激增！买方正在积极挂单");
    } else if features.ask_volume_change > dec!(30) {
        println!("   🔴 卖单量激增！卖方正在积极挂单");
    } else if features.bid_concentration > dec!(50) {
        println!("   🟡 买单高度集中！少数几个价位堆积了大量买单");
    } else if features.ask_concentration > dec!(50) {
        println!("   🟡 卖单高度集中！少数几个价位堆积了大量卖单");
    } else {
        println!("   🟢 市场相对平稳，无明显异常信号");
    }

    std::io::stdout().flush().unwrap();
}
