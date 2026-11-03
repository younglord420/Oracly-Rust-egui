//! Elliott Wave tool rendering implementations
//!
//! Includes: impulse, correction, triangle, double combo, and triple combo waves.

use crate::drawings::domain::{Drawing, DrawingToolType, Wave};
use crate::drawings::rendering::ChartView;
use crate::styles::stroke;
use egui::{Align2, Pos2, Stroke};

/// The wave 3 target as a multiple of wave 1, projected from the wave 2 end.
const WAVE_3_TARGET: f64 = 1.618;

/// How far past their last anchor a triangle's boundaries run on.
const TRIANGLE_EXTENSION: f32 = 50.0;

impl Drawing {
    /// The waves the anchors of this Elliott tool mark, from its origin.
    fn elliott_waves(&self) -> Vec<Wave> {
        use Wave::{Letter, Number};
        match self.tool_type {
            DrawingToolType::ElliottImpulse => (0..=5).map(Number).collect(),
            DrawingToolType::ElliottCorrection => {
                vec![Number(0), Letter('A'), Letter('B'), Letter('C')]
            }
            DrawingToolType::ElliottTriangle => std::iter::once(Number(0))
                .chain("ABCDE".chars().map(Letter))
                .collect(),
            DrawingToolType::ElliottDoubleCombo => std::iter::once(Number(0))
                .chain("WXY".chars().map(Letter))
                .collect(),
            DrawingToolType::ElliottTripleCombo => std::iter::once(Number(0))
                .chain("WXYXZ".chars().map(Letter))
                .collect(),
            _ => Vec::new(),
        }
    }

    /// Indices of the legs ending in an X wave, which link the simple corrections of a
    /// combination and are drawn dashed.
    fn elliott_link_legs(&self) -> &'static [usize] {
        match self.tool_type {
            DrawingToolType::ElliottDoubleCombo => &[2],
            DrawingToolType::ElliottTripleCombo => &[2, 4],
            _ => &[],
        }
    }

    /// An Elliott wave: its legs, labels in the configured degree and, for an impulse, the
    /// wave 3 target; a triangle also runs its boundaries on.
    pub(crate) fn render_elliott_wave(&self, painter: &egui::Painter, view: &ChartView) {
        let color = self.color32();
        let config = self.pattern_config();

        match self.tool_type {
            DrawingToolType::ElliottCorrection if self.points.len() >= 3 => {
                self.paint_pattern_fill(painter, self.points.clone());
            }
            DrawingToolType::ElliottTriangle if self.points.len() >= 4 => {
                self.paint_pattern_fill(painter, self.points[1..].to_vec());
            }
            _ => {}
        }

        let links = self.elliott_link_legs();
        for (i, leg) in self.points.windows(2).enumerate() {
            if links.contains(&(i + 1)) {
                super::utils::draw_dashed_line(
                    painter,
                    leg[0],
                    leg[1],
                    Stroke::new(stroke::HAIRLINE, color.gamma_multiply(0.7)),
                    6.0,
                    4.0,
                );
            } else {
                painter.line_segment([leg[0], leg[1]], self.stroke());
            }
        }

        if self.tool_type == DrawingToolType::ElliottTriangle {
            self.paint_triangle_boundaries(painter);
        }

        if self.tool_type == DrawingToolType::ElliottImpulse
            && config.show_target
            && let Some(target) = self.impulse_target()
        {
            let wave2_end = self.points[2];
            let target_y = (view.price_to_y)(target);
            super::utils::draw_dashed_line(
                painter,
                wave2_end,
                Pos2::new(wave2_end.x + 40.0, target_y),
                Stroke::new(stroke::HAIRLINE, color.gamma_multiply(0.6)),
                3.0,
                2.0,
            );
            painter.text(
                Pos2::new(wave2_end.x + 45.0, target_y),
                Align2::LEFT_CENTER,
                "1.618",
                self.annotation_font(),
                color,
            );
        }

        let labels: Vec<_> = self
            .elliott_waves()
            .into_iter()
            .map(|wave| config.degree.label(wave))
            .collect();
        self.paint_anchor_labels(painter, &labels);
    }

    /// The dashed boundaries through A, C and E and through B and D, run on past their last
    /// anchor.
    fn paint_triangle_boundaries(&self, painter: &egui::Painter) {
        if self.points.len() < 5 {
            return;
        }
        let stroke = Stroke::new(stroke::HAIRLINE, self.color32().gamma_multiply(0.5));
        for side in [1, 2] {
            let anchors: Vec<Pos2> = self.points.iter().skip(side).step_by(2).copied().collect();
            let [first, second, ..] = anchors[..] else {
                continue;
            };
            let last = anchors[anchors.len() - 1];
            let dx = second.x - first.x;
            let slope = if dx.abs() > 0.001 {
                (second.y - first.y) / dx
            } else {
                0.0
            };
            let end = Pos2::new(
                last.x + TRIANGLE_EXTENSION,
                last.y + slope * TRIANGLE_EXTENSION,
            );
            super::utils::draw_dashed_line(painter, first, end, stroke, 5.0, 3.0);
        }
    }

    /// The price of the wave 3 target: the 0 to 1 price leg times [`WAVE_3_TARGET`] from the
    /// wave 2 end, in the direction of wave 1. A logarithmic construction (the drawing's
    /// `fib_config`) scales the wave 2 end by the same multiple of the leg's log return. `None`
    /// until wave 2 is placed or while wave 1 is flat.
    fn impulse_target(&self) -> Option<f64> {
        let [wave0, wave1, wave2, ..] = self.chart_points.as_slice() else {
            return None;
        };
        let (from, to, end) = (wave0.price, wave1.price, wave2.price);
        if from == to {
            return None;
        }
        if self.fib_config.as_ref().is_some_and(|c| c.logarithmic)
            && from > 0.0
            && to > 0.0
            && end > 0.0
        {
            Some((end.ln() + (to.ln() - from.ln()) * WAVE_3_TARGET).exp())
        } else {
            Some(end + (to - from) * WAVE_3_TARGET)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::drawings::domain::{ChartPoint, FibonacciConfig, PatternConfig, WaveDegree};

    #[test]
    fn impulse_target_projects_the_first_wave_from_the_wave_2_end_in_price() {
        let target = |prices: &[f64], logarithmic: bool| {
            let mut drawing = Drawing::new(0, DrawingToolType::ElliottImpulse);
            drawing.chart_points = prices
                .iter()
                .enumerate()
                .map(|(i, &price)| ChartPoint::new(10.0 * i as f32, price))
                .collect();
            drawing.fib_config = Some(FibonacciConfig {
                logarithmic,
                ..FibonacciConfig::default()
            });
            drawing.impulse_target()
        };
        // Wave 1 is 100 to 150: 120 + 50 * 1.618 above wave 2, or 120 * 1.5^1.618 on a log scale.
        // The falling impulse mirrors it. A flat wave 1, or no wave 2 yet, has no target.
        let cases = [
            (&[100.0, 150.0, 120.0][..], false, Some(200.9)),
            (
                &[100.0, 150.0, 120.0],
                true,
                Some(120.0 * 1.5f64.powf(1.618)),
            ),
            (&[150.0, 100.0, 130.0], false, Some(49.1)),
            (
                &[150.0, 100.0, 130.0],
                true,
                Some(130.0 * (2.0f64 / 3.0).powf(1.618)),
            ),
            (&[-10.0, 10.0, 0.0], true, Some(32.36)),
            (&[100.0, 100.0, 90.0], false, None),
            (&[100.0, 150.0], false, None),
        ];
        for (prices, logarithmic, want) in cases {
            match (target(prices, logarithmic), want) {
                (Some(got), Some(want)) => {
                    assert!((got - want).abs() < 1e-9, "{prices:?}: {got} != {want}")
                }
                (got, want) => assert_eq!(got, want, "{prices:?}"),
            }
        }
    }

    fn texts(drawing: &Drawing) -> Vec<String> {
        let view = ChartView {
            instrument: None,
            bars: &[],
            revision: 0,
            bar_to_x: &|x| x,
            price_to_y: &|p| 300.0 - p as f32,
            y_to_price: &|y| f64::from(300.0 - y),
            time_axis: true,
            executions: None,
        };
        let output = egui::Context::default().run_ui(egui::RawInput::default(), |ui| {
            drawing.render_elliott_wave(ui.painter(), &view);
        });
        let found = output
            .shapes
            .iter()
            .filter_map(|shape| match &shape.shape {
                egui::Shape::Text(t) => Some(t.galley.job.text.clone()),
                _ => None,
            })
            .collect();
        output.drop_without_applying_deltas();
        found
    }

    fn wave(tool: DrawingToolType) -> Drawing {
        let mut drawing = Drawing::new(0, tool);
        let count = tool.required_points().unwrap();
        for i in 0..count {
            let price = if i % 2 == 0 { 100.0 } else { 150.0 } + i as f64;
            drawing.chart_points.push(ChartPoint::new(i as f32, price));
            drawing
                .points
                .push(Pos2::new(40.0 * i as f32, 300.0 - price as f32));
        }
        drawing
    }

    /// Each Elliott tool labels its waves from the origin; combinations start at an origin
    /// before W and draw their X waves dashed.
    #[test]
    fn elliott_waves_label_every_anchor_from_the_origin() {
        let cases = [
            (
                DrawingToolType::ElliottImpulse,
                vec!["0", "1", "2", "3", "4", "5"],
            ),
            (DrawingToolType::ElliottCorrection, vec!["0", "A", "B", "C"]),
            (
                DrawingToolType::ElliottTriangle,
                vec!["0", "A", "B", "C", "D", "E"],
            ),
            (
                DrawingToolType::ElliottDoubleCombo,
                vec!["0", "W", "X", "Y"],
            ),
            (
                DrawingToolType::ElliottTripleCombo,
                vec!["0", "W", "X", "Y", "X", "Z"],
            ),
        ];
        for (tool, want) in cases {
            let mut drawing = wave(tool);
            drawing.pattern = Some(PatternConfig {
                show_target: false,
                ..PatternConfig::default()
            });
            assert_eq!(texts(&drawing), want, "{tool:?}");
        }
        assert_eq!(
            wave(DrawingToolType::ElliottTripleCombo).elliott_link_legs(),
            [2, 4]
        );
    }

    /// The degree sets the notation, and labels can be hidden.
    #[test]
    fn elliott_degree_and_label_visibility_follow_settings() {
        let mut drawing = wave(DrawingToolType::ElliottCorrection);
        drawing.pattern = Some(PatternConfig {
            degree: WaveDegree::Intermediate,
            ..PatternConfig::default()
        });
        assert_eq!(texts(&drawing), ["(0)", "(A)", "(B)", "(C)"]);
        drawing.pattern = Some(PatternConfig {
            degree: WaveDegree::Cycle,
            ..PatternConfig::default()
        });
        assert_eq!(texts(&drawing), ["a", "b", "c"]);
        drawing.pattern = Some(PatternConfig {
            show_labels: false,
            ..PatternConfig::default()
        });
        assert!(texts(&drawing).is_empty());

        let mut impulse = wave(DrawingToolType::ElliottImpulse);
        assert!(texts(&impulse).contains(&"1.618".to_owned()));
        impulse.pattern = Some(PatternConfig {
            show_target: false,
            ..PatternConfig::default()
        });
        assert!(!texts(&impulse).contains(&"1.618".to_owned()));
    }
}
