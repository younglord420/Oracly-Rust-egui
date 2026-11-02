//! Pitchfork tool rendering implementations
//!
//! Includes: standard pitchfork, modified Schiff, inside pitchfork, and pitchfan.

use crate::drawings::domain::{Drawing, DrawingToolType};
use crate::drawings::validation::MAX_PITCHFORK_LEVEL;
use crate::styles::{stroke, typography};
use crate::tokens::DESIGN_TOKENS;
use egui::{Color32, Pos2, Rect, Stroke};

impl Drawing {
    /// Standard and Schiff pitchfork
    pub(crate) fn render_pitchfork(&self, painter: &egui::Painter, chart_rect: Rect) {
        let color = Color32::from_rgba_unmultiplied(
            self.color[0],
            self.color[1],
            self.color[2],
            self.color[3],
        );

        // Need at least 3 points for pitchfork
        if self.points.len() < 3 {
            // Draw points we have so far
            for (i, &point) in self.points.iter().enumerate() {
                painter.circle_filled(point, DESIGN_TOKENS.rounding.sm, color);
                if i > 0 {
                    painter.line_segment(
                        [self.points[i - 1], point],
                        Stroke::new(stroke::HAIRLINE, color),
                    );
                }
            }
            return;
        }

        self.draw_fork(painter, chart_rect);
    }

    pub(crate) fn render_modified_schiff_pitchfork(
        &self,
        painter: &egui::Painter,
        chart_rect: Rect,
    ) {
        // Modified Schiff moves the anchor point to midway between p0 and p1
        if self.points.len() < 3 {
            for &p in &self.points {
                painter.circle_filled(
                    p,
                    DESIGN_TOKENS.rounding.sm,
                    Color32::from_rgba_unmultiplied(
                        self.color[0],
                        self.color[1],
                        self.color[2],
                        self.color[3],
                    ),
                );
            }
            return;
        }

        self.draw_fork(painter, chart_rect);
    }

    pub(crate) fn render_inside_pitchfork(&self, painter: &egui::Painter, chart_rect: Rect) {
        // Inside uses the first leg midpoint as one tine, and the second anchor as the other.
        if self.points.len() < 3 {
            for &p in &self.points {
                painter.circle_filled(
                    p,
                    DESIGN_TOKENS.rounding.sm,
                    Color32::from_rgba_unmultiplied(
                        self.color[0],
                        self.color[1],
                        self.color[2],
                        self.color[3],
                    ),
                );
            }
            return;
        }

        self.draw_fork(painter, chart_rect);
    }

    /// Median handle, the point the median crosses the base, the two tine anchors, and the
    /// construction links. `None` below three anchors.
    fn fork_parts(&self) -> Option<ForkParts> {
        let &[p0, p1, p2, ..] = self.points.as_slice() else {
            return None;
        };
        let mid = |a: Pos2, b: Pos2| a + (b - a) / 2.0;
        let (origin, through, upper, lower, links) = match self.tool_type {
            DrawingToolType::Pitchfork | DrawingToolType::SchiffPitchfork => {
                let schiff = self.tool_type == DrawingToolType::SchiffPitchfork;
                let (origin, through) = median_line(p0, p1, p2, schiff);
                (origin, through, p1, p2, vec![[p0, p1], [p0, p2], [p1, p2]])
            }
            DrawingToolType::ModifiedSchiffPitchfork => {
                (mid(p0, p1), mid(p1, p2), p1, p2, vec![[p0, p1], [p1, p2]])
            }
            DrawingToolType::InsidePitchfork => {
                // Median starts halfway from the handle to the base midpoint and runs through
                // that midpoint. The tines pass through the second and third anchors.
                let through = mid(p1, p2);
                (mid(p0, through), through, p1, p2, vec![[p0, p1], [p1, p2]])
            }
            _ => return None,
        };
        Some(ForkParts {
            origin,
            through,
            upper,
            lower,
            links,
        })
    }

    /// The pitchfork's median and parallels, extended as configured, then the construction links.
    fn pitchfork_base_lines(&self, rect: Rect) -> Option<Vec<[Pos2; 2]>> {
        let parts = self.fork_parts()?;
        let mut lines = fork_lines(
            self,
            parts.origin,
            parts.through,
            parts.upper,
            parts.lower,
            rect,
        )
        .to_vec();
        lines.extend(parts.links);
        Some(lines)
    }

    /// All visible level lines, followed by the construction links, shared with selection.
    pub(super) fn pitchfork_lines(&self, rect: Rect) -> Option<Vec<[Pos2; 2]>> {
        let base = self.pitchfork_base_lines(rect)?;
        let config = self.pitchfork.clone().unwrap_or_default();
        let mut lines = Vec::new();
        for level in config
            .levels
            .iter()
            .filter(|l| l.visible && (0.0..=MAX_PITCHFORK_LEVEL).contains(&l.value))
        {
            lines.extend(self.level_lines(level.value, rect));
        }
        lines.extend_from_slice(&base[3..]);
        Some(lines)
    }

    /// Bands fill between consecutive visible levels (sorted by value).
    pub(super) fn pitchfork_bands(&self, rect: Rect) -> Vec<Vec<Pos2>> {
        if self.fork_parts().is_none() {
            return Vec::new();
        }
        let mut config = self.pitchfork.clone().unwrap_or_default();
        if !config.show_background {
            return Vec::new();
        }
        let visible = visible_pitchfork_levels(&mut config);
        let mut bands = Vec::new();
        for pair in visible.windows(2) {
            let inner = self.level_lines(pair[0].value, rect);
            let outer = self.level_lines(pair[1].value, rect);
            for (side, outside) in outer.iter().enumerate() {
                let inside = inner[side.min(inner.len() - 1)];
                bands.push(vec![inside[0], inside[1], outside[1], outside[0]]);
            }
        }
        bands
    }

    /// Paints the levels, band fill, construction links and anchors.
    fn draw_fork(&self, painter: &egui::Painter, rect: Rect) {
        let Some(base) = self.pitchfork_base_lines(rect) else {
            return;
        };
        let mut config = self.pitchfork.clone().unwrap_or_default();
        let opacity = config.background_opacity;
        let visible = visible_pitchfork_levels(&mut config);
        let mut bands = self.pitchfork_bands(rect).into_iter();
        for pair in visible.windows(2) {
            let color = if self.pitchfork.is_some() {
                pair[1].color
            } else {
                self.color
            };
            let fill = Color32::from_rgba_unmultiplied(color[0], color[1], color[2], opacity);
            for band in bands
                .by_ref()
                .take(self.level_lines(pair[1].value, rect).len())
            {
                painter.add(egui::Shape::convex_polygon(band, fill, Stroke::NONE));
            }
        }
        for level in &visible {
            let rgba = if self.pitchfork.is_none() {
                self.color
            } else {
                level.color
            };
            let color = Color32::from_rgba_unmultiplied(rgba[0], rgba[1], rgba[2], rgba[3]);
            let width = level.width.unwrap_or(self.stroke_width);
            if !(width > 0.0 && width <= crate::drawings::validation::MAX_STROKE_WIDTH) {
                continue;
            }
            for [a, b] in self.level_lines(level.value, rect) {
                if let Some((dash, gap)) = level.line_style.dash_pattern() {
                    super::utils::draw_dashed_line(
                        painter,
                        a,
                        b,
                        Stroke::new(width, color),
                        dash,
                        gap,
                    );
                } else {
                    painter.line_segment([a, b], Stroke::new(width, color));
                }
            }
        }
        for &line in &base[3..] {
            painter.line_segment(
                line,
                Stroke::new(stroke::HAIRLINE / 2.0, self.color32().gamma_multiply(0.4)),
            );
        }
    }

    pub(crate) fn render_pitchfan(&self, painter: &egui::Painter, rect: Rect) {
        let Some(&center) = self.points.first() else {
            return;
        };
        let color = self.color32();
        if self.points.len() < 3 {
            painter.circle_filled(center, DESIGN_TOKENS.rounding.md, color);
            if let Some(&end) = self.points.get(1) {
                painter.line_segment(
                    [center, end],
                    Stroke::new(self.stroke_width.max(stroke::HAIRLINE), color),
                );
                painter.circle_filled(end, DESIGN_TOKENS.rounding.sm, color);
            }
            return;
        }
        let boundary = self.points[2];
        let end = self.points[1];
        for level in self.pitchfan_levels() {
            let [_, fan_end] = pitchfan_ray(center, end, boundary, level.value, rect);
            let rgba = level.color;
            let line_color = Color32::from_rgba_unmultiplied(rgba[0], rgba[1], rgba[2], rgba[3]);
            let width = level.width.unwrap_or(self.stroke_width);
            if !(width > 0.0 && width <= crate::drawings::validation::MAX_STROKE_WIDTH) {
                continue;
            }
            let stroke = Stroke::new(width, line_color);
            if let Some((dash, gap)) = level.line_style.dash_pattern() {
                super::utils::draw_dashed_line(painter, center, fan_end, stroke, dash, gap);
            } else {
                painter.line_segment([center, fan_end], stroke);
            }
            if level.value > 0.0 {
                let run = fan_end - center;
                let t = if fan_end.y < rect.min.y {
                    (rect.min.y - center.y) / run.y
                } else if fan_end.y > rect.max.y {
                    (rect.max.y - center.y) / run.y
                } else {
                    1.0
                };
                painter.text(
                    center + run * t,
                    if fan_end.x >= center.x {
                        egui::Align2::RIGHT_BOTTOM
                    } else {
                        egui::Align2::LEFT_BOTTOM
                    },
                    format!("{:.1}%", level.value * 100.0),
                    egui::FontId::proportional(typography::XS),
                    line_color,
                );
            }
        }
        painter.circle_filled(center, DESIGN_TOKENS.rounding.md, color);
        painter.circle_filled(end, DESIGN_TOKENS.rounding.sm, color);
    }

    fn pitchfan_levels(&self) -> Vec<crate::drawings::PitchforkLevel> {
        let mut config = self
            .pitchfork
            .clone()
            .unwrap_or_else(crate::drawings::PitchforkConfig::pitchfan);
        config
            .levels
            .retain(|level| level.visible && (0.0..=MAX_PITCHFORK_LEVEL).contains(&level.value));
        config.levels.sort_by(|a, b| a.value.total_cmp(&b.value));
        config.levels
    }
}

struct ForkParts {
    origin: Pos2,
    through: Pos2,
    upper: Pos2,
    lower: Pos2,
    links: Vec<[Pos2; 2]>,
}

/// Visible levels in value order, dropping out-of-range entries.
fn visible_pitchfork_levels(
    config: &mut crate::drawings::PitchforkConfig,
) -> Vec<crate::drawings::PitchforkLevel> {
    config
        .levels
        .retain(|l| (0.0..=MAX_PITCHFORK_LEVEL).contains(&l.value));
    config.levels.sort_by(|a, b| a.value.total_cmp(&b.value));
    config
        .levels
        .iter()
        .filter(|l| l.visible)
        .cloned()
        .collect()
}

impl Drawing {
    /// A level starts on the base between the tines and runs parallel to the median.
    /// The median itself still starts at its handle.
    fn level_lines(&self, value: f32, rect: Rect) -> Vec<[Pos2; 2]> {
        let Some(ForkParts {
            origin,
            through,
            upper,
            lower,
            ..
        }) = self.fork_parts()
        else {
            return Vec::new();
        };
        let (back, forward) = self.pitchfork_reach();
        let direction = through - origin;
        if value == 0.0 {
            return vec![pitched_line(origin, direction, rect, back, forward)];
        }
        [upper, lower]
            .into_iter()
            .map(|anchor| {
                let start = through + (anchor - through) * value;
                pitched_line(start, direction, rect, back, forward)
            })
            .collect()
    }

    fn pitchfork_reach(&self) -> (bool, bool) {
        (self.extend_left, self.extend_right)
    }
}

/// Rays from the first pivot through fractional points on the second-to-third leg.
pub(super) fn pitchfan_lines(drawing: &Drawing, rect: Rect) -> Vec<[Pos2; 2]> {
    match drawing.points.as_slice() {
        &[center, end] => vec![[center, end]],
        &[center, end, boundary, ..] => drawing
            .pitchfan_levels()
            .into_iter()
            .map(|level| pitchfan_ray(center, end, boundary, level.value, rect))
            .collect(),
        _ => Vec::new(),
    }
}

fn pitchfan_ray(center: Pos2, end: Pos2, boundary: Pos2, ratio: f32, rect: Rect) -> [Pos2; 2] {
    [
        center,
        super::lines::ray_end(center, end.lerp(boundary, ratio), rect),
    ]
}

/// Median from `origin` through `through`, and the parallels from the tine anchors,
/// each extended the way the drawing asks.
fn fork_lines(
    drawing: &Drawing,
    origin: Pos2,
    through: Pos2,
    p1: Pos2,
    p2: Pos2,
    rect: Rect,
) -> [[Pos2; 2]; 3] {
    let (back, forward) = drawing.pitchfork_reach();
    let direction = through - origin;
    [origin, p1, p2].map(|start| pitched_line(start, direction, rect, back, forward))
}

/// `direction` is the unextended median. Forward runs with it; back runs against it.
fn pitched_line(
    start: Pos2,
    direction: egui::Vec2,
    rect: Rect,
    back: bool,
    forward: bool,
) -> [Pos2; 2] {
    if direction.length_sq() < 1e-8 {
        return [start, start];
    }
    let end = if forward {
        super::lines::ray_end(start, start + direction, rect)
    } else {
        start + direction
    };
    let begin = if back {
        super::lines::ray_end(end, start, rect)
    } else {
        start
    };
    [begin, end]
}

/// Origin of the median line and the point it passes through, the midpoint of the
/// second and third anchors. The Schiff variant keeps the first anchor's time and
/// moves its price halfway to the second anchor.
fn median_line(p0: Pos2, p1: Pos2, p2: Pos2, is_schiff: bool) -> (Pos2, Pos2) {
    let origin = if is_schiff {
        Pos2::new(p0.x, (p0.y + p1.y) / 2.0)
    } else {
        p0
    };
    (origin, p1.lerp(p2, 0.5))
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Reference: anchors (0, 0), (10, 100), (20, -50). Both medians run through
    /// (15, 25), the midpoint of the second and third anchors. The standard median
    /// starts at the first anchor (slope 5/3); the Schiff median starts at (0, 50),
    /// halfway up the first leg (slope -5/3).
    #[test]
    fn median_line_matches_construction() {
        let anchors = [
            Pos2::new(0.0, 0.0),
            Pos2::new(10.0, 100.0),
            Pos2::new(20.0, -50.0),
        ];
        for (is_schiff, origin, through) in [
            (false, Pos2::new(0.0, 0.0), Pos2::new(15.0, 25.0)),
            (true, Pos2::new(0.0, 50.0), Pos2::new(15.0, 25.0)),
        ] {
            let got = median_line(anchors[0], anchors[1], anchors[2], is_schiff);
            assert!(
                got.0.distance(origin) < 1e-4 && got.1.distance(through) < 1e-4,
                "schiff {is_schiff}: got {got:?}, want ({origin:?}, {through:?})"
            );
        }
        // Inside: median from the midpoint of A and (B+C)/2, which is (7.5, 12.5),
        // through (15, 25). That is the same slope as the standard median, 5/3,
        // and the tines pass through B and C.
        for (tool, origin, slope) in [
            (DrawingToolType::Pitchfork, Pos2::new(0.0, 0.0), 5.0 / 3.0),
            (
                DrawingToolType::SchiffPitchfork,
                Pos2::new(0.0, 50.0),
                -5.0 / 3.0,
            ),
            (
                DrawingToolType::ModifiedSchiffPitchfork,
                Pos2::new(5.0, 50.0),
                -2.5,
            ),
            (
                DrawingToolType::InsidePitchfork,
                Pos2::new(7.5, 12.5),
                5.0 / 3.0,
            ),
        ] {
            let mut drawing = Drawing::new(0, tool);
            drawing.points = anchors.to_vec();
            let lines = drawing
                .pitchfork_base_lines(Rect::from_min_max(
                    Pos2::new(-100.0, -1000.0),
                    Pos2::new(100.0, 1000.0),
                ))
                .unwrap();
            assert!(
                lines[0][0].distance(origin) < 1e-4,
                "{tool:?}: {:?}",
                lines[0]
            );
            for [a, b] in &lines[..3] {
                assert!(
                    ((b.y - a.y) / (b.x - a.x) - slope).abs() < 1e-4,
                    "{tool:?}: {a:?} {b:?}"
                );
            }
        }
    }
    #[test]
    fn pitchfork_levels_are_mirrored_filled_saved_and_selectable() {
        let rect = Rect::from_min_max(Pos2::new(-100.0, -2000.0), Pos2::new(400.0, 2000.0));
        // Default visible levels are 0, 0.5 and 1.0. At x=40: median, half, then tines.
        for (tool, ys) in [
            (
                DrawingToolType::Pitchfork,
                [66.6667, 108.3333, 25.0, 150.0, -16.6667],
            ),
            (
                DrawingToolType::SchiffPitchfork,
                [-16.6667, 16.6667, -50.0, 50.0, -83.3333],
            ),
            (
                DrawingToolType::ModifiedSchiffPitchfork,
                [-37.5, -6.25, -68.75, 25.0, -100.0],
            ),
            (
                DrawingToolType::InsidePitchfork,
                [66.6667, 108.3333, 25.0, 150.0, -16.6667],
            ),
        ] {
            let mut drawing = Drawing::new(12, tool);
            drawing.points = vec![Pos2::ZERO, Pos2::new(10.0, 100.0), Pos2::new(20.0, -50.0)];
            let lines = drawing.pitchfork_lines(rect).unwrap();
            assert!(lines.len() >= 5, "{tool:?}: {lines:?}");
            for ([a, b], want) in lines.iter().take(5).zip(ys) {
                let y = a.y + (40.0 - a.x) * (b.y - a.y) / (b.x - a.x);
                assert!((y - want).abs() < 1e-3, "{tool:?}: {y} != {want}");
            }
            let output = egui::Context::default().run_ui(egui::RawInput::default(), |ui| {
                drawing.draw_fork(ui.painter(), rect)
            });
            let first_color = output.shapes.iter().find_map(|s| {
                if let egui::Shape::LineSegment { stroke, .. } = &s.shape {
                    Some(stroke.color)
                } else {
                    None
                }
            });
            output.drop_without_applying_deltas();
            // With no stored pitchfork config, level strokes inherit the drawing colour.
            assert_eq!(
                first_color,
                Some(drawing.color32()),
                "default levels inherit the drawing colour"
            );
            let outline = drawing.outline(rect).unwrap();
            // Bands 0→0.5 and 0.5→1: two pairs × two sides.
            assert_eq!(outline.areas.len(), 4);
            let point = Pos2::new(40.0, ys[3]);
            assert_eq!(
                crate::drawings::services::DrawingInteraction::new().hit_test(
                    point,
                    &[drawing.clone()],
                    "1m"
                ),
                Some(12)
            );

            // Inject custom settings through the saved form so old files and new fields both use serde.
            let mut saved = serde_json::to_value(&drawing).unwrap();
            let mut config = serde_json::json!({"levels":[
                {"value":0.0,"visible":true,"color":[10,20,30,255],"line_style":"Dotted"},
                {"value":0.5,"visible":false,"color":[10,20,30,255],"line_style":"Dashed"},
                {"value":1.0,"visible":true,"color":[10,20,30,255],"line_style":"Solid"},
                {"value":1.5,"visible":true,"color":[10,20,30,255],"line_style":"Solid"}
            ],"show_background":true,"background_opacity":35});
            saved["pitchfork"] = config.clone();
            let mut restored: Drawing = serde_json::from_value(saved.clone()).unwrap();
            for level in config["levels"].as_array_mut().unwrap() {
                level["width"] = serde_json::Value::Null;
            }
            assert_eq!(
                serde_json::to_value(&restored).unwrap()["pitchfork"],
                config
            );
            restored.points = drawing.points.clone();
            // Visible 0 → 1.0 → 1.5: two band pairs × two sides.
            assert_eq!(restored.outline(rect).unwrap().areas.len(), 4);
            // Custom: 0+1+1.5 = 5 level lines; default: 0+0.5+1 = 5 level lines → same count + links.
            assert_eq!(restored.pitchfork_lines(rect).unwrap().len(), lines.len());
            // Missing widths inherit the drawing width; explicit widths survive saving and painting.
            saved["pitchfork"]["levels"][2]["width"] = 6.0.into();
            saved["pitchfork"]["levels"][3]["width"] = 3.0.into();
            let mut wide: Drawing = serde_json::from_value(saved.clone()).unwrap();
            wide.points = drawing.points.clone();
            let mut output = egui::Context::default().run_ui(egui::RawInput::default(), |ui| {
                wide.draw_fork(ui.painter(), rect)
            });
            output.textures_delta.clear();
            for width in [3.0, 6.0] {
                assert!(
                    output.shapes.iter().any(|s| matches!(&s.shape,
                    egui::Shape::LineSegment {stroke,..} if stroke.width==width)),
                    "{tool:?}: level width {width}"
                );
            }
            output.drop_without_applying_deltas();
            saved["pitchfork"]["show_background"] = false.into();
            let mut unfilled: Drawing = serde_json::from_value(saved.clone()).unwrap();
            unfilled.points = drawing.points.clone();
            assert!(unfilled.outline(rect).unwrap().areas.is_empty());
            saved.as_object_mut().unwrap().remove("pitchfork");
            let mut legacy: Drawing = serde_json::from_value(saved).unwrap();
            legacy.points = drawing.points;
            assert_eq!(legacy.pitchfork_lines(rect).unwrap(), lines);
        }
    }
}
