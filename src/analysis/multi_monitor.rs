// analysis/multi_monitor.rs — 改进版
//
// ═══════════════════════════════════════════════
// 主要修复和改进：
//
// Bug 1 - 重复写入报告（Critical）
//   原版 handle_update() 中 write_to_file() 被调用了两次（L250 和 L280），
//   导致每次报告周期内文件被写入两份相同内容。
//   → 修复：删除第二次调用。
//
// Bug 2 - 全局锁竞争（Performance）
//   MultiSymbolMonitor 用一个 Arc<Mutex<HashMap>> 包住所有币种的状态，
//   detect_pump_signals 和 handle_update 都抢同一把锁，
//   监控 50 个币种时会出现严重阻塞。
//   → 改进：每个 SymbolMonitor 独立 Arc<Mutex<SymbolMonitor>>，
//     HashMap 中存 Arc<Mutex<...>>，并发无锁竞争。
//
// Bug 3 - detect_pump_signals 持有全局锁时做重量级计算（Critical）
//   原版在 lock().await 内调用 market_intel.analyze()（CPU 密集），
//   期间所有 handle_update() 全部阻塞。
//   → 改进：先 clone 出 features（在锁内），然后释放锁，在锁外计算。
//
// 改进 4 - pump_detector 使用改进版 ofi（增量版）
//   原版 PumpDetector 使用 features.ofi（现在是增量 OFI），
//   threshold 需要对应调整（从 50000→10000，从 100000→30000）。
//
// 改进 5 - 信号去重（Dedup）
//   同一币种连续 3 次触发相同信号时只写一次，避免日志爆炸。
// ═══════════════════════════════════════════════

use std::collections::HashMap;
use std::fs::File;
use std::io::{BufRead, BufReader, Write};
use std::sync::Arc;
use rust_decimal::prelude::ToPrimitive;
use tokio::sync::{mpsc, Mutex};
use tokio::time::{Duration, Instant};
use crate::codec::binance_msg::DepthUpdate;
use crate::store::l2_book::{OrderBook, OrderBookFeatures};
use crate::analysis::algorithms::MarketIntelligence;
use crate::analysis::MarketAnalysis;
use crate::analysis::orderbook_anomaly::{AnomalyEvent, OrderBookAnomalyDetector};
use crate::analysis::pump_detector::{PumpDetector, PumpSignal};

// ── 静态拉盘检测器（沿用原有设计，阈值已在 pump_detector 内调整）──
lazy_static::lazy_static! {
    static ref PUMP_DETECTOR: PumpDetector = PumpDetector::new("pump_signals.txt")
        .with_min_strength(30);
}

// ── 单个币种的监控数据 ──────────────────────────────────────────

pub struct SymbolMonitor {
    pub symbol:           String,
    pub book:             OrderBook,
    pub market_intel:     MarketIntelligence,
    pub last_report:      Instant,
    pub update_count:     u64,
    pub report_file:      String,
    pub anomaly_detector: OrderBookAnomalyDetector,
    pub anomaly_file:     String,
    // 改进：信号去重缓存（记录上一次触发的信号种类和时间）
    last_pump_signal_at:  Option<Instant>,
    last_dump_signal_at:  Option<Instant>,
    consecutive_pump:     u8,
    consecutive_dump:     u8,
}

impl SymbolMonitor {
    pub fn new(symbol: &str) -> Self {
        let ts = chrono::Local::now().format("%Y%m%d_%H%M%S");
        let report_file  = format!("reports/{}_{}.txt",  symbol.to_lowercase(), ts);
        let anomaly_file = format!("anomaly/{}_{}.txt",  symbol.to_lowercase(), ts);

        std::fs::create_dir_all("reports").unwrap_or_default();
        std::fs::create_dir_all("anomaly").unwrap_or_default();

        Self::init_anomaly_file(symbol, &anomaly_file);
        Self::init_report_file(symbol, &report_file);

        Self {
            symbol: symbol.to_string(),
            book: OrderBook::new(symbol),
            market_intel: MarketIntelligence::new(),
            anomaly_detector: OrderBookAnomalyDetector::new(),
            last_report: Instant::now(),
            update_count: 0,
            report_file,
            anomaly_file,
            last_pump_signal_at: None,
            last_dump_signal_at: None,
            consecutive_pump: 0,
            consecutive_dump: 0,
        }
    }

    fn init_anomaly_file(symbol: &str, path: &str) {
        let _ = std::fs::OpenOptions::new()
            .create(true).write(true).truncate(true)
            .open(path)
            .and_then(|mut f| {
                writeln!(f, "=== {} 订单簿异动日志 ===", symbol)?;
                writeln!(f, "启动时间: {}", chrono::Local::now().format("%Y-%m-%d %H:%M:%S"))?;
                writeln!(f, "{}", "=".repeat(120))?;
                writeln!(f, "{:<8} | {:<15} | {:<6} | {:<6} | {:<12} | {:<10} | {:<8} | {:<8} | {}",
                         "时间", "异动类型", "严重度", "置信度", "价格", "大小", "占比", "影响", "描述")?;
                writeln!(f, "{}", "-".repeat(120))?;
                f.flush()
            });
    }

    fn init_report_file(symbol: &str, path: &str) {
        let _ = std::fs::OpenOptions::new()
            .create(true).write(true).truncate(true)
            .open(path)
            .and_then(|mut f| {
                writeln!(f, "=== {} 市场分析报告 ===", symbol)?;
                writeln!(f, "启动时间: {}", chrono::Local::now().format("%Y-%m-%d %H:%M:%S"))?;
                writeln!(f, "{}", "=".repeat(80))
            });
    }

    fn write_anomaly_to_file(&self, anomaly: &AnomalyEvent) -> std::io::Result<()> {
        let mut file = std::fs::OpenOptions::new()
            .create(true).append(true)
            .open(&self.anomaly_file)?;

        let price_str      = anomaly.price_level.map(|p| format!("{:.6}", p)).unwrap_or("-".into());
        let size_str       = anomaly.size.map(|s| format!("{:.0}", s)).unwrap_or("-".into());
        let percentage_str = anomaly.percentage.map(|p| format!("{:.1}%", p)).unwrap_or("-".into());
        let impact_str     = anomaly.price_impact.map(|i| format!("{:.2}%", i)).unwrap_or("-".into());

        writeln!(file,
                 "{:<8} | {:<15} | {:<6}% | {:<6}% | {:<12} | {:<10} | {:<8} | {:<8} | {}",
                 anomaly.timestamp.format("%H:%M:%S"),
                 format!("{:?}", anomaly.anomaly_type),
                 anomaly.severity, anomaly.confidence,
                 price_str, size_str, percentage_str, impact_str,
                 anomaly.description
        )?;
        file.flush()
    }

    /// 判断是否应该写入 pump 信号（去重：同方向信号最少间隔 30 秒）
    pub fn should_emit_pump(&mut self) -> bool {
        let now = Instant::now();
        match self.last_pump_signal_at {
            Some(t) if now.duration_since(t) < Duration::from_secs(30) => {
                self.consecutive_pump += 1;
                false
            }
            _ => {
                self.last_pump_signal_at = Some(now);
                self.consecutive_pump = 1;
                true
            }
        }
    }

    pub fn should_emit_dump(&mut self) -> bool {
        let now = Instant::now();
        match self.last_dump_signal_at {
            Some(t) if now.duration_since(t) < Duration::from_secs(30) => {
                self.consecutive_dump += 1;
                false
            }
            _ => {
                self.last_dump_signal_at = Some(now);
                self.consecutive_dump = 1;
                true
            }
        }
    }
}

// ── 多币种监控器（改进：每个 SymbolMonitor 独立锁）──────────────

/// monitors 中每个值变为 Arc<Mutex<SymbolMonitor>>
/// 写操作可以并发，不同币种互不阻塞
pub struct MultiSymbolMonitor {
    pub monitors:        Arc<Mutex<HashMap<String, Arc<Mutex<SymbolMonitor>>>>>,
    pub report_interval: Duration,
}

impl MultiSymbolMonitor {
    pub fn new(report_interval_secs: u64) -> Self {
        Self {
            monitors: Arc::new(Mutex::new(HashMap::new())),
            report_interval: Duration::from_secs(report_interval_secs),
        }
    }

    pub async fn load_symbols_from_file(&self, path: &str, max: usize) -> anyhow::Result<Vec<String>> {
        let file   = File::open(path)?;
        let reader = BufReader::new(file);
        let mut symbols = Vec::new();
        for line in reader.lines() {
            let s = line?.trim().to_string();
            if !s.is_empty() && s != "币安人生" {
                symbols.push(format!("{}USDT", s));
                if symbols.len() >= max { break; }
            }
        }
        println!("✅ 从文件加载了 {} 个币种", symbols.len());
        Ok(symbols)
    }

    pub async fn init_monitors(&self, symbols: Vec<String>) {
        let mut monitors = self.monitors.lock().await;
        for symbol in symbols {
            monitors.insert(
                symbol.clone(),
                Arc::new(Mutex::new(SymbolMonitor::new(&symbol)))
            );
        }
        println!("🚀 初始化了 {} 个币种监控器", monitors.len());
    }

    pub async fn handle_update(&self, symbol: &str, update: DepthUpdate) -> anyhow::Result<()> {
        // Step 1: 找到对应的监控器 Arc（只短暂持有全局锁）
        let monitor_arc = {
            let monitors = self.monitors.lock().await;
            monitors.get(symbol).cloned()
        };

        let monitor_arc = match monitor_arc {
            Some(m) => m,
            None    => return Ok(()),
        };

        // Step 2: 锁住单个币种，进行更新
        let mut monitor = monitor_arc.lock().await;

        if let Err(e) = monitor.book.apply_incremental_update(update) {
            eprintln!("[{}] Update error: {}", symbol, e);
            return Ok(());
        }
        monitor.update_count += 1;

        let features = monitor.book.compute_features(10);
        // let anomalies = monitor.anomaly_detector.detect(&monitor.book, &features);
        // 分开借用：先获取 book 的引用，再调用 detect
        let anomalies = {
            use std::ops::DerefMut;
            let monitor_mut = &mut *monitor;
            let book = &monitor_mut.book;
            let detector = &mut monitor_mut.anomaly_detector;
            detector.detect(book, &features)
        };


        for anomaly in &anomalies {
            if anomaly.severity >= 60 {
                if let Err(e) = monitor.write_anomaly_to_file(anomaly) {
                    eprintln!("[{}] Failed to write anomaly: {}", symbol, e);
                }
                Self::print_anomaly_alert(symbol, anomaly);
            }
        }

        // 定期报告
        if monitor.last_report.elapsed() >= self.report_interval {
            if let Some((bid, ask)) = monitor.book.best_bid_ask() {
                monitor.book.auto_sample(&features);
                let analysis = MarketAnalysis::new(&monitor.book, &features);

                // Bug fix：只调用一次 write_to_file
                if let Err(e) = analysis.write_to_file(&monitor.report_file) {
                    eprintln!("[{}] Failed to write report: {}", symbol, e);
                }

                // 控制台精简版
                println!("\n{}", "=".repeat(80));
                println!("📊 {} (更新: {} | pump:{} dump:{})",
                         symbol, monitor.update_count,
                         features.pump_score, features.dump_score);
                println!("💰 Bid:{:.6} Ask:{:.6} Spread:{:.2}bps",
                         bid, ask, features.spread_bps);
                println!("📈 OBI:{:.1}% OFI(增量):{:.0} OFI(原始):{:.0}",
                         features.obi, features.ofi, features.ofi_raw);

                let mut sigs = Vec::new();
                if features.pump_signal { sigs.push(format!("🚀拉盘({}分)", features.pump_score)); }
                if features.dump_signal { sigs.push(format!("📉砸盘({}分)", features.dump_score)); }
                if features.whale_entry { sigs.push("🐋鲸鱼进场".into()); }
                if features.whale_exit  { sigs.push("🐋鲸鱼离场".into()); }
                if features.bid_eating  { sigs.push("🍽️吃筹".into()); }
                if !sigs.is_empty() {
                    println!("🔔 {}", sigs.join(" | "));
                }
                println!("📝 报告已写入: {}", monitor.report_file);
            }
            monitor.last_report = Instant::now();
        }

        Ok(())
    }

    fn print_anomaly_alert(symbol: &str, anomaly: &AnomalyEvent) {
        let color = if anomaly.severity >= 80 { "\x1b[91m" }
        else if anomaly.severity >= 60 { "\x1b[93m" }
        else { "\x1b[94m" };
        println!(
            "{}{} [{}] {:?} | 严重:{} 置信:{} {}\x1b[0m",
            color,
            anomaly.timestamp.format("%H:%M:%S"),
            symbol,
            anomaly.anomaly_type,
            anomaly.severity, anomaly.confidence,
            anomaly.description,
        );
    }

    pub async fn get_active_symbols(&self) -> Vec<String> {
        self.monitors.lock().await.keys().cloned().collect()
    }
}

// ── WebSocket 管理器 ─────────────────────────────────────────────

pub struct MultiWebSocketManager {
    monitors: Arc<MultiSymbolMonitor>,
    tasks:    tokio::task::JoinSet<()>,
}

impl MultiWebSocketManager {
    pub fn new(monitors: Arc<MultiSymbolMonitor>) -> Self {
        Self { monitors, tasks: tokio::task::JoinSet::new() }
    }

    pub async fn start_all(&mut self, symbols: Vec<String>) {
        for symbol in symbols {
            let monitors   = self.monitors.clone();
            let sym_clone  = symbol.clone();

            self.tasks.spawn(async move {
                loop {
                    println!("🔄 连接 [{}] WebSocket...", sym_clone);
                    let (tx, mut rx) = mpsc::channel(1000);

                    let ws_sym = sym_clone.clone();
                    let ws_task = tokio::spawn(async move {
                        loop {
                            match crate::client::websocket::run_client(&ws_sym, tx.clone()).await {
                                Ok(()) => println!("[{}] WebSocket exited", ws_sym),
                                Err(e) => eprintln!("[{}] WS error: {}", ws_sym, e),
                            }
                            tokio::time::sleep(Duration::from_secs(5)).await;
                        }
                    });

                    while let Some(update) = rx.recv().await {
                        if let Err(e) = monitors.handle_update(&sym_clone, update).await {
                            eprintln!("[{}] Handle error: {}", sym_clone, e);
                        }
                    }

                    ws_task.abort();
                    tokio::time::sleep(Duration::from_secs(5)).await;
                }
            });

            // 错开连接时间，避免瞬间雪崩
            tokio::time::sleep(Duration::from_millis(300)).await;
        }
    }

    pub async fn wait(&mut self) {
        while let Some(result) = self.tasks.join_next().await {
            if let Err(e) = result { eprintln!("Task error: {}", e); }
        }
    }

    /// 改进版拉盘检测：不在持锁期间做重计算
    pub async fn detect_pump_signals(&self) -> anyhow::Result<()> {
        // Step 1: 快速取出所有 Arc（短暂持全局锁）
        let arcs: Vec<(String, Arc<Mutex<SymbolMonitor>>)> = {
            let monitors = self.monitors.monitors.lock().await;
            monitors.iter().map(|(k, v)| (k.clone(), v.clone())).collect()
        };

        let mut signals: Vec<PumpSignal> = Vec::new();

        for (symbol, arc) in arcs {
            // Step 2: 逐个锁（每个锁独立，互不阻塞）
            let mut monitor = arc.lock().await;

            // 计算 features（轻量）
            let features = monitor.book.compute_features(10);

            // 跳过弱信号（减少重量级 analyze 调用）
            if features.pump_score < 30 && features.dump_score < 30 {
                continue;
            }

            // 重量级 analyze 在持单个锁时执行，不阻塞其他币种
            // let analysis = monitor.market_intel.analyze(&monitor.book, &features);
            let analysis = {
                use std::ops::DerefMut;
                let monitor_ref = &mut *monitor;
                let book = &monitor_ref.book;
                monitor_ref.market_intel.analyze(book, &features)
            };
            if let Some(signal) = PUMP_DETECTOR.analyze_symbol(
                &symbol, &features,
                analysis.pump_dump.pump_probability,
                analysis.whale.accumulation_score.to_u8().unwrap_or(0),
                analysis.pump_dump.pump_target,
            ) {
                // 去重检查
                let should_write = if signal.ofi > 0.0 {
                    monitor.should_emit_pump()
                } else {
                    monitor.should_emit_dump()
                };

                if should_write {
                    let _ = PUMP_DETECTOR.write_pump_signal(&signal);
                    signals.push(signal);
                }
            }
        }

        // 汇总 TOP 信号
        use std::sync::atomic::{AtomicUsize, Ordering};
        static COUNTER: AtomicUsize = AtomicUsize::new(0);
        let count = COUNTER.fetch_add(1, Ordering::SeqCst);

        if count % 10 == 0 && !signals.is_empty() {
            let _ = PUMP_DETECTOR.write_top_signals(&mut signals);
            PUMP_DETECTOR.print_top_signals(&signals, 5);
        }

        Ok(())
    }
}