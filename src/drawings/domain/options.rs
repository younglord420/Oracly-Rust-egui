//! Drawing options and configuration types.
//!
//! Contains styling enums ([`LineStyle`](crate::model::LineStyle), [`ArrowStyle`], [`FontWeight`]),
//! interaction configuration ([`DrawingOptions`], [`HandlePos`]),
//! per-timeframe visibility ([`TimeframeVisibility`]), Fibonacci level
//! configuration ([`FibonacciConfig`], [`FibonacciLevel`]), and the settings of
//! the volume profile, regression trend and position drawings
//! ([`VolumeProfileConfig`](crate::data::VolumeProfileConfig), [`RegressionTrendConfig`], [`PositionSize`], [`PositionSizing`]).

use crate::model::LineStyle;
use serde::{Deserialize, Serialize};
use std::collections::HashSet;

/// Identifies a selection handle position on a drawing.
///
/// Different tool types expose different handle layouts. Two-point tools use
/// `Start`, `End`, and `Middle`; rectangles use the four corner handles;
/// multi-point tools use indexed `Point(i)` handles.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum HandlePos {
    /// First anchor point of the drawing.
    Start,
    /// Last anchor point of the drawing.
    End,
    /// Centroid/midpoint handle -- dragging this moves the entire drawing.
    Middle,
    /// Top-left corner (used by rectangle and similar axis-aligned shapes).
    TopLeft,
    /// Top-right corner.
    TopRight,
    /// Bottom-left corner.
    BottomLeft,
    /// Bottom-right corner.
    BottomRight,
    /// Indexed anchor point for multi-point drawings (e.g., `Point(2)` is the
    /// third point in an XABCD pattern).
    Point(usize),
}

/// Arrow/arrowhead style for line drawings
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Hash, Serialize, Deserialize)]
pub enum ArrowStyle {
    /// No arrowhead
    None,
    /// Standard filled arrow (default)
    #[default]
    Arrow,
    /// Open/outline arrow
    OpenArrow,
    /// Circle marker
    Circle,
    /// Square marker
    Square,
    /// Diamond marker
    Diamond,
}

impl ArrowStyle {
    /// Check if this style requires any rendering
    #[inline]
    pub fn is_visible(&self) -> bool {
        !matches!(self, ArrowStyle::None)
    }

    /// Get the marker size multiplier relative to stroke width
    #[inline]
    pub fn size_multiplier(&self) -> f32 {
        match self {
            ArrowStyle::None => 0.0,
            ArrowStyle::Arrow | ArrowStyle::OpenArrow => 4.0,
            ArrowStyle::Circle => 3.0,
            ArrowStyle::Square => 3.0,
            ArrowStyle::Diamond => 3.5,
        }
    }

    /// All available arrow styles
    pub fn all() -> &'static [ArrowStyle] {
        &[
            ArrowStyle::None,
            ArrowStyle::Arrow,
            ArrowStyle::OpenArrow,
            ArrowStyle::Circle,
            ArrowStyle::Square,
            ArrowStyle::Diamond,
        ]
    }
}

pub use crate::config::FontWeight;

/// Vertical placement of a label relative to its line or shape.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Hash, Serialize, Deserialize)]
pub enum TextVerticalAlign {
    /// Positioned above the line or on a channel's upper boundary.
    #[default]
    Top,
    /// Between a channel's two boundaries (or near the top edge of a box).
    Inside,
    /// Centered on the line or middle of the shape.
    Middle,
    /// Positioned below the line or on a channel's lower boundary.
    Bottom,
}

impl TextVerticalAlign {
    /// Line/shape label choices in UI order. `Inside` is channel-only — see [`Self::channel`].
    pub fn all() -> &'static [Self] {
        &[Self::Top, Self::Middle, Self::Bottom]
    }

    /// Vertical choices for two-boundary channel tools (Flat Top/Bottom, Disjoint).
    pub fn channel() -> &'static [Self] {
        &[Self::Top, Self::Inside, Self::Bottom]
    }

    /// Display name for UI controls.
    pub fn label(self) -> &'static str {
        match self {
            Self::Top => "Top",
            Self::Inside => "Inside",
            Self::Middle => "Middle",
            Self::Bottom => "Bottom",
        }
    }
}

/// Horizontal placement of a label along its line or shape.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Hash, Serialize, Deserialize)]
pub enum TextHorizontalAlign {
    /// Near the start anchor / left edge.
    Left,
    /// Centered between the anchors.
    #[default]
    Center,
    /// Near the end anchor / right edge.
    Right,
}

impl TextHorizontalAlign {
    /// All horizontal alignment choices in UI order.
    pub fn all() -> &'static [Self] {
        &[Self::Left, Self::Center, Self::Right]
    }

    /// Display name for UI controls.
    pub fn label(self) -> &'static str {
        match self {
            Self::Left => "Left",
            Self::Center => "Center",
            Self::Right => "Right",
        }
    }
}

/// Per-timeframe visibility configuration
///
/// Allows drawings to be visible only on specific timeframes.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct TimeframeVisibility {
    /// If true, show on all timeframes (default behavior)
    pub show_on_all: bool,
    /// Set of timeframe strings where this drawing is visible
    #[serde(serialize_with = "serialize_sorted")]
    pub visible_timeframes: HashSet<String>,
}

/// Writes the set sorted, so saving the same drawings gives the same file.
fn serialize_sorted<S: serde::Serializer>(
    set: &HashSet<String>,
    serializer: S,
) -> Result<S::Ok, S::Error> {
    serializer.collect_seq(set.iter().collect::<std::collections::BTreeSet<_>>())
}

impl TimeframeVisibility {
    /// Create visibility for all timeframes (default)
    pub fn all() -> Self {
        Self {
            show_on_all: true,
            visible_timeframes: HashSet::new(),
        }
    }

    /// Create visibility for specific timeframes only
    pub fn specific(timeframes: &[&str]) -> Self {
        Self {
            show_on_all: false,
            visible_timeframes: timeframes.iter().map(|s| (*s).to_string()).collect(),
        }
    }

    /// Check if visible on the given timeframe
    #[inline]
    pub fn is_visible_on(&self, timeframe: &str) -> bool {
        self.show_on_all || self.visible_timeframes.contains(timeframe)
    }

    /// Add a timeframe to visibility
    pub fn add_timeframe(&mut self, timeframe: &str) {
        self.show_on_all = false;
        self.visible_timeframes.insert(timeframe.to_string());
    }

    /// Remove a timeframe from visibility
    pub fn remove_timeframe(&mut self, timeframe: &str) {
        self.visible_timeframes.remove(timeframe);
        if self.visible_timeframes.is_empty() {
            self.show_on_all = true;
        }
    }
}

/// Drawing interaction options
#[derive(Debug, Clone)]
pub struct DrawingOptions {
    /// Enable snap to OHLC prices
    pub snap_to_price: bool,
    /// Enable snap to candle open/close times
    pub snap_to_time: bool,
    /// Snap distance in pixels
    pub snap_distance: f32,
    /// Enable magnet mode (snap to nearby points)
    pub magnet_mode: bool,
    /// Magnet distance in pixels
    pub magnet_distance: f32,
    /// Show selection handles
    pub show_handles: bool,
    /// Handle size in pixels
    pub handle_size: f32,
    /// Handle color (RGBA)
    pub handle_color: [u8; 4],
    /// Selected handle color (RGBA)
    pub handle_sel_color: [u8; 4],
    /// Default drawing color (RGBA)
    pub default_color: [u8; 4],
}

impl Default for DrawingOptions {
    fn default() -> Self {
        Self {
            snap_to_price: true,
            snap_to_time: true,
            snap_distance: 10.0,
            magnet_mode: false,
            magnet_distance: 15.0,
            show_handles: true,
            handle_size: 6.0,
            handle_color: [255, 255, 255, 200],
            handle_sel_color: [242, 54, 69, 255], // Selection accent
            // Dark-theme default; hosts should call `default_drawing_color(theme.dark)`.
            default_color: [255, 255, 255, 255],
        }
    }
}

/// Configuration for a single Fibonacci level
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct FibonacciLevel {
    /// The ratio value (e.g., 0.236, 0.382, 0.618)
    pub value: f32,
    /// Legacy custom percentage label. Empty derives the label from `value`.
    #[serde(default)]
    pub label: String,
    /// Level color as RGBA
    pub color: [u8; 4],
    /// Whether this level is visible
    pub visible: bool,
    /// Legacy price visibility, used when the drawing leaves `show_prices` unset.
    pub show_price: bool,
}

impl FibonacciLevel {
    /// Creates a new Fibonacci level with the given ratio, display label, and color.
    ///
    /// The level is visible by default; price is omitted from the label until enabled.
    pub fn new(value: f32, label: &str, color: [u8; 4]) -> Self {
        Self {
            value,
            label: label.to_string(),
            color,
            visible: true,
            show_price: false,
        }
    }

    /// Same as [`Self::new`], with explicit visibility.
    pub fn with_visibility(value: f32, label: &str, color: [u8; 4], visible: bool) -> Self {
        Self {
            visible,
            ..Self::new(value, label, color)
        }
    }

    /// Label derived from the current ratio. Saved nonnumeric custom labels remain in percentage mode.
    pub fn label(&self, as_percent: bool) -> String {
        let numeric = self
            .label
            .strip_suffix('%')
            .is_some_and(|v| v.parse::<f32>().is_ok());
        if as_percent && !self.label.is_empty() && !numeric {
            return self.label.clone();
        }
        let text = if as_percent {
            format!("{:.4}", f64::from(self.value) * 100.0)
        } else {
            format!("{:.6}", self.value)
        };
        let text = text.trim_end_matches('0').trim_end_matches('.');
        if as_percent {
            format!("{text}%")
        } else {
            text.to_owned()
        }
    }
}

/// Configuration for all Fibonacci levels in a drawing
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct FibonacciConfig {
    /// Individual level configurations
    pub levels: Vec<FibonacciLevel>,
    /// Show background fill between levels
    pub show_background: bool,
    /// Background opacity (0-255)
    pub background_opacity: u8,
    /// Extend lines to the left
    pub extend_left: bool,
    /// Extend lines to the right
    pub extend_right: bool,
    /// Reverse the direction (flip vertically)
    pub reverse: bool,
    /// Show prices in level labels. `None` retains the tool's legacy choice:
    /// on for retracements and extensions, off for channels.
    #[serde(default)]
    pub show_prices: Option<bool>,
    /// Show the level value in labels. `None` preserves legacy labels (off for wedges).
    #[serde(default)]
    pub show_levels: Option<bool>,
    /// Format level values as percentages rather than ratios.
    /// Missing field keeps the older percentage default for saved drawings.
    #[serde(default = "percent_levels")]
    pub levels_as_percent: bool,
    /// Place labels at the left end of level lines instead of the right.
    #[serde(default)]
    pub labels_on_left: bool,
    /// Horizontal placement of level labels (`Left`, `Center`, `Right`).
    /// When `None`, falls back to `labels_on_left` (`true` -> `Left`, `false` -> `Right`).
    #[serde(default)]
    pub label_h_align: Option<TextHorizontalAlign>,
    /// Vertical placement of level labels (`Top`, `Middle`, `Bottom`).
    #[serde(default = "default_fib_label_v_align")]
    pub label_v_align: TextVerticalAlign,
    /// Font size of level labels in points.
    #[serde(default = "default_fib_label_font_size")]
    pub label_font_size: f32,
    /// Measure retracement and extension levels in logarithmic price when all anchors are positive.
    #[serde(default)]
    pub logarithmic: bool,
    /// Draw the trend line between the two anchors (retracement / extension).
    #[serde(default = "default_show_trend_line")]
    pub show_trend_line: bool,
    /// Trend line color as RGBA.
    #[serde(default = "default_trend_line_color")]
    pub trend_line_color: [u8; 4],
    /// Trend line dash pattern.
    #[serde(default = "default_trend_line_style")]
    pub trend_line_style: LineStyle,
    /// Trend line stroke width in pixels.
    #[serde(default = "default_trend_line_width")]
    pub trend_line_width: f32,
    /// Uniform stroke width for horizontal (or channel) level lines.
    #[serde(default = "default_levels_line_width")]
    pub levels_line_width: f32,
    /// Dash pattern of the level lines.
    #[serde(default)]
    pub levels_line_style: LineStyle,
}

fn percent_levels() -> bool {
    true
}

fn default_show_trend_line() -> bool {
    true
}

fn default_trend_line_color() -> [u8; 4] {
    [120, 123, 134, 255]
}

fn default_trend_line_style() -> LineStyle {
    LineStyle::Dashed
}

fn default_trend_line_width() -> f32 {
    1.0
}

fn default_levels_line_width() -> f32 {
    1.0
}

fn default_fib_label_v_align() -> TextVerticalAlign {
    TextVerticalAlign::Middle
}

fn default_fib_label_font_size() -> f32 {
    12.0
}

/// Neutral bound colour of the 0, 1, 2 and 3 levels.
const LEVEL_ANCHOR: [u8; 4] = [120, 123, 134, 255];
const LEVEL_PURPLE: [u8; 4] = [156, 39, 176, 255];
const LEVEL_AMBER: [u8; 4] = [255, 183, 77, 255];
/// Single colour of the time zone lines.
const LEVEL_ZONE: [u8; 4] = [56, 192, 253, 255];

/// The conventional colour of a Fibonacci ratio, shared by every level tool so the same ratio
/// reads the same everywhere. Unknown ratios cycle through distinguishable hues by position.
pub(crate) fn fib_ratio_color(ratio: f32, index: usize) -> [u8; 4] {
    const RED: [u8; 4] = [242, 54, 69, 255];
    const ORANGE: [u8; 4] = [255, 152, 0, 255];
    const GREEN: [u8; 4] = [76, 175, 80, 255];
    const TEAL: [u8; 4] = [8, 153, 129, 255];
    const BLUE: [u8; 4] = [91, 156, 246, 255];
    const PINK: [u8; 4] = [233, 30, 99, 255];
    const AQUA: [u8; 4] = [38, 166, 154, 255];
    const VIOLET: [u8; 4] = [171, 71, 188, 255];
    const CORAL: [u8; 4] = [239, 83, 80, 255];
    const CYAN: [u8; 4] = [56, 192, 253, 255];
    const TABLE: &[(f32, [u8; 4])] = &[
        (0.0, LEVEL_ANCHOR),
        (0.125, RED),
        (0.236, RED),
        (0.25, RED),
        (0.382, ORANGE),
        (0.5, GREEN),
        (0.618, TEAL),
        (0.705, LEVEL_AMBER),
        (0.75, BLUE),
        (0.786, BLUE),
        (0.886, LEVEL_PURPLE),
        (1.0, LEVEL_ANCHOR),
        (1.272, BLUE),
        (1.382, ORANGE),
        (1.414, AQUA),
        (1.618, RED),
        (2.0, LEVEL_ANCHOR),
        (2.272, VIOLET),
        (2.382, ORANGE),
        (2.414, PINK),
        (2.618, ORANGE),
        (3.0, LEVEL_ANCHOR),
        (3.272, CORAL),
        (3.618, LEVEL_PURPLE),
        (4.236, GREEN),
        (4.618, PINK),
        (-0.27, CYAN),
        (-0.272, CYAN),
        (-0.62, BLUE),
        (-0.618, BLUE),
        (-1.0, AQUA),
        (-1.5, VIOLET),
        (-2.0, LEVEL_PURPLE),
        (-2.5, PINK),
    ];
    const CYCLE: [[u8; 4]; 12] = [
        CYAN,
        BLUE,
        TEAL,
        GREEN,
        ORANGE,
        RED,
        PINK,
        LEVEL_PURPLE,
        LEVEL_ANCHOR,
        AQUA,
        VIOLET,
        CORAL,
    ];
    TABLE
        .iter()
        .find(|(r, _)| (r - ratio).abs() < 1e-6)
        .map_or(CYCLE[index % CYCLE.len()], |&(_, color)| color)
}

/// Levels at `ratios`, coloured by convention, with only `on` enabled.
fn ladder(ratios: &[f32], on: &[f32]) -> Vec<FibonacciLevel> {
    ratios
        .iter()
        .enumerate()
        .map(|(i, &ratio)| {
            let visible = on.iter().any(|v| (v - ratio).abs() < 1e-6);
            FibonacciLevel::with_visibility(ratio, "", fib_ratio_color(ratio, i), visible)
        })
        .collect()
}

/// The full retracement ladder with the classic pullback ratios on and the rest off.
fn retracement_levels() -> Vec<FibonacciLevel> {
    FibPreset::Classic.levels().0
}

/// A ready-made retracement level set, offered in the Inputs preset menu.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FibPreset {
    /// The standard pullback ratios, level 0 on the second anchor.
    Classic,
    /// The swing plus projections beyond it, level 0 on the first anchor.
    Extension,
    /// The optimal trade entry zone around 0.705 with targets beyond the swing.
    OptimalEntry,
}

impl FibPreset {
    /// Every preset, in menu order.
    pub const ALL: [Self; 3] = [Self::Classic, Self::Extension, Self::OptimalEntry];

    /// Menu label.
    pub fn label(self) -> &'static str {
        match self {
            Self::Classic => "Classic",
            Self::Extension => "Extension",
            Self::OptimalEntry => "OTE",
        }
    }

    /// The preset's levels and its reverse flag.
    pub fn levels(self) -> (Vec<FibonacciLevel>, bool) {
        const LADDER: &[f32] = &[
            0.0, 0.236, 0.382, 0.5, 0.618, 0.705, 0.786, 0.886, 1.0, 1.272, 1.414, 1.618, 2.0,
            2.272, 2.414, 2.618, 3.0, 3.272, 3.618, 4.236, 4.618, -0.272, -0.618, -1.0,
        ];
        match self {
            Self::Classic => (
                ladder(LADDER, &[0.0, 0.236, 0.382, 0.5, 0.618, 0.786, 1.0]),
                false,
            ),
            Self::Extension => (
                ladder(
                    LADDER,
                    &[0.0, 1.0, 1.272, 1.414, 1.618, 2.0, 2.618, 3.618, 4.236],
                ),
                true,
            ),
            Self::OptimalEntry => {
                let mut levels = ladder(
                    &[
                        0.0, 0.236, 0.382, 0.5, 0.62, 0.705, 0.79, 0.886, 1.0, 1.272, 1.414, 1.618,
                        2.0, 2.618, 3.0, 3.618, 4.236, 4.618, -0.27, -0.62, -1.0, -1.5, -2.0, -2.5,
                    ],
                    &[0.0, 0.5, 0.62, 0.705, 0.79, 1.0, -0.27, -0.62],
                );
                // The entry zone reads as one band.
                for level in &mut levels {
                    if [0.62, 0.79].iter().any(|v| (v - level.value).abs() < 1e-6) {
                        level.color = LEVEL_AMBER;
                    }
                }
                (levels, false)
            }
        }
    }

    /// The preset `config` matches exactly, or `None` once the user has edited it.
    pub fn matching(config: &FibonacciConfig) -> Option<Self> {
        Self::ALL.into_iter().find(|preset| {
            let (levels, reverse) = preset.levels();
            reverse == config.reverse && levels == config.levels
        })
    }
}

impl Default for FibonacciConfig {
    /// Platform Fibonacci retracement: ratio+price labels, dashed trend line, soft fills.
    fn default() -> Self {
        Self {
            levels: retracement_levels(),
            show_background: true,
            background_opacity: 20,
            extend_left: false,
            extend_right: false,
            reverse: false,
            logarithmic: false,
            show_prices: Some(true),
            show_levels: Some(true),
            levels_as_percent: false,
            labels_on_left: false,
            label_h_align: Some(TextHorizontalAlign::Right),
            label_v_align: TextVerticalAlign::Middle,
            label_font_size: default_fib_label_font_size(),
            show_trend_line: true,
            trend_line_color: default_trend_line_color(),
            trend_line_style: default_trend_line_style(),
            trend_line_width: default_trend_line_width(),
            levels_line_width: default_levels_line_width(),
            levels_line_style: LineStyle::Solid,
        }
    }
}

impl FibonacciConfig {
    /// Labels above the left end of each level, where drawings saved without a config put them.
    pub(crate) fn with_legacy_labels(self) -> Self {
        Self {
            labels_on_left: true,
            label_h_align: Some(TextHorizontalAlign::Left),
            label_v_align: TextVerticalAlign::Top,
            ..self
        }
    }

    /// Extension levels (beyond 100%)
    pub fn extension_default() -> Self {
        let mut levels = ladder(
            &[0.0, 0.382, 0.5, 0.618, 1.0, 1.618, 2.618],
            &[0.0, 0.382, 0.5, 0.618, 1.0, 1.618, 2.618],
        );
        // The furthest default target reads as the outermost projection.
        if let Some(last) = levels.last_mut() {
            last.color = LEVEL_PURPLE;
        }
        Self {
            levels,
            show_background: false,
            background_opacity: 20,
            extend_left: false,
            extend_right: false,
            reverse: false,
            logarithmic: false,
            show_prices: Some(true),
            show_levels: Some(true),
            levels_as_percent: false,
            labels_on_left: true,
            label_h_align: Some(TextHorizontalAlign::Left),
            label_v_align: TextVerticalAlign::Top,
            label_font_size: default_fib_label_font_size(),
            show_trend_line: true,
            trend_line_color: default_trend_line_color(),
            trend_line_style: default_trend_line_style(),
            trend_line_width: default_trend_line_width(),
            levels_line_width: default_levels_line_width(),
            levels_line_style: LineStyle::Solid,
        }
    }

    /// Effective horizontal alignment of level labels, honouring legacy `labels_on_left`.
    pub fn effective_h_align(&self) -> TextHorizontalAlign {
        if self.labels_on_left {
            return TextHorizontalAlign::Left;
        }
        self.label_h_align.unwrap_or(TextHorizontalAlign::Right)
    }

    /// Update horizontal alignment and keep `labels_on_left` in sync.
    pub fn set_h_align(&mut self, align: TextHorizontalAlign) {
        self.label_h_align = Some(align);
        self.labels_on_left = align == TextHorizontalAlign::Left;
    }

    /// Initial levels for a tool. Existing drawings without a stored config keep their
    /// original renderer defaults; new tools store this table when constructed.
    pub fn for_tool(tool: super::DrawingToolType) -> Self {
        use super::DrawingToolType as T;
        let levels = match tool {
            T::FibonacciChannel => ladder(
                &[0.0, 0.236, 0.382, 0.5, 0.618, 0.786, 1.0, 1.618, 2.618],
                &[0.0, 0.236, 0.382, 0.5, 0.618, 0.786, 1.0, 1.618, 2.618],
            ),
            T::FibonacciTimeZones => [0.0, 1.0, 2.0, 3.0, 5.0, 8.0, 13.0, 21.0, 34.0]
                .map(|value| FibonacciLevel::new(value, "", LEVEL_ZONE))
                .to_vec(),
            T::TrendBasedFibTime => {
                let ratios = [
                    0.0, 0.382, 0.5, 0.618, 1.0, 1.382, 1.618, 2.0, 2.382, 2.618, 3.0,
                ];
                // 0.5 is present but hidden by default.
                let on: Vec<f32> = ratios.into_iter().filter(|&r| r != 0.5).collect();
                ladder(&ratios, &on)
            }
            T::FibonacciSpeedFan => ladder(
                &[0.0, 0.236, 0.382, 0.5, 0.618, 0.786, 1.0],
                &[0.0, 0.236, 0.382, 0.5, 0.618, 0.786, 1.0],
            ),
            T::FibonacciCircles | T::FibonacciSpeedResistanceArcs => ladder(
                &[0.236, 0.382, 0.5, 0.618, 0.786, 1.0, 1.618, 2.618],
                &[0.236, 0.382, 0.5, 0.618, 0.786, 1.0, 1.618, 2.618],
            ),
            T::FibonacciWedge => ladder(
                &[0.236, 0.382, 0.5, 0.618, 0.786, 1.0],
                &[0.236, 0.382, 0.5, 0.618, 0.786, 1.0],
            ),
            T::FibonacciExtension => return Self::extension_default(),
            _ => return Self::default(),
        };
        Self {
            levels,
            // Only the horizontal level tools tint between levels by default.
            show_background: false,
            background_opacity: 20,
            // Ratios read as plain numbers; channels label the ratio only.
            show_prices: Some(false),
            levels_as_percent: false,
            labels_on_left: false,
            label_h_align: Some(TextHorizontalAlign::Right),
            label_v_align: TextVerticalAlign::Top,
            ..Self::default()
        }
    }

    /// Label content; a tool without a price passes `None`.
    pub(crate) fn label_text(&self, level: &FibonacciLevel, price: Option<f64>) -> String {
        if self.show_prices.is_none()
            && self.show_levels.is_none()
            && !level.show_price
            && level.label.is_empty()
        {
            return String::new();
        }
        let value = self
            .show_levels
            .unwrap_or(true)
            .then(|| level.label(self.levels_as_percent));
        let price = price.filter(|_| self.show_prices.unwrap_or(level.show_price));
        match (value, price) {
            (Some(value), Some(price)) => format!("{value} ({price:.2})"),
            (Some(value), None) => value,
            (None, Some(price)) => format!("{price:.2}"),
            (None, None) => String::new(),
        }
    }

    /// Get visible levels only
    pub fn visible_levels(&self) -> impl Iterator<Item = &FibonacciLevel> {
        self.levels.iter().filter(|l| l.visible)
    }

    /// Position of retracement level `value` between the first anchor `start`
    /// and the second anchor `end`, in price units.
    ///
    /// Level 0 sits on the second anchor and level 1 on the first, so levels
    /// measure back from the end of the move; `reverse` swaps the two anchors.
    pub(crate) fn retracement_position(&self, start: f64, end: f64, value: f64) -> f64 {
        let (from, to) = if self.reverse {
            (start, end)
        } else {
            (end, start)
        };
        if self.logarithmic && from > 0.0 && to > 0.0 {
            (from.ln() + (to.ln() - from.ln()) * value).exp()
        } else {
            from + (to - from) * value
        }
    }

    /// Price of each visible level, in config order, between the anchor prices `start` and `end`:
    /// a retracement's as [`Self::retracement_position`] places them, an extension's projected
    /// from `origin` by `end - start`, or by the log swing when all three prices are positive.
    pub(crate) fn level_prices(
        &self,
        start: f64,
        end: f64,
        extension: bool,
        origin: f64,
    ) -> impl Iterator<Item = f64> + '_ {
        self.visible_levels().map(move |level| {
            let value = f64::from(level.value);
            if extension {
                if self.logarithmic && start > 0.0 && end > 0.0 && origin > 0.0 {
                    (origin.ln() + (end.ln() - start.ln()) * value).exp()
                } else {
                    origin + (end - start) * value
                }
            } else {
                self.retracement_position(start, end, value)
            }
        })
    }
}

/// Input series for the regression trend fit.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default, Serialize, Deserialize)]
pub enum RegressionSource {
    /// Opening price.
    Open,
    /// Highest price.
    High,
    /// Lowest price.
    Low,
    /// Closing price.
    #[default]
    Close,
    /// Bar volume.
    Volume,
    /// (High + Low) / 2.
    Hl2,
    /// (High + Low + Close) / 3.
    Hlc3,
    /// (Open + High + Low + Close) / 4.
    Ohlc4,
    /// (High + Low + Close + Close) / 4.
    Hlcc4,
}

impl RegressionSource {
    /// Display label for menus.
    pub fn label(self) -> &'static str {
        match self {
            Self::Open => "Open",
            Self::High => "High",
            Self::Low => "Low",
            Self::Close => "Close",
            Self::Volume => "Volume",
            Self::Hl2 => "(H + L)/2",
            Self::Hlc3 => "(H + L + C)/3",
            Self::Ohlc4 => "(O + H + L + C)/4",
            Self::Hlcc4 => "(H + L + C + C)/4",
        }
    }

    /// All sources in UI order.
    pub fn all() -> [Self; 9] {
        [
            Self::Open,
            Self::High,
            Self::Low,
            Self::Close,
            Self::Volume,
            Self::Hl2,
            Self::Hlc3,
            Self::Ohlc4,
            Self::Hlcc4,
        ]
    }

    /// Value of `bar` for this source.
    pub fn value(self, bar: &crate::model::Bar) -> f64 {
        match self {
            Self::Open => bar.open,
            Self::High => bar.high,
            Self::Low => bar.low,
            Self::Close => bar.close,
            Self::Volume => bar.volume,
            Self::Hl2 => (bar.high + bar.low) / 2.0,
            Self::Hlc3 => (bar.high + bar.low + bar.close) / 3.0,
            Self::Ohlc4 => (bar.open + bar.high + bar.low + bar.close) / 4.0,
            Self::Hlcc4 => (bar.high + bar.low + bar.close + bar.close) / 4.0,
        }
    }
}

fn default_regression_deviation() -> f64 {
    2.0
}

/// Midline default `#9C9C9CEA`.
fn default_regression_midline_color() -> [u8; 4] {
    [156, 156, 156, 234]
}

/// Upper line / fill base `#089981EA`.
fn default_regression_up_color() -> [u8; 4] {
    [8, 153, 129, 234]
}

/// Lower line / fill base `#F23645EA`.
fn default_regression_down_color() -> [u8; 4] {
    [242, 54, 69, 234]
}

fn default_regression_upper_fill() -> [u8; 4] {
    [8, 153, 129, 51] // 20%
}

fn default_regression_lower_fill() -> [u8; 4] {
    [242, 54, 69, 51] // 20%
}

fn default_regression_midline_style() -> LineStyle {
    LineStyle::Dashed
}

fn default_regression_band_style() -> LineStyle {
    LineStyle::Solid
}

/// Settings of the regression trend drawing.
#[derive(Debug, Clone, Copy, PartialEq, Serialize)]
pub struct RegressionTrendConfig {
    /// Upper band distance in standard deviations.
    pub upper_deviation: f64,
    /// Lower band distance in standard deviations.
    pub lower_deviation: f64,
    /// Whether the upper band is drawn.
    pub use_upper: bool,
    /// Whether the lower band is drawn.
    pub use_lower: bool,
    /// Series fitted by the regression.
    pub source: RegressionSource,
    /// Whether Pearson's R is labelled on the chart.
    pub show_pearson: bool,
    /// Midline stroke colour (RGBA).
    pub midline_color: [u8; 4],
    /// Midline dash pattern.
    pub midline_style: LineStyle,
    /// Upper band stroke colour (RGBA).
    pub up_color: [u8; 4],
    /// Upper band dash pattern.
    pub upper_style: LineStyle,
    /// Lower band stroke colour (RGBA).
    pub down_color: [u8; 4],
    /// Lower band dash pattern.
    pub lower_style: LineStyle,
    /// Fill between midline and upper band (RGBA).
    pub upper_fill: [u8; 4],
    /// Fill between midline and lower band (RGBA).
    pub lower_fill: [u8; 4],
    /// Whether the upper fill is painted.
    pub show_upper_fill: bool,
    /// Whether the lower fill is painted.
    pub show_lower_fill: bool,
}

/// Stored settings of a regression trend, checked on load. Accepts the legacy
/// single `deviations` field and the expanded upper/lower form.
#[derive(Deserialize)]
struct RegressionTrendFields {
    #[serde(default)]
    deviations: Option<f64>,
    #[serde(default = "default_regression_deviation")]
    upper_deviation: f64,
    #[serde(default = "default_regression_deviation")]
    lower_deviation: f64,
    #[serde(default = "enabled_true")]
    use_upper: bool,
    #[serde(default = "enabled_true")]
    use_lower: bool,
    #[serde(default)]
    source: RegressionSource,
    #[serde(default = "enabled_true")]
    show_pearson: bool,
    #[serde(default = "default_regression_midline_color")]
    midline_color: [u8; 4],
    #[serde(default = "default_regression_midline_style")]
    midline_style: LineStyle,
    #[serde(default = "default_regression_up_color")]
    up_color: [u8; 4],
    #[serde(default = "default_regression_band_style")]
    upper_style: LineStyle,
    #[serde(default = "default_regression_down_color")]
    down_color: [u8; 4],
    #[serde(default = "default_regression_band_style")]
    lower_style: LineStyle,
    #[serde(default = "default_regression_upper_fill")]
    upper_fill: [u8; 4],
    #[serde(default = "default_regression_lower_fill")]
    lower_fill: [u8; 4],
    #[serde(default = "enabled_true")]
    show_upper_fill: bool,
    #[serde(default = "enabled_true")]
    show_lower_fill: bool,
}

fn enabled_true() -> bool {
    true
}

impl TryFrom<RegressionTrendFields> for RegressionTrendConfig {
    type Error = &'static str;

    fn try_from(fields: RegressionTrendFields) -> Result<Self, Self::Error> {
        let (upper, lower) = match fields.deviations {
            Some(deviations) => (deviations, deviations),
            None => (fields.upper_deviation, fields.lower_deviation),
        };
        Self::validated(
            upper,
            lower,
            fields.use_upper,
            fields.use_lower,
            fields.source,
            fields.show_pearson,
            fields.midline_color,
            fields.midline_style,
            fields.up_color,
            fields.upper_style,
            fields.down_color,
            fields.lower_style,
            fields.upper_fill,
            fields.lower_fill,
            fields.show_upper_fill,
            fields.show_lower_fill,
        )
        .ok_or("regression trend settings out of bounds")
    }
}

impl<'de> Deserialize<'de> for RegressionTrendConfig {
    fn deserialize<D: serde::Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        RegressionTrendFields::deserialize(deserializer)?
            .try_into()
            .map_err(serde::de::Error::custom)
    }
}

impl RegressionTrendConfig {
    /// Symmetric bands `deviations` standard deviations either side of the fitted
    /// line (above 0, at most 10); `None` when out of bounds.
    pub fn new(deviations: f64) -> Option<Self> {
        Self::validated(
            deviations,
            deviations,
            true,
            true,
            RegressionSource::Close,
            true,
            default_regression_midline_color(),
            default_regression_midline_style(),
            default_regression_up_color(),
            default_regression_band_style(),
            default_regression_down_color(),
            default_regression_band_style(),
            default_regression_upper_fill(),
            default_regression_lower_fill(),
            true,
            true,
        )
    }

    /// Builds a config when both deviations are in (0, 10].
    #[allow(clippy::too_many_arguments)]
    pub fn validated(
        upper_deviation: f64,
        lower_deviation: f64,
        use_upper: bool,
        use_lower: bool,
        source: RegressionSource,
        show_pearson: bool,
        midline_color: [u8; 4],
        midline_style: LineStyle,
        up_color: [u8; 4],
        upper_style: LineStyle,
        down_color: [u8; 4],
        lower_style: LineStyle,
        upper_fill: [u8; 4],
        lower_fill: [u8; 4],
        show_upper_fill: bool,
        show_lower_fill: bool,
    ) -> Option<Self> {
        let in_range = |v: f64| v > 0.0 && v <= 10.0;
        (in_range(upper_deviation) && in_range(lower_deviation)).then_some(Self {
            upper_deviation,
            lower_deviation,
            use_upper,
            use_lower,
            source,
            show_pearson,
            midline_color,
            midline_style,
            up_color,
            upper_style,
            down_color,
            lower_style,
            upper_fill,
            lower_fill,
            show_upper_fill,
            show_lower_fill,
        })
    }

    /// Keep validated deviations; refresh appearance from `self`.
    pub fn with_deviations(self, upper: f64, lower: f64) -> Option<Self> {
        Self::validated(
            upper,
            lower,
            self.use_upper,
            self.use_lower,
            self.source,
            self.show_pearson,
            self.midline_color,
            self.midline_style,
            self.up_color,
            self.upper_style,
            self.down_color,
            self.lower_style,
            self.upper_fill,
            self.lower_fill,
            self.show_upper_fill,
            self.show_lower_fill,
        )
    }

    /// Distance of the upper band from the fitted line, in standard deviations.
    pub fn deviations(&self) -> f64 {
        self.upper_deviation
    }
}

impl Default for RegressionTrendConfig {
    /// Bands two standard deviations away, both sides on, close source, platform colours.
    fn default() -> Self {
        Self {
            upper_deviation: 2.0,
            lower_deviation: 2.0,
            use_upper: true,
            use_lower: true,
            source: RegressionSource::Close,
            show_pearson: true,
            midline_color: default_regression_midline_color(),
            midline_style: default_regression_midline_style(),
            up_color: default_regression_up_color(),
            upper_style: default_regression_band_style(),
            down_color: default_regression_down_color(),
            lower_style: default_regression_band_style(),
            upper_fill: default_regression_upper_fill(),
            lower_fill: default_regression_lower_fill(),
            show_upper_fill: true,
            show_lower_fill: true,
        }
    }
}

/// Instrument economics used by long and short position drawings.
/// Quantities are lots; one lot contains `lot_size` units, each worth `point_value`
/// in the account currency per price point. Defaults retain unit sizing without rounding.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct PositionSizing {
    /// Account-currency value of one price point per underlying unit.
    pub point_value: f64,
    /// Underlying units per displayed lot.
    pub lot_size: f64,
    /// Permitted lot increment; zero leaves the quantity unrounded.
    pub quantity_step: f64,
    /// Notional value divided by required capital; positive and finite.
    pub leverage: f64,
}

impl Default for PositionSizing {
    fn default() -> Self {
        Self {
            point_value: 1.0,
            lot_size: 1.0,
            quantity_step: 0.0,
            leverage: 1.0,
        }
    }
}

impl PositionSizing {
    /// Whether the economics are finite and positive, with a nonnegative quantity step.
    pub fn is_valid(&self) -> bool {
        [
            self.point_value,
            self.lot_size,
            self.leverage,
            self.point_value * self.lot_size,
        ]
        .into_iter()
        .all(|v| v.is_finite() && v > 0.0)
            && self.quantity_step.is_finite()
            && self.quantity_step >= 0.0
    }

    /// Required capital: absolute entry notional divided by leverage, in account currency.
    /// Returns `None` for invalid settings or a nonpositive or nonfinite quantity.
    pub fn required_capital(&self, entry: f64, quantity: f64) -> Option<f64> {
        if !self.is_valid() || !quantity.is_finite() || quantity <= 0.0 {
            return None;
        }
        let capital = entry.abs() * self.point_value * self.lot_size * quantity / self.leverage;
        capital.is_finite().then_some(capital)
    }
}

/// How a long or short position drawing sizes its quantity.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub enum PositionSize {
    /// A fixed quantity, interpreted as lots by [`Self::quantity_with_sizing`].
    Quantity(f64),
    /// The quantity that loses this amount if the stop is hit.
    RiskAmount(f64),
    /// The quantity that loses `percent` of `account_size` if the stop is hit.
    RiskPercent {
        /// Account size, in the same currency as risk amounts and the configured point value.
        account_size: f64,
        /// Share of the account risked, in percent (above 0, at most 100).
        percent: f64,
    },
}

impl PositionSize {
    /// Size in lots using the instrument's point value and lot size. Risk-based sizes and
    /// fixed lot quantities round down to `quantity_step`, so rounding cannot increase risk.
    /// Returns `None` if no positive permitted quantity fits, or inputs are invalid.
    pub fn quantity_with_sizing(
        &self,
        entry: f64,
        stop: f64,
        sizing: &PositionSizing,
    ) -> Option<f64> {
        if !sizing.is_valid() {
            return None;
        }
        let mut quantity = self.quantity(entry, stop)?;
        if !matches!(self, Self::Quantity(_)) {
            quantity /= sizing.point_value * sizing.lot_size;
        }
        if sizing.quantity_step > 0.0 {
            let steps = quantity / sizing.quantity_step;
            // Decimal steps can put an exact multiple just below its integer quotient.
            let nearest = steps.round();
            let steps = if (steps - nearest).abs() <= 4.0 * f64::EPSILON * steps.abs() {
                nearest
            } else {
                steps.floor()
            };
            quantity = steps * sizing.quantity_step;
        }
        (quantity.is_finite() && quantity > 0.0).then_some(quantity)
    }

    /// Unit quantity at `entry` with its stop at `stop`, without instrument conversion.
    /// Use [`Self::quantity_with_sizing`] for lot sizing and quantity-step rounding.
    ///
    /// `None` when a size from the risk has its stop at the entry, or a
    /// setting is out of bounds (not positive and finite, or a percent above 100).
    pub fn quantity(&self, entry: f64, stop: f64) -> Option<f64> {
        let positive = |value: f64| value.is_finite() && value > 0.0;
        let risk = match *self {
            Self::Quantity(quantity) => return positive(quantity).then_some(quantity),
            Self::RiskAmount(amount) => amount,
            Self::RiskPercent {
                account_size,
                percent,
            } if positive(account_size) && positive(percent) && percent <= 100.0 => {
                account_size * (percent / 100.0)
            }
            Self::RiskPercent { .. } => return None,
        };
        let quantity = risk / (entry - stop).abs();
        positive(quantity).then_some(quantity)
    }
}

impl Default for PositionSize {
    /// One unit, so the profit and loss read per unit.
    fn default() -> Self {
        Self::Quantity(1.0)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::data::VolumeProfileConfig;

    #[test]
    fn text_vertical_align_inside_is_channel_only() {
        assert_eq!(
            TextVerticalAlign::all(),
            &[
                TextVerticalAlign::Top,
                TextVerticalAlign::Middle,
                TextVerticalAlign::Bottom
            ]
        );
        assert_eq!(
            TextVerticalAlign::channel(),
            &[
                TextVerticalAlign::Top,
                TextVerticalAlign::Inside,
                TextVerticalAlign::Bottom
            ]
        );
        assert!(!TextVerticalAlign::all().contains(&TextVerticalAlign::Inside));
    }

    /// Reference: quantity = risked amount / |entry - stop| (reference.py);
    /// none when a size from the risk has its stop at the entry, or a setting
    /// is out of bounds.
    #[test]
    fn position_size_quantity() {
        let percent = |account_size, percent| PositionSize::RiskPercent {
            account_size,
            percent,
        };
        for (size, entry, stop, want) in [
            (PositionSize::Quantity(3.0), 50.0, 48.0, Some(3.0)),
            (PositionSize::RiskAmount(100.0), 50.0, 48.0, Some(50.0)),
            (percent(10_000.0, 1.0), 50.0, 52.5, Some(40.0)),
            (percent(2_000.0, 2.5), 1.1, 1.0975, Some(19999.99999999865)),
            (PositionSize::RiskAmount(100.0), 50.0, 50.0, None),
            (PositionSize::RiskAmount(-100.0), 50.0, 48.0, None),
            (PositionSize::Quantity(f64::NAN), 50.0, 48.0, None),
            (percent(10_000.0, 150.0), 50.0, 48.0, None),
            (percent(-10_000.0, -1.0), 50.0, 48.0, None),
            (percent(1e308, 100.0), 1.0, 0.0, Some(1e308)),
        ] {
            let got = size.quantity(entry, stop);
            assert!(
                got.zip(want)
                    .map_or(got == want, |(got, want)| (got - want).abs() < 1e-6),
                "{size:?}: got {got:?}, want {want:?}"
            );
        }

        // Hand calculations in lots, independent of the renderer's label formatting.
        let sizing = PositionSizing {
            point_value: 50.0,
            lot_size: 10.0,
            quantity_step: 0.01,
            leverage: 5.0,
        };
        for (size, stop, expected) in [
            (PositionSize::RiskAmount(230.0), 97.5, Some(0.18)),
            (PositionSize::RiskAmount(230.0), 102.5, Some(0.18)),
            (
                PositionSize::RiskPercent {
                    account_size: 23000.0,
                    percent: 1.0,
                },
                97.5,
                Some(0.18),
            ),
            (PositionSize::Quantity(0.189), 97.5, Some(0.18)),
            (PositionSize::Quantity(0.29), 97.5, Some(0.29)),
            (PositionSize::Quantity(0.009), 97.5, None),
            (PositionSize::RiskAmount(230.0), 100.0, None),
        ] {
            let quantity = size.quantity_with_sizing(100.0, stop, &sizing);
            assert_eq!(quantity, expected, "{size:?}, stop {stop}");
            let unleveraged = PositionSizing {
                leverage: 1.0,
                ..sizing
            };
            assert_eq!(
                quantity,
                size.quantity_with_sizing(100.0, stop, &unleveraged)
            );
        }
        assert_eq!(sizing.required_capital(100.0, 0.18), Some(1800.0));
        assert_eq!(
            PositionSize::Quantity(2.13).quantity_with_sizing(
                100.0,
                97.5,
                &PositionSizing {
                    quantity_step: 0.25,
                    ..sizing
                }
            ),
            Some(2.0)
        );
        for sizing in [
            PositionSizing {
                point_value: 0.0,
                ..sizing
            },
            PositionSizing {
                lot_size: -1.0,
                ..sizing
            },
            PositionSizing {
                point_value: 1e-200,
                lot_size: 1e-200,
                ..sizing
            },
            PositionSizing {
                quantity_step: -0.01,
                ..sizing
            },
            PositionSizing {
                leverage: f64::NAN,
                ..sizing
            },
        ] {
            assert!(
                PositionSize::default()
                    .quantity_with_sizing(100.0, 97.5, &sizing)
                    .is_none()
            );
            assert!(sizing.required_capital(100.0, 0.18).is_none());
        }
    }

    #[test]
    fn drawing_settings_reject_out_of_bounds_values() {
        assert!(VolumeProfileConfig::new(1, 1.0).is_some());
        assert!(VolumeProfileConfig::new(1000, 0.01).is_some());
        for (rows, value_area) in [(0, 0.7), (1001, 0.7), (24, 0.0), (24, 1.01), (24, f64::NAN)] {
            assert!(VolumeProfileConfig::new(rows, value_area).is_none());
        }
        for height in [0.0, -1.0, f64::INFINITY, f64::NAN] {
            assert!(
                VolumeProfileConfig::default()
                    .with_row_size(Some(height))
                    .is_none()
            );
        }
        assert!(RegressionTrendConfig::new(10.0).is_some());
        for deviations in [0.0, -1.0, 10.5, f64::INFINITY, f64::NAN] {
            assert!(RegressionTrendConfig::new(deviations).is_none());
        }
    }

    #[test]
    fn test_timeframe_visibility() {
        let mut vis = TimeframeVisibility::all();
        assert!(vis.is_visible_on("1h"));
        assert!(vis.is_visible_on("1D"));

        vis.add_timeframe("1h");
        assert!(vis.is_visible_on("1h"));
        assert!(!vis.is_visible_on("1D"));
    }

    /// New tools start from the conventional level sets: the classic retracement (matching its
    /// preset), shared ratio colours, and fills only where levels are horizontal.
    #[test]
    fn fibonacci_defaults_follow_the_conventional_sets() {
        use super::super::DrawingToolType as T;
        let on = |config: &FibonacciConfig| -> Vec<f32> {
            config.visible_levels().map(|l| l.value).collect()
        };
        let retracement = FibonacciConfig::default();
        assert_eq!(
            on(&retracement),
            [0.0, 0.236, 0.382, 0.5, 0.618, 0.786, 1.0]
        );
        assert_eq!(FibPreset::matching(&retracement), Some(FibPreset::Classic));
        assert_eq!(retracement.effective_h_align(), TextHorizontalAlign::Right);
        assert_eq!(retracement.label_v_align, TextVerticalAlign::Middle);
        assert!(retracement.show_background);
        // Drawings saved without a config keep their labels above the left end.
        let legacy = crate::drawings::Drawing::new(0, T::FibonacciRetracement);
        let legacy = crate::drawings::Drawing {
            fib_config: None,
            ..legacy
        };
        let legacy = legacy.fibonacci_config();
        assert_eq!(legacy.effective_h_align(), TextHorizontalAlign::Left);
        assert_eq!(legacy.label_v_align, TextVerticalAlign::Top);
        assert_eq!(
            FibonacciConfig::for_tool(T::FibonacciChannel).label_v_align,
            TextVerticalAlign::Top
        );

        let (levels, reverse) = FibPreset::Extension.levels();
        assert!(reverse);
        assert!(levels.iter().any(|l| l.visible && l.value == 4.236));

        let extension = FibonacciConfig::extension_default();
        assert_eq!(on(&extension), [0.0, 0.382, 0.5, 0.618, 1.0, 1.618, 2.618]);
        assert!(!extension.show_background && !extension.extend_right);

        for (tool, want) in [
            (
                T::FibonacciChannel,
                &[0.0, 0.236, 0.382, 0.5, 0.618, 0.786, 1.0, 1.618, 2.618][..],
            ),
            (
                T::FibonacciTimeZones,
                &[0.0, 1.0, 2.0, 3.0, 5.0, 8.0, 13.0, 21.0, 34.0],
            ),
            (
                T::FibonacciSpeedFan,
                &[0.0, 0.236, 0.382, 0.5, 0.618, 0.786, 1.0],
            ),
            (
                T::TrendBasedFibTime,
                &[0.0, 0.382, 0.618, 1.0, 1.382, 1.618, 2.0, 2.382, 2.618, 3.0],
            ),
            (
                T::FibonacciCircles,
                &[0.236, 0.382, 0.5, 0.618, 0.786, 1.0, 1.618, 2.618],
            ),
            (T::FibonacciWedge, &[0.236, 0.382, 0.5, 0.618, 0.786, 1.0]),
        ] {
            let config = FibonacciConfig::for_tool(tool);
            assert_eq!(on(&config), want, "{tool:?}");
            assert!(!config.show_background, "{tool:?}");
        }
        // The same ratio reads the same colour in every tool.
        let teal = |config: FibonacciConfig| {
            config
                .levels
                .into_iter()
                .find(|l| l.value == 0.618)
                .unwrap()
                .color
        };
        assert_eq!(
            teal(FibonacciConfig::default()),
            teal(FibonacciConfig::for_tool(T::FibonacciCircles))
        );
    }

    /// Anchors 100 then 200: level v sits at 200 - 100 * v, or at 100 + 100 * v when reversed.
    /// Reference: reverse=false → p2 + ratio*(p1-p2); reverse=true → p1 + ratio*(p2-p1).
    #[test]
    fn fibonacci_retracement_position() {
        for (reverse, value, want) in [
            (false, 0.0, 200.0),
            (false, 0.236, 176.4),
            (false, 0.618, 138.2),
            (false, 1.0, 100.0),
            (true, 0.0, 100.0),
            (true, 0.236, 123.6),
            (true, 0.786, 178.6),
            (true, 1.0, 200.0),
        ] {
            let config = FibonacciConfig {
                reverse,
                ..Default::default()
            };
            let (p1, p2) = (100.0, 200.0);
            let got = config.retracement_position(p1, p2, value);
            let expected = if reverse {
                p1 + value * (p2 - p1)
            } else {
                p2 + value * (p1 - p2)
            };
            assert!(
                (got - want).abs() < 1e-9 && (got - expected).abs() < 1e-9,
                "reverse {reverse}, level {value}: got {got}, want {want}"
            );
        }
        // With 0 at 10 and 1 at 1000, log10(price) = 1 + 2*r.
        // Rounded independently: 10^1.472, 10^2, 10^2.236.
        for (logarithmic, low, ratio, want) in [
            (true, 10.0, 0.236, 29.64831389524342),
            (true, 10.0, 0.5, 100.0),
            (true, 10.0, 0.618, 172.1868574986006),
            (false, 10.0, 0.236, 243.64),
            (false, 10.0, 0.5, 505.0),
            (false, 10.0, 0.618, 621.82),
            (true, 0.0, 0.5, 500.0),
            (true, -10.0, 0.5, 495.0),
        ] {
            for reverse in [false, true] {
                let config = FibonacciConfig {
                    logarithmic,
                    reverse,
                    ..Default::default()
                };
                let (start, end) = if reverse {
                    (low, 1000.0)
                } else {
                    (1000.0, low)
                };
                let got = config.retracement_position(start, end, ratio);
                assert!(
                    (got - want).abs() < 1e-9,
                    "log {logarithmic}, reverse {reverse}, ratio {ratio}: {got} != {want}"
                );
            }
        }
    }
}
