// src/sync_symbols.rs
use anyhow::{Context, Result};
use reqwest::Client;
use serde::Deserialize;
use std::fs::File;
use std::io::{BufWriter, Write};
use std::path::Path;

// 定义我们需要的响应结构体（只解析我们关心的字段）
#[derive(Debug, Deserialize)]
struct ExchangeInfo {
    symbols: Vec<SymbolInfo>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct SymbolInfo {
    symbol: String,
    base_asset: String,
    quote_asset: String,
    status: String,
}

/// 从 Binance 获取所有 USDT 交易对的币种名称，并保存到文件
pub async fn sync_usdt_symbols(client: &Client, output_file: &str) -> Result<()> {
    println!("🔄 开始同步 USDT 交易对信息...");
    let url = "https://api.binance.com/api/v3/exchangeInfo";

    let response = client
        .get(url)
        .send()
        .await
        .context("Failed to send request to exchangeInfo")?;

    if !response.status().is_success() {
        anyhow::bail!("HTTP error: {}", response.status());
    }

    let exchange_info: ExchangeInfo = response
        .json()
        .await
        .context("Failed to parse exchangeInfo JSON")?;

    // 筛选条件：1. 报价资产是 USDT 2. 交易状态为 TRADING（交易中）
    let usdt_symbols: Vec<String> = exchange_info
        .symbols
        .into_iter()
        .filter(|s| s.quote_asset == "USDT" && s.status == "TRADING")
        .map(|s| s.base_asset) // 只取基础币种名称，如 BTC、ETH
        .collect();

    let count = usdt_symbols.len();
    println!("✅ 找到 {} 个正在交易的 USDT 交易对。", count);

    // 将结果写入文件
    let path = Path::new(output_file);
    // 确保父目录存在
    if let Some(parent) = path.parent() {
        tokio::fs::create_dir_all(parent).await?;
    }

    let file = File::create(path).context("Failed to create output file")?;
    let mut writer = BufWriter::new(file);

    for symbol in usdt_symbols {
        writeln!(writer, "{}", symbol)?;
    }

    writer.flush()?;
    println!("💾 币种列表已保存到: {}", output_file);

    // 显示前10个币种作为示例
    println!("\n📋 前10个币种示例:");
    let content = std::fs::read_to_string(output_file)?;
    for (i, line) in content.lines().take(10).enumerate() {
        println!("  {}. {}", i + 1, line);
    }
    if count > 10 {
        println!("  ... 共 {} 个币种", count);
    }

    Ok(())
}

/// 打印帮助信息
pub fn print_help() {
    println!("用法:");
    println!("  cargo run -- [选项]");
    println!();
    println!("选项:");
    println!("  --sync-usdt       同步所有 USDT 交易对到文件");
    println!("  --output <文件>   指定输出文件 (默认: usdt_symbols.txt)");
    println!("  --help            显示此帮助信息");
    println!();
    println!("示例:");
    println!("  cargo run -- --sync-usdt");
    println!("  cargo run -- --sync-usdt --output my_coins.txt");
    println!("  cargo run                    # 正常启动监控");
}
