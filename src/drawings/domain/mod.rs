//! Drawing records, chart coordinates and tool-specific settings.
//!
//! Use [`Drawing`] and [`ChartPoint`] when working with the drawing manager directly.
//! For persistence across history or interval changes, use timestamp anchors through
//! [`persistence`](super::persistence) or the chart's drawing APIs.

mod coordinates;
mod drawing;
mod gann;
mod ghost_feed;
mod options;
mod pattern;
mod pitchfork;
mod template;

pub use crate::config::{DrawingInteractionMode, DrawingToolType, ParseToolTypeError};
pub use coordinates::ChartPoint;
pub use drawing::Drawing;
pub use gann::{
    GannConfig, GannFanDirection, GannLevel, gann_fan_label, gann_square_fan_label,
    parse_gann_ratio,
};
pub use ghost_feed::{GhostCandle, GhostFeedConfig, MAX_GHOST_CANDLES};
pub(crate) use options::fib_ratio_color;
pub use options::{
    ArrowStyle, DrawingOptions, FibPreset, FibonacciConfig, FibonacciLevel, FontWeight, HandlePos,
    PositionSize, PositionSizing, RegressionSource, RegressionTrendConfig, TextHorizontalAlign,
    TextVerticalAlign, TimeframeVisibility,
};
pub use pattern::{
    HarmonicRule, PatternConfig, RatioBand, Wave, WaveDegree, WaveLabel, harmonic_rules,
};
pub use pitchfork::{PitchforkConfig, PitchforkLevel};
pub use template::DrawingTemplate;
