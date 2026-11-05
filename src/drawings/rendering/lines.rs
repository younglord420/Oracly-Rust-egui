//! Basic drawing tool rendering implementations
//!
//! Includes: lines, rays, shapes (rect, circle, ellipse, triangle, arc),
//! arrows, and polylines.

use super::style::placed_anchor;
use crate::drawings::domain::{Drawing, DrawingToolType};
use crate::styles::{stroke, typography};
use crate::theme::ThemeExt;
use crate::tokens::DESIGN_TOKENS;
use egui::{Color32, Pos2, Rect, Stroke, epaint::StrokeKind};

impl Drawing {
    /// Draws the straight segment from `a` to `b` in the drawing's line style.
    pub(super) fn styled_segment(&self, painter: &egui::Painter, a: Pos2, b: Pos2, stroke: Stroke) {
        match self.line_style.dash_pattern() {
            None => {
                painter.line_segment([a, b], stroke);
            }
            Some((dash, gap)) => super::utils::draw_dashed_line(painter, a, b, stroke, dash, gap),
        }
    }

    /// Ends of the stroked segment; `None` below two anchors. A ray always runs to the
    /// right pane edge: from the leftmost of its two anchors along the line through both.
    /// A vertical ray runs from the first anchor through the second to the top or bottom.
    /// Other lines run left to right. A vertical extended line runs top to bottom.
    pub(crate) fn trend_line_ends(&self, rect: Rect) -> Option<(Pos2, Pos2)> {
        let &[a, b, ..] = self.points.as_slice() else {
            return None;
        };
        if self.tool_type == crate::drawings::DrawingToolType::Ray {
            return Some(ray_segment(a, b, rect));
        }
        if a.x == b.x && self.tool_type == crate::drawings::DrawingToolType::ExtendedLine {
            let (start, end) = if b.y < a.y { (b, a) } else { (a, b) };
            return Some((
                if self.extend_left {
                    ray_end(end, start, rect)
                } else {
                    start
                },
                if self.extend_right {
                    ray_end(start, end, rect)
                } else {
                    end
                },
            ));
        }
        let (mut left, mut right) = if a.x <= b.x { (a, b) } else { (b, a) };
        let slope = (right.y - left.y) / (right.x - left.x);
        if slope.is_finite() {
            if self.extend_left && rect.min.x < left.x {
                left = Pos2::new(rect.min.x, left.y + (rect.min.x - left.x) * slope);
            }
            if self.extend_right && rect.max.x > right.x {
                right = Pos2::new(rect.max.x, right.y + (rect.max.x - right.x) * slope);
            }
        }
        Some((left, right))
    }

    /// Render trend line - a line between two points, extended to the left or right edge of
    /// `rect` when the drawing asks for it
    pub(crate) fn render_trend_line(
        &self,
        painter: &egui::Painter,
        rect: Rect,
        color: Color32,
        stroke: Stroke,
    ) {
        if let Some((left, right)) = self.trend_line_ends(rect) {
            self.styled_segment(painter, left, right, stroke);
        } else if self.points.len() == 1 {
            placed_anchor(painter, self.points[0], color);
        }
    }

    /// Render a ray with its configured extensions.
    pub(crate) fn render_ray(
        &self,
        painter: &egui::Painter,
        rect: Rect,
        color: Color32,
        stroke: Stroke,
    ) {
        self.render_trend_line(painter, rect, color, stroke);
    }

    /// Render an extended line with its configured extensions.
    pub(crate) fn render_extended_line(
        &self,
        painter: &egui::Painter,
        rect: Rect,
        color: Color32,
        stroke: Stroke,
    ) {
        self.render_trend_line(painter, rect, color, stroke);
    }

    /// Render horizontal line
    pub(crate) fn render_horizontal_line(
        &self,
        painter: &egui::Painter,
        rect: Rect,
        stroke: Stroke,
    ) {
        if !self.points.is_empty() {
            let y = self.points[0].y;
            let (left, right) = (Pos2::new(rect.min.x, y), Pos2::new(rect.max.x, y));
            self.styled_segment(painter, left, right, stroke);
        }
    }

    /// Render horizontal ray - extends right from start point
    pub(crate) fn render_horizontal_ray(
        &self,
        painter: &egui::Painter,
        rect: Rect,
        _color: Color32,
        stroke: Stroke,
    ) {
        if !self.points.is_empty() {
            let start = self.points[0];
            self.styled_segment(painter, start, Pos2::new(rect.max.x, start.y), stroke);
        }
    }

    /// Render vertical line
    pub(crate) fn render_vertical_line(&self, painter: &egui::Painter, rect: Rect, stroke: Stroke) {
        if !self.points.is_empty() {
            let x = self.points[0].x;
            let (top, bottom) = (Pos2::new(x, rect.min.y), Pos2::new(x, rect.max.y));
            self.styled_segment(painter, top, bottom, stroke);
        }
    }

    /// Render cross line - horizontal and vertical lines intersecting
    pub(crate) fn render_cross_line(
        &self,
        painter: &egui::Painter,
        rect: Rect,
        _color: Color32,
        stroke: Stroke,
    ) {
        if !self.points.is_empty() {
            let point = self.points[0];
            let (left, right) = (
                Pos2::new(rect.min.x, point.y),
                Pos2::new(rect.max.x, point.y),
            );
            let (top, bottom) = (
                Pos2::new(point.x, rect.min.y),
                Pos2::new(point.x, rect.max.y),
            );
            self.styled_segment(painter, left, right, stroke);
            self.styled_segment(painter, top, bottom, stroke);
        }
    }

    /// Render a signed screen angle with plain text and dotted construction guides.
    pub(crate) fn render_trend_angle(
        &self,
        painter: &egui::Painter,
        rect: Rect,
        color: Color32,
        stroke: Stroke,
    ) {
        let &[start, end, ..] = self.points.as_slice() else {
            if let Some(&point) = self.points.first() {
                placed_anchor(painter, point, color);
            }
            return;
        };
        let first_shape = painter.ctx().graphics(|g| {
            g.get(painter.layer_id())
                .map_or(0, |list| list.next_idx().0)
        });
        if let Some((start, end)) = self.trend_line_ends(rect) {
            painter.line_segment([start, end], stroke);
        }

        // Screen y grows downward, so a rising line has a positive angle.
        let angle = (start.y - end.y).atan2(end.x - start.x);
        let radius = DESIGN_TOKENS
            .sizing
            .technical_labels
            .line_arc_radius
            .min(start.distance(end));
        let label_direction = egui::vec2((angle / 2.0).cos(), -(angle / 2.0).sin());
        let galley = painter.layout_no_wrap(
            format!("{:.1}°", angle.to_degrees()),
            self.annotation_font(),
            color,
        );
        let label_distance =
            radius + DESIGN_TOKENS.spacing.sm + label_direction.abs().dot(galley.size()) / 2.0;
        painter.galley(
            start + label_direction * label_distance - galley.size() / 2.0,
            galley,
            color,
        );
        self.finish_style(painter, first_shape, rect);

        // Construction guides stay thin and dotted regardless of the main line's style.
        let (dash, gap) = crate::model::LineStyle::Dotted.hairline_pattern().unwrap();
        let guide_stroke = Stroke::new(stroke::HAIRLINE, color);
        let horizontal = [start + egui::vec2(radius, 0.0), start];
        // Anchor the dash phase at the leg so even a short arc touches it.
        let arc: Vec<_> = (0..=32)
            .map(|i| {
                let theta = -angle * (1.0 - i as f32 / 32.0);
                start + radius * egui::vec2(theta.cos(), theta.sin())
            })
            .collect();
        for path in [&horizontal[..], &arc] {
            crate::painting::dashed_line::draw_dashed_polyline(
                painter,
                path,
                guide_stroke,
                dash,
                gap,
            );
        }
    }

    /// Rectangle border and fill bounds, including its configured extensions.
    pub(crate) fn rectangle_bounds(&self, pane: Rect) -> Option<Rect> {
        let &[a, b, ..] = self.points.as_slice() else {
            return None;
        };
        let mut rect = Rect::from_two_pos(a, b);
        if self.extend_left {
            rect.min.x = rect.min.x.min(pane.min.x);
        }
        if self.extend_right {
            rect.max.x = rect.max.x.max(pane.max.x);
        }
        Some(rect)
    }

    /// Render rect: its border and fill.
    pub(crate) fn render_rect(
        &self,
        painter: &egui::Painter,
        pane: Rect,
        color: Color32,
        stroke: Stroke,
    ) {
        if let Some(rect) = self.rectangle_bounds(pane) {
            let snap = crate::painting::pixel_snap::PixelSnap::of(painter);
            let rect = Rect::from_min_max(
                Pos2::new(snap.edge(rect.min.x), snap.edge(rect.min.y)),
                Pos2::new(snap.edge(rect.max.x), snap.edge(rect.max.y)),
            );
            if let Some(fill) = self.painted_fill() {
                painter.rect_filled(rect, 0.0, fill);
            }

            // Main rectangle stroke
            painter.rect_stroke(rect, 0.0, stroke, StrokeKind::Inside);
        } else if self.points.len() == 1 {
            placed_anchor(painter, self.points[0], color);
        }
    }

    /// Draw the optional middle line after common border styling, so it stays thin and dashed.
    pub(super) fn render_rect_middle_line(&self, painter: &egui::Painter, pane: Rect) {
        if self.channel_midline
            && let Some(rect) = self.rectangle_bounds(pane)
        {
            super::utils::draw_dashed_line(
                painter,
                Pos2::new(rect.min.x, rect.center().y),
                Pos2::new(rect.max.x, rect.center().y),
                Stroke::new(stroke::HAIRLINE, self.color32()),
                DESIGN_TOKENS.rounding.sm,
                DESIGN_TOKENS.rounding.xs,
            );
        }
    }

    /// Render the centre guides and the size labels of a rectangle that is selected or being
    /// drawn. The optional persistent middle line is rendered separately.
    pub(crate) fn render_rect_guides(&self, painter: &egui::Painter) {
        if self.points.len() < 2 {
            return;
        }
        let rect = Rect::from_two_pos(self.points[0], self.points[1]);
        // Draw middle horizontal line (50% level)
        let middle_y = (rect.min.y + rect.max.y) / 2.0;
        let middle_color =
            Color32::from_rgba_unmultiplied(self.color[0], self.color[1], self.color[2], 100);
        super::utils::draw_dashed_line(
            painter,
            Pos2::new(rect.min.x, middle_y),
            Pos2::new(rect.max.x, middle_y),
            Stroke::new(stroke::HAIRLINE, middle_color),
            DESIGN_TOKENS.rounding.sm,
            DESIGN_TOKENS.rounding.xs,
        );

        // Draw middle vertical line
        let middle_x = (rect.min.x + rect.max.x) / 2.0;
        super::utils::draw_dashed_line(
            painter,
            Pos2::new(middle_x, rect.min.y),
            Pos2::new(middle_x, rect.max.y),
            Stroke::new(stroke::HAIRLINE, middle_color),
            DESIGN_TOKENS.rounding.sm,
            DESIGN_TOKENS.rounding.xs,
        );

        // Width in bars below, height in price on the right
        let height = rect.height();
        let width = rect.width();
        let [width_label, height_label] =
            <[String; 2]>::try_from(self.size_labels()).unwrap_or_default();
        let chart = painter.ctx().active_theme().chart;
        let label_bg = chart.bg_axis;

        // Width label (bottom center)
        if width > 50.0 {
            let width_label_pos = Pos2::new(rect.center().x, rect.max.y + typography::XS);
            self.label_box_on(
                painter,
                egui::Align2::CENTER_CENTER,
                width_label_pos,
                width_label,
                label_bg,
            );
        }

        // Height label (right center)
        if height > 50.0 {
            self.label_box_on(
                painter,
                egui::Align2::LEFT_CENTER,
                Pos2::new(rect.max.x + typography::XS, rect.center().y),
                height_label,
                label_bg,
            );
        }
    }

    /// Render circle
    pub(crate) fn render_circle(&self, painter: &egui::Painter, color: Color32, stroke: Stroke) {
        if self.points.len() >= 2 {
            let center = self.points[0];
            let edge = self.points[1];
            let radius = ((edge.x - center.x).powi(2) + (edge.y - center.y).powi(2)).sqrt();
            if let Some(fill) = self.painted_fill() {
                painter.circle_filled(center, radius, fill);
            }
            painter.circle_stroke(center, radius, stroke);
        } else if self.points.len() == 1 {
            placed_anchor(painter, self.points[0], color);
        }
    }

    /// Render ellipse
    pub(crate) fn render_ellipse(&self, painter: &egui::Painter, color: Color32, stroke: Stroke) {
        if self.points.len() >= 2 {
            let ellipse_points = ellipse_points(self.points[0], self.points[1]);

            if let Some(fill) = self.painted_fill() {
                painter.add(egui::Shape::convex_polygon(
                    ellipse_points.clone(),
                    fill,
                    Stroke::NONE,
                ));
            }

            for i in 1..ellipse_points.len() {
                painter.line_segment([ellipse_points[i - 1], ellipse_points[i]], stroke);
            }
        } else if self.points.len() == 1 {
            placed_anchor(painter, self.points[0], color);
        }
    }

    /// Render triangle
    pub(crate) fn render_triangle(&self, painter: &egui::Painter, color: Color32, stroke: Stroke) {
        if self.points.len() >= 3 {
            let p1 = self.points[0];
            let p2 = self.points[1];
            let p3 = self.points[2];
            if let Some(fill) = self.painted_fill() {
                painter.add(egui::Shape::convex_polygon(
                    vec![p1, p2, p3],
                    fill,
                    Stroke::NONE,
                ));
            }
            painter.line_segment([p1, p2], stroke);
            painter.line_segment([p2, p3], stroke);
            painter.line_segment([p3, p1], stroke);
        } else {
            for (i, &p) in self.points.iter().enumerate() {
                placed_anchor(painter, p, color);
                if i > 0 {
                    painter.line_segment([self.points[i - 1], p], stroke);
                }
            }
        }
    }

    /// Half-ellipse dome over the chord from the first anchor to the second. The third
    /// anchor's height sets the bulge; its horizontal position stays on the chord midpoint.
    pub(crate) fn render_arc(&self, painter: &egui::Painter, color: Color32, stroke: Stroke) {
        let path = self.arc_path();
        if path.len() >= 2 {
            if path.len() >= 4
                && let Some(fill) = self.painted_fill()
            {
                let mut ring = path.clone();
                ring.pop();
                painter.add(egui::Shape::convex_polygon(ring, fill, Stroke::NONE));
            }
            painter.add(egui::Shape::line(path, stroke));
        } else if self.points.len() == 1 {
            placed_anchor(painter, self.points[0], color);
        }
    }

    /// The dome from the second anchor back to the first, closed along the chord.
    /// Two anchors are the chord alone.
    pub(super) fn arc_path(&self) -> Vec<Pos2> {
        let &[a, b, ..] = self.points.as_slice() else {
            return Vec::new();
        };
        let Some(&apex) = self.points.get(2) else {
            return vec![a, b];
        };
        let mut dome = arc_dome(a, b, apex);
        if let Some(&first) = dome.first()
            && dome.len() >= 2
            && dome.last() != Some(&first)
        {
            dome.push(first);
        }
        dome
    }

    /// Render polyline - connected line segments through all points
    pub(crate) fn render_polyline(&self, painter: &egui::Painter, color: Color32, stroke: Stroke) {
        if let [point] = self.points[..] {
            placed_anchor(painter, point, color);
        }
        for pair in self.points.windows(2) {
            painter.line_segment([pair[0], pair[1]], stroke);
        }
    }

    /// Render arrow - line with arrowhead
    pub(crate) fn render_arrow(
        &self,
        painter: &egui::Painter,
        rect: Rect,
        color: Color32,
        stroke: Stroke,
    ) {
        if self.points.len() >= 2 {
            if let Some((start, end)) = self.trend_line_ends(rect) {
                painter.line_segment([start, end], stroke);
            }
        } else if self.points.len() == 1 {
            placed_anchor(painter, self.points[0], color);
        }
    }

    /// Arrow marker from its tail anchor to its head anchor
    pub(crate) fn render_arrow_marker(&self, painter: &egui::Painter, rect: Rect, color: Color32) {
        self.render_arrow(painter, rect, color, Stroke::new(self.stroke_width, color));
    }

    /// Fixed-size directional mark: the tip sits on the anchor and the body extends away from it.
    pub(crate) fn render_arrow_mark(&self, painter: &egui::Painter) {
        let Some(mut points) = self.arrow_mark_glyph() else {
            return;
        };
        if let Some(&tip) = self.points.first() {
            let snap = crate::painting::pixel_snap::PixelSnap::of(painter);
            let snapped = Pos2::new(snap.edge(tip.x), snap.edge(tip.y));
            let delta = snapped - tip;
            if delta != egui::Vec2::ZERO {
                for point in &mut points {
                    *point += delta;
                }
            }
        }
        painter.add(egui::Shape::convex_polygon(
            points,
            self.color32(),
            Stroke::NONE,
        ));
    }

    /// The mark's polygon in screen space, tip first. `None` for other tools and with no anchor.
    /// The glyph grows with the stroke width; at the default width of 2 it is the base size.
    pub(super) fn arrow_mark_glyph(&self) -> Option<Vec<Pos2>> {
        let &tip = self.points.first()?;
        let scale = (self.stroke_width / 2.0).max(0.25);
        let (head, head_w, stem_w, len) = (11.0 * scale, 9.0 * scale, 3.5 * scale, 22.0 * scale);
        // Base glyph points up in screen space: tip at the origin, body toward +y.
        let base = [
            [0.0, 0.0],
            [-head_w, head],
            [-stem_w, head],
            [-stem_w, len],
            [stem_w, len],
            [stem_w, head],
            [head_w, head],
        ];
        let (sign_x, sign_y, swap) = match self.tool_type {
            DrawingToolType::ArrowMarkUp => (1.0, 1.0, false),
            DrawingToolType::ArrowMarkDown => (1.0, -1.0, false),
            DrawingToolType::ArrowMarkLeft => (1.0, 1.0, true),
            DrawingToolType::ArrowMarkRight => (-1.0, 1.0, true),
            _ => return None,
        };
        Some(
            base.into_iter()
                .map(|[x, y]| {
                    let (rx, ry) = if swap {
                        (sign_x * y, sign_y * x)
                    } else {
                        (sign_x * x, sign_y * y)
                    };
                    tip + egui::vec2(rx, ry)
                })
                .collect(),
        )
    }
}

/// Where a directed ray from `start` through `end` leaves `rect`: on the side
/// `end` is towards, or at the top or bottom when it is vertical. Keep the defining
/// segment when its endpoint is already beyond that edge, so panning never reverses it.
pub(super) fn ray_end(start: Pos2, end: Pos2, rect: Rect) -> Pos2 {
    let (dx, dy) = (end.x - start.x, end.y - start.y);
    if dx.abs() > 1e-6 {
        let edge = if dx > 0.0 { rect.max.x } else { rect.min.x };
        let t = ((edge - start.x) / dx).max(1.0);
        start + (end - start) * t
    } else if dy.abs() > 1e-6 {
        let edge = if dy > 0.0 { rect.max.y } else { rect.min.y };
        start + (end - start) * ((edge - start.y) / dy).max(1.0)
    } else {
        start
    }
}

/// Stroke endpoints for the Ray tool: always extend to the right pane edge.
/// Non-vertical: from `min(x1, x2)` along the line through both anchors to `rect.right()`.
/// Vertical: from the first anchor through the second to the top or bottom edge.
fn ray_segment(p1: Pos2, p2: Pos2, rect: Rect) -> (Pos2, Pos2) {
    let dx = p2.x - p1.x;
    if dx.abs() < 1e-6 {
        let end_y = if p2.y >= p1.y { rect.max.y } else { rect.min.y };
        (Pos2::new(p1.x, p1.y), Pos2::new(p2.x, end_y))
    } else {
        let slope = (p2.y - p1.y) / dx;
        let y_at = |x: f32| p1.y + slope * (x - p1.x);
        let lx = p1.x.min(p2.x);
        let rx = rect.max.x;
        (Pos2::new(lx, y_at(lx)), Pos2::new(rx, y_at(rx)))
    }
}

/// The line through `p1` and `p2` across `rect`, edge to edge.
pub(super) fn extended_line(p1: Pos2, p2: Pos2, rect: Rect) -> [Pos2; 2] {
    let (dx, dy) = (p2.x - p1.x, p2.y - p1.y);
    if dx.abs() > 1e-6 {
        [rect.min.x, rect.max.x].map(|x| Pos2::new(x, p1.y + (x - p1.x) / dx * dy))
    } else {
        [rect.min.y, rect.max.y].map(|y| Pos2::new(p1.x, y))
    }
}

/// The closed outline of the ellipse inscribed in the box from `p1` to `p2`.
pub(super) fn ellipse_points(p1: Pos2, p2: Pos2) -> Vec<Pos2> {
    let center = Pos2::new((p1.x + p2.x) / 2.0, (p1.y + p2.y) / 2.0);
    let (rx, ry) = ((p2.x - p1.x).abs() / 2.0, (p2.y - p1.y).abs() / 2.0);
    let segments = 48;
    (0..=segments)
        .map(|i| {
            let angle = (i as f32 / segments as f32) * std::f32::consts::TAU;
            Pos2::new(center.x + rx * angle.cos(), center.y + ry * angle.sin())
        })
        .collect()
}

/// Half-ellipse from `b` to `a` bulging by the height of `apex` off the chord.
/// The apex's screen x is ignored: the dome stays symmetric about the chord midpoint.
fn arc_dome(a: Pos2, b: Pos2, apex: Pos2) -> Vec<Pos2> {
    let (mx, my) = ((a.x + b.x) * 0.5, (a.y + b.y) * 0.5);
    let (hx, hy) = ((b.x - a.x) * 0.5, (b.y - a.y) * 0.5);
    let len = hx.hypot(hy);
    if len < 1e-6 {
        return vec![a, b];
    }
    let (mut nx, mut ny) = (-hy / len, hx / len);
    let mut off = (apex.y - my) * ny;
    if off < 0.0 {
        nx = -nx;
        ny = -ny;
        off = -off;
    }
    let samples = 40;
    (0..=samples)
        .map(|i| {
            let t = i as f32 / samples as f32 * std::f32::consts::PI;
            let (c, s) = (t.cos(), t.sin());
            Pos2::new(mx + c * hx + s * off * nx, my + c * hy + s * off * ny)
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::drawings::rendering::ChartView;

    /// Screen y points down: equal rise and run give 45 degrees, and a 3-4-5
    /// triangle gives atan(3/4) = 36.9 degrees. Guides share a radius capped by the leg.
    #[test]
    fn trend_angle_has_signed_plain_text_and_dotted_guides() {
        use crate::{drawings::DrawingToolType, model::LineStyle};
        let start = Pos2::new(250., 250.);
        let pane = Rect::from_min_max(Pos2::ZERO, Pos2::new(600., 600.));
        for (delta, label) in [
            (egui::vec2(100., -100.), "45.0°"),
            (egui::vec2(100., 100.), "-45.0°"),
            (egui::vec2(0., -100.), "90.0°"),
            (egui::vec2(0., 100.), "-90.0°"),
            (egui::vec2(-100., -100.), "135.0°"),
            (egui::vec2(-100., 100.), "-135.0°"),
            (egui::vec2(100., 0.), "0.0°"),
            (egui::vec2(16., -12.), "36.9°"),
        ] {
            for (size, width, style) in [(12., 2., LineStyle::Solid), (24., 5., LineStyle::Dashed)]
            {
                let mut drawing = Drawing::new(1, DrawingToolType::TrendAngle);
                drawing.points = vec![start, start + delta];
                drawing.chart_points = drawing
                    .points
                    .iter()
                    .map(|p| crate::drawings::ChartPoint::new(p.x, 600.0 - f64::from(p.y)))
                    .collect();
                drawing.font_size = size;
                drawing.stroke_width = width;
                drawing.line_style = style;
                drawing.color = [40, 120, 200, 255];
                let mut output = egui::Context::default().run_ui(Default::default(), |ui| {
                    let view = ChartView {
                        instrument: None,
                        bars: &[],
                        executions: None,
                        revision: 0,
                        bar_to_x: &|x| x,
                        price_to_y: &|p| p as f32,
                        y_to_price: &f64::from,
                        time_axis: true,
                    };
                    drawing.render(ui.painter(), pane, &view);
                });
                output.textures_delta.clear();
                let texts: Vec<_> = output
                    .shapes
                    .iter()
                    .filter_map(|s| match &s.shape {
                        egui::Shape::Text(t) => Some(t),
                        _ => None,
                    })
                    .collect();
                assert_eq!(texts.len(), 1, "only the angle is labelled");
                assert_eq!(texts[0].galley.job.text, label);
                let format = &texts[0].galley.job.sections[0].format;
                assert_eq!(format.font_id.size, size);
                assert_eq!(format.color, drawing.color32());
                assert!(
                    !output
                        .shapes
                        .iter()
                        .any(|s| matches!(s.shape, egui::Shape::Rect(_) | egui::Shape::Circle(_)))
                );
                let guides: Vec<_> = output
                    .shapes
                    .iter()
                    .filter_map(|s| match s.shape {
                        egui::Shape::LineSegment { points, stroke } if stroke.width == 1. => {
                            Some(points)
                        }
                        _ => None,
                    })
                    .collect();
                assert_eq!(guides.is_empty(), delta.y == 0.0);
                let radius = 40_f32.min(delta.length());
                for points in &guides {
                    assert!(points[0].distance(points[1]) <= 2.01, "dotted guides");
                    for p in points {
                        assert!(p.distance(start) <= radius + 0.01);
                        assert!(
                            (p.y - start.y).abs() < 0.01
                                || (p.distance(start) - radius).abs() < 0.1
                        );
                    }
                }
                let on_line = start + delta.normalized() * radius;
                if delta.y != 0.0 {
                    assert!(
                        guides.iter().flatten().any(|p| p.distance(on_line) < 0.01),
                        "arc reaches the leg"
                    );
                }
                let horizontal = output
                    .shapes
                    .iter()
                    .find_map(|s| match &s.shape {
                        egui::Shape::Mesh(m) => Some(m),
                        _ => None,
                    })
                    .expect("horizontal uses visible square dots");
                assert_eq!(horizontal.calc_bounds().right(), start.x + radius);
                assert!(
                    horizontal
                        .vertices
                        .iter()
                        .all(|v| v.color == drawing.color32()
                            && v.pos.x >= start.x
                            && v.pos.x <= start.x + radius
                            && (v.pos.y - start.y).abs() <= 1.0)
                );
                output.drop_without_applying_deltas();
            }
        }
    }

    #[test]
    fn rectangle_size_labels_use_the_active_theme_pair() {
        use crate::{
            drawings::DrawingToolType,
            theme::{apply_to_egui, presets},
        };
        let ctx = egui::Context::default();
        let mut drawing = Drawing::new(1, DrawingToolType::Rect);
        drawing.add_point(Pos2::new(100.0, 100.0));
        drawing.add_point(Pos2::new(300.0, 300.0));
        for (_, theme) in presets() {
            apply_to_egui(&ctx, &theme);
            let output = ctx.run_ui(Default::default(), |ui| {
                drawing.render_rect_guides(ui.painter());
            });
            let labels: Vec<_> = output
                .shapes
                .iter()
                .filter_map(|shape| match &shape.shape {
                    egui::Shape::Text(text) => Some(text.galley.job.sections[0].format.color),
                    _ => None,
                })
                .collect();
            let fills = output.shapes.iter().filter(|shape| matches!(&shape.shape, egui::Shape::Rect(rect) if rect.fill == theme.chart.bg_axis)).count();
            output.drop_without_applying_deltas();
            let readable =
                crate::chart::rendering::axes::tag_text_color(theme.chart.bg_axis.to_opaque());
            assert_eq!(labels, vec![readable; 2]);
            assert_eq!(fills, 2);
        }
        drawing.text_color = Some([20, 30, 40, 255]);
        let output = ctx.run_ui(Default::default(), |ui| {
            drawing.render_rect_guides(ui.painter());
        });
        let custom = Color32::from_rgba_unmultiplied(20, 30, 40, 255);
        assert!(output.shapes.iter().any(|shape| matches!(&shape.shape, egui::Shape::Text(text) if text.galley.job.sections[0].format.color == custom)));
        output.drop_without_applying_deltas();
    }

    /// Dragging the second anchor left of the first still strokes from the leftmost
    /// anchor through both points to the right pane edge — never leftward.
    #[test]
    fn a_ray_always_extends_to_the_right_edge() {
        use crate::drawings::DrawingToolType;
        let rect = Rect::from_min_max(Pos2::ZERO, Pos2::new(1000.0, 600.0));
        let mut drawing = Drawing::new(1, DrawingToolType::Ray);
        // First click at x=300, drag left to x=100: left end follows the cursor.
        drawing.points = vec![Pos2::new(300.0, 100.0), Pos2::new(100.0, 180.0)];
        let (left, right) = drawing.trend_line_ends(rect).unwrap();
        assert_eq!(left, Pos2::new(100.0, 180.0));
        assert!(
            (right.x - 1000.0).abs() < 0.01 && (right.y - (-180.0)).abs() < 0.01,
            "{right:?}"
        );
        // Drag right of the first anchor: left end stays at the first anchor.
        drawing.points[1] = Pos2::new(420.0, 40.0);
        let (left, right) = drawing.trend_line_ends(rect).unwrap();
        assert_eq!(left, Pos2::new(300.0, 100.0));
        assert!(
            (right.x - 1000.0).abs() < 0.01 && (right.y - (-250.0)).abs() < 0.01,
            "{right:?}"
        );
        // Extension flags do not change Ray geometry.
        drawing.points = vec![Pos2::new(200.0, 80.0), Pos2::new(800.0, 200.0)];
        drawing.extend_left = true;
        drawing.extend_right = false;
        let (origin, far) = drawing.trend_line_ends(rect).unwrap();
        assert_eq!(origin, Pos2::new(200.0, 80.0));
        assert!(
            (far.x - 1000.0).abs() < 0.01 && (far.y - 240.0).abs() < 0.01,
            "{far:?}"
        );
    }

    /// Ray stroke geometry matches right-edge extension: leftmost of the two anchors to
    /// the right pane edge. Vertical rays still run from the first anchor through the second.
    #[test]
    fn rays_extend_from_leftmost_anchor_to_the_right_edge() {
        let rect = Rect::from_min_max(Pos2::ZERO, Pos2::new(1000.0, 600.0));
        let start = Pos2::new(500.0, 100.0);
        let cases = [
            // through right: left = start, right edge at y=350
            (
                Pos2::new(600.0, 150.0),
                Pos2::new(500.0, 100.0),
                Pos2::new(1000.0, 350.0),
            ),
            // through left: left = second anchor, still to the right edge
            (
                Pos2::new(400.0, 150.0),
                Pos2::new(400.0, 150.0),
                Pos2::new(1000.0, -150.0),
            ),
            // vertical down / up
            (
                Pos2::new(500.0, 300.0),
                Pos2::new(500.0, 100.0),
                Pos2::new(500.0, 600.0),
            ),
            (
                Pos2::new(500.0, 50.0),
                Pos2::new(500.0, 100.0),
                Pos2::new(500.0, 0.0),
            ),
        ];
        let wrong: Vec<String> = cases
            .into_iter()
            .filter_map(|(through, want_l, want_r)| {
                let got = ray_segment(start, through, rect);
                (got != (want_l, want_r)).then(|| {
                    format!("through {through:?}: got {got:?}, want ({want_l:?}, {want_r:?})")
                })
            })
            .collect();
        assert!(wrong.is_empty(), "{}", wrong.join("\n"));

        // Directed `ray_end` (pitchfork / fib helpers) still reaches whichever edge it faces.
        assert_eq!(
            ray_end(start, Pos2::new(400.0, 150.0), rect),
            Pos2::new(0.0, 350.0)
        );

        use crate::drawings::{ArrowStyle, DrawingToolType};
        for tool in [DrawingToolType::Ray, DrawingToolType::ExtendedLine] {
            let defaults = Drawing::new(1, tool);
            assert!(defaults.extend_right);
            assert_eq!(defaults.extend_left, tool == DrawingToolType::ExtendedLine);
            if tool == DrawingToolType::Ray {
                for (left, right) in [(false, false), (false, true), (true, false), (true, true)] {
                    let mut vertical = defaults.clone();
                    vertical.points = vec![start, Pos2::new(500., 300.)];
                    vertical.extend_left = left;
                    vertical.extend_right = right;
                    assert_eq!(
                        vertical.outline(rect).unwrap().lines,
                        [[Pos2::new(500., 100.), Pos2::new(500., 600.)]]
                    );
                }
                for reversed in [false, true] {
                    let mut drawing = defaults.clone();
                    drawing.points = vec![start, Pos2::new(600., 150.)];
                    if reversed {
                        drawing.points.reverse();
                    }
                    drawing.extend_left = true;
                    drawing.extend_right = false;
                    drawing.arrow_start_style = ArrowStyle::Square;
                    drawing.arrow_end_style = ArrowStyle::Square;
                    // Both orders stroke from the leftmost anchor to the right edge.
                    let expected = [Pos2::new(500., 100.), Pos2::new(1000., 350.)];
                    assert_eq!(drawing.outline(rect).unwrap().lines, [expected]);
                    let interaction = crate::drawings::services::DrawingInteraction::new();
                    for (point, hit) in [
                        (Pos2::new(400., 50.), false),
                        (Pos2::new(550., 125.), true),
                        (Pos2::new(800., 250.), true),
                    ] {
                        assert_eq!(
                            interaction.hit_test(point, std::slice::from_ref(&drawing), "1D"),
                            hit.then_some(1)
                        );
                    }
                    let output = egui::Context::default().run_ui(Default::default(), |ui| {
                        let view = ChartView {
                            instrument: None,
                            bars: &[],
                            executions: None,
                            revision: 0,
                            bar_to_x: &|x| x,
                            price_to_y: &|p| p as f32,
                            y_to_price: &f64::from,
                            time_axis: true,
                        };
                        drawing.render(ui.painter(), rect, &view);
                    });
                    assert!(output.shapes.iter().any(|s| matches!(&s.shape,
                        egui::Shape::LineSegment { points, .. } if *points == expected)));
                    for tip in expected {
                        assert!(output.shapes.iter().any(|s| matches!(&s.shape,
                            egui::Shape::Rect(r) if r.rect.center() == tip)));
                    }
                    output.drop_without_applying_deltas();
                }
                continue;
            }
            for (left, right, ys) in [
                (false, false, [100., 300.]),
                (false, true, [100., 600.]),
                (true, false, [0., 300.]),
                (true, true, [0., 600.]),
            ] {
                let mut vertical = defaults.clone();
                vertical.points = vec![start, Pos2::new(500., 300.)];
                vertical.extend_left = left;
                vertical.extend_right = right;
                assert_eq!(
                    vertical.outline(rect).unwrap().lines,
                    [ys.map(|y| Pos2::new(500., y))]
                );
            }
            type Case = (bool, bool, [(f32, f32); 2], [(f32, f32); 2]);
            let cases: &[Case] = &[
                (
                    false,
                    false,
                    [(500., 100.), (600., 150.)],
                    [(500., 100.), (600., 150.)],
                ),
                (
                    false,
                    true,
                    [(500., 100.), (1000., 350.)],
                    [(500., 100.), (1000., 350.)],
                ),
                (
                    true,
                    false,
                    [(0., -150.), (600., 150.)],
                    [(0., -150.), (600., 150.)],
                ),
                (
                    true,
                    true,
                    [(0., -150.), (1000., 350.)],
                    [(0., -150.), (1000., 350.)],
                ),
            ];
            for &(left, right, forward, backward) in cases {
                for (reversed, ends) in [(false, forward), (true, backward)] {
                    let mut drawing = defaults.clone();
                    drawing.points = vec![start, Pos2::new(600., 150.)];
                    if reversed {
                        drawing.points.reverse();
                    }
                    drawing.extend_left = left;
                    drawing.extend_right = right;
                    drawing.arrow_start_style = ArrowStyle::Square;
                    drawing.arrow_end_style = ArrowStyle::Square;
                    let expected = ends.map(|(x, y)| Pos2::new(x, y));
                    assert_eq!(drawing.outline(rect).unwrap().lines, [expected]);
                    let interaction = crate::drawings::services::DrawingInteraction::new();
                    let (before, after) = (left, right);
                    for (point, hit) in [
                        (Pos2::new(400., 50.), before),
                        (Pos2::new(550., 125.), true),
                        (Pos2::new(800., 250.), after),
                    ] {
                        assert_eq!(
                            interaction.hit_test(point, std::slice::from_ref(&drawing), "1D"),
                            hit.then_some(1)
                        );
                    }
                    let output = egui::Context::default().run_ui(Default::default(), |ui| {
                        let view = ChartView {
                            instrument: None,
                            bars: &[],
                            executions: None,
                            revision: 0,
                            bar_to_x: &|x| x,
                            price_to_y: &|p| p as f32,
                            y_to_price: &f64::from,
                            time_axis: true,
                        };
                        drawing.render(ui.painter(), rect, &view);
                    });
                    assert!(output.shapes.iter().any(|s| matches!(&s.shape,
                        egui::Shape::LineSegment { points, .. } if *points == expected)));
                    for tip in expected {
                        assert!(output.shapes.iter().any(|s| matches!(&s.shape,
                            egui::Shape::Rect(r) if r.rect.center() == tip)));
                    }
                    output.drop_without_applying_deltas();
                }
            }
        }
    }

    /// Extended zones paint and select the same box; the optional middle line stays thin and
    /// dashed even with a wide border, and does not require selection.
    #[test]
    fn rectangle_extensions_and_middle_line_follow_the_painted_zone() {
        use crate::drawings::{DrawingToolType, services::DrawingInteraction};
        let pane = Rect::from_min_max(Pos2::ZERO, Pos2::new(500., 400.));
        for (left, right, min, max) in [
            (false, false, 100., 300.),
            (true, false, 0., 300.),
            (false, true, 100., 500.),
            (true, true, 0., 500.),
        ] {
            for middle in [false, true] {
                let mut drawing = Drawing::new(1, DrawingToolType::Rect);
                assert!(!drawing.channel_midline);
                drawing.points = vec![Pos2::new(300., 300.), Pos2::new(100., 100.)];
                drawing.extend_left = left;
                drawing.extend_right = right;
                drawing.channel_midline = middle;
                drawing.stroke_width = 5.;
                drawing.fill_color = Some([10, 20, 30, 40]);
                let expected = Rect::from_min_max(Pos2::new(min, 100.), Pos2::new(max, 300.));
                let outline = drawing.outline(pane).expect("rectangle outline");
                assert_eq!(Rect::from_points(&outline.areas[0]), expected);
                let interaction = DrawingInteraction::new();
                for (x, hit) in [(50., left), (200., true), (450., right)] {
                    assert_eq!(
                        interaction.hit_test(
                            Pos2::new(x, 150.),
                            std::slice::from_ref(&drawing),
                            "1D"
                        ),
                        hit.then_some(1)
                    );
                }
                let output = egui::Context::default().run_ui(Default::default(), |ui| {
                    let view = ChartView {
                        instrument: None,
                        bars: &[],
                        executions: None,
                        revision: 0,
                        bar_to_x: &|x| x,
                        price_to_y: &|p| p as f32,
                        y_to_price: &f64::from,
                        time_axis: true,
                    };
                    drawing.render(ui.painter(), pane, &view);
                });
                assert!(output.shapes.iter().any(|s| matches!(&s.shape,
                    egui::Shape::Rect(r) if r.rect == expected && r.fill == drawing.fill_color32().unwrap())));
                let dashes: Vec<_> = output
                    .shapes
                    .iter()
                    .filter_map(|s| match s.shape {
                        egui::Shape::LineSegment { points, stroke }
                            if points[0].y == 200. && points[1].y == 200. =>
                        {
                            Some((points, stroke))
                        }
                        _ => None,
                    })
                    .collect();
                assert_eq!(!dashes.is_empty(), middle);
                if middle {
                    assert_eq!(dashes.first().unwrap().0[0].x, min);
                    assert!(dashes.last().unwrap().0[1].x > max - 10.);
                    assert!(dashes.iter().all(|(p, s)| p[0].distance(p[1]) <= 8.
                        && s.width == stroke::HAIRLINE
                        && s.color == drawing.color32()));
                }
                output.drop_without_applying_deltas();
            }
        }
    }

    /// Size labels and centre guides belong to a selected rectangle; a deselected one has
    /// only its border and fill.
    #[test]
    fn rectangle_guides_show_only_while_selected() {
        use crate::drawings::{DrawingManager, DrawingToolType};
        let mut manager = DrawingManager::new();
        let mut drawing = Drawing::new(1, DrawingToolType::Rect);
        drawing.add_point(Pos2::new(100.0, 100.0));
        drawing.add_point(Pos2::new(300.0, 300.0));
        manager.drawings.push(drawing);
        let pane = Rect::from_min_max(Pos2::ZERO, Pos2::new(500.0, 400.0));
        let texts = |manager: &DrawingManager| {
            let ctx = egui::Context::default();
            let output = ctx.run_ui(Default::default(), |ui| {
                let view = ChartView {
                    instrument: None,
                    bars: &[],
                    executions: None,
                    revision: 0,
                    bar_to_x: &|x| x,
                    price_to_y: &|p| p as f32,
                    y_to_price: &f64::from,
                    time_axis: true,
                };
                manager.render_all(ui.painter(), pane, &view);
            });
            let texts = (output.shapes.iter())
                .filter(|shape| matches!(shape.shape, egui::Shape::Text(_)))
                .count();
            output.drop_without_applying_deltas();
            texts
        };
        assert_eq!(texts(&manager), 0);
        manager.select(1);
        assert_eq!(texts(&manager), 2);
    }
}
