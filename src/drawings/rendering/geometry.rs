//! Geometry tool rendering: the Mach wavefront figures and the Dedekind tessellation.

use crate::drawings::domain::{
    DedekindGeodesic, Drawing, DrawingToolType, dedekind_geodesics, mach_figure,
};
use crate::styles::stroke;
use egui::{Align2, Color32, Pos2, Rect, Stroke, epaint::PathShape};

/// Arcs smaller than this many points are left out of a Dedekind tessellation.
const MIN_ARC_RADIUS: f32 = 0.75;

fn rgba(color: [u8; 4]) -> Color32 {
    Color32::from_rgba_unmultiplied(color[0], color[1], color[2], color[3])
}

/// A ratio as short text: trailing zeros dropped.
fn ratio_text(ratio: f32) -> String {
    let text = format!("{ratio:.3}");
    text.trim_end_matches('0').trim_end_matches('.').to_owned()
}

impl Drawing {
    pub(crate) fn render_geometry(&self, painter: &egui::Painter) {
        let &[a, b, ..] = self.points.as_slice() else {
            return;
        };
        if self.tool_type == DrawingToolType::DedekindTessellation {
            self.paint_dedekind(painter, Rect::from_two_pos(a, b));
            return;
        }
        let config = self.geometry_config();
        let Some(figure) = mach_figure(a, b, &config.levels, config.mach_number(self.tool_type))
        else {
            painter.line_segment([a, b], self.stroke());
            return;
        };
        let width = self.stroke_width.max(stroke::HAIRLINE);
        for circle in &figure.circles {
            painter.circle_stroke(
                circle.center,
                circle.radius,
                Stroke::new(width, rgba(circle.color)),
            );
        }
        // The envelope reads a little stronger than the circles, in the line colour.
        let envelope = Stroke::new(width + 0.5, self.color32());
        for ray in &figure.rays {
            painter.line_segment(*ray, envelope);
        }
        painter.circle_filled(figure.nose, (width + 1.5).max(2.5), self.color32());
        if config.show_ratios {
            let font = self.annotation_font();
            for circle in &figure.circles {
                painter.text(
                    circle.center + figure.direction * circle.radius + egui::vec2(4.0, 0.0),
                    Align2::LEFT_CENTER,
                    ratio_text(circle.ratio),
                    font.clone(),
                    rgba(circle.color),
                );
            }
        }
    }

    /// The box the corners span, faintly outlined, and the tessellation clipped to it.
    fn paint_dedekind(&self, painter: &egui::Painter, area: Rect) {
        let stroke = self.stroke();
        painter.rect_stroke(
            area,
            0.0,
            Stroke::new(stroke.width, stroke.color.gamma_multiply(0.45)),
            egui::StrokeKind::Middle,
        );
        let config = self.geometry_config();
        let clipped = painter.with_clip_rect(painter.clip_rect().intersect(area.expand(0.5)));
        for geodesic in dedekind_geodesics(area, config.max_curvature, MIN_ARC_RADIUS) {
            match geodesic {
                DedekindGeodesic::Line { x } => {
                    clipped.line_segment(
                        [Pos2::new(x, area.bottom()), Pos2::new(x, area.top())],
                        stroke,
                    );
                }
                DedekindGeodesic::Arc { center, radius } => {
                    let steps = (radius / 2.0).clamp(8.0, 96.0) as usize;
                    let points = (0..=steps)
                        .map(|i| {
                            let angle = std::f32::consts::PI * i as f32 / steps as f32;
                            center + radius * egui::vec2(-angle.cos(), -angle.sin())
                        })
                        .collect();
                    clipped.add(PathShape::line(points, stroke));
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use egui::Shape;

    fn paint(drawing: &Drawing) -> Vec<Shape> {
        let ctx = egui::Context::default();
        let output = ctx.run_ui(egui::RawInput::default(), |ui| {
            drawing.render_geometry(ui.painter());
        });
        let shapes = output.shapes.iter().map(|s| s.shape.clone()).collect();
        output.drop_without_applying_deltas();
        shapes
    }

    /// A sonic figure strokes each enabled circle in its colour, one wall and a nose; a
    /// supersonic one two cone edges; ratios are labelled unless hidden.
    #[test]
    fn mach_figures_paint_circles_envelope_and_ratios() {
        for (tool, rays) in [
            (DrawingToolType::Sonic, 1),
            (DrawingToolType::Supersonic, 2),
            (DrawingToolType::GoldenSonic, 1),
            (DrawingToolType::GoldenSupersonic, 2),
        ] {
            let mut drawing = Drawing::new(1, tool);
            drawing.points = vec![Pos2::new(100.0, 100.0), Pos2::new(120.0, 100.0)];
            let config = drawing.geometry_config();
            let on = config.levels.iter().filter(|l| l.enabled).count();
            let shapes = paint(&drawing);
            let rings = shapes
                .iter()
                .filter(|s| matches!(s, Shape::Circle(c) if c.fill == Color32::TRANSPARENT))
                .count();
            assert_eq!(rings, on, "{tool:?}");
            let lines = shapes
                .iter()
                .filter(|s| matches!(s, Shape::LineSegment { .. }))
                .count();
            assert_eq!(lines, rays, "{tool:?}");
            let texts = shapes
                .iter()
                .filter(|s| matches!(s, Shape::Text(_)))
                .count();
            assert_eq!(texts, on, "{tool:?}");
            drawing.geometry = Some(crate::drawings::GeometryConfig {
                show_ratios: false,
                ..config
            });
            assert!(!paint(&drawing).iter().any(|s| matches!(s, Shape::Text(_))));
        }
    }

    /// The tessellation stays inside its box and grows denser with the max curvature.
    #[test]
    fn dedekind_fills_its_box_by_curvature() {
        let mut drawing = Drawing::new(1, DrawingToolType::DedekindTessellation);
        drawing.points = vec![Pos2::new(10.0, 20.0), Pos2::new(410.0, 220.0)];
        let count = |drawing: &Drawing| paint(drawing).len();
        let dense = count(&drawing);
        drawing.geometry = Some(crate::drawings::GeometryConfig {
            max_curvature: 4,
            ..drawing.geometry_config()
        });
        assert!(count(&drawing) < dense);
        assert_eq!(ratio_text(1.618), "1.618");
        assert_eq!(ratio_text(2.0), "2");
    }
}
