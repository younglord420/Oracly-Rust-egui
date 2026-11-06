use crate::model::Bar;
use crate::studies::palette;
use crate::studies::stepper::Stepper;
use crate::studies::{Indicator, IndicatorValue, IndicatorValues};
use egui::Color32;

/// Volume Price Trend (VPT)
/// Cumulative volume indicator that considers price changes
/// Similar to OBV but uses percentage changes
#[derive(Clone)]
pub struct VolumePriceTrend {
    values: IndicatorValues,
    stepper: Stepper<(Option<f64>, f64)>,
    color: Color32,
    visible: bool,
}

impl VolumePriceTrend {
    /// Create with the default plot colors.
    pub fn new() -> Self {
        Self {
            values: IndicatorValues::new(),
            stepper: Stepper::default(),
            color: palette::BLUE, // Teal
            visible: true,
        }
    }

    /// Set the plot color.
    pub fn with_color(mut self, color: Color32) -> Self {
        self.color = color;
        self
    }
}

impl Default for VolumePriceTrend {
    fn default() -> Self {
        Self::new()
    }
}

impl Indicator for VolumePriceTrend {
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

    fn value_format(&self) -> crate::scales::StudyFormat {
        crate::scales::StudyFormat::Volume
    }

    fn name(&self) -> &str {
        "VPT"
    }

    fn desc(&self) -> &str {
        "Volume Price Trend - Volume weighted by price change"
    }

    fn calculate(&mut self, data: &[Bar]) {
        self.update(data, 0);
    }

    fn update(&mut self, data: &[Bar], from: usize) {
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
                IndicatorValue::Single(state.1)
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
        vec!["VPT".to_string()]
    }
}

/// Resume the volume-gated index and its first-value-seeded signal.
fn volume_index(
    stepper: &mut Stepper<(Option<[f64; 2]>, f64, f64)>,
    values: &mut IndicatorValues,
    data: &[Bar],
    from: usize,
    base: f64,
    signal_length: usize,
    counts: fn(f64, f64) -> bool,
) {
    let alpha = 2.0 / (signal_length.max(1) as f64 + 1.0);
    stepper.run(data, from, values, move |state, bar| {
        if !bar.close.is_finite() || !bar.volume.is_finite() {
            return IndicatorValue::None;
        }
        if let Some([close, volume]) = state.0 {
            if counts(bar.volume, volume) && close != 0.0 {
                state.1 += state.1 * ((bar.close - close) / close);
            }
            state.2 += (state.1 - state.2) * alpha;
        } else {
            state.1 = base;
            state.2 = base;
        }
        state.0 = Some([bar.close, bar.volume]);
        IndicatorValue::Multiple(vec![state.1, state.2])
    });
}

/// Negative Volume Index (NVI)
///
/// Starts at 1000 and moves by the close's percentage change only on bars whose volume fell.
/// Outputs `Multiple([index, signal])`: the signal line is the 255-bar EMA of the index, seeded
/// with the first index value so that it starts at bar 0 like the index.
#[derive(Clone)]
pub struct NegativeVolumeIndex {
    base: f64,
    signal_length: usize,
    values: IndicatorValues,
    stepper: Stepper<(Option<[f64; 2]>, f64, f64)>,
    color: Color32,
    signal_color: Color32,
    visible: bool,
}

impl NegativeVolumeIndex {
    /// Create with the default plot colors.
    pub fn new() -> Self {
        Self {
            base: 1000.0,
            signal_length: 255,
            values: IndicatorValues::new(),
            stepper: Stepper::default(),
            color: palette::BLUE,
            signal_color: palette::WARM_ORANGE,
            visible: true,
        }
    }

    /// Set the plot color.
    pub fn with_color(mut self, color: Color32) -> Self {
        self.color = color;
        self
    }
}

impl Default for NegativeVolumeIndex {
    fn default() -> Self {
        Self::new()
    }
}

impl Indicator for NegativeVolumeIndex {
    fn shading(&self) -> Option<crate::studies::StudyShading> {
        Some(crate::studies::StudyShading::regime(0, 1))
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
        base: number("Base", f64::MIN, f64::MAX),
        signal_length: length("Signal length", 1, usize::MAX)
    ; {});

    fn required_bars(&self) -> Option<usize> {
        Some(1)
    }

    fn name(&self) -> &str {
        "NVI"
    }

    fn desc(&self) -> &str {
        "Negative Volume Index - Tracks price on low volume days"
    }

    fn calculate(&mut self, data: &[Bar]) {
        self.update(data, 0);
    }

    fn update(&mut self, data: &[Bar], from: usize) {
        volume_index(
            &mut self.stepper,
            &mut self.values,
            data,
            from,
            self.base,
            self.signal_length,
            |volume, prev| volume < prev,
        );
    }

    fn is_incremental(&self) -> bool {
        true
    }

    fn values(&self) -> &IndicatorValues {
        &self.values
    }

    fn colors(&self) -> Vec<Color32> {
        vec![self.color, self.signal_color]
    }

    fn set_colors(&mut self, colors: Vec<Color32>) {
        if let Some(&color) = colors.first() {
            self.color = color;
        }
        if let Some(&color) = colors.get(1) {
            self.signal_color = color;
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
        vec!["NVI".to_string(), "Signal".to_string()]
    }
}

/// Positive Volume Index (PVI)
///
/// Starts at 1000 and moves by the close's percentage change only on bars whose volume rose.
/// Outputs `Multiple([index, signal])`: the signal line is the 255-bar EMA of the index, seeded
/// with the first index value so that it starts at bar 0 like the index.
#[derive(Clone)]
pub struct PositiveVolumeIndex {
    base: f64,
    signal_length: usize,
    values: IndicatorValues,
    stepper: Stepper<(Option<[f64; 2]>, f64, f64)>,
    color: Color32,
    signal_color: Color32,
    visible: bool,
}

impl PositiveVolumeIndex {
    /// Create with the default plot colors.
    pub fn new() -> Self {
        Self {
            base: 1000.0,
            signal_length: 255,
            values: IndicatorValues::new(),
            stepper: Stepper::default(),
            color: palette::BLUE,
            signal_color: palette::WARM_ORANGE,
            visible: true,
        }
    }

    /// Set the plot color.
    pub fn with_color(mut self, color: Color32) -> Self {
        self.color = color;
        self
    }
}

impl Default for PositiveVolumeIndex {
    fn default() -> Self {
        Self::new()
    }
}

impl Indicator for PositiveVolumeIndex {
    fn shading(&self) -> Option<crate::studies::StudyShading> {
        Some(crate::studies::StudyShading::regime(0, 1))
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
        base: number("Base", f64::MIN, f64::MAX),
        signal_length: length("Signal length", 1, usize::MAX)
    ; {});

    fn required_bars(&self) -> Option<usize> {
        Some(1)
    }

    fn name(&self) -> &str {
        "PVI"
    }

    fn desc(&self) -> &str {
        "Positive Volume Index - Tracks price on high volume days"
    }

    fn calculate(&mut self, data: &[Bar]) {
        self.update(data, 0);
    }

    fn update(&mut self, data: &[Bar], from: usize) {
        volume_index(
            &mut self.stepper,
            &mut self.values,
            data,
            from,
            self.base,
            self.signal_length,
            |volume, prev| volume > prev,
        );
    }

    fn is_incremental(&self) -> bool {
        true
    }

    fn values(&self) -> &IndicatorValues {
        &self.values
    }

    fn colors(&self) -> Vec<Color32> {
        vec![self.color, self.signal_color]
    }

    fn set_colors(&mut self, colors: Vec<Color32>) {
        if let Some(&color) = colors.first() {
            self.color = color;
        }
        if let Some(&color) = colors.get(1) {
            self.signal_color = color;
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
        vec!["PVI".to_string(), "Signal".to_string()]
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::Utc;

    fn make_bar(close: f64, volume: f64) -> Bar {
        Bar {
            time: Utc::now(),
            open: close,
            high: close,
            low: close,
            close,
            volume,
            colors: None,
            payload: None,
        }
    }

    #[test]
    fn test_vpt() {
        let mut vpt = VolumePriceTrend::new();

        let data = vec![
            make_bar(100.0, 1000.0),
            make_bar(105.0, 1200.0), // +5% change
            make_bar(102.0, 800.0),  // -2.86% change
        ];

        vpt.calculate(&data);

        assert_eq!(vpt.values.len(), 3);

        let rows = vpt.values.to_rows();
        assert!(matches!(rows[0], IndicatorValue::None));
        let IndicatorValue::Single(value) = rows[1] else {
            panic!("no value: {:?}", rows[1]);
        };
        assert!((value - 60.0).abs() < 1e-9);
    }

    /// Index and signal line against the published definition. On a bar whose volume fell (NVI)
    /// or rose (PVI), the index is multiplied by close / previous close (Fosback, "Stock Market
    /// Logic", 1976). The index starts at 1000 and the signal is its 255-bar EMA, about one
    /// trading year, seeded with its first value as the crate's EMA is. Expected values were
    /// computed independently in exact rational arithmetic. Hand check, NVI bar 1: the index
    /// rises 5% to 1050 and the signal is 1000 + (1050 - 1000) * 2 / 256 = 1000.390625.
    #[test]
    fn nvi_pvi_match_published_definition() {
        // Volume down, up, unchanged with a zero close, down after a zero close, unchanged, up after
        // a zero close (the index holds without a previous close), down, up.
        let data: Vec<Bar> = [
            (100.0, 1000.0),
            (105.0, 800.0),
            (108.0, 1200.0),
            (0.0, 1200.0),
            (50.0, 900.0),
            (0.0, 900.0),
            (40.0, 1100.0),
            (44.0, 700.0),
            (33.0, 1400.0),
        ]
        .iter()
        .map(|&(close, volume)| make_bar(close, volume))
        .collect();
        let nvi = [
            (1000.0, 1000.0),
            (1050.0, 1000.390625),
            (1050.0, 1000.7781982421875),
            (1050.0, 1001.1627435684204),
            (1050.0, 1001.5442846342921),
            (1050.0, 1001.9228449105867),
            (1050.0, 1002.2984476847228),
            (1155.0, 1003.4914285621859),
            (1155.0, 1004.6750892765438),
        ];
        let pvi = [
            (1000.0, 1000.0),
            (1000.0, 1000.0),
            (1028.5714285714287, 1000.2232142857143),
            (1028.5714285714287, 1000.4446847098214),
            (1028.5714285714287, 1000.6644248962402),
            (1028.5714285714287, 1000.8824483624527),
            (1028.5714285714287, 1001.0987685203353),
            (1028.5714285714287, 1001.3133986769844),
            (771.4285714285714, 999.5174234641062),
        ];
        let cases = [
            (
                Box::new(NegativeVolumeIndex::new()) as Box<dyn Indicator>,
                nvi,
            ),
            (Box::new(PositiveVolumeIndex::new()), pvi),
        ];
        for (mut study, expected) in cases {
            study.calculate(&data);
            assert_eq!(study.line_count(), 2, "{}", study.name());
            assert_eq!(study.values().len(), expected.len(), "{}", study.name());
            for (bar, (value, &(index, signal))) in
                study.values().to_rows().iter().zip(&expected).enumerate()
            {
                let IndicatorValue::Multiple(lines) = value else {
                    panic!(
                        "{} bar {bar}: expected index and signal, got {value:?}",
                        study.name()
                    );
                };
                assert!(
                    (lines[0] - index).abs() < 1e-12 && (lines[1] - signal).abs() < 1e-12,
                    "{} bar {bar}: got {lines:?}, expected [{index}, {signal}]",
                    study.name()
                );
            }
        }
    }
}
