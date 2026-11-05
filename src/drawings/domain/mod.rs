//! Drawing records, chart coordinates and tool-specific settings.
//!
//! Use [`Drawing`] and [`ChartPoint`] when working with the drawing manager directly.
//! For persistence across history or interval changes, use timestamp anchors through
//! [`persistence`](super::persistence) or the chart's drawing APIs.

mod bars_pattern;
mod coordinates;
mod drawing;
mod gann;
mod geometry;
mod ghost_feed;
mod options;
mod pattern;
mod pitchfork;
mod position;
mod price_action;
mod template;

pub use crate::config::{DrawingInteractionMode, DrawingToolType, ParseToolTypeError};
pub use bars_pattern::{BarsPatternConfig, BarsPatternMode, MAX_PATTERN_BARS};
pub use coordinates::ChartPoint;
pub use drawing::Drawing;
pub use gann::{
    GannConfig, GannFanDirection, GannLevel, gann_fan_label, gann_square_fan_label,
    parse_gann_ratio,
};
pub use geometry::{
    DEDEKIND_CURVATURES, DedekindGeodesic, GeometryConfig, MachCircle, MachFigure,
    dedekind_geodesics, is_dedekind_center, mach_figure,
};
pub use ghost_feed::{GhostCandle, GhostFeedConfig, MAX_GHOST_CANDLES};
pub(crate) use options::fib_ratio_color;
pub use options::{
    ArrowStyle, DEFAULT_ACCOUNT_BALANCE, DrawingOptions, FibPreset, FibonacciConfig,
    FibonacciLevel, FontWeight, ForecastConfig, HandlePos, MeasureConfig, PositionDisplay,
    PositionSize, PositionSizing, RegressionSource, RegressionTrendConfig, TextHorizontalAlign,
    TextVerticalAlign, TimeframeVisibility, VwapConfig,
};
pub use pattern::{
    HarmonicRule, PatternConfig, RatioBand, Wave, WaveDegree, WaveLabel, harmonic_rules,
};
pub use pitchfork::{PitchforkConfig, PitchforkLevel};
pub use position::POSITION_REWARD_RATIO;
pub use price_action::{
    BlockState, GapState, LiquidityState, PriceActionConfig, ZoneEnd, ZoneFormation, block_state,
    gap_state, liquidity_state,
};
pub use template::DrawingTemplate;
