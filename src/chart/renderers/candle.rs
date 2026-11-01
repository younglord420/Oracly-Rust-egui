use super::context::{BarRenderParams, RenderContext, StyleColors};
use crate::model::{Bar, CandlesStyle};
use crate::painting::pixel_snap::PixelSnap;
use crate::scales::PriceMap;
use egui::{Pos2, Rect, Stroke};

/// Renders a single candlestick with customizable body, border, and wick colors; the bar's own
/// [`colors`](Bar::colors) replace them, and a whitespace bar draws nothing. `rising` picks the
/// rising or falling colours, and `style` hides the body or the wick.
pub fn render_candle(
    context: &RenderContext,
    candle: &Bar,
    rising: bool,
    price_scale: &PriceMap,
    colors: &StyleColors,
    params: &BarRenderParams,
    style: &CandlesStyle,
) {
    if candle.is_whitespace() {
        return;
    }
    let own = candle.colors.as_deref();
    let body_color = own
        .and_then(|own| own.color)
        .unwrap_or_else(|| colors.bar_color(rising));
    let wick_color = own
        .and_then(|own| own.wick)
        .unwrap_or_else(|| colors.wick_color(rising));

    // Convert prices to screen coords
    let high_y = price_scale.price_to_y(candle.high);
    let low_y = price_scale.price_to_y(candle.low);
    let open_y = price_scale.price_to_y(candle.open);
    let close_y = price_scale.price_to_y(candle.close);

    let snap = PixelSnap::of(context.painter);

    // Draw wick (high-low line) - uses separate wick color
    if style.wick_visible {
        let wick_width = snap.line_width(params.wick_width);
        let x = snap.line_center(params.x, wick_width);
        let (top, bottom) = snap.vertical_span(high_y.min(low_y), high_y.max(low_y), 0.0);
        context.painter.line_segment(
            [Pos2::new(x, top), Pos2::new(x, bottom)],
            Stroke::new(wick_width, wick_color),
        );
    }

    // Draw body (open-close rect), at least a pixel tall
    let (left, right) = snap.bar_span(params.x, params.width);
    let (body_top, body_bottom) = snap.vertical_span(open_y.min(close_y), open_y.max(close_y), 1.0);
    let body_rect = Rect::from_min_max(Pos2::new(left, body_top), Pos2::new(right, body_bottom));

    // Draw filled body
    if style.body_visible {
        context.painter.rect_filled(body_rect, 0.0, body_color);
    }

    // Draw border if configured, inside the body so that it never widens the candle.
    if colors.has_border() {
        let border_color = own
            .and_then(|own| own.border)
            .unwrap_or_else(|| colors.border_color(rising));
        context.painter.rect_stroke(
            body_rect,
            0.0,
            Stroke::new(snap.line_width(colors.candle_border_width), border_color),
            egui::StrokeKind::Inside,
        );
    }
}
