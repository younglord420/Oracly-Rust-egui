//! Text label and anchored text rendering

use crate::drawings::domain::Drawing;
use crate::painting::pixel_snap::PixelSnap;
use crate::theme::ThemeExt;
use egui::{Color32, Pos2, Rect, Stroke, epaint::StrokeKind};

/// Frame around a selected or hovered text label, shared with the inline editor.
const TEXT_FRAME_INSET: f32 = 5.0;
const TEXT_FRAME_RISE: f32 = 3.0;

impl Drawing {
    /// Glyphs only, sitting just inside the anchor. A thin frame appears while the text is
    /// selected or under the pointer. The inline editor paints the glyphs while it is open.
    pub(crate) fn render_text_label(&self, painter: &egui::Painter) {
        if self.points.is_empty() || self.text_editing {
            return;
        }
        let Some(text) = self.text.as_deref().filter(|text| !text.is_empty()) else {
            return;
        };
        let snap = PixelSnap::of(painter);
        let color = self.text_color.map_or_else(
            || painter.ctx().active_theme().ui.text,
            |c| Color32::from_rgba_unmultiplied(c[0], c[1], c[2], c[3]),
        );
        let galley = painter.layout_no_wrap(text.to_string(), self.annotation_font(), color);
        let (shift_x, shift_y) = if self.tool_type == crate::drawings::DrawingToolType::AnchoredText
        {
            let width = galley.size().x;
            let height = galley.size().y;
            let shift_x = match self.text_h_align {
                crate::drawings::TextHorizontalAlign::Left => -(width + 4.0),
                crate::drawings::TextHorizontalAlign::Right => 4.0,
                crate::drawings::TextHorizontalAlign::Center => -width * 0.5,
            };
            let shift_y = match self.text_v_align {
                crate::drawings::TextVerticalAlign::Top => -(height + 4.0),
                crate::drawings::TextVerticalAlign::Bottom => 4.0,
                crate::drawings::TextVerticalAlign::Middle
                | crate::drawings::TextVerticalAlign::Inside => -height * 0.5,
            };
            (shift_x, shift_y)
        } else {
            (2.0, 2.0)
        };
        let origin = Pos2::new(
            snap.edge(self.points[0].x + shift_x),
            snap.edge(self.points[0].y + shift_y),
        );
        let glyph = Rect::from_min_size(origin, galley.size());
        let hovered = painter.ctx().input(|input| {
            input
                .pointer
                .hover_pos()
                .is_some_and(|pos| glyph.contains(pos))
        });
        if self.annotation_open || hovered {
            let alpha = if self.annotation_open { 0.3 } else { 0.12 };
            painter.rect_stroke(
                glyph.expand2(egui::vec2(TEXT_FRAME_INSET, TEXT_FRAME_RISE)),
                4.0,
                Stroke::new(1.0, color.gamma_multiply(alpha)),
                StrokeKind::Outside,
            );
        }
        painter.galley(origin, galley, color);
    }

    /// Anchored text is plain text like a text label; only its anchor follows the pane.
    pub(crate) fn render_anchored_text(&self, painter: &egui::Painter) {
        self.render_text_label(painter);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::drawings::domain::DrawingToolType;
    use crate::drawings::rendering::ChartView;
    use egui::{Color32, Context, LayerId, Painter, Pos2, RawInput, Rect, Shape};

    fn paint(drawing: &Drawing) -> Vec<Shape> {
        let ctx = Context::default();
        let pane = Rect::from_min_size(Pos2::ZERO, egui::vec2(400.0, 300.0));
        let output = ctx.run_ui(
            RawInput {
                screen_rect: Some(pane),
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

    fn texts(shapes: &[Shape]) -> Vec<egui::epaint::TextShape> {
        fn walk(shape: &Shape, out: &mut Vec<egui::epaint::TextShape>) {
            match shape {
                Shape::Vec(shapes) => shapes.iter().for_each(|s| walk(s, out)),
                Shape::Text(text) => out.push(text.clone()),
                _ => {}
            }
        }
        let mut out = Vec::new();
        shapes.iter().for_each(|s| walk(s, &mut out));
        out
    }

    /// Text paints glyphs and no plate. A selection frame is a stroke, not a fill.
    #[test]
    fn text_label_paints_glyphs_without_a_plate() {
        let red = Color32::from_rgb(220, 30, 40);
        let mut drawing = Drawing::new(1, DrawingToolType::TextLabel);
        drawing.points = vec![Pos2::new(40.0, 50.0)];
        drawing.text = Some("Hello".into());
        drawing.text_color = Some(red.to_array());
        drawing.fill_color = Some([0, 0, 0, 0]);
        drawing.stroke_width = 0.0;

        let shapes = paint(&drawing);
        let painted = texts(&shapes);
        assert!(
            painted.iter().any(|text| {
                text.galley.text() == "Hello"
                    && text.override_text_color.is_none()
                    && text
                        .galley
                        .job
                        .sections
                        .iter()
                        .all(|section| section.format.color == red)
            }),
            "text colour stays on the galley"
        );
        assert!(
            shapes.iter().all(|shape| match shape {
                Shape::Rect(rect) => rect.fill.a() == 0 && rect.stroke.is_empty(),
                Shape::Text(_) | Shape::Noop | Shape::Vec(_) => true,
                _ => false,
            }),
            "transparent fill and zero stroke paint no box"
        );

        drawing.fill_color = None;
        drawing.stroke_width = 2.0;
        drawing.text_color = None;
        drawing.annotation_open = true;
        let shapes = paint(&drawing);
        assert!(
            shapes
                .iter()
                .all(|shape| !matches!(shape, Shape::Rect(rect) if rect.fill.a() > 0)),
            "text has no filled plate"
        );
        assert!(
            shapes.iter().any(|shape| {
                matches!(shape, Shape::Rect(rect) if !rect.stroke.is_empty() && rect.fill.a() == 0)
            }),
            "selection draws a frame"
        );
        drawing.text = None;
        assert!(
            paint(&drawing)
                .iter()
                .all(|shape| !matches!(shape, Shape::Text(_))),
            "empty text paints nothing"
        );
    }
}
