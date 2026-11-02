//! Price label and price note rendering

use crate::drawings::domain::Drawing;
use crate::painting::pixel_snap::PixelSnap;
use crate::styles::stroke;
use crate::theme::ThemeExt;
use crate::tokens::DESIGN_TOKENS;
use egui::{Color32, Pos2, Rect, Stroke, epaint::StrokeKind};

impl Drawing {
    /// Renders a price label badge.
    /// Features: colored badge, price display, arrow indicator.
    pub(crate) fn render_price_label(&self, painter: &egui::Painter) {
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

        let price_text = if !self.chart_points.is_empty() {
            format!("{:.2}", self.chart_points[0].price)
        } else {
            self.text.as_deref().unwrap_or("0.00").to_string()
        };

        let lum = 0.2126 * f32::from(color.r())
            + 0.7152 * f32::from(color.g())
            + 0.0722 * f32::from(color.b());
        let text_fg = if lum > 165.0 {
            Color32::from_rgb(18, 20, 24)
        } else {
            Color32::WHITE
        };

        let font = self.annotation_font();
        let galley = painter.layout_no_wrap(price_text, font, text_fg);
        let text_size = galley.size();
        let padding = egui::vec2(
            DESIGN_TOKENS.sizing.annotation.label_padding_x,
            DESIGN_TOKENS.sizing.annotation.label_padding_y,
        );

        let arrow_width = DESIGN_TOKENS.sizing.annotation.pointer_size.round();
        let half_h = ((text_size.y + padding.y * 2.0) * 0.5).ceil();
        let box_min_x = snap.edge(pos.x + arrow_width);
        let box_max_x = snap.edge(box_min_x + text_size.x + padding.x * 2.0);
        let box_top = snap.edge(pos.y - half_h);
        let box_bottom = snap.edge(pos.y + half_h);
        let box_rect = Rect::from_min_max(
            Pos2::new(box_min_x, box_top),
            Pos2::new(box_max_x, box_bottom),
        );

        // Seamless tag polygon connecting the pointer tip and the badge body
        let tag_points = vec![
            pos,
            Pos2::new(box_rect.min.x, box_rect.min.y),
            Pos2::new(box_rect.max.x, box_rect.min.y),
            Pos2::new(box_rect.max.x, box_rect.max.y),
            Pos2::new(box_rect.min.x, box_rect.max.y),
        ];
        painter.add(egui::epaint::PathShape::convex_polygon(
            tag_points,
            color,
            Stroke::NONE,
        ));

        let text_pos = Pos2::new(
            snap.edge(box_rect.center().x - text_size.x * 0.5),
            snap.edge(box_rect.center().y - text_size.y * 0.5),
        );
        painter.galley(text_pos, galley, text_fg);
    }

    /// Render price note - horizontal price line with note box
    pub(crate) fn render_price_note(&self, painter: &egui::Painter, rect: Rect) {
        if self.points.is_empty() {
            return;
        }
        let snap = PixelSnap::of(painter);
        let pos = self.points[0];
        let color = Color32::from_rgba_unmultiplied(
            self.color[0],
            self.color[1],
            self.color[2],
            self.color[3],
        );

        let y = snap.line_center(pos.y, stroke::HAIRLINE);
        let dashed_color =
            Color32::from_rgba_unmultiplied(self.color[0], self.color[1], self.color[2], 120);
        painter.hline(
            rect.x_range(),
            y,
            Stroke::new(stroke::HAIRLINE, dashed_color),
        );

        let text = self.text.as_deref().unwrap_or("Note");
        let font = self.annotation_font();
        let galley = painter.layout_no_wrap(text.to_string(), font, color);
        let text_size = galley.size();
        let padding = egui::vec2(8.0, 4.0);
        let raw_label = self.points.get(1).copied().unwrap_or(pos);
        let label = Pos2::new(snap.edge(raw_label.x), snap.edge(raw_label.y));
        painter.line_segment([pos, label], Stroke::new(stroke::HAIRLINE, color));
        let box_rect = Rect::from_min_max(
            label,
            Pos2::new(
                snap.edge(label.x + text_size.x + padding.x * 2.0),
                snap.edge(label.y + text_size.y + padding.y * 2.0),
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
            Pos2::new(
                snap.edge(label.x + padding.x),
                snap.edge(label.y + padding.y),
            ),
            galley,
            color,
        );

        painter.circle_filled(pos, 3.0, color);
    }
}
