use super::smoothing::Smoothing;
use crate::model::Bar;
use crate::studies::palette;
use crate::studies::stepper::Stepper;
use crate::studies::{Indicator, IndicatorValue, IndicatorValues};
use egui::Color32;

/// True Strength Index (TSI), William Blau, "Momentum, Direction, and Divergence" (1995).
///
/// `TSI = 100 * EMA(EMA(m, long), short) / EMA(EMA(|m|, long), short)` with `m = close - previous close`,
/// plus a signal line, the EMA of the TSI. Every EMA is seeded with the mean of its first full window.
#[derive(Clone)]
pub struct TrueStrengthIndex {
    /// Long period (typically 25)
    long_period: usize,
    /// Short period (typically 13)
    short_period: usize,
    /// Signal line period (typically 13)
    signal_period: usize,
    values: IndicatorValues,
    stepper: Stepper<(usize, Option<f64>, [Smoothing; 5])>,
    /// Colors: [TSI, Signal]
    colors: Vec<Color32>,
    visible: bool,
}

impl TrueStrengthIndex {
    /// Create with `long_period`, `short_period`, `signal_period` in bars, each clamped to at
    /// least one.
    pub fn new(long_period: usize, short_period: usize, signal_period: usize) -> Self {
        Self {
            long_period: long_period.max(1),
            short_period: short_period.max(1),
            signal_period: signal_period.max(1),
            values: IndicatorValues::new(),
            stepper: Stepper::default(),
            colors: vec![
                palette::BLUE, // TSI
                palette::PINK, // Signal
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

impl Default for TrueStrengthIndex {
    fn default() -> Self {
        Self::new(25, 13, 13)
    }
}

impl Indicator for TrueStrengthIndex {
    fn shading(&self) -> Option<crate::studies::StudyShading> {
        Some(crate::studies::StudyShading::centered(0, 0.0))
    }

    fn source(&self) -> Option<crate::studies::StudySource> {
        Some(crate::studies::StudySource::Price(
            crate::model::PriceSource::Close,
        ))
    }

    crate::studies::inputs::study_inputs!(self;
        long_period: length("Long period", 1, usize::MAX),
        short_period: length("Short period", 1, usize::MAX),
        signal_period: length("Signal period", 1, usize::MAX)
    ; {});

    fn name(&self) -> &str {
        "TSI"
    }

    fn desc(&self) -> &str {
        "True Strength Index - Double-smoothed momentum"
    }

    fn calculate(&mut self, data: &[Bar]) {
        self.update(data, 0);
    }

    fn update(&mut self, data: &[Bar], from: usize) {
        self.stepper
            .run(data, from, &mut self.values, |state, bar| {
                if !bar.close.is_finite() {
                    state.1 = None;
                    state.2 = Default::default();
                    return IndicatorValue::None;
                }
                let previous = state.1.replace(bar.close);
                let Some(previous) = previous else {
                    return IndicatorValue::None;
                };
                let change = bar.close - previous;
                let first_alpha = 2.0 / (self.long_period as f64 + 1.0);
                let second_alpha = 2.0 / (self.short_period as f64 + 1.0);
                let first = state.2[0].next(change, self.long_period, first_alpha);
                let second = state.2[1].next(first, self.short_period, second_alpha);
                let abs_first = state.2[2].next(change.abs(), self.long_period, first_alpha);
                let abs_second = state.2[3].next(abs_first, self.short_period, second_alpha);
                let tsi = if !second.is_finite() || !abs_second.is_finite() {
                    f64::NAN
                } else if abs_second == 0.0 {
                    0.0
                } else {
                    100.0 * second / abs_second
                };
                let signal = state.2[4].next(
                    tsi,
                    self.signal_period,
                    2.0 / (self.signal_period as f64 + 1.0),
                );
                if tsi.is_nan() {
                    IndicatorValue::None
                } else {
                    IndicatorValue::Multiple(vec![tsi, signal])
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
            format!("TSI({},{})", self.long_period, self.short_period),
            format!("Signal({})", self.signal_period),
        ]
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::studies::test_support::make_bar;

    /// Blau, "Momentum, Direction, and Divergence" (1995): TSI = 100 * EMA(EMA(m, r), s) / EMA(EMA(|m|, r), s),
    /// each EMA seeded with the mean of its first full window.
    #[test]
    fn matches_blau_from_the_first_price_change() {
        let data: Vec<Bar> = (0..45)
            .map(|i| make_bar(100.0 + ((7 * i) % 11) as f64 * 0.5 + 0.25 * i as f64))
            .collect();
        let expected: [(usize, f64, f64); 36] = [
            (9, 25.26427061310783, 22.14576941845351),
            (10, 15.574670571010255, 18.860219994731885),
            (11, -2.2525753031685896, 8.303822345781647),
            (12, 16.965170607390633, 12.63449647658614),
            (13, 9.291635280600124, 10.963065878593131),
            (14, 28.168363285950534, 19.56571458227183),
            (15, 19.670299902737106, 19.61800724250447),
            (16, 2.018498681654022, 10.818252962079246),
            (17, 19.941762580226445, 15.380007771152846),
            (18, 11.667421103645005, 13.523714437398926),
            (19, -5.2733148606528255, 4.125199788373051),
            (20, 15.20292152451087, 9.664060656441961),
            (21, 8.035293482822471, 8.849677069632216),
            (22, -8.165190261101198, 0.3422434042655089),
            (23, 13.484735361176787, 6.913489382721147),
            (24, 6.792410170307287, 6.852949776514217),
            (25, 26.71457047995074, 16.78376012823248),
            (26, 18.623862840009018, 17.70381148412075),
            (27, 1.215604461889991, 9.45970797300537),
            (28, 19.47220410703292, 14.465956040019144),
            (29, 11.332446099508273, 12.89920106976371),
            (30, -5.527037005754623, 3.686082032004543),
            (31, 15.057274717486768, 9.371678374745656),
            (32, 7.932591325850813, 8.652134850298236),
            (33, -8.24231893575003, 0.20490795727410305),
            (34, 13.440837206905682, 6.822872582089893),
            (35, 6.761605311682332, 6.792238946886112),
            (36, 26.696961935956477, 16.744600441421294),
            (37, 18.61134106486778, 17.677970753144535),
            (38, 1.2060814660876573, 9.442026109616096),
            (39, 19.466672610210143, 14.45434935991312),
            (40, 11.328519054459866, 12.891434207186492),
            (41, -5.530000943296097, 3.680716631945198),
            (42, 15.055577931411358, 9.368147281678278),
            (43, 7.931397185803277, 8.649772233740777),
            (44, -8.243214431104834, 0.20327890131797144),
        ];

        let mut tsi = TrueStrengthIndex::new(5, 3, 3);
        tsi.calculate(&data);

        assert_eq!(tsi.values.len(), data.len());
        assert!(
            tsi.values.to_rows()[..7]
                .iter()
                .all(|v| matches!(v, IndicatorValue::None))
        );
        for (bar, main, signal) in expected {
            let IndicatorValue::Multiple(ref got) = tsi.values.to_rows()[bar] else {
                panic!(
                    "bar {bar}: expected a value, got {:?}",
                    tsi.values.to_rows()[bar]
                );
            };
            assert_eq!(got.len(), tsi.line_count());
            assert!(
                (got[0] - main).abs() < 1e-12,
                "bar {bar}: TSI {} vs {main}",
                got[0]
            );
            assert!(
                (got[1] - signal).abs() < 1e-12,
                "bar {bar}: signal {} vs {signal}",
                got[1]
            );
        }
    }
}
