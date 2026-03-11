// src/multi_monitor.rs
use std::collections::HashMap;
use std::fs::File;
use std::io::{BufRead, BufReader, Write};
use std::sync::Arc;
use rust_decimal::prelude::ToPrimitive;
use tokio::sync::{mpsc, Mutex};
use tokio::time::{self, Duration, Instant};
use crate::codec::binance_msg::DepthUpdate;
use crate::store::l2_book::{OrderBook, OrderBookFeatures};
use crate::analysis::algorithms::MarketIntelligence;
use crate::analysis::MarketAnalysis;
use crate::analysis::pump_detector::PumpDetector;




// 添加一个静态的拉盘检测器
lazy_static::lazy_static! {
        static ref PUMP_DETECTOR: PumpDetector = PumpDetector::new("pump_signals.txt")
            .with_min_strength(30);
    }

// 单个币种的监控数据
pub struct SymbolMonitor {
    pub symbol: String,
    pub book: OrderBook,
    pub market_intel: MarketIntelligence,
    pub last_report: Instant,
    pub update_count: u64,
    pub report_file: String,  // 每个币种对应的报告文件
}

impl SymbolMonitor {
    pub fn new(symbol: &str) -> Self {
        // 为每个币种创建独立的报告文件
        let report_file = format!("reports/{}_{}.txt",
                                  symbol.to_lowercase(),
                                  chrono::Local::now().format("%Y%m%d_%H%M%S")
        );

        // 确保 reports 目录存在
        std::fs::create_dir_all("reports").unwrap_or_default();

        // 初始化文件，写入表头
        if let Ok(mut file) = std::fs::OpenOptions::new()
            .create(true)
            .write(true)
            .truncate(true)
            .open(&report_file)
        {
            let _ = writeln!(file, "=== {} 市场分析报告 ===", symbol);
            let _ = writeln!(file, "启动时间: {}", chrono::Local::now().format("%Y-%m-%d %H:%M:%S"));
            let _ = writeln!(file, "{}", "=".repeat(80));
        }

        Self {
            symbol: symbol.to_string(),
            book: OrderBook::new(symbol),
            market_intel: MarketIntelligence::new(),
            last_report: Instant::now(),
            update_count: 0,
            report_file,
        }
    }
}

// 多币种监控器
pub struct MultiSymbolMonitor {
    monitors: Arc<Mutex<HashMap<String, SymbolMonitor>>>,
    report_interval: Duration,
}

impl MultiSymbolMonitor {
    pub fn new(report_interval_secs: u64) -> Self {
        Self {
            monitors: Arc::new(Mutex::new(HashMap::new())),
            report_interval: Duration::from_secs(report_interval_secs),
        }
    }

    // 从文件加载币种列表
    pub async fn load_symbols_from_file(&self, file_path: &str, max_count: usize) -> anyhow::Result<Vec<String>> {
        let file = File::open(file_path)?;
        let reader = BufReader::new(file);
        let mut symbols = Vec::new();

        for line in reader.lines() {
            let symbol = line?.trim().to_string();
            if !symbol.is_empty() && symbol != "币安人生" { // 过滤掉无效的币种
                symbols.push(format!("{}USDT", symbol));
                if symbols.len() >= max_count {
                    break;
                }
            }
        }

        println!("✅ 从文件加载了 {} 个币种", symbols.len());
        Ok(symbols)
    }

    // 初始化所有监控器
    pub async fn init_monitors(&self, symbols: Vec<String>) {
        let mut monitors = self.monitors.lock().await;
        for symbol in symbols {
            monitors.insert(symbol.clone(), SymbolMonitor::new(&symbol));
        }
        println!("🚀 初始化了 {} 个币种监控器", monitors.len());
    }

    // 处理深度更新
    pub async fn handle_update(&self, symbol: &str, update: DepthUpdate) -> anyhow::Result<()> {
        let mut monitors = self.monitors.lock().await;

        if let Some(monitor) = monitors.get_mut(symbol) {
            if let Err(e) = monitor.book.apply_incremental_update(update) {
                eprintln!("[{}] Update error: {}", symbol, e);
                return Ok(());
            }

            monitor.update_count += 1;

            // 定期生成报告
            if monitor.last_report.elapsed() >= self.report_interval {
                if let Some((bid, ask)) = monitor.book.best_bid_ask() {
                    let features = monitor.book.compute_features(10);
                    monitor.book.auto_sample(&features);

                    // 生成分析报告
                    let analysis = MarketAnalysis::new(&monitor.book, &features);
                    let comprehensive = monitor.market_intel.analyze(&monitor.book, &features);

                    // 打印到控制台（精简版）
                    println!("\n{}", "=".repeat(100));
                    println!("📊 币种: {} (更新次数: {})", symbol, monitor.update_count);
                    println!("{}", "=".repeat(100));
                    println!("💰 价格: Bid: {:.6} | Ask: {:.6} | Spread: {:.2} bps",
                             bid, ask, features.spread_bps);
                    println!("📈 OBI: {:.1}% | OFI: {:.0} | 趋势强度: {:.1}",
                             features.obi, features.ofi, features.trend_strength);

                    // 显示信号
                    let mut signals = Vec::new();
                    if features.pump_signal { signals.push("🚀 拉盘"); }
                    if features.dump_signal { signals.push("📉 砸盘"); }
                    if features.whale_entry { signals.push("🐋 鲸鱼进场"); }
                    if features.whale_exit { signals.push("🐋 鲸鱼离场"); }
                    if features.bid_eating { signals.push("🍽️ 吃筹"); }
                    if features.ask_eating { signals.push("💥 砸盘"); }

                    if !signals.is_empty() {
                        println!("🔔 信号: {}", signals.join(" | "));
                    }

                    // 写入文件（完整版）
                    if let Err(e) = analysis.write_to_file(&monitor.report_file) {
                        eprintln!("[{}] Failed to write report to file: {}", symbol, e);
                    }

                    // 可选：打印简短的确认信息
                    println!("📝 报告已追加到文件: {}", monitor.report_file);
                }
                monitor.last_report = Instant::now();
            }
        }

        Ok(())
    }

    // 获取所有活跃的币种
    pub async fn get_active_symbols(&self) -> Vec<String> {
        let monitors = self.monitors.lock().await;
        monitors.keys().cloned().collect()
    }
}

// 多币种 WebSocket 管理器
pub struct MultiWebSocketManager {
    monitors: Arc<MultiSymbolMonitor>,
    tasks: tokio::task::JoinSet<()>,
}

impl MultiWebSocketManager {
    pub fn new(monitors: Arc<MultiSymbolMonitor>) -> Self {
        Self {
            monitors,
            tasks: tokio::task::JoinSet::new(),
        }
    }

    // 为每个币种启动一个 WebSocket 连接
    pub async fn start_all(&mut self, symbols: Vec<String>) {
        for symbol in symbols {
            let monitors = self.monitors.clone();
            let symbol_clone = symbol.clone();

            self.tasks.spawn(async move {
                loop {
                    println!("🔄 连接 [{}] WebSocket...", symbol_clone);

                    // 为每个币种创建独立的 channel
                    let (tx, mut rx) = mpsc::channel(1000);

                    // 启动 WebSocket 客户端
                    let ws_symbol = symbol_clone.clone();
                    let ws_task = tokio::spawn(async move {
                        loop {
                            match crate::client::websocket::run_client(&ws_symbol, tx.clone()).await {
                                Ok(()) => println!("[{}] WebSocket exited", ws_symbol),
                                Err(e) => eprintln!("[{}] WebSocket error: {}", ws_symbol, e),
                            }
                            tokio::time::sleep(Duration::from_secs(5)).await;
                        }
                    });

                    // 处理接收到的消息
                    while let Some(update) = rx.recv().await {
                        if let Err(e) = monitors.handle_update(&symbol_clone, update).await {
                            eprintln!("[{}] Handle error: {}", symbol_clone, e);
                        }
                    }

                    ws_task.abort();
                    tokio::time::sleep(Duration::from_secs(5)).await;
                }
            });

            // 稍微错开连接时间，避免同时连接
            tokio::time::sleep(Duration::from_millis(500)).await;
        }
    }

    // 等待所有任务完成（实际上不会完成）
    pub async fn wait(&mut self) {
        while let Some(result) = self.tasks.join_next().await {
            if let Err(e) = result {
                eprintln!("Task error: {}", e);
            }
        }
    }






    // 独立的拉盘检测函数 - 不修改原有逻辑
    pub async fn detect_pump_signals(&self) -> anyhow::Result<()> {
        let mut monitors = self.monitors.monitors.lock().await;
        let mut signals = Vec::new();

        for (symbol, monitor) in monitors.iter_mut() {
            let features = monitor.book.compute_features(10);

            // 运行完整分析获取概率数据
            let analysis = monitor.market_intel.analyze(&monitor.book, &features);

            // 分析拉盘信号
            if let Some(signal) = PUMP_DETECTOR.analyze_symbol(
                symbol,
                &features,
                analysis.pump_dump.pump_probability,
                analysis.whale.accumulation_score.to_u8().unwrap_or(0),
                analysis.pump_dump.pump_target,
            ) {
                // 单个写入
                let _ = PUMP_DETECTOR.write_pump_signal(&signal);
                signals.push(signal);
            }
        }

        // 每10次检测写一次TOP汇总
        use std::sync::atomic::{AtomicUsize, Ordering};
        static COUNTER: AtomicUsize = AtomicUsize::new(0);

        let count = COUNTER.fetch_add(1, Ordering::SeqCst);
        if count % 10 == 0 && !signals.is_empty() {
            let _ = PUMP_DETECTOR.write_top_signals(&mut signals);

            // 可选：同时在控制台显示
            PUMP_DETECTOR.print_top_signals(&signals, 5);
        }

        Ok(())
    }
}

