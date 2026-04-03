//! 这些测试主要覆盖 spot 服务最关键的两条链路：
//! 1. 成交后快照是否更新
//! 2. cancel_all 是否正确清空挂单

use std::fs;
use std::path::PathBuf;

use rust_decimal_macros::dec;

use super::{ApiOrderRequest, CancelAllRequest, SpotTradingService};

fn test_log_dir(name: &str) -> PathBuf {
    let mut path = std::env::temp_dir();
    path.push(format!(
        "bb_market_spot_test_{}_{}",
        name,
        std::process::id()
    ));
    let _ = fs::remove_dir_all(&path);
    path
}

#[tokio::test]
async fn submit_order_updates_snapshot() {
    let service = SpotTradingService::new(
        &["BTCUSDT".to_string()],
        std::collections::HashMap::new(),
        test_log_dir("submit"),
    )
    .unwrap();
    service
        .sync_liquidity(
            "BTCUSDT",
            &[(dec!(63990), dec!(1.0))],
            &[(dec!(64000), dec!(1.0))],
        )
        .await
        .unwrap();

    let result = service
        .submit_order(ApiOrderRequest {
            symbol: "BTCUSDT".to_string(),
            side: "buy".to_string(),
            order_type: "limit".to_string(),
            time_in_force: Some("ioc".to_string()),
            price: Some(64000.0),
            quantity: 1.0,
            trigger_price: None,
            trigger_kind: None,
        })
        .await
        .unwrap();

    assert_eq!(result.status, "Filled");
    let snapshot = service.snapshot().await;
    assert!(!snapshot.trade_history.is_empty());
    assert!(snapshot
        .balances
        .iter()
        .any(|balance| balance.asset == "BTC" && balance.available > 10000.0));
}

#[tokio::test]
async fn cancel_all_clears_open_orders() {
    let service = SpotTradingService::new(
        &["BTCUSDT".to_string()],
        std::collections::HashMap::new(),
        test_log_dir("cancel"),
    )
    .unwrap();
    service
        .submit_order(ApiOrderRequest {
            symbol: "BTCUSDT".to_string(),
            side: "buy".to_string(),
            order_type: "limit".to_string(),
            time_in_force: Some("gtc".to_string()),
            price: Some(100.0),
            quantity: 1.0,
            trigger_price: None,
            trigger_kind: None,
        })
        .await
        .unwrap();

    let snapshot = service.snapshot().await;
    assert_eq!(snapshot.open_orders.len(), 1);

    let result = service
        .cancel_all(CancelAllRequest {
            symbol: Some("BTCUSDT".to_string()),
        })
        .await
        .unwrap();
    assert_eq!(result.cancelled, 1);

    let snapshot = service.snapshot().await;
    assert!(snapshot.open_orders.is_empty());
    assert!(!snapshot.order_history.is_empty());
}
