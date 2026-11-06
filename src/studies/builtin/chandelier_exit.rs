use super::clamp_param;
use crate::model::Bar;
use crate::studies::palette;
use crate::studies::stepper::Stepper;
use crate::studies::{ATR, Indicator, IndicatorValue, IndicatorValues};
use egui::Color32;
use std::collections::VecDeque;

#[derive(Clone, Default)]
struct ExitState {
    index: usize,
    window: VecDeque<[f64; 2]>,
    prev_long: Option<f64>,
    prev_short: Option<f64>,
    prev_close: Option<f64>,
}

/// Chandelier Exit
/// Trailing stop-loss indicator based on Wilder's ATR
/// Developed by Chuck LeBeau
#[derive(Clone)]
pub struct ChandelierExit {
    /// ATR period (typically 22)
    period: usize,
    /// ATR multiplier (typically 3.0)
    multiplier: f64,
    /// Keep a long stop from falling and a short stop from rising.
    ratchet: bool,
    values: IndicatorValues,
    atr: ATR,
    stepper: Stepper<ExitState>,
    /// Colors: [Long exit, Short exit]
    colors: Vec<Color32>,
    visible: bool,
}

impl ChandelierExit {
    /// `multiplier` is clamped into 0 to `f64::MAX`, NaN to 0.
    pub fn new(period: usize, multiplier: f64) -> Self {
        Self {
            period: period.max(1),
            multiplier: clamp_param(multiplier, 0.0, f64::MAX),
            ratchet: false,
            values: IndicatorValues::new(),
            atr: ATR::new(period),
            stepper: Stepper::default(),
            colors: vec![
                palette::BLUE,   // Long exit
                palette::ORANGE, // Short exit
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

impl Default for ChandelierExit {
    fn default() -> Self {
        Self::new(22, 3.0)
    }
}

impl Indicator for ChandelierExit {
    crate::studies::inputs::study_inputs!(self;
        period: length("Period", 1, usize::MAX),
        multiplier: number("Multiplier", 0.0, f64::MAX),
        ratchet: toggle("Ratchet stops")
    ; {});

    fn name(&self) -> &str {
        "Chandelier"
    }

    fn desc(&self) -> &str {
        "Chandelier Exit - ATR-based trailing stop"
    }

    fn calculate(&mut self, data: &[Bar]) {
        self.update(data, 0);
    }

    fn update(&mut self, data: &[Bar], from: usize) {
        if from == 0 {
            self.atr = ATR::new(self.period);
        }
        self.atr.update(data, from);
        self.stepper
            .run(data, from, &mut self.values, |state, bar| {
                let index = state.index;
                state.index += 1;
                if ![bar.high, bar.low, bar.close].iter().all(|x| x.is_finite()) {
                    return IndicatorValue::None;
                }
                state.window.push_back([bar.high, bar.low]);
                if state.window.len() > self.period {
                    state.window.pop_front();
                }
                let Some(atr) = self.atr.values().value(0, index) else {
                    state.prev_close = Some(bar.close);
                    return IndicatorValue::None;
                };
                let high = state
                    .window
                    .iter()
                    .map(|v| v[0])
                    .fold(f64::NEG_INFINITY, f64::max);
                let low = state
                    .window
                    .iter()
                    .map(|v| v[1])
                    .fold(f64::INFINITY, f64::min);
                let basic_long = high - atr * self.multiplier;
                let basic_short = low + atr * self.multiplier;
                let (long, short) = if self.ratchet {
                    let prev_close = state.prev_close.unwrap_or(bar.close);
                    let prev_long = state.prev_long.unwrap_or(basic_long);
                    let prev_short = state.prev_short.unwrap_or(basic_short);
                    (
                        if prev_close > prev_long {
                            basic_long.max(prev_long)
                        } else {
                            basic_long
                        },
                        if prev_close < prev_short {
                            basic_short.min(prev_short)
                        } else {
                            basic_short
                        },
                    )
                } else {
                    (basic_long, basic_short)
                };
                state.prev_long = Some(long);
                state.prev_short = Some(short);
                state.prev_close = Some(bar.close);
                IndicatorValue::Multiple(vec![long, short])
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
        if colors.len() >= 2 {
            self.colors = colors;
        }
    }

    fn is_overlay(&self) -> bool {
        true // Overlay on price chart
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

    fn line_count(&self) -> usize {
        2
    }

    fn line_names(&self) -> Vec<String> {
        vec![
            format!("Long Exit({},{})", self.period, self.multiplier),
            format!("Short Exit({},{})", self.period, self.multiplier),
        ]
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::studies::test_support::make_range_bar as make_bar;

    /// Reference values from LeBeau's definition over Wilder's ATR, computed
    /// independently: highest high or lowest low of the last `period` bars,
    /// minus or plus `multiplier` times ATR(`period`). The gap on bar 4 makes
    /// Wilder's smoothing differ from a simple average of true range.
    #[test]
    fn matches_reference_values() {
        let trend = [
            (102.0, 98.0, 100.0),
            (104.0, 99.0, 103.0),
            (106.0, 101.0, 105.0),
            (108.0, 103.0, 107.0),
            (118.0, 112.0, 116.0),
            (117.0, 113.0, 114.0),
            (115.0, 109.0, 110.0),
            (112.0, 108.0, 111.0),
        ];
        let trend_expected = [
            (96.66666666666667, 107.33333333333333),
            (98.44444444444444, 108.55555555555556),
            (104.29629629629629, 114.70370370370371),
            (106.19753086419753, 114.80246913580247),
            (106.13168724279835, 120.86831275720165),
            (106.42112482853224, 118.57887517146776),
        ];
        let flat = [(101.0, 99.0, 100.0); 5];
        let flat_expected = [(97.0, 103.0); 3];
        let cases = [
            ("trend with gap", &trend[..], &trend_expected[..]),
            ("flat", &flat[..], &flat_expected[..]),
        ];

        let mut failures = Vec::new();
        for (label, bars, expected) in cases {
            let data: Vec<Bar> = bars.iter().map(|&(h, l, c)| make_bar(h, l, c)).collect();
            let mut ce = ChandelierExit::new(3, 2.0);
            ce.calculate(&data);

            let got = ce.values().to_rows();
            let warm_up = got
                .iter()
                .take(2)
                .all(|v| matches!(v, IndicatorValue::None));
            if got.len() != data.len() || !warm_up {
                failures.push(format!("{label}: warm-up or length wrong: {got:?}"));
                continue;
            }
            for (i, &(long, short)) in expected.iter().enumerate() {
                let ok = matches!(&got[i + 2], IndicatorValue::Multiple(v)
                    if (v[0] - long).abs() < 1e-9 && (v[1] - short).abs() < 1e-9);
                if !ok {
                    failures.push(format!(
                        "{label}: bar {} is {:?}, expected [{long}, {short}]",
                        i + 2,
                        got[i + 2]
                    ));
                }
            }
        }
        assert!(failures.is_empty(), "{}", failures.join("\n"));
    }

    /// Period 1 makes ATR equal the true range, so the ratchet steps are exact.
    #[test]
    fn ratchet_holds_the_stop_until_price_crosses_it() {
        let data = vec![
            make_bar(10.0, 8.0, 9.0),
            make_bar(11.0, 9.0, 11.0),
            make_bar(8.0, 6.0, 7.0),
        ];
        let mut exit = ChandelierExit::new(1, 1.0);
        exit.ratchet = true;
        exit.calculate(&data);
        let rows = exit.values().to_rows();
        let pair = |value: &IndicatorValue| match value {
            IndicatorValue::Multiple(values) => (values[0], values[1]),
            other => panic!("{other:?}"),
        };
        let (long, short) = pair(&rows[0]);
        assert!((long - 8.0).abs() < 1e-9 && (short - 10.0).abs() < 1e-9);
        let (long, short) = pair(&rows[1]);
        assert!((long - 9.0).abs() < 1e-9 && (short - 10.0).abs() < 1e-9);
        let (long, short) = pair(&rows[2]);
        assert!((long - 9.0).abs() < 1e-9 && (short - 11.0).abs() < 1e-9);
    }
}
