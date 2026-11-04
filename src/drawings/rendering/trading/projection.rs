//! Projection tool rendering

use crate::drawings::domain::Drawing;
use crate::drawings::rendering::ChartView;
use crate::drawings::rendering::utils::draw_dashed_line;
use crate::model::percent_change;
use crate::scales::{
    DefaultPriceFormatter, PriceFormatter, instrument_price_precision, price_decimals,
};
use crate::styles::stroke;
use crate::tokens::DESIGN_TOKENS;
use egui::{Color32, Pos2, Rect, Stroke, epaint::StrokeKind};

impl Drawing {
    /// Renders Projection Tool with measured move and percentage.
    /// Features: AB=CD style projection, percentage labels, target zone.
    pub(crate) fn render_projection_tool(
        &self,
        painter: &egui::Painter,
        _chart_rect: Rect,
        view: &ChartView,
    ) {
        if self.points.len() < 2 {
            return;
        }
        let start = self.points[0];
        let end = self.points[1];
        let color = Color32::from_rgba_unmultiplied(
            self.color[0],
            self.color[1],
            self.color[2],
            self.color[3],
        );

        // Main projection line with arrow
        self.draw_projection_main_line(painter, start, end, color);

        // The measured move repeats from the third anchor; until it is placed there is none.
        let [a, b, c, ..] = self.chart_points.as_slice() else {
            return;
        };
        let Some(&proj_point) = self.points.get(2) else {
            return;
        };
        let dashed_color =
            Color32::from_rgba_unmultiplied(self.color[0], self.color[1], self.color[2], 150);
        let proj_end = Pos2::new(
            (view.bar_to_x)(c.bar_idx + b.bar_idx - a.bar_idx),
            (view.price_to_y)(c.price + b.price - a.price),
        );
        draw_dashed_line(
            painter,
            proj_point,
            proj_end,
            Stroke::new(self.stroke_width, dashed_color),
            8.0,
            5.0,
        );

        // Target zone (shaded rectangle at projection end)
        let zone = target_zone(end - start);
        self.draw_projection_target_zone(painter, proj_end, zone, dashed_color);

        let precision = view
            .instrument
            .and_then(instrument_price_precision)
            .map_or_else(|| price_decimals(a.price), usize::from);
        self.draw_projection_labels(
            painter,
            [start, end, proj_end],
            zone.y,
            precision,
            color,
            dashed_color,
        );
    }

    pub(super) fn draw_projection_main_line(
        &self,
        painter: &egui::Painter,
        start: Pos2,
        end: Pos2,
        color: Color32,
    ) {
        let width = self.stroke_width.max(stroke::HAIRLINE);
        painter.line_segment([start, end], Stroke::new(width, color));

        // The arrowhead grows with the line.
        let dx = end.x - start.x;
        let dy = end.y - start.y;
        let angle = dy.atan2(dx);

        let arrow_size = 9.0 + width * 1.5;
        let arrow_angle = 0.4;
        let p1 = Pos2::new(
            end.x - arrow_size * (angle - arrow_angle).cos(),
            end.y - arrow_size * (angle - arrow_angle).sin(),
        );
        let p2 = Pos2::new(
            end.x - arrow_size * (angle + arrow_angle).cos(),
            end.y - arrow_size * (angle + arrow_angle).sin(),
        );
        painter.line_segment([end, p1], Stroke::new(width.max(stroke::THICK), color));
        painter.line_segment([end, p2], Stroke::new(width.max(stroke::THICK), color));
    }

    fn draw_projection_target_zone(
        &self,
        painter: &egui::Painter,
        proj_end: Pos2,
        zone: egui::Vec2,
        dashed_color: Color32,
    ) {
        let zone_fill = self.fill_color32().unwrap_or_else(|| {
            Color32::from_rgba_unmultiplied(self.color[0], self.color[1], self.color[2], 30)
        });
        let zone_rect = Rect::from_center_size(proj_end, zone);
        painter.rect_filled(zone_rect, DESIGN_TOKENS.rounding.md, zone_fill);
        painter.rect_stroke(
            zone_rect,
            DESIGN_TOKENS.rounding.md,
            Stroke::new(stroke::HAIRLINE, dashed_color),
            StrokeKind::Outside,
        );
    }

    fn draw_projection_labels(
        &self,
        painter: &egui::Painter,
        [start, end, proj_end]: [Pos2; 3],
        zone_height: f32,
        precision: usize,
        color: Color32,
        dashed_color: Color32,
    ) {
        let [a, b, ..] = self.chart_points.as_slice() else {
            return;
        };
        let format = |price: f64| DefaultPriceFormatter::with_precision(precision).format(price);
        let change = b.price - a.price;
        let sign = if change < 0.0 { "\u{2212}" } else { "+" };
        let pct_change = percent_change(a.price, b.price).abs();

        // Move label (at midpoint of AB)
        let mid_ab = Pos2::new((start.x + end.x) / 2.0, (start.y + end.y) / 2.0);
        let label_text = format!("{sign}{} ({sign}{pct_change:.2}%)", format(change.abs()));
        self.label_box(
            painter,
            egui::Align2::CENTER_CENTER,
            Pos2::new(mid_ab.x + 10.0, mid_ab.y - 10.0),
            label_text,
            color,
            DESIGN_TOKENS
                .semantic
                .extended
                .chart_axis_bg
                .gamma_multiply(0.86),
        );

        // Projection target label
        let target_label = self.chart_points.get(2).map_or_else(
            || "Target: n/a".to_owned(),
            |c| {
                format!(
                    "Target: {}",
                    format(c.price + self.chart_points[1].price - self.chart_points[0].price)
                )
            },
        );
        self.label_box_on(
            painter,
            egui::Align2::CENTER_CENTER,
            Pos2::new(proj_end.x, proj_end.y - zone_height / 2.0 - 12.0),
            target_label,
            dashed_color.to_opaque(),
        );
    }
}

/// Target zone a fifth of the measured move on each axis, at least 20 points either way.
fn target_zone(leg: egui::Vec2) -> egui::Vec2 {
    (leg.abs() * 0.2).max(egui::Vec2::splat(20.0))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::drawings::{ChartPoint, DrawingToolType};

    #[test]
    fn projection_label_boxes_contain_long_prices() {
        for price in [100.0, 123_456_789.0, -123_456_789.0] {
            let mut drawing = Drawing::new(1, DrawingToolType::ProjectionTool);
            drawing.chart_points = vec![
                ChartPoint::new(0.0, price),
                ChartPoint::new(1.0, price * 2.0),
                ChartPoint::new(2.0, price),
            ];
            let ctx = egui::Context::default();
            let mut output = ctx.run_ui(egui::RawInput::default(), |ui| {
                drawing.draw_projection_labels(
                    ui.painter(),
                    [
                        Pos2::new(100.0, 300.0),
                        Pos2::new(300.0, 200.0),
                        Pos2::new(500.0, 100.0),
                    ],
                    20.0,
                    2,
                    Color32::BLUE,
                    Color32::BLUE,
                );
            });
            // egui panics when an output with unfreed textures is dropped.
            output.textures_delta.clear();
            let texts: Vec<_> = output
                .shapes
                .iter()
                .filter_map(|s| match &s.shape {
                    egui::Shape::Text(t) => Some(t),
                    _ => None,
                })
                .collect();
            assert_eq!(texts.len(), 2);
            for text in texts {
                let bounds = Rect::from_min_size(text.pos, text.galley.size());
                assert!(
                    output.shapes.iter().any(|s| matches!(
                        &s.shape, egui::Shape::Rect(r) if r.rect.contains_rect(bounds)
                    )),
                    "label box must contain {:?}",
                    text.galley.job.text
                );
            }
        }
    }

    #[test]
    fn projection_adds_the_price_leg_on_a_logarithmic_chart() {
        use crate::widget::test_frames::{chart, shapes};
        let mut chart = chart(&[80.0], &[200.0]);
        chart.config_mut().right_axis_scale_mode = crate::scales::PriceScaleMode::Logarithmic;
        let mut drawing = Drawing::new(1, DrawingToolType::ProjectionTool);
        drawing.chart_points = vec![
            ChartPoint::new(10.0, 100.0),
            ChartPoint::new(20.0, 150.0),
            ChartPoint::new(40.0, 120.0),
        ];
        drawing.completed = true;
        chart.drawings_mut().drawings.push(drawing);
        let output = shapes(&egui::Context::default(), &mut chart, 0.0, vec![]);
        let map = chart.get_chart_mapping();
        // D=C+(B-A): bar 40+(20-10)=50, price 120+(150-100)=170.
        let target = Pos2::new(map.idx_to_x(50), map.price_to_y(170.0));
        let width = target_zone(egui::vec2(map.idx_to_x(20) - map.idx_to_x(10), 0.0)).x;
        // The target zone, painted as a rect or within a tessellated mesh, whose fill
        // vertices stop half a pixel inside the rect for the feathered edge.
        let [r, g, b, _] = chart.drawings().drawings[0].color;
        let fill = egui::Color32::from_rgba_unmultiplied(r, g, b, 30);
        assert!(output.iter().any(|s| match s {
            egui::Shape::Rect(r) =>
                (r.rect.width() - width).abs() < 1e-3 && r.rect.center().distance(target) < 1e-2,
            egui::Shape::Mesh(m) => {
                let zone = Rect::from_points(
                    &(m.vertices.iter())
                        .filter(|v| v.color == fill)
                        .map(|v| v.pos)
                        .collect::<Vec<_>>(),
                );
                (zone.width() - (width - 1.0)).abs() < 1e-3 && zone.center().distance(target) < 1e-2
            }
            _ => false,
        }));
        assert!(
            output
                .iter()
                .any(|s| matches!(s,egui::Shape::Text(t) if t.galley.job.text=="Target: 170.00"))
        );
    }
}
