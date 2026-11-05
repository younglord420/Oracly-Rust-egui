//! Ghost feed rendering.

use super::candles::paint_candle;
use crate::drawings::domain::Drawing;
use crate::drawings::rendering::ChartView;
use crate::theme::ThemeExt;
use egui::Color32;

impl Drawing {
    /// Projects candles in instrument ticks, each on the bar it closes on and shaped and
    /// coloured like the chart's candles. The clicked path shows only while it is drawn.
    pub(crate) fn render_ghost_feed(&self, painter: &egui::Painter, view: &ChartView) {
        // Without tick metadata, one price unit is one tick.
        let tick_size = view
            .instrument
            .and_then(|i| i.tick_size)
            .filter(|tick| tick.is_finite() && *tick > 0.0)
            .unwrap_or(1.0);
        let config = self.ghost_feed.unwrap_or_default();
        let theme = painter.ctx().active_theme();
        let opacity = f32::from(config.opacity.min(100)) / 100.0;
        let color = |set: Option<[u8; 4]>, chart: Color32| {
            set.map_or(chart, |[r, g, b, a]| {
                Color32::from_rgba_unmultiplied(r, g, b, a)
            })
            .gamma_multiply(opacity)
        };
        let up = color(config.up_color, theme.chart.candle_up);
        let down = color(config.down_color, theme.chart.candle_down);
        for c in self.ghost_candles(tick_size) {
            paint_candle(
                painter,
                view,
                c.bar_idx + 1.0,
                [c.open, c.high, c.low, c.close],
                if c.close >= c.open { up } else { down },
            );
        }
        if !self.completed {
            for pair in self.points.windows(2) {
                self.styled_segment(painter, pair[0], pair[1], self.stroke());
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::drawings::rendering::ChartView;
    use crate::drawings::{ChartPoint, DrawingToolType};
    use egui::{Rect, Shape};

    /// Candles sit on the bars they close on in the chart's candle colours at the set opacity,
    /// and the path shows only while the drawing is placed.
    #[test]
    fn ghost_candles_look_like_the_charts_candles() {
        let mut drawing = Drawing::new(1404, DrawingToolType::GhostFeed);
        drawing.chart_points = vec![ChartPoint::new(0.0, 10.0), ChartPoint::new(32.0, 11.0)];
        let bar_to_x = |b| 20.0 + b * 10.0;
        let price_to_y = |p| 400.0 - p as f32 * 10.0;
        drawing.update_screen_coords(bar_to_x, price_to_y);
        let instrument = crate::model::Instrument::new("TEST").with_tick_size(0.25);
        let view = ChartView {
            instrument: Some(&instrument),
            bars: &[],
            revision: 0,
            bar_to_x: &bar_to_x,
            price_to_y: &price_to_y,
            y_to_price: &|y| f64::from((400.0 - y) / 10.0),
            time_axis: true,
            executions: None,
        };
        for completed in [false, true] {
            drawing.completed = completed;
            let ctx = egui::Context::default();
            let theme = ctx.active_theme();
            let up = theme.chart.candle_up.gamma_multiply(0.6);
            let down = theme.chart.candle_down.gamma_multiply(0.6);
            let mut output = ctx.run_ui(egui::RawInput::default(), |ui| {
                drawing.render(
                    ui.painter(),
                    Rect::from_min_max(egui::Pos2::ZERO, egui::pos2(600.0, 500.0)),
                    &view,
                );
            });
            output.textures_delta.clear();
            let candles = drawing.ghost_candles(0.25);
            let mut bodies = 0;
            let mut path = 0;
            for shape in output.shapes.iter().map(|s| &s.shape) {
                match shape {
                    Shape::Rect(body) => {
                        let candle = candles
                            .iter()
                            .find(|c| {
                                (bar_to_x(c.bar_idx + 1.0) - body.rect.center().x).abs() <= 1.0
                            })
                            .expect("body on the bar its candle closes on");
                        let color = if candle.close >= candle.open {
                            up
                        } else {
                            down
                        };
                        assert_eq!(body.fill, color);
                        bodies += 1;
                    }
                    Shape::LineSegment { stroke, .. } if *stroke == drawing.stroke() => path += 1,
                    Shape::LineSegment { stroke, .. } => {
                        assert!(stroke.color == up || stroke.color == down, "{stroke:?}");
                    }
                    shape => panic!("unexpected ghost decoration: {shape:?}"),
                }
            }
            assert_eq!(bodies, candles.len());
            assert_eq!(path > 0, !completed);
        }
    }
}
