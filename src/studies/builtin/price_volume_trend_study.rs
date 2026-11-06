use crate::model::Bar;
use crate::studies::palette;
use crate::studies::stepper::Stepper;
use crate::studies::{Indicator, IndicatorValue, IndicatorValues};
use egui::Color32;

#[derive(Clone, Copy, PartialEq, Eq)]
enum SignalAverage {
    Sma,
    Ema,
}

impl std::fmt::Debug for SignalAverage {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter.write_str(match self {
            Self::Sma => "SMA",
            Self::Ema => "EMA",
        })
    }
}

/// Price Volume Trend (Enhanced) with Signal Line
/// PVT = prev_PVT + volume * (close - prev_close) / prev_close
/// Signal = SMA or EMA of PVT. The signal defaults to a 21-bar SMA.
#[derive(Clone)]
pub struct PriceVolumeTrendStudy {
    signal_period: usize,
    signal_type: SignalAverage,
    values: IndicatorValues,
    stepper: Stepper<(
        Option<f64>,
        f64,
        super::smoothing::Smoothing,
        std::collections::VecDeque<f64>,
    )>,
    colors: Vec<Color32>,
    visible: bool,
}

impl PriceVolumeTrendStudy {
    /// Create with `signal_period` in bars, each clamped to at least one.
    pub fn new(signal_period: usize) -> Self {
        Self {
            signal_period: signal_period.max(1),
            signal_type: SignalAverage::Sma,
            values: IndicatorValues::new(),
            stepper: Stepper::default(),
            colors: vec![
                palette::BLUE,   // PVT line
                palette::ORANGE, // Signal line
            ],
            visible: true,
        }
    }
}

impl Default for PriceVolumeTrendStudy {
    fn default() -> Self {
        Self::new(21)
    }
}

impl Indicator for PriceVolumeTrendStudy {
    fn shading(&self) -> Option<crate::studies::StudyShading> {
        Some(crate::studies::StudyShading::slope(0))
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
        signal_type: choice("Signal type", [SignalAverage::Sma, SignalAverage::Ema]),
        signal_period: length("Signal length", 1, usize::MAX)
    ; {});

    fn value_format(&self) -> crate::scales::StudyFormat {
        crate::scales::StudyFormat::Volume
    }

    fn name(&self) -> &str {
        "PVT Enhanced"
    }

    fn desc(&self) -> &str {
        "Price Volume Trend Enhanced - PVT with EMA signal line"
    }

    fn calculate(&mut self, data: &[Bar]) {
        self.update(data, 0);
    }

    fn update(&mut self, data: &[Bar], from: usize) {
        let (kind, period) = (self.signal_type, self.signal_period);
        self.stepper
            .run(data, from, &mut self.values, |state, bar| {
                if !bar.close.is_finite() || !bar.volume.is_finite() {
                    return IndicatorValue::None;
                }
                let Some(previous) = state.0 else {
                    state.0 = Some(bar.close);
                    return IndicatorValue::None;
                };
                if previous != 0.0 {
                    state.1 += bar.volume * (bar.close - previous) / previous;
                }
                state.0 = Some(bar.close);
                let signal = match kind {
                    SignalAverage::Ema => {
                        state.2.next(state.1, period, 2.0 / (period as f64 + 1.0))
                    }
                    SignalAverage::Sma => {
                        state.3.push_back(state.1);
                        while state.3.len() > period {
                            state.3.pop_front();
                        }
                        if state.3.len() < period {
                            f64::NAN
                        } else {
                            state.3.iter().sum::<f64>() / period as f64
                        }
                    }
                };
                IndicatorValue::Multiple(vec![state.1, signal])
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
        } else if !colors.is_empty() {
            self.colors = vec![colors[0], colors[0]];
        }
    }

    fn is_overlay(&self) -> bool {
        false
    }

    fn line_count(&self) -> usize {
        2
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
        vec![format!("PVT({})", self.signal_period), "Signal".to_string()]
    }
}
