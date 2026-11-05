//! Drawing struct - represents a single drawing on the chart
//!
//! This is the core data structure for chart drawings. It contains:
//! - Dual coordinate system (screen coords + chart coords)
//! - Styling properties (color, stroke, line style)
//! - Tool-specific configuration (Fibonacci levels, etc.)
//!
//! Note: Rendering methods are in the rendering module, not here.
//! This keeps the domain layer pure and testable.

use super::{
    ArrowStyle, ChartPoint, DrawingToolType, FibonacciConfig, FontWeight, GannConfig,
    GeometryConfig, GhostFeedConfig, MeasureConfig, PatternConfig, PitchforkConfig,
    PositionDisplay, PositionSize, PositionSizing, PriceActionConfig, RegressionTrendConfig,
    TextHorizontalAlign, TextVerticalAlign, TimeframeVisibility, VwapConfig,
};

use crate::data::VolumeProfileConfig;
use crate::model::{AlertSource, LineStyle};
use crate::scales::PriceScaleMode;
use serde::{Deserialize, Serialize};

/// A drawing on the chart
///
/// Drawings use a dual coordinate system:
/// - `points: Vec<Pos2>` - Screen coordinates, updated each frame
/// - `chart_points: Vec<ChartPoint>` - Chart coordinates (bar_idx, price), stable across pan/zoom
///
/// # Example
///
/// ```
/// use egui_charts::drawings::{Drawing, DrawingToolType};
///
/// let mut drawing = Drawing::new(1, DrawingToolType::TrendLine);
/// drawing.color = [33, 150, 243, 255]; // Blue
/// drawing.stroke_width = 2.0;
/// ```
///
/// Serde covers the tool and its settings only: the id, the points and the
/// completion state are left out, since the anchors are stored as
/// timestamps by [`persistence`](crate::drawings::persistence). A field added
/// later needs `#[serde(default)]`, so that saved version 2 files still load.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Drawing {
    /// Unique identifier for this drawing
    #[serde(skip)]
    pub id: usize,
    /// Id of the stored record this drawing was restored from; `None` for a drawing never
    /// stored, which is saved under its `id`
    #[serde(skip)]
    pub stored_id: Option<String>,
    /// Creation time, in Unix milliseconds, of the stored record this drawing was restored
    /// from; `None` for a drawing never stored, which is saved with the time of the save
    #[serde(skip)]
    pub created_at: Option<i64>,
    /// User notes, saved with the drawing and restored from its stored record
    #[serde(skip)]
    pub notes: Option<String>,
    /// Optional object-tree name, saved with the drawing.
    #[serde(default)]
    pub label: Option<String>,
    /// Owning study id, or `None` for the main price pane. Follows that study when panes move
    /// or merge; hosts restoring studies must retain or remap their ids. Older files use main.
    #[serde(default)]
    pub pane: Option<usize>,
    /// Whether pointer and programmatic selection can select this drawing.
    #[serde(default = "enabled")]
    pub selectable: bool,
    /// Whether chart saves and persistence notifications include this drawing.
    #[serde(default = "enabled")]
    pub saveable: bool,
    /// Whether host object lists should include this drawing. Rendering is independent.
    #[serde(default = "enabled")]
    pub listed: bool,
    /// Host-defined group identity, retained through saving and undo.
    #[serde(default)]
    pub group: Option<String>,
    /// Named object-tree folder path, from outermost to innermost; empty means the root.
    /// Independent of host-defined groups. Older saved drawings start at the root.
    #[serde(default)]
    pub folder: Vec<String>,
    /// Stable id of a host-registered tool; `None` uses the built-in renderer.
    #[serde(default)]
    pub custom_tool: Option<String>,
    /// Host-defined settings passed to the registered renderer and saved without interpretation.
    #[serde(default)]
    pub custom_settings: serde_json::Value,
    /// Type of drawing tool
    pub tool_type: DrawingToolType,
    /// Screen coordinates (updated each frame from chart_points)
    #[serde(skip)]
    pub points: Vec<egui::Pos2>,
    /// Persistent chart coordinates (bar_idx, price) - stable across pan/zoom
    #[serde(skip)]
    pub chart_points: Vec<ChartPoint>,
    /// Screen y of each visible level of a Fibonacci retracement or extension, in config order
    /// (empty for other tools); rebuilt with `points` by [`Self::update_screen_coords`]
    #[serde(skip)]
    pub level_ys: Vec<f32>,
    /// Position as fractions of the pane width and height for anchored text and notes.
    /// `None` adopts the first displayed position, including for older saved drawings.
    #[serde(default)]
    pub screen_anchor: Option<[f32; 2]>,
    /// Drawing color as RGBA
    pub color: [u8; 4],
    /// Stroke width in pixels
    pub stroke_width: f32,
    /// Whether the drawing is complete
    #[serde(skip)]
    pub completed: bool,
    /// Optional annotation content, preserved with the drawing. Tables use one row per line
    /// and tab-separated cells. Images use an image URI resolved by the host's egui image
    /// loaders. Posts and ideas use the first line as author or title and the rest as body.
    pub text: Option<String>,

    // === Styling properties ===
    /// Line style (Solid, Dashed, Dotted)
    pub line_style: LineStyle,
    /// Fill color for shapes - None means no fill
    pub fill_color: Option<[u8; 4]>,
    /// Whether the drawing is locked (prevents editing/moving)
    pub locked: bool,
    /// Whether the drawing is visible
    pub visible: bool,
    /// Font size for text annotations
    pub font_size: f32,
    /// Font weight for text annotations
    pub font_weight: FontWeight,
    /// Whether text annotations are rendered in italics
    #[serde(default)]
    pub font_italic: bool,
    /// Optional custom color for text annotations (None uses the drawing's primary color)
    #[serde(default)]
    pub text_color: Option<[u8; 4]>,
    /// Vertical placement of line/shape labels relative to the line (Top / Middle / Bottom)
    #[serde(default)]
    pub text_v_align: TextVerticalAlign,
    /// Horizontal placement of line/shape labels along the line (Left / Center / Right)
    #[serde(default)]
    pub text_h_align: TextHorizontalAlign,
    /// Extend line to left edge
    pub extend_left: bool,
    /// Extend line to right edge
    pub extend_right: bool,
    /// Arrow style at start of line
    pub arrow_start_style: ArrowStyle,
    /// Arrow style at end of line
    pub arrow_end_style: ArrowStyle,
    /// Per-timeframe visibility
    pub timeframe_visibility: TimeframeVisibility,

    // === Tool-specific configuration (None uses the defaults) ===
    /// Quantity of the long and short position tools
    pub position_size: Option<PositionSize>,
    /// Instrument economics for long and short position sizing. Older drawings use unit sizing.
    #[serde(default)]
    pub position_sizing: PositionSizing,
    /// Labels and zone colours of the long and short positions.
    #[serde(default)]
    pub position_display: Option<PositionDisplay>,
    /// Readouts of the measure and range tools.
    #[serde(default)]
    pub measure: Option<MeasureConfig>,
    /// Bands of the anchored VWAP.
    #[serde(default)]
    pub vwap: Option<VwapConfig>,
    /// Card colours of a forecast.
    #[serde(default)]
    pub forecast: Option<super::ForecastConfig>,
    /// Customizable Fibonacci levels for Fib tools
    pub fib_config: Option<FibonacciConfig>,
    /// Rows and value area of the volume profile tools
    pub volume_profile: Option<VolumeProfileConfig>,
    /// Band width of the regression trend tool
    pub regression_trend: Option<RegressionTrendConfig>,

    /// Price and time divisions of Gann boxes and squares.
    #[serde(default)]
    pub gann: Option<GannConfig>,
    /// Annotations shown by the pattern, Elliott wave and cycle tools.
    #[serde(default)]
    pub pattern: Option<PatternConfig>,
    /// Average range and noise, in ticks, of the candles a Ghost Feed projects.
    #[serde(default)]
    pub ghost_feed: Option<GhostFeedConfig>,
    /// Mode and copied bars of a Bars Pattern.
    #[serde(default)]
    pub bars_pattern: Option<super::BarsPatternConfig>,
    /// Whether a rectangle or parallel channel draws its dashed halfway line.
    #[serde(default)]
    pub channel_midline: bool,
    /// Whether Flat Top/Bottom and Disjoint Channel show a price label.
    #[serde(default)]
    pub show_price_labels: bool,
    /// Level lines and band fill of the pitchfork tools.
    #[serde(default)]
    pub pitchfork: Option<PitchforkConfig>,
    /// How far fair value gaps, order blocks and liquidity levels run, and how they look.
    #[serde(default)]
    pub price_action: Option<PriceActionConfig>,
    /// Circles of the Mach figures and density of the Dedekind tessellation.
    #[serde(default)]
    pub geometry: Option<GeometryConfig>,

    // === Z-Order (rendering layer) ===
    /// Z-order for rendering priority (higher = drawn on top)
    pub z_order: i32,
}

fn enabled() -> bool {
    true
}

/// The Sonic and Supersonic figures and their golden variants.
pub(crate) fn is_mach_figure(tool: DrawingToolType) -> bool {
    matches!(
        tool,
        DrawingToolType::Sonic
            | DrawingToolType::Supersonic
            | DrawingToolType::GoldenSonic
            | DrawingToolType::GoldenSupersonic
    )
}

impl Drawing {
    /// Create a new drawing with default properties
    pub fn new(id: usize, tool_type: DrawingToolType) -> Self {
        Self {
            id,
            stored_id: None,
            created_at: None,
            notes: None,
            label: None,
            selectable: true,
            saveable: true,
            listed: true,
            group: None,
            folder: Vec::new(),
            tool_type,
            custom_tool: None,
            custom_settings: serde_json::Value::Null,
            points: Vec::new(),
            chart_points: Vec::new(),
            level_ys: Vec::new(),
            screen_anchor: None,
            pane: None,
            color: [255, 255, 255, 255], // Dark-theme default; prefer `with_color` + theme
            stroke_width: if matches!(
                tool_type,
                DrawingToolType::GannFan
                    | DrawingToolType::GannBox
                    | DrawingToolType::GannSquare
                    | DrawingToolType::GannFixed
            ) {
                1.0
            } else {
                2.0
            },
            completed: false,
            text: (tool_type == DrawingToolType::Sticker).then(|| "sticker:bull".into()),
            // Styling defaults
            line_style: LineStyle::Solid,
            fill_color: match tool_type {
                DrawingToolType::XABCDPattern
                | DrawingToolType::CypherPattern
                | DrawingToolType::GartleyPattern
                | DrawingToolType::BatPattern
                | DrawingToolType::ButterflyPattern
                | DrawingToolType::CrabPattern
                | DrawingToolType::SharkPattern
                | DrawingToolType::ABCDPattern
                | DrawingToolType::TrianglePattern => Some([242, 54, 69, 25]),
                DrawingToolType::HeadAndShoulders
                | DrawingToolType::ThreeDrivesPattern
                | DrawingToolType::ElliottTriangle => Some([242, 54, 69, 20]),
                DrawingToolType::ElliottCorrection => Some(
                    crate::tokens::DESIGN_TOKENS
                        .semantic
                        .extended
                        .bearish
                        .gamma_multiply(20.0 / 255.0)
                        .to_srgba_unmultiplied(),
                ),
                DrawingToolType::ParallelChannel
                | DrawingToolType::FlatTopBottom
                | DrawingToolType::DisjointChannel => Some([255, 255, 255, 51]),
                _ => None,
            },
            locked: false,
            visible: true,
            font_size: if matches!(
                tool_type,
                DrawingToolType::FontIcon | DrawingToolType::Sticker
            ) {
                32.0
            } else {
                12.0
            },
            font_weight: FontWeight::Normal,
            font_italic: false,
            text_color: None,
            text_v_align: if matches!(
                tool_type,
                DrawingToolType::FlatTopBottom | DrawingToolType::DisjointChannel
            ) {
                TextVerticalAlign::Inside
            } else {
                TextVerticalAlign::Top
            },
            text_h_align: TextHorizontalAlign::Center,
            extend_left: tool_type == DrawingToolType::ExtendedLine,
            extend_right: matches!(
                tool_type,
                DrawingToolType::Ray
                    | DrawingToolType::ExtendedLine
                    | DrawingToolType::Pitchfork
                    | DrawingToolType::SchiffPitchfork
                    | DrawingToolType::ModifiedSchiffPitchfork
                    | DrawingToolType::InsidePitchfork
            ),
            arrow_start_style: ArrowStyle::None,
            arrow_end_style: if matches!(
                tool_type,
                DrawingToolType::Arrow | DrawingToolType::ArrowMarker | DrawingToolType::Path
            ) {
                ArrowStyle::Arrow
            } else {
                ArrowStyle::None
            },
            timeframe_visibility: TimeframeVisibility::all(),
            position_size: None,
            position_sizing: PositionSizing::default(),
            position_display: None,
            measure: None,
            vwap: None,
            forecast: None,
            fib_config: matches!(
                tool_type,
                DrawingToolType::FibonacciRetracement
                    | DrawingToolType::FibonacciExtension
                    | DrawingToolType::FibonacciChannel
                    | DrawingToolType::FibonacciWedge
                    | DrawingToolType::FibonacciTimeZones
                    | DrawingToolType::FibonacciSpeedFan
                    | DrawingToolType::TrendBasedFibTime
                    | DrawingToolType::FibonacciCircles
                    | DrawingToolType::FibonacciSpeedResistanceArcs
            )
            .then(|| FibonacciConfig::for_tool(tool_type)),
            volume_profile: None,
            regression_trend: (tool_type == DrawingToolType::RegressionTrend)
                .then(RegressionTrendConfig::default),
            pitchfork: (tool_type == DrawingToolType::Pitchfan).then(PitchforkConfig::pitchfan),
            channel_midline: matches!(tool_type, DrawingToolType::ParallelChannel),
            show_price_labels: matches!(
                tool_type,
                DrawingToolType::FlatTopBottom | DrawingToolType::DisjointChannel
            ),
            gann: None,
            pattern: None,
            price_action: None,
            geometry: None,
            ghost_feed: None,
            bars_pattern: None,
            // Z-order (default: 0, higher values render on top)
            z_order: 0,
        }
    }

    /// Creates a host-defined drawing. The manager looks up `tool` in its registry; the
    /// built-in polyline identity supplies anchor handles and multi-click input only.
    pub fn custom(tool: impl Into<String>) -> Self {
        let mut drawing = Self::new(0, DrawingToolType::Polyline);
        drawing.custom_tool = Some(tool.into());
        drawing
    }

    /// Create a new drawing with a specific color
    pub fn with_color(id: usize, tool_type: DrawingToolType, color: [u8; 4]) -> Self {
        let mut drawing = Self::new(id, tool_type);
        drawing.color = color;
        if tool_type != DrawingToolType::ElliottCorrection
            && let Some(fill) = &mut drawing.fill_color
        {
            fill[..3].copy_from_slice(&color[..3]);
        }
        drawing
    }

    /// Create a new text label drawing
    pub fn with_text(id: usize, text: String) -> Self {
        let mut drawing = Self::new(id, DrawingToolType::TextLabel);
        drawing.text = Some(text);
        drawing.color = [255, 255, 255, 255]; // White
        drawing.arrow_end_style = ArrowStyle::None;
        drawing
    }

    /// Update screen coordinates from chart_points using current transforms
    ///
    /// This should be called each frame before rendering.
    pub fn update_screen_coords<F, G>(&mut self, bar_to_x: F, price_to_y: G)
    where
        F: Fn(f32) -> f32,
        G: Fn(f64) -> f32,
    {
        self.points.clear();
        for cp in &self.chart_points {
            let x = bar_to_x(cp.bar_idx);
            let y = price_to_y(cp.price);
            self.points.push(egui::Pos2::new(x, y));
        }
        self.level_ys = self
            .fib_levels()
            .map(|(_, prices)| prices.into_iter().map(&price_to_y).collect())
            .unwrap_or_default();
    }

    /// Rebuilds anchored text and note coordinates relative to the pane. Call after
    /// `update_screen_coords`; other tools retain their chart coordinates. Older drawings
    /// adopt their first displayed position. Resizing preserves the width and height fractions.
    pub fn update_pane_coords(&mut self, pane: egui::Rect) {
        if !matches!(
            self.tool_type,
            DrawingToolType::AnchoredText | DrawingToolType::AnchoredNote
        ) || !pane.is_finite()
            || pane.width() <= 0.0
            || pane.height() <= 0.0
        {
            return;
        }
        if let Some(pos) = self.points.first_mut() {
            let anchor = self.screen_anchor.get_or_insert_with(|| {
                [
                    (pos.x - pane.left()) / pane.width(),
                    (pos.y - pane.top()) / pane.height(),
                ]
            });
            *pos = pane.min + egui::vec2(anchor[0] * pane.width(), anchor[1] * pane.height());
        }
    }

    /// The Fibonacci config in use and the price of each of its visible levels, for a retracement
    /// or an extension projected from its third anchor (the first anchor for legacy two-point drawings).
    pub(crate) fn fib_levels(&self) -> Option<(FibonacciConfig, Vec<f64>)> {
        let extension = match self.tool_type {
            DrawingToolType::FibonacciRetracement => false,
            DrawingToolType::FibonacciExtension => true,
            _ => return None,
        };
        let [start, end, ..] = &self.chart_points[..] else {
            return None;
        };
        let config = self.fib_config.clone().unwrap_or_else(|| {
            if extension {
                FibonacciConfig::extension_default()
            } else {
                FibonacciConfig::default()
            }
        });
        // A trend-based extension only projects once C is placed (or previewed); legacy
        // completed two-point drawings keep projecting from A.
        let origin = match self.chart_points.get(2) {
            Some(origin) => origin.price,
            None if extension && !self.completed => return None,
            None => start.price,
        };
        let prices = config
            .level_prices(start.price, end.price, extension, origin)
            .collect();
        Some((config, prices))
    }

    /// The price levels an alert can follow at the fractional bar index `bar`, each with its
    /// level number, name and price: a line's price extended through its two anchors, the base and
    /// second lines of parallel or disjoint channels, the trend and flat lines of a flat
    /// top/bottom, a horizontal level, a rectangle's top and bottom, a position's entry, target
    /// and stop, and each visible Fibonacci retracement or extension level, numbered by its place
    /// in the level settings. Sloping lines are straight on the price axis `scale` paints, so
    /// they are linear in price except on a logarithmic scale. Empty for other drawings, and for
    /// geometry with no price at `bar`, such as a vertical line or a sloping line through a
    /// non-positive price on a logarithmic scale.
    pub fn alert_levels(&self, bar: f64, scale: PriceScaleMode) -> Vec<(usize, String, f64)> {
        use DrawingToolType as T;
        let named = |names: &[&str], prices: &[f64]| {
            names
                .iter()
                .zip(prices)
                .enumerate()
                .map(|(level, (name, price))| (level, (*name).to_owned(), *price))
                .collect::<Vec<_>>()
        };
        let log = scale == PriceScaleMode::Logarithmic;
        let to_axis = |price: f64| if log { price.ln() } else { price };
        let from_axis = |value: f64| if log { value.exp() } else { value };
        // The line through `a` and `b` at `bar`, in axis coordinates.
        let along = |a: &ChartPoint, b: &ChartPoint, bar: f64| {
            let (from, to) = (to_axis(a.price), to_axis(b.price));
            from + (bar - f64::from(a.bar_idx)) * (to - from) / f64::from(b.bar_idx - a.bar_idx)
        };
        let levels = match (self.tool_type, &self.chart_points[..]) {
            (T::HorizontalLine | T::HorizontalRay, [level, ..]) => named(&["Line"], &[level.price]),
            (
                T::TrendLine | T::Ray | T::ExtendedLine | T::InfoLine | T::TrendAngle,
                [start, end, ..],
            ) => named(&["Line"], &[from_axis(along(start, end, bar))]),
            // The second line is the base line moved straight up or down through the third anchor.
            (T::ParallelChannel, [start, end, through, ..]) => {
                let shift = to_axis(through.price) - along(start, end, f64::from(through.bar_idx));
                named(
                    &["Line", "Parallel line"],
                    &[
                        from_axis(along(start, end, bar)),
                        from_axis(along(start, end, bar) + shift),
                    ],
                )
            }
            (T::FlatTopBottom, [start, end, flat, ..]) => named(
                &["Line", "Flat line"],
                &[from_axis(along(start, end, bar)), flat.price],
            ),
            // The second line passes through the third anchor with the opposite slope.
            (T::DisjointChannel, [start, end, from, ..]) => {
                let rise = to_axis(end.price) - to_axis(start.price);
                let run = f64::from(end.bar_idx - start.bar_idx);
                named(
                    &["Line", "Second line"],
                    &[
                        from_axis(along(start, end, bar)),
                        from_axis(
                            to_axis(from.price) - (bar - f64::from(from.bar_idx)) * rise / run,
                        ),
                    ],
                )
            }
            (T::Rect, [a, b, ..]) => named(
                &["Top", "Bottom"],
                &[a.price.max(b.price), a.price.min(b.price)],
            ),
            (T::LongPos | T::ShortPos, [entry, target, rest @ ..]) => named(
                &["Entry", "Target", "Stop"],
                &[
                    entry.price,
                    target.price,
                    // Until the stop is placed it mirrors the target around the entry.
                    rest.first()
                        .map_or(2.0 * entry.price - target.price, |stop| stop.price),
                ],
            ),
            _ => self
                .fib_levels()
                .map(|(config, prices)| {
                    config
                        .levels
                        .iter()
                        .enumerate()
                        .filter(|(_, level)| level.visible)
                        .zip(prices)
                        .map(|((index, level), price)| {
                            (index, level.label(config.levels_as_percent), price)
                        })
                        .collect()
                })
                .unwrap_or_default(),
        };
        if levels.iter().all(|(_, _, price)| price.is_finite()) {
            levels
        } else {
            Vec::new()
        }
    }

    /// The alert source of level `level` of [`Self::alert_levels`]: the drawing's stored id, or
    /// its id when it was never stored, followed by `#` and the level for levels after the
    /// first. `%` and `#` in the id are escaped as `%25` and `%23` so distinct drawings cannot
    /// share a source. It names the same level after the drawing is saved and restored.
    pub fn alert_source(&self, level: usize) -> AlertSource {
        let id = self.stable_id();
        AlertSource::Drawing(if level == 0 {
            id.into_owned()
        } else {
            format!("{id}#{level}")
        })
    }

    /// The level of this drawing that `source`, from [`Self::alert_source`], names.
    pub fn alert_level(&self, source: &AlertSource) -> Option<usize> {
        let AlertSource::Drawing(source) = source else {
            return None;
        };
        let id = self.stable_id();
        if *source == id {
            return Some(0);
        }
        source
            .strip_prefix(id.as_ref())?
            .strip_prefix('#')?
            .parse()
            .ok()
            .filter(|level| *level > 0)
    }

    fn stable_id(&self) -> std::borrow::Cow<'_, str> {
        let id: std::borrow::Cow<'_, str> = self
            .stored_id
            .as_deref()
            .map_or_else(|| self.id.to_string().into(), Into::into);
        if id.contains('%') || id.contains('#') {
            id.replace('%', "%25").replace('#', "%23").into()
        } else {
            id
        }
    }

    /// Add a point with both screen and chart coordinates
    pub fn add_point_with_chart_coords<F, G>(
        &mut self,
        screen_point: egui::Pos2,
        x_to_bar: F,
        y_to_price: G,
    ) where
        F: Fn(f32) -> f32,
        G: Fn(f32) -> f64,
    {
        let bar_idx = x_to_bar(screen_point.x);
        let price = y_to_price(screen_point.y);
        self.chart_points.push(ChartPoint::new(bar_idx, price));
        self.points.push(screen_point);
        self.check_completion();
    }

    /// Update the last point with both screen and chart coordinates
    pub fn update_last_point_with_chart_coords<F, G>(
        &mut self,
        screen_point: egui::Pos2,
        x_to_bar: F,
        y_to_price: G,
    ) where
        F: Fn(f32) -> f32,
        G: Fn(f32) -> f64,
    {
        if let (Some(last_point), Some(last_chart_point)) =
            (self.points.last_mut(), self.chart_points.last_mut())
        {
            *last_point = screen_point;
            *last_chart_point =
                ChartPoint::new(x_to_bar(screen_point.x), y_to_price(screen_point.y));
        }
    }

    /// Puts a Time cycles second anchor on the first anchor's price, so the second only sets
    /// the period.
    pub(crate) fn level_time_cycles(&mut self) {
        if self.tool_type == DrawingToolType::TimeCycles
            && self.points.len() >= 2
            && self.chart_points.len() >= 2
        {
            self.points[1].y = self.points[0].y;
            self.chart_points[1].price = self.chart_points[0].price;
        }
    }

    /// Add a screen point (legacy method)
    pub fn add_point(&mut self, point: egui::Pos2) {
        self.points.push(point);
        self.check_completion();
    }

    /// Shift the bar indices at or after `from` by a delta (used when bars are inserted in front
    /// of them)
    pub fn shift_bar_indices(&mut self, from: f32, delta: f32) {
        for cp in &mut self.chart_points {
            if cp.bar_idx >= from {
                cp.shift_bar_idx(delta);
            }
        }
    }

    /// Check and set completion status based on tool type
    fn check_completion(&mut self) {
        if let Some(required) = self.tool_type.required_points()
            && self.points.len() >= required
        {
            self.completed = true;
        }
    }

    /// Get the egui Color32 from the RGBA array
    #[inline]
    pub fn color32(&self) -> egui::Color32 {
        egui::Color32::from_rgba_unmultiplied(
            self.color[0],
            self.color[1],
            self.color[2],
            self.color[3],
        )
    }

    /// Set the primary stroke colour and, for Fibonacci tools, every level colour.
    ///
    /// The object tree's single swatch uses this so one pick drives the painted levels
    /// (same effect as Inputs → Use one color) and tints the fill to match.
    pub fn apply_primary_color(&mut self, color: [u8; 4]) {
        self.apply_line_color(color);
        if self.tool_type != DrawingToolType::ElliottCorrection
            && let Some(fill) = &mut self.fill_color
        {
            fill[..3].copy_from_slice(&color[..3]);
        }
    }

    /// Like [`Self::apply_primary_color`], but leaves the fill alone, for a picker that
    /// shows the fill as its own swatch.
    pub fn apply_line_color(&mut self, color: [u8; 4]) {
        self.color = color;
        if let Some(config) = &mut self.fib_config {
            for level in &mut config.levels {
                level.color = color;
            }
            config.trend_line_color = color;
        }
        if self.tool_type == DrawingToolType::Pitchfan
            && let Some(config) = &mut self.pitchfork
        {
            config.recolor_pitchfan(color);
        } else if let Some(config) = &mut self.pitchfork {
            for level in &mut config.levels {
                level.color = color;
            }
        }
        // Per-level colours keep their own opacity; only the hue follows the pick.
        let tint = |target: &mut [u8; 4]| target[..3].copy_from_slice(&color[..3]);
        if let Some(config) = self.gann_levels_mut() {
            for level in config
                .levels
                .iter_mut()
                .chain(&mut config.fan_lines)
                .chain(&mut config.arcs)
                .chain(&mut config.fan_angles)
            {
                tint(&mut level.color);
            }
        }
        if let Some(config) = self.regression_mut() {
            for target in [
                &mut config.midline_color,
                &mut config.up_color,
                &mut config.down_color,
            ] {
                tint(target);
            }
        }
        if matches!(
            self.tool_type,
            DrawingToolType::FairValueGap | DrawingToolType::OrderBlock
        ) {
            let mut config = self.price_action_config();
            tint(&mut config.up_color);
            tint(&mut config.down_color);
            self.price_action = Some(config);
        }
        if is_mach_figure(self.tool_type) {
            let mut config = self.geometry_config();
            for level in &mut config.levels {
                tint(&mut level.color);
            }
            self.geometry = Some(config);
        }
    }

    /// Saved price action options, or the tool's defaults.
    pub fn price_action_config(&self) -> PriceActionConfig {
        self.price_action
            .unwrap_or_else(|| PriceActionConfig::for_tool(self.tool_type))
    }

    /// Saved geometry options, or the tool's defaults.
    pub fn geometry_config(&self) -> GeometryConfig {
        self.geometry
            .clone()
            .unwrap_or_else(|| GeometryConfig::for_tool(self.tool_type))
    }

    /// Set the stroke width of every line the drawing paints, including Fibonacci levels and
    /// pitchfork levels that carried their own width.
    pub fn apply_line_width(&mut self, width: f32) {
        self.stroke_width = width;
        if let Some(config) = &mut self.fib_config {
            config.levels_line_width = width;
            config.trend_line_width = width;
        }
        if self.tool_type != DrawingToolType::Pitchfan
            && let Some(config) = &mut self.pitchfork
        {
            for level in &mut config.levels {
                level.width = None;
            }
        }
    }

    /// Set the dash pattern of every line the drawing paints, including Fibonacci, pitchfork
    /// and regression lines that carry their own style.
    pub fn apply_line_style(&mut self, style: LineStyle) {
        self.line_style = style;
        if let Some(config) = &mut self.fib_config {
            config.levels_line_style = style;
            config.trend_line_style = style;
        }
        if let Some(config) = self.pitchfork_levels_mut() {
            for level in &mut config.levels {
                level.line_style = style;
            }
        }
        if let Some(config) = self.regression_mut() {
            config.midline_style = style;
            config.upper_style = style;
            config.lower_style = style;
        }
    }

    /// The Gann levels to edit, created from the tool's defaults when not yet saved.
    fn gann_levels_mut(&mut self) -> Option<&mut GannConfig> {
        matches!(
            self.tool_type,
            DrawingToolType::GannBox
                | DrawingToolType::GannSquare
                | DrawingToolType::GannFixed
                | DrawingToolType::GannFan
        )
        .then(|| {
            let tool = self.tool_type;
            self.gann.get_or_insert_with(|| GannConfig::for_tool(tool))
        })
    }

    /// The regression settings to edit, created from the defaults when not yet saved.
    fn regression_mut(&mut self) -> Option<&mut RegressionTrendConfig> {
        (self.tool_type == DrawingToolType::RegressionTrend)
            .then(|| self.regression_trend.get_or_insert_with(Default::default))
    }

    /// The pitchfork levels to edit. A pitchfork without saved levels paints every default
    /// level in its line colour, which the created levels keep.
    fn pitchfork_levels_mut(&mut self) -> Option<&mut PitchforkConfig> {
        if !matches!(
            self.tool_type,
            DrawingToolType::Pitchfork
                | DrawingToolType::SchiffPitchfork
                | DrawingToolType::ModifiedSchiffPitchfork
                | DrawingToolType::InsidePitchfork
                | DrawingToolType::Pitchfan
        ) {
            return None;
        }
        if self.tool_type == DrawingToolType::Pitchfan {
            return Some(self.pitchfork.get_or_insert_with(PitchforkConfig::pitchfan));
        }
        let color = self.color;
        Some(self.pitchfork.get_or_insert_with(|| {
            let mut config = PitchforkConfig::default();
            for level in &mut config.levels {
                level.color = color;
            }
            config
        }))
    }

    /// Get the fill color as Color32, if set
    #[inline]
    pub fn fill_color32(&self) -> Option<egui::Color32> {
        self.fill_color
            .map(|c| egui::Color32::from_rgba_unmultiplied(c[0], c[1], c[2], c[3]))
    }

    /// Fill the tool paints while none is set, or `None` for tools that start unfilled.
    pub fn default_fill(&self) -> Option<egui::Color32> {
        let base = match self.tool_type {
            DrawingToolType::Measure => self.measure_color(),
            DrawingToolType::PriceRange
            | DrawingToolType::DateRange
            | DrawingToolType::DateAndPriceRange => self.color32(),
            DrawingToolType::ProjectionTool => return Some(self.color32().gamma_multiply(0.2)),
            _ => return None,
        };
        Some(egui::Color32::from_rgba_unmultiplied(
            base.r(),
            base.g(),
            base.b(),
            30,
        ))
    }

    /// The fill this drawing paints: the one set, or the tool's default. A fully transparent
    /// fill set on a tool with a default turns it off.
    pub fn painted_fill(&self) -> Option<egui::Color32> {
        match self.fill_color {
            Some([_, _, _, 0]) => None,
            Some(_) => self.fill_color32(),
            None => self.default_fill(),
        }
    }

    /// Sets the fill to paint, storing nothing while it is the tool's default and a fully
    /// transparent fill when a tool with a default should paint none.
    pub fn set_painted_fill(&mut self, fill: Option<egui::Color32>) {
        let default = self.default_fill();
        self.fill_color = match fill {
            Some(color) if Some(color) == default => None,
            Some(color) => Some(color.to_srgba_unmultiplied()),
            None if default.is_some() => Some([0; 4]),
            None => None,
        };
    }

    /// Line colour of a Measure: the set colour, or red measuring down and blue up while it is
    /// the default drawing colour.
    pub fn measure_color(&self) -> egui::Color32 {
        let default = [
            crate::drawings::manager::DEFAULT_DRAWING_COLOR_DARK,
            crate::drawings::manager::DEFAULT_DRAWING_COLOR_LIGHT,
        ]
        .contains(&self.color);
        let down = matches!(self.chart_points.as_slice(), [a, b, ..] if b.price < a.price);
        if !default {
            self.color32().to_opaque()
        } else if down {
            crate::tokens::DESIGN_TOKENS.semantic.extended.bearish
        } else {
            crate::tokens::DESIGN_TOKENS.semantic.extended.accent
        }
    }

    /// Get the text color as Color32, falling back to the primary drawing color when unset
    #[inline]
    pub fn text_color32(&self) -> egui::Color32 {
        self.text_color
            .map(|c| egui::Color32::from_rgba_unmultiplied(c[0], c[1], c[2], c[3]))
            .unwrap_or_else(|| self.color32())
    }

    /// Get the stroke for rendering
    #[inline]
    pub fn stroke(&self) -> egui::Stroke {
        egui::Stroke::new(self.stroke_width, self.color32())
    }

    /// Check if this drawing is visible on the given timeframe
    #[inline]
    pub fn is_visible_on_timeframe(&self, timeframe: &str) -> bool {
        self.visible && self.timeframe_visibility.is_visible_on(timeframe)
    }

    /// Get the z-order of this drawing
    #[inline]
    pub fn z_order(&self) -> i32 {
        self.z_order
    }

    /// Set the z-order of this drawing
    #[inline]
    pub fn set_z_order(&mut self, order: i32) {
        self.z_order = order;
    }

    /// Get the bounding rect of the drawing's points
    pub fn bounding_rect(&self) -> Option<egui::Rect> {
        if self.points.is_empty() {
            return None;
        }

        let mut min_x = f32::MAX;
        let mut min_y = f32::MAX;
        let mut max_x = f32::MIN;
        let mut max_y = f32::MIN;

        for p in &self.points {
            min_x = min_x.min(p.x);
            min_y = min_y.min(p.y);
            max_x = max_x.max(p.x);
            max_y = max_y.max(p.y);
        }

        Some(egui::Rect::from_min_max(
            egui::pos2(min_x, min_y),
            egui::pos2(max_x, max_y),
        ))
    }

    /// Quadratic bezier curve interpolation
    ///
    /// Returns a point on the bezier curve defined by p0, p1, p2 at parameter t.
    /// Used for smooth curve rendering (e.g., arcs, spirals).
    pub fn quadratic_bezier(p0: egui::Pos2, p1: egui::Pos2, p2: egui::Pos2, t: f32) -> egui::Pos2 {
        let u = 1.0 - t;
        egui::Pos2::new(
            u * u * p0.x + 2.0 * u * t * p1.x + t * t * p2.x,
            u * u * p0.y + 2.0 * u * t * p1.y + t * t * p2.y,
        )
    }
}

impl Default for Drawing {
    fn default() -> Self {
        Self::new(0, DrawingToolType::TrendLine)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_shift_bar_indices() {
        let mut drawing = Drawing::new(1, DrawingToolType::TrendLine);
        drawing.chart_points.push(ChartPoint::new(100.0, 150.0));
        drawing.chart_points.push(ChartPoint::new(110.0, 160.0));

        drawing.shift_bar_indices(105.0, 50.0);

        assert_eq!(
            drawing.chart_points,
            [ChartPoint::new(100.0, 150.0), ChartPoint::new(160.0, 160.0)]
        );
    }

    /// A drawing completes once it holds its tool's required number of
    /// anchors, and each anchor added from the screen gets its chart
    /// coordinates.
    #[test]
    fn test_completion() {
        let x_to_bar = |x: f32| x / 10.0;
        let y_to_price = |y: f32| 1000.0 - y as f64;
        let mut drawing = Drawing::new(1, DrawingToolType::TrendLine);

        drawing.add_point_with_chart_coords(egui::pos2(100.0, 300.0), x_to_bar, y_to_price);
        assert!(!drawing.completed);

        drawing.add_point_with_chart_coords(egui::pos2(200.0, 50.0), x_to_bar, y_to_price);
        assert!(drawing.completed); // TrendLine requires 2 points
        assert_eq!(
            drawing.chart_points,
            [ChartPoint::new(10.0, 700.0), ChartPoint::new(20.0, 950.0)]
        );
    }

    /// Alerts follow each level the drawing shows at the evaluated bar, under a source that
    /// names the same level once the drawing is stored.
    #[test]
    fn alert_levels_follow_the_drawing_geometry() {
        use DrawingToolType as T;
        use PriceScaleMode::{Logarithmic, Normal as Linear};
        let drawing = |tool, points: &[(f32, f64)]| {
            let mut drawing = Drawing::new(7, tool);
            drawing.chart_points = points.iter().map(|&(b, p)| ChartPoint::new(b, p)).collect();
            drawing
        };
        let mut fib = drawing(T::FibonacciRetracement, &[(0.0, 100.0), (10.0, 200.0)]);
        let mut config = FibonacciConfig::default();
        // Numeric-looking custom labels are ignored; empty labels derive from the ratio.
        config.levels[2].label = "100%".into();
        config.levels[3].label.clear();
        // An extension level turned on past the off 0.886..1.414 levels keeps its index.
        config.levels[11].visible = true;
        fib.fib_config = Some(config);
        // (drawing, bar) -> (level, name, price); prices by hand.
        for (drawing, bar, scale, expected) in [
            // 100 + (25 - 10) * (120 - 100) / (20 - 10).
            (
                drawing(T::TrendLine, &[(10.0, 100.0), (20.0, 120.0)]),
                25.0,
                Linear,
                vec![(0, "Line", 130.0)],
            ),
            (
                drawing(T::TrendLine, &[(10.0, 100.0), (10.0, 120.0)]),
                25.0,
                Linear,
                vec![],
            ),
            (
                drawing(T::HorizontalRay, &[(3.0, 50.0)]),
                9.0,
                Linear,
                vec![(0, "Line", 50.0)],
            ),
            (
                drawing(T::Rect, &[(5.0, 90.0), (8.0, 110.0)]),
                9.0,
                Linear,
                vec![(0, "Top", 110.0), (1, "Bottom", 90.0)],
            ),
            (
                drawing(T::LongPos, &[(5.0, 100.0), (9.0, 110.0)]),
                9.0,
                Linear,
                vec![(0, "Entry", 100.0), (1, "Target", 110.0), (2, "Stop", 90.0)],
            ),
            (
                drawing(T::ShortPos, &[(5.0, 100.0), (9.0, 90.0), (9.0, 104.0)]),
                9.0,
                Linear,
                vec![(0, "Entry", 100.0), (1, "Target", 90.0), (2, "Stop", 104.0)],
            ),
            // Level 0 is on the second anchor; off levels keep their indices in settings.
            (
                fib,
                9.0,
                Linear,
                vec![
                    (0, "0", 200.0),
                    (1, "0.236", 176.4),
                    (2, "0.382", 161.8),
                    (3, "0.5", 150.0),
                    (4, "0.618", 138.2),
                    (6, "0.786", 121.4),
                    (8, "1", 100.0),
                    (11, "1.618", 38.2),
                ],
            ),
            // Straight on the scale: the geometric mean of 100 and 400 halfway on a log scale.
            (
                drawing(T::TrendLine, &[(10.0, 100.0), (20.0, 400.0)]),
                15.0,
                Logarithmic,
                vec![(0, "Line", 200.0)],
            ),
            (
                drawing(T::TrendLine, &[(10.0, 100.0), (20.0, 400.0)]),
                15.0,
                Linear,
                vec![(0, "Line", 250.0)],
            ),
            (
                drawing(T::TrendLine, &[(10.0, 0.0), (20.0, 10.0)]),
                15.0,
                Logarithmic,
                vec![],
            ),
            // The base line is 110 at the third anchor's bar 15, 20 below it: 130 + 20 at bar 25.
            // On a log scale the third anchor is twice the base line there, so the parallel is
            // twice as high at every bar: 400 and 800 at bar 20.
            (
                drawing(
                    T::ParallelChannel,
                    &[(10.0, 100.0), (20.0, 120.0), (15.0, 130.0)],
                ),
                25.0,
                Linear,
                vec![(0, "Line", 130.0), (1, "Parallel line", 150.0)],
            ),
            (
                drawing(
                    T::ParallelChannel,
                    &[(10.0, 100.0), (20.0, 400.0), (15.0, 400.0)],
                ),
                20.0,
                Logarithmic,
                vec![(0, "Line", 400.0), (1, "Parallel line", 800.0)],
            ),
            (
                drawing(
                    T::ParallelChannel,
                    &[(10.0, 100.0), (10.0, 120.0), (15.0, 130.0)],
                ),
                25.0,
                Linear,
                vec![],
            ),
            (
                drawing(
                    T::FlatTopBottom,
                    &[(10.0, 100.0), (20.0, 120.0), (15.0, 90.0)],
                ),
                25.0,
                Linear,
                vec![(0, "Line", 130.0), (1, "Flat line", 90.0)],
            ),
            // Through 50 at bar 12, the mirrored line falls 2 per bar: 24 at bar 25.
            // In log price it divides by 4 over 10 bars: 12.5 at bar 22.
            (
                drawing(
                    T::DisjointChannel,
                    &[(10.0, 100.0), (20.0, 120.0), (12.0, 50.0)],
                ),
                25.0,
                Linear,
                vec![(0, "Line", 130.0), (1, "Second line", 24.0)],
            ),
            (
                drawing(
                    T::DisjointChannel,
                    &[(10.0, 100.0), (20.0, 400.0), (12.0, 50.0)],
                ),
                22.0,
                Logarithmic,
                vec![
                    (0, "Line", 100.0 * 4f64.powf(1.2)),
                    (1, "Second line", 12.5),
                ],
            ),
            (
                drawing(T::Brush, &[(1.0, 1.0), (2.0, 2.0)]),
                9.0,
                Linear,
                vec![],
            ),
        ] {
            let levels = drawing.alert_levels(bar, scale);
            assert_eq!(levels.len(), expected.len(), "{:?}", drawing.tool_type);
            for ((level, name, price), (want_level, want_name, want_price)) in
                levels.into_iter().zip(expected)
            {
                assert_eq!((level, name.as_str()), (want_level, want_name));
                // Fibonacci ratios are stored as `f32`, good to about 1e-5 at these prices.
                assert!((price - want_price).abs() < 1e-4, "{name}: {price}");
            }
        }

        let mut line = drawing(T::TrendLine, &[(10.0, 100.0), (20.0, 120.0)]);
        for (stored, level, source) in [
            (None, 0, "7"),
            (None, 2, "7#2"),
            (Some("a%231"), 0, "a%25231"),
            (Some("a#1"), 3, "a%231#3"),
        ] {
            line.stored_id = stored.map(Into::into);
            assert_eq!(
                line.alert_source(level),
                AlertSource::Drawing(source.into())
            );
            assert_eq!(line.alert_level(&line.alert_source(level)), Some(level));
        }
        for other in ["a", "a#1#0", "a#1#x", "7"] {
            assert_eq!(line.alert_level(&AlertSource::Drawing(other.into())), None);
        }
        assert_eq!(line.alert_level(&AlertSource::Price), None);

        // A stored id containing the level separator must not alias another drawing's level.
        let mut other = line.clone();
        other.stored_id = Some("a".into());
        assert_ne!(line.alert_source(0), other.alert_source(1));
    }

    /// Toolbar line picks reach the lines each tool family paints from its own settings, and
    /// leave the fill alone.
    #[test]
    fn line_color_width_and_style_reach_per_level_lines() {
        use DrawingToolType as T;
        let red = [200, 10, 20, 255];
        for tool in [
            T::FibonacciRetracement,
            T::Pitchfork,
            T::Pitchfan,
            T::GannBox,
            T::GannFan,
            T::RegressionTrend,
            T::ParallelChannel,
        ] {
            let mut drawing = Drawing::new(1, tool);
            let fill = drawing.fill_color;
            drawing.apply_line_color(red);
            drawing.apply_line_width(3.0);
            drawing.apply_line_style(LineStyle::Dashed);
            assert_eq!(drawing.color, red, "{tool:?}");
            assert_eq!(drawing.fill_color, fill, "{tool:?} fill");
            assert_eq!(
                (drawing.stroke_width, drawing.line_style),
                (3.0, LineStyle::Dashed)
            );
            if let Some(config) = &drawing.fib_config {
                assert!(config.levels.iter().all(|l| l.color == red));
                assert_eq!(config.trend_line_color, red);
                assert_eq!(config.levels_line_width, 3.0);
                assert_eq!(config.levels_line_style, LineStyle::Dashed);
            }
            if let Some(config) = &drawing.pitchfork {
                assert!(config.levels.iter().all(|l| l.color[..3] == red[..3]));
                assert!(
                    config
                        .levels
                        .iter()
                        .all(|l| l.line_style == LineStyle::Dashed)
                );
            }
            if let Some(config) = &drawing.gann {
                assert!(
                    config
                        .levels
                        .iter()
                        .chain(&config.fan_angles)
                        .all(|l| l.color[..3] == red[..3])
                );
            }
            if let Some(config) = &drawing.regression_trend {
                assert_eq!(config.midline_color[..3], red[..3]);
                assert_eq!(config.upper_style, LineStyle::Dashed);
            }
            let family = match tool {
                T::FibonacciRetracement => drawing.fib_config.is_some(),
                T::Pitchfork | T::Pitchfan => drawing.pitchfork.is_some(),
                T::GannBox | T::GannFan => drawing.gann.is_some(),
                T::RegressionTrend => drawing.regression_trend.is_some(),
                _ => true,
            };
            assert!(family, "{tool:?} settings to edit");
        }
    }

    #[test]
    fn test_bounding_rect() {
        let mut drawing = Drawing::new(1, DrawingToolType::TrendLine);
        assert!(drawing.bounding_rect().is_none());

        drawing.points.push(egui::pos2(10.0, 20.0));
        drawing.points.push(egui::pos2(30.0, 50.0));

        let rect = drawing.bounding_rect().unwrap();
        assert_eq!(rect.min, egui::pos2(10.0, 20.0));
        assert_eq!(rect.max, egui::pos2(30.0, 50.0));
    }
}
