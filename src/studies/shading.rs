//! Per-bar study colour ramps and vertical washes.
//!
//! Transparencies are 0 (opaque) to 100 (invisible). Colour blends are sRGB byte lerps.

use super::palette;
use crate::studies::{Indicator, IndicatorValues};
use egui::Color32;

/// One bar of a vertical wash: `top_color` at `top_value`, `bottom_color` at `bottom_value`.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct GradientStop {
    /// Value where [`top_color`](Self::top_color) sits.
    pub top_value: f64,
    /// Value where [`bottom_color`](Self::bottom_color) sits.
    pub bottom_value: f64,
    /// Colour at [`top_value`](Self::top_value).
    pub top_color: Color32,
    /// Colour at [`bottom_value`](Self::bottom_value).
    pub bottom_color: Color32,
}

/// How a study's main line, wash and zones are coloured.
#[derive(Clone, Debug, PartialEq, serde::Serialize, serde::Deserialize)]
#[non_exhaustive]
pub enum ShadingKind {
    /// A line that ramps between oversold, the midpoint and overbought.
    Bounded {
        /// Output line that ramps.
        line: usize,
        /// Overbought threshold.
        overbought: f64,
        /// Oversold threshold.
        oversold: f64,
        /// Top of the scale, where the overbought zone ends.
        top: f64,
        /// Bottom of the scale, where the oversold zone ends.
        bottom: f64,
        /// Optional dotted midline.
        midline: Option<f64>,
        /// Flat fill between the two thresholds.
        band_fill: bool,
        /// Wash from the line back to the midpoint.
        midpoint_wash: bool,
        /// Zone transparency, 0 opaque to 100 invisible.
        zone_t: u8,
    },
    /// A flat band between two fixed levels. The lines keep their own colours.
    BandOnly {
        /// Upper level.
        upper: f64,
        /// Lower level.
        lower: f64,
    },
    /// A line coloured by its side of a baseline, washed toward that baseline.
    Centered {
        /// Output line.
        line: usize,
        /// Baseline the wash fades into.
        baseline: f64,
        /// Extra dashed levels.
        extra_levels: Vec<f64>,
        /// The baseline is dotted rather than dashed.
        baseline_dotted: bool,
        /// Optional zones `(overbought, oversold, top, bottom)`.
        zones: Option<(f64, f64, f64, f64)>,
    },
    /// A line coloured by its side of a signal line, washed toward that line.
    Regime {
        /// Output line.
        line: usize,
        /// Signal line it is compared with.
        signal: usize,
    },
    /// A line coloured by its side of a signal line, with a flat tint between them.
    Spread {
        /// Output line.
        line: usize,
        /// Signal line.
        signal: usize,
        /// Flat fill transparency.
        fill_t: u8,
    },
    /// A channel whose basis follows the close and whose interior fades from bear to bull.
    Channel {
        /// Upper band line.
        upper: usize,
        /// Lower band line.
        lower: usize,
        /// Basis line, coloured by the close.
        basis: Option<usize>,
    },
    /// A flat fill between two band lines.
    FlatChannel {
        /// Upper band line.
        upper: usize,
        /// Lower band line.
        lower: usize,
        /// Fill transparency.
        fill_t: u8,
    },
    /// Four histogram colours: growing and fading on each side of zero.
    MomentumColumns {
        /// Histogram line.
        line: usize,
        /// Transparency of the faded side. `0` uses the explicit faded colours.
        faded_t: u8,
    },
    /// Histogram colour by whether the value rose from the previous bar.
    RisingColumns {
        /// Histogram line.
        line: usize,
    },
    /// A line that blends from neutral to an accent, washed to zero.
    Intensity {
        /// Output line.
        line: usize,
        /// Range the blend is measured against.
        range: IntensityRange,
    },
    /// A line coloured by whether the close is above it.
    PriceSide {
        /// Output line.
        line: usize,
    },
    /// ADX borrows the leading directional line's colour above a key level.
    AdxInk {
        /// ADX line.
        adx: usize,
        /// Plus directional line.
        plus: usize,
        /// Minus directional line.
        minus: usize,
        /// Level above which a trend is coloured.
        key_level: f64,
    },
    /// A cumulative line coloured by its bar-to-bar slope.
    Slope {
        /// Output line.
        line: usize,
    },
}

/// Window an intensity blend is measured in.
#[derive(Clone, Copy, Debug, PartialEq, serde::Serialize, serde::Deserialize)]
pub enum IntensityRange {
    /// Lowest and highest reading of the last `n` bars.
    Rolling(usize),
    /// From zero up to a fixed stress threshold.
    Threshold(f64),
}

/// Colours and switches the user can edit. Defaults come from the study.
#[derive(Clone, Debug, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct StudyShading {
    /// Which pattern this study uses.
    pub kind: ShadingKind,
    /// Whether the wash or channel fill is drawn.
    pub gradient_fill: bool,
    /// Bullish, oversold, trending or rising colour.
    pub bull: Color32,
    /// Bearish, overbought, choppy or falling colour.
    pub bear: Color32,
    /// Momentum column colour above zero while falling.
    pub faded_bull: Color32,
    /// Momentum column colour below zero while rising.
    pub faded_bear: Color32,
    /// Intensity accent.
    pub accent: Color32,
    /// Band or channel fill.
    pub band: Color32,
    /// Level line colour.
    pub levels: Color32,
    /// Wash strength at the line, 0 opaque to 100 invisible.
    pub value_t: u8,
}

impl StudyShading {
    fn base(kind: ShadingKind) -> Self {
        Self {
            kind,
            gradient_fill: true,
            bull: palette::UP,
            bear: palette::DOWN,
            faded_bull: transp(palette::UP, 50),
            faded_bear: transp(palette::DOWN, 50),
            accent: palette::AMBER,
            band: transp(palette::MUTED, 90),
            levels: palette::MUTED,
            value_t: 50,
        }
    }

    /// A bounded oscillator. `bull` is the oversold ink and `bear` the overbought ink.
    pub fn bounded(line: usize, overbought: f64, oversold: f64, top: f64, bottom: f64) -> Self {
        Self::base(ShadingKind::Bounded {
            line,
            overbought,
            oversold,
            top,
            bottom,
            midline: None,
            band_fill: false,
            midpoint_wash: false,
            zone_t: 92,
        })
    }

    /// A flat band between two levels.
    pub fn band_only(upper: f64, lower: f64) -> Self {
        Self::base(ShadingKind::BandOnly { upper, lower })
    }

    /// A baseline-centred oscillator.
    pub fn centered(line: usize, baseline: f64) -> Self {
        Self::base(ShadingKind::Centered {
            line,
            baseline,
            extra_levels: Vec::new(),
            baseline_dotted: false,
            zones: None,
        })
    }

    /// Colour by the side of a signal line.
    pub fn regime(line: usize, signal: usize) -> Self {
        Self::base(ShadingKind::Regime { line, signal })
    }

    /// A tinted spread against a signal line.
    pub fn spread(line: usize, signal: usize, fill_t: u8) -> Self {
        Self::base(ShadingKind::Spread {
            line,
            signal,
            fill_t,
        })
    }

    /// A directional channel.
    pub fn channel(upper: usize, lower: usize, basis: Option<usize>) -> Self {
        Self::base(ShadingKind::Channel {
            upper,
            lower,
            basis,
        })
    }

    /// A flat channel fill.
    pub fn flat_channel(upper: usize, lower: usize, fill_t: u8) -> Self {
        let mut shading = Self::base(ShadingKind::FlatChannel {
            upper,
            lower,
            fill_t,
        });
        shading.band = transp(palette::MUTED, fill_t);
        shading
    }

    /// Four-tone histogram columns.
    pub fn momentum(line: usize) -> Self {
        Self::base(ShadingKind::MomentumColumns { line, faded_t: 0 })
    }

    /// Columns coloured by the previous bar.
    pub fn rising(line: usize) -> Self {
        Self::base(ShadingKind::RisingColumns { line })
    }

    /// An intensity blend washed to zero.
    pub fn intensity(line: usize, range: IntensityRange) -> Self {
        Self::base(ShadingKind::Intensity { line, range })
    }

    /// Colour a line by the close.
    pub fn price_side(line: usize) -> Self {
        Self::base(ShadingKind::PriceSide { line })
    }

    /// ADX ink above a key level.
    pub fn adx(adx: usize, plus: usize, minus: usize, key_level: f64) -> Self {
        Self::base(ShadingKind::AdxInk {
            adx,
            plus,
            minus,
            key_level,
        })
    }

    /// Colour by the bar-to-bar slope.
    pub fn slope(line: usize) -> Self {
        Self::base(ShadingKind::Slope { line })
    }

    /// Dotted midline for a bounded oscillator.
    pub fn with_midline(mut self, midline: f64) -> Self {
        if let ShadingKind::Bounded { midline: slot, .. } = &mut self.kind {
            *slot = Some(midline);
        }
        self
    }

    /// Flat fill between the bounded thresholds.
    pub fn with_band(mut self, on: bool) -> Self {
        if let ShadingKind::Bounded { band_fill, .. } = &mut self.kind {
            *band_fill = on;
        }
        self
    }

    /// Wash a bounded line back to its midpoint.
    pub fn with_midpoint_wash(mut self) -> Self {
        if let ShadingKind::Bounded { midpoint_wash, .. } = &mut self.kind {
            *midpoint_wash = true;
        }
        self
    }

    /// Zone transparency.
    pub fn with_zone_t(mut self, zone_t: u8) -> Self {
        if let ShadingKind::Bounded { zone_t: slot, .. } = &mut self.kind {
            *slot = zone_t;
        }
        self
    }

    /// Draw the line colours without a wash.
    pub fn without_fill(mut self) -> Self {
        self.gradient_fill = false;
        self
    }

    /// Wash strength at the line.
    pub fn with_value_t(mut self, value_t: u8) -> Self {
        self.value_t = value_t;
        self
    }

    /// Draw the centred baseline dotted.
    pub fn dotted(mut self) -> Self {
        if let ShadingKind::Centered {
            baseline_dotted, ..
        } = &mut self.kind
        {
            *baseline_dotted = true;
        }
        self
    }

    /// Extra dashed levels on a centred oscillator.
    pub fn with_levels(mut self, levels: Vec<f64>) -> Self {
        if let ShadingKind::Centered { extra_levels, .. } = &mut self.kind {
            *extra_levels = levels;
        }
        self
    }

    /// Zones beyond the centred thresholds.
    pub fn with_zones(mut self, overbought: f64, oversold: f64, top: f64, bottom: f64) -> Self {
        if let ShadingKind::Centered { zones, .. } = &mut self.kind {
            *zones = Some((overbought, oversold, top, bottom));
        }
        self
    }

    /// Faded momentum transparency instead of the explicit faded colours.
    pub fn with_faded_t(mut self, faded_t: u8) -> Self {
        if let ShadingKind::MomentumColumns { faded_t: slot, .. } = &mut self.kind {
            *slot = faded_t;
        }
        self
    }

    /// Replace the intensity accent.
    pub fn with_accent(mut self, color: Color32) -> Self {
        self.accent = color;
        self
    }

    /// Replace the bullish ink.
    pub fn with_bull(mut self, color: Color32) -> Self {
        self.bull = color;
        self
    }

    /// Replace the bearish ink.
    pub fn with_bear(mut self, color: Color32) -> Self {
        self.bear = color;
        self
    }

    /// Replace the band fill.
    pub fn with_band_color(mut self, color: Color32) -> Self {
        self.band = color;
        self
    }

    /// The output line this pattern colours, when it has one.
    pub fn colored_line(&self) -> Option<usize> {
        Some(match self.kind {
            ShadingKind::Bounded { line, .. }
            | ShadingKind::Centered { line, .. }
            | ShadingKind::Regime { line, .. }
            | ShadingKind::Spread { line, .. }
            | ShadingKind::MomentumColumns { line, .. }
            | ShadingKind::RisingColumns { line }
            | ShadingKind::Intensity { line, .. }
            | ShadingKind::PriceSide { line }
            | ShadingKind::Slope { line } => line,
            ShadingKind::AdxInk { adx, .. } => adx,
            ShadingKind::Channel { .. }
            | ShadingKind::FlatChannel { .. }
            | ShadingKind::BandOnly { .. } => return None,
        })
    }

    /// Finite thresholds and transparencies the dialog accepts.
    pub fn validate(&self) -> Result<(), &'static str> {
        if self.value_t > 100 {
            return Err("wash transparency must be at most 100");
        }
        let finite = |value: f64| value.is_finite();
        match &self.kind {
            ShadingKind::Bounded {
                overbought,
                oversold,
                top,
                bottom,
                zone_t,
                ..
            } => {
                if *zone_t > 100 {
                    return Err("zone transparency must be at most 100");
                }
                if ![*overbought, *oversold, *top, *bottom]
                    .into_iter()
                    .all(finite)
                {
                    return Err("levels must be finite");
                }
                if *overbought <= *oversold {
                    return Err("overbought must be above oversold");
                }
            }
            ShadingKind::BandOnly { upper, lower } => {
                if !finite(*upper) || !finite(*lower) {
                    return Err("levels must be finite");
                }
                if *upper <= *lower {
                    return Err("overbought must be above oversold");
                }
            }
            ShadingKind::Centered {
                baseline,
                extra_levels,
                zones,
                ..
            } => {
                if !finite(*baseline) || !extra_levels.iter().copied().all(finite) {
                    return Err("levels must be finite");
                }
                if let Some((overbought, oversold, _, _)) = zones
                    && *overbought <= *oversold
                {
                    return Err("overbought must be above oversold");
                }
            }
            ShadingKind::AdxInk { key_level, .. } => {
                if !finite(*key_level) {
                    return Err("levels must be finite");
                }
            }
            ShadingKind::Intensity {
                range: IntensityRange::Threshold(threshold),
                ..
            } if !finite(*threshold) => return Err("levels must be finite"),
            _ => {}
        }
        Ok(())
    }
}

/// Same RGB at alpha `(100 - transparency) / 100`.
pub(crate) fn transp(color: Color32, transparency: u8) -> Color32 {
    let transparency = transparency.min(100);
    let alpha = ((100 - u32::from(transparency)) * 255).div_ceil(100) as u8;
    Color32::from_rgba_unmultiplied(color.r(), color.g(), color.b(), alpha)
}

/// sRGB byte blend of `from` toward `to` by where `value` sits in `[low, high]`.
pub(crate) fn blend(value: f64, low: f64, high: f64, from: Color32, to: Color32) -> Color32 {
    if !value.is_finite() {
        return from;
    }
    let span = high - low;
    let t = if span == 0.0 {
        0.0
    } else {
        ((value - low) / span).clamp(0.0, 1.0)
    };
    let mix =
        |from: u8, to: u8| (f64::from(from) + (f64::from(to) - f64::from(from)) * t).round() as u8;
    Color32::from_rgba_unmultiplied(
        mix(from.r(), to.r()),
        mix(from.g(), to.g()),
        mix(from.b(), to.b()),
        from.a(),
    )
}

/// Blend oversold → line → overbought around the midpoint.
pub(crate) fn ramp(
    value: f64,
    low: f64,
    mid: f64,
    high: f64,
    low_ink: Color32,
    mid_ink: Color32,
    high_ink: Color32,
) -> Color32 {
    if value <= mid {
        blend(value, low, mid, low_ink, mid_ink)
    } else {
        blend(value, mid, high, mid_ink, high_ink)
    }
}

/// A wash that is strongest at `value` and invisible at `baseline`.
pub(crate) fn baseline_stop(
    value: f64,
    baseline: f64,
    ink: Color32,
    value_t: u8,
    base_t: u8,
) -> Option<GradientStop> {
    if !value.is_finite() || !baseline.is_finite() {
        return None;
    }
    let above = value > baseline;
    Some(GradientStop {
        top_value: value.max(baseline),
        bottom_value: value.min(baseline),
        top_color: transp(ink, if above { value_t } else { base_t }),
        bottom_color: transp(ink, if above { base_t } else { value_t }),
    })
}

fn previous(values: &IndicatorValues, line: usize, bar: usize) -> Option<f64> {
    if bar == 0 {
        return None;
    }
    values.value(line, bar - 1)
}

/// Rebuild line colours and wash stops from bar `from`.
pub(crate) fn fill_cache(
    study: &dyn Indicator,
    shading: &StudyShading,
    closes: &[f64],
    from: usize,
    colors: &mut Vec<Option<Color32>>,
    stops: &mut Vec<Option<GradientStop>>,
) {
    let values = study.values();
    let n = values.len();
    colors.truncate(from.min(colors.len()));
    stops.truncate(from.min(stops.len()));
    colors.resize(n, None);
    stops.resize(n, None);
    let line_ink = shading
        .colored_line()
        .map(|line| study.plot_style(line).color)
        .unwrap_or(shading.levels);
    for bar in from..n {
        let (color, stop) = bar_shade(shading, values, closes, bar, line_ink);
        colors[bar] = color;
        stops[bar] = stop;
    }
}

fn bar_shade(
    shading: &StudyShading,
    values: &IndicatorValues,
    closes: &[f64],
    bar: usize,
    line_ink: Color32,
) -> (Option<Color32>, Option<GradientStop>) {
    let read = |line| values.value(line, bar);
    match shading.kind {
        ShadingKind::Bounded {
            line,
            overbought,
            oversold,
            midpoint_wash,
            ..
        } => {
            let Some(value) = read(line) else {
                return (None, None);
            };
            let mid = (overbought + oversold) / 2.0;
            let color = ramp(
                value,
                oversold,
                mid,
                overbought,
                shading.bull,
                line_ink,
                shading.bear,
            );
            let stop = midpoint_wash
                .then(|| baseline_stop(value, mid, color, shading.value_t, 100))
                .flatten();
            (Some(color), stop)
        }
        ShadingKind::Centered { line, baseline, .. } => {
            let Some(value) = read(line) else {
                return (None, None);
            };
            let side = |color: Color32| {
                if color == Color32::PLACEHOLDER {
                    line_ink
                } else {
                    color
                }
            };
            let ink = if shading.bull == shading.bear && shading.bull == Color32::PLACEHOLDER {
                line_ink
            } else if value > baseline {
                side(shading.bull)
            } else if value < baseline {
                side(shading.bear)
            } else {
                line_ink
            };
            let stop = shading
                .gradient_fill
                .then(|| baseline_stop(value, baseline, ink, shading.value_t, 100))
                .flatten();
            (Some(ink), stop)
        }
        ShadingKind::Regime { line, signal } => {
            let (Some(value), Some(signal)) = (read(line), read(signal)) else {
                return (None, None);
            };
            let ink = if value > signal {
                shading.bull
            } else if value < signal {
                shading.bear
            } else {
                line_ink
            };
            let stop = shading
                .gradient_fill
                .then(|| baseline_stop(value, signal, ink, shading.value_t, 100))
                .flatten();
            (Some(ink), stop)
        }
        ShadingKind::Spread {
            line,
            signal,
            fill_t,
        } => {
            let (Some(value), Some(signal)) = (read(line), read(signal)) else {
                return (None, None);
            };
            let ink = if value > signal {
                shading.bull
            } else if value < signal {
                shading.bear
            } else {
                line_ink
            };
            let flat = transp(ink, fill_t);
            (
                Some(ink),
                Some(GradientStop {
                    top_value: value.max(signal),
                    bottom_value: value.min(signal),
                    top_color: flat,
                    bottom_color: flat,
                }),
            )
        }
        ShadingKind::Channel {
            basis,
            upper,
            lower,
            ..
        } => {
            let stop = match (read(upper), read(lower)) {
                (Some(upper), Some(lower)) if shading.gradient_fill => Some(GradientStop {
                    top_value: upper.max(lower),
                    bottom_value: upper.min(lower),
                    top_color: transp(shading.bear, 90),
                    bottom_color: transp(shading.bull, 90),
                }),
                _ => None,
            };
            let color = basis.and_then(|basis| {
                let basis = read(basis)?;
                let close = *closes.get(bar)?;
                if !close.is_finite() {
                    return None;
                }
                Some(if close > basis {
                    shading.bull
                } else if close < basis {
                    shading.bear
                } else {
                    palette::MUTED
                })
            });
            (color, stop)
        }
        ShadingKind::FlatChannel { upper, lower, .. } => {
            let stop = match (read(upper), read(lower)) {
                (Some(upper), Some(lower)) if shading.gradient_fill => Some(GradientStop {
                    top_value: upper.max(lower),
                    bottom_value: upper.min(lower),
                    top_color: shading.band,
                    bottom_color: shading.band,
                }),
                _ => None,
            };
            (None, stop)
        }
        ShadingKind::MomentumColumns { line, faded_t } => {
            let Some(value) = read(line) else {
                return (None, None);
            };
            let previous = previous(values, line, bar).unwrap_or(value);
            let rising = value > previous;
            let (grow, fade) = if faded_t == 0 {
                (shading.bull, shading.faded_bull)
            } else {
                (shading.bull, transp(shading.bull, faded_t))
            };
            let (grow_below, fade_below) = if faded_t == 0 {
                (shading.faded_bear, shading.bear)
            } else {
                (transp(shading.bear, faded_t), shading.bear)
            };
            let color = if value >= 0.0 {
                if rising { grow } else { fade }
            } else if rising {
                grow_below
            } else {
                fade_below
            };
            (Some(color), None)
        }
        ShadingKind::RisingColumns { line } => {
            let Some(value) = read(line) else {
                return (None, None);
            };
            let previous = previous(values, line, bar);
            let color = if previous.is_some_and(|previous| value > previous) {
                shading.bull
            } else {
                shading.bear
            };
            (Some(color), None)
        }
        ShadingKind::Intensity { line, range } => {
            let Some(value) = read(line) else {
                return (None, None);
            };
            let (low, high) = match range {
                IntensityRange::Threshold(threshold) => (0.0, threshold),
                IntensityRange::Rolling(window) => rolling_bounds(values, line, bar, window),
            };
            let color = blend(value, low, high, palette::MUTED, shading.accent);
            let stop = shading
                .gradient_fill
                .then(|| baseline_stop(value, 0.0, color, shading.value_t, 100))
                .flatten();
            (Some(color), stop)
        }
        ShadingKind::PriceSide { line } => {
            let Some(value) = read(line) else {
                return (None, None);
            };
            let close = closes.get(bar).copied().unwrap_or(f64::NAN);
            let color = if close >= value {
                shading.bull
            } else {
                shading.bear
            };
            (Some(color), None)
        }
        ShadingKind::AdxInk {
            adx,
            plus,
            minus,
            key_level,
        } => {
            let Some(value) = read(adx) else {
                return (None, None);
            };
            let color = if value <= key_level {
                palette::MUTED
            } else if read(plus).unwrap_or(0.0) > read(minus).unwrap_or(0.0) {
                shading.bull
            } else {
                shading.bear
            };
            (Some(color), None)
        }
        ShadingKind::Slope { line } => {
            let Some(value) = read(line) else {
                return (None, None);
            };
            let color = match previous(values, line, bar) {
                Some(previous) if value > previous => shading.bull,
                Some(previous) if value < previous => shading.bear,
                _ => line_ink,
            };
            (Some(color), None)
        }
        ShadingKind::BandOnly { .. } => (None, None),
    }
}

fn rolling_bounds(values: &IndicatorValues, line: usize, bar: usize, window: usize) -> (f64, f64) {
    let start = bar.saturating_sub(window.saturating_sub(1));
    let mut low = f64::INFINITY;
    let mut high = f64::NEG_INFINITY;
    for index in start..=bar {
        if let Some(value) = values.value(line, index) {
            low = low.min(value);
            high = high.max(value);
        }
    }
    if low.is_finite() {
        (low, high)
    } else {
        (0.0, 0.0)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn transparency_and_blend_follow_the_byte_lerp() {
        assert_eq!(transp(palette::UP, 90).a(), 26);
        assert_eq!(transp(palette::UP, 0).a(), 255);
        assert_eq!(transp(palette::UP, 100).a(), 0);
        assert_eq!(
            blend(5.0, 0.0, 10.0, Color32::BLACK, Color32::WHITE),
            Color32::from_rgb(128, 128, 128)
        );
        assert_eq!(
            blend(f64::NAN, 0.0, 1.0, palette::UP, palette::DOWN),
            palette::UP
        );
        assert_eq!(
            ramp(
                70.0,
                30.0,
                50.0,
                70.0,
                palette::UP,
                palette::MUTED,
                palette::DOWN
            ),
            palette::DOWN
        );
    }

    #[test]
    fn a_wash_is_strong_at_the_line_and_gone_at_the_baseline() {
        let above = baseline_stop(2.0, 0.0, palette::UP, 50, 100).unwrap();
        assert_eq!(above.top_value, 2.0);
        assert_eq!(above.bottom_value, 0.0);
        assert_eq!(above.top_color.a(), 128);
        assert_eq!(above.bottom_color.a(), 0);
        let below = baseline_stop(-2.0, 0.0, palette::DOWN, 50, 100).unwrap();
        assert_eq!(below.top_color.a(), 0);
        assert_eq!(below.bottom_color.a(), 128);
        assert!(baseline_stop(f64::NAN, 0.0, palette::UP, 50, 100).is_none());
    }
}

#[cfg(all(test, feature = "studies"))]
mod coverage {
    use crate::studies::Indicator;

    #[test]
    fn every_factory_study_is_listed() {
        let factory = crate::studies::IndicatorFactory::new();
        let mut problems = Vec::new();
        for name in factory.list() {
            let study = factory.create(&name).expect("constructor");
            let wrapped = crate::studies::ConfiguredStudy::new(study);
            let shaded = wrapped.shading().is_some();
            match expected(&name) {
                Some(want) if want != shaded => {
                    problems.push(format!("{name}: has shading {shaded}, expected {want}"))
                }
                None => problems.push(format!("{name}: not listed")),
                _ => {}
            }
        }
        assert!(problems.is_empty(), "{problems:#?}");
    }

    fn expected(name: &str) -> Option<bool> {
        Some(match name {
            "SMA"
            | "EMA"
            | "WMA"
            | "DEMA"
            | "TEMA"
            | "HMA"
            | "ALMA"
            | "VWMA"
            | "LSMA"
            | "SWMA"
            | "KAMA"
            | "Smoothed MA"
            | "McGinley Dynamic"
            | "ZLEMA"
            | "Hamming MA"
            | "VIDYA"
            | "Guppy MA"
            | "Multiple MA"
            | "EMA Cross"
            | "MA Cross"
            | "Mixed MA Cross"
            | "Triple MA"
            | "MA Ribbon"
            | "RVI"
            | "Rainbow Oscillator"
            | "ADR Bars"
            | "Bull Bear Power"
            | "Woodies CCI"
            | "Elder Ray"
            | "Aroon"
            | "RCI Ribbon"
            | "Knoxville Divergence"
            | "Technical Ratings"
            | "ADX Breakout"
            | "SuperTrend"
            | "Ichimoku Cloud"
            | "Parabolic SAR"
            | "Williams Alligator"
            | "Chop Zone"
            | "Volatility Stop"
            | "Chande Kroll Stop"
            | "Elder Impulse"
            | "Vortex"
            | "ATR"
            | "TR"
            | "Std Dev"
            | "ADR"
            | "Chandelier Exit"
            | "BB Trend"
            | "Median"
            | "Volume"
            | "Up/Down Volume"
            | "Volume 24h"
            | "Relative Volume at Time"
            | "Market Facilitation Index"
            | "Net Volume"
            | "OBV"
            | "Vol Profile Visible"
            | "Vol Profile Fixed"
            | "Vol Profile Session"
            | "Vol Profile Periodic"
            | "Vol Profile Anchored"
            | "Vol Profile Auto Anchored"
            | "TPO"
            | "Seasonality"
            | "Lunar Phases"
            | "Performance"
            | "Pivot Points"
            | "Pivot Points High/Low"
            | "Intraday Pivot Points"
            | "Projected Pivot Points"
            | "Missed Pivots"
            | "Key Levels"
            | "Ghost Pivots"
            | "Reversal"
            | "Elliott Wave"
            | "Rising Wedge"
            | "Falling Wedge"
            | "Williams Fractals"
            | "ZigZag"
            | "Higher Interval Candles"
            | "Bollinger Bars"
            | "Typical Price"
            | "Median Price"
            | "Weighted Close"
            | "OHLC4"
            | "Visible Average Price"
            | "52 Week High/Low"
            | "52 Week High/Low Levels"
            | "Trading Sessions"
            | "Gaps"
            | "Advance/Decline Ratio"
            | "Ratio"
            | "Spread"
            | "Majority Rule"
            | "Standard Error"
            | "Standard Error Bands"
            | "Linear Regression"
            | "Linear Regression Channel"
            | "Candlestick Patterns"
            | "Continuation Chart Patterns"
            | "Reversal Chart Patterns"
            | "Auto Trend Line"
            | "Auto Parallel Channel"
            | "Auto Fibonacci Retracement"
            | "Auto Fibonacci Extension"
            | "Auto Pitchfork"
            | "Quoted Spread"
            | "Book Size"
            | "Trade Rate"
            | "Aggressor Rate"
            | "Levels Executed"
            | "Multi-Range Delta"
            | "Rolling Volume and Delta"
            | "Size-Filtered CVD"
            | "Levels Traded Through" => false,
            "MA Channel"
            | "RSI"
            | "RSI Divergence Indicator"
            | "MACD"
            | "VW-MACD"
            | "Stochastic"
            | "StochRSI"
            | "KDJ"
            | "DS"
            | "SMI"
            | "Williams %R"
            | "CCI"
            | "Momentum"
            | "ROC"
            | "TRIX"
            | "TSI"
            | "KST"
            | "Coppock"
            | "DPO"
            | "PGO"
            | "CMO"
            | "STC"
            | "UO"
            | "Connors RSI"
            | "COG"
            | "UltMom"
            | "PPO"
            | "PVO"
            | "Volume Osc"
            | "BoP"
            | "EoM"
            | "AO"
            | "Accelerator Oscillator"
            | "Aroon Oscillator"
            | "PMO"
            | "Pring's Special K"
            | "SMI Ergodic Indicator"
            | "SMI Ergodic Oscillator"
            | "Fisher Transform"
            | "Mass Index"
            | "ADX"
            | "DMI"
            | "Squeeze"
            | "TII"
            | "TDI"
            | "Qstick"
            | "CHOP"
            | "BB"
            | "BBW"
            | "Keltner"
            | "Donchian"
            | "AccBands"
            | "Envelopes"
            | "HV"
            | "OHLC Volatility"
            | "Parkinson"
            | "Zero Trend Vol"
            | "Chaikin Volatility"
            | "%B"
            | "CSI"
            | "Ulcer Index"
            | "Relative Volatility Index"
            | "VWAP"
            | "AVWAP"
            | "VWAP Auto"
            | "TWAP"
            | "MFI"
            | "A/D"
            | "CMF"
            | "VROC"
            | "Klinger"
            | "Force Index"
            | "VPT"
            | "PVT Enhanced"
            | "WAD"
            | "NVI"
            | "PVI"
            | "Chaikin Oscillator"
            | "Advance/Decline Line"
            | "Cumulative Volume Index"
            | "ASI"
            | "Linear Regression Slope"
            | "Trend Strength"
            | "Correlation Coefficient"
            | "RCI"
            | "Zero-Trend Vol"
            | "CV"
            | "Trend Strength Index"
            | "RankCorr"
            | "Time Weighted Average Price"
            | "VWAP Auto Anchored"
            | "LinReg Slope"
            | "Price Momentum Oscillator"
            | "Parkinson Vol"
            | "GK Vol" => true,
            "BBTrend"
            | "MajRule"
            | "Time Price Opportunity"
            | "StdErrBands"
            | "Vol Profile FR"
            | "Average Daily Range"
            | "Ziv Ghost Pivots"
            | "TP"
            | "Ichimoku"
            | "Impulse"
            | "GMMA"
            | "Chandelier"
            | "Vol Profile VR"
            | "SMMA"
            | "StdErr"
            | "LinReg Channel"
            | "MP"
            | "PSAR"
            | "Pivot Points High Low"
            | "52W High/Low"
            | "Multi MA"
            | "Advance/Decline Ratio (Bars)"
            | "WC"
            | "52W HL%"
            | "SMA/EMA Cross"
            | "Pivot Points Standard"
            | "All Candlestick Patterns"
            | "Rainbow"
            | "LinReg"
            | "MD"
            | "24-hour Volume"
            | "Missed Pivot Levels"
            | "MFI BW"
            | "StdDev" => false,
            _ => return None,
        })
    }
}
