use crate::model::Bar;
use crate::studies::stepper::Stepper;
use crate::studies::{Indicator, IndicatorValue, IndicatorValues};
use egui::Color32;

/// Anchored VWAP - VWAP calculated from a specific anchor bar index forward.
/// typical_price = (high + low + close) / 3
/// VWAP = cumsum(volume * typical_price) / cumsum(volume) from anchor_bar onward.
#[derive(Clone)]
pub struct AnchoredVWAP {
    anchor_bar: usize,
    values: IndicatorValues,
    stepper: Stepper<(usize, f64, f64)>,
    colors: Vec<Color32>,
    visible: bool,
}

impl AnchoredVWAP {
    /// Create with a zero-based anchor bar index for volume-weighted averaging.
    pub fn new(anchor_bar: usize) -> Self {
        Self {
            anchor_bar,
            values: IndicatorValues::new(),
            stepper: Stepper::default(),
            colors: vec![Color32::from_rgb(30, 136, 229)],
            visible: true,
        }
    }
}

impl Default for AnchoredVWAP {
    fn default() -> Self {
        Self::new(0)
    }
}

impl Indicator for AnchoredVWAP {
    fn shading(&self) -> Option<crate::studies::StudyShading> {
        Some(crate::studies::StudyShading::price_side(0))
    }

    fn source(&self) -> Option<crate::studies::StudySource> {
        Some(crate::studies::StudySource::Price(
            crate::model::PriceSource::HLC3,
        ))
    }

    crate::studies::inputs::study_inputs!(self;
        anchor_bar: length("Anchor bar", 0, usize::MAX)
    ; {});

    fn name(&self) -> &str {
        "AVWAP"
    }

    fn desc(&self) -> &str {
        "Anchored VWAP - Volume weighted average price from anchor bar"
    }

    fn calculate(&mut self, data: &[Bar]) {
        self.update(data, 0);
    }

    fn update(&mut self, data: &[Bar], from: usize) {
        self.stepper
            .run(data, from, &mut self.values, |state, bar| {
                let index = state.0;
                state.0 += 1;
                if index < self.anchor_bar
                    || ![bar.high, bar.low, bar.close, bar.volume]
                        .iter()
                        .all(|x| x.is_finite())
                {
                    return IndicatorValue::None;
                }
                let price = bar.typical_price();
                state.1 += price * bar.volume;
                state.2 += bar.volume;
                if state.2 == 0.0 {
                    IndicatorValue::None
                } else {
                    IndicatorValue::Single(state.1 / state.2)
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
        if !colors.is_empty() {
            self.colors = colors;
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
        vec![format!("AVWAP({})", self.anchor_bar)]
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::studies::InputValue;

    fn bar(high: f64, low: f64, close: f64, volume: f64) -> Bar {
        Bar {
            time: chrono::Utc::now(),
            open: close,
            high,
            low,
            close,
            volume,
            colors: None,
            payload: None,
        }
    }

    #[test]
    fn zero_volume_stays_blank_and_a_new_anchor_drops_the_old_sums() {
        let mut vwap = AnchoredVWAP::new(0);
        vwap.calculate(&[bar(3.0, 1.0, 2.0, 0.0)]);
        assert!(matches!(vwap.values().to_rows()[0], IndicatorValue::None));

        let data = vec![bar(3.0, 1.0, 2.0, 10.0), bar(6.0, 4.0, 5.0, 10.0)];
        vwap.calculate(&data);
        vwap.set_input("anchor_bar", InputValue::Length(1)).unwrap();
        vwap.calculate(&data);
        let mut fresh = AnchoredVWAP::new(1);
        fresh.calculate(&data);
        let fresh_rows = fresh.values().to_rows();
        let same = vwap
            .values()
            .to_rows()
            .iter()
            .zip(&fresh_rows)
            .all(|(left, right)| match (left, right) {
                (IndicatorValue::None, IndicatorValue::None) => true,
                (IndicatorValue::Single(left), IndicatorValue::Single(right)) => left == right,
                _ => false,
            });
        assert!(same);
        let IndicatorValue::Single(value) = fresh.values().to_rows()[1] else {
            panic!("missing");
        };
        assert!((value - 5.0).abs() < 1e-9);
    }
}
