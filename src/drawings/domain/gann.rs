//! Saved price and time divisions for Gann grids, and the fan and arcs of Gann squares.

use crate::model::LineStyle;
use egui::Pos2;
use serde::{Deserialize, Serialize};

/// The corner opposite `start` of a flexible Gann square: towards
/// `end`, as wide and high as the larger of the two distances between them.
pub fn gann_square_corner(start: Pos2, end: Pos2) -> Pos2 {
    let size = (end.x - start.x).abs().max((end.y - start.y).abs());
    let sign = |a: f32, b: f32| if b >= a { 1.0 } else { -1.0 };
    Pos2::new(
        start.x + size * sign(start.x, end.x),
        start.y + size * sign(start.y, end.y),
    )
}

/// The corners the fan lines of a Gann square start from.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
pub enum GannFanDirection {
    /// From the first anchor towards the opposite corner.
    #[default]
    Forward,
    /// From the opposite corner back towards the first anchor.
    Reverse,
    /// From both corners.
    Both,
}

/// Saved presentation of one ray in a Gann fan.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct GannFanAngle {
    /// Whether this ray is painted.
    pub enabled: bool,
    /// Price units per time unit relative to the base angle. Must be positive and finite.
    pub ratio: f32,
    /// Ray color as unmultiplied RGBA.
    pub color: [u8; 4],
    /// Ray width in logical pixels.
    pub line_width: f32,
    /// Ray dash presentation.
    pub line_style: LineStyle,
}

impl GannFanAngle {
    /// Creates an enabled solid fan ray.
    pub fn new(ratio: f32, color: [u8; 4], line_width: f32) -> Self {
        Self {
            enabled: true,
            ratio,
            color,
            line_width,
            line_style: LineStyle::Solid,
        }
    }
}

impl Default for GannFanAngle {
    fn default() -> Self {
        Self::new(1.0, [128, 128, 128, 255], 1.0)
    }
}

/// Divisions of a Gann box or square, and the grid, fan and arcs of a Gann square.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct GannConfig {
    /// Price fractions measured from the first anchor; values outside 0..=1 are ignored.
    pub price_levels: Vec<f32>,
    /// Time fractions measured from the first anchor; values outside 0..=1 are ignored.
    pub time_levels: Vec<f32>,
    /// Whether a Gann square draws its grid of price and time divisions.
    pub show_grid: bool,
    /// Whether a Gann square draws its fan lines.
    pub show_fan: bool,
    /// Price units per time unit of each fan line of a Gann square, where 1 is the diagonal.
    /// Must be positive and finite.
    pub fan_ratios: Vec<f32>,
    /// The corners the fan lines start from.
    pub fan_direction: GannFanDirection,
    /// Whether a Gann square draws its arcs.
    pub show_arcs: bool,
    /// Radius of each arc of a Gann square around its first anchor, as a fraction of its side.
    /// Must be positive and finite; the part outside the square is not drawn.
    pub arc_ratios: Vec<f32>,
    /// Editable rays of a standalone Gann fan.
    pub fan_angles: Vec<GannFanAngle>,
}

impl Default for GannConfig {
    fn default() -> Self {
        // Standard box/square grid on both axes; square fan uses 3×1 … 1×3.
        let levels = vec![0.0, 0.25, 0.382, 0.5, 0.618, 0.75, 1.0];
        Self {
            price_levels: levels.clone(),
            time_levels: levels,
            show_grid: true,
            show_fan: true,
            fan_ratios: vec![3.0, 2.0, 1.0, 0.5, 1.0 / 3.0],
            fan_direction: GannFanDirection::default(),
            show_arcs: true,
            arc_ratios: vec![0.25, 0.5, 0.75, 1.0],
            fan_angles: default_fan_angles(),
        }
    }
}

fn default_fan_angles() -> Vec<GannFanAngle> {
    let tokens = &crate::tokens::DESIGN_TOKENS.semantic;
    [
        (8.0, tokens.extended.bearish, 1.0),
        (4.0, tokens.indicators.ma, 1.0),
        (3.0, tokens.extended.favorite_gold, 1.0),
        (2.0, tokens.extended.caution, 1.0),
        (1.0, tokens.extended.purple, 2.0),
        (0.5, tokens.indicators.ema, 1.0),
        (1.0 / 3.0, tokens.drawings.fib_100, 1.0),
        (0.25, tokens.extended.bullish, 1.0),
        (0.125, tokens.extended.bullish, 1.0),
    ]
    .into_iter()
    .map(|(ratio, color, width)| GannFanAngle::new(ratio, color.to_srgba_unmultiplied(), width))
    .collect()
}
