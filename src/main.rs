// src/main.rs — 集成 Web Dashboard
//
// 新增参数：
//   --web           启用 Web Dashboard（默认端口 9527）
//   --port <PORT>   指定端口（配合 --web 使用）
//
// 启动后访问：http://127.0.0.1:9527

mod client;
mod codec;
mod store;
mod analysis;
mod symbols;
mod web;   // 新增 Web Dashboard 模块

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
use crate::web::state::new_dashboard_state;
use crate::web::bridge::run_bridge;
use crate::web::server::run_server;

const COIN: &str = "ASTR";
const SYMBOL: &str = concatcp!(COIN, "USDT");

#[derive(Parser, Debug)]
#[clap(
    author,
    version,
    about = "BB-Market 实时监控系统",
    long_about = "Binance L2 订单簿实时分析 + Web Dashboard"
)]
struct Args {
    /// 同步 USDT 交易对列表并退出
    #[clap(long, action)]
    sync_usdt: bool,

    /// 输出文件
    #[clap(short, long, default_value = "usdt_symbols.txt")]
    output: String,

    /// 多币种监控模式
    #[clap(short, long, action)]
    multi: bool,

    /// 监控币种数量
    #[clap(short, long, default_value_t = 10)]
    count: usize,

    /// 币种列表文件
    #[clap(long)]
    symbol_file: Option<String>,

    /// 🆕 启用 Web Dashboard（浏览器可视化）
    #[clap(long, action)]
    web: bool,

    /// Web Dashboard 端口（默认 9527）
    #[clap(long, default_value_t = 9527)]
    port: u16,
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let args = Args::parse();

    rustls::crypto::ring::default_provider()
        .install_default()
        .expect("Failed to install rustls crypto provider");
    tracing_subscriber::fmt::init();

    let client = Client::builder().timeout(Duration::from_secs(15)).build()?;

    if args.sync_usdt {
        sync_symbols::sync_usdt_symbols(&client, &args.output).await?;
        return Ok(());
    }

    if args.multi {
        start_multi_monitoring(args).await
    } else {
        start_monitoring(client).await
    }
}

// ─────────────────────────────────────────────────────────────────
// 多币种监控（可选 Web Dashboard）
// ─────────────────────────────────────────────────────────────────
async fn start_multi_monitoring(args: Args) -> anyhow::Result<()> {
    fs::create_dir_all("reports")?;

    let monitor = Arc::new(MultiSymbolMonitor::new(20));
    let port    = args.port;
    let web_on  = args.web;

    let symbols = if let Some(file) = args.symbol_file {
        monitor.load_symbols_from_file(&file, args.count).await?
    } else {
        monitor.load_symbols_from_file("usdt_symbols.txt", args.count).await?
    };

    println!("📋 监控 {} 个币种:", symbols.len());
    for (i, s) in symbols.iter().enumerate() {
        println!("  {}. {}", i + 1, s);
    }

    monitor.init_monitors(symbols.clone()).await;
    let mut manager = MultiWebSocketManager::new(monitor.clone());

    // ── 异动汇总任务 ───────────────────────────────────────────
    let anomaly_monitor = monitor.clone();
    tokio::spawn(async move {
        let mut tick = tokio::time::interval(Duration::from_secs(10));
        loop {
            tick.tick().await;
            write_global_anomaly_summary(&anomaly_monitor).await.ok();

            // 控制台 TOP5
            let monitors = anomaly_monitor.monitors.lock().await;
            let mut top: Vec<(String, u32)> = Vec::new();
            for (sym, arc) in monitors.iter() {
                let m = arc.lock().await;
                let cnt = m.anomaly_detector.get_stats().last_minute_count;
                if cnt > 0 { top.push((sym.clone(), cnt)); }
            }
            top.sort_by(|a, b| b.1.cmp(&a.1));
            if !top.is_empty() && !web_on {
                println!("\n🔥 异动 TOP5:");
                for (i, (s, c)) in top.iter().take(5).enumerate() {
                    println!("  {}. {}: {} 次", i + 1, s, c);
                }
            }
        }
    });

    // ── 拉盘检测任务 ───────────────────────────────────────────
    let pump_monitor = monitor.clone();
    tokio::spawn(async move {
        let mut tick = tokio::time::interval(Duration::from_secs(10));
        loop {
            tick.tick().await;
            let mgr = MultiWebSocketManager::new(pump_monitor.clone());
            mgr.detect_pump_signals().await.ok();
        }
    });

    // ── Web Dashboard ───────────────────────────────────────────
    if web_on {
        let dash_state = new_dashboard_state();

        // 数据桥接任务（每 500ms 同步）
        let bridge_monitor = monitor.clone();
        let bridge_dash    = dash_state.clone();
        tokio::spawn(async move {
            run_bridge(bridge_monitor, bridge_dash, 500).await;
        });

        // HTTP + WebSocket 服务器
        let server_dash = dash_state.clone();
        tokio::spawn(async move {
            if let Err(e) = run_server(server_dash, port).await {
                eprintln!("❌ Web 服务器错误: {}", e);
            }
        });

        println!("\n╔══════════════════════════════════════════════╗");
        println!("║  🌐 Dashboard: http://127.0.0.1:{}         ║", port);
        println!("║  浏览器打开即可查看实时图表                    ║");
        println!("╚══════════════════════════════════════════════╝\n");
    }

    // ── 启动所有 WebSocket 连接 ─────────────────────────────────
    manager.start_all(symbols).await;
    manager.wait().await;
    Ok(())
}

// ─────────────────────────────────────────────────────────────────
// 单币种监控（原逻辑不变）
// ─────────────────────────────────────────────────────────────────
async fn start_monitoring(client: Client) -> anyhow::Result<()> {
    let (tx, mut rx) = mpsc::channel(2000);
    let mut book = OrderBook::new(SYMBOL);
    let mut market_intel = MarketIntelligence::new();
    let max_connection_duration = Duration::from_secs(23 * 60 * 60);

    let sym_task = SYMBOL.to_string();
    let tx_clone = tx.clone();
    tokio::spawn(async move {
        loop {
            let t0 = Instant::now();
            match client::websocket::run_client(&sym_task, tx_clone.clone()).await {
                Ok(())  => println!("WebSocket exited normally"),
                Err(e)  => eprintln!("WebSocket Error: {}", e),
            }
            if t0.elapsed() >= max_connection_duration {
                println!("Connection 24h limit, forcing reconnect");
            }
            tokio::time::sleep(Duration::from_secs(5)).await;
        }
    });

    tokio::time::sleep(Duration::from_secs(2)).await;
    let snapshot = fetch_snapshot_with_retry(&client, SYMBOL, 5).await?;
    book.init_from_snapshot(snapshot);
    println!("Snapshot initialized. ID: {}", book.last_update_id);

    let mut last_print  = Instant::now();
    let mut last_report = Instant::now();
    let print_interval  = Duration::from_millis(100);
    let report_interval = Duration::from_secs(20);

    while let Some(msg) = rx.recv().await {
        if let Err(e) = book.apply_incremental_update(msg) {
            eprintln!("Update Error: {}", e);
            break;
        }
        if last_print.elapsed() >= print_interval {
            book.compute_features(10);
            std::io::stdout().flush()?;
            last_print = Instant::now();
        }
        if last_report.elapsed() >= report_interval {
            if let Some((bid, ask)) = book.best_bid_ask() {
                let features = book.compute_features(10);
                book.auto_sample(&features);
                let analysis = MarketAnalysis::new(&book, &features);
                let comp     = market_intel.analyze(&book, &features);
                analysis.display();
                market_intel.display_summary(&comp);
            }
            last_report = Instant::now();
        }
    }
    Ok(())
}

// ── 工具函数 ─────────────────────────────────────────────────────

async fn write_global_anomaly_summary(monitor: &MultiSymbolMonitor) -> std::io::Result<()> {
    let mut file = std::fs::OpenOptions::new()
        .create(true).append(true)
        .open("reports/global_anomalies.txt")?;

    writeln!(file, "\n{}", "=".repeat(100))?;
    writeln!(file, "📊 全局异动汇总 - {}", chrono::Local::now().format("%Y-%m-%d %H:%M:%S"))?;
    writeln!(file, "{}", "=".repeat(100))?;

    let monitors = monitor.monitors.lock().await;
    for (symbol, arc) in monitors.iter() {
        let guard = arc.lock().await;
        let stats = guard.anomaly_detector.get_stats();
        if stats.total_events > 0 {
            writeln!(file,
                     "{}: 总异动 {} | 近1分 {} | 严重度 {:.1} | 最高 {}",
                     symbol, stats.total_events, stats.last_minute_count,
                     stats.avg_severity, stats.max_severity
            )?;
        }
    }
    file.flush()
}

async fn fetch_snapshot_with_retry(
    client:      &Client,
    symbol:      &str,
    max_retries: u32,
) -> anyhow::Result<Snapshot> {
    let endpoints = [
        "https://api.binance.com",
        "https://api1.binance.com",
        "https://api2.binance.com",
        "https://api3.binance.com",
    ];
    for retry in 0..max_retries {
        for &base in &endpoints {
            let url = format!("{}/api/v3/depth?symbol={}&limit=1000", base, symbol);
            match client.get(&url).send().await {
                Ok(resp) if resp.status().is_success() => {
                    if let Ok(snap) = resp.json::<Snapshot>().await {
                        println!("✅ Snapshot from {}", base);
                        return Ok(snap);
                    }
                }
                _ => {}
            }
        }
        if retry < max_retries - 1 {
            let w = Duration::from_secs(2u64.pow(retry + 1));
            println!("Retrying in {}s...", w.as_secs());
            tokio::time::sleep(w).await;
        }
    }
    anyhow::bail!("Failed to fetch snapshot after {} retries", max_retries)
}