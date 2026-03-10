// src/main.rs
mod client;
mod codec;
mod store;
mod analysis;
mod symbols;

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
use crate::analysis::algorithms::MarketIntelligence;
use crate::analysis::MarketAnalysis;
use clap::Parser;
use crate::symbols::sync_symbols;

const COIN: &str = "BTC";
const SYMBOL: &str = concatcp!(COIN, "USDT");


#[derive(Parser, Debug)]
#[clap(author, version, about, long_about = None)]
struct Args {
    /// 是否只同步 USDT 交易对列表并退出
    #[clap(short, long, action)]
    sync_usdt: bool,

    /// 同步 USDT 交易对列表时，输出的文件名
    #[clap(short, long, default_value = "usdt_symbols.txt")]
    output: String,
}


#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let args = Args::parse();

    // 如果没有参数或只有 --help，clap 会自动处理帮助信息
    // 所以我们不需要手动处理 help 参数

    rustls::crypto::ring::default_provider()
        .install_default()
        .expect("Failed to install rustls crypto provider");
    tracing_subscriber::fmt::init();

    let client = Client::builder().timeout(Duration::from_secs(15)).build()?;

    // 如果指定了 --sync-usdt，执行同步任务后直接退出
    if args.sync_usdt {
        sync_symbols::sync_usdt_symbols(&client, &args.output).await?;
        return Ok(());
    }

    // 正常启动监控
    start_monitoring(client).await
}

async fn start_monitoring(client: Client) -> anyhow::Result<()> {
    let (tx, mut rx) = mpsc::channel(2000);
    let mut book = OrderBook::new(SYMBOL);
    let mut market_intel = MarketIntelligence::new();
    let max_connection_duration = Duration::from_secs(23 * 60 * 60);

    // 启动 WebSocket 采集流
    let symbol_task = SYMBOL.to_string();
    let tx_clone = tx.clone();
    tokio::spawn(async move {
        loop {
            let start_time = Instant::now();
            match client::websocket::run_client(&symbol_task, tx_clone.clone()).await {
                Ok(()) => println!("WebSocket client exited normally"),
                Err(e) => eprintln!("WebSocket Error: {}", e),
            }
            if start_time.elapsed() >= max_connection_duration {
                println!("Connection duration approaching 24h limit, forcing reconnect");
            }
            tokio::time::sleep(Duration::from_secs(5)).await;
        }
    });
    println!("Step 1: WebSocket client started");

    tokio::time::sleep(Duration::from_secs(2)).await;
    println!("Step 2: Fetching snapshot from REST API...");

    let snapshot = fetch_snapshot_with_retry(&client, SYMBOL, 5).await?;
    book.init_from_snapshot(snapshot);
    println!("Step 3: Snapshot initialized. ID: {}", book.last_update_id);

    let (top_bids, top_asks) = book.top_n(5);
    println!("Top 5 Bids: {:?}", top_bids);
    println!("Top 5 Asks: {:?}", top_asks);

    let mut last_print = Instant::now();
    let print_interval = Duration::from_millis(100);
    let mut last_report = Instant::now();
    let report_interval = Duration::from_secs(20);
    let mut update_count = 0;

    while let Some(msg) = rx.recv().await {
        if let Err(e) = book.apply_incremental_update(msg) {
            eprintln!("\nUpdate Error: {}", e);
            break;
        }

        update_count += 1;

        if last_print.elapsed() >= print_interval {
            if let Some((bid, ask)) = book.best_bid_ask() {
                let spread = ask - bid;
                let spread_bps = if !bid.is_zero() {
                    (spread / bid * Decimal::from(10000)).round_dp(2)
                } else {
                    Decimal::from(0)
                };

                let features = book.compute_features(10);
                io::stdout().flush()?;
            }
            last_print = Instant::now();
        }

        if last_report.elapsed() >= report_interval {
            if let Some((bid, ask)) = book.best_bid_ask() {
                let features = book.compute_features(10);
                book.auto_sample(&features);

                let analysis = MarketAnalysis::new(&book, &features);
                let comprehensive = market_intel.analyze(&book, &features);

                analysis.display();
                market_intel.display_summary(&comprehensive);

                // 多周期分析
                let divergence_signals = market_intel.detect_multi_period_divergence(&book);
                let acceleration = market_intel.calculate_acceleration_curve(&book);
                let coherence = market_intel.analyze_trend_coherence(&book);

                println!("\n{}", "📈".repeat(20));
                println!("📊 多周期趋势分析");
                println!("{}", "📈".repeat(20));

                if !divergence_signals.is_empty() {
                    println!("\n🔄 多周期背离检测:");
                    for signal in divergence_signals {
                        let color = if signal.direction.contains("看跌") { "🔴" } else { "🟢" };
                        println!("  {} [{}] {} - 强度:{}% - {}",
                                 color, signal.period, signal.direction, signal.strength, signal.description);
                    }
                } else {
                    println!("\n🔄 未检测到明显背离");
                }

                println!("\n📈 加速度曲线:");
                println!("  5s: {:.6} | 1m: {:.6} | 5m: {:.6} | 1h: {:.6}",
                         acceleration.micro, acceleration.short,
                         acceleration.medium, acceleration.long);

                let coh_color = if coherence.coherence.contains("高度共振") {
                    "🟢"
                } else if coherence.coherence.contains("分歧") {
                    "🟡"
                } else {
                    "🔴"
                };
                println!("\n🎯 趋势共振分析: {} {} (std:{:.4})",
                         coh_color, coherence.coherence, coherence.std_deviation);
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