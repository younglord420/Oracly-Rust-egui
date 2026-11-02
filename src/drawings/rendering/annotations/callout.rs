//! Callout and comment rendering

use crate::drawings::domain::Drawing;
use crate::painting::pixel_snap::PixelSnap;
use crate::styles::stroke;
use crate::theme::ThemeExt;
use crate::tokens::DESIGN_TOKENS;
use egui::{Color32, Pos2, Rect, Stroke, epaint::StrokeKind};

impl Drawing {
    /// Renders a callout with pointer line and auto-positioning.
    /// Features: text box with arrow pointer, background, border.
    pub(crate) fn render_callout(&self, painter: &egui::Painter) {
        if self.points.len() < 2 {
            if self.points.len() == 1 {
                let color = Color32::from_rgba_unmultiplied(
                    self.color[0],
                    self.color[1],
                    self.color[2],
                    self.color[3],
                );
                painter.circle_filled(self.points[0], 3.0, color);
            }
            return;
        }
        let snap = PixelSnap::of(painter);
        let anchor = self.points[0];
        let box_pos = Pos2::new(snap.edge(self.points[1].x), snap.edge(self.points[1].y));
        let color = Color32::from_rgba_unmultiplied(
            self.color[0],
            self.color[1],
            self.color[2],
            self.color[3],
        );

        let text = self.text.as_deref().unwrap_or("Callout text here");
        let font = self.annotation_font();
        let galley = painter.layout_no_wrap(text.to_string(), font, color);
        let text_size = galley.size();
        let padding = egui::vec2(
            DESIGN_TOKENS.sizing.annotation.callout_padding_x,
            DESIGN_TOKENS.sizing.annotation.callout_padding_y,
        );
        let box_rect = Rect::from_min_max(
            box_pos,
            Pos2::new(
                snap.edge(box_pos.x + text_size.x + padding.x * 2.0),
                snap.edge(box_pos.y + text_size.y + padding.y * 2.0),
            ),
        );

        let bg = painter
            .ctx()
            .active_theme()
            .chart
            .bg_axis
            .gamma_multiply(0.94);

        let box_center = box_rect.center();
        let dx = anchor.x - box_center.x;
        let dy = anchor.y - box_center.y;

        let pointer_pos =
            if dx.abs() / box_rect.width().max(1.0) > dy.abs() / box_rect.height().max(1.0) {
                if dx > 0.0 {
                    Pos2::new(box_rect.max.x, box_center.y)
                } else {
                    Pos2::new(box_rect.min.x, box_center.y)
                }
            } else if dy > 0.0 {
                Pos2::new(box_center.x, box_rect.max.y)
            } else {
                Pos2::new(box_center.x, box_rect.min.y)
            };

        let pointer_size = DESIGN_TOKENS.sizing.annotation.pointer_size;
        let perpendicular = if (pointer_pos.x - box_rect.min.x).abs() < 1.0
            || (pointer_pos.x - box_rect.max.x).abs() < 1.0
        {
            egui::vec2(0.0, pointer_size)
        } else {
            egui::vec2(pointer_size, 0.0)
        };

        let p_left = Pos2::new(
            pointer_pos.x - perpendicular.x * 0.5,
            pointer_pos.y - perpendicular.y * 0.5,
        );
        let p_right = Pos2::new(
            pointer_pos.x + perpendicular.x * 0.5,
            pointer_pos.y + perpendicular.y * 0.5,
        );
        let border_stroke = Stroke::new(stroke::HAIRLINE, color.gamma_multiply(0.85));

        painter.add(egui::epaint::PathShape::convex_polygon(
            vec![anchor, p_left, p_right],
            bg,
            border_stroke,
        ));

        painter.rect_filled(box_rect, DESIGN_TOKENS.rounding.md, bg);
        painter.rect_stroke(
            box_rect,
            DESIGN_TOKENS.rounding.md,
            border_stroke,
            StrokeKind::Inside,
        );

        painter.galley(
            Pos2::new(
                snap.edge(box_pos.x + padding.x),
                snap.edge(box_pos.y + padding.y),
            ),
            galley,
            color,
        );

        painter.circle_filled(anchor, 3.0, color);
    }

    /// Renders a comment marker with expandable bubble.
    /// Features: speech bubble icon, expandable text, typing indicator.
    pub(crate) fn render_comment(&self, painter: &egui::Painter) {
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

        let bubble_width = 24.0;
        let bubble_height = 18.0;
        let bubble_rect = Rect::from_center_size(pos, egui::vec2(bubble_width, bubble_height));

        painter.rect_filled(bubble_rect, DESIGN_TOKENS.rounding.md, color);

        let tail_points = vec![
            Pos2::new(bubble_rect.left() + 4.0, bubble_rect.bottom() - 1.0),
            Pos2::new(bubble_rect.left() - 2.0, bubble_rect.bottom() + 5.0),
            Pos2::new(bubble_rect.left() + 10.0, bubble_rect.bottom() - 1.0),
        ];
        painter.add(egui::epaint::PathShape::convex_polygon(
            tail_points,
            color,
            Stroke::NONE,
        ));

        let dot_y = pos.y;
        for i in 0..3 {
            let dot_x = pos.x - 5.0 + i as f32 * 5.0;
            painter.circle_filled(Pos2::new(dot_x, dot_y), 1.5, Color32::WHITE);
        }

        if let Some(text) = &self.text
            && !text.is_empty()
        {
            let font = self.annotation_font();
            let galley = painter.layout_no_wrap(text.clone(), font, color);
            let text_size = galley.size();

            let padding = egui::vec2(10.0, 6.0);
            let min = Pos2::new(
                snap.edge(pos.x + bubble_width * 0.5 + 8.0),
                snap.edge(pos.y - text_size.y * 0.5 - padding.y),
            );
            let comment_rect = Rect::from_min_max(
                min,
                Pos2::new(
                    snap.edge(min.x + text_size.x + padding.x * 2.0),
                    snap.edge(min.y + text_size.y + padding.y * 2.0),
                ),
            );

            painter.line_segment(
                [
                    Pos2::new(bubble_rect.max.x, pos.y),
                    Pos2::new(comment_rect.min.x, comment_rect.center().y),
                ],
                Stroke::new(stroke::HAIRLINE, color.gamma_multiply(0.45)),
            );

            let bg = painter
                .ctx()
                .active_theme()
                .chart
                .bg_axis
                .gamma_multiply(0.94);
            painter.rect_filled(comment_rect, DESIGN_TOKENS.rounding.md, bg);
            painter.rect_stroke(
                comment_rect,
                DESIGN_TOKENS.rounding.md,
                Stroke::new(stroke::HAIRLINE, color.gamma_multiply(0.8)),
                StrokeKind::Inside,
            );

            painter.galley(
                Pos2::new(
                    snap.edge(comment_rect.min.x + padding.x),
                    snap.edge(comment_rect.min.y + padding.y),
                ),
                galley,
                color,
            );
        }
    }
}
