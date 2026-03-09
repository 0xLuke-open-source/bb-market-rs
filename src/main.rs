mod client;
mod codec;
mod store;

use crate::codec::binance_msg::Snapshot;
use crate::store::l2_book::{OrderBook, OrderBookFeatures};
use colored::Colorize;
use reqwest::Client;
use rust_decimal::Decimal;
use std::io::Write;
use std::time::Duration;
use tokio::sync::mpsc;
use tokio::time::Instant;


const COIN: &str = "ASTER";
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
                display_orderbook(&book, &features, update_count); // 方案1：完整表格
                // display_compact(&book, &features, update_count);    // 方案2：紧凑表格

                // display_single_line(&book, &features, update_count);    // 方案3：单行实时更新

                use std::io::{self, Write};
                io::stdout().flush().unwrap();
            }
            last_print = Instant::now();
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
    let spread = ask - bid;
    let spread_bps = if !bid.is_zero() {
        (spread / bid * Decimal::from(10000)).round_dp(2)
    } else {
        Decimal::ZERO
    };

    // 标题
    println!("╔════════════════════════════════════════════════════════════════════════════════╗");
    println!(
        "║                    Binance {} Order Book Monitor                          ║",
        book.symbol
    );
    println!("╠════════════════════════════════════════════════════════════════════════════════╣");

    // 基础数据
    println!(
        "║ 📊 基础数据                                                                      ║"
    );
    println!("╠────────────────────────────────────────────────────────────────────────────────║");
    println!(
        "║  Bid: {:>18}  |  Ask: {:>18}  ║",
        format!("{}", bid).green(),
        format!("{}", ask).red()
    );
    println!(
        "║  Spread: {:>14} ({:>6} bps)  |  Updates: {:>16}  ║",
        spread, spread_bps, update_count
    );
    println!("║  Last Update ID: {:>42}  ║", book.last_update_id);
    println!("╠────────────────────────────────────────────────────────────────────────────────║");

    // 订单簿深度
    println!(
        "║ 📈 订单簿深度                                                                    ║"
    );
    println!("╠────────────────────────────────────────────────────────────────────────────────║");
    println!(
        "║  Bids: {:>6} 个价格档位  |  Total Bid Vol: {:>12.6} {COIN}  ║",
        book.bids.len().to_string().green(),
        book.bids.values().sum::<Decimal>()
    );
    println!(
        "║  Asks: {:>6} 个价格档位  |  Total Ask Vol: {:>12.6} {COIN}  ║",
        book.asks.len().to_string().red(),
        book.asks.values().sum::<Decimal>()
    );
    println!("╠────────────────────────────────────────────────────────────────────────────────║");

    // 特征分析
    println!(
        "║ 🔍 市场特征分析                                                                  ║"
    );
    println!("╠────────────────────────────────────────────────────────────────────────────────║");

    // 鲸鱼检测
    let whale_indicator = match (features.whale_bid, features.whale_ask) {
        (true, true) => "🐋 买单 + 卖单".yellow(),
        (true, false) => "🐋 买单".yellow(),
        (false, true) => "🐋 卖单".yellow(),
        (false, false) => "无".normal(),
    };
    println!("║  鲸鱼检测: {:>51}  ║", whale_indicator);

    // 斜率
    println!(
        "║  买单斜率: {:>20.6}  |  卖单斜率: {:>20.6}  ║",
        if features.slope_bid > Decimal::ZERO {
            features.slope_bid.to_string().green()
        } else {
            features.slope_bid.to_string().red()
        },
        if features.slope_ask > Decimal::ZERO {
            features.slope_ask.to_string().green()
        } else {
            features.slope_ask.to_string().red()
        }
    );

    // 流动性缺口
    println!(
        "║  流动性缺口: Bid: {:>3}处  |  Ask: {:>3}处                          ║",
        features.liquidity_gap_bid, features.liquidity_gap_ask
    );

    // 微价格
    println!("║  微价格(Microprice): {:>39.6}  ║", features.microprice);

    // OFI和买卖比例
    println!(
        "║  OFI: {:>+20.6}  |  Bid/Ask Ratio: {:>16.6}  ║",
        if features.ofi > Decimal::ZERO {
            features.ofi.to_string().green()
        } else {
            features.ofi.to_string().red()
        },
        features.bid_ask_ratio
    );

    println!("╠────────────────────────────────────────────────────────────────────────────────║");

    // 暴涨/暴跌信号
    println!(
        "║ ⚡ 市场信号                                                                       ║"
    );
    println!("╠────────────────────────────────────────────────────────────────────────────────║");

    let pump_signal = if features.pump_flag {
        "⚡ 暴涨信号！".bright_green().bold()
    } else {
        "   无信号    ".normal()
    };

    let dump_signal = if features.dump_flag {
        "⚡ 暴跌信号！".bright_red().bold()
    } else {
        "   无信号    ".normal()
    };

    println!("║  {}  |  {}  ║", pump_signal, dump_signal);
    println!("╚════════════════════════════════════════════════════════════════════════════════╝");

    // Top 5 订单簿
    println!("\n📋 Top 5 订单簿深度");
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

        println!(
            "│ {:>13.6} │ {:>11.6} {COIN} │ {:>13.6} │ {:>11.6} {COIN} │",
            bid_price, bid_qty, ask_price, ask_qty
        );
    }
    println!("└───────────────┴───────────────┴───────────────┴───────────────┘");

    // 进度条（可选）
    let total_depth = (book.bids.len() + book.asks.len()) as f64;
    let ratio = book.bids.len() as f64 / total_depth;
    let bar_len = 40;
    let filled = (ratio * bar_len as f64) as usize;
    let empty = bar_len - filled;

    println!(
        "\n📊 买卖力量对比 [Bids:{:3} Asks:{:3}]",
        book.bids.len(),
        book.asks.len()
    );
    print!("Bids ");
    for _ in 0..filled {
        print!("█");
    }
    for _ in 0..empty {
        print!("░");
    }
    println!(" Asks");

    std::io::stdout().flush().unwrap();
}

fn display_compact(book: &OrderBook, features: &OrderBookFeatures, update_count: u64) {
    let (bid, ask) = book
        .best_bid_ask()
        .unwrap_or((Decimal::ZERO, Decimal::ZERO));
    let spread = ask - bid;

    print!("\x1B[2J\x1B[1;1H"); // 清屏

    println!("┌─────────────────────────────────────────────────────────────────────────────┐");
    println!(
        "│ Binance {} Order Book                                         Updates: {:>6} │",
        book.symbol, update_count
    );
    println!("├───────────────┬───────────────┬───────────────┬─────────────────────────────┤");
    println!("│ Bid           │ Ask           │ Spread        │ Features                     │");
    println!("├───────────────┼───────────────┼───────────────┼─────────────────────────────┤");
    println!(
        "│ {:<13} │ {:<13} │ {:<13} │ Whale: {}{}                       │",
        format!("{}", bid).green(),
        format!("{}", ask).red(),
        format!("{}", spread),
        if features.whale_bid { "B" } else { "-" },
        if features.whale_ask { "A" } else { "-" }
    );
    println!(
        "│               │               │               │ Slope: {:>6.4}/{:>6.4}              │",
        features.slope_bid, features.slope_ask
    );
    println!(
        "│ Bids: {:<4}    │ Asks: {:<4}    │               │ {} {}                      │",
        book.bids.len().to_string().green(),
        book.asks.len().to_string().red(),
        if features.pump_flag {
            "⚡Pump "
        } else {
            "      "
        },
        if features.dump_flag {
            "⚡Dump"
        } else {
            "     "
        }
    );
    println!("└───────────────┴───────────────┴───────────────┴─────────────────────────────┘");

    std::io::stdout().flush().unwrap();
}

use colored::*;
use const_format::concatcp;

fn display_single_line(book: &OrderBook, features: &OrderBookFeatures, update_count: u64) {
    let (bid, ask) = book
        .best_bid_ask()
        .unwrap_or((Decimal::ZERO, Decimal::ZERO));
    let spread = ask - bid;

    // 根据信号添加表情
    let sentiment = if features.pump_flag {
        "🚀"
    } else if features.dump_flag {
        "📉"
    } else if book.bids.len() > book.asks.len() {
        "📈"
    } else {
        "➡️"
    };

    // 鲸鱼指示
    let whale = match (features.whale_bid, features.whale_ask) {
        (true, true) => "🐋🐋".yellow(),
        (true, false) => "🐋B ".yellow(),
        (false, true) => "🐋A ".yellow(),
        (false, false) => "    ".normal(),
    };

    // 买卖比例条
    let total = (book.bids.len() + book.asks.len()) as f64;
    let bid_ratio = book.bids.len() as f64 / total;
    let bar_len = 20;
    let filled = (bid_ratio * bar_len as f64) as usize;
    let bar = format!(
        "{}{}",
        "█".repeat(filled).green(),
        "█".repeat(bar_len - filled).red()
    );

    print!(
        "\r{} Bid:{:>12} Ask:{:>12} | Spread:{:>8} | Bids:{:>4} Asks:{:>4} | Slope:{:>.2}/{:>.2} | {} | {} {}      ",
        sentiment,
        bid.to_string().green(),
        ask.to_string().red(),
        spread,
        book.bids.len().to_string().green(),
        book.asks.len().to_string().red(),
        features.slope_bid,
        features.slope_ask,
        whale,
        bar,
        update_count,
    );

    std::io::stdout().flush().unwrap();
}
