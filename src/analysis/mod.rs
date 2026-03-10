pub mod analysis;
pub mod algorithms;
pub mod multi_monitor;

// 导出模块
pub use analysis::{
    MarketAnalysis,
    MarketRegime,
    KeyIndicator as OtherKeyIndicator,
    IndicatorStatus,
    WhaleIntent,
    Forecast,
    ForecastDirection,
};
