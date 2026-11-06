use super::macd::MACD;
use super::smoothing::Smoothing;
use crate::model::Bar;
use crate::studies::palette;
use crate::studies::stepper::Stepper;
use crate::studies::{Indicator, IndicatorValue, IndicatorValues};
use egui::Color32;

/// Signal state for Elder Impulse
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum ImpulseSignal {
    /// EMA rising, MACD histogram rising
    Bullish,
    /// EMA falling, MACD histogram falling
    Bearish,
    /// Mixed signals or an unchanged EMA or MACD histogram
    Neutral,
}

/// Elder Impulse System
/// Combines EMA slope and MACD histogram for trend signals
/// Green = bullish, Red = bearish, Blue = neutral
#[derive(Clone)]
pub struct ElderImpulseSystem {
    /// EMA period (typically 13)
    ema_period: usize,
    /// MACD fast period
    macd_fast: usize,
    /// MACD slow period
    macd_slow: usize,
    /// MACD signal period
    macd_signal: usize,
    values: IndicatorValues,
    macd: MACD,
    /// Set once the MACD histogram has a value. Until then every update rewrites it from the
    /// first bar, so the impulse has to be recomputed with it.
    macd_ready: bool,
    stepper: Stepper<(usize, Smoothing, Option<f64>, Option<f64>)>,
    /// Colors: [Bullish, Bearish, Neutral]
    colors: Vec<Color32>,
    visible: bool,
}

impl ElderImpulseSystem {
    /// Create with `ema_period`, `macd_fast`, `macd_slow`, `macd_signal` in bars, each clamped
    /// to at least one.
    pub fn new(ema_period: usize, macd_fast: usize, macd_slow: usize, macd_signal: usize) -> Self {
        Self {
            ema_period: ema_period.max(1),
            macd_fast: macd_fast.max(1),
            macd_slow: macd_slow.max(1),
            macd_signal: macd_signal.max(1),
            values: IndicatorValues::new(),
            macd: MACD::new(macd_fast.max(1), macd_slow.max(1), macd_signal.max(1)),
            macd_ready: false,
            stepper: Stepper::default(),
            colors: vec![
                palette::GREEN, // Green - Bullish
                palette::RED,   // Red - Bearish
                palette::BLUE,  // Blue - Neutral
            ],
            visible: true,
        }
    }

    /// Set plot colors in the order returned by [`Indicator::line_names`].
    pub fn with_colors(mut self, colors: Vec<Color32>) -> Self {
        self.colors = colors;
        self
    }
}

impl Default for ElderImpulseSystem {
    fn default() -> Self {
        Self::new(13, 12, 26, 9)
    }
}

impl Indicator for ElderImpulseSystem {
    fn source(&self) -> Option<crate::studies::StudySource> {
        Some(crate::studies::StudySource::Price(
            crate::model::PriceSource::Close,
        ))
    }

    crate::studies::inputs::study_inputs!(self;
        ema_period: length("EMA period", 1, usize::MAX),
        macd_fast: length("MACD fast", 1, usize::MAX),
        macd_slow: length("MACD slow", 1, usize::MAX),
        macd_signal: length("MACD signal", 1, usize::MAX)
    ; {});

    fn name(&self) -> &str {
        "Impulse"
    }

    fn desc(&self) -> &str {
        "Elder Impulse System - Trend direction with momentum"
    }

    fn calculate(&mut self, data: &[Bar]) {
        self.update(data, 0);
    }

    fn update(&mut self, data: &[Bar], from: usize) {
        // A change from the first bar can carry new periods. A later change resumes the
        // averages already built with the current periods.
        if from == 0 {
            self.macd = MACD::new(self.macd_fast, self.macd_slow, self.macd_signal);
            self.macd_ready = false;
        }
        self.macd.update(data, from);
        // Losing the last finite bar can blank the histogram again, which changes every earlier bar.
        let ready =
            (0..self.macd.values().len()).any(|bar| self.macd.values().value(2, bar).is_some());
        let step_from = if self.macd_ready && ready { from } else { 0 };
        self.macd_ready = ready;
        let histogram: Vec<Option<f64>> = (0..data.len())
            .map(|bar| self.macd.values().value(2, bar))
            .collect();
        let alpha = 2.0 / (self.ema_period as f64 + 1.0);
        self.stepper
            .run(data, step_from, &mut self.values, |state, bar| {
                let index = state.0;
                state.0 += 1;
                if !bar.close.is_finite() {
                    return IndicatorValue::None;
                }
                let ema = state.1.next(bar.close, 1, alpha);
                if ema.is_nan() {
                    return IndicatorValue::None;
                }
                let histogram = histogram.get(index).copied().flatten();
                let (previous_ema, previous_histogram) = (state.2, state.3);
                state.2 = Some(ema);
                state.3 = histogram;
                match (previous_ema, previous_histogram, histogram) {
                    (Some(previous_ema), Some(previous_histogram), Some(histogram)) => {
                        IndicatorValue::Single(
                            if ema > previous_ema && histogram > previous_histogram {
                                1.0
                            } else if ema < previous_ema && histogram < previous_histogram {
                                -1.0
                            } else {
                                0.0
                            },
                        )
                    }
                    _ => IndicatorValue::None,
                }
            });
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
        if colors.len() >= 3 {
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
        vec!["Impulse".to_string()]
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::studies::test_support::make_bar;

    #[test]
    fn test_elder_impulse() {
        for direction in [0.0, 1.0, -1.0] {
            let mut impulse = ElderImpulseSystem::new(5, 6, 12, 5);
            // A constant has neither slope; accelerating rises/falls give both slopes that sign.
            let data: Vec<Bar> = (0..40)
                .map(|i| make_bar(100.0 + direction * (i * i) as f64))
                .collect();
            impulse.calculate(&data);
            assert_eq!(impulse.values.len(), data.len());
            for bar in 0..data.len() {
                assert_eq!(
                    impulse.values().value(0, bar),
                    (bar >= 1).then_some(direction),
                    "direction {direction}, bar {bar}"
                );
            }
        }
    }

    /// Feeding the bars one at a time, then revising the last bar, matches one batch calculation.
    /// A short prefix stays blank: the histogram appears only after its slow length, so an early
    /// bar of a short series is not the same bar of the finished series.
    #[test]
    fn incremental_matches_a_full_compute() {
        let data: Vec<Bar> = (0..40)
            .map(|i| make_bar(100.0 + (i as f64 * 0.7).sin() * 4.0 + i as f64 * 0.15))
            .collect();
        let mut batch = ElderImpulseSystem::new(5, 6, 12, 5);
        batch.calculate(&data);
        let mut stepped = ElderImpulseSystem::new(5, 6, 12, 5);
        for end in 1..=data.len() {
            stepped.update(&data[..end], end - 1);
            let mut prefix = ElderImpulseSystem::new(5, 6, 12, 5);
            prefix.calculate(&data[..end]);
            for bar in 0..end {
                assert_eq!(
                    stepped.values().value(0, bar),
                    prefix.values().value(0, bar),
                    "prefix {end} bar {bar}"
                );
            }
        }
        for bar in 0..data.len() {
            assert_eq!(
                stepped.values().value(0, bar),
                batch.values().value(0, bar),
                "bar {bar}"
            );
        }
        let mut revised = data.clone();
        revised[39].close += 3.0;
        batch.calculate(&revised);
        stepped.update(&revised, 39);
        for bar in 0..revised.len() {
            assert_eq!(
                stepped.values().value(0, bar),
                batch.values().value(0, bar),
                "revised bar {bar}"
            );
        }
    }
}
