//! Saved level and background settings for the pitchfork tools.

use crate::model::LineStyle;
use serde::{Deserialize, Serialize};

/// A multiple of the median-to-tine distance, mirrored on both sides of the median.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct PitchforkLevel {
    /// Distance multiple: 0 is the median and 1 is the pair of tines.
    pub value: f32,
    /// Whether the level and its adjacent bands are drawn.
    pub visible: bool,
    /// Line colour as RGBA.
    pub color: [u8; 4],
    /// Stroke style of this level.
    pub line_style: LineStyle,
    /// Stroke width in points. `None` inherits the drawing width, as older files do.
    #[serde(default)]
    pub width: Option<f32>,
}

impl PitchforkLevel {
    /// A solid level at `value` with the given visibility and colour.
    pub fn new(value: f32, visible: bool, color: [u8; 4]) -> Self {
        Self {
            value,
            visible,
            color,
            line_style: LineStyle::Solid,
            width: None,
        }
    }
}

/// The pitchfork's level lines and the fill between adjacent levels.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct PitchforkConfig {
    /// Levels, ordered by distance when rendered; negative and non-finite values are ignored.
    pub levels: Vec<PitchforkLevel>,
    /// Whether bands between adjacent visible levels are filled.
    pub show_background: bool,
    /// Band opacity, from 0 to 255.
    pub background_opacity: u8,
}

impl PitchforkConfig {
    /// Rays of a new pitchfan, matching the former fixed ratios.
    pub fn pitchfan() -> Self {
        let levels = [
            (0.0, [120, 123, 134, 255]),
            (0.236, [242, 54, 69, 255]),
            (0.382, [255, 152, 0, 255]),
            (0.5, [76, 175, 80, 255]),
            (0.618, [8, 153, 129, 255]),
            (0.786, [0, 188, 212, 255]),
            (1.0, [41, 98, 255, 255]),
        ];
        Self {
            levels: levels
                .map(|(value, color)| {
                    let edge = value <= f32::EPSILON || (value - 1.0).abs() <= 1e-4;
                    let mut level = PitchforkLevel::new(value, true, color);
                    if !edge {
                        level.width = Some(crate::styles::stroke::HAIRLINE);
                    }
                    level
                })
                .to_vec(),
            show_background: true,
            background_opacity: 40,
        }
    }

    /// Recolour a pitchfan from the drawing's primary colour, keeping inner rays lighter.
    pub fn recolor_pitchfan(&mut self, color: [u8; 4]) {
        for level in &mut self.levels {
            let edge = level.value <= f32::EPSILON || (level.value - 1.0).abs() <= 1e-4;
            level.color = [
                color[0],
                color[1],
                color[2],
                if edge { color[3] } else { 150 },
            ];
        }
    }

    /// Standard level ratios offered in the Inputs list (median … 2× tine).
    pub const STANDARD_VALUES: [f32; 10] =
        [0.0, 0.25, 0.382, 0.5, 0.618, 0.75, 1.0, 1.5, 1.75, 2.0];

    /// Inserts any missing standard ratios so older drawings get the full Inputs list.
    pub fn ensure_standard_levels(&mut self) {
        for &value in &Self::STANDARD_VALUES {
            if !self.levels.iter().any(|l| (l.value - value).abs() < 1e-4) {
                let visible = (value - 0.0).abs() < 1e-4
                    || (value - 0.5).abs() < 1e-4
                    || (value - 1.0).abs() < 1e-4;
                self.levels
                    .push(PitchforkLevel::new(value, visible, [33, 150, 243, 255]));
            }
        }
        self.levels.sort_by(|a, b| a.value.total_cmp(&b.value));
    }
}

impl Default for PitchforkConfig {
    fn default() -> Self {
        // Median (0), half (0.5) and tines (1) on; remaining ratios available in Inputs.
        let blue = [33, 150, 243, 255];
        let median = [120, 123, 134, 255];
        let half = [66, 165, 245, 255];
        let levels = [
            (0.0, true, median),
            (0.25, false, blue),
            (0.382, false, blue),
            (0.5, true, half),
            (0.618, false, blue),
            (0.75, false, blue),
            (1.0, true, blue),
            (1.5, false, blue),
            (1.75, false, blue),
            (2.0, false, blue),
        ]
        .map(|(value, visible, color)| PitchforkLevel::new(value, visible, color))
        .to_vec();
        Self {
            levels,
            show_background: true,
            background_opacity: 20,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn pitchfan_defaults_are_coloured_and_filled() {
        let config = PitchforkConfig::pitchfan();
        assert!(config.show_background);
        assert!(config.background_opacity > 0);
        assert_eq!(config.levels.len(), 7);
        assert!(
            config
                .levels
                .windows(2)
                .all(|pair| pair[0].color != pair[1].color)
        );

        let drawing = crate::drawings::Drawing::with_color(
            1,
            crate::drawings::DrawingToolType::Pitchfan,
            [9, 8, 7, 255],
        );
        let levels = &drawing.pitchfork.unwrap().levels;
        assert!(
            levels.windows(2).all(|pair| pair[0].color != pair[1].color),
            "the toolbar's initial drawing colour must not flatten the default palette"
        );
    }
}
