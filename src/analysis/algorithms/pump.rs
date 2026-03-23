use super::*;

impl PumpDumpPredictor {
    pub fn new() -> Self {
        Self {
            history: VecDeque::with_capacity(100),
            volume_surge_threshold: dec!(2.0),
        }
    }

    pub fn predict(&mut self, book: &OrderBook, features: &OrderBookFeatures) -> PumpDumpPrediction {
        self.update_history(book, features);

        let mut signals = Vec::new();
        let mut pump_prob = 0;
        let mut dump_prob = 0;

        // 检测成交量激增
        if let Some(vol_signal) = self.detect_volume_surge(features) {
            signals.push(vol_signal);
            pump_prob += 15;
            dump_prob += 15;
        }

        // 检测订单流失衡
        if let Some(ofi_signal) = self.detect_orderflow_imbalance(features) {
            signals.push(ofi_signal);
            if features.ofi > Decimal::ZERO {
                pump_prob += 20;
            } else {
                dump_prob += 20;
            }
        }

        // 检测鲸鱼活动
        if let Some(whale_signal) = self.detect_whale_activity(features) {
            signals.push(whale_signal);
            if features.whale_entry {
                pump_prob += 25;
            }
            if features.whale_exit {
                dump_prob += 25;
            }
        }

        // 检测价格加速
        if features.price_change.abs() > dec!(0.5) {
            if features.price_change > Decimal::ZERO {
                pump_prob += 15;
            } else {
                dump_prob += 15;
            }
        }

        // 检测支撑/阻力突破
        if let Some(break_signal) = self.detect_level_break(book, features) {
            if break_signal.signal_type == SignalType::ResistanceBreak {
                pump_prob += 30;
            } else {
                dump_prob += 30;
            }
            signals.push(break_signal);
        }


        // 计算目标位
        let (pump_target, dump_target) = self.calculate_targets(book, features);

        // 计算置信度
        let confidence = ((pump_prob.max(dump_prob) as f64) * 0.8) as u8;

        PumpDumpPrediction {
            pump_probability: pump_prob.min(100),
            dump_probability: dump_prob.min(100),
            pump_target,
            dump_target,
            time_horizon: "5-15分钟".to_string(),
            confidence,
            signals,
        }
    }

    fn update_history(&mut self, book: &OrderBook, features: &OrderBookFeatures) {
        let (best_bid, best_ask) = book.best_bid_ask().unwrap_or((Decimal::ZERO, Decimal::ZERO));
        let mid_price = (best_bid + best_ask) / dec!(2);

        let snapshot = PriceVolumeSnapshot {
            timestamp: Local::now(),
            price: mid_price,
            volume: features.bid_volume_depth + features.ask_volume_depth,
        };

        self.history.push_back(snapshot);
        if self.history.len() > 100 {
            self.history.pop_front();
        }
    }

    fn detect_volume_surge(&self, features: &OrderBookFeatures) -> Option<PumpDumpSignal> {
        if self.history.len() < 10 {
            return None;
        }

        let avg_volume: Decimal = self.history.iter().rev().take(10).map(|h| h.volume).sum();
        let avg_volume = avg_volume / Decimal::from(10);
        let current_volume = features.bid_volume_depth + features.ask_volume_depth;

        if current_volume > avg_volume * self.volume_surge_threshold {
            Some(PumpDumpSignal {
                signal_type: SignalType::VolumeSurge,
                strength: ((current_volume / avg_volume).to_u64().unwrap_or(2) as u8).min(100),
                description: format!("成交量激增 {:.1}倍", current_volume / avg_volume),
            })
        } else {
            None
        }
    }

    fn detect_orderflow_imbalance(&self, features: &OrderBookFeatures) -> Option<PumpDumpSignal> {
        if features.ofi.abs() > dec!(200000) {
            let strength = (features.ofi.abs() / dec!(10000)).to_u64().unwrap_or(50) as u8;
            Some(PumpDumpSignal {
                signal_type: SignalType::OrderFlowImbalance,
                strength: strength.min(100),
                description: format!("订单流{} {:.0}",
                                     if features.ofi > Decimal::ZERO { "偏多" } else { "偏空" },
                                     features.ofi),
            })
        } else {
            None
        }
    }

    fn detect_whale_activity(&self, features: &OrderBookFeatures) -> Option<PumpDumpSignal> {
        if features.whale_entry {
            Some(PumpDumpSignal {
                signal_type: SignalType::WhaleActivity,
                strength: 80,
                description: "鲸鱼进场".to_string(),
            })
        } else if features.whale_exit {
            Some(PumpDumpSignal {
                signal_type: SignalType::WhaleActivity,
                strength: 80,
                description: "鲸鱼离场".to_string(),
            })
        } else {
            None
        }
    }

    fn detect_level_break(&self, book: &OrderBook, features: &OrderBookFeatures) -> Option<PumpDumpSignal> {
        let (best_bid, best_ask) = book.best_bid_ask().unwrap_or((Decimal::ZERO, Decimal::ZERO));

        // 检测阻力突破
        for (price, qty) in book.asks.iter().take(3) {
            if *price > best_ask && qty < &dec!(1000) && features.ofi > dec!(100000) {
                return Some(PumpDumpSignal {
                    signal_type: SignalType::ResistanceBreak,
                    strength: 85,
                    description: format!("阻力突破 {:.6}", price),
                });
            }
        }

        // 检测支撑跌破
        for (Reverse(price), qty) in book.bids.iter().take(3) {
            if *price < best_bid && qty < &dec!(1000) && features.ofi < dec!(-100000) {
                return Some(PumpDumpSignal {
                    signal_type: SignalType::SupportBreak,
                    strength: 85,
                    description: format!("支撑跌破 {:.6}", price),
                });
            }
        }

        None
    }

    fn calculate_targets(&self, book: &OrderBook, _features: &OrderBookFeatures) -> (Decimal, Decimal) {
        let (best_bid, best_ask) = book.best_bid_ask().unwrap_or((Decimal::ZERO, Decimal::ZERO));

        // 泵目标：下一个阻力位
        let pump_target = book.asks.iter()
            .skip(2)
            .next()
            .map(|(p, _)| *p)
            .unwrap_or(best_ask * dec!(1.05));

        // 砸目标：下一个支撑位
        let dump_target = book.bids.iter()
            .skip(2)
            .next()
            .map(|(Reverse(p), _)| *p)
            .unwrap_or(best_bid * dec!(0.95));

        (pump_target, dump_target)
    }
}
