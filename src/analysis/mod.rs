pub mod analysis;
pub mod algorithms;

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
