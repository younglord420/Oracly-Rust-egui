//! Embedded SVG icons for chart controls and host UI.
//!
//! Use [`icons`] for built-in glyphs or [`Icon`] and [`crate::icon_from_path`] to embed
//! an application SVG. The chart installs the icon image loaders when shown.
//!
//! ```
//! use egui_charts::icons::icons;
//!
//! # egui::__run_test_ui(|ui| {
//! ui.add(icons::SETTINGS.as_image(egui::Vec2::splat(20.0)));
//! # });
//! ```

mod icon;
mod loader;

pub use icon::Icon;
pub(crate) use loader::install as install_loader;

/// Macro to embed an SVG icon at compile time.
///
/// The path is relative to the `assets/icons/` directory of the crate that invokes the macro.
///
/// # Example
///
/// ```
/// use egui_charts::icon_from_path;
/// use egui_charts::icons::Icon;
///
/// const ERASER: Icon = icon_from_path!("lines/eraser.svg");
/// ```
#[macro_export]
macro_rules! icon_from_path {
    ($path:literal) => {
        $crate::icons::Icon::new(
            $path,
            include_bytes!(concat!(env!("CARGO_MANIFEST_DIR"), "/assets/icons/", $path)),
        )
    };
}

/// Pre-defined icon constants - all icons embedded at compile time.
pub mod icons {
    use super::Icon;

    // Cursors
    /// Crosshair SVG icon.
    pub const CROSSHAIR: Icon = crate::icon_from_path!("lines/cursor-cross.svg");
    /// Dot SVG icon.
    pub const DOT: Icon = crate::icon_from_path!("lines/cursor-dot.svg");
    /// Arrow SVG icon.
    pub const ARROW: Icon = crate::icon_from_path!("lines/cursor-arrow.svg");
    /// Pointer with a halo for presenting a position on the chart.
    pub const DEMONSTRATION: Icon = crate::icon_from_path!("lines/cursor-demonstration.svg");
    /// Wand with sparks for the trail that follows the pointer while the button is held.
    pub const MAGIC_CURSOR: Icon = crate::icon_from_path!("lines/cursor-magic.svg");
    /// Eraser SVG icon.
    pub const ERASER: Icon = crate::icon_from_path!("lines/eraser.svg");

    // Lines and channels
    /// Trend line SVG icon.
    pub const TREND_LINE: Icon = crate::icon_from_path!("lines/trend-line.svg");
    /// Ray SVG icon.
    pub const RAY: Icon = crate::icon_from_path!("lines/ray.svg");
    /// Info line SVG icon.
    pub const INFO_LINE: Icon = crate::icon_from_path!("lines/info-line.svg");
    /// Extended SVG icon.
    pub const EXTENDED: Icon = crate::icon_from_path!("lines/extended-line.svg");
    /// Trend angle SVG icon.
    pub const TREND_ANGLE: Icon = crate::icon_from_path!("lines/trend-angle.svg");
    /// Horizontal line SVG icon.
    pub const HORIZONTAL_LINE: Icon = crate::icon_from_path!("lines/horizontal-line.svg");
    /// Horizontal ray SVG icon.
    pub const HORIZONTAL_RAY: Icon = crate::icon_from_path!("lines/horizontal-ray.svg");
    /// Vertical line SVG icon.
    pub const VERTICAL_LINE: Icon = crate::icon_from_path!("lines/vertical-line.svg");
    /// Cross line SVG icon.
    pub const CROSS_LINE: Icon = crate::icon_from_path!("lines/cross-line.svg");
    /// Parallel channel SVG icon.
    pub const PARALLEL_CHANNEL: Icon = crate::icon_from_path!("lines/parallel-channel.svg");
    /// Regression trend SVG icon.
    pub const REGRESSION_TREND: Icon = crate::icon_from_path!("lines/regression-trend.svg");
    /// Flat top or bottom channel SVG icon.
    pub const FLAT_BOTTOM: Icon = crate::icon_from_path!("lines/flat-top-bottom.svg");
    /// Disjoint angle SVG icon.
    pub const DISJOINT_ANGLE: Icon = crate::icon_from_path!("lines/disjoint-channel.svg");

    // Pitchforks and Gann
    /// Pitchfork SVG icon.
    pub const PITCHFORK: Icon = crate::icon_from_path!("pitchforks-gann/pitchfork.svg");
    /// Schiff pitchfork SVG icon.
    pub const SCHIFF_PITCHFORK: Icon =
        crate::icon_from_path!("pitchforks-gann/schiff-pitchfork.svg");
    /// Modified Schiff pitchfork SVG icon.
    pub const SCHIFF_PITCHFORK2: Icon =
        crate::icon_from_path!("pitchforks-gann/modified-schiff-pitchfork.svg");
    /// Inside pitchfork SVG icon.
    pub const INSIDE_PITCHFORK: Icon =
        crate::icon_from_path!("pitchforks-gann/inside-pitchfork.svg");
    /// Pitchfan SVG icon.
    pub const PITCHFAN: Icon = crate::icon_from_path!("pitchforks-gann/pitchfan.svg");
    /// Gann box SVG icon.
    pub const GANN_SQUARE: Icon = crate::icon_from_path!("pitchforks-gann/gann-box.svg");
    /// Gann square SVG icon.
    pub const GANN_COMPLEX: Icon = crate::icon_from_path!("pitchforks-gann/gann-square.svg");
    /// Fixed Gann square SVG icon.
    pub const GANN_FIXED: Icon = crate::icon_from_path!("pitchforks-gann/gann-square-fixed.svg");
    /// Gann fan SVG icon.
    pub const GANN_FAN: Icon = crate::icon_from_path!("pitchforks-gann/gann-fan.svg");

    // Price action
    /// Fair value gap SVG icon.
    pub const FAIR_VALUE_GAP: Icon = crate::icon_from_path!("price-action/fair-value-gap.svg");
    /// Order block SVG icon.
    pub const ORDER_BLOCK: Icon = crate::icon_from_path!("price-action/order-block.svg");
    /// Liquidity level SVG icon.
    pub const LIQUIDITY_LEVEL: Icon = crate::icon_from_path!("price-action/liquidity.svg");

    // Geometry
    /// Dedekind tessellation SVG icon.
    pub const DEDEKIND: Icon = crate::icon_from_path!("geometry/dedekind.svg");
    /// Sonic SVG icon.
    pub const SONIC: Icon = crate::icon_from_path!("geometry/sonic.svg");
    /// Supersonic SVG icon.
    pub const SUPERSONIC: Icon = crate::icon_from_path!("geometry/supersonic.svg");
    /// Golden sonic SVG icon.
    pub const GOLDEN_SONIC: Icon = crate::icon_from_path!("geometry/golden-sonic.svg");
    /// Golden supersonic SVG icon.
    pub const GOLDEN_SUPERSONIC: Icon = crate::icon_from_path!("geometry/golden-supersonic.svg");

    // Fibonacci
    /// Fibonacci retracement SVG icon.
    pub const FIB_RETRACEMENT: Icon = crate::icon_from_path!("fibonacci/fib-retracement.svg");
    /// Fibonacci extension SVG icon.
    pub const FIB_EXTENSION: Icon = crate::icon_from_path!("fibonacci/fib-extension.svg");
    /// Fibonacci channel SVG icon.
    pub const FIB_CHANNEL: Icon = crate::icon_from_path!("fibonacci/fib-channel.svg");
    /// Fibonacci time zone SVG icon.
    pub const FIB_TIME_ZONE: Icon = crate::icon_from_path!("fibonacci/fib-time-zone.svg");
    /// Fibonacci speed resistance fan SVG icon.
    pub const FIB_SPEED_RESISTANCE_FAN: Icon =
        crate::icon_from_path!("fibonacci/fib-speed-fan.svg");
    /// Trend based fibonacci time SVG icon.
    pub const TREND_BASED_FIB_TIME: Icon =
        crate::icon_from_path!("fibonacci/trend-based-fib-time.svg");
    /// Fibonacci circles SVG icon.
    pub const FIB_CIRCLES: Icon = crate::icon_from_path!("fibonacci/fib-circles.svg");
    /// Fibonacci spiral SVG icon.
    pub const FIB_SPIRAL: Icon = crate::icon_from_path!("fibonacci/fib-spiral.svg");
    /// Fibonacci speed resistance arcs SVG icon.
    pub const FIB_SPEED_RESISTANCE_ARCS: Icon =
        crate::icon_from_path!("fibonacci/fib-speed-arcs.svg");
    /// Fibonacci wedge SVG icon.
    pub const FIB_WEDGE: Icon = crate::icon_from_path!("fibonacci/fib-wedge.svg");

    // Patterns and Elliott waves
    /// XABCD pattern SVG icon.
    pub const XABCD_PATTERN: Icon = crate::icon_from_path!("patterns-elliott/xabcd.svg");
    /// Cypher pattern SVG icon.
    pub const CYPHER_PATTERN: Icon = crate::icon_from_path!("patterns-elliott/cypher.svg");
    /// Head and shoulders SVG icon.
    pub const HEAD_AND_SHOULDERS: Icon =
        crate::icon_from_path!("patterns-elliott/head-and-shoulders.svg");
    /// ABCD SVG icon.
    pub const ABCD: Icon = crate::icon_from_path!("patterns-elliott/abcd.svg");
    /// Triangle pattern SVG icon.
    pub const TRIANGLE_PATTERN: Icon =
        crate::icon_from_path!("patterns-elliott/triangle-pattern.svg");
    /// Three drives SVG icon.
    pub const THREE_DRIVERS: Icon = crate::icon_from_path!("patterns-elliott/three-drives.svg");
    /// Elliott impulse SVG icon.
    pub const ELLIOTT_IMPULSE: Icon =
        crate::icon_from_path!("patterns-elliott/elliott-impulse.svg");
    /// Elliott correction SVG icon.
    pub const ELLIOTT_CORRECTION: Icon =
        crate::icon_from_path!("patterns-elliott/elliott-correction.svg");
    /// Elliott triangle SVG icon.
    pub const ELLIOTT_TRIANGLE: Icon =
        crate::icon_from_path!("patterns-elliott/elliott-triangle.svg");
    /// Elliott double combo SVG icon.
    pub const ELLIOTT_DOUBLE_COMBO: Icon =
        crate::icon_from_path!("patterns-elliott/elliott-double-combo.svg");
    /// Elliott triple combo SVG icon.
    pub const ELLIOTT_TRIPLE_COMBO: Icon =
        crate::icon_from_path!("patterns-elliott/elliott-triple-combo.svg");

    // Projection, volume, measurers and cycles
    /// Long position SVG icon.
    pub const LONG_POSITION: Icon =
        crate::icon_from_path!("projection-volume-measure/long-position.svg");
    /// Short position SVG icon.
    pub const SHORT_POSITION: Icon =
        crate::icon_from_path!("projection-volume-measure/short-position.svg");
    /// Prediction SVG icon.
    pub const PREDICTION: Icon = crate::icon_from_path!("projection-volume-measure/forecast.svg");
    /// Bars pattern SVG icon.
    pub const BARS_PATTERN: Icon =
        crate::icon_from_path!("projection-volume-measure/bars-pattern.svg");
    /// Ghost feed SVG icon.
    pub const GHOST_FEED: Icon = crate::icon_from_path!("projection-volume-measure/ghost-feed.svg");
    /// Projection SVG icon.
    pub const PROJECTION: Icon = crate::icon_from_path!("projection-volume-measure/projection.svg");
    /// Anchored vwap SVG icon.
    pub const ANCHORED_VWAP: Icon =
        crate::icon_from_path!("projection-volume-measure/anchored-vwap.svg");
    /// Fixed range volume profile SVG icon.
    pub const FIXED_RANGE_VOLUME_PROFILE: Icon =
        crate::icon_from_path!("projection-volume-measure/fixed-range-volume-profile.svg");
    /// Anchored volume profile SVG icon.
    pub const ANCHORED_VOLUME_PROFILE: Icon =
        crate::icon_from_path!("projection-volume-measure/anchored-volume-profile.svg");
    /// Measure SVG icon.
    pub const MEASURE: Icon = crate::icon_from_path!("projection-volume-measure/measure.svg");
    /// Price range SVG icon.
    pub const PRICE_RANGE: Icon =
        crate::icon_from_path!("projection-volume-measure/price-range.svg");
    /// Date range SVG icon.
    pub const DATE_RANGE: Icon = crate::icon_from_path!("projection-volume-measure/date-range.svg");
    /// Date and price range SVG icon.
    pub const DATE_AND_PRICE_RANGE: Icon =
        crate::icon_from_path!("projection-volume-measure/date-price-range.svg");
    /// Circle lines SVG icon.
    pub const CIRCLE_LINES: Icon =
        crate::icon_from_path!("projection-volume-measure/cyclic-lines.svg");
    /// Time cycles SVG icon.
    pub const TIME_CYCLES: Icon =
        crate::icon_from_path!("projection-volume-measure/time-cycles.svg");
    /// Sine line SVG icon.
    pub const SINE_LINE: Icon = crate::icon_from_path!("projection-volume-measure/sine-line.svg");

    // Shapes, brushes and annotations
    /// Brush SVG icon.
    pub const BRUSH: Icon = crate::icon_from_path!("shapes-annotations/brush.svg");
    /// Highlighter SVG icon.
    pub const HIGHLIGHTER: Icon = crate::icon_from_path!("shapes-annotations/highlighter.svg");
    /// Paintbrush SVG icon.
    pub const PAINTBRUSH: Icon = crate::icon_from_path!("shapes-annotations/paintbrush.svg");
    /// Arrow marker SVG icon.
    pub const ARROW_MARKER: Icon = crate::icon_from_path!("shapes-annotations/arrow-marker.svg");
    /// Arrow line SVG icon.
    pub const ARROW_LINE: Icon = crate::icon_from_path!("shapes-annotations/arrow.svg");
    /// Arrow mark up SVG icon.
    pub const ARROW_MARK_UP: Icon = crate::icon_from_path!("shapes-annotations/arrow-mark-up.svg");
    /// Arrow mark down SVG icon.
    pub const ARROW_MARK_DOWN: Icon =
        crate::icon_from_path!("shapes-annotations/arrow-mark-down.svg");
    /// Arrow mark left SVG icon.
    pub const ARROW_MARK_LEFT: Icon =
        crate::icon_from_path!("shapes-annotations/arrow-mark-left.svg");
    /// Arrow mark right SVG icon.
    pub const ARROW_MARK_RIGHT: Icon =
        crate::icon_from_path!("shapes-annotations/arrow-mark-right.svg");
    /// Rect SVG icon.
    pub const RECT: Icon = crate::icon_from_path!("shapes-annotations/rectangle.svg");
    /// Rotated rect SVG icon.
    pub const ROTATED_RECT: Icon =
        crate::icon_from_path!("shapes-annotations/rotated-rectangle.svg");
    /// Path SVG icon.
    pub const PATH: Icon = crate::icon_from_path!("shapes-annotations/path.svg");
    /// Circle SVG icon.
    pub const CIRCLE: Icon = crate::icon_from_path!("shapes-annotations/circle.svg");
    /// Ellipse SVG icon.
    pub const ELLIPSE: Icon = crate::icon_from_path!("shapes-annotations/ellipse.svg");
    /// Polyline SVG icon.
    pub const POLYLINE: Icon = crate::icon_from_path!("shapes-annotations/polyline.svg");
    /// Triangle SVG icon.
    pub const TRIANGLE: Icon = crate::icon_from_path!("shapes-annotations/triangle.svg");
    /// Sector SVG icon.
    pub const SECTOR: Icon = crate::icon_from_path!("shapes-annotations/sector.svg");
    /// Arc SVG icon.
    pub const ARC: Icon = crate::icon_from_path!("shapes-annotations/arc.svg");
    /// Bezier quadro SVG icon.
    pub const BEZIER_QUADRO: Icon = crate::icon_from_path!("shapes-annotations/curve.svg");
    /// Bezier cubic SVG icon.
    pub const BEZIER_CUBIC: Icon = crate::icon_from_path!("shapes-annotations/double-curve.svg");
    /// Text SVG icon.
    pub const TEXT: Icon = crate::icon_from_path!("shapes-annotations/text.svg");
    /// Anchored text SVG icon.
    pub const ANCHORED_TEXT: Icon = crate::icon_from_path!("shapes-annotations/anchored-text.svg");
    /// Note SVG icon.
    pub const NOTE: Icon = crate::icon_from_path!("shapes-annotations/note.svg");
    /// Price note SVG icon.
    pub const PRICE_NOTE: Icon = crate::icon_from_path!("shapes-annotations/price-note.svg");
    /// Anchored note SVG icon.
    pub const ANCHORED_NOTE: Icon = crate::icon_from_path!("shapes-annotations/anchored-note.svg");
    /// Callout SVG icon.
    pub const CALLOUT: Icon = crate::icon_from_path!("shapes-annotations/callout.svg");
    /// Comment SVG icon.
    pub const COMMENT: Icon = crate::icon_from_path!("shapes-annotations/comment.svg");
    /// Price label SVG icon.
    pub const PRICE_LABEL: Icon = crate::icon_from_path!("shapes-annotations/price-label.svg");
    /// Signpost SVG icon.
    pub const SIGNPOST: Icon = crate::icon_from_path!("shapes-annotations/signpost.svg");
    /// Flag mark SVG icon.
    pub const FLAG_MARK: Icon = crate::icon_from_path!("shapes-annotations/flag-mark.svg");
    /// Table SVG icon.
    pub const TABLE: Icon = crate::icon_from_path!("shapes-annotations/table.svg");
    /// Image SVG icon.
    pub const IMAGE: Icon = crate::icon_from_path!("shapes-annotations/image.svg");
    /// Post SVG icon.
    pub const POST: Icon = crate::icon_from_path!("shapes-annotations/post.svg");
    /// Ideas SVG icon.
    pub const IDEAS: Icon = crate::icon_from_path!("shapes-annotations/idea.svg");
    /// Sticker SVG icon.
    pub const STICKER: Icon = crate::icon_from_path!("shapes-annotations/sticker.svg");
    /// Emoji icon SVG icon.
    pub const EMOJI_ICON: Icon = crate::icon_from_path!("shapes-annotations/emoji.svg");

    // Chart types
    /// Chart bars SVG icon.
    pub const CHART_BARS: Icon = crate::icon_from_path!("chart-types/chart-bars.svg");
    /// Chart candles SVG icon.
    pub const CHART_CANDLES: Icon = crate::icon_from_path!("chart-types/chart-candles.svg");
    /// Chart hollow candles SVG icon.
    pub const CHART_HOLLOW_CANDLES: Icon =
        crate::icon_from_path!("chart-types/chart-hollow-candles.svg");
    /// Chart volume candles SVG icon.
    pub const CHART_VOLUME_CANDLES: Icon =
        crate::icon_from_path!("chart-types/chart-volume-candles.svg");
    /// Chart line SVG icon.
    pub const CHART_LINE: Icon = crate::icon_from_path!("chart-types/chart-line.svg");
    /// Chart line markers SVG icon.
    pub const CHART_LINE_MARKERS: Icon =
        crate::icon_from_path!("chart-types/chart-line-markers.svg");
    /// Chart step line SVG icon.
    pub const CHART_STEP_LINE: Icon = crate::icon_from_path!("chart-types/chart-step-line.svg");
    /// Chart area SVG icon.
    pub const CHART_AREA: Icon = crate::icon_from_path!("chart-types/chart-area.svg");
    /// Chart hlc area SVG icon.
    pub const CHART_HLC_AREA: Icon = crate::icon_from_path!("chart-types/chart-hlc-area.svg");
    /// Chart baseline SVG icon.
    pub const CHART_BASELINE: Icon = crate::icon_from_path!("chart-types/chart-baseline.svg");
    /// Chart columns SVG icon.
    pub const CHART_COLUMNS: Icon = crate::icon_from_path!("chart-types/chart-columns.svg");
    /// Chart high low SVG icon.
    pub const CHART_HIGH_LOW: Icon = crate::icon_from_path!("chart-types/chart-high-low.svg");
    /// Chart tpo SVG icon.
    pub const CHART_TPO: Icon = crate::icon_from_path!("chart-types/chart-tpo.svg");
    /// Chart volume footprint SVG icon.
    pub const CHART_VOLUME_FOOTPRINT: Icon =
        crate::icon_from_path!("chart-types/chart-volume-footprint.svg");
    /// Chart session volume profile SVG icon.
    pub const CHART_SESSION_VOLUME_PROFILE: Icon =
        crate::icon_from_path!("chart-types/chart-session-volume-profile.svg");
    /// Chart line break SVG icon.
    pub const CHART_LINE_BREAK: Icon = crate::icon_from_path!("chart-types/chart-line-break.svg");
    /// Chart kagi SVG icon.
    pub const CHART_KAGI: Icon = crate::icon_from_path!("chart-types/chart-kagi.svg");
    /// Chart range SVG icon.
    pub const CHART_RANGE: Icon = crate::icon_from_path!("chart-types/chart-range.svg");
    /// Chart point figure SVG icon.
    pub const CHART_POINT_FIGURE: Icon =
        crate::icon_from_path!("chart-types/chart-point-figure.svg");
    /// Chart renko SVG icon.
    pub const CHART_RENKO: Icon = crate::icon_from_path!("chart-types/chart-renko.svg");
    /// Chart heikin ashi SVG icon.
    pub const CHART_HEIKIN_ASHI: Icon = crate::icon_from_path!("chart-types/chart-heikin-ashi.svg");

    // Interface
    /// Menu SVG icon.
    pub const MENU: Icon = crate::icon_from_path!("ui/menu.svg");
    /// Settings SVG icon.
    pub const SETTINGS: Icon = crate::icon_from_path!("ui/settings.svg");
    /// Camera SVG icon.
    pub const CAMERA: Icon = crate::icon_from_path!("ui/camera.svg");
    /// Plus SVG icon.
    pub const PLUS: Icon = crate::icon_from_path!("ui/plus.svg");
    /// Circled plus for adding a comparison series.
    pub const COMPARE_ADD: Icon = crate::icon_from_path!("ui/compare-add.svg");
    /// Indicators SVG icon.
    pub const INDICATORS: Icon = crate::icon_from_path!("ui/indicators.svg");
    /// Alerts SVG icon.
    pub const ALERTS: Icon = crate::icon_from_path!("ui/alert.svg");
    /// Replay SVG icon.
    pub const REPLAY: Icon = crate::icon_from_path!("ui/replay.svg");
    /// Undo SVG icon.
    pub const UNDO: Icon = crate::icon_from_path!("ui/undo.svg");
    /// Redo SVG icon.
    pub const REDO: Icon = crate::icon_from_path!("ui/redo.svg");
    /// Minus SVG icon.
    pub const MINUS: Icon = crate::icon_from_path!("ui/minus.svg");
    /// Check SVG icon.
    pub const CHECK: Icon = crate::icon_from_path!("ui/check.svg");
    /// Chevron down SVG icon.
    pub const CHEVRON_DOWN: Icon = crate::icon_from_path!("ui/chevron-down.svg");
    /// Chevron up SVG icon.
    pub const CHEVRON_UP: Icon = crate::icon_from_path!("ui/chevron-up.svg");
    /// Chevron left SVG icon.
    pub const CHEVRON_LEFT: Icon = crate::icon_from_path!("ui/chevron-left.svg");
    /// Chevron right SVG icon.
    pub const CHEVRON_RIGHT: Icon = crate::icon_from_path!("ui/chevron-right.svg");
    /// Fullscreen SVG icon.
    pub const FULLSCREEN: Icon = crate::icon_from_path!("ui/fullscreen.svg");
    /// Theme toggle SVG icon.
    pub const THEME_TOGGLE: Icon = crate::icon_from_path!("ui/theme-toggle.svg");
    /// Quick search SVG icon.
    pub const QUICK_SEARCH: Icon = crate::icon_from_path!("ui/search.svg");
    /// Eyeglasses glyph (Coolicons-style) for symbol / research affordances.
    pub const GLASSES: Icon = crate::icon_from_path!("ui/glasses.svg");
    /// Magnifier with a lightning bolt for Quick Search, the command search.
    pub const COMMAND_SEARCH: Icon = crate::icon_from_path!("ui/quick-search.svg");
    /// Hex nut for the top toolbar's chart settings.
    pub const SETTINGS_NUT: Icon = crate::icon_from_path!("ui/settings-nut.svg");
    /// Light bulb with a plus for publishing the chart.
    pub const PUBLISH: Icon = crate::icon_from_path!("ui/publish.svg");
    /// Widget bar calendar SVG icon.
    pub const WIDGET_BAR_CALENDAR: Icon = crate::icon_from_path!("ui/calendar.svg");
    /// Go to date SVG icon.
    pub const GO_TO_DATE: Icon = crate::icon_from_path!("ui/go-to-date.svg");
    /// Widget bar help SVG icon.
    pub const WIDGET_BAR_HELP: Icon = crate::icon_from_path!("ui/help.svg");
    /// Info SVG icon.
    pub const INFO: Icon = crate::icon_from_path!("ui/info.svg");
    /// Success SVG icon.
    pub const SUCCESS: Icon = crate::icon_from_path!("ui/success.svg");
    /// Warning SVG icon.
    pub const WARNING: Icon = crate::icon_from_path!("ui/warning.svg");
    /// Status error SVG icon.
    pub const STATUS_ERROR: Icon = crate::icon_from_path!("ui/error.svg");
    /// Stacked layers for the widget bar object panel.
    pub const WIDGET_BAR_LAYERS: Icon = crate::icon_from_path!("ui/widget-bar-layers.svg");
    /// Bulleted list for the widget bar watchlist.
    pub const WIDGET_BAR_WATCHLIST: Icon = crate::icon_from_path!("ui/widget-bar-watchlist.svg");
    /// Four-point sparkle for the widget bar highlights.
    pub const WIDGET_BAR_HIGHLIGHTS: Icon = crate::icon_from_path!("ui/highlights.svg");
    /// Funnel for the widget bar screener.
    pub const WIDGET_BAR_SCREENER: Icon = crate::icon_from_path!("ui/screener.svg");
    /// Flame for the widget bar hotlists.
    pub const WIDGET_BAR_HOTLISTS: Icon = crate::icon_from_path!("ui/hotlists.svg");
    /// Treemap for the widget bar market heatmap.
    pub const WIDGET_BAR_MARKET_HEATMAP: Icon = crate::icon_from_path!("ui/market-heatmap.svg");
    /// Folded newspaper for the widget bar news.
    pub const WIDGET_BAR_NEWS: Icon = crate::icon_from_path!("ui/news.svg");
    /// Bell for the widget bar notifications.
    pub const WIDGET_BAR_NOTIFICATIONS: Icon = crate::icon_from_path!("ui/notifications.svg");
    /// Two speech bubbles for the widget bar chats.
    pub const WIDGET_BAR_CHATS: Icon = crate::icon_from_path!("ui/chats.svg");
    /// Grid of filled squares for the widget bar products launcher.
    pub const WIDGET_BAR_PRODUCTS: Icon = crate::icon_from_path!("ui/products.svg");
    /// Object tree SVG icon.
    pub const OBJECT_TREE: Icon = crate::icon_from_path!("ui/object-tree.svg");
    /// Eye SVG icon.
    pub const EYE: Icon = crate::icon_from_path!("ui/eye.svg");
    /// Eye hide SVG icon.
    pub const EYE_HIDE: Icon = crate::icon_from_path!("ui/eye-off.svg");
    /// Lock SVG icon.
    pub const LOCK: Icon = crate::icon_from_path!("ui/lock.svg");
    /// Unlock SVG icon.
    pub const UNLOCK: Icon = crate::icon_from_path!("ui/unlock.svg");
    /// Zoom in SVG icon.
    pub const ZOOM_IN: Icon = crate::icon_from_path!("ui/zoom-in.svg");
    /// Zoom out SVG icon.
    pub const ZOOM_OUT: Icon = crate::icon_from_path!("ui/zoom-out.svg");
    /// Move vertical SVG icon.
    pub const MOVE_VERTICAL: Icon = crate::icon_from_path!("ui/move-vertical.svg");
    /// Percent SVG icon.
    pub const PERCENT: Icon = crate::icon_from_path!("ui/percent.svg");
    /// Rotate ccw SVG icon.
    pub const ROTATE_CCW: Icon = crate::icon_from_path!("ui/rotate-ccw.svg");
    /// Pencil and lock for keeping the drawing tool active.
    pub const DRAWING_KEEP: Icon = crate::icon_from_path!("ui/drawing-keep.svg");
    /// Upright horseshoe magnet for the drawing rail.
    pub const DRAWING_MAGNET: Icon = crate::icon_from_path!("ui/drawing-magnet.svg");
    /// Magnet SVG icon.
    pub const MAGNET: Icon = crate::icon_from_path!("ui/magnet.svg");
    /// Trash SVG icon.
    pub const TRASH: Icon = crate::icon_from_path!("ui/trash.svg");
    /// Duplicate SVG icon.
    pub const DUPLICATE: Icon = crate::icon_from_path!("ui/duplicate.svg");
    /// Star empty SVG icon.
    pub const STAR_EMPTY: Icon = crate::icon_from_path!("ui/star-empty.svg");
    /// Generic outlined star SVG icon.
    pub const STAR: Icon = crate::icon_from_path!("ui/star.svg");
    /// Star filled SVG icon.
    pub const STAR_FILLED: Icon = crate::icon_from_path!("ui/star-filled.svg");
    /// Layout grid SVG icon.
    pub const LAYOUT_GRID: Icon = crate::icon_from_path!("ui/layout-grid.svg");
    /// Single chart pane for choosing the chart layout.
    pub const LAYOUT_SINGLE: Icon = crate::icon_from_path!("ui/layout-single.svg");
    // One glyph per multi-chart preset, the cells arranged as `ChartLayoutMode` tiles them.
    /// Layout two horizontal SVG icon.
    pub const LAYOUT_TWO_HORIZONTAL: Icon = crate::icon_from_path!("ui/layout-2-columns.svg");
    /// Layout two vertical SVG icon.
    pub const LAYOUT_TWO_VERTICAL: Icon = crate::icon_from_path!("ui/layout-2-rows.svg");
    /// Layout three SVG icon.
    pub const LAYOUT_THREE: Icon = crate::icon_from_path!("ui/layout-3.svg");
    /// Layout four SVG icon.
    pub const LAYOUT_FOUR: Icon = crate::icon_from_path!("ui/layout-4.svg");
    /// Layout five SVG icon.
    pub const LAYOUT_FIVE: Icon = crate::icon_from_path!("ui/layout-5.svg");
    /// Layout six SVG icon.
    pub const LAYOUT_SIX: Icon = crate::icon_from_path!("ui/layout-6.svg");
    /// Layout seven SVG icon.
    pub const LAYOUT_SEVEN: Icon = crate::icon_from_path!("ui/layout-7.svg");
    /// Layout eight SVG icon.
    pub const LAYOUT_EIGHT: Icon = crate::icon_from_path!("ui/layout-8.svg");
    /// Layout nine SVG icon.
    pub const LAYOUT_NINE: Icon = crate::icon_from_path!("ui/layout-9.svg");
    /// Layout ten SVG icon.
    pub const LAYOUT_TEN: Icon = crate::icon_from_path!("ui/layout-10.svg");
    /// Layout eleven SVG icon.
    pub const LAYOUT_ELEVEN: Icon = crate::icon_from_path!("ui/layout-11.svg");
    /// Layout twelve SVG icon.
    pub const LAYOUT_TWELVE: Icon = crate::icon_from_path!("ui/layout-12.svg");
    /// Layout thirteen SVG icon.
    pub const LAYOUT_THIRTEEN: Icon = crate::icon_from_path!("ui/layout-13.svg");
    /// Layout fourteen SVG icon.
    pub const LAYOUT_FOURTEEN: Icon = crate::icon_from_path!("ui/layout-14.svg");
    /// Layout fifteen SVG icon.
    pub const LAYOUT_FIFTEEN: Icon = crate::icon_from_path!("ui/layout-15.svg");
    /// Layout sixteen SVG icon.
    pub const LAYOUT_SIXTEEN: Icon = crate::icon_from_path!("ui/layout-16.svg");
    /// One chart with a tab strip, for the tabbed preset.
    pub const LAYOUT_TABS: Icon = crate::icon_from_path!("ui/layout-tabs.svg");
    /// Layout setup SVG icon.
    pub const LAYOUT_SETUP: Icon = crate::icon_from_path!("ui/layout-setup.svg");
    /// Close SVG icon.
    pub const CLOSE: Icon = crate::icon_from_path!("ui/close.svg");
    /// Three columns, for a table's column chooser.
    pub const COLUMNS: Icon = crate::icon_from_path!("ui/columns.svg");
    /// An empty square, for maximizing a panel.
    pub const MAXIMIZE: Icon = crate::icon_from_path!("ui/maximize.svg");
    /// A square with a plus.
    pub const SQUARE_PLUS: Icon = crate::icon_from_path!("ui/square-plus.svg");
    /// Move pane SVG icon.
    pub const MOVE_PANE: Icon = crate::icon_from_path!("ui/move-pane.svg");
    /// An arrow into a price axis, for pinning a study to a scale.
    pub const PIN_TO_SCALE: Icon = crate::icon_from_path!("ui/pin-scale.svg");
    /// Watchlist SVG icon.
    pub const WATCHLIST: Icon = crate::icon_from_path!("ui/watchlist.svg");
    /// Ui drag handle SVG icon.
    pub const UI_DRAG_HANDLE: Icon = crate::icon_from_path!("ui/drag-handle.svg");
    /// Ui more horizontal SVG icon.
    pub const UI_MORE_HORIZONTAL: Icon = crate::icon_from_path!("ui/more.svg");
    /// Ui buffering SVG icon.
    pub const UI_BUFFERING: Icon = crate::icon_from_path!("ui/loading.svg");
    /// Chain link with a copy affordance.
    pub const COPY_LINK: Icon = crate::icon_from_path!("ui/copy-link.svg");
    /// Share SVG icon.
    pub const SHARE: Icon = crate::icon_from_path!("ui/share.svg");
    /// Download image SVG icon.
    pub const DOWNLOAD_IMAGE: Icon = crate::icon_from_path!("ui/download-image.svg");
    /// Copy image SVG icon.
    pub const COPY_IMAGE: Icon = crate::icon_from_path!("ui/copy-image.svg");
    /// Open external SVG icon.
    pub const OPEN_EXTERNAL: Icon = crate::icon_from_path!("ui/open-external.svg");
    /// Send message SVG icon.
    pub const SEND_MESSAGE: Icon = crate::icon_from_path!("ui/send-message.svg");
    /// Export table SVG icon.
    pub const EXPORT_TABLE: Icon = crate::icon_from_path!("ui/export-table.svg");
    /// Generic filter SVG icon.
    pub const FILTER: Icon = crate::icon_from_path!("ui/filter.svg");
    /// Generic pin SVG icon.
    pub const PIN: Icon = crate::icon_from_path!("ui/pin.svg");
    /// Generic save SVG icon.
    pub const SAVE: Icon = crate::icon_from_path!("ui/save.svg");

    // Settings dialog tabs
    /// Settings symbol SVG icon.
    pub const SETTINGS_SYMBOL: Icon = crate::icon_from_path!("ui/symbol.svg");
    /// Settings status line SVG icon.
    pub const SETTINGS_STATUS_LINE: Icon = crate::icon_from_path!("ui/status-line.svg");
    /// Settings scales lines SVG icon.
    pub const SETTINGS_SCALES_LINES: Icon = crate::icon_from_path!("ui/scales-lines.svg");
    /// Settings canvas SVG icon.
    pub const SETTINGS_CANVAS: Icon = crate::icon_from_path!("ui/canvas.svg");
    /// Settings trading SVG icon: a buy and a sell arrow.
    pub const SETTINGS_TRADING: Icon = crate::icon_from_path!("ui/trading.svg");
    /// Settings events SVG icon.
    pub const SETTINGS_EVENTS: Icon = crate::icon_from_path!("ui/events.svg");

    // Indicator categories and object tree groups
    /// Code SVG icon.
    pub const CODE: Icon = crate::icon_from_path!("ui/code.svg");
    /// Users SVG icon.
    pub const USERS: Icon = crate::icon_from_path!("ui/users.svg");
    /// Crown SVG icon.
    pub const CROWN: Icon = crate::icon_from_path!("ui/crown.svg");
    /// Database SVG icon.
    pub const DATABASE: Icon = crate::icon_from_path!("ui/database.svg");
    /// Pencil SVG icon.
    pub const PENCIL: Icon = crate::icon_from_path!("ui/pencil.svg");
    /// Template SVG icon.
    pub const TEMPLATE: Icon = crate::icon_from_path!("ui/template.svg");

    // Connection status
    /// Status connected SVG icon.
    pub const STATUS_CONNECTED: Icon = crate::icon_from_path!("ui/connected.svg");
    /// Status connecting SVG icon.
    pub const STATUS_CONNECTING: Icon = crate::icon_from_path!("ui/connecting.svg");
    /// Status disconnected SVG icon.
    pub const STATUS_DISCONNECTED: Icon = crate::icon_from_path!("ui/disconnected.svg");

    // Media and replay
    /// Media play SVG icon.
    pub const MEDIA_PLAY: Icon = crate::icon_from_path!("ui/play.svg");
    /// Media pause SVG icon.
    pub const MEDIA_PAUSE: Icon = crate::icon_from_path!("ui/pause.svg");
    /// Media stop SVG icon.
    pub const MEDIA_STOP: Icon = crate::icon_from_path!("ui/stop.svg");
    /// Media skip back SVG icon.
    pub const MEDIA_SKIP_BACK: Icon = crate::icon_from_path!("ui/skip-back.svg");
    /// Media skip forward SVG icon.
    pub const MEDIA_SKIP_FORWARD: Icon = crate::icon_from_path!("ui/skip-forward.svg");
    /// Ui arrow back SVG icon.
    pub const UI_ARROW_BACK: Icon = crate::icon_from_path!("ui/step-back.svg");
    /// Ui arrow forward SVG icon.
    pub const UI_ARROW_FORWARD: Icon = crate::icon_from_path!("ui/step-forward.svg");

    // Complete Oracly 550 catalog additions
    /// Crosshair SVG icon.
    pub const ORACLY_CROSSHAIR: Icon = crate::icon_from_path!("lines/crosshair.svg");
    /// Deselect SVG icon.
    pub const DESELECT: Icon = crate::icon_from_path!("lines/deselect.svg");
    /// Fit content SVG icon.
    pub const FIT_CONTENT: Icon = crate::icon_from_path!("lines/fit-content.svg");
    /// Magnet off SVG icon.
    pub const MAGNET_OFF: Icon = crate::icon_from_path!("lines/magnet-off.svg");
    /// Magnet strong SVG icon.
    pub const MAGNET_STRONG: Icon = crate::icon_from_path!("lines/magnet-strong.svg");
    /// Move horizontal SVG icon.
    pub const MOVE_HORIZONTAL: Icon = crate::icon_from_path!("lines/move-horizontal.svg");
    /// Move SVG icon.
    pub const MOVE: Icon = crate::icon_from_path!("lines/move.svg");
    /// Pan hand SVG icon.
    pub const PAN_HAND: Icon = crate::icon_from_path!("lines/pan-hand.svg");
    /// Rotate right SVG icon.
    pub const ROTATE_RIGHT: Icon = crate::icon_from_path!("lines/rotate-right.svg");
    /// Select all SVG icon.
    pub const SELECT_ALL: Icon = crate::icon_from_path!("lines/select-all.svg");
    /// Select box SVG icon.
    pub const SELECT_BOX: Icon = crate::icon_from_path!("lines/select-box.svg");
    /// Select lasso SVG icon.
    pub const SELECT_LASSO: Icon = crate::icon_from_path!("lines/select-lasso.svg");
    /// Snap grid SVG icon.
    pub const SNAP_GRID: Icon = crate::icon_from_path!("lines/snap-grid.svg");
    /// Zoom reset SVG icon.
    pub const ZOOM_RESET: Icon = crate::icon_from_path!("lines/zoom-reset.svg");
    /// Zoom selection SVG icon.
    pub const ZOOM_SELECTION: Icon = crate::icon_from_path!("lines/zoom-selection.svg");
    /// Curved line tool SVG icon.
    pub const CURVED_LINE_TOOL: Icon = crate::icon_from_path!("lines/curved-line-tool.svg");
    /// Dashed line SVG icon.
    pub const DASHED_LINE: Icon = crate::icon_from_path!("lines/dashed-line.svg");
    /// Dotted line SVG icon.
    pub const DOTTED_LINE: Icon = crate::icon_from_path!("lines/dotted-line.svg");
    /// Double arrow line SVG icon.
    pub const DOUBLE_ARROW_LINE: Icon = crate::icon_from_path!("lines/double-arrow-line.svg");
    /// Elbow line SVG icon.
    pub const ELBOW_LINE: Icon = crate::icon_from_path!("lines/elbow-line.svg");
    /// Horizontal level SVG icon.
    pub const HORIZONTAL_LEVEL: Icon = crate::icon_from_path!("lines/horizontal-level.svg");
    /// Line endpoints SVG icon.
    pub const LINE_ENDPOINTS: Icon = crate::icon_from_path!("lines/line-endpoints.svg");
    /// Price line SVG icon.
    pub const PRICE_LINE: Icon = crate::icon_from_path!("lines/price-line.svg");
    /// Range boundaries SVG icon.
    pub const RANGE_BOUNDARIES: Icon = crate::icon_from_path!("lines/range-boundaries.svg");
    /// Resistance line SVG icon.
    pub const RESISTANCE_LINE: Icon = crate::icon_from_path!("lines/resistance-line.svg");
    /// Session separator SVG icon.
    pub const SESSION_SEPARATOR: Icon = crate::icon_from_path!("lines/session-separator.svg");
    /// Slope line SVG icon.
    pub const SLOPE_LINE: Icon = crate::icon_from_path!("lines/slope-line.svg");
    /// Step line tool SVG icon.
    pub const STEP_LINE_TOOL: Icon = crate::icon_from_path!("lines/step-line-tool.svg");
    /// Support line SVG icon.
    pub const SUPPORT_LINE: Icon = crate::icon_from_path!("lines/support-line.svg");
    /// Vertical marker SVG icon.
    pub const VERTICAL_MARKER: Icon = crate::icon_from_path!("lines/vertical-marker.svg");
    /// Ascending channel SVG icon.
    pub const ASCENDING_CHANNEL: Icon = crate::icon_from_path!("lines/ascending-channel.svg");
    /// Channel anchor SVG icon.
    pub const CHANNEL_ANCHOR: Icon = crate::icon_from_path!("lines/channel-anchor.svg");
    /// Channel breakout SVG icon.
    pub const CHANNEL_BREAKOUT: Icon = crate::icon_from_path!("lines/channel-breakout.svg");
    /// Channel fill SVG icon.
    pub const CHANNEL_FILL: Icon = crate::icon_from_path!("lines/channel-fill.svg");
    /// Channel midline SVG icon.
    pub const CHANNEL_MIDLINE: Icon = crate::icon_from_path!("lines/channel-midline.svg");
    /// Channel width SVG icon.
    pub const CHANNEL_WIDTH: Icon = crate::icon_from_path!("lines/channel-width.svg");
    /// Converging channel SVG icon.
    pub const CONVERGING_CHANNEL: Icon = crate::icon_from_path!("lines/converging-channel.svg");
    /// Descending channel SVG icon.
    pub const DESCENDING_CHANNEL: Icon = crate::icon_from_path!("lines/descending-channel.svg");
    /// Envelope channel SVG icon.
    pub const ENVELOPE_CHANNEL: Icon = crate::icon_from_path!("lines/envelope-channel.svg");
    /// Expanding channel SVG icon.
    pub const EXPANDING_CHANNEL: Icon = crate::icon_from_path!("lines/expanding-channel.svg");
    /// Flat bottom channel SVG icon.
    pub const FLAT_BOTTOM_CHANNEL: Icon = crate::icon_from_path!("lines/flat-bottom-channel.svg");
    /// Horizontal channel SVG icon.
    pub const HORIZONTAL_CHANNEL: Icon = crate::icon_from_path!("lines/horizontal-channel.svg");
    /// Median line SVG icon.
    pub const MEDIAN_LINE: Icon = crate::icon_from_path!("lines/median-line.svg");
    /// Price channel SVG icon.
    pub const PRICE_CHANNEL: Icon = crate::icon_from_path!("lines/price-channel.svg");
    /// Raff channel SVG icon.
    pub const RAFF_CHANNEL: Icon = crate::icon_from_path!("lines/raff-channel.svg");
    /// Standard deviation channel SVG icon.
    pub const STANDARD_DEVIATION_CHANNEL: Icon =
        crate::icon_from_path!("lines/standard-deviation-channel.svg");
    /// Fibonacci arcs SVG icon.
    pub const FIBONACCI_ARCS: Icon = crate::icon_from_path!("fibonacci/fibonacci-arcs.svg");
    /// Fibonacci fan SVG icon.
    pub const FIBONACCI_FAN: Icon = crate::icon_from_path!("fibonacci/fibonacci-fan.svg");
    /// Fibonacci projection SVG icon.
    pub const FIBONACCI_PROJECTION: Icon =
        crate::icon_from_path!("fibonacci/fibonacci-projection.svg");
    /// Fibonacci ratio SVG icon.
    pub const FIBONACCI_RATIO: Icon = crate::icon_from_path!("fibonacci/fibonacci-ratio.svg");
    /// Fibonacci time extension SVG icon.
    pub const FIBONACCI_TIME_EXTENSION: Icon =
        crate::icon_from_path!("fibonacci/fibonacci-time-extension.svg");
    /// Gann angle SVG icon.
    pub const GANN_ANGLE: Icon = crate::icon_from_path!("pitchforks-gann/gann-angle.svg");
    /// Gann arc SVG icon.
    pub const GANN_ARC: Icon = crate::icon_from_path!("pitchforks-gann/gann-arc.svg");
    /// Gann grid SVG icon.
    pub const GANN_GRID: Icon = crate::icon_from_path!("pitchforks-gann/gann-grid.svg");
    /// Gann wheel SVG icon.
    pub const GANN_WHEEL: Icon = crate::icon_from_path!("pitchforks-gann/gann-wheel.svg");
    /// Golden ratio SVG icon.
    pub const GOLDEN_RATIO: Icon = crate::icon_from_path!("fibonacci/golden-ratio.svg");
    /// Ascending triangle SVG icon.
    pub const ASCENDING_TRIANGLE: Icon =
        crate::icon_from_path!("patterns-elliott/ascending-triangle.svg");
    /// Bear flag SVG icon.
    pub const BEAR_FLAG: Icon = crate::icon_from_path!("patterns-elliott/bear-flag.svg");
    /// Bull flag SVG icon.
    pub const BULL_FLAG: Icon = crate::icon_from_path!("patterns-elliott/bull-flag.svg");
    /// Descending triangle SVG icon.
    pub const DESCENDING_TRIANGLE: Icon =
        crate::icon_from_path!("patterns-elliott/descending-triangle.svg");
    /// Double bottom SVG icon.
    pub const DOUBLE_BOTTOM: Icon = crate::icon_from_path!("patterns-elliott/double-bottom.svg");
    /// Double top SVG icon.
    pub const DOUBLE_TOP: Icon = crate::icon_from_path!("patterns-elliott/double-top.svg");
    /// Harmonic bat SVG icon.
    pub const HARMONIC_BAT: Icon = crate::icon_from_path!("patterns-elliott/harmonic-bat.svg");
    /// Harmonic butterfly SVG icon.
    pub const HARMONIC_BUTTERFLY: Icon =
        crate::icon_from_path!("patterns-elliott/harmonic-butterfly.svg");
    /// Harmonic crab SVG icon.
    pub const HARMONIC_CRAB: Icon = crate::icon_from_path!("patterns-elliott/harmonic-crab.svg");
    /// Harmonic gartley SVG icon.
    pub const HARMONIC_GARTLEY: Icon =
        crate::icon_from_path!("patterns-elliott/harmonic-gartley.svg");
    /// Harmonic shark SVG icon.
    pub const HARMONIC_SHARK: Icon = crate::icon_from_path!("patterns-elliott/harmonic-shark.svg");
    /// Inverse head and shoulders SVG icon.
    pub const INVERSE_HEAD_AND_SHOULDERS: Icon =
        crate::icon_from_path!("patterns-elliott/inverse-head-and-shoulders.svg");
    /// Triple bottom SVG icon.
    pub const TRIPLE_BOTTOM: Icon = crate::icon_from_path!("patterns-elliott/triple-bottom.svg");
    /// Triple top SVG icon.
    pub const TRIPLE_TOP: Icon = crate::icon_from_path!("patterns-elliott/triple-top.svg");
    /// Diamond SVG icon.
    pub const DIAMOND: Icon = crate::icon_from_path!("shapes-annotations/diamond.svg");
    /// Double arrow shape SVG icon.
    pub const DOUBLE_ARROW_SHAPE: Icon =
        crate::icon_from_path!("shapes-annotations/double-arrow-shape.svg");
    /// Hexagon SVG icon.
    pub const HEXAGON: Icon = crate::icon_from_path!("shapes-annotations/hexagon.svg");
    /// Pentagon SVG icon.
    pub const PENTAGON: Icon = crate::icon_from_path!("shapes-annotations/pentagon.svg");
    /// Polygon SVG icon.
    pub const POLYGON: Icon = crate::icon_from_path!("shapes-annotations/polygon.svg");
    /// Right triangle SVG icon.
    pub const RIGHT_TRIANGLE: Icon =
        crate::icon_from_path!("shapes-annotations/right-triangle.svg");
    /// Rounded rectangle SVG icon.
    pub const ROUNDED_RECTANGLE: Icon =
        crate::icon_from_path!("shapes-annotations/rounded-rectangle.svg");
    /// Speech bubble shape SVG icon.
    pub const SPEECH_BUBBLE_SHAPE: Icon =
        crate::icon_from_path!("shapes-annotations/speech-bubble-shape.svg");
    /// Spiral tool SVG icon.
    pub const SPIRAL_TOOL: Icon = crate::icon_from_path!("shapes-annotations/spiral-tool.svg");
    /// Arrow down right SVG icon.
    pub const ARROW_DOWN_RIGHT: Icon =
        crate::icon_from_path!("shapes-annotations/arrow-down-right.svg");
    /// Arrow up right SVG icon.
    pub const ARROW_UP_RIGHT: Icon =
        crate::icon_from_path!("shapes-annotations/arrow-up-right.svg");
    /// Bookmark SVG icon.
    pub const BOOKMARK: Icon = crate::icon_from_path!("shapes-annotations/bookmark.svg");
    /// Comment add SVG icon.
    pub const COMMENT_ADD: Icon = crate::icon_from_path!("shapes-annotations/comment-add.svg");
    /// Comment resolved SVG icon.
    pub const COMMENT_RESOLVED: Icon =
        crate::icon_from_path!("shapes-annotations/comment-resolved.svg");
    /// Label tag SVG icon.
    pub const LABEL_TAG: Icon = crate::icon_from_path!("shapes-annotations/label-tag.svg");
    /// Text align center SVG icon.
    pub const TEXT_ALIGN_CENTER: Icon =
        crate::icon_from_path!("shapes-annotations/text-align-center.svg");
    /// Text align left SVG icon.
    pub const TEXT_ALIGN_LEFT: Icon =
        crate::icon_from_path!("shapes-annotations/text-align-left.svg");
    /// Text bold SVG icon.
    pub const TEXT_BOLD: Icon = crate::icon_from_path!("shapes-annotations/text-bold.svg");
    /// Text italic SVG icon.
    pub const TEXT_ITALIC: Icon = crate::icon_from_path!("shapes-annotations/text-italic.svg");
    /// Text underline SVG icon.
    pub const TEXT_UNDERLINE: Icon =
        crate::icon_from_path!("shapes-annotations/text-underline.svg");
    /// Angle measure SVG icon.
    pub const ANGLE_MEASURE: Icon =
        crate::icon_from_path!("projection-volume-measure/angle-measure.svg");
    /// Break even SVG icon.
    pub const BREAK_EVEN: Icon = crate::icon_from_path!("projection-volume-measure/break-even.svg");
    /// Distance SVG icon.
    pub const DISTANCE: Icon = crate::icon_from_path!("projection-volume-measure/distance.svg");
    /// Entry marker SVG icon.
    pub const ENTRY_MARKER: Icon =
        crate::icon_from_path!("projection-volume-measure/entry-marker.svg");
    /// Exit marker SVG icon.
    pub const EXIT_MARKER: Icon =
        crate::icon_from_path!("projection-volume-measure/exit-marker.svg");
    /// Prediction SVG icon.
    pub const ORACLY_PREDICTION: Icon =
        crate::icon_from_path!("projection-volume-measure/prediction.svg");
    /// Price box SVG icon.
    pub const PRICE_BOX: Icon = crate::icon_from_path!("projection-volume-measure/price-box.svg");
    /// Range contraction SVG icon.
    pub const RANGE_CONTRACTION: Icon =
        crate::icon_from_path!("projection-volume-measure/range-contraction.svg");
    /// Range expansion SVG icon.
    pub const RANGE_EXPANSION: Icon =
        crate::icon_from_path!("projection-volume-measure/range-expansion.svg");
    /// Risk reward SVG icon.
    pub const RISK_REWARD: Icon =
        crate::icon_from_path!("projection-volume-measure/risk-reward.svg");
    /// Stop loss SVG icon.
    pub const STOP_LOSS: Icon = crate::icon_from_path!("projection-volume-measure/stop-loss.svg");
    /// Take profit SVG icon.
    pub const TAKE_PROFIT: Icon =
        crate::icon_from_path!("projection-volume-measure/take-profit.svg");
    /// Target SVG icon.
    pub const TARGET: Icon = crate::icon_from_path!("projection-volume-measure/target.svg");
    /// Time span SVG icon.
    pub const TIME_SPAN: Icon = crate::icon_from_path!("projection-volume-measure/time-span.svg");
    /// Equivolume chart SVG icon.
    pub const EQUIVOLUME_CHART: Icon = crate::icon_from_path!("chart-types/equivolume-chart.svg");
    /// Histogram chart SVG icon.
    pub const HISTOGRAM_CHART: Icon = crate::icon_from_path!("chart-types/histogram-chart.svg");
    /// Mountain chart SVG icon.
    pub const MOUNTAIN_CHART: Icon = crate::icon_from_path!("chart-types/mountain-chart.svg");
    /// Scatter chart SVG icon.
    pub const SCATTER_CHART: Icon = crate::icon_from_path!("chart-types/scatter-chart.svg");
    /// Stacked area SVG icon.
    pub const STACKED_AREA: Icon = crate::icon_from_path!("chart-types/stacked-area.svg");
    /// Adx SVG icon.
    pub const ADX: Icon = crate::icon_from_path!("indicators/adx.svg");
    /// Atr SVG icon.
    pub const ATR: Icon = crate::icon_from_path!("indicators/atr.svg");
    /// Bollinger bands SVG icon.
    pub const BOLLINGER_BANDS: Icon = crate::icon_from_path!("indicators/bollinger-bands.svg");
    /// Cci SVG icon.
    pub const CCI: Icon = crate::icon_from_path!("indicators/cci.svg");
    /// Chaikin money flow SVG icon.
    pub const CHAIKIN_MONEY_FLOW: Icon =
        crate::icon_from_path!("indicators/chaikin-money-flow.svg");
    /// Correlation SVG icon.
    pub const CORRELATION: Icon = crate::icon_from_path!("indicators/correlation.svg");
    /// Exponential moving average SVG icon.
    pub const EXPONENTIAL_MOVING_AVERAGE: Icon =
        crate::icon_from_path!("indicators/exponential-moving-average.svg");
    /// Historical volatility SVG icon.
    pub const HISTORICAL_VOLATILITY: Icon =
        crate::icon_from_path!("indicators/historical-volatility.svg");
    /// Ichimoku cloud SVG icon.
    pub const ICHIMOKU_CLOUD: Icon = crate::icon_from_path!("indicators/ichimoku-cloud.svg");
    /// Macd SVG icon.
    pub const MACD: Icon = crate::icon_from_path!("indicators/macd.svg");
    /// Momentum SVG icon.
    pub const MOMENTUM: Icon = crate::icon_from_path!("indicators/momentum.svg");
    /// Money flow index SVG icon.
    pub const MONEY_FLOW_INDEX: Icon = crate::icon_from_path!("indicators/money-flow-index.svg");
    /// Moving average SVG icon.
    pub const MOVING_AVERAGE: Icon = crate::icon_from_path!("indicators/moving-average.svg");
    /// Obv SVG icon.
    pub const OBV: Icon = crate::icon_from_path!("indicators/obv.svg");
    /// Parabolic sar SVG icon.
    pub const PARABOLIC_SAR: Icon = crate::icon_from_path!("indicators/parabolic-sar.svg");
    /// Pivot points SVG icon.
    pub const PIVOT_POINTS: Icon = crate::icon_from_path!("indicators/pivot-points.svg");
    /// Rate of change SVG icon.
    pub const RATE_OF_CHANGE: Icon = crate::icon_from_path!("indicators/rate-of-change.svg");
    /// Rsi SVG icon.
    pub const RSI: Icon = crate::icon_from_path!("indicators/rsi.svg");
    /// Standard deviation SVG icon.
    pub const STANDARD_DEVIATION: Icon =
        crate::icon_from_path!("indicators/standard-deviation.svg");
    /// Stochastic SVG icon.
    pub const STOCHASTIC: Icon = crate::icon_from_path!("indicators/stochastic.svg");
    /// Supertrend SVG icon.
    pub const SUPERTREND: Icon = crate::icon_from_path!("indicators/supertrend.svg");
    /// Weighted moving average SVG icon.
    pub const WEIGHTED_MOVING_AVERAGE: Icon =
        crate::icon_from_path!("indicators/weighted-moving-average.svg");
    /// Williams r SVG icon.
    pub const WILLIAMS_R: Icon = crate::icon_from_path!("indicators/williams-r.svg");
    /// Absorption SVG icon.
    pub const ABSORPTION: Icon = crate::icon_from_path!("volume-orderflow/absorption.svg");
    /// Buy volume SVG icon.
    pub const BUY_VOLUME: Icon = crate::icon_from_path!("volume-orderflow/buy-volume.svg");
    /// Cumulative delta SVG icon.
    pub const CUMULATIVE_DELTA: Icon =
        crate::icon_from_path!("volume-orderflow/cumulative-delta.svg");
    /// Funding rate SVG icon.
    pub const FUNDING_RATE: Icon = crate::icon_from_path!("volume-orderflow/funding-rate.svg");
    /// Imbalance SVG icon.
    pub const IMBALANCE: Icon = crate::icon_from_path!("volume-orderflow/imbalance.svg");
    /// Liquidity sweep SVG icon.
    pub const LIQUIDITY_SWEEP: Icon =
        crate::icon_from_path!("volume-orderflow/liquidity-sweep.svg");
    /// Liquidity SVG icon.
    pub const LIQUIDITY: Icon = crate::icon_from_path!("volume-orderflow/liquidity.svg");
    /// Market depth SVG icon.
    pub const MARKET_DEPTH: Icon = crate::icon_from_path!("volume-orderflow/market-depth.svg");
    /// Open interest SVG icon.
    pub const OPEN_INTEREST: Icon = crate::icon_from_path!("volume-orderflow/open-interest.svg");
    /// Order book SVG icon.
    pub const ORDER_BOOK: Icon = crate::icon_from_path!("volume-orderflow/order-book.svg");
    /// Point of control SVG icon.
    pub const POINT_OF_CONTROL: Icon =
        crate::icon_from_path!("volume-orderflow/point-of-control.svg");
    /// Relative volume SVG icon.
    pub const RELATIVE_VOLUME: Icon =
        crate::icon_from_path!("volume-orderflow/relative-volume.svg");
    /// Sell volume SVG icon.
    pub const SELL_VOLUME: Icon = crate::icon_from_path!("volume-orderflow/sell-volume.svg");
    /// Tape SVG icon.
    pub const TAPE: Icon = crate::icon_from_path!("volume-orderflow/tape.svg");
    /// Time and sales SVG icon.
    pub const TIME_AND_SALES: Icon = crate::icon_from_path!("volume-orderflow/time-and-sales.svg");
    /// Value area SVG icon.
    pub const VALUE_AREA: Icon = crate::icon_from_path!("volume-orderflow/value-area.svg");
    /// Volume bars SVG icon.
    pub const VOLUME_BARS: Icon = crate::icon_from_path!("volume-orderflow/volume-bars.svg");
    /// Volume delta SVG icon.
    pub const VOLUME_DELTA: Icon = crate::icon_from_path!("volume-orderflow/volume-delta.svg");
    /// Volume heatmap SVG icon.
    pub const VOLUME_HEATMAP: Icon = crate::icon_from_path!("volume-orderflow/volume-heatmap.svg");
    /// Volume oscillator SVG icon.
    pub const VOLUME_OSCILLATOR: Icon =
        crate::icon_from_path!("volume-orderflow/volume-oscillator.svg");
    /// Volume profile visible SVG icon.
    pub const VOLUME_PROFILE_VISIBLE: Icon =
        crate::icon_from_path!("volume-orderflow/volume-profile-visible.svg");
    /// Market gainers SVG icon.
    pub const MARKET_GAINERS: Icon = crate::icon_from_path!("ui/market-gainers.svg");
    /// Market losers SVG icon.
    pub const MARKET_LOSERS: Icon = crate::icon_from_path!("ui/market-losers.svg");
    /// Market overview SVG icon.
    pub const MARKET_OVERVIEW: Icon = crate::icon_from_path!("ui/market-overview.svg");
    /// Market sectors SVG icon.
    pub const MARKET_SECTORS: Icon = crate::icon_from_path!("ui/market-sectors.svg");
    /// Screener columns SVG icon.
    pub const SCREENER_COLUMNS: Icon = crate::icon_from_path!("ui/screener-columns.svg");
    /// Symbol add SVG icon.
    pub const SYMBOL_ADD: Icon = crate::icon_from_path!("ui/symbol-add.svg");
    /// Symbol favorite SVG icon.
    pub const SYMBOL_FAVORITE: Icon = crate::icon_from_path!("ui/symbol-favorite.svg");
    /// Symbol remove SVG icon.
    pub const SYMBOL_REMOVE: Icon = crate::icon_from_path!("ui/symbol-remove.svg");
    /// Symbol search SVG icon.
    pub const SYMBOL_SEARCH: Icon = crate::icon_from_path!("ui/symbol-search.svg");
    /// Symbol switch SVG icon.
    pub const SYMBOL_SWITCH: Icon = crate::icon_from_path!("ui/symbol-switch.svg");
    /// Watchlist add SVG icon.
    pub const WATCHLIST_ADD: Icon = crate::icon_from_path!("ui/watchlist-add.svg");
    /// Watchlist edit SVG icon.
    pub const WATCHLIST_EDIT: Icon = crate::icon_from_path!("ui/watchlist-edit.svg");
    /// Watchlist export SVG icon.
    pub const WATCHLIST_EXPORT: Icon = crate::icon_from_path!("ui/watchlist-export.svg");
    /// Watchlist filter SVG icon.
    pub const WATCHLIST_FILTER: Icon = crate::icon_from_path!("ui/watchlist-filter.svg");
    /// Watchlist flag SVG icon.
    pub const WATCHLIST_FLAG: Icon = crate::icon_from_path!("ui/watchlist-flag.svg");
    /// Watchlist import SVG icon.
    pub const WATCHLIST_IMPORT: Icon = crate::icon_from_path!("ui/watchlist-import.svg");
    /// Watchlist remove SVG icon.
    pub const WATCHLIST_REMOVE: Icon = crate::icon_from_path!("ui/watchlist-remove.svg");
    /// Watchlist sort down SVG icon.
    pub const WATCHLIST_SORT_DOWN: Icon = crate::icon_from_path!("ui/watchlist-sort-down.svg");
    /// Watchlist sort up SVG icon.
    pub const WATCHLIST_SORT_UP: Icon = crate::icon_from_path!("ui/watchlist-sort-up.svg");
    /// Alert add SVG icon.
    pub const ALERT_ADD: Icon = crate::icon_from_path!("ui/alert-add.svg");
    /// Alert calendar SVG icon.
    pub const ALERT_CALENDAR: Icon = crate::icon_from_path!("ui/alert-calendar.svg");
    /// Alert cloud SVG icon.
    pub const ALERT_CLOUD: Icon = crate::icon_from_path!("ui/alert-cloud.svg");
    /// Alert crossing SVG icon.
    pub const ALERT_CROSSING: Icon = crate::icon_from_path!("ui/alert-crossing.svg");
    /// Alert disabled SVG icon.
    pub const ALERT_DISABLED: Icon = crate::icon_from_path!("ui/alert-disabled.svg");
    /// Alert edit SVG icon.
    pub const ALERT_EDIT: Icon = crate::icon_from_path!("ui/alert-edit.svg");
    /// Alert history SVG icon.
    pub const ALERT_HISTORY: Icon = crate::icon_from_path!("ui/alert-history.svg");
    /// Alert indicator SVG icon.
    pub const ALERT_INDICATOR: Icon = crate::icon_from_path!("ui/alert-indicator.svg");
    /// Alert info SVG icon.
    pub const ALERT_INFO: Icon = crate::icon_from_path!("ui/alert-info.svg");
    /// Alert once SVG icon.
    pub const ALERT_ONCE: Icon = crate::icon_from_path!("ui/alert-once.svg");
    /// Alert pause SVG icon.
    pub const ALERT_PAUSE: Icon = crate::icon_from_path!("ui/alert-pause.svg");
    /// Alert percent change SVG icon.
    pub const ALERT_PERCENT_CHANGE: Icon = crate::icon_from_path!("ui/alert-percent-change.svg");
    /// Alert price above SVG icon.
    pub const ALERT_PRICE_ABOVE: Icon = crate::icon_from_path!("ui/alert-price-above.svg");
    /// Alert price below SVG icon.
    pub const ALERT_PRICE_BELOW: Icon = crate::icon_from_path!("ui/alert-price-below.svg");
    /// Alert protected SVG icon.
    pub const ALERT_PROTECTED: Icon = crate::icon_from_path!("ui/alert-protected.svg");
    /// Alert recurring SVG icon.
    pub const ALERT_RECURRING: Icon = crate::icon_from_path!("ui/alert-recurring.svg");
    /// Alert remove SVG icon.
    pub const ALERT_REMOVE: Icon = crate::icon_from_path!("ui/alert-remove.svg");
    /// Alert resume SVG icon.
    pub const ALERT_RESUME: Icon = crate::icon_from_path!("ui/alert-resume.svg");
    /// Alert settings SVG icon.
    pub const ALERT_SETTINGS: Icon = crate::icon_from_path!("ui/alert-settings.svg");
    /// Alert time SVG icon.
    pub const ALERT_TIME: Icon = crate::icon_from_path!("ui/alert-time.svg");
    /// Alert triggered SVG icon.
    pub const ALERT_TRIGGERED: Icon = crate::icon_from_path!("ui/alert-triggered.svg");
    /// Alert warning SVG icon.
    pub const ALERT_WARNING: Icon = crate::icon_from_path!("ui/alert-warning.svg");
    /// Alert watchlist SVG icon.
    pub const ALERT_WATCHLIST: Icon = crate::icon_from_path!("ui/alert-watchlist.svg");
    /// Alert webhook SVG icon.
    pub const ALERT_WEBHOOK: Icon = crate::icon_from_path!("ui/alert-webhook.svg");
    /// Calendar add SVG icon.
    pub const CALENDAR_ADD: Icon = crate::icon_from_path!("ui/calendar-add.svg");
    /// Calendar range SVG icon.
    pub const CALENDAR_RANGE: Icon = crate::icon_from_path!("ui/calendar-range.svg");
    /// Clock SVG icon.
    pub const CLOCK: Icon = crate::icon_from_path!("ui/clock.svg");
    /// Countdown SVG icon.
    pub const COUNTDOWN: Icon = crate::icon_from_path!("ui/countdown.svg");
    /// Market closed SVG icon.
    pub const MARKET_CLOSED: Icon = crate::icon_from_path!("ui/market-closed.svg");
    /// Market open SVG icon.
    pub const MARKET_OPEN: Icon = crate::icon_from_path!("ui/market-open.svg");
    /// Replay end SVG icon.
    pub const REPLAY_END: Icon = crate::icon_from_path!("ui/replay-end.svg");
    /// Replay jump date SVG icon.
    pub const REPLAY_JUMP_DATE: Icon = crate::icon_from_path!("ui/replay-jump-date.svg");
    /// Replay reset SVG icon.
    pub const REPLAY_RESET: Icon = crate::icon_from_path!("ui/replay-reset.svg");
    /// Replay speed SVG icon.
    pub const REPLAY_SPEED: Icon = crate::icon_from_path!("ui/replay-speed.svg");
    /// Replay start SVG icon.
    pub const REPLAY_START: Icon = crate::icon_from_path!("ui/replay-start.svg");
    /// Session SVG icon.
    pub const SESSION: Icon = crate::icon_from_path!("ui/session.svg");
    /// Timeframe add SVG icon.
    pub const TIMEFRAME_ADD: Icon = crate::icon_from_path!("ui/timeframe-add.svg");
    /// Timeframe SVG icon.
    pub const TIMEFRAME: Icon = crate::icon_from_path!("ui/timeframe.svg");
    /// Timezone SVG icon.
    pub const TIMEZONE: Icon = crate::icon_from_path!("ui/timezone.svg");
    /// Exit fullscreen SVG icon.
    pub const EXIT_FULLSCREEN: Icon = crate::icon_from_path!("ui/exit-fullscreen.svg");
    /// Layout bottom main SVG icon.
    pub const LAYOUT_BOTTOM_MAIN: Icon = crate::icon_from_path!("ui/layout-bottom-main.svg");
    /// Layout left main SVG icon.
    pub const LAYOUT_LEFT_MAIN: Icon = crate::icon_from_path!("ui/layout-left-main.svg");
    /// Layout right main SVG icon.
    pub const LAYOUT_RIGHT_MAIN: Icon = crate::icon_from_path!("ui/layout-right-main.svg");
    /// Layout three rows SVG icon.
    pub const LAYOUT_THREE_ROWS: Icon = crate::icon_from_path!("ui/layout-three-rows.svg");
    /// Layout top main SVG icon.
    pub const LAYOUT_TOP_MAIN: Icon = crate::icon_from_path!("ui/layout-top-main.svg");
    /// Panel bottom SVG icon.
    pub const PANEL_BOTTOM: Icon = crate::icon_from_path!("ui/panel-bottom.svg");
    /// Panel float SVG icon.
    pub const PANEL_FLOAT: Icon = crate::icon_from_path!("ui/panel-float.svg");
    /// Panel left SVG icon.
    pub const PANEL_LEFT: Icon = crate::icon_from_path!("ui/panel-left.svg");
    /// Panel right SVG icon.
    pub const PANEL_RIGHT: Icon = crate::icon_from_path!("ui/panel-right.svg");
    /// Panel top SVG icon.
    pub const PANEL_TOP: Icon = crate::icon_from_path!("ui/panel-top.svg");
    /// Workspace add SVG icon.
    pub const WORKSPACE_ADD: Icon = crate::icon_from_path!("ui/workspace-add.svg");
    /// Workspace link SVG icon.
    pub const WORKSPACE_LINK: Icon = crate::icon_from_path!("ui/workspace-link.svg");
    /// Workspace sync SVG icon.
    pub const WORKSPACE_SYNC: Icon = crate::icon_from_path!("ui/workspace-sync.svg");
    /// Align bottom SVG icon.
    pub const ALIGN_BOTTOM: Icon = crate::icon_from_path!("ui/align-bottom.svg");
    /// Align left SVG icon.
    pub const ALIGN_LEFT: Icon = crate::icon_from_path!("ui/align-left.svg");
    /// Align right SVG icon.
    pub const ALIGN_RIGHT: Icon = crate::icon_from_path!("ui/align-right.svg");
    /// Align top SVG icon.
    pub const ALIGN_TOP: Icon = crate::icon_from_path!("ui/align-top.svg");
    /// Bring to front SVG icon.
    pub const BRING_TO_FRONT: Icon = crate::icon_from_path!("ui/bring-to-front.svg");
    /// Layer add SVG icon.
    pub const LAYER_ADD: Icon = crate::icon_from_path!("ui/layer-add.svg");
    /// Layer down SVG icon.
    pub const LAYER_DOWN: Icon = crate::icon_from_path!("ui/layer-down.svg");
    /// Layer lock SVG icon.
    pub const LAYER_LOCK: Icon = crate::icon_from_path!("ui/layer-lock.svg");
    /// Layer remove SVG icon.
    pub const LAYER_REMOVE: Icon = crate::icon_from_path!("ui/layer-remove.svg");
    /// Layer unlock SVG icon.
    pub const LAYER_UNLOCK: Icon = crate::icon_from_path!("ui/layer-unlock.svg");
    /// Layer up SVG icon.
    pub const LAYER_UP: Icon = crate::icon_from_path!("ui/layer-up.svg");
    /// Object copy SVG icon.
    pub const OBJECT_COPY: Icon = crate::icon_from_path!("ui/object-copy.svg");
    /// Object group SVG icon.
    pub const OBJECT_GROUP: Icon = crate::icon_from_path!("ui/object-group.svg");
    /// Object properties SVG icon.
    pub const OBJECT_PROPERTIES: Icon = crate::icon_from_path!("ui/object-properties.svg");
    /// Object ungroup SVG icon.
    pub const OBJECT_UNGROUP: Icon = crate::icon_from_path!("ui/object-ungroup.svg");
    /// Send to back SVG icon.
    pub const SEND_TO_BACK: Icon = crate::icon_from_path!("ui/send-to-back.svg");
    /// Code autocomplete SVG icon.
    pub const CODE_AUTOCOMPLETE: Icon = crate::icon_from_path!("ui/code-autocomplete.svg");
    /// Code brackets SVG icon.
    pub const CODE_BRACKETS: Icon = crate::icon_from_path!("ui/code-brackets.svg");
    /// Code breakpoint SVG icon.
    pub const CODE_BREAKPOINT: Icon = crate::icon_from_path!("ui/code-breakpoint.svg");
    /// Code comment SVG icon.
    pub const CODE_COMMENT: Icon = crate::icon_from_path!("ui/code-comment.svg");
    /// Function SVG icon.
    pub const FUNCTION: Icon = crate::icon_from_path!("ui/function.svg");
    /// Script compile SVG icon.
    pub const SCRIPT_COMPILE: Icon = crate::icon_from_path!("ui/script-compile.svg");
    /// Script debug SVG icon.
    pub const SCRIPT_DEBUG: Icon = crate::icon_from_path!("ui/script-debug.svg");
    /// Script error SVG icon.
    pub const SCRIPT_ERROR: Icon = crate::icon_from_path!("ui/script-error.svg");
    /// Script export SVG icon.
    pub const SCRIPT_EXPORT: Icon = crate::icon_from_path!("ui/script-export.svg");
    /// Script format SVG icon.
    pub const SCRIPT_FORMAT: Icon = crate::icon_from_path!("ui/script-format.svg");
    /// Script import SVG icon.
    pub const SCRIPT_IMPORT: Icon = crate::icon_from_path!("ui/script-import.svg");
    /// Script info SVG icon.
    pub const SCRIPT_INFO: Icon = crate::icon_from_path!("ui/script-info.svg");
    /// Script library SVG icon.
    pub const SCRIPT_LIBRARY: Icon = crate::icon_from_path!("ui/script-library.svg");
    /// Script new SVG icon.
    pub const SCRIPT_NEW: Icon = crate::icon_from_path!("ui/script-new.svg");
    /// Script open SVG icon.
    pub const SCRIPT_OPEN: Icon = crate::icon_from_path!("ui/script-open.svg");
    /// Script protected SVG icon.
    pub const SCRIPT_PROTECTED: Icon = crate::icon_from_path!("ui/script-protected.svg");
    /// Script publish SVG icon.
    pub const SCRIPT_PUBLISH: Icon = crate::icon_from_path!("ui/script-publish.svg");
    /// Script run SVG icon.
    pub const SCRIPT_RUN: Icon = crate::icon_from_path!("ui/script-run.svg");
    /// Script save SVG icon.
    pub const SCRIPT_SAVE: Icon = crate::icon_from_path!("ui/script-save.svg");
    /// Script settings SVG icon.
    pub const SCRIPT_SETTINGS: Icon = crate::icon_from_path!("ui/script-settings.svg");
    /// Script stop SVG icon.
    pub const SCRIPT_STOP: Icon = crate::icon_from_path!("ui/script-stop.svg");
    /// Script version SVG icon.
    pub const SCRIPT_VERSION: Icon = crate::icon_from_path!("ui/script-version.svg");
    /// Terminal SVG icon.
    pub const TERMINAL: Icon = crate::icon_from_path!("ui/terminal.svg");
    /// Variable SVG icon.
    pub const VARIABLE: Icon = crate::icon_from_path!("ui/variable.svg");
    /// Archive SVG icon.
    pub const ARCHIVE: Icon = crate::icon_from_path!("ui/archive.svg");
    /// File chart SVG icon.
    pub const FILE_CHART: Icon = crate::icon_from_path!("ui/file-chart.svg");
    /// File copy SVG icon.
    pub const FILE_COPY: Icon = crate::icon_from_path!("ui/file-copy.svg");
    /// File delete SVG icon.
    pub const FILE_DELETE: Icon = crate::icon_from_path!("ui/file-delete.svg");
    /// File download SVG icon.
    pub const FILE_DOWNLOAD: Icon = crate::icon_from_path!("ui/file-download.svg");
    /// File edit SVG icon.
    pub const FILE_EDIT: Icon = crate::icon_from_path!("ui/file-edit.svg");
    /// File export SVG icon.
    pub const FILE_EXPORT: Icon = crate::icon_from_path!("ui/file-export.svg");
    /// File history SVG icon.
    pub const FILE_HISTORY: Icon = crate::icon_from_path!("ui/file-history.svg");
    /// File import SVG icon.
    pub const FILE_IMPORT: Icon = crate::icon_from_path!("ui/file-import.svg");
    /// File lock SVG icon.
    pub const FILE_LOCK: Icon = crate::icon_from_path!("ui/file-lock.svg");
    /// File new SVG icon.
    pub const FILE_NEW: Icon = crate::icon_from_path!("ui/file-new.svg");
    /// File save SVG icon.
    pub const FILE_SAVE: Icon = crate::icon_from_path!("ui/file-save.svg");
    /// File script SVG icon.
    pub const FILE_SCRIPT: Icon = crate::icon_from_path!("ui/file-script.svg");
    /// File search SVG icon.
    pub const FILE_SEARCH: Icon = crate::icon_from_path!("ui/file-search.svg");
    /// File settings SVG icon.
    pub const FILE_SETTINGS: Icon = crate::icon_from_path!("ui/file-settings.svg");
    /// File upload SVG icon.
    pub const FILE_UPLOAD: Icon = crate::icon_from_path!("ui/file-upload.svg");
    /// File SVG icon.
    pub const FILE: Icon = crate::icon_from_path!("ui/file.svg");
    /// Folder export SVG icon.
    pub const FOLDER_EXPORT: Icon = crate::icon_from_path!("ui/folder-export.svg");
    /// Folder favorite SVG icon.
    pub const FOLDER_FAVORITE: Icon = crate::icon_from_path!("ui/folder-favorite.svg");
    /// Folder import SVG icon.
    pub const FOLDER_IMPORT: Icon = crate::icon_from_path!("ui/folder-import.svg");
    /// Folder new SVG icon.
    pub const FOLDER_NEW: Icon = crate::icon_from_path!("ui/folder-new.svg");
    /// Folder open SVG icon.
    pub const FOLDER_OPEN: Icon = crate::icon_from_path!("ui/folder-open.svg");
    /// Folder search SVG icon.
    pub const FOLDER_SEARCH: Icon = crate::icon_from_path!("ui/folder-search.svg");
    /// Folder SVG icon.
    pub const FOLDER: Icon = crate::icon_from_path!("ui/folder.svg");
    /// Api SVG icon.
    pub const API: Icon = crate::icon_from_path!("ui/api.svg");
    /// Connection retry SVG icon.
    pub const CONNECTION_RETRY: Icon = crate::icon_from_path!("ui/connection-retry.svg");
    /// Data add SVG icon.
    pub const DATA_ADD: Icon = crate::icon_from_path!("ui/data-add.svg");
    /// Data cache SVG icon.
    pub const DATA_CACHE: Icon = crate::icon_from_path!("ui/data-cache.svg");
    /// Data cloud SVG icon.
    pub const DATA_CLOUD: Icon = crate::icon_from_path!("ui/data-cloud.svg");
    /// Data export SVG icon.
    pub const DATA_EXPORT: Icon = crate::icon_from_path!("ui/data-export.svg");
    /// Data history SVG icon.
    pub const DATA_HISTORY: Icon = crate::icon_from_path!("ui/data-history.svg");
    /// Data import SVG icon.
    pub const DATA_IMPORT: Icon = crate::icon_from_path!("ui/data-import.svg");
    /// Data live SVG icon.
    pub const DATA_LIVE: Icon = crate::icon_from_path!("ui/data-live.svg");
    /// Data locked SVG icon.
    pub const DATA_LOCKED: Icon = crate::icon_from_path!("ui/data-locked.svg");
    /// Data refresh SVG icon.
    pub const DATA_REFRESH: Icon = crate::icon_from_path!("ui/data-refresh.svg");
    /// Data remove SVG icon.
    pub const DATA_REMOVE: Icon = crate::icon_from_path!("ui/data-remove.svg");
    /// Data search SVG icon.
    pub const DATA_SEARCH: Icon = crate::icon_from_path!("ui/data-search.svg");
    /// Data settings SVG icon.
    pub const DATA_SETTINGS: Icon = crate::icon_from_path!("ui/data-settings.svg");
    /// Data source SVG icon.
    pub const DATA_SOURCE: Icon = crate::icon_from_path!("ui/data-source.svg");
    /// Data stream SVG icon.
    pub const DATA_STREAM: Icon = crate::icon_from_path!("ui/data-stream.svg");
    /// Data verified SVG icon.
    pub const DATA_VERIFIED: Icon = crate::icon_from_path!("ui/data-verified.svg");
    /// Data warning SVG icon.
    pub const DATA_WARNING: Icon = crate::icon_from_path!("ui/data-warning.svg");
    /// Latency SVG icon.
    pub const LATENCY: Icon = crate::icon_from_path!("ui/latency.svg");
    /// Offline mode SVG icon.
    pub const OFFLINE_MODE: Icon = crate::icon_from_path!("ui/offline-mode.svg");
    /// Server SVG icon.
    pub const SERVER: Icon = crate::icon_from_path!("ui/server.svg");
    /// Websocket SVG icon.
    pub const WEBSOCKET: Icon = crate::icon_from_path!("ui/websocket.svg");
    /// Accessibility SVG icon.
    pub const ACCESSIBILITY: Icon = crate::icon_from_path!("ui/accessibility.svg");
    /// Grid settings SVG icon.
    pub const GRID_SETTINGS: Icon = crate::icon_from_path!("ui/grid-settings.svg");
    /// Keyboard SVG icon.
    pub const KEYBOARD: Icon = crate::icon_from_path!("ui/keyboard.svg");
    /// Language SVG icon.
    pub const LANGUAGE: Icon = crate::icon_from_path!("ui/language.svg");
    /// Logout SVG icon.
    pub const LOGOUT: Icon = crate::icon_from_path!("ui/logout.svg");
    /// Opacity SVG icon.
    pub const OPACITY: Icon = crate::icon_from_path!("ui/opacity.svg");
    /// Scale auto SVG icon.
    pub const SCALE_AUTO: Icon = crate::icon_from_path!("ui/scale-auto.svg");
    /// Scale invert SVG icon.
    pub const SCALE_INVERT: Icon = crate::icon_from_path!("ui/scale-invert.svg");
    /// Scale linear SVG icon.
    pub const SCALE_LINEAR: Icon = crate::icon_from_path!("ui/scale-linear.svg");
    /// Scale logarithmic SVG icon.
    pub const SCALE_LOGARITHMIC: Icon = crate::icon_from_path!("ui/scale-logarithmic.svg");
    /// Scale percent SVG icon.
    pub const SCALE_PERCENT: Icon = crate::icon_from_path!("ui/scale-percent.svg");
    /// Security SVG icon.
    pub const SECURITY: Icon = crate::icon_from_path!("ui/security.svg");
    /// Stroke width SVG icon.
    pub const STROKE_WIDTH: Icon = crate::icon_from_path!("ui/stroke-width.svg");
    /// Theme dark SVG icon.
    pub const THEME_DARK: Icon = crate::icon_from_path!("ui/theme-dark.svg");
    /// Theme light SVG icon.
    pub const THEME_LIGHT: Icon = crate::icon_from_path!("ui/theme-light.svg");
    /// Theme system SVG icon.
    pub const THEME_SYSTEM: Icon = crate::icon_from_path!("ui/theme-system.svg");

    // Aliases
    /// Bell SVG icon.
    pub const BELL: Icon = ALERTS;
    /// Widget bar alerts SVG icon.
    pub const WIDGET_BAR_ALERTS: Icon = ALERTS;
    /// Candlestick SVG icon.
    pub const CANDLESTICK: Icon = CHART_CANDLES;
    /// Settings gear SVG icon.
    pub const SETTINGS_GEAR: Icon = SETTINGS;
    /// Bar chart SVG icon.
    pub const BAR_CHART: Icon = CHART_BARS;
    /// Remove SVG icon.
    pub const REMOVE: Icon = TRASH;
    /// Long pos SVG icon.
    pub const LONG_POS: Icon = LONG_POSITION;
    /// Short pos SVG icon.
    pub const SHORT_POS: Icon = SHORT_POSITION;
}

#[cfg(test)]
mod tests {
    use std::collections::HashSet;
    use std::path::Path;

    /// Asset paths of every `icon_from_path!` constant in this file.
    fn embedded_paths() -> Vec<String> {
        let needle = concat!("icon_from_path", "!(\"");
        include_str!("mod.rs")
            .split(needle)
            .skip(1)
            .map(|rest| rest[..rest.find('"').unwrap()].to_string())
            .collect()
    }

    fn svg_files(dir: &Path, out: &mut Vec<String>, root: &Path) {
        for entry in std::fs::read_dir(dir).unwrap() {
            let path = entry.unwrap().path();
            if path.is_dir() {
                svg_files(&path, out, root);
            } else if path.extension().is_some_and(|e| e == "svg") {
                let rel = path.strip_prefix(root).unwrap();
                out.push(rel.to_string_lossy().replace('\\', "/"));
            }
        }
    }

    /// The SVG loader resolves `currentColor` to black unless the root sets a colour, and egui's
    /// tint multiplies the texture, so an icon without a white root stays black in every theme.
    #[test]
    fn every_icon_file_is_embedded_and_tintable() {
        let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("assets/icons");
        let mut files = Vec::new();
        svg_files(&root, &mut files, &root);
        let embedded: HashSet<String> = embedded_paths().into_iter().collect();
        for f in &files {
            assert!(embedded.contains(f), "{f} is not embedded by any constant");
            let svg = std::fs::read_to_string(root.join(f)).unwrap();
            let open_tag = &svg[..svg.find('>').unwrap()];
            assert!(
                open_tag.contains(r##"color="#fff""##),
                "{f}: root element must set color=\"#fff\" so tinting works"
            );
        }
    }
}
