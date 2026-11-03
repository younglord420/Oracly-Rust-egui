//! Saved levels of the Gann tools: the grid of a Gann box or square, the fan and arcs of a
//! Gann square, and the rays of a Gann fan. Each level has its own visibility and colour.

use super::options::fib_ratio_color;
use crate::config::DrawingToolType;
use serde::{Deserialize, Serialize};

const RED: [u8; 4] = [242, 54, 69, 255];
const ORANGE: [u8; 4] = [255, 152, 0, 255];
const AMBER: [u8; 4] = [255, 183, 77, 255];
const GREEN: [u8; 4] = [76, 175, 80, 255];
const TEAL: [u8; 4] = [8, 153, 129, 255];
const BLUE: [u8; 4] = [91, 156, 246, 255];
const AQUA: [u8; 4] = [38, 166, 154, 255];
const PURPLE: [u8; 4] = [156, 39, 176, 255];
/// The 1x1 line: chart ink rather than a hue.
const UNITY: [u8; 4] = [178, 181, 190, 255];
/// Colour of the diagonals when the grid has no 1 level.
pub(crate) const GANN_ANCHOR_COLOR: [u8; 4] = [120, 123, 134, 255];

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

/// One saved level of a Gann tool.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct GannLevel {
    /// Whether this level is painted.
    pub enabled: bool,
    /// The level's ratio; what it measures depends on the list holding it.
    pub ratio: f32,
    /// Line colour as unmultiplied RGBA.
    pub color: [u8; 4],
}

impl GannLevel {
    /// Creates a visible level.
    pub fn new(ratio: f32, color: [u8; 4]) -> Self {
        Self {
            enabled: true,
            ratio,
            color,
        }
    }
}

impl Default for GannLevel {
    fn default() -> Self {
        Self::new(1.0, UNITY)
    }
}

/// Levels of the Gann tools. Lines take their width and dash from the drawing.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(from = "SavedGannConfig")]
pub struct GannConfig {
    /// Divisions of both the price and the time side of a Gann box or square, as fractions
    /// measured from the first anchor; values outside 0..=1 are ignored.
    pub levels: Vec<GannLevel>,
    /// Whether a Gann square draws its grid.
    pub show_grid: bool,
    /// Whether a Gann square draws its fan.
    pub show_fan: bool,
    /// Fan lines of a Gann square in price units per time unit, where 1 is the diagonal.
    /// Must be positive and finite.
    pub fan_lines: Vec<GannLevel>,
    /// The corners the fan lines of a Gann square start from.
    pub fan_direction: GannFanDirection,
    /// Whether a Gann square draws its arcs.
    pub show_arcs: bool,
    /// Arcs of a Gann square around its first anchor, as fractions of its sides. Must be
    /// positive and finite; the part outside the square is not drawn.
    pub arcs: Vec<GannLevel>,
    /// Rays of a Gann fan in price units per time unit, where 1 passes through the second
    /// anchor. Must be positive and finite.
    pub fan_angles: Vec<GannLevel>,
}

impl Default for GannConfig {
    fn default() -> Self {
        Self {
            levels: grid_levels(&[0.0, 0.25, 0.382, 0.5, 0.618, 0.75, 1.0]),
            show_grid: true,
            show_fan: true,
            fan_lines: [
                (1.0 / 3.0, RED),
                (0.5, ORANGE),
                (1.0, UNITY),
                (2.0, TEAL),
                (3.0, BLUE),
            ]
            .map(|(ratio, color)| GannLevel::new(ratio, color))
            .to_vec(),
            fan_direction: GannFanDirection::default(),
            show_arcs: true,
            arcs: [(0.25, RED), (0.5, ORANGE), (0.75, GREEN), (1.0, TEAL)]
                .map(|(ratio, color)| GannLevel::new(ratio, color))
                .to_vec(),
            fan_angles: [
                (0.125, RED),
                (0.25, ORANGE),
                (1.0 / 3.0, AMBER),
                (0.5, GREEN),
                (1.0, UNITY),
                (2.0, TEAL),
                (3.0, BLUE),
                (4.0, AQUA),
                (8.0, PURPLE),
            ]
            .map(|(ratio, color)| GannLevel::new(ratio, color))
            .to_vec(),
        }
    }
}

impl GannConfig {
    /// The levels a new drawing of `tool` starts with: the fixed square divides into quarters.
    pub fn for_tool(tool: DrawingToolType) -> Self {
        let mut config = Self::default();
        if tool == DrawingToolType::GannFixed {
            config.levels = grid_levels(&[0.0, 0.25, 0.5, 0.75, 1.0]);
        }
        config
    }

    /// Colour of the diagonals of a Gann box: that of the grid's 1 level.
    pub(crate) fn diagonal_color(&self) -> [u8; 4] {
        self.levels
            .iter()
            .find(|l| (l.ratio - 1.0).abs() < 1e-6)
            .map_or(GANN_ANCHOR_COLOR, |l| l.color)
    }
}

fn grid_levels(ratios: &[f32]) -> Vec<GannLevel> {
    ratios
        .iter()
        .enumerate()
        .map(|(i, &ratio)| GannLevel::new(ratio, fib_ratio_color(ratio, i)))
        .collect()
}

/// `ratio` without trailing zeros, to three decimals.
fn short(ratio: f32) -> String {
    let text = format!("{ratio:.3}");
    text.trim_end_matches('0').trim_end_matches('.').to_owned()
}

/// Label of a Gann fan ray: price units per time unit, as `8/1` or `1/3`.
pub fn gann_fan_label(ratio: f32) -> String {
    if ratio >= 1.0 {
        format!("{}/1", short(ratio))
    } else {
        format!("1/{}", short(1.0 / ratio))
    }
}

/// Label of a Gann square fan line: time units by price units, as `3x1` or `1x2`.
pub fn gann_square_fan_label(ratio: f32) -> String {
    if ratio >= 1.0 {
        format!("1x{}", short(ratio))
    } else {
        format!("{}x1", short(1.0 / ratio))
    }
}

/// Price units per time unit of a typed fan ratio: a plain number, or two numbers joined by
/// `/` (price over time) or `x` (time by price).
pub fn parse_gann_ratio(text: &str) -> Option<f32> {
    let text = text.trim();
    let pair = |sep: char| {
        let (a, b) = text.split_once(sep)?;
        Some((a.trim().parse::<f32>().ok()?, b.trim().parse::<f32>().ok()?))
    };
    let ratio = if let Some((price, time)) = pair('/') {
        price / time
    } else if let Some((time, price)) = pair('x').or_else(|| pair('X')).or_else(|| pair('×')) {
        price / time
    } else {
        text.parse().ok()?
    };
    (ratio.is_finite() && ratio > 0.0).then_some(ratio)
}

/// The saved form, which also reads the plain-number lists of earlier files.
#[derive(Deserialize, Default)]
#[serde(default)]
struct SavedGannConfig {
    levels: Option<Vec<GannLevel>>,
    price_levels: Option<Vec<f32>>,
    time_levels: Option<Vec<f32>>,
    show_grid: Option<bool>,
    show_fan: Option<bool>,
    fan_lines: Option<Vec<GannLevel>>,
    fan_ratios: Option<Vec<f32>>,
    fan_direction: Option<GannFanDirection>,
    show_arcs: Option<bool>,
    arcs: Option<Vec<GannLevel>>,
    arc_ratios: Option<Vec<f32>>,
    fan_angles: Option<Vec<GannLevel>>,
}

/// Levels at `ratios`, coloured like the default level of the same ratio.
fn from_ratios(ratios: &[f32], defaults: &[GannLevel]) -> Vec<GannLevel> {
    ratios
        .iter()
        .enumerate()
        .map(|(i, &ratio)| {
            let color = defaults
                .iter()
                .find(|l| (l.ratio - ratio).abs() < 1e-4)
                .map_or_else(|| fib_ratio_color(ratio, i), |l| l.color);
            GannLevel::new(ratio, color)
        })
        .collect()
}

impl From<SavedGannConfig> for GannConfig {
    fn from(saved: SavedGannConfig) -> Self {
        let defaults = Self::default();
        let levels = saved.levels.unwrap_or_else(|| {
            if saved.price_levels.is_none() && saved.time_levels.is_none() {
                return defaults.levels.clone();
            }
            // Earlier files divided price and time separately; both sides now share one list.
            let mut ratios: Vec<f32> = saved
                .price_levels
                .into_iter()
                .chain(saved.time_levels)
                .flatten()
                .collect();
            ratios.sort_by(f32::total_cmp);
            ratios.dedup_by(|a, b| (*a - *b).abs() < 1e-6);
            from_ratios(&ratios, &defaults.levels)
        });
        let fan_lines = saved.fan_lines.unwrap_or_else(|| {
            saved.fan_ratios.map_or_else(
                || defaults.fan_lines.clone(),
                |r| from_ratios(&r, &defaults.fan_lines),
            )
        });
        let arcs = saved.arcs.unwrap_or_else(|| {
            saved.arc_ratios.map_or_else(
                || defaults.arcs.clone(),
                |r| from_ratios(&r, &defaults.arcs),
            )
        });
        Self {
            levels,
            show_grid: saved.show_grid.unwrap_or(defaults.show_grid),
            show_fan: saved.show_fan.unwrap_or(defaults.show_fan),
            fan_lines,
            fan_direction: saved.fan_direction.unwrap_or(defaults.fan_direction),
            show_arcs: saved.show_arcs.unwrap_or(defaults.show_arcs),
            arcs,
            fan_angles: saved.fan_angles.unwrap_or(defaults.fan_angles),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn labels_and_typed_ratios_round_trip() {
        for (ratio, fan, square) in [
            (8.0, "8/1", "1x8"),
            (1.0, "1/1", "1x1"),
            (0.5, "1/2", "2x1"),
            (1.0 / 3.0, "1/3", "3x1"),
            (0.125, "1/8", "8x1"),
        ] {
            assert_eq!(gann_fan_label(ratio), fan);
            assert_eq!(gann_square_fan_label(ratio), square);
            for text in [fan, square] {
                let parsed = parse_gann_ratio(text).unwrap();
                assert!((parsed - ratio).abs() < 1e-5, "{text}: {parsed}");
            }
        }
        assert_eq!(parse_gann_ratio("2.5"), Some(2.5));
        assert_eq!(parse_gann_ratio("0"), None);
        assert_eq!(parse_gann_ratio("1/0"), None);
        assert_eq!(parse_gann_ratio("abc"), None);
    }

    #[test]
    fn earlier_files_keep_their_divisions_and_ratios() {
        let config: GannConfig = serde_json::from_value(serde_json::json!({
            "price_levels": [0.5, 0.25],
            "time_levels": [0.5, 0.75],
            "fan_ratios": [2.0, 1.7],
            "arc_ratios": [0.5],
            "show_arcs": false,
            "fan_angles": [{"enabled": false, "ratio": 2.5, "color": [1, 2, 3, 4],
                            "line_width": 3.0, "line_style": "Dashed"}]
        }))
        .unwrap();
        let ratios = |levels: &[GannLevel]| levels.iter().map(|l| l.ratio).collect::<Vec<_>>();
        assert_eq!(ratios(&config.levels), [0.25, 0.5, 0.75]);
        assert_eq!(ratios(&config.fan_lines), [2.0, 1.7]);
        assert_eq!(
            config.fan_lines[0].color, TEAL,
            "a known ratio keeps its colour"
        );
        assert_eq!(ratios(&config.arcs), [0.5]);
        assert!(!config.show_arcs && config.show_grid);
        assert_eq!(
            config.fan_angles,
            [GannLevel {
                enabled: false,
                ratio: 2.5,
                color: [1, 2, 3, 4]
            }]
        );

        let saved = serde_json::to_value(&config).unwrap();
        assert_eq!(serde_json::from_value::<GannConfig>(saved).unwrap(), config);
        let empty: GannConfig = serde_json::from_value(serde_json::json!({})).unwrap();
        assert_eq!(empty, GannConfig::default());
    }

    #[test]
    fn defaults_follow_the_level_colour_convention() {
        let config = GannConfig::default();
        let colors = |levels: &[GannLevel]| levels.iter().map(|l| l.color).collect::<Vec<_>>();
        assert_eq!(
            colors(&config.fan_angles),
            [RED, ORANGE, AMBER, GREEN, UNITY, TEAL, BLUE, AQUA, PURPLE]
        );
        assert_eq!(colors(&config.fan_lines), [RED, ORANGE, UNITY, TEAL, BLUE]);
        assert_eq!(colors(&config.arcs), [RED, ORANGE, GREEN, TEAL]);
        assert_eq!(
            config.levels.iter().map(|l| l.ratio).collect::<Vec<_>>(),
            [0.0, 0.25, 0.382, 0.5, 0.618, 0.75, 1.0]
        );
        assert_eq!(config.diagonal_color(), config.levels[6].color);
        assert_eq!(
            GannConfig::for_tool(DrawingToolType::GannFixed)
                .levels
                .iter()
                .map(|l| l.ratio)
                .collect::<Vec<_>>(),
            [0.0, 0.25, 0.5, 0.75, 1.0]
        );
    }
}
