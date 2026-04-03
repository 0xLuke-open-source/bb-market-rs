// src/client/websocket.rs
// 组合订阅：depth@100ms + aggTrade + miniTicker + 全部15个K线周期

use crate::market_data::domain::stream::{CombinedMessage, StreamMsg};
use async_trait::async_trait;
use futures_util::{SinkExt, StreamExt};
use std::time::Duration;
use tokio::sync::mpsc::Sender;
use tokio_tungstenite::{connect_async, tungstenite::protocol::Message};

use crate::market_data::application::ports::MarketStreamClient;

const WS_BASE_URLS: &[&str] = &[
    "wss://stream.binance.com:9443",
    "wss://stream.binance.com:443",
    "wss://data-stream.binance.vision:9443",
    "wss://data-stream.binance.vision:443",
];

pub const KLINE_INTERVALS: &[&str] = &[
    "1m", "3m", "5m", "15m", "30m", "1h", "2h", "4h", "6h", "8h", "12h", "1d", "3d", "1w", "1M",
];

#[derive(Debug, Clone, Default)]
pub struct BinanceMarketStreamClient;

#[async_trait]
impl MarketStreamClient for BinanceMarketStreamClient {
    async fn run_symbol_stream(&self, symbol: &str, tx: Sender<StreamMsg>) -> anyhow::Result<()> {
        run_client(symbol, tx).await
    }
}

pub async fn run_client(symbol: &str, tx: Sender<StreamMsg>) -> anyhow::Result<()> {
    let sym = symbol.to_lowercase();
    let expected_symbol = symbol.to_ascii_uppercase();
    let mut streams = vec![
        format!("{}@depth@100ms", sym),
        format!("{}@aggTrade", sym),
        format!("{}@miniTicker", sym),
    ];
    for interval in KLINE_INTERVALS {
        streams.push(format!("{}@kline_{}", sym, interval));
    }
    let streams_param = streams.join("/");
    let mut last_error = None;
    for &base_url in WS_BASE_URLS {
        let url = format!("{}/stream?streams={}", base_url, streams_param);
        match tokio::time::timeout(Duration::from_secs(10), connect_async(&url)).await {
            Ok(Ok((ws_stream, _))) => {
                return handle_combined(ws_stream, tx, expected_symbol.clone()).await;
            }
            Ok(Err(e)) => {
                last_error = Some(e.to_string());
            }
            Err(_) => {
                last_error = Some("timeout".into());
            }
        }
    }
    Err(anyhow::anyhow!(
        "All endpoints failed for {}: {:?}",
        symbol,
        last_error
    ))
}

async fn handle_combined(
    ws_stream: tokio_tungstenite::WebSocketStream<
        tokio_tungstenite::MaybeTlsStream<tokio::net::TcpStream>,
    >,
    tx: Sender<StreamMsg>,
    expected_symbol: String,
) -> anyhow::Result<()> {
    let (mut write, mut read) = ws_stream.split();
    let mut heartbeat = tokio::time::interval(Duration::from_secs(15));
    loop {
        tokio::select! {
            msg = read.next() => match msg {
                Some(Ok(Message::Text(text))) => {
                    if let Ok(combined) = serde_json::from_str::<CombinedMessage>(&text) {
                        if let Some(msg) = combined.parse() {
                            let msg_symbol = match &msg {
                                StreamMsg::Depth(update) => update.symbol.as_str(),
                                StreamMsg::Trade(trade) => trade.symbol.as_str(),
                                StreamMsg::Ticker(ticker) => ticker.symbol.as_str(),
                                StreamMsg::Kline(kline) => kline.symbol.as_str(),
                            };
                            if !msg_symbol.eq_ignore_ascii_case(&expected_symbol) {
                                continue;
                            }
                            if tx.send(msg).await.is_err() { return Ok(()); }
                        }
                    }
                }
                Some(Ok(Message::Ping(p))) => { write.send(Message::Pong(p)).await.ok(); }
                Some(Ok(Message::Close(_))) | None => return Ok(()),
                Some(Err(e)) => return Err(e.into()),
                _ => {}
            },
            _ = heartbeat.tick() => {
                if write.send(Message::Pong(vec![].into())).await.is_err() { return Ok(()); }
            }
        }
    }
}
