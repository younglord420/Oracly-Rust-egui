//! Kaufman Adaptive Moving Average (KAMA) indicator.
//!
//! KAMA is an adaptive moving average that adjusts its smoothing speed based
//! on the market's efficiency ratio. In trending markets it becomes more
//! responsive; in choppy, ranging markets it slows down to filter noise.
//!
//! # Formula
//!
//! ```text
//! Efficiency Ratio (ER) = |Close - Close[N]| / Sum(|Close[i] - Close[i-1]|, N)
//! Smoothing Constant (SC) = [ER * (fast_sc - slow_sc) + slow_sc]^2
//! KAMA[t] = KAMA[t-1] + SC * (Close[t] - KAMA[t-1])
//! ```
//!
//! # Default parameters
//!
//! `KAMA::new(10)` with fast period 2 and slow period 30.
//!
//! # Example
//!
//! ```rust
//! # let bars: Vec<egui_charts::model::Bar> = (0..60).map(|i| { let p = 100.0 + (i as f64 * 0.3).sin() * 5.0; egui_charts::model::Bar::new(chrono::Utc::now(), p, p + 1.0, p - 1.0, p + 0.5, 1_000.0) }).collect();
//! use egui_charts::studies::{KAMA, Indicator};
//!
//! let mut kama = KAMA::new(10);
//! kama.calculate(&bars);
//! ```

use crate::model::Bar;
use crate::studies::palette;
use crate::studies::stepper::Stepper;
use crate::studies::{Indicator, IndicatorValue, IndicatorValues};
use egui::Color32;
use std::collections::VecDeque;

/// Kaufman Adaptive Moving Average indicator.
///
/// Adapts its smoothing speed to market conditions via the efficiency
/// ratio. Overlay indicator drawn on the price chart.
#[derive(Clone)]
pub struct KAMA {
    /// Period for Efficiency Ratio calculation.
    period: usize,
    /// Fastest EMA period (typically 2).
    fast_period: usize,
    /// Slowest EMA period (typically 30).
    slow_period: usize,
    values: IndicatorValues,
    stepper: Stepper<State>,
    color: Color32,
    visible: bool,
}

#[derive(Clone, Default)]
struct State {
    /// Consecutive finite closes ending at the current bar.
    closes: VecDeque<f64>,
    prev: Option<f64>,
}

impl KAMA {
    /// Create a new KAMA indicator.
    ///
    /// # Arguments
    /// * `period` -- Efficiency ratio lookback period (default: 10).
    ///
    /// Fast/slow periods default to 2 and 30 respectively. Use
    /// [`with_periods`](Self::with_periods) to customise.
    pub fn new(period: usize) -> Self {
        Self {
            period: period.max(1),
            fast_period: 2,
            slow_period: 30,
            values: IndicatorValues::new(),
            stepper: Stepper::default(),
            color: palette::BLUE,
            visible: true,
        }
    }

    /// Set custom fast and slow EMA periods.
    pub fn with_periods(mut self, fast: usize, slow: usize) -> Self {
        self.fast_period = fast.max(1);
        self.slow_period = slow.max(1);
        self
    }

    /// Set a custom line colour (builder pattern).
    pub fn with_color(mut self, color: Color32) -> Self {
        self.color = color;
        self
    }
}

/// Construct with the conventional default parameters.
impl Default for KAMA {
    fn default() -> Self {
        Self::new(10)
    }
}

impl Indicator for KAMA {
    fn source(&self) -> Option<crate::studies::StudySource> {
        Some(crate::studies::StudySource::Price(
            crate::model::PriceSource::Close,
        ))
    }

    crate::studies::inputs::study_inputs!(self;
        period: length("Period", 1, usize::MAX),
        fast_period: length("Fast period", 1, usize::MAX),
        slow_period: length("Slow period", 1, usize::MAX)
    ; {});

    fn name(&self) -> &str {
        "KAMA"
    }

    fn desc(&self) -> &str {
        "Kaufman Adaptive Moving Avg - Adjusts smoothing based on market efficiency"
    }

    fn calculate(&mut self, data: &[Bar]) {
        self.update(data, 0);
    }

    fn update(&mut self, data: &[Bar], from: usize) {
        let fast = 2.0 / (self.fast_period as f64 + 1.0);
        let slow = 2.0 / (self.slow_period as f64 + 1.0);
        self.stepper
            .run(data, from, &mut self.values, |state, bar| {
                let close = bar.close;
                if !close.is_finite() {
                    state.closes.clear();
                    return IndicatorValue::None;
                }
                state.closes.push_back(close);
                while state.closes.len() > self.period.saturating_add(1) {
                    state.closes.pop_front();
                }
                let ready = state.closes.len() > self.period;
                let next = if ready {
                    let oldest = *state.closes.front().unwrap();
                    let direction = (close - oldest).abs();
                    let volatility = state
                        .closes
                        .iter()
                        .zip(state.closes.iter().skip(1))
                        .fold(0.0, |sum, (previous, close)| sum + (close - previous).abs());
                    let er = if volatility == 0.0 {
                        0.0
                    } else {
                        direction / volatility
                    };
                    let sc = (er * (fast - slow) + slow).powi(2);
                    match state.prev {
                        Some(previous) => previous + sc * (close - previous),
                        None => close,
                    }
                } else {
                    state.prev.unwrap_or(close)
                };
                state.prev = Some(next);
                IndicatorValue::Single(next)
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
        true
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
        vec![format!(
            "KAMA({}, {}, {})",
            self.period, self.fast_period, self.slow_period
        )]
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::studies::test_support::make_bar;

    /// Expected values from the efficiency-ratio recurrence, carrying the
    /// first close until a full window exists. Fast period 2, slow period 30.
    #[test]
    fn calculate_matches_kaufman_reference() {
        // (name, period, closes, expected KAMA per bar)
        type Case = (&'static str, usize, &'static [f64], &'static [Option<f64>]);
        let cases: &[Case] = &[
            (
                "choppy",
                4,
                &[10.0, 11.0, 10.5, 12.0, 11.0, 13.0, 12.5, 14.0, 13.0, 13.5],
                &[
                    Some(10.0),
                    Some(10.0),
                    Some(10.0),
                    Some(10.0),
                    Some(10.046248121170077),
                    Some(10.321699400553712),
                    Some(10.524836194574917),
                    Some(10.8489115927601),
                    Some(11.04951072164707),
                    Some(11.10504267893198),
                ],
            ),
            (
                "trend",
                3,
                &[1.0, 2.0, 3.0, 4.0, 5.0, 6.0],
                &[
                    Some(1.0),
                    Some(1.0),
                    Some(1.0),
                    Some(2.333333333333333),
                    Some(3.518518518518518),
                    Some(4.621399176954732),
                ],
            ),
            (
                "flat",
                3,
                &[7.0; 5],
                &[Some(7.0), Some(7.0), Some(7.0), Some(7.0), Some(7.0)],
            ),
            (
                "short",
                5,
                &[1.0, 2.0, 3.0],
                &[Some(1.0), Some(1.0), Some(1.0)],
            ),
            // The NaN close has no value; the next efficiency ratio spans the closes around it.
            (
                "NaN close",
                2,
                &[1.0, 2.0, f64::NAN, 4.0, 5.0, 6.0, 7.0],
                &[
                    Some(1.0),
                    Some(1.0),
                    None,
                    Some(1.0),
                    Some(1.0),
                    Some(3.2222222222222214),
                    Some(4.901234567901233),
                ],
            ),
        ];

        for (name, period, closes, expected) in cases {
            let data: Vec<Bar> = closes.iter().map(|&c| make_bar(c)).collect();
            let mut kama = KAMA::new(*period);
            for len in 0..=data.len() {
                kama.update(&data[..len], len.saturating_sub(1));
            }
            assert_eq!(kama.values.len(), expected.len(), "{name}");
            for (i, (got, want)) in kama
                .values
                .to_rows()
                .iter()
                .zip(expected.iter())
                .enumerate()
            {
                match (got, want) {
                    (IndicatorValue::None, None) => {}
                    (IndicatorValue::Single(g), Some(w)) => {
                        assert!((g - w).abs() < 1e-12, "{name}[{i}]: got {g}, want {w}")
                    }
                    _ => panic!("{name}[{i}]: got {got:?}, want {want:?}"),
                }
            }
        }
    }
}
