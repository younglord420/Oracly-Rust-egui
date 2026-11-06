use super::smoothing::Smoothing;
use crate::model::Bar;
use crate::studies::palette;
use crate::studies::stepper::Stepper;
use crate::studies::{Indicator, IndicatorValue, IndicatorValues};
use egui::Color32;
use std::cell::RefCell;
use std::collections::VecDeque;

/// What rescales VIDYA's smoothing constant.
#[derive(Clone, Copy, PartialEq, Eq)]
enum VidyaScaler {
    Momentum,
    Volatility,
}

impl std::fmt::Debug for VidyaScaler {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter.write_str(match self {
            Self::Momentum => "Momentum (|CMO|)",
            Self::Volatility => "Volatility (StDev ratio)",
        })
    }
}

#[derive(Clone, Default)]
struct VidyaState {
    index: usize,
    prev: Option<f64>,
    closes: VecDeque<f64>,
}

/// VIDYA - Variable Index Dynamic Avg
/// Adaptive moving avg that adjusts based on volatility
/// Uses Chande Momentum Oscillator for volatility measurement
#[derive(Clone)]
pub struct VIDYA {
    /// Period for calculation
    period: usize,
    scaler: VidyaScaler,
    scaler_len: usize,
    ref_len: usize,
    values: IndicatorValues,
    /// Adaptive scaler at each source bar. Outside the stepper so it is not cloned per bar.
    scales: RefCell<Vec<f64>>,
    stepper: Stepper<VidyaState>,
    color: Color32,
    adaptive: bool,
    visible: bool,
}

impl VIDYA {
    /// Create with a lookback of `period` bars, clamped to at least 1.
    pub fn new(period: usize) -> Self {
        Self {
            period: period.max(1),
            scaler: VidyaScaler::Momentum,
            scaler_len: 9,
            ref_len: 30,
            values: IndicatorValues::new(),
            scales: RefCell::new(Vec::new()),
            stepper: Stepper::default(),
            color: palette::BLUE,
            adaptive: true,
            visible: true,
        }
    }

    /// Set the plot color.
    pub fn with_color(mut self, color: Color32) -> Self {
        self.color = color;
        self
    }
}

impl Default for VIDYA {
    fn default() -> Self {
        Self::new(12)
    }
}

impl Indicator for VIDYA {
    fn source(&self) -> Option<crate::studies::StudySource> {
        Some(crate::studies::StudySource::Price(
            crate::model::PriceSource::Close,
        ))
    }

    crate::studies::inputs::study_inputs!(self;
        period: length("Period", 1, usize::MAX),
        scaler: choice("Adaptive scaler", [VidyaScaler::Momentum, VidyaScaler::Volatility]),
        scaler_len: length("Scaler lookback", 1, usize::MAX),
        ref_len: length("Reference stdev length", 2, usize::MAX),
        adaptive: toggle("Adaptive gradient")
    ; {});

    fn name(&self) -> &str {
        "VIDYA"
    }

    fn desc(&self) -> &str {
        "Variable Index Dynamic Avg - Volatility-adjusted MA"
    }

    fn calculate(&mut self, data: &[Bar]) {
        self.update(data, 0);
    }

    fn update(&mut self, data: &[Bar], from: usize) {
        let scales = &self.scales;
        let (period, scaler, scaler_len, ref_len) = (
            self.period,
            self.scaler,
            self.scaler_len.max(1),
            self.ref_len.max(2),
        );
        self.stepper
            .run(data, from, &mut self.values, |state, bar| {
                let index = state.index;
                state.index += 1;
                let mut scales = scales.borrow_mut();
                if index >= scales.len() {
                    scales.resize(index + 1, f64::NAN);
                }
                if !bar.close.is_finite() {
                    state.closes.clear();
                    scales[index] = f64::NAN;
                    return IndicatorValue::None;
                }
                state.closes.push_back(bar.close);
                let keep = scaler_len.max(ref_len) + 1;
                while state.closes.len() > keep {
                    state.closes.pop_front();
                }
                let k = vidya_scale(&state.closes, scaler, scaler_len, ref_len);
                scales[index] = k;
                let next = match state.prev {
                    Some(previous) if k.is_finite() => {
                        let alpha = (2.0 / (period as f64 + 1.0) * k).min(1.0);
                        alpha * bar.close + (1.0 - alpha) * previous
                    }
                    _ => bar.close,
                };
                state.prev = Some(next);
                IndicatorValue::Single(next)
            });
        self.scales.borrow_mut().truncate(data.len());
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

    fn plot_color(&self, _line: usize, bar: usize) -> Option<Color32> {
        if !self.adaptive {
            return None;
        }
        let scale = self
            .scales
            .borrow()
            .get(bar)
            .copied()
            .filter(|scale| scale.is_finite())
            .unwrap_or(0.0);
        Some(crate::studies::shading::blend(
            scale.min(1.0),
            0.0,
            1.0,
            palette::MUTED,
            self.color,
        ))
    }

    fn line_names(&self) -> Vec<String> {
        vec![format!("VIDYA({})", self.period)]
    }
}

fn population_stdev(window: &[f64]) -> f64 {
    let n = window.len() as f64;
    let mean = window.iter().sum::<f64>() / n;
    (window
        .iter()
        .map(|value| (value - mean).powi(2))
        .sum::<f64>()
        / n)
        .sqrt()
}

/// `|CMO| / 100` or the short/reference standard-deviation ratio, NaN until the window exists.
fn vidya_scale(
    closes: &VecDeque<f64>,
    scaler: VidyaScaler,
    scaler_len: usize,
    ref_len: usize,
) -> f64 {
    match scaler {
        VidyaScaler::Momentum => {
            if closes.len() <= scaler_len {
                return f64::NAN;
            }
            let start = closes.len() - scaler_len - 1;
            let mut up = 0.0;
            let mut down = 0.0;
            for pair in closes.range(start..).collect::<Vec<_>>().windows(2) {
                let change = pair[1] - pair[0];
                if change > 0.0 {
                    up += change;
                } else {
                    down -= change;
                }
            }
            if up + down == 0.0 {
                0.0
            } else {
                (up - down).abs() / (up + down)
            }
        }
        VidyaScaler::Volatility => {
            if closes.len() < ref_len || closes.len() < scaler_len {
                return f64::NAN;
            }
            let short = population_stdev(
                &closes
                    .iter()
                    .rev()
                    .take(scaler_len)
                    .copied()
                    .collect::<Vec<_>>(),
            );
            let reference = population_stdev(
                &closes
                    .iter()
                    .rev()
                    .take(ref_len)
                    .copied()
                    .collect::<Vec<_>>(),
            );
            if reference > 0.0 {
                short / reference
            } else {
                0.0
            }
        }
    }
}

/// Zero Lag EMA
/// Reduces lag in EMA by using error correction
#[derive(Clone)]
pub struct ZLEMA {
    period: usize,
    values: IndicatorValues,
    stepper: Stepper<(std::collections::VecDeque<f64>, Smoothing)>,
    color: Color32,
    visible: bool,
}

impl ZLEMA {
    /// Create with a lookback of `period` bars, clamped to at least 1.
    pub fn new(period: usize) -> Self {
        Self {
            period: period.max(1),
            values: IndicatorValues::new(),
            stepper: Stepper::default(),
            color: palette::BLUE,
            visible: true,
        }
    }

    /// Set the plot color.
    pub fn with_color(mut self, color: Color32) -> Self {
        self.color = color;
        self
    }
}

impl Default for ZLEMA {
    fn default() -> Self {
        Self::new(21)
    }
}

impl Indicator for ZLEMA {
    fn source(&self) -> Option<crate::studies::StudySource> {
        Some(crate::studies::StudySource::Price(
            crate::model::PriceSource::Close,
        ))
    }

    crate::studies::inputs::study_inputs!(self;
        period: length("Period", 1, usize::MAX)
    ; {});

    fn name(&self) -> &str {
        "ZLEMA"
    }

    fn desc(&self) -> &str {
        "Zero Lag EMA - Reduced lag moving avg"
    }

    fn calculate(&mut self, data: &[Bar]) {
        self.update(data, 0);
    }

    fn update(&mut self, data: &[Bar], from: usize) {
        self.stepper
            .run(data, from, &mut self.values, |state, bar| {
                if !bar.close.is_finite() {
                    state.0.clear();
                    state.1 = Smoothing::default();
                    return IndicatorValue::None;
                }
                let lag = ((self.period - 1) as f64 / 2.0).round() as usize;
                state.0.push_back(bar.close);
                while state.0.len() > lag + 1 {
                    state.0.pop_front();
                }
                if state.0.len() <= lag {
                    return IndicatorValue::None;
                }
                let delagged = 2.0 * bar.close - state.0.front().copied().unwrap_or(bar.close);
                let ema = state
                    .1
                    .next(delagged, self.period, 2.0 / (self.period as f64 + 1.0));
                if ema.is_nan() {
                    IndicatorValue::None
                } else {
                    IndicatorValue::Single(ema)
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
        vec![format!("ZLEMA({})", self.period)]
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::studies::test_support::make_bar;

    fn singles(study: &impl Indicator) -> Vec<Option<f64>> {
        study
            .values()
            .to_rows()
            .into_iter()
            .map(|value| match value {
                IndicatorValue::Single(value) => Some(value),
                IndicatorValue::None => None,
                other => panic!("{other:?}"),
            })
            .collect()
    }

    #[test]
    fn zlema_lag_rounds_half_away_from_zero_and_seeds_the_ema() {
        assert_eq!(((10 - 1) as f64 / 2.0).round() as usize, 5);
        assert_eq!(((21 - 1) as f64 / 2.0).round() as usize, 10);
        let data: Vec<_> = (1..=12).map(|close| make_bar(close as f64)).collect();
        let mut zlema = ZLEMA::new(4);
        zlema.calculate(&data);
        let got = singles(&zlema);
        let expected = [
            None,
            None,
            None,
            None,
            None,
            Some(6.5),
            Some(7.5),
            Some(8.5),
            Some(9.5),
            Some(10.5),
            Some(11.5),
            Some(12.5),
        ];
        assert_eq!(got.len(), expected.len());
        for (index, (got, want)) in got.iter().zip(expected).enumerate() {
            match (got, want) {
                (None, None) => {}
                (Some(got), Some(want)) => {
                    assert!((got - want).abs() < 1e-9, "{index}: {got} vs {want}")
                }
                _ => panic!("{index}: {got:?} vs {want:?}"),
            }
        }
    }

    #[test]
    fn vidya_matches_both_scalers() {
        let closes = [
            10.0, 11.0, 12.0, 13.0, 12.0, 11.0, 10.0, 11.0, 12.0, 13.0, 14.0, 13.0, 12.0, 11.0,
            10.0,
        ];
        let data: Vec<_> = closes.iter().copied().map(make_bar).collect();
        let momentum = [
            10.0,
            11.0,
            12.0,
            13.0,
            12.833333333333,
            12.833333333333,
            12.361111111111,
            12.134259259259,
            12.134259259259,
            12.278549382716,
            12.852366255144,
            12.876971879287,
            12.876971879287,
            12.564143232739,
            11.709428821826,
        ];
        let volatility = [
            10.0,
            11.0,
            12.0,
            13.0,
            12.0,
            11.753817018041,
            11.071144669642,
            11.053630062719,
            11.286610235915,
            11.953546297054,
            12.521130432349,
            12.605129711251,
            12.456157074463,
            11.889348701962,
            11.365337654104,
        ];
        for (scaler, expected) in [
            (VidyaScaler::Momentum, momentum),
            (VidyaScaler::Volatility, volatility),
        ] {
            let mut vidya = VIDYA::new(5);
            vidya.scaler = scaler;
            vidya.scaler_len = 4;
            vidya.ref_len = 6;
            vidya.calculate(&data);
            let got = singles(&vidya);
            for (index, (got, want)) in got.iter().zip(expected).enumerate() {
                let Some(got) = got else {
                    panic!("{scaler:?} {index} missing")
                };
                assert!(
                    (got - want).abs() < 1e-9,
                    "{scaler:?} {index}: {got} vs {want}"
                );
            }
        }
    }
}
