use super::smoothing::Smoothing;
use crate::model::Bar;
use crate::studies::palette;
use crate::studies::stepper::Stepper;
use crate::studies::{Indicator, IndicatorValue, IndicatorValues};
use egui::Color32;
use std::collections::VecDeque;

/// Pretty Good Oscillator (PGO)
/// Measures price distance from moving avg in ATR units
/// Values > 3 or < -3 indicate potential reversal
#[derive(Clone)]
pub struct PrettyGoodOscillator {
    /// Period for calculations (typically 14)
    period: usize,
    values: IndicatorValues,
    stepper: Stepper<(Option<f64>, Smoothing, VecDeque<f64>)>,
    color: Color32,
    visible: bool,
}

impl PrettyGoodOscillator {
    /// Create with a lookback of `period` bars, clamped to at least 1.
    pub fn new(period: usize) -> Self {
        Self {
            period: period.max(1),
            values: IndicatorValues::new(),
            stepper: Stepper::default(),
            color: palette::PURPLE,
            visible: true,
        }
    }

    /// Set the plot color.
    pub fn with_color(mut self, color: Color32) -> Self {
        self.color = color;
        self
    }
}

impl Default for PrettyGoodOscillator {
    fn default() -> Self {
        Self::new(14)
    }
}

impl Indicator for PrettyGoodOscillator {
    fn shading(&self) -> Option<crate::studies::StudyShading> {
        Some(crate::studies::StudyShading::centered(0, 0.0))
    }

    crate::studies::inputs::study_inputs!(self;
        period: length("Period", 1, usize::MAX)
    ; {});

    fn name(&self) -> &str {
        "PGO"
    }

    fn desc(&self) -> &str {
        "Pretty Good Oscillator - Price distance in ATR units"
    }

    fn calculate(&mut self, data: &[Bar]) {
        self.update(data, 0);
    }

    fn update(&mut self, data: &[Bar], from: usize) {
        // `from == 0` replaces the series. Resetting here also covers a one-bar series, which
        // the stepper would otherwise resume.
        if from == 0 {
            self.stepper = Stepper::default();
        }
        let period = self.period;
        let alpha = 2.0 / (period as f64 + 1.0);
        self.stepper
            .run(data, from, &mut self.values, |state, bar| {
                if !(bar.high.is_finite() && bar.low.is_finite() && bar.close.is_finite()) {
                    return IndicatorValue::None;
                }
                let range = match state.0 {
                    Some(previous) => {
                        let high_low = bar.high - bar.low;
                        let high_close = (bar.high - previous).abs();
                        let low_close = (bar.low - previous).abs();
                        high_low.max(high_close).max(low_close)
                    }
                    None => bar.high - bar.low,
                };
                state.0 = Some(bar.close);
                let average = state.1.next(range, period, alpha);
                state.2.push_back(bar.close);
                if state.2.len() > period {
                    state.2.pop_front();
                }
                if state.2.len() < period || !average.is_finite() || average == 0.0 {
                    return IndicatorValue::None;
                }
                let mean = state.2.iter().sum::<f64>() / period as f64;
                IndicatorValue::Single((bar.close - mean) / average)
            });
    }

    fn is_incremental(&self) -> bool {
        true
    }

    fn values(&self) -> &IndicatorValues {
        &self.values
    }

    fn colors(&self) -> Vec<Color32> {
        vec![self.color]
    }

    fn set_colors(&mut self, colors: Vec<Color32>) {
        if !colors.is_empty() {
            self.color = colors[0];
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
        vec![format!("PGO({})", self.period)]
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::studies::test_support::make_range_bar as make_bar;

    #[test]
    fn test_pgo_calculation() {
        let mut pgo = PrettyGoodOscillator::new(5);

        let data = vec![
            make_bar(102.0, 98.0, 100.0),
            make_bar(103.0, 99.0, 101.0),
            make_bar(104.0, 100.0, 102.0),
            make_bar(105.0, 101.0, 103.0),
            make_bar(106.0, 102.0, 104.0),
            make_bar(110.0, 105.0, 109.0), // Big move up
        ];

        pgo.calculate(&data);

        assert_eq!(pgo.values.len(), 6);

        // Last value should be positive (price above avg)
        let rows = pgo.values.to_rows();
        let IndicatorValue::Single(v) = rows.last().unwrap() else {
            panic!("no value: {:?}", pgo.values.to_rows().last().unwrap());
        };
        assert!(*v > 0.0, "PGO should be positive after upward move");
        // First five true ranges are 4, so the seed is 4. Bar 4: (104 - 102) / 4.
        assert_eq!(pgo.values().value(0, 4), Some(0.5));
        // Bar 5 true range is 6. alpha = 2/6, so the average is 4 + (6 - 4) * alpha.
        // The close mean is (101 + 102 + 103 + 104 + 109) / 5.
        let alpha = 2.0 / 6.0;
        let average = 4.0 + (6.0 - 4.0) * alpha;
        let mean = (101.0 + 102.0 + 103.0 + 104.0 + 109.0) / 5.0;
        assert_eq!(pgo.values().value(0, 5), Some((109.0 - mean) / average));
    }

    #[test]
    fn incremental_matches_a_full_compute() {
        let mut data = vec![
            make_bar(102.0, 98.0, 100.0),
            make_bar(103.0, 99.0, 101.0),
            make_bar(f64::NAN, 1.0, 1.0),
            make_bar(104.0, 100.0, 102.0),
            make_bar(105.0, 101.0, 103.0),
            make_bar(106.0, 102.0, 104.0),
            make_bar(110.0, 105.0, 109.0),
        ];
        super::super::assert_replay_matches_batch(&PrettyGoodOscillator::new(5), &data);
        data[6].close = 111.0;
        let mut batch = PrettyGoodOscillator::new(5);
        batch.calculate(&data);
        let mut stepped = PrettyGoodOscillator::new(5);
        stepped.calculate(&data[..6]);
        stepped.update(&data, 6);
        for bar in 0..data.len() {
            assert_eq!(
                stepped.values().value(0, bar),
                batch.values().value(0, bar),
                "bar {bar}"
            );
        }
    }
}
