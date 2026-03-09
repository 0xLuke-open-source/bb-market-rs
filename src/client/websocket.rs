use std::time::Duration;
use futures_util::{SinkExt, StreamExt};
use tokio_tungstenite::{connect_async, tungstenite::protocol::Message};
use serde_json::json;
use crate::codec::binance_msg::DepthUpdate;
use tokio::sync::mpsc::Sender;

// Binance WebSocket 基础 URL
const WS_BASE_URLS: &[&str] = &[
    "wss://stream.binance.com:9443",
    "wss://stream.binance.com:443",
    "wss://data-stream.binance.vision:9443",
    "wss://data-stream.binance.vision:443",
];

// 订阅消息 ID
const SUBSCRIBE_ID: u64 = 1;

pub async fn run_client(symbol: &str, tx: Sender<DepthUpdate>) -> anyhow::Result<()> {
    let symbol_lower = symbol.to_lowercase();
    let stream_name = format!("{}@depth@100ms", symbol_lower);

    // 尝试所有可能的 URL
    let mut last_error = None;
    for &base_url in WS_BASE_URLS {
        let url = format!("{}/ws/{}", base_url, stream_name);

        match connect_with_retry(&url).await {
            Ok((mut ws_stream, _)) => {
                println!("✅ WebSocket connected to {}", url);

                if let Err(e) = send_subscribe(&mut ws_stream, &[&stream_name]).await {
                    eprintln!("Failed to send subscription: {}", e);
                }

                return handle_websocket(ws_stream, tx).await;
            }
            Err(e) => {
                eprintln!("❌ Failed to connect to {}: {}", url, e);
                last_error = Some(e);
                continue;
            }
        }
    }

    Err(anyhow::anyhow!("All WebSocket endpoints failed: {:?}", last_error))
}

async fn connect_with_retry(url: &str) -> anyhow::Result<(tokio_tungstenite::WebSocketStream<tokio_tungstenite::MaybeTlsStream<tokio::net::TcpStream>>, tokio_tungstenite::tungstenite::handshake::client::Response)> {
    tokio::time::timeout(
        Duration::from_secs(10),
        connect_async(url)
    ).await
        .map_err(|_| anyhow::anyhow!("Connection timeout"))?
        .map_err(|e| anyhow::anyhow!("Connection error: {}", e))
}

async fn send_subscribe(ws_stream: &mut tokio_tungstenite::WebSocketStream<tokio_tungstenite::MaybeTlsStream<tokio::net::TcpStream>>, streams: &[&str]) -> anyhow::Result<()> {
    let subscribe_msg = json!({
        "method": "SUBSCRIBE",
        "params": streams,
        "id": SUBSCRIBE_ID
    });

    ws_stream.send(Message::Text(subscribe_msg.to_string().into())).await?;
    Ok(())
}

async fn handle_websocket(
    ws_stream: tokio_tungstenite::WebSocketStream<tokio_tungstenite::MaybeTlsStream<tokio::net::TcpStream>>,
    tx: Sender<DepthUpdate>
) -> anyhow::Result<()> {
    let (mut write, mut read) = ws_stream.split();

    // 心跳任务
    let heartbeat_interval = Duration::from_secs(15);
    let mut heartbeat_interval = tokio::time::interval(heartbeat_interval);

    loop {
        tokio::select! {
            msg = read.next() => {
                match msg {
                    Some(Ok(Message::Text(text))) => {
                        // 检查是否是订阅确认
                        if text.contains("\"result\"") && text.contains(&format!("\"id\":{}", SUBSCRIBE_ID)) {
                            println!("✅ Subscription confirmed");
                            continue;
                        }

                        // 解析深度更新
                        match serde_json::from_str::<DepthUpdate>(&text) {
                            Ok(update) => {
                                if tx.send(update).await.is_err() {
                                    println!("Channel closed, exiting");
                                    return Ok(());
                                }
                            }
                            Err(e) => {
                                // 只在调试时打印，正常运行时可以注释掉
                                // eprintln!("Failed to parse depth update: {}", e);
                            }
                        }
                    }
                    Some(Ok(Message::Ping(payload))) => {
                        if let Err(e) = write.send(Message::Pong(payload)).await {
                            eprintln!("Failed to send pong: {}", e);
                            return Err(e.into());
                        }
                    }
                    Some(Ok(Message::Close(frame))) => {
                        println!("WebSocket closed: {:?}", frame);
                        return Ok(());
                    }
                    Some(Err(e)) => {
                        eprintln!("WebSocket read error: {}", e);
                        return Err(e.into());
                    }
                    None => {
                        println!("WebSocket stream ended");
                        return Ok(());
                    }
                    _ => {} // 忽略其他消息类型
                }
            }

            _ = heartbeat_interval.tick() => {
                if let Err(e) = write.send(Message::Pong(Vec::new().into())).await {
                    eprintln!("Failed to send heartbeat pong: {}", e);
                    return Err(e.into());
                }
            }
        }
    }
}