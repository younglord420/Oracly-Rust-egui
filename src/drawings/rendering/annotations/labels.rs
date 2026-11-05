//! Price label and price note rendering

use super::{axis_background, contrast_ink, paint_label_box};
use crate::drawings::domain::Drawing;
use crate::drawings::rendering::ChartView;
use crate::painting::pixel_snap::PixelSnap;
use crate::theme::ThemeExt;
use crate::scales::{
    DefaultPriceFormatter, PriceFormatter, instrument_price_precision, price_decimals,
};
use crate::tokens::DESIGN_TOKENS;
use egui::{Pos2, Rect, Stroke};

/// Price text at `precision` decimal places, keeping trailing zeros.
pub(crate) fn format_annotated_price(price: f64, precision: usize) -> String {
    DefaultPriceFormatter::with_precision(precision).format(price)
}

fn annotated_price_precision(view: &ChartView, price: f64) -> usize {
    view.instrument
        .and_then(instrument_price_precision)
        .map_or_else(|| price_decimals(price), usize::from)
}

impl Drawing {
    /// Renders a price label badge.
    /// Features: colored badge, price display, arrow indicator.
    pub(crate) fn render_price_label(&self, painter: &egui::Painter, view: &ChartView) {
        if self.points.is_empty() {
            return;
        }
        let snap = PixelSnap::of(painter);
        let pos = Pos2::new(snap.edge(self.points[0].x), snap.edge(self.points[0].y));
        let color = self.color32();

        let price_text = if !self.chart_points.is_empty() {
            let price = self.chart_points[0].price;
            format_annotated_price(price, annotated_price_precision(view, price))
        } else {
            self.text.as_deref().unwrap_or("0.00").to_string()
        };

        let text_fg = contrast_ink(color);
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

        // Seamless tag polygon connecting the pointer tip and the badge body.
        // The tag is the line colour; its ink stays high-contrast.
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

    /// A box centred on the second anchor, joined to the pinned price by a leader and a dot.
    /// The box shows that price, not typed text.
    pub(crate) fn render_price_note(&self, painter: &egui::Painter, _rect: Rect, view: &ChartView) {
        if self.points.is_empty() {
            return;
        }
        let snap = PixelSnap::of(painter);
        let tip = self.points[0];
        let color = self.color32();
        let center = self.points.get(1).copied().unwrap_or(tip);
        let center = Pos2::new(snap.edge(center.x), snap.edge(center.y));
        if self.stroke_width > 0.0 {
            painter.line_segment([tip, center], Stroke::new(self.stroke_width, color));
        }
        painter.circle_filled(tip, 3.0, color);

        let price = self.chart_points.first().map(|point| point.price);
        let price_text = price.map_or_else(
            || "0.00".to_string(),
            |price| format_annotated_price(price, annotated_price_precision(view, price)),
        );
        let theme = painter.ctx().active_theme();
        let fill = self
            .annotation_box_fill(axis_background(painter, 0.92))
            .unwrap_or(theme.chart.bg_axis);
        let ink = self
            .text_color
            .map(|c| egui::Color32::from_rgba_unmultiplied(c[0], c[1], c[2], c[3]))
            .unwrap_or_else(|| contrast_ink(fill));
        let galley = painter.layout_no_wrap(price_text, self.annotation_font(), ink);
        let pad = egui::vec2(9.0, 6.0);
        let box_rect = Rect::from_center_size(center, galley.size() + pad * 2.0);
        paint_label_box(
            painter,
            box_rect,
            5.0,
            Some(fill),
            self.annotation_border(),
        );
        painter.galley(
            Pos2::new(
                snap.edge(box_rect.center().x - galley.size().x * 0.5),
                snap.edge(box_rect.center().y - galley.size().y * 0.5),
            ),
            galley,
            ink,
        );
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::drawings::domain::{ChartPoint, DrawingToolType};
    use crate::model::Instrument;
    use egui::{Context, LayerId, Painter, Pos2, RawInput, Rect, Shape};

    fn paint(drawing: &Drawing, view: &ChartView, pane: Rect) -> Vec<Shape> {
        let ctx = Context::default();
        let output = ctx.run_ui(
            RawInput {
                screen_rect: Some(pane),
                ..Default::default()
            },
            |_| {
                drawing.render_original(
                    &Painter::new(ctx.clone(), LayerId::background(), pane),
                    pane,
                    view,
                );
            },
        );
        fn walk(shape: &Shape, out: &mut Vec<Shape>) {
            match shape {
                Shape::Vec(shapes) => shapes.iter().for_each(|s| walk(s, out)),
                _ => out.push(shape.clone()),
            }
        }
        let mut shapes = Vec::new();
        for shape in &output.shapes {
            walk(&shape.shape, &mut shapes);
        }
        output.drop_without_applying_deltas();
        shapes
    }

    /// 1.5 keeps two places at precision 2 and five at precision 5.
    #[test]
    fn price_label_uses_instrument_precision() {
        assert_eq!(format_annotated_price(1.5, 2), "1.50");
        assert_eq!(format_annotated_price(1.5, 5), "1.50000");

        let pane = Rect::from_min_size(Pos2::ZERO, egui::vec2(300.0, 200.0));
        let mut drawing = Drawing::new(1, DrawingToolType::PriceLabel);
        drawing.points = vec![Pos2::new(40.0, 80.0)];
        drawing.chart_points = vec![ChartPoint::new(1.0, 1.5)];
        for (decimals, expected) in [(2_u8, "1.50"), (5, "1.50000")] {
            let instrument = Instrument::new("TEST").with_price_decimals(decimals);
            let view = ChartView {
                instrument: Some(&instrument),
                bars: &[],
                revision: 0,
                bar_to_x: &|b| b,
                price_to_y: &|p| p as f32,
                y_to_price: &f64::from,
                time_axis: true,
                executions: None,
            };
            let shapes = paint(&drawing, &view, pane);
            assert!(
                shapes.iter().any(|shape| {
                    matches!(shape, Shape::Text(text) if text.galley.text() == expected)
                }),
                "precision {decimals}"
            );
        }
    }

    /// The price note joins the pinned price to a box that shows that price.
    #[test]
    fn price_note_shows_the_pinned_price() {
        let pane = Rect::from_min_size(Pos2::ZERO, egui::vec2(400.0, 240.0));
        let mut drawing = Drawing::new(1, DrawingToolType::PriceNote);
        drawing.points = vec![Pos2::new(40.0, 80.0), Pos2::new(180.0, 140.0)];
        drawing.chart_points = vec![ChartPoint::new(1.0, 1.5)];
        drawing.text = Some("ignore me".into());
        drawing.stroke_width = 2.0;
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
        let shapes = paint(&drawing, &view, pane);
        let expected = format_annotated_price(1.5, price_decimals(1.5));
        assert!(
            shapes.iter().any(|shape| {
                matches!(shape, Shape::Text(text) if text.galley.text() == expected)
            }),
            "painted price was not {expected}"
        );
        assert!(shapes.iter().all(|shape| {
            !matches!(shape, Shape::Text(text) if text.galley.text() == "ignore me")
        }));
        assert!(shapes.iter().any(|shape| {
            matches!(shape, Shape::LineSegment { points, .. }
                if points[0].distance(Pos2::new(40.0, 80.0)) < 1.0
                    || points[1].distance(Pos2::new(40.0, 80.0)) < 1.0)
        }));
    }
}
