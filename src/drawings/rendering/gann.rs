//! Gann tool rendering: fan, box, square and fixed square.
//!
//! Every line takes its colour from its level and its width and dash from the drawing.

use crate::drawings::domain::{
    Drawing, GannConfig, GannFanDirection, GannLevel, gann_fan_label, gann_square_fan_label,
};
use crate::painting::dashed_line::draw_dashed_line;
use crate::styles::{stroke, typography};
use crate::tokens::DESIGN_TOKENS;
use egui::{Align2, Color32, FontId, Pos2, Rect, Stroke};

/// Gap between a level line and its label.
const LABEL_GAP: f32 = 5.0;

fn color32(color: [u8; 4]) -> Color32 {
    Color32::from_rgba_unmultiplied(color[0], color[1], color[2], color[3])
}

/// Screen y at `x` of the fan line through `start` whose slope is `ratio`
/// times the 1x1 slope from `start` to `end`.
fn gann_fan_line_y(start: Pos2, end: Pos2, ratio: f32, x: f32) -> f32 {
    let dy = end.y - start.y;
    let dx = end.x - start.x;
    // A vertical base has no slope; fall back to one pixel per pixel in the direction of the move.
    let base_slope = if dx.abs() > 0.001 {
        dy / dx
    } else if dy < 0.0 {
        -1.0
    } else {
        1.0
    };
    start.y + (x - start.x) * base_slope * ratio
}

/// Each enabled ray of a Gann fan from `start` with its level, running from the nearer anchor
/// to the right pane edge. The 1x1 ray (ratio 1) passes through `end`.
pub(super) fn gann_fan_rays(
    start: Pos2,
    end: Pos2,
    rect: Rect,
    config: &GannConfig,
) -> Vec<(GannLevel, [Pos2; 2])> {
    let from = start.x.min(end.x);
    let to = rect.max.x;
    config
        .fan_angles
        .iter()
        .filter(|l| l.enabled && l.ratio.is_finite() && l.ratio > 0.0)
        .map(|l| {
            let at = |x| Pos2::new(x, gann_fan_line_y(start, end, l.ratio, x));
            (*l, [at(from), at(to)])
        })
        .collect()
}

/// The horizontal and vertical lines of a Gann grid between `p1` and `p2`, each with its
/// level and whether it divides the price side.
fn gann_grid_lines(p1: Pos2, p2: Pos2, levels: &[GannLevel]) -> Vec<(GannLevel, bool, [Pos2; 2])> {
    let rect = Rect::from_two_pos(p1, p2);
    levels
        .iter()
        .filter(|l| l.enabled && (0.0..=1.0).contains(&l.ratio))
        .flat_map(|l| {
            let y = p1.y + (p2.y - p1.y) * l.ratio;
            let x = p1.x + (p2.x - p1.x) * l.ratio;
            [
                (
                    *l,
                    true,
                    [Pos2::new(rect.min.x, y), Pos2::new(rect.max.x, y)],
                ),
                (
                    *l,
                    false,
                    [Pos2::new(x, rect.min.y), Pos2::new(x, rect.max.y)],
                ),
            ]
        })
        .collect()
}

/// The fan lines of a Gann square whose first anchor is `start` and opposite
/// corner `corner`, each with its level: from `start` towards the opposite
/// corner and from the opposite corner back, as the direction selects, up to
/// the square's edge.
fn gann_square_fan_lines(
    start: Pos2,
    corner: Pos2,
    config: &GannConfig,
) -> Vec<(GannLevel, [Pos2; 2])> {
    let (dx, dy) = (corner.x - start.x, corner.y - start.y);
    let mut lines = Vec::new();
    for level in config
        .fan_lines
        .iter()
        .filter(|l| l.enabled && l.ratio.is_finite() && l.ratio > 0.0)
    {
        // The line rises `ratio` per unit of time, so it reaches the far side of the square at
        // `ratio` of its height or, if steeper, the top at `1 / ratio` of its width.
        let (u, v) = if level.ratio <= 1.0 {
            (1.0, level.ratio)
        } else {
            (1.0 / level.ratio, 1.0)
        };
        let offset = egui::vec2(dx * u, dy * v);
        if config.fan_direction != GannFanDirection::Reverse {
            lines.push((*level, [start, start + offset]));
        }
        if config.fan_direction != GannFanDirection::Forward {
            lines.push((*level, [corner, corner - offset]));
        }
    }
    lines
}

/// The arcs of a Gann square around its first anchor `start`, one per enabled
/// level, as fractions of the box's sides, cut to the part inside the box.
fn gann_square_arcs(
    start: Pos2,
    corner: Pos2,
    levels: &[GannLevel],
) -> Vec<(GannLevel, Vec<Pos2>)> {
    let (dx, dy) = (corner.x - start.x, corner.y - start.y);
    levels
        .iter()
        .filter(|l| l.enabled && l.ratio.is_finite() && l.ratio > 0.0)
        .filter_map(|level| {
            let ratio = level.ratio;
            // Both coordinates of the arc stay within the box: cos and sin at most 1 / ratio.
            let (from, to) = if ratio <= 1.0 {
                (0.0, std::f32::consts::FRAC_PI_2)
            } else {
                ((1.0 / ratio).acos(), (1.0 / ratio).asin())
            };
            (from < to).then(|| {
                let points = (0..=64)
                    .map(|i| {
                        let angle = from + (to - from) * i as f32 / 64.0;
                        start + egui::vec2(dx * ratio * angle.cos(), dy * ratio * angle.sin())
                    })
                    .collect();
                (*level, points)
            })
        })
        .collect()
}

/// `ratio` of a grid level as its label, without trailing zeros.
fn grid_label(ratio: f32) -> String {
    let text = format!("{ratio:.3}");
    text.trim_end_matches('0').trim_end_matches('.').to_owned()
}

impl Drawing {
    /// The saved Gann levels, or the defaults of this tool.
    pub(crate) fn gann_config(&self) -> GannConfig {
        self.gann
            .clone()
            .unwrap_or_else(|| GannConfig::for_tool(self.tool_type))
    }

    fn gann_stroke(&self, color: [u8; 4]) -> Stroke {
        Stroke::new(self.stroke_width.max(stroke::HAIRLINE), color32(color))
    }

    fn paint_gann_segment(&self, painter: &egui::Painter, segment: [Pos2; 2], color: [u8; 4]) {
        let stroke = self.gann_stroke(color);
        match self.line_style.dash_pattern() {
            Some((dash, gap)) => {
                draw_dashed_line(painter, segment[0], segment[1], stroke, dash, gap);
            }
            None => {
                painter.line_segment(segment, stroke);
            }
        }
    }

    fn paint_gann_label(
        painter: &egui::Painter,
        pos: Pos2,
        align: Align2,
        text: String,
        color: [u8; 4],
    ) {
        painter.text(
            pos,
            align,
            text,
            FontId::proportional(typography::XS),
            color32(color),
        );
    }

    fn paint_gann_fill(&self, painter: &egui::Painter, rect: Rect) {
        if let Some(fill) = self.fill_color32() {
            painter.rect_filled(rect, 0.0, fill);
        }
    }

    /// The grid of a Gann box or square, its price levels labelled on the left.
    fn paint_gann_grid(&self, painter: &egui::Painter, p1: Pos2, p2: Pos2, config: &GannConfig) {
        let left = p1.x.min(p2.x);
        for (level, price, segment) in gann_grid_lines(p1, p2, &config.levels) {
            self.paint_gann_segment(painter, segment, level.color);
            if price {
                Self::paint_gann_label(
                    painter,
                    Pos2::new(left - LABEL_GAP, segment[0].y),
                    Align2::RIGHT_CENTER,
                    grid_label(level.ratio),
                    level.color,
                );
            }
        }
    }

    /// The grid and both diagonals of a Gann box.
    fn paint_gann_box(&self, painter: &egui::Painter, p1: Pos2, p2: Pos2) {
        let config = self.gann_config();
        let rect = Rect::from_two_pos(p1, p2);
        self.paint_gann_fill(painter, rect);
        self.paint_gann_grid(painter, p1, p2, &config);
        let diagonal = config.diagonal_color();
        self.paint_gann_segment(painter, [rect.left_top(), rect.right_bottom()], diagonal);
        self.paint_gann_segment(painter, [rect.left_bottom(), rect.right_top()], diagonal);
    }

    /// Gann fan: rays from the first anchor at the enabled angles, the 1x1 through the second.
    pub(crate) fn render_gann_fan(&self, painter: &egui::Painter, chart_rect: Rect) {
        let [start, end, ..] = self.points[..] else {
            return;
        };
        let painter = &painter.with_clip_rect(chart_rect);
        let config = self.gann_config();
        for (level, ray) in gann_fan_rays(start, end, chart_rect, &config) {
            self.paint_gann_segment(painter, ray, level.color);
            let label_x = end.x + LABEL_GAP;
            Self::paint_gann_label(
                painter,
                Pos2::new(label_x, gann_fan_line_y(start, end, level.ratio, end.x)),
                Align2::LEFT_CENTER,
                gann_fan_label(level.ratio),
                level.color,
            );
        }
    }

    /// Gann square: a price/time box with its grid, a fan of angles and arcs around the
    /// first anchor.
    pub(crate) fn render_gann_square(&self, painter: &egui::Painter) {
        let [start, end, ..] = self.points[..] else {
            return;
        };
        let config = self.gann_config();
        self.paint_gann_fill(painter, Rect::from_two_pos(start, end));
        if config.show_grid {
            self.paint_gann_grid(painter, start, end, &config);
        }
        if config.show_fan {
            for (level, segment) in gann_square_fan_lines(start, end, &config) {
                self.paint_gann_segment(painter, segment, level.color);
                let outward = (segment[1] - segment[0]).x.signum();
                let align = if outward < 0.0 {
                    Align2::RIGHT_CENTER
                } else {
                    Align2::LEFT_CENTER
                };
                Self::paint_gann_label(
                    painter,
                    segment[1] + egui::vec2(outward * 4.0, 0.0),
                    align,
                    gann_square_fan_label(level.ratio),
                    level.color,
                );
            }
        }
        if config.show_arcs {
            for (level, arc) in gann_square_arcs(start, end, &config.arcs) {
                for pair in arc.windows(2) {
                    self.paint_gann_segment(painter, [pair[0], pair[1]], level.color);
                }
            }
        }
    }

    /// Gann box: the price/time grid between two corners and its diagonals.
    pub(crate) fn render_gann_box(&self, painter: &egui::Painter) {
        let [p1, p2, ..] = self.points[..] else {
            return;
        };
        self.paint_gann_box(painter, p1, p2);
    }

    /// Fixed square: a Gann box that also states its price per bar.
    pub(crate) fn render_gann_fixed(&self, painter: &egui::Painter) {
        let [p1, p2, ..] = self.points[..] else {
            return;
        };
        self.paint_gann_box(painter, p1, p2);

        let rect = Rect::from_two_pos(p1, p2);
        let label = self
            .chart_points
            .first()
            .zip(self.chart_points.get(1))
            .map_or_else(
                || "n/a".to_owned(),
                |(a, b)| {
                    let bars = (b.bar_idx - a.bar_idx).abs();
                    if bars > 0.0 {
                        format!("{:.4}/bar", (b.price - a.price).abs() / f64::from(bars))
                    } else {
                        "n/a".to_owned()
                    }
                },
            );
        let label_bg = DESIGN_TOKENS
            .semantic
            .extended
            .chart_axis_bg
            .gamma_multiply(0.8);
        super::measurements::boxed_label(
            painter,
            Align2::LEFT_TOP,
            Pos2::new(rect.max.x + DESIGN_TOKENS.rounding.lg, rect.min.y),
            label,
            color32(self.gann_config().diagonal_color()),
            label_bg,
        );
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::drawings::domain::DrawingToolType;
    use crate::model::LineStyle;

    fn segments(drawing: &Drawing, rect: Rect) -> Vec<([Pos2; 2], Stroke)> {
        let output = egui::Context::default().run_ui(egui::RawInput::default(), |ui| {
            let painter = ui.painter();
            match drawing.tool_type {
                DrawingToolType::GannFan => drawing.render_gann_fan(painter, rect),
                DrawingToolType::GannSquare => drawing.render_gann_square(painter),
                DrawingToolType::GannBox => drawing.render_gann_box(painter),
                _ => drawing.render_gann_fixed(painter),
            }
        });
        let found = output
            .shapes
            .iter()
            .filter_map(|s| match &s.shape {
                egui::Shape::LineSegment { points, stroke } => Some((*points, *stroke)),
                _ => None,
            })
            .collect();
        output.drop_without_applying_deltas();
        found
    }

    fn near(a: Pos2, b: Pos2) -> bool {
        a.distance(b) < 1e-3
    }

    /// Anchors (0, 100) and (10, 90) put the 1x1 line through the second anchor.
    /// An N/M line moves N price units per M bars, so at x = 10 it sits at
    /// 100 - 10 * N / M (screen y grows downward).
    #[test]
    fn fan_line_slopes_follow_their_labels() {
        let start = Pos2::new(0.0, 100.0);
        let end = Pos2::new(10.0, 90.0);
        let cases = [
            ("8/1", 8.0, 20.0),
            ("4/1", 4.0, 60.0),
            ("3/1", 3.0, 70.0),
            ("2/1", 2.0, 80.0),
            ("1/1", 1.0, 90.0),
            ("1/2", 0.5, 95.0),
            ("1/3", 1.0 / 3.0, 96.6667),
            ("1/4", 0.25, 97.5),
            ("1/8", 0.125, 98.75),
        ];
        for direction in [1.0, -1.0] {
            for (label, ratio, expected) in cases {
                assert_eq!(gann_fan_label(ratio), label);
                let end = Pos2::new(end.x * direction, end.y);
                let y = gann_fan_line_y(start, end, ratio, 10.0 * direction);
                assert!(
                    (y - expected).abs() < 1e-3,
                    "{label} direction {direction}: {y} != {expected}"
                );
            }
        }
    }

    /// Rays run from the nearer anchor to the right pane edge; the 1x1 passes through p2
    /// even when p2 lies left of the origin.
    #[test]
    fn gann_fan_rays_extend_right_and_one_by_one_through_p2() {
        let rect = Rect::from_min_max(Pos2::ZERO, Pos2::new(400.0, 300.0));
        let start = Pos2::new(50.0, 100.0);
        for end in [Pos2::new(100.0, 50.0), Pos2::new(0.0, 150.0)] {
            let rays = gann_fan_rays(start, end, rect, &GannConfig::default());
            assert_eq!(rays.len(), 9);
            let (_, one) = rays.iter().find(|(l, _)| l.ratio == 1.0).unwrap();
            assert_eq!(one[0].x, start.x.min(end.x));
            assert_eq!(one[1].x, rect.max.x, "ray must end at the right edge");
            assert!((gann_fan_line_y(start, end, 1.0, end.x) - end.y).abs() < 1e-3);
        }
    }

    /// Each ray takes its level's colour and the drawing's width and dash; a hidden level is
    /// not drawn and no extra base line joins the anchors.
    #[test]
    fn gann_fan_paints_its_levels_only() {
        let rect = Rect::from_min_max(Pos2::ZERO, Pos2::new(400.0, 300.0));
        let mut drawing = Drawing::new(0, DrawingToolType::GannFan);
        drawing.points = vec![Pos2::new(0.0, 200.0), Pos2::new(100.0, 100.0)];
        let mut config = GannConfig::default();
        config.fan_angles[0].enabled = false;
        config.fan_angles[1].color = [10, 20, 30, 255];
        drawing.gann = Some(config);
        drawing.stroke_width = 3.0;
        let lines = segments(&drawing, rect);
        assert_eq!(lines.len(), 8, "{lines:?}");
        assert!(lines.iter().all(|(_, s)| s.width == 3.0));
        assert!(
            lines
                .iter()
                .any(|(_, s)| s.color == Color32::from_rgb(10, 20, 30))
        );
        assert!(
            !lines
                .iter()
                .any(|(p, _)| near(p[0], drawing.points[0]) && near(p[1], drawing.points[1])),
            "no base line between the anchors"
        );

        drawing.line_style = LineStyle::Dashed;
        assert!(
            segments(&drawing, rect).len() > 8,
            "dashed rays split into dashes"
        );
    }

    /// A box draws both lines of each enabled level in its colour, and its diagonals in the
    /// colour of the 1 level.
    #[test]
    fn gann_box_grid_uses_level_colors_and_diagonals() {
        let rect = Rect::from_min_max(Pos2::ZERO, Pos2::new(400.0, 300.0));
        let mut drawing = Drawing::new(0, DrawingToolType::GannBox);
        drawing.points = vec![Pos2::new(100.0, 0.0), Pos2::new(0.0, 80.0)];
        let mut config = GannConfig::default();
        config.levels[3].enabled = false;
        drawing.gann = Some(config.clone());
        let lines = segments(&drawing, rect);
        assert_eq!(lines.len(), 6 * 2 + 2, "{lines:?}");
        let quarter = color32(config.levels[1].color);
        for want in [
            [Pos2::new(0.0, 20.0), Pos2::new(100.0, 20.0)],
            [Pos2::new(75.0, 0.0), Pos2::new(75.0, 80.0)],
        ] {
            assert!(
                lines
                    .iter()
                    .any(|(p, s)| near(p[0], want[0]) && near(p[1], want[1]) && s.color == quarter),
                "missing {want:?}"
            );
        }
        let diagonal = color32(config.diagonal_color());
        assert_eq!(
            lines
                .iter()
                .filter(|(p, s)| s.color == diagonal && p[0].x != p[1].x && p[0].y != p[1].y)
                .count(),
            2
        );
    }

    /// A Gann square spans its anchors freely: grid, fan and arcs all fit the box they make.
    #[test]
    fn gann_square_follows_its_anchors_and_toggles() {
        let rect = Rect::from_min_max(Pos2::ZERO, Pos2::new(400.0, 300.0));
        let mut drawing = Drawing::new(0, DrawingToolType::GannSquare);
        drawing.points = vec![Pos2::new(0.0, 0.0), Pos2::new(200.0, 100.0)];
        let config = GannConfig::default();
        drawing.gann = Some(config.clone());
        let lines = segments(&drawing, rect);
        let fan = color32(config.fan_lines[0].color);
        assert!(
            lines.iter().any(|(p, s)| s.color == fan
                && near(p[0], Pos2::ZERO)
                && near(p[1], Pos2::new(200.0, 100.0 / 3.0))),
            "the 3x1 line reaches the far side a third of the way up"
        );
        assert!(
            lines
                .iter()
                .all(|(p, _)| p.iter().all(|p| p.x <= 200.001 && p.y <= 100.001)),
            "nothing leaves the box"
        );

        let mut hidden = config;
        hidden.show_grid = false;
        hidden.show_fan = false;
        hidden.show_arcs = false;
        drawing.gann = Some(hidden);
        assert!(segments(&drawing, rect).is_empty());
    }

    #[test]
    fn gann_square_fan_and_arcs_follow_their_settings() {
        // Box from (0, 0) to (100, 100): a fan line of ratio r leaves a corner at r times
        // the diagonal's slope and ends on the opposite side, at (100, 100 r) or (100 / r, 100).
        let config = |fan_direction, ratios: &[f32]| GannConfig {
            fan_direction,
            fan_lines: ratios
                .iter()
                .map(|&r| GannLevel::new(r, [0, 0, 0, 255]))
                .collect(),
            ..GannConfig::default()
        };
        let (start, corner) = (Pos2::new(0.0, 0.0), Pos2::new(100.0, 100.0));
        let ends = |config: &GannConfig| {
            gann_square_fan_lines(start, corner, config)
                .into_iter()
                .map(|(_, line)| line)
                .collect::<Vec<_>>()
        };
        let ratios = [0.5, 2.0, 0.0, f32::NAN, -1.0];
        assert_eq!(
            ends(&config(GannFanDirection::Forward, &ratios)),
            [
                [start, Pos2::new(100.0, 50.0)],
                [start, Pos2::new(50.0, 100.0)]
            ]
        );
        assert_eq!(
            ends(&config(GannFanDirection::Reverse, &ratios)),
            [
                [corner, Pos2::new(0.0, 50.0)],
                [corner, Pos2::new(50.0, 0.0)]
            ]
        );
        assert_eq!(ends(&config(GannFanDirection::Both, &ratios)).len(), 4);

        // The arc of ratio 1 is a quarter circle through (70.7, 70.7); that of 1.2 is cut where
        // it leaves the square, at (100, 66.3) and (66.3, 100); one beyond 1.42 is outside it.
        let levels: Vec<_> = [1.0, 1.2, 1.5, 0.0, f32::NAN]
            .map(|r| GannLevel::new(r, [0, 0, 0, 255]))
            .to_vec();
        let arcs = gann_square_arcs(start, corner, &levels);
        assert_eq!(arcs.len(), 2);
        let near = |a: Pos2, b: Pos2| a.distance(b) < 0.1;
        assert!(
            near(arcs[0].1[0], Pos2::new(100.0, 0.0))
                && near(*arcs[0].1.last().unwrap(), Pos2::new(0.0, 100.0))
        );
        assert!(arcs[0].1.iter().any(|&p| near(p, Pos2::new(70.71, 70.71))));
        assert!(
            near(arcs[1].1[0], Pos2::new(100.0, 66.33))
                && near(*arcs[1].1.last().unwrap(), Pos2::new(66.33, 100.0))
        );
    }

    /// Square arcs stay quarter-ellipses of box deltas in every quadrant.
    #[test]
    fn gann_square_arcs_flip_with_quadrant() {
        let start = Pos2::new(50.0, 50.0);
        for corner in [
            Pos2::new(150.0, 150.0),
            Pos2::new(-50.0, 150.0),
            Pos2::new(150.0, -50.0),
            Pos2::new(-50.0, -50.0),
        ] {
            let arcs = gann_square_arcs(start, corner, &[GannLevel::new(1.0, [0; 4])]);
            assert_eq!(arcs.len(), 1, "{corner:?}");
            let (dx, dy) = (corner.x - start.x, corner.y - start.y);
            let first = arcs[0].1[0];
            let last = *arcs[0].1.last().unwrap();
            assert!(
                (first - Pos2::new(start.x + dx, start.y)).length() < 0.1
                    && (last - Pos2::new(start.x, start.y + dy)).length() < 0.1,
                "{corner:?}: {first:?} .. {last:?}"
            );
        }
    }

    /// The fixed square divides into quarters by default and keeps saved divisions.
    #[test]
    fn gann_fixed_grid_keeps_quarters_and_custom_divisions() {
        let rect = Rect::from_min_max(Pos2::ZERO, Pos2::new(400.0, 300.0));
        let mut drawing = Drawing::new(0, DrawingToolType::GannFixed);
        drawing.points = vec![Pos2::new(20.0, 200.0), Pos2::new(120.0, 100.0)];
        let verticals = |drawing: &Drawing| {
            segments(drawing, rect)
                .iter()
                .filter(|(p, _)| p[0].x == p[1].x && p[0].x > 20.0 && p[0].x < 120.0)
                .count()
        };
        assert_eq!(verticals(&drawing), 3);

        let mut saved = serde_json::to_value(&drawing).unwrap();
        saved["gann"] = serde_json::json!({"price_levels": [0.3_f32], "time_levels": [0.7_f32]});
        let mut restored: Drawing = serde_json::from_value(saved).unwrap();
        restored.points = drawing.points.clone();
        let lines = segments(&restored, rect);
        for want in [
            [Pos2::new(90.0, 100.0), Pos2::new(90.0, 200.0)],
            [Pos2::new(20.0, 170.0), Pos2::new(120.0, 170.0)],
        ] {
            assert!(
                lines
                    .iter()
                    .any(|(p, _)| near(p[0], want[0]) && near(p[1], want[1])),
                "missing {want:?}: {lines:?}"
            );
        }
    }
}
