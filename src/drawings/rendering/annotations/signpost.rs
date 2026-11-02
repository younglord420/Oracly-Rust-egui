//! Signpost rendering

use crate::drawings::domain::Drawing;
use crate::drawings::rendering::utils::draw_dashed_line;
use crate::painting::pixel_snap::PixelSnap;
use crate::tokens::DESIGN_TOKENS;
use egui::{Color32, Pos2, Rect, Stroke};

impl Drawing {
    /// Renders a vertical signpost with label at top.
    /// Features: vertical line to chart top, sign plate, date/event text.
    pub(crate) fn render_signpost(&self, painter: &egui::Painter, rect: Rect) {
        if self.points.is_empty() {
            return;
        }
        let snap = PixelSnap::of(painter);
        let pos = Pos2::new(
            snap.line_center(self.points[0].x, 1.5),
            snap.edge(self.points[0].y),
        );
        let color = Color32::from_rgba_unmultiplied(
            self.color[0],
            self.color[1],
            self.color[2],
            self.color[3],
        );

        let dash = DESIGN_TOKENS.sizing.annotation.signpost_padding_x;
        let gap = DESIGN_TOKENS.sizing.annotation.signpost_padding_y;
        if rect.min.y < pos.y {
            draw_dashed_line(
                painter,
                Pos2::new(pos.x, rect.min.y),
                pos,
                Stroke::new(1.5, color),
                dash,
                gap,
            );
        }

        let text = self.text.as_deref().unwrap_or("Event");
        let lum = 0.2126 * f32::from(color.r())
            + 0.7152 * f32::from(color.g())
            + 0.0722 * f32::from(color.b());
        let text_fg = if lum > 165.0 {
            Color32::from_rgb(18, 20, 24)
        } else {
            Color32::WHITE
        };
        let font = self.annotation_font();
        let galley = painter.layout_no_wrap(text.to_string(), font, text_fg);
        let text_size = galley.size();
        let padding = egui::vec2(
            DESIGN_TOKENS.sizing.annotation.callout_padding_x,
            DESIGN_TOKENS.sizing.annotation.text_padding_y,
        );
        let sign_min = Pos2::new(
            snap.edge(pos.x - text_size.x * 0.5 - padding.x),
            snap.edge(rect.min.y + DESIGN_TOKENS.spacing.lg),
        );
        let sign_rect = Rect::from_min_max(
            sign_min,
            Pos2::new(
                snap.edge(sign_min.x + text_size.x + padding.x * 2.0),
                snap.edge(sign_min.y + text_size.y + padding.y * 2.0),
            ),
        );

        painter.rect_filled(sign_rect, DESIGN_TOKENS.rounding.md, color);

        let arrow_size = 6.0;
        let arrow_points = vec![
            Pos2::new(pos.x, sign_rect.max.y + arrow_size),
            Pos2::new(pos.x - arrow_size, sign_rect.max.y),
            Pos2::new(pos.x + arrow_size, sign_rect.max.y),
        ];
        painter.add(egui::epaint::PathShape::convex_polygon(
            arrow_points,
            color,
            Stroke::NONE,
        ));

        painter.galley(
            Pos2::new(
                snap.edge(sign_rect.center().x - text_size.x * 0.5),
                snap.edge(sign_rect.center().y - text_size.y * 0.5),
            ),
            galley,
            text_fg,
        );

        if !self.chart_points.is_empty() {
            let bar_idx = self.chart_points[0].bar_idx as i64;
            let date_text = format!("Bar {bar_idx}");
            let date_font = self.annotation_font();
            painter.text(
                Pos2::new(
                    snap.edge(pos.x),
                    snap.edge(
                        sign_rect.max.y
                            + arrow_size
                            + DESIGN_TOKENS.sizing.annotation.signpost_bracket,
                    ),
                ),
                egui::Align2::CENTER_TOP,
                date_text,
                date_font,
                Color32::from_rgba_unmultiplied(self.color[0], self.color[1], self.color[2], 170),
            );
        }

        painter.circle_filled(pos, 3.5, color);
    }
}
