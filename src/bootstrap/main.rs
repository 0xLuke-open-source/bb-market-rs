use crate::execution::application::spot::SpotTradingService;
use crate::execution::domain::matching::run_spot_engine_demo;
use crate::identity::application::AuthApplicationService;
use crate::identity::infrastructure::postgres::PostgresAuthRepository;
use crate::identity::interfaces::AuthService;
use crate::instrument_catalog::application as catalog_app;
use crate::instrument_catalog::infrastructure::PostgresSymbolRegistryAdapter;
use crate::market_data::application::ports::{
    BigTradeReader, BigTradeSink, MarketStreamClient, OrderBookTickSink, RecentTradeReader,
    RecentTradeSink,
};
use crate::market_data::application::runtime::{MultiSymbolMonitor, MultiWebSocketManager};
use crate::market_data::domain::order_book::OrderBook;
use crate::market_data::domain::stream::{Snapshot, StreamMsg};
use crate::market_data::infrastructure::binance::{self, BinanceMarketStreamClient};
use crate::market_data::infrastructure::persistence::big_trade::{
    BigTradePersistenceService, BigTradeQueryService,
};
use crate::market_data::infrastructure::persistence::orderbook_tick::OrderBookTickPersistenceService;
use crate::market_data::infrastructure::persistence::recent_trade::{
    RecentTradePersistenceService, RecentTradeQueryService,
};
use crate::shared::config::AppConfig;
use crate::shared::postgres::PgPool;
use crate::signal_intelligence::application::signal_resolver::{
    spawn_signal_resolver, SignalResolver,
};
use crate::signal_intelligence::domain::algorithms::MarketIntelligence;
use crate::signal_intelligence::domain::market_analysis::MarketAnalysis;
use crate::signal_intelligence::infrastructure::persistence::adaptive_threshold::AdaptiveThresholdPersistenceService;
use crate::signal_intelligence::infrastructure::persistence::anomaly_event::AnomalyEventPersistenceService;
use crate::terminal::application::bridge::run_bridge;
use crate::terminal::application::cache::load_dashboard_cache;
use crate::terminal::application::ports::{
    OrderBookSnapshotSink, SymbolPanelSnapshotReader, SymbolPanelSnapshotStore,
};
use crate::terminal::application::price_source::DashboardPriceSource;
use crate::terminal::application::projection::new_dashboard_state;
use crate::terminal::application::query::TerminalQueryService;
use crate::terminal::infrastructure::persistence::order_book_snapshot::OrderBookPersistenceService;
use crate::terminal::infrastructure::persistence::panel_snapshot::{
    SymbolPanelPersistenceService, SymbolPanelQueryService,
};
use crate::terminal::interfaces::http::run_server;
use clap::Parser;
use const_format::concatcp;
use reqwest::Client;
use std::io::Write;
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::mpsc;
use tokio::time::Instant;

const COIN: &str = "ASTR";
const SYMBOL: &str = concatcp!(COIN, "USDT");
const DASHBOARD_CACHE_PATH: &str = "logs/dashboard-cache.json";
const DASHBOARD_CACHE_MAX_AGE_SECS: u64 = 30 * 60;

#[derive(Parser, Debug)]
#[clap(
    author,
    version,
    about = "BB-Market 实时监控系统",
    long_about = "Binance 多数据流实时分析（订单簿+成交+K线+Ticker）+ Web Dashboard"
)]
struct Args {
    #[clap(long, action)]
    sync_usdt: bool,
    #[clap(long)]
    sync_symbol_file: Option<String>,
    #[clap(short, long, action)]
    multi: bool,
    #[clap(short, long, default_value_t = 10)]
    count: usize,
    #[clap(long, value_delimiter = ',')]
    enable_symbol: Vec<String>,
    #[clap(long, value_delimiter = ',')]
    disable_symbol: Vec<String>,
    #[clap(long, value_delimiter = ',')]
    show_symbol: Vec<String>,
    #[clap(long, value_delimiter = ',')]
    hide_symbol: Vec<String>,
    #[clap(long, action)]
    show_all_symbols: bool,
    #[clap(long, action)]
    hide_all_symbols: bool,
    #[clap(long, action)]
    web: bool,
    #[clap(long, default_value_t = 9527)]
    port: u16,
    #[clap(long, action)]
    spot_match_demo: bool,
}

pub async fn run() -> anyhow::Result<()> {
    let args = Args::parse();

    rustls::crypto::ring::default_provider()
        .install_default()
        .expect("Failed to install rustls crypto provider");
    tracing_subscriber::fmt::init();

    if args.spot_match_demo {
        return run_spot_engine_demo();
    }

    let needs_db = args.sync_usdt
        || args.sync_symbol_file.is_some()
        || args.multi
        || !args.enable_symbol.is_empty()
        || !args.disable_symbol.is_empty()
        || !args.show_symbol.is_empty()
        || !args.hide_symbol.is_empty()
        || args.show_all_symbols
        || args.hide_all_symbols;
    let db_pool = if needs_db {
        let config = AppConfig::load()?;
        Some(Arc::new(PgPool::new(config.database.clone()).await?))
    } else {
        None
    };
    let symbol_registry_admin = if let Some(pool) = db_pool.as_ref() {
        let adapter = Arc::new(PostgresSymbolRegistryAdapter::new(pool.clone()).await?);
        Some(catalog_app::SymbolRegistryAdminService::new(adapter))
    } else {
        None
    };

    if args.sync_usdt {
        let client = Client::builder().timeout(Duration::from_secs(15)).build()?;
        symbol_registry_admin
            .as_ref()
            .expect("db pool required")
            .sync_usdt_symbols(&client)
            .await?;
        return Ok(());
    }

    if let Some(file_path) = &args.sync_symbol_file {
        symbol_registry_admin
            .as_ref()
            .expect("db pool required")
            .sync_symbols_from_file(file_path)
            .await?;
        return Ok(());
    }

    if !args.enable_symbol.is_empty() {
        let affected = symbol_registry_admin
            .as_ref()
            .expect("db pool required")
            .set_symbols_enabled(&args.enable_symbol, true)
            .await?;
        println!("✅ 已启用 {} 个币种。", affected);
    }

    if !args.disable_symbol.is_empty() {
        let affected = symbol_registry_admin
            .as_ref()
            .expect("db pool required")
            .set_symbols_enabled(&args.disable_symbol, false)
            .await?;
        println!("✅ 已禁用 {} 个币种。", affected);
    }

    if args.show_all_symbols {
        let affected = symbol_registry_admin
            .as_ref()
            .expect("db pool required")
            .set_all_symbols_visibility(true)
            .await?;
        println!("✅ 已统一显示 {} 个币种。", affected);
    }

    if args.hide_all_symbols {
        let affected = symbol_registry_admin
            .as_ref()
            .expect("db pool required")
            .set_all_symbols_visibility(false)
            .await?;
        println!("✅ 已统一隐藏 {} 个币种。", affected);
    }

    if !args.show_symbol.is_empty() {
        let affected = symbol_registry_admin
            .as_ref()
            .expect("db pool required")
            .set_symbols_visibility(&args.show_symbol, true)
            .await?;
        println!("✅ 已统一显示 {} 个指定币种。", affected);
    }

    if !args.hide_symbol.is_empty() {
        let affected = symbol_registry_admin
            .as_ref()
            .expect("db pool required")
            .set_symbols_visibility(&args.hide_symbol, false)
            .await?;
        println!("✅ 已统一隐藏 {} 个指定币种。", affected);
    }

    if args.multi {
        start_multi_monitoring(args, db_pool.expect("db pool required")).await
    } else {
        let client = Client::builder().timeout(Duration::from_secs(15)).build()?;
        start_monitoring(client).await
    }
}

async fn start_multi_monitoring(args: Args, pg_pool: Arc<PgPool>) -> anyhow::Result<()> {
    let trade_persistence = Arc::new(RecentTradePersistenceService::new(pg_pool.clone()).await?);
    let big_trade_persistence = Arc::new(BigTradePersistenceService::new(pg_pool.clone()).await?);
    let orderbook_tick_persistence =
        Arc::new(OrderBookTickPersistenceService::new(pg_pool.clone()).await?);
    let anomaly_persistence = AnomalyEventPersistenceService::new(pg_pool.clone()).await?;
    let _ = anomaly_persistence;

    let mut monitor_inner = MultiSymbolMonitor::new(
        20,
        Some(trade_persistence.clone() as Arc<dyn RecentTradeSink>),
        Some(big_trade_persistence.clone() as Arc<dyn BigTradeSink>),
    );
    monitor_inner
        .set_orderbook_tick_sink(orderbook_tick_persistence.clone() as Arc<dyn OrderBookTickSink>);

    let monitor = Arc::new(monitor_inner);
    let symbol_registry_adapter =
        Arc::new(PostgresSymbolRegistryAdapter::new(pg_pool.clone()).await?);
    let symbol_registry = catalog_app::SymbolRegistryService::new(symbol_registry_adapter);
    let port = args.port;
    let web_on = args.web;

    let symbols = symbol_registry.load_enabled_symbols(args.count).await?;

    println!("📋 监控 {} 个币种:", symbols.len());
    for (i, s) in symbols.iter().enumerate() {
        println!("  {}. {}", i + 1, s);
    }

    monitor.init_monitors(symbols.clone()).await;
    let stream_client = Arc::new(BinanceMarketStreamClient) as Arc<dyn MarketStreamClient>;
    let mut manager = MultiWebSocketManager::new(monitor.clone(), stream_client);

    let pump_monitor = monitor.clone();
    tokio::spawn(async move {
        let mut tick = tokio::time::interval(Duration::from_secs(10));
        loop {
            tick.tick().await;
            pump_monitor.detect_pump_signals().await.ok();
        }
    });

    let cleanup_pool = pg_pool.clone();
    tokio::spawn(async move {
        let mut tick = tokio::time::interval(Duration::from_secs(24 * 60 * 60));
        tick.tick().await;
        loop {
            tick.tick().await;
            match cleanup_pool.acquire().await {
                Ok(client) => {
                    let sqls = [
                        ("orderbook_tick",    "delete from market.orderbook_tick    where created_at < now() - interval '14 days'"),
                        ("anomaly_event",     "delete from market.anomaly_event     where detected_at < now() - interval '14 days'"),
                        ("adaptive_threshold","delete from market.adaptive_threshold where window_end_at < now() - interval '14 days'"),
                    ];
                    for (name, sql) in &sqls {
                        match client.client().execute(*sql, &[]).await {
                            Ok(n) => {
                                if n > 0 {
                                    eprintln!("[cleanup] {} 清理 {} 条", name, n);
                                }
                            }
                            Err(e) => eprintln!("[cleanup] {} 清理失败: {}", name, e),
                        }
                    }
                }
                Err(e) => eprintln!("[cleanup] 获取数据库连接失败: {}", e),
            }
        }
    });

    if web_on {
        let dash_state = new_dashboard_state();
        if load_dashboard_cache(
            &dash_state,
            DASHBOARD_CACHE_PATH,
            Duration::from_secs(DASHBOARD_CACHE_MAX_AGE_SECS),
        )
        .await
        .unwrap_or(false)
        {
            println!("📦 已加载 30 分钟内的 Dashboard 缓存快照");
        }
        let spot_precisions = symbol_registry.symbol_precisions(&symbols).await;
        let spot_service = SpotTradingService::new(&symbols, spot_precisions, "logs/spot")?;
        let auth_repository = Arc::new(PostgresAuthRepository::new(pg_pool.clone()));
        let auth_application = AuthApplicationService::new(auth_repository);
        auth_application.ensure_ready().await?;
        let auth_service = AuthService::new(auth_application);
        let orderbook_persistence =
            Arc::new(OrderBookPersistenceService::new(pg_pool.clone()).await?);
        let panel_persistence =
            Arc::new(SymbolPanelPersistenceService::new(pg_pool.clone()).await?);
        let panel_query = Arc::new(SymbolPanelQueryService::new(pg_pool.clone()).await?);
        let recent_trade_query = Arc::new(RecentTradeQueryService::new(pg_pool.clone()).await?);
        let big_trade_query = Arc::new(BigTradeQueryService::new(pg_pool.clone()).await?);
        let terminal_queries = TerminalQueryService::new(
            panel_persistence.clone() as Arc<dyn SymbolPanelSnapshotStore>,
            panel_query.clone() as Arc<dyn SymbolPanelSnapshotReader>,
            recent_trade_query.clone() as Arc<dyn RecentTradeReader>,
            big_trade_query.clone() as Arc<dyn BigTradeReader>,
        );

        let adaptive_persistence =
            AdaptiveThresholdPersistenceService::new(pg_pool.clone()).await?;
        let adaptive_monitor = monitor.clone();
        let adaptive_svc = adaptive_persistence.clone();
        tokio::spawn(async move {
            let mut tick = tokio::time::interval(Duration::from_secs(5 * 60));
            loop {
                tick.tick().await;
                let monitors = adaptive_monitor.monitors.lock().await;
                for (symbol, arc) in monitors.iter() {
                    let guard = arc.lock().await;
                    let snapshot = guard.threshold.snapshot();
                    adaptive_svc.submit(symbol, snapshot);
                }
            }
        });

        let price_source = Arc::new(DashboardPriceSource::new(dash_state.clone()));
        let signal_resolver = SignalResolver::new(pg_pool.clone(), price_source);
        spawn_signal_resolver(signal_resolver);

        let bridge_monitor = monitor.clone();
        let bridge_registry = symbol_registry.clone();
        let bridge_dash = dash_state.clone();
        let bridge_spot = spot_service.clone();
        let bridge_orderbook_persistence =
            orderbook_persistence.clone() as Arc<dyn OrderBookSnapshotSink>;
        let bridge_panel_persistence =
            panel_persistence.clone() as Arc<dyn SymbolPanelSnapshotStore>;
        tokio::spawn(async move {
            run_bridge(
                bridge_monitor,
                bridge_registry,
                bridge_dash,
                bridge_spot,
                bridge_orderbook_persistence,
                bridge_panel_persistence,
                500,
            )
            .await;
        });

        let server_dash = dash_state.clone();
        let server_monitor = monitor.clone();
        let server_registry = symbol_registry.clone();
        let server_queries = terminal_queries.clone();
        let server_spot = spot_service.clone();
        let server_auth = auth_service.clone();
        tokio::spawn(async move {
            if let Err(e) = run_server(
                server_monitor,
                server_registry,
                server_queries,
                server_dash,
                server_spot,
                server_auth,
                port,
            )
            .await
            {
                eprintln!("❌ Web 服务器错误: {}", e);
            }
        });

        println!("\n╔══════════════════════════════════════════════╗");
        println!("║  🌐 Dashboard: http://127.0.0.1:{}         ║", port);
        println!("║  数据源：订单簿 + 成交流 + K线 + 24h Ticker   ║");
        println!("╚══════════════════════════════════════════════╝\n");
    }

    manager.start_all(symbols).await;
    manager.wait().await;
    Ok(())
}

async fn start_monitoring(client: Client) -> anyhow::Result<()> {
    let (tx, mut rx) = mpsc::channel::<StreamMsg>(2000);
    let mut book = OrderBook::new(SYMBOL);
    let mut market_intel = MarketIntelligence::new();
    let max_connection_duration = Duration::from_secs(23 * 60 * 60);

    let sym_task = SYMBOL.to_string();
    let tx_clone = tx.clone();
    tokio::spawn(async move {
        loop {
            let t0 = Instant::now();
            match binance::run_client(&sym_task, tx_clone.clone()).await {
                Ok(()) => println!("WebSocket exited normally"),
                Err(e) => eprintln!("WebSocket Error: {}", e),
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

    let mut last_print = Instant::now();
    let mut last_report = Instant::now();
    let print_interval = Duration::from_millis(100);
    let report_interval = Duration::from_secs(20);

    while let Some(msg) = rx.recv().await {
        match msg {
            StreamMsg::Depth(update) => {
                if let Err(e) = book.apply_incremental_update(update) {
                    eprintln!("Depth Update Error: {}", e);
                    break;
                }
                if last_print.elapsed() >= print_interval {
                    book.compute_features(10);
                    std::io::stdout().flush()?;
                    last_print = Instant::now();
                }
                if last_report.elapsed() >= report_interval {
                    if book.best_bid_ask().is_some() {
                        let features = book.compute_features(10);
                        book.auto_sample(&features);
                        let analysis = MarketAnalysis::new(&book, &features);
                        let comp = market_intel.analyze(&book, &features);
                        analysis.display();
                        market_intel.display_summary(&comp);
                    }
                    last_report = Instant::now();
                }
            }
            StreamMsg::Trade(trade) => {
                let qty = trade.qty.parse::<f64>().unwrap_or(0.0);
                if qty > 100000.0 {
                    let dir = if trade.is_taker_buy() {
                        "🟢 主动买"
                    } else {
                        "🔴 主动卖"
                    };
                    println!("[{}] {} {} @ {}", trade.symbol, dir, trade.qty, trade.price);
                }
            }
            StreamMsg::Ticker(ticker) => {
                if last_report.elapsed() >= report_interval {
                    println!(
                        "[24h] {} 涨跌:{:.2}% 高:{} 低:{} 量:{}",
                        ticker.symbol,
                        ticker.change_pct(),
                        ticker.high,
                        ticker.low,
                        ticker.volume
                    );
                }
            }
            StreamMsg::Kline(kline) => {
                if kline.kline.is_closed {
                    let k = &kline.kline;
                    println!(
                        "[Kline {}] {} O:{} H:{} L:{} C:{} V:{}",
                        k.interval, kline.symbol, k.open, k.high, k.low, k.close, k.volume
                    );
                }
            }
        }
    }

    Ok(())
}

async fn fetch_snapshot_with_retry(
    client: &Client,
    symbol: &str,
    retries: usize,
) -> anyhow::Result<Snapshot> {
    let sym = symbol.to_uppercase();
    let urls = [
        format!(
            "https://api.binance.com/api/v3/depth?symbol={}&limit=1000",
            sym
        ),
        format!(
            "https://api1.binance.com/api/v3/depth?symbol={}&limit=1000",
            sym
        ),
        format!(
            "https://api2.binance.com/api/v3/depth?symbol={}&limit=1000",
            sym
        ),
        format!(
            "https://api3.binance.com/api/v3/depth?symbol={}&limit=1000",
            sym
        ),
        format!(
            "https://api4.binance.com/api/v3/depth?symbol={}&limit=1000",
            sym
        ),
    ];
    for i in 0..retries {
        for url in &urls {
            match client.get(url).send().await {
                Ok(resp) if resp.status().is_success() => {
                    return Ok(resp.json::<Snapshot>().await?);
                }
                Ok(resp) => {
                    eprintln!("Snapshot fetch failed: {} {}", url, resp.status());
                }
                Err(e) => {
                    eprintln!("Snapshot fetch error: {}: {}", url, e);
                }
            }
        }
        if i + 1 < retries {
            tokio::time::sleep(Duration::from_secs(2)).await;
        }
    }
    Err(anyhow::anyhow!(
        "Failed to fetch snapshot after {} retries",
        retries
    ))
}
