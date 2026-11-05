//! Shape and curve tool rendering implementations
//!
//! Includes: rotated rect, double curve, brush, highlighter, path, curve, and sine line.

use crate::drawings::domain::{Drawing, DrawingToolType};
use egui::{Color32, Pos2, Shape, Stroke};

impl Drawing {
    /// The first anchor is the center, the second fixes the radius, and the third fixes
    /// the end ray. The sweep takes the short arc in (-180°, 180°], clockwise at 180°.
    pub(super) fn sector_points(&self) -> Option<Vec<Pos2>> {
        let [center, start, end, ..] = *self.points else {
            return None;
        };
        let radius = center.distance(start);
        if radius < 1e-6 || center.distance_sq(end) < 1e-12 {
            return None;
        }
        let from = (start - center).angle();
        let mut sweep = (end - center).angle() - from;
        if sweep > std::f32::consts::PI {
            sweep -= std::f32::consts::TAU;
        } else if sweep <= -std::f32::consts::PI {
            sweep += std::f32::consts::TAU;
        }
        let steps = ((sweep.abs() / std::f32::consts::TAU * 128.0).ceil() as usize).max(1);
        let mut points = vec![center];
        points.extend((0..=steps).map(|i| {
            let angle = from + sweep * i as f32 / steps as f32;
            center + egui::Vec2::angled(angle) * radius
        }));
        Some(points)
    }

    pub(super) fn render_sector(&self, painter: &egui::Painter) {
        let Some(points) = self.sector_points() else {
            return;
        };
        if let Some(fill) = self.painted_fill() {
            // One mesh keeps translucent fill continuous across the fan's internal edges.
            let mut mesh = egui::Mesh::default();
            for &point in &points {
                mesh.colored_vertex(point, fill);
            }
            for i in 1..points.len() - 1 {
                mesh.add_triangle(0, i as u32, (i + 1) as u32);
            }
            painter.add(Shape::mesh(mesh));
        }
        painter.add(Shape::closed_line(points, self.stroke()));
    }

    /// A rectangle whose first two anchors form its base and whose third sets the width.
    pub(crate) fn render_rotated_rect(&self, painter: &egui::Painter) {
        let Some(corners) = self.rotated_rect_corners() else {
            return;
        };
        painter.add(egui::Shape::convex_polygon(
            corners.to_vec(),
            self.painted_fill().unwrap_or(Color32::TRANSPARENT),
            self.stroke(),
        ));
    }

    pub(super) fn rotated_rect_corners(&self) -> Option<[Pos2; 4]> {
        let &[a, b, c, ..] = self.points.as_slice() else {
            return None;
        };
        let base = b - a;
        if base.length_sq() < 1e-6 {
            return None;
        }
        let normal = egui::vec2(-base.y, base.x);
        let offset = normal * ((c - a).dot(normal) / base.length_sq());
        Some([a, b, b + offset, a + offset])
    }

    /// Cubic Bezier curve with two independently placed control points.
    pub(crate) fn render_double_curve(&self, painter: &egui::Painter) {
        let Some(points) = self.double_curve_points() else {
            return;
        };
        painter.add(egui::Shape::line(points, self.stroke()));
    }

    pub(super) fn double_curve_points(&self) -> Option<Vec<Pos2>> {
        let &[a, b, c, d, ..] = self.points.as_slice() else {
            return None;
        };
        Some(
            (0..=64)
                .map(|i| {
                    let t = i as f32 / 64.0;
                    let u = 1.0 - t;
                    (a.to_vec2() * u * u * u
                        + b.to_vec2() * 3.0 * u * u * t
                        + c.to_vec2() * 3.0 * u * t * t
                        + d.to_vec2() * t * t * t)
                        .to_pos2()
                })
                .collect(),
        )
    }

    /// The width a brush is drawn at: twice the stroke width for the brush,
    /// eight times for the highlighter; `None` for other tools.
    pub(crate) fn brush_width(&self) -> Option<f32> {
        match self.tool_type {
            DrawingToolType::Brush => Some(self.stroke_width * 2.0),
            DrawingToolType::Highlighter => Some(self.stroke_width * 8.0),
            _ => None,
        }
    }

    pub(crate) fn render_brush(&self, painter: &egui::Painter) {
        self.paint_freehand(painter, self.color32());
    }

    pub(crate) fn render_highlighter(&self, painter: &egui::Painter) {
        self.paint_freehand(painter, self.highlighter_color());
    }

    /// The marker colour. A fully opaque line colour stays see-through; any other alpha is
    /// the opacity stored on the colour.
    fn highlighter_color(&self) -> Color32 {
        let color = self.color32();
        if color.a() == 255 {
            Color32::from_rgba_unmultiplied(color.r(), color.g(), color.b(), 100)
        } else {
            color
        }
    }

    /// One continuous round stroke, so a freehand path does not darken where its segments meet.
    fn paint_freehand(&self, painter: &egui::Painter, color: Color32) {
        let width = self.brush_width().unwrap_or(self.stroke_width);
        match self.points.as_slice() {
            [] => {}
            [point] => {
                painter.circle_filled(*point, width * 0.5, color);
            }
            points => {
                painter.add(egui::Shape::line(
                    points.to_vec(),
                    Stroke::new(width, color),
                ));
            }
        }
    }

    // Note: render_sine_line is now in cycles.rs as it's a cyclic analysis tool

    /// Render Path - connected anchors ending in an arrowhead.
    pub(crate) fn render_path(&self, painter: &egui::Painter) {
        if self.points.is_empty() {
            return;
        }
        let color = self.color32();
        if let [point] = self.points[..] {
            super::style::placed_anchor(painter, point, color);
            return;
        }
        painter.add(egui::Shape::line(self.points.clone(), self.stroke()));
    }

    /// Quadratic curve. The first two anchors are the ends; the third pulls the curve.
    pub(crate) fn render_curve(&self, painter: &egui::Painter) {
        let Some([start, control, end]) = self.curve_controls() else {
            return;
        };
        let points = curve_points(start, control, end);
        if self.points.len() >= 3
            && let Some(fill) = self.painted_fill()
        {
            painter.add(egui::Shape::convex_polygon(
                points.clone(),
                fill,
                Stroke::NONE,
            ));
        }
        painter.add(egui::Shape::line(points, self.stroke()));
    }

    /// Start, off-curve control and end. Below three anchors the control sits on the
    /// chord, so the preview is the straight segment between the two ends.
    pub(super) fn curve_controls(&self) -> Option<[Pos2; 3]> {
        let (&start, &end) = (self.points.first()?, self.points.get(1)?);
        let control = self
            .points
            .get(2)
            .copied()
            .unwrap_or_else(|| Pos2::new((start.x + end.x) * 0.5, (start.y + end.y) * 0.5));
        Some([start, control, end])
    }
}

/// The quadratic Bezier curve from `p0` to `p2` pulled towards `p1`.
pub(super) fn curve_points(p0: Pos2, p1: Pos2, p2: Pos2) -> Vec<Pos2> {
    (0..=32)
        .map(|i| Drawing::quadratic_bezier(p0, p1, p2, i as f32 / 32.0))
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn shape_and_fan_geometry_uses_the_placed_controls() {
        let rect = egui::Rect::from_min_max(egui::pos2(-400.0, -400.0), egui::pos2(400.0, 400.0));
        for (tool, anchors, want) in [
            (
                DrawingToolType::Sector,
                vec![(100.0, 100.0), (200.0, 100.0), (100.0, 150.0)],
                vec![(200.0, 100.0), (170.71068, 170.71068), (100.0, 200.0)],
            ),
            (
                DrawingToolType::Sector,
                vec![(100.0, 100.0), (200.0, 100.0), (100.0, 50.0)],
                vec![(200.0, 100.0), (170.71068, 29.28932), (100.0, 0.0)],
            ),
            (
                DrawingToolType::RotatedRect,
                vec![(0.0, 0.0), (100.0, 100.0), (0.0, 100.0)],
                vec![(50.0, 150.0), (-50.0, 50.0)],
            ),
            (
                DrawingToolType::DoubleCurve,
                vec![(0.0, 0.0), (0.0, 100.0), (100.0, -100.0), (100.0, 0.0)],
                vec![(50.0, 0.0), (100.0, 0.0)],
            ),
            (
                DrawingToolType::Arc,
                vec![(-50.0, 0.0), (50.0, 0.0), (0.0, 50.0)],
                vec![(-50.0, 0.0), (50.0, 0.0), (0.0, 50.0)],
            ),
            (
                DrawingToolType::Pitchfan,
                vec![(0.0, 0.0), (100.0, 0.0), (100.0, 100.0)],
                vec![(400.0, 0.0), (400.0, 200.0), (400.0, 400.0)],
            ),
        ] {
            let mut drawing = Drawing::new(0, tool);
            drawing.points = anchors.into_iter().map(Pos2::from).collect();
            let outline = drawing.outline(rect).unwrap();
            for point in want.into_iter().map(Pos2::from) {
                assert!(
                    outline
                        .lines
                        .iter()
                        .flatten()
                        .any(|p| p.distance(point) < 1e-3),
                    "{tool:?} missing {point:?}"
                );
            }
        }
        let mut sector = Drawing::new(0, DrawingToolType::Sector);
        for points in [
            vec![Pos2::ZERO; 3],
            vec![Pos2::ZERO, Pos2::new(100.0, 0.0), Pos2::ZERO],
        ] {
            sector.points = points;
            assert!(sector.outline(rect).is_none());
        }
    }

    #[test]
    fn arc_is_a_symmetric_dome_and_curve_bends_toward_its_third_anchor() {
        let rect = egui::Rect::from_min_max(egui::pos2(-400.0, -400.0), egui::pos2(400.0, 400.0));
        let mut arc = Drawing::new(0, DrawingToolType::Arc);
        arc.points = vec![
            Pos2::new(0.0, 0.0),
            Pos2::new(100.0, 0.0),
            Pos2::new(80.0, 40.0),
        ];
        let outline = arc.outline(rect).unwrap();
        assert!(
            outline
                .lines
                .iter()
                .flatten()
                .any(|p| p.distance(Pos2::new(50.0, 40.0)) < 1.0),
            "dome peak follows the apex height at the chord midpoint"
        );
        assert!(
            outline.areas.is_empty(),
            "an arc without a fill is its border"
        );
        arc.fill_color = Some([10, 20, 30, 40]);
        assert_eq!(arc.outline(rect).unwrap().areas.len(), 1);

        let mut curve = Drawing::new(0, DrawingToolType::Curve);
        curve.points = vec![
            Pos2::new(0.0, 0.0),
            Pos2::new(100.0, 0.0),
            Pos2::new(50.0, 80.0),
        ];
        let [start, control, end] = curve.curve_controls().unwrap();
        assert_eq!((start, control, end).2, Pos2::new(100.0, 0.0));
        assert_eq!(control, Pos2::new(50.0, 80.0));
        let outline = curve.outline(rect).unwrap();
        assert!(
            outline
                .lines
                .iter()
                .flatten()
                .any(|p| p.distance(Pos2::new(50.0, 40.0)) < 1.0),
            "the quadratic passes halfway toward its control"
        );
        curve.fill_color = Some([10, 20, 30, 40]);
        assert!(
            curve
                .outline(rect)
                .unwrap()
                .areas
                .iter()
                .any(|area| area.iter().any(|p| p.distance(Pos2::new(50.0, 40.0)) < 1.0))
        );
    }

    #[test]
    fn arrow_marks_point_away_from_the_anchor_in_the_line_colour() {
        let tip = Pos2::new(100.0, 100.0);
        for (tool, outward) in [
            (DrawingToolType::ArrowMarkUp, egui::vec2(0.0, 1.0)),
            (DrawingToolType::ArrowMarkDown, egui::vec2(0.0, -1.0)),
            (DrawingToolType::ArrowMarkLeft, egui::vec2(1.0, 0.0)),
            (DrawingToolType::ArrowMarkRight, egui::vec2(-1.0, 0.0)),
        ] {
            let mut drawing = Drawing::with_color(1, tool, [12, 80, 200, 255]);
            drawing.points = vec![tip];
            let glyph = drawing.arrow_mark_glyph().unwrap();
            assert_eq!(glyph[0], tip, "{tool:?}");
            assert!(
                glyph[1..].iter().all(|p| (*p - tip).dot(outward) >= -1e-3),
                "{tool:?} body is not on the outward side"
            );
            assert!(!tool.supports_fill(), "{tool:?} colour is its line colour");
        }
    }

    #[test]
    fn highlighter_keeps_a_chosen_alpha_and_a_wide_stroke() {
        let mut drawing = Drawing::with_color(1, DrawingToolType::Highlighter, [10, 20, 30, 40]);
        assert_eq!(drawing.brush_width(), Some(16.0));
        drawing.points = vec![Pos2::new(0.0, 0.0), Pos2::new(50.0, 0.0)];
        drawing.stroke_width = 3.0;
        assert_eq!(drawing.brush_width(), Some(24.0));
        let opaque = Drawing::with_color(1, DrawingToolType::Highlighter, [10, 20, 30, 255]);
        assert_eq!(drawing.highlighter_color().a(), 40);
        assert_eq!(opaque.highlighter_color().a(), 100);
        assert_eq!(
            opaque.highlighter_color(),
            Color32::from_rgba_unmultiplied(10, 20, 30, 100)
        );
    }

    #[test]
    fn thick_strokes_and_filled_shapes_are_hit_on_their_paint() {
        use crate::drawings::services::DrawingInteraction;
        let interaction = DrawingInteraction::new();
        let mut circle = Drawing::new(7, DrawingToolType::Circle);
        circle.points = vec![Pos2::ZERO, Pos2::new(10.0, 0.0)];
        circle.stroke_width = 20.0;
        assert_eq!(
            interaction.hit_test(Pos2::new(0.0, 18.0), std::slice::from_ref(&circle), "1D"),
            Some(7),
            "a thick ring includes its stroke"
        );
        assert_eq!(
            interaction.hit_test(Pos2::new(0.0, 40.0), std::slice::from_ref(&circle), "1D"),
            None
        );

        let mut rect = Drawing::new(7, DrawingToolType::Rect);
        rect.points = vec![Pos2::new(0.0, 0.0), Pos2::new(100.0, 80.0)];
        assert_eq!(
            interaction.hit_test(Pos2::new(50.0, 40.0), std::slice::from_ref(&rect), "1D"),
            None,
            "an unfilled rectangle is not its interior"
        );
        rect.fill_color = Some([1, 2, 3, 40]);
        assert_eq!(
            interaction.hit_test(Pos2::new(50.0, 40.0), std::slice::from_ref(&rect), "1D"),
            Some(7)
        );

        let mut sector = Drawing::new(7, DrawingToolType::Sector);
        sector.points = vec![
            Pos2::new(0.0, 0.0),
            Pos2::new(40.0, 0.0),
            Pos2::new(0.0, 40.0),
        ];
        sector.fill_color = Some([0, 0, 0, 0]);
        assert!(sector.painted_fill().is_none());
        assert!(
            sector
                .outline(egui::Rect::EVERYTHING)
                .unwrap()
                .areas
                .is_empty()
        );
    }
}
