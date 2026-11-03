//! Price action tool rendering: fair value gaps and order blocks as zones read off the bars,
//! and liquidity levels.

use crate::drawings::domain::{
    Drawing, DrawingToolType, PriceActionConfig, ZoneEnd, ZoneFormation, liquidity_state,
};
use crate::drawings::rendering::ChartView;
use crate::styles::stroke;
use egui::{Color32, Pos2, Rect, Stroke, StrokeKind};

/// Radius of the dot where a zone stopped.
const MARKER_RADIUS: f32 = 3.5;
/// How far past the sweeping bar's wick the sweep cross sits.
const SWEEP_OFFSET: f32 = 9.0;
/// Half the size of the sweep cross.
const SWEEP_HALF: f32 = 4.0;

fn rgba(color: [u8; 4]) -> Color32 {
    Color32::from_rgba_unmultiplied(color[0], color[1], color[2], color[3])
}

/// The fill's hue, opaque: the outline, halfway line and marker of a zone.
fn opaque(color: [u8; 4]) -> Color32 {
    Color32::from_rgb(color[0], color[1], color[2])
}

/// The bars of the chart a price action tool reads, and its screen geometry on them.
struct Candles<'a> {
    view: &'a ChartView<'a>,
    half: f32,
}

impl Candles<'_> {
    fn x(&self, i: usize) -> f32 {
        (self.view.bar_to_x)(i as f32)
    }

    fn y(&self, price: f64) -> f32 {
        (self.view.price_to_y)(price)
    }

    /// Screen x of the right edge of a zone stopping at bar `end`, or of the pane when it runs on.
    fn right(&self, end: Option<usize>, rect: Rect) -> f32 {
        match end {
            Some(i) => self.x(i) + self.half,
            None => rect.right().max(self.x(self.view.bars.len() - 1)),
        }
    }
}

impl Drawing {
    /// The bar the tool is placed on, clamped to the series.
    fn price_action_bar(&self, view: &ChartView) -> Option<usize> {
        let last = view.bars.len().checked_sub(1)?;
        let anchor = self.chart_points.first()?.bar_idx.round();
        anchor
            .is_finite()
            .then(|| (anchor.max(0.0) as usize).min(last))
    }

    pub(crate) fn render_price_action(
        &self,
        painter: &egui::Painter,
        rect: Rect,
        view: &ChartView,
    ) {
        let config = self.price_action_config();
        let at = self.price_action_bar(view);
        let candles = Candles {
            view,
            half: ((view.bar_to_x)(1.0) - (view.bar_to_x)(0.0)).abs().max(2.0) / 2.0,
        };
        let drawn = at.is_some_and(|at| {
            if self.tool_type == DrawingToolType::Liquidity {
                self.paint_liquidity(painter, rect, &candles, &config, at)
            } else {
                config
                    .zone(self.tool_type, view.bars, at)
                    .is_some_and(|zone| {
                        paint_zone(painter, rect, &candles, &config, &zone);
                        true
                    })
            }
        });
        // With no formation under the anchor, a faint guide still shows where it is.
        if !drawn && let Some(&anchor) = self.points.first() {
            super::utils::draw_dashed_line(
                painter,
                Pos2::new(anchor.x, rect.top()),
                Pos2::new(anchor.x, rect.bottom()),
                Stroke::new(stroke::HAIRLINE, self.color32().gamma_multiply(0.35)),
                4.0,
                4.0,
            );
        }
    }

    /// The level beyond the high or low of the bar at `at`, on the side the anchor is nearer,
    /// to the bar that took it, and a cross where it was swept.
    fn paint_liquidity(
        &self,
        painter: &egui::Painter,
        rect: Rect,
        candles: &Candles,
        config: &PriceActionConfig,
        at: usize,
    ) -> bool {
        let bars = candles.view.bars;
        let bar = &bars[at];
        let high = self
            .chart_points
            .first()
            .is_none_or(|p| p.price >= (bar.high + bar.low) / 2.0);
        let Some(state) = liquidity_state(bars, at, high) else {
            return false;
        };
        let end = match config.extend {
            ZoneEnd::Always => None,
            ZoneEnd::Broken => state.broken,
            _ => state.swept.or(state.broken),
        };
        let y = candles.y(state.price);
        painter.line_segment(
            [
                Pos2::new(candles.x(state.index), y),
                Pos2::new(candles.right(end, rect), y),
            ],
            self.stroke(),
        );
        if config.mark_sweep
            && let Some(swept) = state.swept
            && end.is_none_or(|end| swept <= end)
        {
            let wick = if high {
                bars[swept].high
            } else {
                bars[swept].low
            };
            let center = Pos2::new(
                candles.x(swept),
                candles.y(wick) + if high { -SWEEP_OFFSET } else { SWEEP_OFFSET },
            );
            let cross = Stroke::new(1.75, self.color32());
            for (dx, dy) in [(1.0, 1.0), (1.0, -1.0)] {
                let d = egui::vec2(dx, dy) * SWEEP_HALF;
                painter.line_segment([center - d, center + d], cross);
            }
        }
        true
    }
}

/// A fair value gap or order block: its box and halfway line, quieter once it has run its
/// course, a dot where it stopped, and the dashed box it carries on as once price flips it.
fn paint_zone(
    painter: &egui::Painter,
    rect: Rect,
    candles: &Candles,
    config: &PriceActionConfig,
    zone: &ZoneFormation,
) {
    let (fill, other) = if zone.bull {
        (config.up_color, config.down_color)
    } else {
        (config.down_color, config.up_color)
    };
    let left = |i: usize| candles.x(i) - candles.half;
    let (y_top, y_bottom) = {
        let (a, b) = (candles.y(zone.top), candles.y(zone.bottom));
        (a.min(b), a.max(b))
    };
    let faded = zone.end.is_some() && config.fade && zone.flip.is_none();
    let fade = |color: Color32| {
        if faded {
            color.gamma_multiply(0.6)
        } else {
            color
        }
    };
    let body = Rect::from_x_y_ranges(
        left(zone.start)..=candles.right(zone.end, rect),
        y_top..=y_bottom,
    );
    painter.rect_filled(body, 0.0, fade(rgba(fill)));
    let line = Stroke::new(stroke::HAIRLINE, fade(opaque(fill)));
    if config.border {
        painter.rect_stroke(body, 0.0, line, StrokeKind::Inside);
    }
    if config.midline {
        let y = candles.y(zone.mid);
        super::utils::draw_dashed_line(
            painter,
            Pos2::new(body.left(), y),
            Pos2::new(body.right(), y),
            line,
            5.0,
            4.0,
        );
    }
    if let Some((from, to)) = zone.flip {
        let flipped = Rect::from_x_y_ranges(left(from)..=candles.right(to, rect), y_top..=y_bottom);
        painter.rect_filled(flipped, 0.0, rgba(other));
        let edge = Stroke::new(stroke::HAIRLINE, opaque(other));
        let c = [
            flipped.left_top(),
            flipped.right_top(),
            flipped.right_bottom(),
            flipped.left_bottom(),
        ];
        for i in 0..4 {
            super::utils::draw_dashed_line(painter, c[i], c[(i + 1) % 4], edge, 5.0, 4.0);
        }
    } else if let Some(end) = zone.end {
        // The dot sits on the edge price reached: the near side, the halfway line or the far side.
        let edge = match config.extend {
            ZoneEnd::Tested if zone.bull => zone.top,
            ZoneEnd::Tested => zone.bottom,
            ZoneEnd::Half => zone.mid,
            _ if zone.bull => zone.bottom,
            _ => zone.top,
        };
        painter.circle_filled(
            Pos2::new(candles.x(end), candles.y(edge)),
            MARKER_RADIUS,
            opaque(fill),
        );
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::drawings::ChartPoint;
    use crate::model::Bar;
    use chrono::{Duration, TimeZone, Utc};
    use egui::Shape;

    fn bars() -> Vec<Bar> {
        let start = Utc.with_ymd_and_hms(2024, 1, 1, 0, 0, 0).unwrap();
        [
            [10.0, 11.0, 9.0, 10.5],
            [10.5, 14.0, 10.4, 13.8],
            [13.8, 16.0, 13.0, 15.0],
            [15.0, 15.5, 12.5, 13.5],
            [13.5, 13.6, 11.9, 12.0],
            [12.0, 12.2, 10.8, 11.5],
        ]
        .iter()
        .enumerate()
        .map(|(i, &[o, h, l, c])| Bar::new(start + Duration::hours(i as i64), o, h, l, c, 1.0))
        .collect()
    }

    fn paint(drawing: &Drawing, bars: &[Bar]) -> Vec<Shape> {
        let rect = Rect::from_min_size(Pos2::ZERO, egui::vec2(400.0, 300.0));
        let view = ChartView {
            instrument: None,
            bars,
            revision: 0,
            bar_to_x: &|i| 20.0 + i * 20.0,
            price_to_y: &|p| (300.0 - p * 10.0) as f32,
            y_to_price: &|y| (300.0 - f64::from(y)) / 10.0,
            time_axis: true,
            executions: None,
        };
        let ctx = egui::Context::default();
        let mut shapes = Vec::new();
        let output = ctx.run_ui(egui::RawInput::default(), |ui| {
            drawing.render_price_action(ui.painter(), rect, &view);
        });
        for clipped in output.shapes.iter() {
            shapes.push(clipped.shape.clone());
        }
        output.drop_without_applying_deltas();
        shapes
    }

    fn placed(tool: DrawingToolType, bar: f32, price: f64) -> Drawing {
        let mut drawing = Drawing::new(1, tool);
        drawing.chart_points = vec![ChartPoint::new(bar, price)];
        drawing.points = vec![Pos2::new(20.0 + bar * 20.0, (300.0 - price * 10.0) as f32)];
        drawing
    }

    /// A gap clicked near its middle bar paints a box from the bar before it to the bar that
    /// fills it, in the bullish colour, faded, with its halfway line and the stop marker.
    #[test]
    fn gap_paints_from_the_formation_to_its_fill() {
        let bars = bars();
        let shapes = paint(&placed(DrawingToolType::FairValueGap, 2.0, 12.0), &bars);
        let config = PriceActionConfig::for_tool(DrawingToolType::FairValueGap);
        let body = shapes
            .iter()
            .find_map(|s| match s {
                Shape::Rect(r) if r.fill.a() > 0 => Some(r.clone()),
                _ => None,
            })
            .expect("zone body");
        assert_eq!(body.fill, rgba(config.up_color).gamma_multiply(0.6));
        assert_eq!(body.rect.left(), 20.0 - 10.0);
        assert_eq!(body.rect.right(), 20.0 + 5.0 * 20.0 + 10.0);
        assert_eq!((body.rect.top(), body.rect.bottom()), (170.0, 190.0));
        assert!(shapes.iter().any(|s| matches!(s, Shape::Circle(c)
            if c.center == Pos2::new(120.0, 190.0))));
    }

    /// Off any formation the tool paints only its dashed guide; liquidity takes the side the
    /// anchor is nearer and runs to the bar that swept it.
    #[test]
    fn guides_and_liquidity_follow_the_anchor() {
        let flat: Vec<Bar> = bars().into_iter().take(1).collect();
        let shapes = paint(&placed(DrawingToolType::OrderBlock, 0.0, 10.0), &flat[..0]);
        assert!(shapes.iter().all(|s| !matches!(s, Shape::Rect(_))));
        let bars = bars();
        let level = paint(&placed(DrawingToolType::Liquidity, 3.0, 15.4), &bars);
        assert!(
            level
                .iter()
                .any(|s| matches!(s, Shape::LineSegment { points, .. }
            if points[0] == Pos2::new(80.0, 145.0)))
        );
    }
}
