//! Note, anchored note, and flag note rendering

use super::{axis_background, body_open, contrast_ink, paint_label_box};
use crate::drawings::domain::Drawing;
use crate::theme::ThemeExt;
use crate::painting::pixel_snap::PixelSnap;
use crate::styles::stroke;
use crate::tokens::DESIGN_TOKENS;
use egui::{Color32, Pos2, Rect, Stroke};

impl Drawing {
    /// A rounded plate of free text, its top-left on the anchor.
    pub(crate) fn render_note(&self, painter: &egui::Painter) {
        if self.points.is_empty() {
            return;
        }
        let snap = PixelSnap::of(painter);
        let pos = Pos2::new(snap.edge(self.points[0].x), snap.edge(self.points[0].y));
        let theme = painter.ctx().active_theme();
        let fill = self
            .annotation_box_fill(theme.chart.bg_axis)
            .unwrap_or(theme.chart.bg_axis);
        let text = self
            .text
            .as_deref()
            .filter(|text| !text.is_empty())
            .unwrap_or("Note");
        let ink = self
            .text_color
            .map(|c| Color32::from_rgba_unmultiplied(c[0], c[1], c[2], c[3]))
            .unwrap_or_else(|| contrast_ink(fill));
        let galley = painter.layout_no_wrap(text.to_string(), self.annotation_font(), ink);
        let pad = egui::vec2(8.0, 5.0);
        let note_rect = Rect::from_min_size(pos, galley.size() + pad * 2.0);
        painter.rect_filled(note_rect, 5.0, fill);
        if let Some(border) = self.annotation_border() {
            painter.rect_stroke(note_rect, 5.0, border, egui::epaint::StrokeKind::Outside);
        }
        painter.galley(
            Pos2::new(snap.edge(pos.x + pad.x), snap.edge(pos.y + pad.y)),
            galley,
            ink,
        );
    }

    /// Renders an anchored note with pin marker and expandable content.
    /// The needle tip is the anchor; the head sits above it.
    pub(crate) fn render_anchored_note(&self, painter: &egui::Painter) {
        if self.points.is_empty() {
            return;
        }
        let snap = PixelSnap::of(painter);
        let pos = Pos2::new(snap.edge(self.points[0].x), snap.edge(self.points[0].y));
        let color = self.color32();
        let scale = (self.stroke_width / stroke::THICK).clamp(0.5, 4.0);
        let pin_head_radius = DESIGN_TOKENS.sizing.annotation.pin_head_radius * scale;
        let pin_length = DESIGN_TOKENS.sizing.annotation.pin_needle * scale;
        let needle_top = Pos2::new(pos.x, snap.edge(pos.y - pin_length));
        let head_center = Pos2::new(pos.x, snap.edge(needle_top.y - pin_head_radius));

        let shaft = Stroke::new(self.stroke_width.max(stroke::HAIRLINE), color);
        painter.line_segment([needle_top, pos], shaft);
        painter.circle_filled(head_center, pin_head_radius, color);
        painter.circle_stroke(
            head_center,
            pin_head_radius,
            Stroke::new(stroke::HAIRLINE, Color32::WHITE),
        );

        let marker = Rect::from_min_max(
            Pos2::new(
                head_center.x - pin_head_radius,
                head_center.y - pin_head_radius,
            ),
            pos,
        );
        if body_open(self, painter, marker)
            && let Some(text) = &self.text
            && !text.is_empty()
        {
            let text_color = self.text_color32();
            let font = self.annotation_font();
            let galley = painter.layout_no_wrap(text.clone(), font, text_color);
            let text_size = galley.size();

            let padding = egui::vec2(
                DESIGN_TOKENS.sizing.annotation.label_padding_x,
                DESIGN_TOKENS.sizing.annotation.text_padding_y,
            );
            let min = Pos2::new(
                snap.edge(head_center.x + pin_head_radius + padding.x),
                snap.edge(head_center.y - text_size.y * 0.5 - padding.y),
            );
            let note_rect = Rect::from_min_max(
                min,
                Pos2::new(
                    snap.edge(min.x + text_size.x + padding.x * 2.0),
                    snap.edge(min.y + text_size.y + padding.y * 2.0),
                ),
            );

            if self.stroke_width > 0.0 {
                painter.line_segment(
                    [
                        head_center,
                        Pos2::new(note_rect.min.x, note_rect.center().y),
                    ],
                    Stroke::new(self.stroke_width, color),
                );
            }
            // A pin has no fill control; the bubble keeps the theme background.
            let bg = axis_background(painter, 0.92);
            paint_label_box(
                painter,
                note_rect,
                DESIGN_TOKENS.rounding.md,
                Some(bg),
                self.annotation_border(),
            );
            painter.galley(
                Pos2::new(
                    snap.edge(note_rect.min.x + padding.x),
                    snap.edge(note_rect.min.y + padding.y),
                ),
                galley,
                text_color,
            );
        }
    }

    /// A flag mark is a glyph centred on the anchor. Its pixel size does not follow the zoom.
    pub(crate) fn render_flag_note(&self, painter: &egui::Painter) {
        if self.points.is_empty() {
            return;
        }
        let snap = PixelSnap::of(painter);
        let pos = Pos2::new(snap.edge(self.points[0].x), snap.edge(self.points[0].y));
        painter.text(
            pos,
            egui::Align2::CENTER_CENTER,
            "\u{2691}",
            egui::FontId::proportional(self.font_size),
            self.color32(),
        );
        if let Some(text) = self.text.as_deref().filter(|text| !text.is_empty()) {
            let galley = painter.layout_no_wrap(
                text.to_string(),
                egui::FontId::proportional(crate::styles::typography::LG),
                self.text_color32(),
            );
            painter.galley(
                Pos2::new(
                    pos.x - galley.size().x * 0.5,
                    pos.y + self.font_size * 0.5 + DESIGN_TOKENS.spacing.xs,
                ),
                galley,
                self.text_color32(),
            );
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::drawings::domain::DrawingToolType;
    use crate::drawings::rendering::ChartView;
    use egui::{Color32, Context, Event, LayerId, Painter, Pos2, RawInput, Rect, Shape};

    fn paint(drawing: &Drawing, hover: Option<Pos2>) -> Vec<Shape> {
        let ctx = Context::default();
        let pane = Rect::from_min_size(Pos2::ZERO, egui::vec2(400.0, 300.0));
        let mut events = Vec::new();
        if let Some(pos) = hover {
            events.push(Event::PointerMoved(pos));
        }
        let output = ctx.run_ui(
            RawInput {
                screen_rect: Some(pane),
                events,
                ..Default::default()
            },
            |_| {
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
                drawing.render_original(
                    &Painter::new(ctx.clone(), LayerId::background(), pane),
                    pane,
                    &view,
                );
            },
        );
        let shapes = output.shapes.iter().map(|s| s.shape.clone()).collect();
        output.drop_without_applying_deltas();
        shapes
    }

    fn labels(shapes: &[Shape]) -> Vec<String> {
        fn walk(shape: &Shape, out: &mut Vec<String>) {
            match shape {
                Shape::Vec(shapes) => shapes.iter().for_each(|s| walk(s, out)),
                Shape::Text(text) => out.push(text.galley.text().to_owned()),
                _ => {}
            }
        }
        let mut out = Vec::new();
        shapes.iter().for_each(|s| walk(s, &mut out));
        out
    }

    /// A note is a plate whose text is always visible, including the placeholder.
    #[test]
    fn note_plate_always_shows_its_text() {
        let mut drawing = Drawing::new(1, DrawingToolType::Note);
        drawing.points = vec![Pos2::new(40.0, 40.0)];
        let placeholder = labels(&paint(&drawing, None));
        assert!(placeholder.iter().any(|text| text == "Note"));
        assert!(!placeholder.iter().any(|text| text == "N"));

        drawing.text = Some("Secret".into());
        drawing.text_color = Some(Color32::RED.to_array());
        let painted = paint(&drawing, None);
        assert!(labels(&painted).iter().any(|text| text == "Secret"));
        assert!(painted.iter().any(|shape| {
            matches!(shape, Shape::Rect(rect) if rect.rect.min.distance(drawing.points[0]) < 0.5)
        }));
    }

    /// The needle's lowest point is the anchor and the head sits above it.
    #[test]
    fn pin_needle_points_down_into_the_anchor() {
        let anchor = Pos2::new(120.0, 160.0);
        let mut drawing = Drawing::new(1, DrawingToolType::AnchoredNote);
        drawing.points = vec![anchor];
        drawing.color = Color32::from_rgb(20, 90, 200).to_array();
        let shapes = paint(&drawing, None);
        let needle = shapes.iter().find_map(|shape| match shape {
            Shape::LineSegment { points, .. }
                if points.iter().any(|point| point.distance(anchor) < 0.5) =>
            {
                Some(*points)
            }
            _ => None,
        });
        let needle = needle.expect("needle");
        let lowest = if needle[0].y >= needle[1].y {
            needle[0]
        } else {
            needle[1]
        };
        assert!(
            lowest.distance(anchor) < 0.5,
            "needle tip {lowest:?} is not the anchor {anchor:?}"
        );
        let head = shapes.iter().find_map(|shape| match shape {
            Shape::Circle(circle) if circle.fill == drawing.color32() => Some(circle.center),
            _ => None,
        });
        let head = head.expect("pin head");
        assert!(head.y < anchor.y, "head {head:?} is not above {anchor:?}");
    }

    /// A flag mark is the flag glyph centred on the anchor.
    #[test]
    fn flag_mark_is_a_glyph_on_the_anchor() {
        let anchor = Pos2::new(80.0, 220.0);
        let mut drawing = Drawing::new(1, DrawingToolType::FlagNote);
        drawing.points = vec![anchor];
        drawing.color = Color32::from_rgb(180, 40, 40).to_array();
        let shapes = paint(&drawing, None);
        let glyph = shapes.iter().find_map(|shape| match shape {
            Shape::Text(text) if text.galley.text().contains('\u{2691}') => Some(text),
            _ => None,
        });
        let glyph = glyph.expect("flag glyph");
        let center = glyph.pos + glyph.galley.size() * 0.5;
        assert!(
            center.distance(anchor) < 2.0,
            "glyph centre {center:?} is not the anchor {anchor:?}"
        );
        assert!(
            shapes
                .iter()
                .all(|shape| !matches!(shape, Shape::Rect(rect) if rect.fill == drawing.color32()))
        );
    }
}
