//! Text label and anchored text rendering

use crate::drawings::domain::Drawing;
use crate::painting::pixel_snap::PixelSnap;
use crate::styles::stroke;
use crate::theme::ThemeExt;
use crate::tokens::DESIGN_TOKENS;
use egui::{Color32, Pos2, Stroke, epaint::StrokeKind};

impl Drawing {
    /// Renders a text label with customizable styling.
    /// Features: background box, border, multi-line support.
    pub(crate) fn render_text_label(&self, painter: &egui::Painter) {
        if self.points.is_empty() {
            return;
        }
        let snap = PixelSnap::of(painter);
        let pos = Pos2::new(snap.edge(self.points[0].x), snap.edge(self.points[0].y));
        let text = self.text.as_deref().unwrap_or("Label");

        let color = Color32::from_rgba_unmultiplied(
            self.color[0],
            self.color[1],
            self.color[2],
            self.color[3],
        );

        let font = self.annotation_font();
        let galley = painter.layout_no_wrap(text.to_string(), font, color);
        let text_size = galley.size();

        let padding = egui::vec2(
            DESIGN_TOKENS.sizing.annotation.label_padding_x,
            DESIGN_TOKENS.sizing.annotation.text_padding_y,
        );
        let box_rect = egui::Rect::from_min_max(
            Pos2::new(snap.edge(pos.x - padding.x), snap.edge(pos.y - padding.y)),
            Pos2::new(
                snap.edge(pos.x + text_size.x + padding.x),
                snap.edge(pos.y + text_size.y + padding.y),
            ),
        );

        let bg = painter
            .ctx()
            .active_theme()
            .chart
            .bg_axis
            .gamma_multiply(0.92);
        painter.rect_filled(box_rect, DESIGN_TOKENS.rounding.md, bg);
        painter.rect_stroke(
            box_rect,
            DESIGN_TOKENS.rounding.md,
            Stroke::new(stroke::HAIRLINE, color.gamma_multiply(0.8)),
            StrokeKind::Inside,
        );

        painter.galley(pos, galley, color);
    }

    /// Renders anchored text at its pane-relative screen position.
    pub(crate) fn render_anchored_text(&self, painter: &egui::Painter) {
        if self.points.is_empty() {
            return;
        }
        let snap = PixelSnap::of(painter);
        let pos = Pos2::new(snap.edge(self.points[0].x), snap.edge(self.points[0].y));
        let text = self.text.as_deref().unwrap_or("Anchored Text");

        let color = Color32::from_rgba_unmultiplied(
            self.color[0],
            self.color[1],
            self.color[2],
            self.color[3],
        );

        let font = self.annotation_font();
        let galley = painter.layout_no_wrap(text.to_string(), font, color);
        let text_size = galley.size();

        let padding = egui::vec2(
            DESIGN_TOKENS.sizing.annotation.signpost_padding_x,
            DESIGN_TOKENS.sizing.annotation.label_padding_y,
        );
        let box_rect = egui::Rect::from_min_max(
            pos,
            Pos2::new(
                snap.edge(pos.x + text_size.x + padding.x * 2.0),
                snap.edge(pos.y + text_size.y + padding.y * 2.0),
            ),
        );

        let bg = painter
            .ctx()
            .active_theme()
            .chart
            .bg_axis
            .gamma_multiply(0.92);
        painter.rect_filled(box_rect, DESIGN_TOKENS.rounding.sm, bg);
        painter.rect_stroke(
            box_rect,
            DESIGN_TOKENS.rounding.sm,
            Stroke::new(stroke::HAIRLINE, color.gamma_multiply(0.8)),
            StrokeKind::Inside,
        );

        painter.galley(
            Pos2::new(snap.edge(pos.x + padding.x), snap.edge(pos.y + padding.y)),
            galley,
            color,
        );
    }
}
