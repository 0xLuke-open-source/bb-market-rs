use super::*;

impl MarketMakerDetector {
    pub fn new() -> Self {
        Self {
            quote_history: VecDeque::with_capacity(1000),
        }
    }

    pub fn detect(&mut self, book: &OrderBook, features: &OrderBookFeatures) -> MarketMakerBehavior {
        self.update_history(book);

        let mm_type = self.determine_mm_type(features);
        let strategy = self.determine_strategy(features);
        let inventory_bias = self.calculate_inventory_bias(features);
        let spread_policy = self.determine_spread_policy(features);
        let quote_frequency = self.calculate_quote_frequency();

        MarketMakerBehavior {
            is_active: quote_frequency > 0.1,
            mm_type,
            strategy,
            inventory_bias,
            spread_policy,
            quote_frequency,
        }
    }

    fn update_history(&mut self, book: &OrderBook) {
        if let Some((bid, ask)) = book.best_bid_ask() {
            let snapshot = QuoteSnapshot {
                timestamp: Local::now(),
                bid_price: bid,
                ask_price: ask,
            };

            self.quote_history.push_back(snapshot);
            if self.quote_history.len() > 1000 {
                self.quote_history.pop_front();
            }
        }
    }

    fn determine_mm_type(&self, features: &OrderBookFeatures) -> MarketMakerType {
        if features.slope_bid.abs() < dec!(100000) && features.slope_ask.abs() < dec!(100000) {
            MarketMakerType::HighFrequencyMM
        } else if features.max_bid_ratio > dec!(20) || features.max_ask_ratio > dec!(20) {
            MarketMakerType::InstitutionalMM
        } else {
            MarketMakerType::RetailMM
        }
    }

    fn determine_strategy(&self, features: &OrderBookFeatures) -> MMStrategy {
        let spread = features.spread_bps;
        let inventory_bias = self.calculate_inventory_bias(features);

        if spread < dec!(10) && inventory_bias.abs() < dec!(20) {
            MMStrategy::SpreadCapture
        } else if inventory_bias.abs() > dec!(50) {
            MMStrategy::InventoryMgmt
        } else if features.price_change.abs() < dec!(0.1) && features.obi.abs() < dec!(10) {
            MMStrategy::PriceStabilization
        } else {
            MMStrategy::Directional
        }
    }

    fn calculate_inventory_bias(&self, features: &OrderBookFeatures) -> Decimal {
        let bid_depth = features.bid_volume_depth;
        let ask_depth = features.ask_volume_depth;
        let total = bid_depth + ask_depth;

        if total > Decimal::ZERO {
            (bid_depth - ask_depth) / total * dec!(100)
        } else {
            Decimal::ZERO
        }
    }

    fn determine_spread_policy(&self, features: &OrderBookFeatures) -> SpreadPolicy {
        let bps = features.spread_bps;

        if bps < dec!(10) {
            SpreadPolicy::Tight
        } else if bps < dec!(30) {
            SpreadPolicy::Normal
        } else {
            SpreadPolicy::Wide
        }
    }

    fn calculate_quote_frequency(&self) -> f64 {
        if self.quote_history.len() < 2 {
            return 0.0;
        }

        let time_span = (self.quote_history.back().unwrap().timestamp -
            self.quote_history.front().unwrap().timestamp).num_seconds();

        if time_span > 0 {
            self.quote_history.len() as f64 / time_span as f64
        } else {
            0.0
        }
    }
}
