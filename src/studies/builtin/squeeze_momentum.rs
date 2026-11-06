use super::{clamp_param, over_finite_bars};
use crate::model::Bar;
use crate::studies::helpers;
use crate::studies::palette;
use crate::studies::stepper::{update_window, usable_behind};
use crate::studies::{Indicator, IndicatorValue, IndicatorValues};
use egui::Color32;

/// Squeeze Momentum Indicator
/// Combines Bollinger Bands and Keltner Channels to identify periods of low volatility
/// When BB is inside KC = "squeeze" (low volatility period)
#[derive(Clone)]
pub struct SqueezeMomentum {
    /// Bollinger Bands period (typically 20)
    bb_period: usize,
    /// Bollinger Bands standard deviation multiplier (typically 2.0)
    bb_mult: f64,
    /// Keltner Channel period (typically 20)
    kc_period: usize,
    /// Keltner Channel ATR multiplier (typically 1.5)
    kc_mult: f64,
    /// Momentum period (typically 20)
    momentum_period: usize,
    /// Mark the first bar the squeeze releases.
    mark_fires: bool,
    values: IndicatorValues,
    markers: Vec<crate::studies::IndicatorMarker>,
    /// Colors: [Momentum positive, Squeeze on (the squeeze line), Momentum negative, Squeeze off]
    colors: Vec<Color32>,
    visible: bool,
}

impl SqueezeMomentum {
    /// Create with the default Bollinger and Keltner channel settings.
    pub fn new() -> Self {
        Self {
            bb_period: 20,
            bb_mult: 2.0,
            kc_period: 20,
            kc_mult: 1.5,
            momentum_period: 20,
            mark_fires: true,
            values: IndicatorValues::new(),
            markers: Vec::new(),
            colors: vec![
                palette::GREEN,    // Green - Momentum positive
                palette::DISABLED, // Gray - Squeeze on
                palette::RED,      // Red - Momentum negative
                palette::BLUE,     // Blue - Squeeze off
            ],
            visible: true,
        }
    }

    /// Set the Bollinger Bands period and multiplier; `mult` is clamped into 0
    /// to `f64::MAX`, NaN to 0.
    pub fn with_bb_params(mut self, period: usize, mult: f64) -> Self {
        self.bb_period = period.max(1);
        self.bb_mult = clamp_param(mult, 0.0, f64::MAX);
        self
    }

    /// Set the Keltner Channel period and multiplier; `mult` is clamped into 0
    /// to `f64::MAX`, NaN to 0.
    pub fn with_kc_params(mut self, period: usize, mult: f64) -> Self {
        self.kc_period = period.max(1);
        self.kc_mult = clamp_param(mult, 0.0, f64::MAX);
        self
    }

    /// Set plot colors in the order returned by [`Indicator::line_names`].
    pub fn with_colors(mut self, colors: Vec<Color32>) -> Self {
        self.colors = colors;
        self
    }

    fn wilder_atr(data: &[Bar], period: usize) -> Vec<f64> {
        let mut average = super::smoothing::Smoothing::default();
        let mut previous = None;
        let alpha = 1.0 / period.max(1) as f64;
        data.iter()
            .map(|bar| {
                if ![bar.high, bar.low, bar.close]
                    .iter()
                    .all(|value| value.is_finite())
                {
                    average = super::smoothing::Smoothing::default();
                    previous = None;
                    return f64::NAN;
                }
                let range = match previous {
                    Some(close) => {
                        let close: f64 = close;
                        (bar.high - bar.low)
                            .max((bar.high - close).abs())
                            .max((bar.low - close).abs())
                    }
                    None => bar.high - bar.low,
                };
                previous = Some(bar.close);
                average.next(range, period, alpha)
            })
            .collect()
    }

    /// Full-window highest or lowest. Earlier bars and windows with a gap are NaN.
    fn window_extreme(
        values: impl IntoIterator<Item = f64>,
        period: usize,
        highest: bool,
    ) -> Vec<f64> {
        let values: Vec<f64> = values.into_iter().collect();
        let mut run = 0;
        values
            .iter()
            .enumerate()
            .map(|(index, value)| {
                run = if value.is_finite() { run + 1 } else { 0 };
                if run < period {
                    return f64::NAN;
                }
                let window = &values[index + 1 - period..=index];
                if highest {
                    window.iter().copied().fold(f64::NEG_INFINITY, f64::max)
                } else {
                    window.iter().copied().fold(f64::INFINITY, f64::min)
                }
            })
            .collect()
    }

    fn linear_regression(data: &[f64], period: usize) -> Vec<f64> {
        let mut result = Vec::with_capacity(data.len());

        for i in 0..data.len() {
            if i < period - 1 {
                result.push(f64::NAN);
            } else {
                let window = &data[i + 1 - period..=i];
                let n = period as f64;

                let sum_x: f64 = (0..period).map(|x| x as f64).sum();
                let sum_y: f64 = window.iter().sum();
                let sum_xy: f64 = window.iter().enumerate().map(|(x, &y)| x as f64 * y).sum();
                let sum_xx: f64 = (0..period).map(|x| (x * x) as f64).sum();

                let slope = (n * sum_xy - sum_x * sum_y) / (n * sum_xx - sum_x * sum_x);
                let intercept = (sum_y - slope * sum_x) / n;

                // Value at the end of the regression line
                let value = intercept + slope * (period - 1) as f64;
                result.push(value);
            }
        }

        result
    }
}

impl Default for SqueezeMomentum {
    fn default() -> Self {
        Self::new()
    }
}

impl SqueezeMomentum {
    /// The values over `data`, every bar of which has finite inputs.
    fn compute(&self, data: &[Bar]) -> IndicatorValues {
        let mut values = IndicatorValues::new();
        let required = self.bb_period.max(self.kc_period).max(self.momentum_period);
        if data.len() < required {
            for _ in 0..data.len() {
                values.push(IndicatorValue::None);
            }
            return values;
        }

        // Get closes
        let closes: Vec<f64> = data.iter().map(|b| b.close).collect();

        let bb_std = helpers::stdev(&closes, self.bb_period);
        let atr = Self::wilder_atr(data, self.kc_period);
        let basis = helpers::sma(&closes, self.momentum_period);
        let highest =
            Self::window_extreme(data.iter().map(|bar| bar.high), self.momentum_period, true);
        let lowest =
            Self::window_extreme(data.iter().map(|bar| bar.low), self.momentum_period, false);
        let mut deviation = Vec::with_capacity(data.len());
        for i in 0..data.len() {
            if highest[i].is_nan() || lowest[i].is_nan() || basis[i].is_nan() {
                deviation.push(f64::NAN);
            } else {
                let anchor = ((highest[i] + lowest[i]) / 2.0 + basis[i]) / 2.0;
                deviation.push(closes[i] - anchor);
            }
        }
        let momentum = Self::linear_regression(&deviation, self.momentum_period);

        // Store results: [momentum, squeeze_state (1.0 = squeeze on, 0.0 = off)]
        for i in 0..data.len() {
            if bb_std[i].is_nan() || atr[i].is_nan() || momentum[i].is_nan() {
                values.push(IndicatorValue::None);
            } else {
                let squeeze_on = self.bb_mult * bb_std[i] < self.kc_mult * atr[i];

                values.push_lines(&[momentum[i], if squeeze_on { 1.0 } else { 0.0 }]);
            }
        }

        values
    }
}

impl Indicator for SqueezeMomentum {
    fn shading(&self) -> Option<crate::studies::StudyShading> {
        Some(crate::studies::StudyShading::momentum(0).with_faded_t(60))
    }

    crate::studies::inputs::study_inputs!(self;
        bb_period: length("Bb period", 1, usize::MAX),
        bb_mult: number("Bb mult", 0.0, f64::MAX),
        kc_period: length("Kc period", 1, usize::MAX),
        kc_mult: number("Kc mult", 0.0, f64::MAX),
        momentum_period: length("Momentum period", 1, usize::MAX),
        mark_fires: toggle("Mark squeeze fires")
    ; {});

    fn value_format(&self) -> crate::scales::StudyFormat {
        crate::scales::StudyFormat::Price
    }

    fn name(&self) -> &str {
        "Squeeze"
    }

    fn desc(&self) -> &str {
        "Squeeze Momentum - Identifies volatility breakouts"
    }

    fn calculate(&mut self, data: &[Bar]) {
        self.update(data, 0);
    }

    fn update(&mut self, data: &[Bar], from: usize) {
        let mut values = std::mem::take(&mut self.values);
        update_window(
            &mut values,
            data,
            from,
            usable_behind(
                (self.bb_period - 1)
                    .max(self.kc_period)
                    .max((self.momentum_period - 1).saturating_mul(2)),
                |b| b.high.is_finite() && b.low.is_finite() && b.close.is_finite(),
            ),
            |data| {
                over_finite_bars(
                    data,
                    |b| b.high.is_finite() && b.low.is_finite() && b.close.is_finite(),
                    |data| self.compute(data),
                )
            },
        );
        self.values = values;
        self.markers.clear();
        if !self.mark_fires {
            return;
        }
        let mut previous = None;
        for (bar, value) in self.values.to_rows().into_iter().enumerate() {
            let IndicatorValue::Multiple(values) = value else {
                previous = None;
                continue;
            };
            let on = values[1] == 1.0;
            if previous == Some(true) && !on {
                self.markers.push(crate::studies::IndicatorMarker {
                    bar,
                    value: 0.0,
                    direction: if values[0] >= 0.0 {
                        crate::studies::CrossDirection::Up
                    } else {
                        crate::studies::CrossDirection::Down
                    },
                });
            }
            previous = Some(on);
        }
    }

    fn markers(&self) -> &[crate::studies::IndicatorMarker] {
        &self.markers
    }

    fn is_incremental(&self) -> bool {
        true
    }

    fn values(&self) -> &IndicatorValues {
        &self.values
    }

    fn colors(&self) -> Vec<Color32> {
        self.colors.clone()
    }

    fn set_colors(&mut self, colors: Vec<Color32>) {
        if colors.len() >= 4 {
            self.colors = colors;
        }
    }

    fn is_overlay(&self) -> bool {
        false
    }

    fn is_visible(&self) -> bool {
        self.visible
    }

    fn set_visible(&mut self, visible: bool) {
        self.visible = visible;
    }

    fn clone_box(&self) -> Box<dyn Indicator> {
        Box::new(self.clone())
    }

    fn line_names(&self) -> Vec<String> {
        vec!["Momentum".to_string(), "Squeeze".to_string()]
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::studies::test_support::make_range_bar as make_bar;

    #[test]
    fn test_squeeze_calculation() {
        let mut squeeze = SqueezeMomentum::new();

        let data: Vec<Bar> = (0..50)
            .map(|i| make_bar(102.0 + i as f64, 98.0 + i as f64, 100.0 + i as f64))
            .collect();

        squeeze.calculate(&data);

        assert_eq!(squeeze.values.len(), 50);

        // Should have momentum and squeeze state
        let rows = squeeze.values.to_rows();
        let IndicatorValue::Multiple(v) = rows.last().unwrap() else {
            panic!("no value: {:?}", squeeze.values.to_rows().last().unwrap());
        };
        assert_eq!(v.len(), 2);
    }
}
