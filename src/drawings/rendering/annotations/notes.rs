//! Note, anchored note, and flag note rendering

use crate::drawings::domain::Drawing;
use crate::painting::pixel_snap::PixelSnap;
use crate::styles::{stroke, typography};
use crate::theme::ThemeExt;
use crate::tokens::DESIGN_TOKENS;
use egui::{Color32, Pos2, Rect, Stroke, epaint::StrokeKind};

impl Drawing {
    /// Renders a simple note marker (sticky note style).
    pub(crate) fn render_note(&self, painter: &egui::Painter) {
        if self.points.is_empty() {
            return;
        }
        let snap = PixelSnap::of(painter);
        let pos = Pos2::new(snap.edge(self.points[0].x), snap.edge(self.points[0].y));
        let color = Color32::from_rgba_unmultiplied(
            self.color[0],
            self.color[1],
            self.color[2],
            self.color[3],
        );

        let size = DESIGN_TOKENS.sizing.annotation.note_size;
        let note_rect = Rect::from_min_size(pos, egui::vec2(size, size));

        let note_color = DESIGN_TOKENS.semantic.extended.caution;
        painter.rect_filled(note_rect, DESIGN_TOKENS.rounding.xs, note_color);

        let fold_size = DESIGN_TOKENS.sizing.annotation.note_fold_size;
        let fold_points = vec![
            Pos2::new(note_rect.max.x - fold_size, note_rect.min.y),
            Pos2::new(note_rect.max.x, note_rect.min.y),
            Pos2::new(note_rect.max.x, note_rect.min.y + fold_size),
        ];
        painter.add(egui::epaint::PathShape::convex_polygon(
            fold_points,
            note_color.gamma_multiply(0.82),
            Stroke::NONE,
        ));

        let border_color = if self.color[0] > 200 && self.color[1] > 200 && self.color[2] < 100 {
            note_color.gamma_multiply(0.75)
        } else {
            color
        };
        painter.rect_stroke(
            note_rect,
            DESIGN_TOKENS.rounding.xs,
            Stroke::new(stroke::HAIRLINE, border_color),
            StrokeKind::Inside,
        );

        let dark_ink = Color32::from_rgb(24, 26, 32);
        painter.text(
            note_rect.center(),
            egui::Align2::CENTER_CENTER,
            "N",
            egui::FontId::proportional(typography::MD),
            dark_ink,
        );

        if let Some(text) = &self.text
            && !text.is_empty()
        {
            let font = self.annotation_font();
            let galley = painter.layout_no_wrap(text.clone(), font, dark_ink);
            let text_size = galley.size();

            let pad = egui::vec2(
                DESIGN_TOKENS.sizing.annotation.label_padding_x,
                DESIGN_TOKENS.sizing.annotation.label_padding_y,
            );
            let min = Pos2::new(
                snap.edge(note_rect.max.x + DESIGN_TOKENS.spacing.sm),
                snap.edge(note_rect.min.y),
            );
            let tooltip_rect = Rect::from_min_max(
                min,
                Pos2::new(
                    snap.edge(min.x + text_size.x + pad.x),
                    snap.edge(min.y + text_size.y + pad.y),
                ),
            );
            painter.rect_filled(tooltip_rect, DESIGN_TOKENS.rounding.sm, note_color);
            painter.rect_stroke(
                tooltip_rect,
                DESIGN_TOKENS.rounding.sm,
                Stroke::new(stroke::HAIRLINE, note_color.gamma_multiply(0.75)),
                StrokeKind::Inside,
            );
            painter.galley(
                Pos2::new(
                    snap.edge(tooltip_rect.min.x + pad.x * 0.5),
                    snap.edge(tooltip_rect.center().y - text_size.y * 0.5),
                ),
                galley,
                dark_ink,
            );
        }
    }

    /// Renders an anchored note with pin marker and expandable content.
    pub(crate) fn render_anchored_note(&self, painter: &egui::Painter) {
        if self.points.is_empty() {
            return;
        }
        let snap = PixelSnap::of(painter);
        let pos = Pos2::new(snap.edge(self.points[0].x), snap.edge(self.points[0].y));
        let color = Color32::from_rgba_unmultiplied(
            self.color[0],
            self.color[1],
            self.color[2],
            self.color[3],
        );

        let pin_head_radius = 5.0;
        let pin_length = 12.0;

        painter.line_segment(
            [pos, Pos2::new(pos.x, pos.y + pin_length)],
            Stroke::new(1.5, DESIGN_TOKENS.semantic.extended.chart_text_secondary),
        );

        painter.circle_filled(pos, pin_head_radius, color);
        painter.circle_stroke(
            pos,
            pin_head_radius,
            Stroke::new(stroke::HAIRLINE, Color32::WHITE),
        );

        if let Some(text) = &self.text
            && !text.is_empty()
        {
            let font = self.annotation_font();
            let galley = painter.layout_no_wrap(text.clone(), font, color);
            let text_size = galley.size();

            let padding = egui::vec2(8.0, 5.0);
            let min = Pos2::new(
                snap.edge(pos.x + pin_head_radius + 8.0),
                snap.edge(pos.y - text_size.y * 0.5 - padding.y),
            );
            let note_rect = Rect::from_min_max(
                min,
                Pos2::new(
                    snap.edge(min.x + text_size.x + padding.x * 2.0),
                    snap.edge(min.y + text_size.y + padding.y * 2.0),
                ),
            );

            painter.line_segment(
                [pos, Pos2::new(note_rect.min.x, note_rect.center().y)],
                Stroke::new(stroke::HAIRLINE, color.gamma_multiply(0.45)),
            );

            let bg = painter
                .ctx()
                .active_theme()
                .chart
                .bg_axis
                .gamma_multiply(0.92);
            painter.rect_filled(note_rect, DESIGN_TOKENS.rounding.md, bg);
            painter.rect_stroke(
                note_rect,
                DESIGN_TOKENS.rounding.md,
                Stroke::new(stroke::HAIRLINE, color.gamma_multiply(0.8)),
                StrokeKind::Inside,
            );

            painter.galley(
                Pos2::new(
                    snap.edge(note_rect.min.x + padding.x),
                    snap.edge(note_rect.min.y + padding.y),
                ),
                galley,
                color,
            );
        }
    }

    /// Renders a flag marker with attached note.
    /// Features: flag pole, waving flag, optional text note.
    pub(crate) fn render_flag_note(&self, painter: &egui::Painter) {
        if self.points.is_empty() {
            return;
        }
        let snap = PixelSnap::of(painter);
        let pos = Pos2::new(snap.edge(self.points[0].x), snap.edge(self.points[0].y));
        let color = Color32::from_rgba_unmultiplied(
            self.color[0],
            self.color[1],
            self.color[2],
            self.color[3],
        );

        let pole_length = 30.0;
        painter.line_segment(
            [pos, Pos2::new(pos.x, pos.y + pole_length)],
            Stroke::new(1.5, DESIGN_TOKENS.semantic.extended.chart_text_secondary),
        );

        let flag_width = 22.0;
        let flag_height = 14.0;
        let flag_points = vec![
            pos,
            Pos2::new(pos.x + flag_width, pos.y + flag_height * 0.5),
            Pos2::new(pos.x, pos.y + flag_height),
        ];
        painter.add(egui::epaint::PathShape::convex_polygon(
            flag_points,
            color,
            Stroke::new(stroke::HAIRLINE, color),
        ));

        if let Some(text) = &self.text
            && !text.is_empty()
        {
            let font = self.annotation_font();
            let galley = painter.layout_no_wrap(text.clone(), font, color);
            let text_size = galley.size();

            let padding = egui::vec2(8.0, 5.0);
            let min = Pos2::new(
                snap.edge(pos.x + flag_width + 6.0),
                snap.edge(pos.y + flag_height * 0.5 - text_size.y * 0.5 - padding.y),
            );
            let note_rect = Rect::from_min_max(
                min,
                Pos2::new(
                    snap.edge(min.x + text_size.x + padding.x * 2.0),
                    snap.edge(min.y + text_size.y + padding.y * 2.0),
                ),
            );

            let bg = painter
                .ctx()
                .active_theme()
                .chart
                .bg_axis
                .gamma_multiply(0.92);
            painter.rect_filled(note_rect, DESIGN_TOKENS.rounding.sm, bg);
            painter.rect_stroke(
                note_rect,
                DESIGN_TOKENS.rounding.sm,
                Stroke::new(stroke::HAIRLINE, color.gamma_multiply(0.8)),
                StrokeKind::Inside,
            );
            painter.galley(
                Pos2::new(
                    snap.edge(note_rect.min.x + padding.x),
                    snap.edge(note_rect.min.y + padding.y),
                ),
                galley,
                color,
            );
        }

        painter.circle_filled(Pos2::new(pos.x, pos.y + pole_length), 3.0, color);
    }
}
