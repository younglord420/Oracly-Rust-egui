//! Chart pattern rendering implementations
//!
//! Includes: XABCD and the named harmonic patterns (Gartley, Bat, Butterfly, Crab, Shark,
//! Cypher), Head & Shoulders, ABCD, Triangle, and Three Drives patterns.

use crate::drawings::domain::{Drawing, DrawingToolType, PatternConfig, WaveLabel, harmonic_rules};
use crate::drawings::rendering::ChartView;
use crate::styles::stroke;
use crate::tokens::DESIGN_TOKENS;
use egui::{Align2, Color32, Pos2, Rect, Stroke};

/// The extension of the second correction, as the least and the greatest ratio, in which the third
/// drive is expected to end.
const THREE_DRIVES_EXTENSION: [f64; 2] = [1.272, 1.618];

/// Gap between an anchor and its label.
const LABEL_GAP: f32 = 6.0;

/// Chord ends of a ratio of `leg` over `base`: the outermost anchors of the two legs.
fn chord(leg: [usize; 2], base: [usize; 2]) -> [usize; 2] {
    let all = [leg[0], leg[1], base[0], base[1]];
    [
        *all.iter().min().unwrap_or(&0),
        *all.iter().max().unwrap_or(&0),
    ]
}

/// Whether the anchor at `i` is a peak on screen: above the middle of its neighbours.
fn is_peak(points: &[Pos2], i: usize) -> bool {
    let p = points[i];
    let before = i.checked_sub(1).and_then(|j| points.get(j)).unwrap_or(&p).y;
    let after = points.get(i + 1).unwrap_or(&p).y;
    p.y <= (before + after) / 2.0
}

/// Display name of a named harmonic pattern.
fn harmonic_name(tool: DrawingToolType) -> &'static str {
    match tool {
        DrawingToolType::GartleyPattern => "Gartley",
        DrawingToolType::BatPattern => "Bat",
        DrawingToolType::ButterflyPattern => "Butterfly",
        DrawingToolType::CrabPattern => "Crab",
        DrawingToolType::SharkPattern => "Shark",
        DrawingToolType::CypherPattern => "Cypher",
        _ => "",
    }
}

impl Drawing {
    /// Saved pattern options, or the defaults.
    pub(crate) fn pattern_config(&self) -> PatternConfig {
        self.pattern.unwrap_or_default()
    }

    /// The pattern's legs between consecutive anchors.
    pub(super) fn paint_pattern_legs(&self, painter: &egui::Painter) {
        let stroke = Stroke::new(self.stroke_width, self.color32());
        for leg in self.points.windows(2) {
            painter.line_segment([leg[0], leg[1]], stroke);
        }
    }

    /// Fills `polygon` with the drawing's fill colour, when it has one.
    pub(super) fn paint_pattern_fill(&self, painter: &egui::Painter, polygon: Vec<Pos2>) {
        if polygon.len() >= 3
            && let Some(fill) = self.fill_color32()
        {
            painter.add(egui::epaint::PathShape::convex_polygon(
                polygon,
                fill,
                Stroke::NONE,
            ));
        }
    }

    /// Labels each anchor that has one, above a peak and below a trough, unless labels are hidden.
    pub(super) fn paint_anchor_labels(&self, painter: &egui::Painter, labels: &[WaveLabel]) {
        if !self.pattern_config().show_labels {
            return;
        }
        let color = self.color32();
        for (i, label) in labels.iter().enumerate().take(self.points.len()) {
            if label.text.is_empty() {
                continue;
            }
            let point = self.points[i];
            let galley = painter.layout_no_wrap(label.text.clone(), self.annotation_font(), color);
            let size = galley.size();
            let ring = if label.circled {
                size.max_elem() / 2.0 + 3.0
            } else {
                0.0
            };
            let offset = LABEL_GAP + ring + size.y / 2.0;
            let center = if is_peak(&self.points, i) {
                point - egui::vec2(0.0, offset)
            } else {
                point + egui::vec2(0.0, offset)
            };
            painter.galley(center - size / 2.0, galley, color);
            if label.circled {
                painter.circle_stroke(center, ring, Stroke::new(stroke::HAIRLINE, color));
            }
        }
    }

    /// Draws a ratio on the dashed chord joining the outside ends of its two legs.
    fn draw_pattern_ratio(
        &self,
        painter: &egui::Painter,
        [from, to]: [usize; 2],
        ratio: Option<f64>,
        text_color: Color32,
    ) {
        let (Some(&a), Some(&b)) = (self.points.get(from), self.points.get(to)) else {
            return;
        };
        super::utils::draw_dashed_line(
            painter,
            a,
            b,
            Stroke::new(stroke::HAIRLINE, self.color32().gamma_multiply(0.4)),
            DESIGN_TOKENS.spacing.sm,
            DESIGN_TOKENS.spacing.xs + DESIGN_TOKENS.spacing.hairline,
        );
        let Some(ratio) = ratio else {
            return;
        };
        let galley =
            painter.layout_no_wrap(format!("{ratio:.3}"), self.annotation_font(), text_color);
        let at = |t| Align2::CENTER_BOTTOM.anchor_size(a.lerp(b, t), galley.size());
        let rect = [0.5, 0.4, 0.6, 0.3, 0.7]
            .into_iter()
            .map(at)
            .find(|r| {
                !self
                    .points
                    .iter()
                    .any(|p| r.intersects(Rect::from_center_size(*p, egui::vec2(24.0, 48.0))))
            })
            .unwrap_or_else(|| at(0.5));
        painter.galley(rect.min, galley, text_color);
    }

    /// XABCD and the named harmonic patterns: the X-A-B-C-D legs over the XAB and BCD bodies,
    /// each ratio on its chord and, for a named pattern, its verdict at D.
    pub(crate) fn render_harmonic_pattern(&self, painter: &egui::Painter) {
        let config = self.pattern_config();
        if self.points.len() >= 5 {
            self.paint_pattern_fill(painter, self.points[0..3].to_vec());
            self.paint_pattern_fill(painter, self.points[2..5].to_vec());
        }
        self.paint_pattern_legs(painter);
        self.paint_anchor_labels(painter, &plain_labels(&["X", "A", "B", "C", "D"]));

        let color = self.color32();
        let theme = &DESIGN_TOKENS.semantic.extended;
        let Some(rules) = harmonic_rules(self.tool_type) else {
            if config.show_ratios {
                for (ends, ratio) in [
                    ([0, 2], self.leg_ratio([1, 2], [0, 1])),
                    ([1, 3], self.leg_ratio([2, 3], [1, 2])),
                    ([2, 4], self.leg_ratio([3, 4], [2, 3])),
                    ([0, 4], self.leg_ratio([1, 4], [0, 1])),
                ] {
                    self.draw_pattern_ratio(painter, ends, ratio, color);
                }
            }
            return;
        };
        let verdicts: Vec<_> = rules
            .iter()
            .map(|r| self.leg_ratio(r.leg, r.base).map(|v| r.band.contains(v)))
            .collect();
        if config.show_ratios {
            for (rule, verdict) in rules.iter().zip(&verdicts) {
                let text_color = match (config.show_validity, verdict) {
                    (true, Some(true)) => theme.bullish,
                    (true, Some(false)) => theme.bearish,
                    _ => color,
                };
                self.draw_pattern_ratio(
                    painter,
                    chord(rule.leg, rule.base),
                    self.leg_ratio(rule.leg, rule.base),
                    text_color,
                );
            }
        }
        if config.show_validity && self.points.len() >= 5 {
            let valid = verdicts.iter().all(|v| *v == Some(true));
            self.paint_harmonic_verdict(painter, valid);
        }
    }

    /// The pattern's name and a check or cross beyond D's label.
    fn paint_harmonic_verdict(&self, painter: &egui::Painter, valid: bool) {
        let theme = &DESIGN_TOKENS.semantic.extended;
        let color = if valid { theme.bullish } else { theme.bearish };
        let d = self.points[4];
        let galley = painter.layout_no_wrap(
            harmonic_name(self.tool_type).to_owned(),
            self.annotation_font(),
            color,
        );
        let size = galley.size();
        let mark = size.y * 0.6;
        let width = size.x + 4.0 + mark;
        // Beyond the D label, on the same side.
        let offset = LABEL_GAP * 2.0 + size.y * 1.5;
        let center = if is_peak(&self.points, 4) {
            d - egui::vec2(0.0, offset)
        } else {
            d + egui::vec2(0.0, offset)
        };
        let left = center.x - width / 2.0;
        painter.galley(Pos2::new(left, center.y - size.y / 2.0), galley, color);
        let m = Rect::from_center_size(
            Pos2::new(left + size.x + 4.0 + mark / 2.0, center.y),
            egui::Vec2::splat(mark),
        );
        let stroke = Stroke::new(1.5, color);
        if valid {
            painter.line_segment([m.left_center(), m.center_bottom()], stroke);
            painter.line_segment([m.center_bottom(), m.right_top()], stroke);
        } else {
            painter.line_segment([m.left_top(), m.right_bottom()], stroke);
            painter.line_segment([m.left_bottom(), m.right_top()], stroke);
        }
    }

    /// Head and Shoulders: the seven anchors, the neckline through the troughs and the target.
    pub(crate) fn render_head_and_shoulders(&self, painter: &egui::Painter) {
        let config = self.pattern_config();
        if self.points.len() >= 5 {
            self.paint_pattern_fill(painter, self.points.clone());
        }
        self.paint_pattern_legs(painter);
        self.paint_anchor_labels(painter, &plain_labels(&["", "LS", "", "H", "", "RS", ""]));

        let Some((neckline, target_y)) = self.neckline() else {
            return;
        };
        let color = self.color32();
        if config.show_neckline {
            painter.line_segment(neckline, Stroke::new(self.stroke_width, color));
        }
        if config.show_target {
            let last_x = self.points[self.points.len() - 1].x;
            let end = last_x + DESIGN_TOKENS.sizing.technical_labels.fib_label_offset_x;
            super::utils::draw_dashed_line(
                painter,
                Pos2::new(last_x, target_y),
                Pos2::new(end, target_y),
                Stroke::new(stroke::HAIRLINE, color),
                DESIGN_TOKENS.spacing.md + DESIGN_TOKENS.spacing.xs,
                DESIGN_TOKENS.spacing.sm,
            );
            painter.text(
                Pos2::new(end + LABEL_GAP, target_y),
                Align2::LEFT_CENTER,
                "Target",
                self.annotation_font(),
                color,
            );
        }
    }

    /// ABCD: the A-B-C-D legs, with C's retracement of AB and D's extension of BC.
    pub(crate) fn render_abcd_pattern(&self, painter: &egui::Painter) {
        if self.points.len() >= 4 {
            self.paint_pattern_fill(painter, self.points[0..4].to_vec());
        }
        self.paint_pattern_legs(painter);
        self.paint_anchor_labels(painter, &plain_labels(&["A", "B", "C", "D"]));
        if self.pattern_config().show_ratios {
            let color = self.color32();
            self.draw_pattern_ratio(painter, [0, 2], self.leg_ratio([1, 2], [0, 1]), color);
            self.draw_pattern_ratio(painter, [1, 3], self.leg_ratio([2, 3], [1, 2]), color);
        }
    }

    /// Triangle: the zigzag between two boundaries, run on dashed to where they meet.
    pub(crate) fn render_triangle_pattern(&self, painter: &egui::Painter) {
        let color = self.color32();
        let boundaries = triangle_boundaries(&self.points, painter.clip_rect().right());
        if self.points.len() == 3 {
            self.paint_pattern_fill(painter, self.points.clone());
        } else if let Some((_, fill)) = &boundaries {
            self.paint_pattern_fill(painter, fill.clone());
        }
        self.paint_pattern_legs(painter);
        if self.points.len() == 3 {
            painter.line_segment(
                [self.points[2], self.points[0]],
                Stroke::new(self.stroke_width, color),
            );
        }
        if let Some((lines, _)) = &boundaries {
            let extend = color.gamma_multiply(0.3);
            for (line, last) in lines.iter().zip([self.points[2], self.points[3]]) {
                painter.line_segment([line[0], last], Stroke::new(self.stroke_width, color));
                if line[1].x > last.x {
                    super::utils::draw_dashed_line(
                        painter,
                        last,
                        line[1],
                        Stroke::new(stroke::HAIRLINE, extend),
                        DESIGN_TOKENS.spacing.sm + DESIGN_TOKENS.spacing.hairline,
                        DESIGN_TOKENS.spacing.xs + DESIGN_TOKENS.spacing.hairline,
                    );
                }
            }
        }
        self.paint_anchor_labels(painter, &plain_labels(&["A", "B", "C", "D"]));
    }

    /// Three Drives: the drives and their corrections, each leg's ratio and the reversal zone.
    pub(crate) fn render_three_drives_pattern(&self, painter: &egui::Painter, view: &ChartView) {
        let config = self.pattern_config();
        if self.points.len() >= 7 {
            self.paint_pattern_fill(painter, self.points.clone());
        }
        self.paint_pattern_legs(painter);
        self.paint_anchor_labels(painter, &plain_labels(&["", "1", "A", "2", "B", "3", ""]));

        let color = self.color32();
        if config.show_ratios {
            // Each correction's retracement of the drive before it and each drive's extension
            // of the correction before it, on the chord joining the outside ends of the two legs.
            for (ends, ratio) in [[0, 2], [1, 3], [2, 4], [3, 5]]
                .into_iter()
                .zip(self.three_drives_ratios())
            {
                self.draw_pattern_ratio(painter, ends, ratio, color);
            }
        }

        // The price interval the third drive is expected to end in, from the last drive's
        // anchor to the pattern's last.
        if config.show_zone
            && self.points.len() >= 7
            && let Some(prices) = self.three_drives_zone()
        {
            let [y1, y2] = prices.map(|price| (view.price_to_y)(price));
            let (x1, x2) = (self.points[5].x, self.points[6].x);
            let zone = Rect::from_x_y_ranges(x1.min(x2)..=x1.max(x2), y1.min(y2)..=y1.max(y2));
            if let Some(fill) = self.fill_color32() {
                painter.rect_filled(zone, 0.0, fill);
            }
            painter.text(
                zone.center(),
                Align2::CENTER_CENTER,
                "PRZ",
                self.annotation_font(),
                color,
            );
        }
    }
}

/// Labels without circles.
fn plain_labels(texts: &[&str]) -> Vec<WaveLabel> {
    texts
        .iter()
        .map(|t| WaveLabel {
            text: (*t).to_owned(),
            circled: false,
        })
        .collect()
}

/// Triangle pattern boundaries from the zigzag anchors (first peak, first trough, second peak,
/// second trough): the line through each pair of same-side anchors, from its first anchor to where
/// the two meet, and the fill between them and the first zigzag leg as a convex polygon. The
/// boundaries end at `right` when they meet beyond it, and at the last anchor when they are
/// parallel, diverge or meet before the first anchor. `None` until four anchors are placed, or
/// while a boundary is vertical.
pub(super) fn triangle_boundaries(
    points: &[Pos2],
    right: f32,
) -> Option<([[Pos2; 2]; 2], Vec<Pos2>)> {
    let &[a0, b0, a1, b1, ..] = points else {
        return None;
    };
    if a1.x == a0.x || b1.x == b0.x {
        return None;
    }
    let (slope_a, slope_b) = ((a1.y - a0.y) / (a1.x - a0.x), (b1.y - b0.y) / (b1.x - b0.x));
    let y_a = |x: f32| a0.y + slope_a * (x - a0.x);
    let y_b = |x: f32| b0.y + slope_b * (x - b0.x);
    let left = a0.x.min(b0.x);
    let last = a1.x.max(b1.x);
    let apex_x = (b0.y - a0.y + slope_a * a0.x - slope_b * b0.x) / (slope_a - slope_b);
    let apex_x = (apex_x.is_finite() && apex_x > left).then_some(apex_x);
    let end = apex_x.map_or(last, |x| x.min(last.max(right)));
    let mut fill = vec![a0, Pos2::new(end, y_a(end))];
    if apex_x != Some(end) {
        fill.push(Pos2::new(end, y_b(end)));
    }
    fill.push(b0);
    Some((
        [
            [a0, Pos2::new(end, y_a(end))],
            [b0, Pos2::new(end, y_b(end))],
        ],
        fill,
    ))
}

/// Head and shoulders neckline and target from the anchors (start, left shoulder, trough, head,
/// trough, right shoulder, end): the neckline runs through the two troughs, from the left trough to
/// `extend` past the last anchor's x, and the target y lies at the last anchor's x, as far beyond
/// the neckline as the head is from it at the head's x, on the side away from the head. `None`
/// until both troughs are placed, or while they share an x.
fn head_and_shoulders_neckline(points: &[Pos2], extend: f32) -> Option<([Pos2; 2], f32)> {
    let &[_, _, left, head, right, ..] = points else {
        return None;
    };
    let dx = right.x - left.x;
    if dx == 0.0 {
        return None;
    }
    let neckline_y = |x: f32| left.y + (right.y - left.y) / dx * (x - left.x);
    let last_x = points[points.len() - 1].x;
    let end_x = last_x + extend;
    Some((
        [left, Pos2::new(end_x, neckline_y(end_x))],
        neckline_y(last_x) - (head.y - neckline_y(head.x)),
    ))
}

impl Drawing {
    /// The head and shoulders' neckline, run on past the last anchor, and the
    /// y of its target.
    pub(super) fn neckline(&self) -> Option<([Pos2; 2], f32)> {
        let extend =
            DESIGN_TOKENS.sizing.technical_labels.fib_label_offset_x + DESIGN_TOKENS.spacing.xxxl;
        head_and_shoulders_neckline(&self.points, extend)
    }

    /// Price ratios of the Three drives legs: correction 1 over drive 1, drive 2 over correction 1,
    /// correction 2 over drive 2 and drive 3 over correction 2.
    fn three_drives_ratios(&self) -> [Option<f64>; 4] {
        [
            self.leg_ratio([1, 2], [0, 1]),
            self.leg_ratio([2, 3], [1, 2]),
            self.leg_ratio([3, 4], [2, 3]),
            self.leg_ratio([4, 5], [3, 4]),
        ]
    }

    /// The prices the third drive is expected to end between: the second correction extended by
    /// [`THREE_DRIVES_EXTENSION`] from its end, in the direction opposite to that correction;
    /// `None` until both ends are placed or while the correction is flat.
    fn three_drives_zone(&self) -> Option<[f64; 2]> {
        let (from, to) = (
            self.chart_points.get(3)?.price,
            self.chart_points.get(4)?.price,
        );
        if from == to {
            return None;
        }
        Some(THREE_DRIVES_EXTENSION.map(|ratio| to + (from - to) * ratio))
    }

    /// Price height of the leg between two anchors over that of a base leg; `None` when an anchor
    /// has no chart coordinates or the base leg is flat.
    fn leg_ratio(&self, leg: [usize; 2], base: [usize; 2]) -> Option<f64> {
        let height = |[from, to]: [usize; 2]| {
            Some((self.chart_points.get(to)?.price - self.chart_points.get(from)?.price).abs())
        };
        let base = height(base)?;
        if base == 0.0 {
            return None;
        }
        Some(height(leg)? / base)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::drawings::domain::{ChartPoint, PatternConfig};

    fn pattern(tool: DrawingToolType, prices: &[f64]) -> Drawing {
        // Screen y is 1000 / price, a non-linear scale on which screen ratios differ from price
        // ratios.
        let mut drawing = Drawing::new(0, tool);
        for (i, &price) in prices.iter().enumerate() {
            let x = 40.0 * i as f32;
            drawing.chart_points.push(ChartPoint::new(x, price));
            drawing.points.push(Pos2::new(x, (1000.0 / price) as f32));
        }
        drawing
    }

    fn texts(drawing: &Drawing) -> Vec<(String, Color32)> {
        let output = egui::Context::default().run_ui(egui::RawInput::default(), |ui| {
            let painter = ui.painter();
            match drawing.tool_type {
                DrawingToolType::ABCDPattern => drawing.render_abcd_pattern(painter),
                DrawingToolType::HeadAndShoulders => drawing.render_head_and_shoulders(painter),
                _ => drawing.render_harmonic_pattern(painter),
            }
        });
        let found = output
            .shapes
            .iter()
            .filter_map(|shape| match &shape.shape {
                egui::Shape::Text(t) => Some((t.galley.job.text.clone(), t.fallback_color)),
                _ => None,
            })
            .collect();
        output.drop_without_applying_deltas();
        found
    }

    #[test]
    fn head_and_shoulders_neckline_runs_through_the_troughs() {
        let anchors = |ys: [f32; 7], last_trough_x: f32| {
            let mut points = ys.map(|y| Pos2::new(0.0, y));
            for (i, p) in points.iter_mut().enumerate() {
                p.x = 10.0 * i as f32;
            }
            points[4].x = last_trough_x;
            points
        };
        let top = anchors([300.0, 200.0, 250.0, 100.0, 260.0, 210.0, 320.0], 40.0);
        let inverse = anchors([100.0, 200.0, 150.0, 300.0, 140.0, 190.0, 80.0], 40.0);
        let vertical = anchors([300.0, 200.0, 250.0, 100.0, 260.0, 210.0, 320.0], 20.0);
        // Neckline slope 0.5 from the left trough: 255 at the head's x (height 155), 270 at the
        // last anchor and 275 at 10 past it, so the target is 270 + 155. The inverse pattern
        // mirrors y about 200: slope -0.5, 130 at the last anchor, target 130 - 155.
        let cases = [
            (&top[..], Some(([top[2], Pos2::new(70.0, 275.0)], 425.0))),
            (
                &inverse[..],
                Some(([inverse[2], Pos2::new(70.0, 125.0)], -25.0)),
            ),
            (&vertical[..], None),
            (&top[..4], None),
        ];
        for (points, expected) in cases {
            assert_eq!(
                head_and_shoulders_neckline(points, 10.0),
                expected,
                "{points:?}"
            );
        }
    }

    #[test]
    fn triangle_boundaries_meet_at_the_apex_and_clip_the_fill() {
        let p = Pos2::new;
        // Zigzag anchors on an upper line y = 0.5x and a lower line y = 95 - 0.5 (x - 10), which
        // meet at (100, 50); parallel lines; and an upper y = 55 - 0.5x with a lower y = 40 + 2x,
        // which meet at x = 6, left of every anchor.
        let converging = [p(0.0, 0.0), p(10.0, 95.0), p(20.0, 10.0), p(30.0, 85.0)];
        let parallel = [p(0.0, 0.0), p(10.0, 50.0), p(20.0, 0.0), p(30.0, 50.0)];
        let diverging = [p(10.0, 50.0), p(20.0, 80.0), p(30.0, 40.0), p(40.0, 120.0)];
        let vertical = [p(0.0, 0.0), p(10.0, 95.0), p(0.0, 10.0), p(30.0, 85.0)];
        let cases = [
            (
                &converging[..],
                f32::INFINITY,
                Some((
                    [
                        [p(0.0, 0.0), p(100.0, 50.0)],
                        [p(10.0, 95.0), p(100.0, 50.0)],
                    ],
                    vec![p(0.0, 0.0), p(100.0, 50.0), p(10.0, 95.0)],
                )),
            ),
            (
                &converging[..],
                60.0,
                Some((
                    [[p(0.0, 0.0), p(60.0, 30.0)], [p(10.0, 95.0), p(60.0, 70.0)]],
                    vec![p(0.0, 0.0), p(60.0, 30.0), p(60.0, 70.0), p(10.0, 95.0)],
                )),
            ),
            (
                &parallel[..],
                f32::INFINITY,
                Some((
                    [[p(0.0, 0.0), p(30.0, 0.0)], [p(10.0, 50.0), p(30.0, 50.0)]],
                    vec![p(0.0, 0.0), p(30.0, 0.0), p(30.0, 50.0), p(10.0, 50.0)],
                )),
            ),
            (
                &diverging[..],
                f32::INFINITY,
                Some((
                    [
                        [p(10.0, 50.0), p(40.0, 35.0)],
                        [p(20.0, 80.0), p(40.0, 120.0)],
                    ],
                    vec![p(10.0, 50.0), p(40.0, 35.0), p(40.0, 120.0), p(20.0, 80.0)],
                )),
            ),
            (&vertical[..], f32::INFINITY, None),
            (&converging[..3], f32::INFINITY, None),
        ];
        for (points, right, expected) in cases {
            assert_eq!(triangle_boundaries(points, right), expected, "{points:?}");
        }
    }

    /// Ratios are measured in price: XABCD shows B, C, D and AD/XA; ABCD shows C and D; the
    /// named patterns show their own rules, Cypher measuring C against XA and D against XC.
    #[test]
    fn pattern_ratios_are_measured_in_price() {
        use DrawingToolType::{ABCDPattern, CypherPattern, XABCDPattern};
        let cases = [
            (
                ABCDPattern,
                &[200.0, 100.0, 161.8, 38.2][..],
                vec![0.618, 2.0],
            ),
            (
                XABCDPattern,
                &[100.0, 200.0, 138.2, 176.4, 78.6],
                vec![0.618, 38.2 / 61.8, 97.8 / 38.2, 121.4 / 100.0],
            ),
            (XABCDPattern, &[100.0, 150.0, 125.0, 175.0], vec![0.5, 2.0]),
            (
                CypherPattern,
                &[100.0, 200.0, 150.0, 227.2, 127.2208],
                vec![0.5, 1.272, 0.786],
            ),
            (
                CypherPattern,
                &[300.0, 200.0, 250.0, 172.8, 272.7792],
                vec![0.5, 1.272, 0.786],
            ),
        ];
        for (tool, prices, ratios) in cases {
            let labels: Vec<_> = texts(&pattern(tool, prices))
                .into_iter()
                .map(|(t, _)| t)
                .collect();
            for ratio in ratios {
                assert!(
                    labels.contains(&format!("{ratio:.3}")),
                    "{tool:?} {ratio}: {labels:?}"
                );
            }
        }
    }

    /// A named harmonic colours each ratio by its rule and names itself at D, green when every
    /// rule holds and red otherwise; both can be hidden.
    #[test]
    fn harmonic_patterns_judge_their_ratios() {
        let theme = &DESIGN_TOKENS.semantic.extended;
        // Gartley: AB/XA 0.618, BC/AB 0.618, CD/BC 1.272, AD/XA 0.786.
        let ideal = [100.0, 200.0, 138.2, 176.4, 121.4];
        let gartley = pattern(DrawingToolType::GartleyPattern, &ideal);
        let found = texts(&gartley);
        let color_of = |text: &str| found.iter().find(|(t, _)| t == text).map(|(_, c)| *c);
        assert_eq!(color_of("Gartley"), Some(theme.bullish), "{found:?}");
        assert_eq!(color_of("0.618"), Some(theme.bullish), "{found:?}");
        for label in ["X", "A", "B", "C", "D"] {
            assert!(color_of(label).is_some(), "{label}: {found:?}");
        }

        // The same anchors are no Bat: AB/XA 0.618 lies outside its 0.382 to 0.5.
        let bat = pattern(DrawingToolType::BatPattern, &ideal);
        let found = texts(&bat);
        let color_of = |text: &str| found.iter().find(|(t, _)| t == text).map(|(_, c)| *c);
        assert_eq!(color_of("Bat"), Some(theme.bearish), "{found:?}");

        let mut quiet = gartley.clone();
        quiet.pattern = Some(PatternConfig {
            show_labels: false,
            show_ratios: false,
            show_validity: false,
            ..PatternConfig::default()
        });
        assert!(texts(&quiet).is_empty());
    }

    /// Head and Shoulders names its shoulders and head, and shows the target unless hidden.
    #[test]
    fn head_and_shoulders_labels_and_target_follow_settings() {
        let mut drawing = pattern(
            DrawingToolType::HeadAndShoulders,
            &[10.0, 5.0, 8.0, 4.0, 8.0, 5.0, 10.0],
        );
        let found: Vec<_> = texts(&drawing).into_iter().map(|(t, _)| t).collect();
        for label in ["LS", "H", "RS", "Target"] {
            assert!(found.contains(&label.to_owned()), "{label}: {found:?}");
        }
        drawing.pattern = Some(PatternConfig {
            show_target: false,
            ..PatternConfig::default()
        });
        let found: Vec<_> = texts(&drawing).into_iter().map(|(t, _)| t).collect();
        assert!(!found.contains(&"Target".to_owned()));
    }

    #[test]
    fn three_drives_ratios_and_zone_are_measured_in_price() {
        let drives = |prices: &[f64]| {
            let mut drawing = Drawing::new(0, DrawingToolType::ThreeDrivesPattern);
            drawing.chart_points = prices
                .iter()
                .enumerate()
                .map(|(i, &price)| ChartPoint::new(10.0 * i as f32, price))
                .collect();
            drawing
        };
        // Drive 1 and 2 rise by 100 and 80, corrections fall by 61.8 and 60; drive 3 would end
        // 1.272 to 1.618 corrections above the second trough: 158.2 + 60 * [1.272, 1.618].
        let rising = drives(&[100.0, 200.0, 138.2, 218.2, 158.2, 255.0]);
        let falling = drives(&[300.0, 200.0, 261.8, 181.8, 241.8, 145.0]);
        // A flat second correction has no zone, and a flat first drive no ratio.
        let flat = drives(&[100.0, 100.0, 90.0, 120.0, 120.0]);
        let cases = [
            (
                &rising,
                [
                    Some(0.618),
                    Some(80.0 / 61.8),
                    Some(0.75),
                    Some(96.8 / 60.0),
                ],
                Some([234.52, 255.28]),
            ),
            (
                &falling,
                [
                    Some(0.618),
                    Some(80.0 / 61.8),
                    Some(0.75),
                    Some(96.8 / 60.0),
                ],
                Some([165.48, 144.72]),
            ),
            (&flat, [None, Some(30.0 / 10.0), Some(0.0), None], None),
        ];
        for (drawing, ratios, zone) in cases {
            for (actual, expected) in drawing.three_drives_ratios().into_iter().zip(ratios) {
                match (actual, expected) {
                    (Some(a), Some(e)) => assert!((a - e).abs() < 1e-9, "{a} != {e}"),
                    _ => assert_eq!(actual, expected),
                }
            }
            match (drawing.three_drives_zone(), zone) {
                (Some(a), Some(e)) => assert!(
                    a.iter().zip(e).all(|(a, e)| (a - e).abs() < 1e-9),
                    "{a:?} != {e:?}"
                ),
                (a, e) => assert_eq!(a, e),
            }
        }
    }
}
