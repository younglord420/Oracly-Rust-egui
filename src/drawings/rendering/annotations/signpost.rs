//! Signpost rendering

use super::contrast_ink;
use crate::drawings::domain::Drawing;
use crate::drawings::rendering::ChartView;
use crate::painting::pixel_snap::PixelSnap;
use crate::theme::ThemeExt;
use crate::tokens::DESIGN_TOKENS;
use egui::{Color32, Pos2, Rect, Stroke, epaint::StrokeKind};

impl Drawing {
    /// A sign plate centred on the second anchor, on a pole rising from the first.
    pub(crate) fn render_signpost(&self, painter: &egui::Painter, _rect: Rect, _view: &ChartView) {
        if self.points.is_empty() {
            return;
        }
        let snap = PixelSnap::of(painter);
        let width = self.stroke_width.max(DESIGN_TOKENS.stroke.hairline);
        let base = Pos2::new(
            snap.line_center(self.points[0].x, width),
            snap.edge(self.points[0].y),
        );
        let color = self.color32();
        painter.circle_filled(base, 3.5, color);
        let Some(plate_at) = self.box_anchor() else {
            return;
        };
        let plate_at = Pos2::new(snap.edge(plate_at.x), snap.edge(plate_at.y));
        if self.stroke_width > 0.0 {
            painter.line_segment([base, plate_at], Stroke::new(width, color));
        }
        if self.text_editing {
            return;
        }
        let theme = painter.ctx().active_theme();
        let fill = self
            .annotation_box_fill(theme.chart.bg_axis)
            .unwrap_or(theme.chart.bg_axis);
        let text = self
            .text
            .as_deref()
            .filter(|text| !text.is_empty())
            .unwrap_or("Signpost");
        let ink = self
            .text_color
            .map(|c| Color32::from_rgba_unmultiplied(c[0], c[1], c[2], c[3]))
            .unwrap_or_else(|| contrast_ink(fill));
        let galley = painter.layout_no_wrap(text.to_string(), self.annotation_font(), ink);
        let pad = egui::vec2(9.0, 6.0);
        let sign_rect = Rect::from_center_size(plate_at, galley.size() + pad * 2.0);
        painter.rect_filled(sign_rect, 5.0, fill);
        if let Some(border) = self.annotation_border() {
            painter.rect_stroke(sign_rect, 5.0, border, StrokeKind::Outside);
        }
        painter.galley(sign_rect.min + pad, galley, ink);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::drawings::domain::DrawingToolType;
    use egui::{Context, LayerId, Painter, Pos2, RawInput, Shape};

    fn labels(drawing: &Drawing) -> Vec<String> {
        let ctx = Context::default();
        let pane = Rect::from_min_size(Pos2::ZERO, egui::vec2(400.0, 300.0));
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
        let output = ctx.run_ui(RawInput::default(), |_| {
            drawing.render_original(
                &Painter::new(ctx.clone(), LayerId::background(), pane),
                pane,
                &view,
            );
        });
        fn walk(shape: &Shape, out: &mut Vec<String>) {
            match shape {
                Shape::Vec(shapes) => shapes.iter().for_each(|s| walk(s, out)),
                Shape::Text(text) => out.push(text.galley.text().to_owned()),
                _ => {}
            }
        }
        let mut out = Vec::new();
        for shape in &output.shapes {
            walk(&shape.shape, &mut out);
        }
        output.drop_without_applying_deltas();
        out
    }

    /// The plate carries the signpost text and sits on the second anchor.
    #[test]
    fn signpost_plate_uses_its_text() {
        let mut drawing = Drawing::new(1, DrawingToolType::Signpost);
        drawing.points = vec![Pos2::new(80.0, 180.0), Pos2::new(140.0, 60.0)];
        drawing.text = Some("Event".into());
        let painted = labels(&drawing);
        assert_eq!(painted, vec!["Event".to_string()]);
    }
}
