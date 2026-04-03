use std::collections::{HashMap, HashSet};

#[derive(Debug, Clone, Default)]
pub struct SymbolVisibility {
    pub public_symbols: HashSet<String>,
    pub member_symbols: HashSet<String>,
    pub plan_symbols: HashMap<String, HashSet<String>>,
}

#[derive(Debug, Clone, Copy, Default)]
pub struct SymbolPrecision {
    pub price_precision: u32,
    pub quantity_precision: u32,
}

#[derive(Debug, Clone)]
pub enum VisibilityTier {
    Public,
    Member,
    Plan(String),
}
