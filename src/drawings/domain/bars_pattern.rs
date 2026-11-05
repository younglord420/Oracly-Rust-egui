//! The copied bars of a Bars Pattern drawing and how it shows them.

use super::{Drawing, DrawingToolType};
use crate::model::Bar;
use serde::{Deserialize, Serialize};

/// The most bars one Bars Pattern copies.
pub const MAX_PATTERN_BARS: usize = 2000;

/// What a Bars Pattern draws for each copied bar.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
pub enum BarsPatternMode {
    /// A column from the high to the low.
    #[default]
    HlBars,
    /// A column from the open to the close.
    OcBars,
    /// A line through the opens.
    LineOpen,
    /// A line through the highs.
    LineHigh,
    /// A line through the lows.
    LineLow,
    /// A line through the closes.
    LineClose,
}

impl BarsPatternMode {
    /// Every mode, in menu order.
    pub const ALL: [Self; 6] = [
        Self::HlBars,
        Self::OcBars,
        Self::LineOpen,
        Self::LineHigh,
        Self::LineLow,
        Self::LineClose,
    ];

    /// Name shown in the settings.
    pub fn name(self) -> &'static str {
        match self {
            Self::HlBars => "HL Bars",
            Self::OcBars => "OC Bars",
            Self::LineOpen => "Line - Open",
            Self::LineHigh => "Line - High",
            Self::LineLow => "Line - Low",
            Self::LineClose => "Line - Close",
        }
    }
}

/// A Bars Pattern keeps the bars it copied, so moving it keeps their shape.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
#[serde(default)]
pub struct BarsPatternConfig {
    /// What each bar is drawn as.
    pub mode: BarsPatternMode,
    /// Draws the bars in reverse order.
    pub mirrored: bool,
    /// Draws the bars upside down.
    pub flipped: bool,
    /// Open, high, low and close of each copied bar, oldest first; empty until it is placed.
    pub bars: Vec<[f64; 4]>,
    /// Price of the first anchor when the bars were copied.
    pub origin: f64,
}

impl Drawing {
    /// The settings of a Bars Pattern, or the defaults.
    pub fn bars_pattern_config(&self) -> BarsPatternConfig {
        self.bars_pattern.clone().unwrap_or_default()
    }

    /// Copies the bars between the two anchors of a placed Bars Pattern that has none yet.
    /// Returns whether it copied any.
    pub(crate) fn capture_bars_pattern(&mut self, bars: &[Bar]) -> bool {
        if self.tool_type != DrawingToolType::BarsPattern || !self.completed {
            return false;
        }
        let mut config = self.bars_pattern_config();
        if !config.bars.is_empty() {
            return false;
        }
        let [from, to, ..] = self.chart_points.as_slice() else {
            return false;
        };
        let Some((_, source)) =
            crate::drawings::rendering::bars_between(bars, from.bar_idx, to.bar_idx)
        else {
            return false;
        };
        config.bars = source
            .iter()
            .filter(|bar| !bar.is_whitespace())
            .take(MAX_PATTERN_BARS)
            .map(|bar| [bar.open, bar.high, bar.low, bar.close])
            .collect();
        config.origin = from.price;
        let copied = !config.bars.is_empty();
        self.bars_pattern = Some(config);
        copied
    }

    /// The bars a Bars Pattern draws, from its first anchor's bar on: the copied ones moved with
    /// the first anchor's price, or while it is being placed, the bars between its anchors.
    /// Mirroring reverses their order and flipping turns them upside down about the first open.
    pub(crate) fn bars_pattern_bars(&self, bars: &[Bar]) -> Vec<[f64; 4]> {
        let config = self.bars_pattern_config();
        let [from, to, ..] = self.chart_points.as_slice() else {
            return Vec::new();
        };
        let (mut copied, origin) = if config.bars.is_empty() {
            let live = crate::drawings::rendering::bars_between(bars, from.bar_idx, to.bar_idx)
                .map(|(_, source)| {
                    source
                        .iter()
                        .filter(|bar| !bar.is_whitespace())
                        .take(MAX_PATTERN_BARS)
                        .map(|bar| [bar.open, bar.high, bar.low, bar.close])
                        .collect()
                })
                .unwrap_or_default();
            (live, from.price)
        } else {
            (config.bars.clone(), config.origin)
        };
        let shift = from.price - origin;
        if config.mirrored {
            copied.reverse();
            for bar in &mut copied {
                bar.swap(0, 3);
            }
        }
        let pivot = copied.first().map_or(0.0, |bar| bar[0]);
        for bar in &mut copied {
            if config.flipped {
                *bar = [
                    2.0 * pivot - bar[0],
                    2.0 * pivot - bar[2],
                    2.0 * pivot - bar[1],
                    2.0 * pivot - bar[3],
                ];
            }
            for price in bar.iter_mut() {
                *price += shift;
            }
        }
        copied
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::drawings::ChartPoint;
    use crate::drawings::rendering::sample_bars;

    /// Placing copies the bars between the anchors; moving the drawing then moves the copy
    /// without reading other bars, and mirroring and flipping reorder and invert it.
    #[test]
    fn the_copy_keeps_its_bars_when_moved() {
        let bars = sample_bars();
        let mut drawing = Drawing::new(1, DrawingToolType::BarsPattern);
        drawing.chart_points = vec![ChartPoint::new(1.0, 12.0), ChartPoint::new(3.0, 15.0)];
        drawing.completed = true;
        assert!(drawing.capture_bars_pattern(&bars));
        assert!(!drawing.capture_bars_pattern(&bars));
        let copied: Vec<_> = bars[1..=3]
            .iter()
            .map(|b| [b.open, b.high, b.low, b.close])
            .collect();
        assert_eq!(drawing.bars_pattern_bars(&bars), copied);
        drawing.chart_points[0] = ChartPoint::new(8.0, 14.0);
        let moved = drawing.bars_pattern_bars(&[]);
        assert_eq!(moved.len(), 3);
        for (moved, bar) in moved.iter().zip(&copied) {
            for (m, b) in moved.iter().zip(bar) {
                assert!((m - (b + 2.0)).abs() < 1e-9);
            }
        }
        let mut config = drawing.bars_pattern_config();
        config.mirrored = true;
        drawing.bars_pattern = Some(config);
        let mirrored = drawing.bars_pattern_bars(&[]);
        assert_eq!(mirrored[0][0], moved[2][3]);
        assert_eq!(mirrored[0][3], moved[2][0]);
        let mut config = drawing.bars_pattern_config();
        config.mirrored = false;
        config.flipped = true;
        drawing.bars_pattern = Some(config);
        let flipped = drawing.bars_pattern_bars(&[]);
        assert_eq!(flipped[0][0], moved[0][0]);
        assert!((flipped[0][1] - (2.0 * moved[0][0] - moved[0][2])).abs() < 1e-9);
    }
}
