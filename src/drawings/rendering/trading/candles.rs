//! Candles and bars that drawings paint in the shape of the chart's own candles.

use crate::drawings::rendering::ChartView;
use crate::painting::pixel_snap::PixelSnap;
use egui::{Color32, Pos2, Rect, Stroke};

/// Body width over bar spacing, as the chart's default candle width.
const BODY_RATIO: f32 = 0.6;

/// Screen x of bar `bar_idx` and the snapped width of a candle body there.
fn column(painter: &egui::Painter, view: &ChartView, bar_idx: f32) -> Option<(f32, f32)> {
    let x = (view.bar_to_x)(bar_idx);
    let spacing = ((view.bar_to_x)(bar_idx + 1.0) - x).abs();
    (x.is_finite() && spacing.is_finite())
        .then(|| (x, PixelSnap::of(painter).bar_width(spacing, BODY_RATIO)))
}

/// Paints a candle at bar `bar_idx` like the chart's candles: a body of the chart's candle width
/// on whole pixels, at least a pixel tall, and one-pixel wicks of the same colour above and below
/// it, so a translucent colour does not darken where they would overlap.
pub(super) fn paint_candle(
    painter: &egui::Painter,
    view: &ChartView,
    bar_idx: f32,
    [open, high, low, close]: [f64; 4],
    color: Color32,
) {
    let Some((x, width)) = column(painter, view, bar_idx) else {
        return;
    };
    let [open, high, low, close] = [open, high, low, close].map(view.price_to_y);
    if ![open, high, low, close].into_iter().all(f32::is_finite) {
        return;
    }
    let snap = PixelSnap::of(painter);
    let (body_top, body_bottom) = snap.vertical_span(open.min(close), open.max(close), 1.0);
    let (wick_top, wick_bottom) = snap.vertical_span(high.min(low), high.max(low), 0.0);
    let wick = snap.line_width(1.0);
    let wick_x = snap.line_center(x, wick);
    let stroke = Stroke::new(wick, color);
    if wick_top < body_top {
        painter.line_segment(
            [Pos2::new(wick_x, wick_top), Pos2::new(wick_x, body_top)],
            stroke,
        );
    }
    if wick_bottom > body_bottom {
        painter.line_segment(
            [
                Pos2::new(wick_x, body_bottom),
                Pos2::new(wick_x, wick_bottom),
            ],
            stroke,
        );
    }
    let (left, right) = snap.bar_span(x, width);
    painter.rect_filled(
        Rect::from_min_max(Pos2::new(left, body_top), Pos2::new(right, body_bottom)),
        0.0,
        color,
    );
}

/// Paints a column of the chart's candle width at bar `bar_idx` between prices `a` and `b`, at
/// least a pixel tall.
pub(super) fn paint_column(
    painter: &egui::Painter,
    view: &ChartView,
    bar_idx: f32,
    [a, b]: [f64; 2],
    color: Color32,
) {
    let Some((x, width)) = column(painter, view, bar_idx) else {
        return;
    };
    let (a, b) = ((view.price_to_y)(a), (view.price_to_y)(b));
    if !a.is_finite() || !b.is_finite() {
        return;
    }
    let snap = PixelSnap::of(painter);
    let (top, bottom) = snap.vertical_span(a.min(b), a.max(b), 1.0);
    let (left, right) = snap.bar_span(x, width);
    painter.rect_filled(
        Rect::from_min_max(Pos2::new(left, top), Pos2::new(right, bottom)),
        0.0,
        color,
    );
}
