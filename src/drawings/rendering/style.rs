//! Common drawing styles applied to renderer primitives.

use crate::drawings::{
    ArrowStyle, Drawing, DrawingToolType, FontWeight, TextHorizontalAlign, TextVerticalAlign,
};
use crate::model::LineStyle;
use egui::{
    Color32, Painter, Pos2, Rect, Shape, Stroke,
    epaint::{ColorMode, TextShape},
    layers::ShapeIdx,
};

/// Placed label text plus optional line-cut geometry (anchor, unit direction, t0, t1).
type LineLabelLayout = (TextShape, Option<(Pos2, egui::Vec2, f32, f32)>);

/// Radius of the dot marking an anchor placed before the drawing has enough points to paint,
/// no larger than a selection handle. Complete drawings mark no anchors; handles do.
pub(super) const PLACED_ANCHOR_RADIUS: f32 = 3.0;

/// Marks an anchor placed while the drawing still lacks the points it needs.
pub(super) fn placed_anchor(painter: &Painter, pos: Pos2, color: Color32) {
    painter.circle_filled(pos, PLACED_ANCHOR_RADIUS, color);
}

impl Drawing {
    /// Size annotations before laying out both the text and its surrounding box.
    pub(super) fn annotation_font(&self) -> egui::FontId {
        egui::FontId::proportional(crate::styles::typography::font_size(self.font_size))
    }

    /// Optional user label for line/shape tools that support annotations.
    ///
    /// Primary text tools already paint their body; this only draws when `supports_line_label`
    /// is set and `text` is non-empty. Sloped lines rotate the label to match the line angle
    /// (always kept upright right-side-up) and offset it above, on, or below the line.
    /// When `TextVerticalAlign::Middle` is active on a line tool, the underlying line segment
    /// is split around the label so the line does not strike through the text.
    pub(super) fn paint_optional_line_label(&self, painter: &Painter, start: usize, rect: Rect) {
        if !self.tool_type.supports_line_label() {
            return;
        }
        let Some(text) = self.text.as_deref().filter(|t| !t.is_empty()) else {
            return;
        };
        let color = self.text_color32();
        let font = self.annotation_font();
        let galley = if self.font_italic {
            let mut job = egui::text::LayoutJob::single_section(
                text.to_owned(),
                egui::TextFormat {
                    font_id: font.clone(),
                    color,
                    italics: true,
                    ..Default::default()
                },
            );
            job.wrap.max_width = f32::INFINITY;
            painter.layout_job(job)
        } else {
            painter.layout_no_wrap(text.to_owned(), font.clone(), color)
        };
        let Some((shape, line_cut)) = self.place_line_label_shape(galley, color, rect) else {
            return;
        };
        if let Some((left_pt, u, t0, t1)) = line_cut {
            painter.ctx().graphics_mut(|g| {
                let list = g.entry(painter.layer_id());
                for index in start..list.next_idx().0 {
                    list.mutate_shape(ShapeIdx(index), |s| {
                        cut_line_segment_for_label(&mut s.shape, left_pt, u, t0, t1);
                    });
                }
            });
        }
        if self.font_weight == FontWeight::Bold {
            let mut weighted = shape.clone();
            let dir = egui::vec2(shape.angle.cos(), shape.angle.sin());
            weighted.pos += dir * 0.5;
            weighted.opacity_factor = 0.55;
            painter.add(Shape::Text(shape));
            painter.add(Shape::Text(weighted));
        } else {
            painter.add(Shape::Text(shape));
        }
    }

    fn place_line_label_shape(
        &self,
        galley: std::sync::Arc<egui::Galley>,
        color: Color32,
        rect: Rect,
    ) -> Option<LineLabelLayout> {
        let size = galley.size();
        let gap = (self.stroke_width * 0.5 + 5.0).max(6.0);

        // Axis-aligned rectangle: place horizontally at top/middle/bottom and left/center/right.
        if self.tool_type == DrawingToolType::Rect
            && let [p1, p2, ..] = self.points.as_slice()
        {
            let r = Rect::from_two_pos(*p1, *p2);
            let x = match self.text_h_align {
                TextHorizontalAlign::Left => r.left() + 8.0,
                TextHorizontalAlign::Center => r.center().x - size.x * 0.5,
                TextHorizontalAlign::Right => (r.right() - 8.0 - size.x).max(r.left() + 4.0),
            }
            .round();
            let y = match self.text_v_align {
                TextVerticalAlign::Top => r.top() - size.y - 4.0,
                TextVerticalAlign::Inside => r.top() + 4.0,
                TextVerticalAlign::Middle => r.center().y - size.y * 0.5,
                TextVerticalAlign::Bottom => r.bottom() + 4.0,
            }
            .round();
            return Some((TextShape::new(Pos2::new(x, y), galley, color), None));
        }

        // Two-boundary channels: Top on the upper line, Bottom on the lower, Inside between.
        if matches!(
            self.tool_type,
            DrawingToolType::FlatTopBottom | DrawingToolType::DisjointChannel
        ) && let Some(layout) =
            self.place_channel_line_label(galley.clone(), color, rect, size, gap)
        {
            return Some(layout);
        }

        // Determine the primary segment (a -> b) along which the label runs.
        let (a, b) = match (self.tool_type, self.points.as_slice()) {
            (_, []) => return None,
            (DrawingToolType::HorizontalLine, [only, ..]) => (
                Pos2::new(rect.left() + 16.0, only.y),
                Pos2::new(rect.right() - 16.0, only.y),
            ),
            (DrawingToolType::HorizontalRay, [only, ..]) => (
                Pos2::new(only.x.max(rect.left() + 8.0), only.y),
                Pos2::new(rect.right() - 16.0, only.y),
            ),
            (DrawingToolType::VerticalLine, [only, ..]) => (
                Pos2::new(only.x, rect.top() + 16.0),
                Pos2::new(only.x, rect.bottom() - 16.0),
            ),
            (DrawingToolType::CrossLine, [only, ..]) => (
                Pos2::new(rect.left() + 16.0, only.y),
                Pos2::new(rect.right() - 16.0, only.y),
            ),
            (_, [only]) => (*only, *only + egui::vec2(1.0, 0.0)),
            (_, [p0, p1, ..]) => (*p0, *p1),
        };

        // Ensure left-to-right reading order so rotated text is never upside-down.
        let (left_pt, right_pt) = if a.x < b.x || (a.x == b.x && a.y <= b.y) {
            (a, b)
        } else {
            (b, a)
        };

        let delta = right_pt - left_pt;
        let len = delta.length();
        let (u, mut angle) = if len > 1e-3 {
            (delta / len, delta.angle())
        } else {
            (egui::vec2(1.0, 0.0), 0.0)
        };
        if angle.abs() < 0.02 {
            angle = 0.0;
        }
        // Screen-upward normal vector (`u` points rightwards, so `n.y <= 0`).
        let n_up = egui::vec2(u.y, -u.x);

        let anchor_t = match self.text_h_align {
            TextHorizontalAlign::Left => 8.0_f32.min(len * 0.25),
            TextHorizontalAlign::Center => len * 0.5,
            TextHorizontalAlign::Right => len - 8.0_f32.min(len * 0.25),
        };
        let anchor = left_pt + u * anchor_t;

        let along_shift = match self.text_h_align {
            TextHorizontalAlign::Left => 0.0,
            TextHorizontalAlign::Center => -size.x * 0.5,
            TextHorizontalAlign::Right => -size.x,
        };
        let perp_shift = match self.text_v_align {
            TextVerticalAlign::Top => size.y + gap,
            TextVerticalAlign::Inside => size.y + 2.0,
            TextVerticalAlign::Middle => size.y * 0.5,
            TextVerticalAlign::Bottom => -gap,
        };

        let mut top_left = anchor + u * along_shift + n_up * perp_shift;
        if angle == 0.0 {
            top_left = Pos2::new(top_left.x.round(), top_left.y.round());
        }

        let line_cut = (self.text_v_align == TextVerticalAlign::Middle && len > size.x + 12.0)
            .then(|| {
                let pad = 6.0;
                let t0 = anchor_t + along_shift - pad;
                let t1 = anchor_t + along_shift + size.x + pad;
                (left_pt, u, t0, t1)
            });

        Some((
            TextShape::new(top_left, galley, color).with_angle(angle),
            line_cut,
        ))
    }

    /// Label Text on Flat Top/Bottom and Disjoint Channel: Top on the upper boundary,
    /// Bottom on the lower boundary, Inside between both.
    fn place_channel_line_label(
        &self,
        galley: std::sync::Arc<egui::Galley>,
        color: Color32,
        rect: Rect,
        size: egui::Vec2,
        gap: f32,
    ) -> Option<LineLabelLayout> {
        let outline = self.outline(rect)?;
        let &[first, second, ..] = outline.lines.as_slice() else {
            return None;
        };
        let mid_y = |line: [Pos2; 2]| (line[0].y + line[1].y) * 0.5;
        let (upper, lower) = if mid_y(first) <= mid_y(second) {
            (first, second)
        } else {
            (second, first)
        };
        let align = match self.text_v_align {
            TextVerticalAlign::Bottom => TextVerticalAlign::Bottom,
            TextVerticalAlign::Inside | TextVerticalAlign::Middle => TextVerticalAlign::Inside,
            TextVerticalAlign::Top => TextVerticalAlign::Top,
        };
        let (a, b) = match align {
            TextVerticalAlign::Bottom => (lower[0], lower[1]),
            _ => (upper[0], upper[1]),
        };
        let (left_pt, right_pt) = if a.x < b.x || (a.x == b.x && a.y <= b.y) {
            (a, b)
        } else {
            (b, a)
        };
        let delta = right_pt - left_pt;
        let len = delta.length();
        let (u, mut angle) = if len > 1e-3 {
            (delta / len, delta.angle())
        } else {
            (egui::vec2(1.0, 0.0), 0.0)
        };
        if angle.abs() < 0.02 {
            angle = 0.0;
        }
        let n_up = egui::vec2(u.y, -u.x);
        let anchor_t = match self.text_h_align {
            TextHorizontalAlign::Left => 8.0_f32.min(len * 0.25),
            TextHorizontalAlign::Center => len * 0.5,
            TextHorizontalAlign::Right => len - 8.0_f32.min(len * 0.25),
        };
        let anchor = left_pt + u * anchor_t;
        let along_shift = match self.text_h_align {
            TextHorizontalAlign::Left => 0.0,
            TextHorizontalAlign::Center => -size.x * 0.5,
            TextHorizontalAlign::Right => -size.x,
        };
        let perp_shift = match align {
            TextVerticalAlign::Top => size.y + gap,
            TextVerticalAlign::Bottom => -gap,
            TextVerticalAlign::Inside | TextVerticalAlign::Middle => {
                // Midpoint between the two boundaries at this horizontal position.
                let t = if (right_pt.x - left_pt.x).abs() > 1e-3 {
                    ((anchor.x - left_pt.x) / (right_pt.x - left_pt.x)).clamp(0.0, 1.0)
                } else {
                    0.5
                };
                let upper_y = upper[0].y + (upper[1].y - upper[0].y) * t;
                let lower_y = lower[0].y + (lower[1].y - lower[0].y) * t;
                let mid = Pos2::new(anchor.x, (upper_y + lower_y) * 0.5);
                // Offset so the text center sits at the channel midpoint.
                let to_mid = mid - anchor;
                return Some((
                    TextShape::new(
                        {
                            let mut top_left =
                                anchor + u * along_shift + to_mid - n_up * (size.y * 0.5);
                            if angle == 0.0 {
                                top_left = Pos2::new(top_left.x.round(), top_left.y.round());
                            }
                            top_left
                        },
                        galley,
                        color,
                    )
                    .with_angle(angle),
                    None,
                ));
            }
        };
        let mut top_left = anchor + u * along_shift + n_up * perp_shift;
        if angle == 0.0 {
            top_left = Pos2::new(top_left.x.round(), top_left.y.round());
        }
        Some((
            TextShape::new(top_left, galley, color).with_angle(angle),
            None,
        ))
    }

    /// Apply line patterns and font weight without changing the renderer's widths or fills.
    pub(super) fn finish_style(&self, painter: &Painter, start: usize, rect: Rect) {
        let styled = self.fill_color.is_some()
            || self.text_color.is_some()
            || self.line_style != LineStyle::Solid
            || self.font_weight == FontWeight::Bold
            || self.font_italic
            || self.extend_left
            || self.extend_right;
        let clip = crate::painting::dashed_line::finite_clip(painter);
        painter.ctx().graphics_mut(|g| {
            let list = g.entry(painter.layer_id());
            // Segments that continue one another become one path, which the tessellator joins
            // with half the vertices and a dash pattern runs along without restarting.
            let mut previous: Option<(ShapeIdx, Pos2, Stroke, Rect)> = None;
            for index in start..list.next_idx().0 {
                let mut segment = None;
                list.mutate_shape(ShapeIdx(index), |s| {
                    if styled
                        && matches!(
                            self.tool_type,
                            DrawingToolType::InfoLine
                                | DrawingToolType::TrendAngle
                                | DrawingToolType::Arrow
                                | DrawingToolType::ArrowMarker
                        )
                        && let Shape::LineSegment { points, .. } = &mut s.shape
                        && self.points.get(..2) == Some(points.as_slice())
                        && let Some((a, b)) = self.trend_line_ends(rect)
                    {
                        *points = [a, b];
                    }
                    if let Shape::LineSegment { points, stroke } = s.shape {
                        segment = Some((points, stroke, s.clip_rect));
                    }
                });
                if let Some((points, stroke, clip)) = segment {
                    if let Some((first, end, old_stroke, old_clip)) = previous
                        && end == points[0]
                        && old_stroke == stroke
                        && old_clip == clip
                    {
                        list.mutate_shape(first, |s| match &mut s.shape {
                            Shape::LineSegment { points: old, .. } => {
                                s.shape = Shape::line(vec![old[0], old[1], points[1]], stroke)
                            }
                            Shape::Path(p) => p.points.push(points[1]),
                            _ => {}
                        });
                        list.reset_shape(ShapeIdx(index));
                        previous = Some((first, points[1], stroke, clip));
                    } else {
                        previous = Some((ShapeIdx(index), points[1], stroke, clip));
                    }
                } else {
                    previous = None;
                }
            }
            if styled {
                for index in start..list.next_idx().0 {
                    list.mutate_shape(ShapeIdx(index), |s| self.style_shape(&mut s.shape, clip));
                }
            }
        });
        self.render_end_markers(painter, rect);
        let pixels_per_point = painter.pixels_per_point();
        let snap = crate::painting::pixel_snap::PixelSnap::new(pixels_per_point);
        painter.ctx().graphics_mut(|g| {
            let list = g.entry(painter.layer_id());
            for index in start..list.next_idx().0 {
                list.mutate_shape(ShapeIdx(index), |s| {
                    fewer_circle_sides(&mut s.shape, pixels_per_point);
                    snap_axis_aligned_lines(&mut s.shape, snap);
                });
            }
        });
    }

    /// Restyles one shape. Dashes are generated only inside `clip`, so a drawing reaching far off
    /// screen costs no more than its visible part; without a finite clip nothing is dashed.
    fn style_shape(&self, shape: &mut Shape, clip: Option<Rect>) {
        if let Shape::Vec(shapes) = shape {
            for s in shapes {
                self.style_shape(s, clip);
            }
            return;
        }
        if let Shape::Text(t) = shape {
            if let Some(custom_color) = self
                .text_color
                .map(|c| Color32::from_rgba_unmultiplied(c[0], c[1], c[2], c[3]))
            {
                t.override_text_color = Some(custom_color);
                t.fallback_color = custom_color;
            }
            if self.font_italic && !t.galley.job.sections.iter().any(|s| s.format.italics) {
                let mut galley = (*t.galley).clone();
                for placed in &mut galley.rows {
                    let bottom = placed.rect().height();
                    let row = std::sync::Arc::make_mut(&mut placed.row);
                    for v in &mut row.visuals.mesh.vertices {
                        v.pos.x += (bottom - v.pos.y) * 0.18;
                    }
                }
                t.galley = std::sync::Arc::new(galley);
            }
            if self.font_weight == FontWeight::Bold {
                // Synthetic weight keeps host font families and fallback glyphs intact.
                let mut weighted = t.clone();
                let dir = egui::vec2(t.angle.cos(), t.angle.sin());
                weighted.pos += dir * 0.5;
                weighted.opacity_factor = 0.55;
                *shape = Shape::Vec(vec![Shape::Text(t.clone()), Shape::Text(weighted)]);
            }
            return;
        }
        let fork = matches!(
            self.tool_type,
            DrawingToolType::Pitchfork
                | DrawingToolType::SchiffPitchfork
                | DrawingToolType::ModifiedSchiffPitchfork
                | DrawingToolType::InsidePitchfork
        );
        let custom_fork = fork && self.pitchfork.is_some();
        // Annotation body fills take the drawing fill colour. Pitchfork bands are styled only
        // through PitchforkConfig (Inputs → Background), not Drawing::fill_color.
        let fill = |color: &mut Color32| {
            if color.a() > 0
                && matches!(
                    self.tool_type,
                    DrawingToolType::TextLabel
                        | DrawingToolType::AnchoredText
                        | DrawingToolType::Note
                        | DrawingToolType::Callout
                        | DrawingToolType::PriceLabel
                        | DrawingToolType::FlagNote
                        | DrawingToolType::PriceNote
                        | DrawingToolType::Table
                        | DrawingToolType::Comment
                        | DrawingToolType::Signpost
                        | DrawingToolType::Image
                        | DrawingToolType::Tweet
                        | DrawingToolType::Idea
                        | DrawingToolType::Forecast
                        | DrawingToolType::ArrowMarkUp
                        | DrawingToolType::ArrowMarkDown
                        | DrawingToolType::ArrowMarkLeft
                        | DrawingToolType::ArrowMarkRight
                )
                && let Some(configured) = self.fill_color32()
            {
                *color = configured;
            }
        };
        let dash = if custom_fork || self.tool_type == DrawingToolType::RegressionTrend {
            None
        } else {
            self.line_style.dash_pattern()
        };
        let mut path = Vec::new();
        let stroke = match shape {
            Shape::LineSegment { points, stroke } => {
                path.extend(*points);
                *stroke
            }
            Shape::Path(p) => {
                fill(&mut p.fill);
                let ColorMode::Solid(color) = &p.stroke.color else {
                    return;
                };
                let stroke = Stroke::new(p.stroke.width, *color);
                if dash.is_none() || stroke.is_empty() {
                    return;
                }
                path = p.points.clone();
                if p.closed
                    && let Some(&first) = path.first()
                {
                    path.push(first);
                }
                p.stroke.width = 0.0;
                stroke
            }
            Shape::Rect(r) => {
                fill(&mut r.fill);
                if dash.is_none() || r.stroke.is_empty() {
                    return;
                }
                egui::epaint::tessellator::path::rounded_rectangle(
                    &mut path,
                    egui::epaint::RoundedRect::new(r.rect, r.corner_radius),
                );
                if let Some(&first) = path.first() {
                    path.push(first);
                }
                let stroke = r.stroke;
                r.stroke = Stroke::NONE;
                stroke
            }
            Shape::Circle(c) => {
                // Anchor decorations keep their thin white rings when the body is restyled.
                if c.stroke.color == Color32::WHITE
                    && c.radius <= crate::tokens::DESIGN_TOKENS.rounding.lg
                    && self.tool_type != DrawingToolType::Circle
                {
                    return;
                }

                if dash.is_none() || c.stroke.is_empty() {
                    return;
                }
                path = (0..=64)
                    .map(|i| {
                        let angle = i as f32 * std::f32::consts::TAU / 64.0;
                        c.center + c.radius * egui::vec2(angle.cos(), angle.sin())
                    })
                    .collect();
                let stroke = c.stroke;
                c.stroke = Stroke::NONE;
                stroke
            }
            Shape::Ellipse(e) => {
                if dash.is_none() || e.stroke.is_empty() {
                    return;
                }
                path = (0..=64)
                    .map(|i| {
                        let angle = i as f32 * std::f32::consts::TAU / 64.0;
                        e.center + e.radius * egui::vec2(angle.cos(), angle.sin())
                    })
                    .collect();
                let stroke = e.stroke;
                e.stroke = Stroke::NONE;
                stroke
            }
            _ => return,
        };
        if let Some((dash, gap)) = dash
            && !stroke.is_empty()
        {
            let mut shapes = clip.map_or_else(Vec::new, |clip| {
                // Widened by the stroke so an edge just outside the clip keeps its visible half.
                let clip = clip.expand(stroke.width);
                crate::painting::dashed_line::dashed_polyline_shapes(&path, stroke, dash, gap, clip)
            });
            if !matches!(shape, Shape::LineSegment { .. }) {
                shapes.insert(0, std::mem::replace(shape, Shape::Noop));
            }
            *shape = Shape::Vec(shapes);
        }
    }

    fn render_end_markers(&self, painter: &Painter, rect: Rect) {
        use DrawingToolType as T;
        if !self.arrow_start_style.is_visible() && !self.arrow_end_style.is_visible() {
            return;
        }
        let Some(&a) = self.points.first() else {
            return;
        };
        let b = self.points.get(1).copied().unwrap_or(a);
        let ends = match self.tool_type {
            T::TrendLine
            | T::InfoLine
            | T::TrendAngle
            | T::Ray
            | T::ExtendedLine
            | T::Arrow
            | T::ArrowMarker => self.trend_line_ends(rect).map(|(a, b)| [a, b]),
            T::HorizontalLine => Some([Pos2::new(rect.min.x, a.y), Pos2::new(rect.max.x, a.y)]),
            T::HorizontalRay => Some([a, Pos2::new(rect.max.x, a.y)]),
            T::VerticalLine => Some([Pos2::new(a.x, rect.min.y), Pos2::new(a.x, rect.max.y)]),
            T::Polyline | T::Path => self.points.last().map(|&b| [a, b]),
            _ => None,
        };
        if let Some(mut ends) = ends {
            if matches!(
                self.tool_type,
                T::TrendLine
                    | T::InfoLine
                    | T::TrendAngle
                    | T::ExtendedLine
                    | T::Arrow
                    | T::ArrowMarker
            ) && (b.x < a.x || (self.tool_type == T::ExtendedLine && b.x == a.x && b.y < a.y))
            {
                ends.swap(0, 1);
            }
            let [start, end] = ends;
            let (start_direction, end_direction) =
                if matches!(self.tool_type, T::Polyline | T::Path) && self.points.len() > 2 {
                    (a - b, end - self.points[self.points.len() - 2])
                } else {
                    (start - end, end - start)
                };
            self.render_marker(painter, start, start_direction, self.arrow_start_style);
            self.render_marker(painter, end, end_direction, self.arrow_end_style);
        }
    }

    fn render_marker(
        &self,
        painter: &Painter,
        tip: Pos2,
        direction: egui::Vec2,
        style: ArrowStyle,
    ) {
        if direction.length_sq() < 1e-6 {
            return;
        }
        let color = self.color32();
        let size = (self.stroke_width * style.size_multiplier()).max(4.0);
        let along = direction.normalized() * size;
        let across = egui::vec2(-along.y, along.x) / 2.0;
        let base = tip - along;
        match style {
            ArrowStyle::None => {}
            ArrowStyle::Arrow => {
                painter.add(Shape::convex_polygon(
                    vec![tip, base + across, base - across],
                    color,
                    Stroke::NONE,
                ));
            }
            ArrowStyle::OpenArrow => {
                painter.add(Shape::line(
                    vec![base + across, tip, base - across],
                    self.stroke(),
                ));
            }
            ArrowStyle::Circle => {
                painter.circle_filled(tip, size / 2.0, color);
            }
            ArrowStyle::Square => {
                painter.rect_filled(
                    Rect::from_center_size(tip, egui::Vec2::splat(size)),
                    0.0,
                    color,
                );
            }
            ArrowStyle::Diamond => {
                painter.add(Shape::convex_polygon(
                    vec![
                        tip + along / 2.0,
                        tip + across,
                        tip - along / 2.0,
                        tip - across,
                    ],
                    color,
                    Stroke::NONE,
                ));
            }
        }
    }
}

/// Cuts any line segment that lies collinearly on the ray `(left_pt, u)` across `[t0, t1]`
/// so a `TextVerticalAlign::Middle` label sits inside a clean gap in the line.
fn cut_line_segment_for_label(shape: &mut Shape, left_pt: Pos2, u: egui::Vec2, t0: f32, t1: f32) {
    match shape {
        Shape::Vec(shapes) => {
            for s in shapes {
                cut_line_segment_for_label(s, left_pt, u, t0, t1);
            }
        }
        Shape::LineSegment { points, stroke } => {
            let [p0, p1] = *points;
            let stroke = *stroke;
            let n = egui::vec2(u.y, -u.x);
            let d0 = (p0 - left_pt).dot(n).abs();
            let d1 = (p1 - left_pt).dot(n).abs();
            if d0 > 2.0 || d1 > 2.0 {
                return;
            }
            let s0 = (p0 - left_pt).dot(u);
            let s1 = (p1 - left_pt).dot(u);
            let (seg_min, seg_max, a_pt, b_pt) = if s0 <= s1 {
                (s0, s1, p0, p1)
            } else {
                (s1, s0, p1, p0)
            };
            if t1 <= seg_min || t0 >= seg_max {
                return;
            }
            let mut pieces = Vec::new();
            if t0 > seg_min + 1.0 {
                let cut_a = left_pt + u * t0 + n * (a_pt - left_pt).dot(n);
                pieces.push(Shape::LineSegment {
                    points: [a_pt, cut_a],
                    stroke,
                });
            }
            if t1 < seg_max - 1.0 {
                let cut_b = left_pt + u * t1 + n * (b_pt - left_pt).dot(n);
                pieces.push(Shape::LineSegment {
                    points: [cut_b, b_pt],
                    stroke,
                });
            }
            *shape = match pieces.len() {
                0 => Shape::Noop,
                1 => pieces.pop().unwrap(),
                _ => Shape::Vec(pieces),
            };
        }
        _ => {}
    }
}

/// Replaces a circle with the polygon of the fewest sides within a quarter of a physical pixel of
/// it. The tessellator gives a circle 32 sides from a radius of 5 points and 64 from 18, which made
/// the anchor dots and rings of a drawing most of its vertices. A small disc without an outline
/// stays a circle, which the tessellator draws as one textured quad.
fn fewer_circle_sides(shape: &mut Shape, pixels_per_point: f32) {
    match shape {
        Shape::Vec(shapes) => {
            for shape in shapes {
                fewer_circle_sides(shape, pixels_per_point);
            }
        }
        Shape::Circle(c) if !c.stroke.is_empty() || c.radius * pixels_per_point > 6.0 => {
            let sides = circle_sides(c.radius * pixels_per_point);
            let points = (0..sides)
                .map(|i| {
                    let angle = i as f32 * std::f32::consts::TAU / sides as f32;
                    c.center + c.radius * egui::vec2(angle.cos(), angle.sin())
                })
                .collect();
            let mut path = egui::epaint::PathShape::convex_polygon(points, c.fill, c.stroke);
            path.stroke = path.stroke.outside();
            *shape = Shape::Path(path);
        }
        _ => {}
    }
}

/// Moves horizontal and vertical line segments and upright text onto the physical pixel grid.
fn snap_axis_aligned_lines(shape: &mut Shape, snap: crate::painting::pixel_snap::PixelSnap) {
    match shape {
        Shape::Vec(shapes) => {
            for shape in shapes {
                snap_axis_aligned_lines(shape, snap);
            }
        }
        Shape::LineSegment { points, stroke } if !stroke.is_empty() => {
            let [a, b] = points;
            if a.y == b.y && a.x != b.x {
                let y = snap.line_center(a.y, stroke.width);
                (a.y, b.y) = (y, y);
            } else if a.x == b.x && a.y != b.y {
                let x = snap.line_center(a.x, stroke.width);
                (a.x, b.x) = (x, x);
            }
        }
        Shape::Text(t) if t.angle.abs() < 1e-4 => {
            t.pos = Pos2::new(snap.edge(t.pos.x), snap.edge(t.pos.y));
        }
        _ => {}
    }
}

/// The fewest sides of a polygon whose sides stay within a quarter of a physical pixel of a
/// circle of `radius` physical pixels, from 24 to 128.
pub(super) fn circle_sides(radius: f32) -> usize {
    let step = (1.0 - 0.25 / radius).clamp(-1.0, 1.0).acos();
    ((std::f32::consts::PI / step).ceil() as usize).clamp(24, 128)
}

#[cfg(test)]
mod tests {
    use super::*;
    use egui::epaint::StrokeKind;

    fn paint(drawing: &Drawing) -> Vec<Shape> {
        let pane = Rect::from_min_size(Pos2::ZERO, egui::vec2(800.0, 600.0));
        let ctx = egui::Context::default();
        let output = ctx.run_ui(
            egui::RawInput {
                screen_rect: Some(pane),
                ..Default::default()
            },
            |ui| {
                drawing.render_original(
                    ui.painter(),
                    pane,
                    &crate::drawings::rendering::ChartView {
                        instrument: None,
                        bars: &[],
                        revision: 0,
                        bar_to_x: &|x| x,
                        price_to_y: &|p| p as f32,
                        y_to_price: &f64::from,
                        time_axis: true,
                        executions: None,
                    },
                );
            },
        );
        fn collect(shape: &Shape, out: &mut Vec<Shape>) {
            match shape {
                Shape::Vec(shapes) => shapes.iter().for_each(|s| collect(s, out)),
                _ => out.push(shape.clone()),
            }
        }
        let mut shapes = Vec::new();
        for s in &output.shapes {
            collect(&s.shape, &mut shapes);
        }
        output.drop_without_applying_deltas();
        shapes
    }

    fn pattern(tool: DrawingToolType) -> Drawing {
        let mut drawing = Drawing::with_color(1, tool, [242, 54, 69, 255]);
        drawing.points = [
            (100., 300.),
            (180., 100.),
            (260., 250.),
            (340., 150.),
            (420., 220.),
            (500., 180.),
            (580., 300.),
        ]
        .map(|(x, y)| Pos2::new(x, y))
        .to_vec();
        drawing.points.truncate(tool.required_points().unwrap_or(7));
        drawing.chart_points = drawing
            .points
            .iter()
            .map(|p| crate::drawings::ChartPoint::new(p.x, f64::from(p.y)))
            .collect();
        drawing
    }

    #[test]
    fn main_widths_are_exact_and_details_stay_thin() {
        use DrawingToolType::*;
        for tool in [
            XABCDPattern,
            CypherPattern,
            HeadAndShoulders,
            ABCDPattern,
            TrianglePattern,
            ThreeDrivesPattern,
            ElliottImpulse,
            ElliottCorrection,
            ElliottTriangle,
            ElliottDoubleCombo,
            ElliottTripleCombo,
            CyclicLines,
            TimeCycles,
            SineLine,
            GannFan,
            GannBox,
            GannSquare,
            GannFixed,
        ] {
            for width in [1.0, 2.0, 3.0] {
                let mut drawing = pattern(tool);
                drawing.stroke_width = width;
                let shapes = paint(&drawing);
                let mut main = 0;
                let mut details = 0;
                for shape in &shapes {
                    let (is_main, stroke) = match shape {
                        Shape::LineSegment { points, stroke } => (
                            drawing.points.windows(2).any(|p| p == points)
                                || matches!(
                                    tool,
                                    CyclicLines | GannFan | GannBox | GannSquare | GannFixed
                                )
                                || (tool == HeadAndShoulders && stroke.color.a() == 200),
                            *stroke,
                        ),
                        Shape::Path(p) => {
                            let ColorMode::Solid(color) = p.stroke.color else {
                                continue;
                            };
                            (
                                p.points
                                    .windows(2)
                                    .any(|p| drawing.points.windows(2).any(|leg| leg == p))
                                    || (tool == SineLine && p.points.len() > 100)
                                    || (tool == TimeCycles
                                        && p.points.len() > 4
                                        && color == drawing.color32()),
                                Stroke::new(p.stroke.width, color),
                            )
                        }
                        Shape::Rect(r) => (
                            matches!(tool, GannBox | GannSquare | GannFixed)
                                && !r.stroke.is_empty(),
                            r.stroke,
                        ),
                        _ => continue,
                    };
                    if stroke.is_empty() {
                        continue;
                    }
                    if is_main {
                        main += 1;
                        assert_eq!(stroke.width, width, "{tool:?} main at {width}");
                    } else if stroke.color != Color32::WHITE
                        && (stroke.color.a() < 255
                            || matches!(shape, Shape::Path(p) if p.closed && p.points.len() >= 8))
                    {
                        details += 1;
                        assert!(
                            stroke.width <= 2.0,
                            "{tool:?} detail widened to {}",
                            stroke.width
                        );
                    }
                }
                assert!(main > 0, "{tool:?} missing main line");
                if matches!(
                    tool,
                    XABCDPattern | ElliottTriangle | ElliottDoubleCombo | ElliottTripleCombo
                ) {
                    assert!(details > 0, "{tool:?} missing details");
                }
            }
        }
    }

    #[test]
    fn pattern_fills_toggle_without_recolouring_labels() {
        use DrawingToolType::*;
        for tool in [
            XABCDPattern,
            CypherPattern,
            HeadAndShoulders,
            ABCDPattern,
            TrianglePattern,
            ThreeDrivesPattern,
            ElliottImpulse,
            ElliottCorrection,
            ElliottTriangle,
            GannBox,
            GannSquare,
            GannFixed,
        ] {
            let mut drawing = pattern(tool);
            let gann = matches!(tool, GannBox | GannSquare | GannFixed);
            if gann {
                assert!(drawing.fill_color.is_none(), "{tool:?} starts unfilled");
            } else if tool != ElliottImpulse {
                assert!(
                    drawing.fill_color.is_some(),
                    "{tool:?} lost its default tint"
                );
            }
            let coloured = Drawing::with_color(1, tool, [67, 89, 123, 255]);
            if !gann && tool != ElliottCorrection {
                assert_eq!(
                    &coloured.fill_color.unwrap()[..3],
                    &[67, 89, 123],
                    "{tool:?} default fill must follow its line colour"
                );
            }
            let configured = Color32::from_rgba_unmultiplied(12, 34, 56, 80);
            for fill in [None, Some([12, 34, 56, 80])] {
                drawing.fill_color = fill;
                let shapes = paint(&drawing);
                let mut bodies = 0;
                for shape in &shapes {
                    match shape {
                        Shape::Path(p) if p.closed && p.points.len() <= 7 => {
                            bodies += usize::from(p.fill.a() > 0);
                            assert_eq!(
                                p.fill,
                                fill.map_or(Color32::TRANSPARENT, |_| configured),
                                "{tool:?} body"
                            );
                        }
                        Shape::Rect(r) if r.fill.a() > 0 => {
                            if (matches!(tool, GannBox | GannSquare | GannFixed)
                                && r.rect.contains(drawing.points[0]))
                                || (tool == ThreeDrivesPattern
                                    && r.rect.left() == drawing.points[5].x)
                            {
                                bodies += 1;
                                assert_eq!(r.fill, configured, "{tool:?} body");
                            } else {
                                assert_ne!(r.fill, configured, "{tool:?} label recoloured");
                            }
                        }
                        _ => {}
                    }
                }
                assert_eq!(bodies > 0, fill.is_some(), "{tool:?} fill toggle");
            }
        }
    }

    /// Guards the circles of drawings at 1x and 2x: a circle with an outline, or too large for a
    /// textured quad, becomes a closed polygon whose corners lie on the circle and whose sides
    /// stay within a quarter of a physical pixel of it, with the outline still outside the
    /// radius; a small disc without an outline stays a circle.
    #[test]
    fn circles_become_polygons_within_a_quarter_pixel() {
        let center = Pos2::new(50.0, 40.0);
        let ring = Stroke::new(1.0, Color32::WHITE);
        for (radius, ppp, stroke, polygon) in [
            (3.0, 1.0, Stroke::NONE, false),
            (3.0, 2.0, Stroke::NONE, false),
            (3.0, 1.0, ring, true),
            (8.0, 1.0, Stroke::NONE, true),
            (8.0, 2.0, ring, true),
            (200.0, 2.0, ring, true),
        ] {
            let circle = egui::epaint::CircleShape {
                center,
                radius,
                fill: Color32::RED,
                stroke,
            };
            let mut shape = Shape::Circle(circle);
            fewer_circle_sides(&mut shape, ppp);
            let case = format!("radius {radius} at {ppp}x");
            let Shape::Path(path) = &shape else {
                assert!(!polygon, "{case}: still a circle");
                continue;
            };
            assert!(polygon, "{case}: a quad disc became a polygon");
            assert!(path.closed, "{case}");
            assert_eq!(
                (path.fill, path.stroke.kind),
                (Color32::RED, StrokeKind::Outside)
            );
            let corners = &path.points;
            for (i, a) in corners.iter().enumerate() {
                let b = corners[(i + 1) % corners.len()];
                assert!(((*a - center).length() - radius).abs() < 1e-3, "{case}");
                let error = (radius - (center.distance(a.lerp(b, 0.5)))) * ppp;
                assert!((0.0..=0.25).contains(&error), "{case}: {error} px");
            }
        }
    }

    /// Guards the frame cost of dashed drawings: a rect, a circle and an ellipse reaching a
    /// million points beyond an 800 x 600 pane are dashed only inside it, rather than with one
    /// shape per dash along their whole outline.
    #[test]
    fn far_reaching_dashed_outlines_are_dashed_only_inside_the_pane() {
        use crate::drawings::rendering::ChartView;
        use egui::{Context, RawInput};
        fn leaves(shape: &Shape) -> usize {
            match shape {
                Shape::Vec(shapes) => shapes.iter().map(leaves).sum(),
                _ => 1,
            }
        }
        let pane = Rect::from_min_size(Pos2::ZERO, egui::vec2(800.0, 600.0));
        for (tool, a, b) in [
            (DrawingToolType::Rect, (-1e6, 100.0), (1e6, 200.0)),
            (
                DrawingToolType::Circle,
                (400.0, 1e6 + 300.0),
                (400.0, 300.0),
            ),
            (DrawingToolType::Ellipse, (-1e6, 300.0), (1e6, 2e6 + 300.0)),
        ] {
            let mut drawing = Drawing::new(1, tool);
            drawing.line_style = LineStyle::Dashed;
            drawing.points = vec![Pos2::new(a.0, a.1), Pos2::new(b.0, b.1)];
            let ctx = Context::default();
            let input = RawInput {
                screen_rect: Some(pane),
                ..Default::default()
            };
            let output = ctx.run_ui(input, |ui| {
                let view = ChartView {
                    instrument: None,
                    bars: &[],
                    revision: 0,
                    bar_to_x: &|b| b,
                    price_to_y: &|p| p as f32,
                    y_to_price: &f64::from,
                    time_axis: true,
                    executions: None,
                };
                drawing.render_original(ui.painter(), pane, &view);
            });
            let shapes: usize = output.shapes.iter().map(|s| leaves(&s.shape)).sum();
            output.drop_without_applying_deltas();
            assert!((1..1_000).contains(&shapes), "{tool:?}: {shapes} shapes");
        }
    }
}
