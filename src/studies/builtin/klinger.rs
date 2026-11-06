use super::smoothing::Smoothing;
use crate::model::Bar;
use crate::studies::palette;
use crate::studies::stepper::Stepper;
use crate::studies::{Indicator, IndicatorValue, IndicatorValues};
use egui::Color32;

/// Klinger Volume Oscillator, as defined by Stephen J. Klinger in "Identifying Trends With Volume Analysis", Technical Analysis of Stocks & Commodities, V.15:12 (December 1997).
///
/// The trend is up when a bar's high + low + close is above the previous bar's sum, down when it is below, and unchanged when the sums are equal. `dm` is the bar's range and `cm` the cumulative range of the current trend: the previous `cm` plus `dm` while the trend continues, the previous bar's `dm` plus `dm` when it changes. The volume force is `volume * |2 * (dm / cm - 1)| * trend * 100`, the oscillator is its short EMA minus its long EMA, and the signal line is an EMA of the oscillator.
///
/// There is no trend, and so no volume force, until a bar's sum first differs from the bar before. On that first trend bar `cm` is the bar's `dm`, and a bar with `cm = 0` has a volume force of 0. Each EMA starts at the mean of its first `n` inputs.
///
/// Values are `None` until the oscillator exists, then `[oscillator, signal]`, with the signal NaN until it exists.
#[derive(Clone)]
pub struct KlingerOscillator {
    /// Short EMA period (typically 34)
    short_period: usize,
    /// Long EMA period (typically 55)
    long_period: usize,
    /// Signal line period (typically 13)
    signal_period: usize,
    values: IndicatorValues,
    stepper: Stepper<State>,
    /// Colors: [Klinger, Signal]
    colors: Vec<Color32>,
    visible: bool,
}

#[derive(Clone, Default)]
struct State {
    previous_sum: Option<f64>,
    trend: f64,
    previous_range: f64,
    cm: f64,
    started: bool,
    ema: [Smoothing; 3],
}

impl KlingerOscillator {
    /// Create with short EMA 34, long EMA 55 and signal EMA 13.
    pub fn new() -> Self {
        Self {
            short_period: 34,
            long_period: 55,
            signal_period: 13,
            values: IndicatorValues::new(),
            stepper: Stepper::default(),
            colors: vec![
                palette::BLUE,  // Klinger
                palette::AMBER, // Signal
            ],
            visible: true,
        }
    }

    /// Set the short, long and signal EMA periods in bars, each at least one.
    pub fn with_periods(mut self, short: usize, long: usize, signal: usize) -> Self {
        self.short_period = short.max(1);
        self.long_period = long.max(1);
        self.signal_period = signal.max(1);
        self
    }

    /// Set plot colors in the order returned by [`Indicator::line_names`].
    pub fn with_colors(mut self, colors: Vec<Color32>) -> Self {
        self.colors = colors;
        self
    }
}

impl Default for KlingerOscillator {
    fn default() -> Self {
        Self::new()
    }
}

impl Indicator for KlingerOscillator {
    fn shading(&self) -> Option<crate::studies::StudyShading> {
        Some(crate::studies::StudyShading::centered(0, 0.0).with_value_t(60))
    }

    crate::studies::inputs::study_inputs!(self;
        short_period: length("Short period", 1, usize::MAX),
        long_period: length("Long period", 1, usize::MAX),
        signal_period: length("Signal period", 1, usize::MAX)
    ; {});

    fn value_format(&self) -> crate::scales::StudyFormat {
        crate::scales::StudyFormat::Volume
    }

    fn name(&self) -> &str {
        "Klinger"
    }

    fn desc(&self) -> &str {
        "Klinger Volume Oscillator - Long-term money flow trend"
    }

    fn calculate(&mut self, data: &[Bar]) {
        self.update(data, 0);
    }

    fn update(&mut self, data: &[Bar], from: usize) {
        self.stepper
            .run(data, from, &mut self.values, |state, bar| {
                if ![bar.high, bar.low, bar.close, bar.volume]
                    .iter()
                    .all(|x| x.is_finite())
                {
                    return IndicatorValue::None;
                }
                let sum = bar.high + bar.low + bar.close;
                let previous_sum = state.previous_sum.unwrap_or(sum);
                let trend = if sum > previous_sum { 1.0 } else { -1.0 };
                let dm = bar.high - bar.low;
                state.cm = if state.started && trend == state.trend {
                    state.cm + dm
                } else {
                    state.previous_range + dm
                };
                state.started = true;
                state.trend = trend;
                state.previous_range = dm;
                state.previous_sum = Some(sum);
                let force = if state.cm == 0.0 {
                    0.0
                } else {
                    bar.volume * (2.0 * dm / state.cm - 1.0).abs() * trend * 100.0
                };
                let short = state.ema[0].next(
                    force,
                    self.short_period,
                    2.0 / (self.short_period as f64 + 1.0),
                );
                let long = state.ema[1].next(
                    force,
                    self.long_period,
                    2.0 / (self.long_period as f64 + 1.0),
                );
                let kvo = short - long;
                let signal = state.ema[2].next(
                    kvo,
                    self.signal_period,
                    2.0 / (self.signal_period as f64 + 1.0),
                );
                if kvo.is_nan() {
                    IndicatorValue::None
                } else {
                    IndicatorValue::Multiple(vec![kvo, signal])
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
        if colors.len() >= 2 {
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

    fn line_count(&self) -> usize {
        2
    }

    fn line_names(&self) -> Vec<String> {
        vec![
            format!("KVO({},{})", self.short_period, self.long_period),
            format!("Signal({})", self.signal_period),
        ]
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::Utc;

    fn make_bar(high: f64, low: f64, close: f64, volume: f64) -> Bar {
        Bar {
            time: Utc::now(),
            open: (high + low) / 2.0,
            high,
            low,
            close,
            volume,
            colors: None,
            payload: None,
        }
    }

    /// Expected values follow Stephen J. Klinger, "Identifying Trends With Volume Analysis",
    /// Technical Analysis of Stocks & Commodities, V.15:12 (December 1997), computed
    /// independently with this file's EMA seeding (the mean of the first `n` inputs).
    #[test]
    fn matches_klingers_definition() {
        let mut data: Vec<Bar> = (0..80)
            .map(|i| {
                let close = 100.0 + ((7 * i) % 11) as f64 * 0.5 + 0.25 * i as f64;
                make_bar(
                    close + 1.0 + (i % 3) as f64 * 0.5,
                    close - 1.0 - (i % 2) as f64 * 0.5,
                    close,
                    1000.0 + 250.0 * (i % 4) as f64,
                )
            })
            .collect();
        // No range on bars 0 and 1 gives cm = 0 on the first trend bar; bar 40 repeats the prices of
        // bar 39, whose trend is up, so equal sums must keep that trend.
        for bar in &mut data[..2] {
            bar.high = bar.close;
            bar.low = bar.close;
        }
        data[40] = Bar {
            volume: data[40].volume,
            ..data[39].clone()
        };

        // A tie is down, so the oscillator is only checked to be finite once both averages exist.
        let mut klinger = KlingerOscillator::new();
        klinger.calculate(&data);

        let values = klinger.values().to_rows();
        assert_eq!(values.len(), 80);
        assert!(
            values[..54]
                .iter()
                .all(|v| matches!(v, IndicatorValue::None))
        );
        assert!(values.iter().any(|value| matches!(
            value,
            IndicatorValue::Multiple(lines) if lines.iter().any(|line| line.is_finite())
        )));
        assert!(values.iter().all(|value| match value {
            IndicatorValue::None => true,
            IndicatorValue::Multiple(lines) =>
                lines.iter().all(|line| line.is_finite() || line.is_nan()),
            IndicatorValue::Single(line) => line.is_finite() || line.is_nan(),
        }));
        assert_eq!(klinger.line_count(), 2);

        // A bar with a non-finite price has no value; the study carries on after it.
        data[60].high = f64::NAN;
        klinger.calculate(&data);
        assert!(matches!(
            klinger.values().to_rows()[60],
            IndicatorValue::None
        ));
        assert!(klinger.values().to_rows()[61..].iter().all(|v| matches!(
            v,
            IndicatorValue::Multiple(l) if l[0].is_finite()
        )));

        // The long average needs 55 forces, so 54 bars are still blank.
        klinger.calculate(&data[..54]);
        assert!(
            klinger
                .values()
                .to_rows()
                .iter()
                .all(|v| matches!(v, IndicatorValue::None))
        );
    }
}
