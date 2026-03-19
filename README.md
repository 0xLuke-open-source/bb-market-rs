# NXPCUSDT 市场监控系统

[![Rust](https://img.shields.io/badge/Rust-1.70%2B-orange.svg)](https://www.rust-lang.org)
[![License](https://img.shields.io/badge/license-MIT-blue.svg)](LICENSE)
[![Binance](https://img.shields.io/badge/Binance-API-yellow.svg)](https://binance-docs.github.io/apidocs/)

<div align="center">
  <h3>🚀 高性能数字货币市场微观结构监控系统</h3>
  <p>毫秒级订单簿分析 · 主力意图识别 · 实时异动检测</p>
</div>


---

## 📖 项目简介

这是一个基于 Rust 的高性能数字货币市场微观结构监控系统。系统通过 WebSocket 实时连接 Binance 交易所，对多个交易对进行毫秒级的订单簿分析，检测市场异动、识别主力意图、预测短期价格走势。

**核心定位**：高频/超高频（HFT/Ultra HFT）量化交易基础设施

**设计目标**：为捕捉市场微观结构的瞬时异动而生

---

## ✨ 核心特性

### 🔄 多币种并发监控
- 同时监控多个 USDT 交易对
- 支持从文件批量加载币种列表
- 每个币种独立维护订单簿和分析报告

### 📊 完整的订单簿指标体系（30+指标）

| 类别 | 核心指标 |
|------|----------|
| **基础指标** | OBI（订单簿不平衡）、OFI（订单流不平衡）、价差、微价格 |
| **流动性指标** | 滑点计算（1k/10k）、深度比率、集中度、加权价差 |
| **订单流质量** | 主动单/被动单比例、订单流失衡速度 |
| **价格发现** | 微价格效率、价格影响、市场冲击成本 |
| **波动率指标** | 已实现波动率、隐含波动率、恐慌指数等效 |
| **风险指标** | VaR、CVaR、尾部风险指数、流动性调整VaR |
| **做市商指标** | 库存风险、价差盈利性、逆向选择成本 |

### 🔍 智能异动检测（9大类）

```
🚨 MegaBid/Ask          - 超大订单出现
⚡ RapidCancellation    - 快速撤单模式
📈 PriceSpike          - 价格尖峰
💥 FlashCrash          - 闪崩事件
🌊 LiquidityDrop       - 流动性骤降
🕳️ DepthGap            - 深度缺口
🐋 WhaleWall           - 鲸鱼墙出现
🎭 Spoofing/Layering   - 操纵行为识别
🔮 ComplexPattern      - 复杂模式组合
```

### 🐋 主力行为分析
- **鲸鱼检测**：识别吸筹/出货意图
- **Spoofing识别**：检测欺诈挂单
- **做市商行为分析**：识别不同类型的做市策略
- **Pump/Dump预测**：计算拉升/砸盘概率

### 📈 多周期趋势分析
- **5秒级**：微观加速度（瞬时动量）
- **1分钟级**：短期趋势（订单流）
- **5分钟级**：中期趋势（主力意图）
- **1小时级**：长期参考（宏观方向）

### 📝 实时报告系统
- **市场分析报告**：完整的60+指标分析（TXT格式）
- **订单簿异动日志**：秒级异动记录，包含严重度/置信度
- **拉盘信号监测**：实时检测拉升信号，TOP N排序
- **全局异动汇总**：所有币种的异动统计

---

## 🏗️ 系统架构

```
┌─────────────────────────────────────────────────────────────┐
│                     MultiWebSocketManager                    │
│                     多币种WebSocket管理器                    │
└───────────────────────────────┬─────────────────────────────┘
                                │ 管理N个连接
                                ▼
┌─────────────────────────────────────────────────────────────┐
│                    MultiSymbolMonitor                        │
│                    多币种监控器（HashMap）                   │
└───────────────────────────────┬─────────────────────────────┘
                                │ 每个币种独立
                                ▼
┌─────────────────────────────────────────────────────────────┐
│                     SymbolMonitor                            │
│  ┌──────────────────────────────────────────────────────┐  │
│  │  OrderBook              // 订单簿（BTreeMap）       │  │
│  │  MarketIntelligence     // 市场智能分析            │  │
│  │  OrderBookAnomalyDetector // 异动检测              │  │
│  │  HistoryManager         // 多周期历史数据          │  │
│  └──────────────────────────────────────────────────────┘  │
└─────────────────────────────────────────────────────────────┘
                                │
                                ▼
┌─────────────────────────────────────────────────────────────┐
│                       报告输出系统                           │
│  ├── reports/           // 市场分析报告                    │
│  ├── anomaly/           // 订单簿异动日志                  │
│  ├── PumpDetector/      // 拉盘信号                        │
│  └── global_anomalies.txt // 全局异动汇总                  │
└─────────────────────────────────────────────────────────────┘
```

### 核心模块说明

| 模块 | 功能描述 |
|------|----------|
| `multi_monitor.rs` | 多币种监控器核心，管理所有WebSocket连接和报告生成 |
| `algorithms.rs` | 核心算法库：鲸鱼检测、Spoofing识别、Pump/Dump预测 |
| `orderbook_anomaly.rs` | 订单簿异动检测引擎，9种异动类型识别 |
| `pump_detector.rs` | 独立的拉盘信号检测器，实时输出TOP信号 |
| `l2_book.rs` | 订单簿数据结构，30+指标计算和多周期采样 |
| `websocket.rs` | WebSocket客户端，连接Binance 100ms深度流 |

---

## 🚀 快速开始

### 环境要求
- Rust 1.70+
- 操作系统：Linux/macOS/Windows
- 内存：建议 4GB+
- 网络：稳定连接 Binance API

### 安装步骤

```bash
# 1. 克隆项目
git clone https://github.com/yourusername/nxpc-monitor.git
cd nxpc-monitor

# 2. 编译（release模式优化性能）
cargo build --release

# 3. 同步USDT交易对列表
cargo run -- --sync-usdt --output usdt_symbols.txt

# 4. 启动多币种监控（监控前10个币种）
cargo run --release -- --multi --count 10

# 5. 使用自定义币种列表
cargo run --release -- --multi --symbol-file my_symbols.txt --count 20
```

### 命令行参数

```bash
cargo run -- [选项]

选项：
  --sync-usdt               同步所有 USDT 交易对到文件
  --output <文件>           指定输出文件 (默认: usdt_symbols.txt)
  
  --multi                   启动多币种监控模式
  --count <数量>            监控的币种数量 (默认: 10)
  --symbol-file <文件>      指定要监控的币种列表文件
  
  --help                    显示帮助信息
```

### 目录结构

```
├── reports/               # 市场分析报告
│   ├── btcusdt_20240319_*.txt
│   └── global_anomalies.txt  # 全局异动汇总
├── anomaly/               # 订单簿异动日志
│   ├── btcusdt_*.txt
│   └── ethusdt_*.txt
├── PumpDetector/          # 拉盘信号
│   └── pump_signals.txt
├── usdt_symbols.txt       # USDT交易对列表
└── src/                   # 源代码
```

---

## 📊 输出文件解读

### 1. 市场分析报告 (`reports/XXX_*.txt`)
完整的60+指标分析报告，包含：
```
📌 市场概览
当前价格: 0.299350
市场状态: ⚡ 剧烈波动 (置信度 70%)
主力意图: 🐋 正在吸筹

📊 基础指标
🟢 OBI: 47.15% - 买方主导市场
⚪ OFI: 13868 - 订单流平稳
🟡 最大买单占比: 73.0% - 存在托单

⚠️ 风险指标
🐉 尾部风险指数: 119.1 - 警惕极端行情

💡 操作建议
✅ 强烈看涨，可考虑建仓做多
🎯 目标位: 0.025730
🛑 止损位: 0.025403
```

### 2. 订单簿异动日志 (`anomaly/XXX_*.txt`)
```
时间       | 异动类型     | 严重度 | 置信度 | 价格     | 描述
09:19:24 | PriceSpike   | 234%  | 90%   | 0.285000 | 价格飙升 2450.85bps
09:19:25 | WhaleWall    | 60%   | 85%   | -        | 买单鲸鱼墙: 3个大单, 总 8989 USDT
```

### 3. 拉盘信号 (`pump_signals.txt`)
```
时间       | 币种      | 强度 | OFI     | OBI%   | 价格    | 信号原因
09:31:31 | SAHARAUSDT | 65%🚀| 210800 | +50.1% | 0.0258 | OFI=210800 OBI=50.1% SLOPE↑ BIG37%
09:33:21 | SAHARAUSDT | 60%🚀| 135139 | +48.7% | 0.0262 | OFI=135139 OBI=48.7% SLOPE↑ big22%
```

### 4. 全局异动汇总 (`global_anomalies.txt`)
```
📊 全局异动汇总 - 2026-03-19 09:31:21
NXPCUSDT: 总异动 234 | 最近1分钟 233 | 严重度 50.1 | 最高 234
ENJUSDT : 总异动 1089 | 最近1分钟 1088 | 严重度 26.2 | 最高 142
```

---

## 📈 指标详解

### 核心指标说明

| 指标 | 全称 | 计算公式 | 解读 |
|------|------|----------|------|
| **OBI** | Order Book Imbalance | (买单总量 - 卖单总量) / (买单总量 + 卖单总量) × 100 | >20%:买方主导；<-20%:卖方主导 |
| **OFI** | Order Flow Imbalance | 主动买单量 - 主动卖单量 | >50000:买盘强劲；<-50000:卖盘强劲 |
| **价差** | Spread | 卖一价 - 买一价 | <10bps:流动性好；>50bps:流动性差 |
| **微价格** | Microprice | (买一价×卖一量 + 卖一价×买一量) / (买一量+卖一量) | 优于中间价的价格发现指标 |
| **VaR** | Value at Risk | 历史模拟法计算 | 95%置信度下的最大可能亏损 |

### 信号标志

| 标志 | 含义 | 触发条件 |
|------|------|----------|
| 🚀 `pump_signal` | 拉盘信号 | OFI>50000 + OBI>30% + 斜率>100万 |
| 📉 `dump_signal` | 砸盘信号 | OFI<-50000 + OBI<-30% + 卖单斜率<-100万 |
| 🐋 `whale_entry` | 鲸鱼进场 | 大单占比>40% + 成交量变化>20% |
| 🐋 `whale_exit` | 鲸鱼离场 | 卖单大单>40% + 卖量变化>20% |
| 🍽️ `bid_eating` | 主动吃筹 | 买单量变化>30% + 价格上升 |
| 💥 `ask_eating` | 主动砸盘 | 卖单量变化>30% + 价格下降 |

---

## ⚙️ 高级配置

### 修改报告间隔

在 `main.rs` 中调整：
```rust
// 参数为秒数，例如20秒生成一次报告
let monitor = Arc::new(MultiSymbolMonitor::new(20));
```

### 调整异动检测阈值

在 `orderbook_anomaly.rs` 中修改 `AnomalyConfig`：
```rust
pub struct AnomalyConfig {
    pub mega_bid_threshold: Decimal,     // 超大买单阈值（默认20%）
    pub price_spike_bps: Decimal,        // 价格尖峰阈值（默认50bps）
    pub rapid_cancel_ms: u64,            // 快速撤销窗口（默认100ms）
    pub liquidity_drop_threshold: Decimal, // 流动性下降阈值（默认30%）
    // ...
}
```

### 拉盘信号灵敏度

在 `pump_detector.rs` 中调整：
```rust
// 创建检测器时设置最小强度（只记录强度>=30的信号）
PUMP_DETECTOR.with_min_strength(30)
```

---

## 🎯 实战案例

### 案例1：NXPCUSDT 主力拉升识别

```rust
// 系统检测到的特征
OBI: 47.15%          // 买方主导市场
OFI: 13868           // 订单流平稳
最大买单占比: 73.0%   // 存在明显托单
信息不对称度: 79.4%   // 高度操纵特征
尾部风险: 119.1      // 极端行情警告
```

**系统输出**：`⚠️ 尾部风险指数 119.1 - 警惕极端行情`

### 案例2：SAHARAUSDT 拉盘信号

```
09:31:31 | SAHARAUSDT | 65%🚀 | 210800 | +50.1% | 0.0258 | OFI=210800 OBI=50.1% SLOPE↑ BIG37%
09:33:21 | SAHARAUSDT | 60%🚀 | 135139 | +48.7% | 0.0262 | OFI=135139 OBI=48.7% SLOPE↑ big22%
```

**实际走势**：系统在拉升前10-30秒连续发出强烈信号，随后价格从0.0258拉升至0.0262。

### 案例3：多币种异动对比

```
09:27:36 | ENJUSDT  | 峰值异动 1277次/分钟
09:32:31 | SAHARAUSDT | 峰值异动 1129次/分钟
```

系统可同时监控多个币种，识别出当前最活跃的交易对。

---

## ⚠️ 注意事项

1. **资源消耗**：监控50个币种约占用 2-4GB 内存
2. **网络要求**：需要稳定连接 Binance WebSocket（建议使用专线）
3. **数据连续性**：系统会检测数据缺口，自动重连
4. **延迟控制**：Rust异步架构确保毫秒级处理（平均<10ms）
5. **磁盘占用**：持续运行每天约产生 500MB-1GB 日志文件

---

## 🤝 贡献指南

欢迎提交 PR 和 Issue！

### 开发计划
- [ ] 添加更多交易所支持（OKX、Bybit）
- [ ] 实现策略回测模块
- [ ] 添加 Web 仪表盘（实时监控界面）
- [ ] 支持自定义指标公式
- [ ] 集成 Telegram/Discord 报警
- [ ] 添加数据库存储（InfluxDB/TimescaleDB）

### 如何贡献

1. Fork 本仓库
2. 创建特性分支 (`git checkout -b feature/AmazingFeature`)
3. 提交改动 (`git commit -m 'Add some AmazingFeature'`)
4. 推送到分支 (`git push origin feature/AmazingFeature`)
5. 开启 Pull Request

---

## 📄 许可证

MIT License © 2024 [Your Name]

---

## 🙏 致谢

- [Binance API](https://binance-docs.github.io/apidocs/) - 提供优质的数据源
- [Rust 社区](https://www.rust-lang.org) - 优秀的系统编程语言
- [Tokio](https://tokio.rs) - 高性能异步运行时
- 所有贡献者和使用者

---

## 📮 联系方式

- 作者：[Your Name]
- 邮箱：[your.email@example.com]
- GitHub：[@yourusername](https://github.com/yourusername)

---

<div align="center">
  <p>⭐ 如果这个项目对你有帮助，欢迎 Star ⭐</p>
  <p>Built with ❤️ and 🦀</p>
</div>
```
