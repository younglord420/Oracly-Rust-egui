//! Callout and comment rendering

use super::contrast_ink;
use crate::drawings::domain::Drawing;
use crate::painting::pixel_snap::PixelSnap;
use crate::theme::ThemeExt;
use crate::tokens::DESIGN_TOKENS;
use egui::{Color32, Pos2, Rect, Stroke, epaint::StrokeKind};

/// Where a box saved with only its target point sits: above the target.
const LEGACY_BOX_RISE: f32 = 48.0;

impl Drawing {
    /// Centre of the box of a callout, comment, signpost, or price note: the second anchor,
    /// or above the target for a finished drawing saved with one point.
    pub(crate) fn box_anchor(&self) -> Option<Pos2> {
        match self.points.as_slice() {
            [_, center, ..] => Some(*center),
            [target] if self.completed => Some(*target - egui::vec2(0.0, LEGACY_BOX_RISE)),
            _ => None,
        }
    }

    /// A text box centred on the second anchor, with a tail to the first.
    pub(crate) fn render_callout(&self, painter: &egui::Painter) {
        self.paint_bubble(painter, 5.0, "Callout");
    }

    /// A comment is the same box with balloon corners.
    pub(crate) fn render_comment(&self, painter: &egui::Painter) {
        self.paint_bubble(painter, f32::MAX, "Comment");
    }

    fn paint_bubble(&self, painter: &egui::Painter, radius_cap: f32, placeholder: &str) {
        let Some(center) = self.box_anchor() else {
            if let Some(point) = self.points.first() {
                painter.circle_filled(
                    *point,
                    DESIGN_TOKENS.spacing.md * 0.5,
                    self.color32(),
                );
            }
            return;
        };
        let snap = PixelSnap::of(painter);
        let target = self.points[0];
        let center = Pos2::new(snap.edge(center.x), snap.edge(center.y));
        let theme = painter.ctx().active_theme();
        let fill = self
            .annotation_box_fill(theme.chart.bg_axis)
            .unwrap_or(theme.chart.bg_axis);
        if self.text_editing {
            // The inline editor is the box while it is open; keep the tail pointing into it.
            let half = egui::vec2(6.0, 0.0);
            painter.add(egui::epaint::PathShape::convex_polygon(
                vec![target, center - half, center + half],
                fill,
                Stroke::new(self.stroke_width.max(1.0), self.color32()),
            ));
            return;
        }
        let text = self
            .text
            .as_deref()
            .filter(|text| !text.is_empty())
            .unwrap_or(placeholder);
        let ink = self
            .text_color
            .map(|c| Color32::from_rgba_unmultiplied(c[0], c[1], c[2], c[3]))
            .unwrap_or_else(|| contrast_ink(fill));
        let galley = painter.layout_no_wrap(text.to_string(), self.annotation_font(), ink);
        let pad = egui::vec2(9.0, 6.0);
        let box_rect = Rect::from_center_size(center, galley.size() + pad * 2.0);
        let radius = radius_cap.min(box_rect.width() * 0.5).min(box_rect.height() * 0.5);
        painter.rect_filled(box_rect, radius, fill);
        if !box_rect.contains(target) {
            let edge = Pos2::new(
                target.x.clamp(box_rect.left(), box_rect.right()),
                target.y.clamp(box_rect.top(), box_rect.bottom()),
            );
            let half = 6.0_f32.min(box_rect.width().min(box_rect.height()) * 0.18);
            let along = if (edge.x - box_rect.left()).abs() < 0.5
                || (edge.x - box_rect.right()).abs() < 0.5
            {
                egui::vec2(0.0, half)
            } else {
                egui::vec2(half, 0.0)
            };
            painter.add(egui::epaint::PathShape::convex_polygon(
                vec![target, edge - along, edge + along],
                fill,
                Stroke::NONE,
            ));
        }
        if let Some(border) = self.annotation_border() {
            painter.rect_stroke(box_rect, radius, border, StrokeKind::Outside);
        }
        painter.galley(box_rect.min + pad, galley, ink);
    }
}
