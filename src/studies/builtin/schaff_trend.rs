use super::smoothing::Smoothing;
use crate::model::Bar;
use crate::studies::palette;
use crate::studies::stepper::Stepper;
use crate::studies::{Indicator, IndicatorValue, IndicatorValues};
use egui::Color32;

/// Schaff Trend Cycle (STC)
/// Combines MACD with stochastic to create a faster, more accurate oscillator
/// Oscillates between 0 and 100
#[derive(Clone)]
pub struct SchaffTrendCycle {
    /// MACD short period (typically 23)
    macd_short: usize,
    /// MACD long period (typically 50)
    macd_long: usize,
    /// Stochastic cycle period (typically 10)
    cycle: usize,
    factor: f64,
    values: IndicatorValues,
    stepper: Stepper<State>,
    color: Color32,
    visible: bool,
}

#[derive(Clone)]
struct State {
    fast: Smoothing,
    slow: Smoothing,
    macd: std::collections::VecDeque<f64>,
    first: std::collections::VecDeque<f64>,
    /// Last stochastic reading of each pass. NaN until a window has a range.
    percent: [f64; 2],
    /// Recursive smoother of each pass. NaN until its first finite input.
    smoothed: [f64; 2],
}

impl Default for State {
    fn default() -> Self {
        Self {
            fast: Smoothing::default(),
            slow: Smoothing::default(),
            macd: std::collections::VecDeque::new(),
            first: std::collections::VecDeque::new(),
            percent: [f64::NAN; 2],
            smoothed: [f64::NAN; 2],
        }
    }
}

impl SchaffTrendCycle {
    /// Create with MACD periods 23 and 50 and cycle length 10.
    pub fn new() -> Self {
        Self {
            macd_short: 23,
            macd_long: 50,
            cycle: 10,
            factor: 0.5,
            values: IndicatorValues::new(),
            stepper: Stepper::default(),
            color: palette::PURPLE,
            visible: true,
        }
    }

    /// Set the short and long MACD periods and cycle length in bars, each at least one.
    pub fn with_periods(mut self, short: usize, long: usize, cycle: usize) -> Self {
        self.macd_short = short.max(1);
        self.macd_long = long.max(1);
        self.cycle = cycle.max(1);
        self
    }

    /// Set the plot color.
    pub fn with_color(mut self, color: Color32) -> Self {
        self.color = color;
        self
    }

    fn push_window(window: &mut std::collections::VecDeque<f64>, value: f64, period: usize) {
        if !value.is_finite() {
            window.clear();
            return;
        }
        window.push_back(value);
        while window.len() > period {
            window.pop_front();
        }
    }

    /// Position in the window. A flat window keeps the previous reading.
    fn stochastic(
        window: &std::collections::VecDeque<f64>,
        period: usize,
        previous: f64,
    ) -> Option<f64> {
        if window.len() < period {
            return None;
        }
        let value = *window.back().unwrap();
        let high = window.iter().copied().fold(f64::NEG_INFINITY, f64::max);
        let low = window.iter().copied().fold(f64::INFINITY, f64::min);
        let percent = if high > low {
            100.0 * (value - low) / (high - low)
        } else {
            previous
        };
        percent.is_finite().then_some(percent)
    }

    fn smooth(state: &mut f64, value: f64, factor: f64) -> f64 {
        *state = if state.is_finite() {
            *state + factor * (value - *state)
        } else {
            value
        };
        *state
    }
}

impl Default for SchaffTrendCycle {
    fn default() -> Self {
        Self::new()
    }
}

impl Indicator for SchaffTrendCycle {
    fn shading(&self) -> Option<crate::studies::StudyShading> {
        Some(
            crate::studies::StudyShading::centered(0, 50.0)
                .with_bear(palette::DOWN)
                .with_levels(vec![75.0, 25.0]),
        )
    }

    fn line_style(&self, line: usize) -> crate::studies::IndicatorLineStyle {
        let mut style = crate::studies::IndicatorLineStyle::default();
        if line == 0 {
            style.width = 2.0;
        }
        style
    }

    fn source(&self) -> Option<crate::studies::StudySource> {
        Some(crate::studies::StudySource::Price(
            crate::model::PriceSource::Close,
        ))
    }

    crate::studies::inputs::study_inputs!(self;
        macd_short: length("MACD short", 1, usize::MAX),
        macd_long: length("MACD long", 1, usize::MAX),
        cycle: length("Cycle", 2, usize::MAX),
        factor: number("Smoothing factor", 0.01, 1.0)
    ; {});

    fn name(&self) -> &str {
        "STC"
    }

    fn desc(&self) -> &str {
        "Schaff Trend Cycle - Fast trend oscillator"
    }

    fn calculate(&mut self, data: &[Bar]) {
        self.update(data, 0);
    }

    fn update(&mut self, data: &[Bar], from: usize) {
        let (short_len, long_len, cycle, factor) = (
            self.macd_short,
            self.macd_long,
            self.cycle.max(2),
            self.factor,
        );
        self.stepper
            .run(data, from, &mut self.values, |state, bar| {
                if !bar.close.is_finite() {
                    state.fast = Smoothing::default();
                    state.slow = Smoothing::default();
                    state.macd.clear();
                    state.first.clear();
                    return IndicatorValue::None;
                }
                let fast = state
                    .fast
                    .next(bar.close, short_len, 2.0 / (short_len as f64 + 1.0));
                let slow = state
                    .slow
                    .next(bar.close, long_len, 2.0 / (long_len as f64 + 1.0));
                let macd = if fast.is_finite() && slow.is_finite() {
                    fast - slow
                } else {
                    f64::NAN
                };
                Self::push_window(&mut state.macd, macd, cycle);
                let Some(first) = Self::stochastic(&state.macd, cycle, state.percent[0]) else {
                    return IndicatorValue::None;
                };
                state.percent[0] = first;
                let smooth = Self::smooth(&mut state.smoothed[0], first, factor);
                Self::push_window(&mut state.first, smooth, cycle);
                let Some(second) = Self::stochastic(&state.first, cycle, state.percent[1]) else {
                    return IndicatorValue::None;
                };
                state.percent[1] = second;
                IndicatorValue::Single(Self::smooth(&mut state.smoothed[1], second, factor))
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
        vec![format!(
            "STC({},{},{})",
            self.macd_short, self.macd_long, self.cycle
        )]
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::studies::test_support::make_bar;

    #[test]
    fn test_stc_calculation() {
        let mut stc = SchaffTrendCycle::new().with_periods(5, 10, 3);

        // A straight line makes the underlying difference constant, so the cycle has no range.
        let data: Vec<Bar> = (0..50)
            .map(|i| make_bar(100.0 + i as f64 + if i % 2 == 0 { 3.0 } else { 0.0 }))
            .collect();

        stc.calculate(&data);

        assert_eq!(stc.values.len(), 50);

        // Check that valid values are between 0 and 100
        let mut checked = 0;
        for value in &stc.values.to_rows() {
            if let IndicatorValue::Single(v) = value {
                assert!(*v >= 0.0 && *v <= 100.0, "STC should be 0-100, got {v}");
                checked += 1;
            }
        }
        assert!(checked > 0, "no value to check");
    }

    #[test]
    fn test_stc_uptrend() {
        let mut stc = SchaffTrendCycle::new().with_periods(3, 6, 3);

        let data: Vec<Bar> = (0..40)
            .map(|i| make_bar(100.0 + i as f64 * 2.0 + if i % 2 == 0 { 4.0 } else { 0.0 }))
            .collect();

        stc.calculate(&data);

        // In uptrend, STC should be high (close to 100)
        let high = stc
            .values
            .to_rows()
            .iter()
            .any(|value| matches!(value, IndicatorValue::Single(value) if *value > 50.0));
        assert!(high, "an uptrend should lift the cycle above 50");
    }
}
