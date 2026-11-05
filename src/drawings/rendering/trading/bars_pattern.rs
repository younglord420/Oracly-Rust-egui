//! Bars pattern rendering.

use super::candles::paint_column;
use crate::drawings::domain::{BarsPatternMode, Drawing};
use crate::drawings::rendering::ChartView;
use crate::styles::stroke;
use crate::tokens::DESIGN_TOKENS;
use egui::{Color32, Pos2, Stroke};

impl Drawing {
    /// Draws the copied bars from the first anchor's bar on, in the chart's candle width as the
    /// mode's columns or as a line through one price of each bar.
    pub(crate) fn render_bars_pattern(&self, painter: &egui::Painter, view: &ChartView) {
        let Some(from) = self.chart_points.first() else {
            return;
        };
        let bars = self.bars_pattern_bars(view.bars);
        if bars.is_empty() {
            return;
        }
        let color = self.bars_pattern_color();
        let first = from.bar_idx.round();
        let mode = self.bars_pattern_config().mode;
        let price = |bar: &[f64; 4]| match mode {
            BarsPatternMode::LineOpen => bar[0],
            BarsPatternMode::LineHigh => bar[1],
            BarsPatternMode::LineLow => bar[2],
            _ => bar[3],
        };
        match mode {
            BarsPatternMode::HlBars | BarsPatternMode::OcBars => {
                for (i, bar) in bars.iter().enumerate() {
                    let span = if mode == BarsPatternMode::HlBars {
                        [bar[1], bar[2]]
                    } else {
                        [bar[0], bar[3]]
                    };
                    paint_column(painter, view, first + i as f32, span, color);
                }
            }
            _ => {
                let points: Vec<Pos2> = bars
                    .iter()
                    .enumerate()
                    .map(|(i, bar)| {
                        Pos2::new(
                            (view.bar_to_x)(first + i as f32),
                            (view.price_to_y)(price(bar)),
                        )
                    })
                    .filter(|p| p.x.is_finite() && p.y.is_finite())
                    .collect();
                let width = self.stroke_width.max(stroke::HAIRLINE);
                painter.add(egui::Shape::line(points, Stroke::new(width, color)));
            }
        }
    }

    /// The colour of the copy: the line colour, or the accent while that is the default
    /// drawing colour, at the line colour's opacity and translucent so the chart shows through.
    fn bars_pattern_color(&self) -> Color32 {
        let default = [
            crate::drawings::manager::DEFAULT_DRAWING_COLOR_DARK,
            crate::drawings::manager::DEFAULT_DRAWING_COLOR_LIGHT,
        ]
        .contains(&self.color);
        let base = if default {
            DESIGN_TOKENS.semantic.extended.accent
        } else {
            self.color32()
        };
        base.gamma_multiply(0.6)
    }
}

#[cfg(test)]
mod tests {
    use crate::drawings::rendering::{ChartView, sample_bars};
    use crate::drawings::{ChartPoint, Drawing, DrawingToolType};
    use egui::{Rect, Shape};

    /// Each copied bar is one column from its high to its low, centred on its bar; a line mode
    /// draws one line through the chosen price.
    #[test]
    fn copies_draw_columns_or_a_line_on_their_bars() {
        let bars = sample_bars();
        let mut drawing = Drawing::new(1, DrawingToolType::BarsPattern);
        drawing.chart_points = vec![ChartPoint::new(1.0, 12.0), ChartPoint::new(3.0, 15.0)];
        drawing.completed = true;
        drawing.capture_bars_pattern(&bars);
        drawing.chart_points[0] = ChartPoint::new(8.0, 12.0);
        drawing.chart_points[1] = ChartPoint::new(10.0, 12.0);
        let bar_to_x = |b: f32| 20.0 + b * 10.0;
        let price_to_y = |p: f64| 400.0 - p as f32 * 10.0;
        drawing.update_screen_coords(bar_to_x, price_to_y);
        let view = ChartView {
            instrument: None,
            bars: &bars,
            revision: 0,
            bar_to_x: &bar_to_x,
            price_to_y: &price_to_y,
            y_to_price: &|y| f64::from((400.0 - y) / 10.0),
            time_axis: true,
            executions: None,
        };
        let shapes = |drawing: &Drawing| {
            let ctx = egui::Context::default();
            let mut output = ctx.run_ui(egui::RawInput::default(), |ui| {
                drawing.render(
                    ui.painter(),
                    Rect::from_min_max(egui::Pos2::ZERO, egui::pos2(600.0, 500.0)),
                    &view,
                );
            });
            output.textures_delta.clear();
            output
                .shapes
                .into_iter()
                .map(|s| s.shape)
                .collect::<Vec<_>>()
        };
        let columns: Vec<_> = shapes(&drawing)
            .into_iter()
            .filter_map(|s| match s {
                Shape::Rect(r) => Some(r.rect),
                _ => None,
            })
            .collect();
        assert_eq!(columns.len(), 3);
        for (i, (column, bar)) in columns.iter().zip(&bars[1..=3]).enumerate() {
            assert!((column.center().x - bar_to_x(8.0 + i as f32)).abs() <= 1.0);
            assert!((column.top() - price_to_y(bar.high)).abs() <= 1.0);
            assert!((column.bottom() - price_to_y(bar.low)).abs() <= 1.0);
        }
        let mut config = drawing.bars_pattern_config();
        config.mode = crate::drawings::BarsPatternMode::LineClose;
        drawing.bars_pattern = Some(config);
        let lines: Vec<_> = shapes(&drawing)
            .into_iter()
            .filter(|s| matches!(s, Shape::Path(_) | Shape::LineSegment { .. }))
            .collect();
        assert_eq!(lines.len(), 1);
    }
}
