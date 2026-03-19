// src/main.rs
mod client;
mod codec;
mod store;
mod analysis;
mod symbols;

use std::fs;
use std::io::Write;
use std::sync::Arc;
use crate::codec::binance_msg::Snapshot;
use crate::store::l2_book::OrderBook;
use reqwest::Client;
use std::time::Duration;
use const_format::concatcp;
use tokio::sync::mpsc;
use tokio::time::Instant;
use clap::Parser;
use crate::analysis::algorithms::MarketIntelligence;
use crate::analysis::MarketAnalysis;
use crate::symbols::sync_symbols;
use crate::analysis::multi_monitor::{MultiSymbolMonitor, MultiWebSocketManager};

const COIN: &str = "ASTR";
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

    /// 多币种监控模式
    #[clap(short, long, action)]
    multi: bool,

    /// 监控的币种数量 (默认: 10)
    #[clap(short, long, default_value_t = 10)]
    count: usize,

    /// 指定要监控的币种列表文件
    #[clap(long)]
    symbol_file: Option<String>,
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let args = Args::parse();

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

    // 多币种监控模式
    if args.multi {
        start_multi_monitoring(args).await
    } else {
        // 单币种监控模式
        start_monitoring(client).await
    }
}

async fn start_multi_monitoring(args: Args) -> anyhow::Result<()> {
    println!("🚀 启动多币种监控模式...");

    // 创建 reports 目录
    fs::create_dir_all("reports")?;
    println!("📁 报告将保存到 reports 目录");
    println!("📁 拉盘信号将保存到 pump_signals.txt");
    println!("📁 异动日志将保存到 anomaly/*.txt");

    // 创建监控器
    let monitor = Arc::new(MultiSymbolMonitor::new(20)); // 每20秒报告一次

    // 确定要监控的币种
    let symbols = if let Some(file) = args.symbol_file {
        // 从指定文件加载
        monitor.load_symbols_from_file(&file, args.count).await?
    } else {
        // 从默认文件加载
        monitor.load_symbols_from_file("usdt_symbols.txt", args.count).await?
    };

    println!("📋 将监控以下 {} 个币种:", symbols.len());
    for (i, symbol) in symbols.iter().enumerate() {
        println!("  {}. {}", i+1, symbol);
    }

    // 初始化监控器
    monitor.init_monitors(symbols.clone()).await;

    // 启动 WebSocket 管理器
    let mut manager = MultiWebSocketManager::new(monitor.clone());


    // ===== 新增：异动监控任务 =====
    let anomaly_monitor = monitor.clone();
    tokio::spawn(async move {
        let mut interval = tokio::time::interval(Duration::from_secs(10)); // 每5秒汇总一次
        println!("🔍 异动监控任务已启动 (每10秒汇总一次)");

        loop {
            interval.tick().await;

            // 调用全局异动汇总函数
            if let Err(e) = write_global_anomaly_summary(&anomaly_monitor).await {
                eprintln!("❌ 全局异动汇总错误: {}", e);
            }

            // 找出当前异动最频繁的币种
            let monitors = anomaly_monitor.monitors.lock().await;
            let mut top_anomalies: Vec<(String, u32)> = monitors.iter()
                .map(|(symbol, m)| (symbol.clone(), m.anomaly_detector.get_stats().last_minute_count))
                .filter(|(_, count)| *count > 0)
                .collect();

            // 按异动数量排序
            top_anomalies.sort_by(|a, b| b.1.cmp(&a.1));

            // 打印TOP 5异动最频繁的币种
            if !top_anomalies.is_empty() {
                println!("\n{}", "🔥".repeat(30));
                println!("📊 最近1分钟异动最频繁的币种 TOP {}", top_anomalies.len().min(5));
                println!("{}", "🔥".repeat(30));

                for (i, (symbol, count)) in top_anomalies.iter().take(5).enumerate() {
                    let medal = if i == 0 { "🥇" } else if i == 1 { "🥈" } else if i == 2 { "🥉" } else { "  " };
                    println!("{} {}: {} 次异动", medal, symbol, count);
                }
                println!("{}", "🔥".repeat(30));
            }
        }
    });


    // ===== 新增：启动独立的拉盘检测任务 =====
    let pump_monitor = monitor.clone();
    tokio::spawn(async move {
        let mut interval = tokio::time::interval(Duration::from_secs(10)); // 每3秒检测一次
        println!("🔍 拉盘信号检测任务已启动 (每10秒检测一次)");

        loop {
            interval.tick().await;

            // 创建临时的 WebSocket 管理器用于检测
            let pump_manager = MultiWebSocketManager::new(pump_monitor.clone());
            if let Err(e) = pump_manager.detect_pump_signals().await {
                eprintln!("❌ 拉盘检测错误: {}", e);
            }
        }
    });

    // 启动所有 WebSocket 连接
    println!("🔄 正在启动 WebSocket 连接...");
    manager.start_all(symbols).await;

    // 等待（不会返回）
    println!("✅ 所有监控任务已启动，开始监控...");
    manager.wait().await;

    Ok(())
}

async fn start_monitoring(client: Client) -> anyhow::Result<()> {
    // ... 原有的单币种监控代码保持不变 ...
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
                let _features = book.compute_features(10);
                std::io::stdout().flush()?;
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

async fn write_global_anomaly_summary(monitor: &MultiSymbolMonitor) -> std::io::Result<()> {
    let mut file = std::fs::OpenOptions::new()
        .create(true)
        .append(true)
        .open("reports/global_anomalies.txt")?;

    writeln!(file, "\n{}", "=".repeat(100))?;
    writeln!(file, "📊 全局异动汇总 - {}", chrono::Local::now().format("%Y-%m-%d %H:%M:%S"))?;
    writeln!(file, "{}", "=".repeat(100))?;

    let monitors = monitor.monitors.lock().await;

    // 收集所有币种的异动统计
    for (symbol, monitor) in monitors.iter() {
        let stats = monitor.anomaly_detector.get_stats();
        if stats.total_events > 0 {
            writeln!(
                file,
                "{}: 总异动 {} | 最近1分钟 {} | 严重度 {:.1} | 最高 {}",
                symbol,
                stats.total_events,
                stats.last_minute_count,
                stats.avg_severity,
                stats.max_severity
            )?;
        }
    }

    file.flush()?;
    Ok(())
}

async fn fetch_snapshot_with_retry(
    client: &Client,
    symbol: &str,
    max_retries: u32,
) -> anyhow::Result<Snapshot> {
    // ... 保持不变 ...
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