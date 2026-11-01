use super::context::{BarRenderParams, RenderContext, StyleColors};
use crate::model::Bar;
use crate::painting::pixel_snap::PixelSnap;
use crate::theme::ThemeExt;
use egui::{Pos2, Rect};

/// Renders a volume bar with the active theme's volume colour and opacity.
pub fn render_volume_bar(
    context: &RenderContext,
    candle: &Bar,
    max_volume: f64,
    _colors: &StyleColors,
    params: &BarRenderParams,
) {
    if candle.is_whitespace() {
        return;
    }
    let theme = context.painter.ctx().active_theme();
    let color = if candle.is_bullish() {
        theme.chart.volume_up()
    } else {
        theme.chart.volume_down()
    };

    let bar_height = (candle.volume / max_volume) as f32 * context.rect.height();
    let bar_bottom = context.rect.max.y;
    let snap = PixelSnap::of(context.painter);
    let (left, right) = snap.bar_span(params.x, params.width);
    let top = snap.edge(bar_bottom - bar_height);

    context.painter.rect_filled(
        Rect::from_min_max(Pos2::new(left, top), Pos2::new(right, snap.edge(bar_bottom))),
        0.0,
        color,
    );
}
