//! Projection tool rendering

use crate::drawings::domain::Drawing;
use crate::drawings::rendering::ChartView;
use crate::model::percent_change;
use crate::scales::{
    DefaultPriceFormatter, PriceFormatter, instrument_price_precision, price_decimals,
};
use crate::styles::stroke;
use egui::{Color32, Pos2, Rect, Shape, Stroke};

impl Drawing {
    /// A sector about the first anchor: the second anchor fixes the radius and the start ray,
    /// the third the end ray. It is filled, outlined in the line colour, and labelled at the end
    /// of its arc with the price move from the centre.
    pub(crate) fn render_projection_tool(
        &self,
        painter: &egui::Painter,
        _chart_rect: Rect,
        view: &ChartView,
    ) {
        let stroke = Stroke::new(self.stroke_width.max(stroke::HAIRLINE), self.color32());
        let Some(points) = self.sector_points() else {
            if let [center, start, ..] = *self.points {
                painter.line_segment([center, start], stroke);
            }
            return;
        };
        if let Some(fill) = self.painted_fill() {
            let mut mesh = egui::Mesh::default();
            for &point in &points {
                mesh.colored_vertex(point, fill);
            }
            for i in 1..points.len() - 1 {
                mesh.add_triangle(0, i as u32, (i + 1) as u32);
            }
            painter.add(Shape::mesh(mesh));
        }
        painter.add(Shape::closed_line(points.clone(), stroke));

        let (Some(center), Some(&arc_end)) = (self.chart_points.first(), points.last()) else {
            return;
        };
        let price = (view.y_to_price)(arc_end.y);
        if !price.is_finite() {
            return;
        }
        let precision = view
            .instrument
            .and_then(instrument_price_precision)
            .map_or_else(|| price_decimals(center.price), usize::from);
        let format = |price: f64| DefaultPriceFormatter::with_precision(precision).format(price);
        let change = price - center.price;
        let sign = if change < 0.0 { "\u{2212}" } else { "+" };
        let percent = percent_change(center.price, price).abs();
        let text = format!(
            "{} ({sign}{}, {sign}{percent:.2}%)",
            format(price),
            format(change.abs())
        );
        let outward = (arc_end - points[0]).normalized();
        let align = egui::Align2([
            if outward.x >= 0.0 {
                egui::Align::Min
            } else {
                egui::Align::Max
            },
            if outward.y >= 0.0 {
                egui::Align::Min
            } else {
                egui::Align::Max
            },
        ]);
        let color = self.color32().to_opaque();
        self.label_box_on(painter, align, arc_end + outward * 6.0, text, color);
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
}

#[cfg(test)]
mod tests {
    use crate::drawings::{ChartPoint, DrawingToolType};
    use egui::Pos2;

    /// The sector is filled by default, outlined, and its end is labelled with the price there
    /// and the move from the centre.
    #[test]
    fn projection_is_a_labelled_sector() {
        use crate::widget::test_frames::{chart, shapes};
        let mut chart = chart(&[80.0], &[200.0]);
        let mut drawing = crate::drawings::Drawing::new(1, DrawingToolType::ProjectionTool);
        drawing.chart_points = vec![
            ChartPoint::new(10.0, 100.0),
            ChartPoint::new(30.0, 100.0),
            ChartPoint::new(30.0, 150.0),
        ];
        drawing.completed = true;
        chart.drawings_mut().drawings.push(drawing);
        let output = shapes(&egui::Context::default(), &mut chart, 0.0, vec![]);
        let fill = chart.drawings().drawings[0].painted_fill().unwrap();
        assert!(output.iter().any(|s| matches!(s, egui::Shape::Mesh(m)
            if m.vertices.iter().any(|v| v.color == fill))));
        let map = chart.get_chart_mapping();
        let center = Pos2::new(map.idx_to_x(10), map.price_to_y(100.0));
        let radius = Pos2::new(map.idx_to_x(30), map.price_to_y(100.0)).distance(center);
        let line = chart.drawings().drawings[0].color32();
        assert!(output.iter().any(|s| matches!(s, egui::Shape::Mesh(m)
            if m.vertices.iter().any(|v| v.color == line
                && (v.pos.distance(center) - radius).abs() < 1.5))));
        assert!(output.iter().any(|s| matches!(s, egui::Shape::Text(t)
            if t.galley.job.text.contains('%'))));
    }
}
