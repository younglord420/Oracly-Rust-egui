//! Long/short position tool rendering

use crate::drawings::domain::Drawing;
use crate::drawings::rendering::utils::draw_dashed_line;
use crate::drawings::rendering::{ChartView, bars_between};
use crate::model::Bar;
use crate::scales::{
    DefaultPriceFormatter, PriceFormatter, instrument_price_precision, price_decimals,
};
use crate::styles::stroke;
use crate::theme::ThemeExt;
use crate::tokens::DESIGN_TOKENS;
use egui::{Align2, Color32, Pos2, Rect, Stroke};

/// Opacity of the reward and risk zones.
const ZONE_ALPHA: f32 = 0.13;
/// Opacity of the part of a zone the trade has covered so far.
const COVERED_ALPHA: f32 = 0.12;
/// Gap between the box and the labels above and below it.
const LABEL_GAP: f32 = 9.0;

fn rgba(color: [u8; 4]) -> Color32 {
    Color32::from_rgba_unmultiplied(color[0], color[1], color[2], color[3])
}

/// A money amount: whole when it is, else two decimals.
fn money_text(amount: f64) -> String {
    if !amount.is_finite() {
        return "0".to_owned();
    }
    let rounded = (amount * 100.0).round() / 100.0;
    if rounded.fract() == 0.0 {
        format!("{rounded:.0}")
    } else {
        format!("{rounded:.2}")
    }
}

/// A position size: two decimals from 100, up to four from 1, four significant digits below.
fn size_text(size: f64) -> String {
    if !size.is_finite() || size == 0.0 {
        return "0".to_owned();
    }
    let text = if size >= 100.0 {
        return format!("{size:.2}");
    } else if size >= 1.0 {
        format!("{size:.4}")
    } else {
        let decimals = (3 - size.log10().floor() as i32).clamp(0, 12) as usize;
        format!("{size:.decimals$}")
    };
    text.trim_end_matches('0').trim_end_matches('.').to_owned()
}

/// Paints `text` aligned at `pos`, moved inside the painter's clip; returns its rect.
fn text_inside(
    painter: &egui::Painter,
    pos: Pos2,
    align: Align2,
    text: String,
    font: egui::FontId,
    color: Color32,
) -> Rect {
    let galley = painter.layout_no_wrap(text, font, color);
    let rect = align.anchor_size(pos, galley.size());
    let clip = painter.clip_rect();
    let shift = egui::vec2(
        (clip.right() - rect.right()).min(0.0) + (clip.left() - rect.left()).max(0.0),
        (clip.bottom() - rect.bottom()).min(0.0) + (clip.top() - rect.top()).max(0.0),
    );
    let rect = rect.translate(shift);
    painter.galley(rect.min, galley, color);
    rect
}

impl Drawing {
    /// Renders a long or short position: the reward and risk zones, the target, stop and entry
    /// lines, its direction, risk to reward, loss and size above the box, the target and stop
    /// percents inside their zones, and the result of replaying the bars after the entry.
    pub(crate) fn render_pos_tool(&self, painter: &egui::Painter, view: &ChartView) {
        let (&[entry, target, ..], Some((entry_price, target_price, stop_price))) =
            (self.points.as_slice(), self.position_prices())
        else {
            if let Some(&point) = self.points.first() {
                crate::drawings::rendering::style::placed_anchor(painter, point, self.color32());
            }
            return;
        };
        let display = self.position_display();
        let is_long = target_price >= entry_price;
        let side = if is_long { 1.0 } else { -1.0 };
        let theme = painter.ctx().active_theme();
        let (profit, loss) = (rgba(display.profit_color), rgba(display.loss_color));
        let stop_y = self
            .points
            .get(2)
            .map_or_else(|| (view.price_to_y)(stop_price), |stop| stop.y);
        let right = Self::position_right(entry, target);

        self.draw_zone(
            painter,
            entry,
            target.y,
            right,
            profit.gamma_multiply(ZONE_ALPHA),
        );
        self.draw_zone(
            painter,
            entry,
            stop_y,
            right,
            loss.gamma_multiply(ZONE_ALPHA),
        );

        // The trade's path from the entry to its exit, over the part of the box it covered.
        let entry_bar = self.chart_points[0].bar_idx.round();
        let span = (self.chart_points[1].bar_idx - self.chart_points[0].bar_idx)
            .abs()
            .round();
        let (first_after, after_entry) = if span >= 1.0 {
            bars_between(view.bars, entry_bar + 1.0, entry_bar + span).unwrap_or((0, &[]))
        } else {
            (0, &[][..])
        };
        let exit = position_exit(after_entry, entry_price, target_price, stop_price, is_long);
        if let Some((exit_price, _, exit_bar)) = exit {
            let exit_x = exit_bar.map_or(entry.x, |bar| {
                (view.bar_to_x)((first_after + bar) as f32).clamp(entry.x, right)
            });
            let exit_at = Pos2::new(exit_x, (view.price_to_y)(exit_price));
            let won = side * (exit_price - entry_price) >= 0.0;
            self.draw_zone(
                painter,
                entry,
                exit_at.y,
                exit_x,
                (if won { profit } else { loss }).gamma_multiply(COVERED_ALPHA),
            );
            draw_dashed_line(
                painter,
                entry,
                exit_at,
                Stroke::new(
                    stroke::HAIRLINE,
                    theme.chart.axis_text_secondary.gamma_multiply(0.6),
                ),
                4.0,
                5.0,
            );
        }

        let width = self.stroke_width.max(stroke::HAIRLINE);
        for (y, color) in [
            (target.y, profit),
            (stop_y, loss),
            (entry.y, self.color32()),
        ] {
            painter.line_segment(
                [Pos2::new(entry.x, y), Pos2::new(right, y)],
                Stroke::new(width, color),
            );
        }

        if !display.show_text {
            return;
        }
        let precision = view
            .instrument
            .and_then(instrument_price_precision)
            .map_or_else(|| price_decimals(entry_price), usize::from);
        let price_text =
            |price: f64| DefaultPriceFormatter::with_precision(precision).format(price);
        let percent = |price: f64| {
            if entry_price == 0.0 {
                0.0
            } else {
                (price - entry_price).abs() / entry_price.abs() * 100.0
            }
        };
        let risk = (entry_price - stop_price).abs();
        let ratio = if risk < 1e-12 {
            0.0
        } else {
            (target_price - entry_price).abs() / risk
        };
        let (loss_amount, size) = self.position_loss_and_size();
        let text_color = self.text_color.map_or(theme.chart.axis_text, rgba);
        let font = self.annotation_font();
        let small = egui::FontId::proportional((font.size - 1.0).max(10.0));
        let center_x = (entry.x + right) / 2.0;
        let top = entry.y.min(target.y).min(stop_y) - LABEL_GAP;
        let bottom = entry.y.max(target.y).max(stop_y) + LABEL_GAP;

        let mut above = Vec::new();
        if display.show_header {
            above.push((
                format!(
                    "{}  \u{b7}  R:R {ratio:.2}",
                    if is_long { "LONG" } else { "SHORT" }
                ),
                font.clone(),
            ));
        }
        if display.show_loss_size {
            above.push((
                format!(
                    "Loss {}  \u{b7}  Size {}",
                    money_text(loss_amount),
                    size_text(size)
                ),
                small.clone(),
            ));
        }
        let mut y = top;
        for (text, font) in above.into_iter().rev() {
            let rect = text_inside(
                painter,
                Pos2::new(center_x, y),
                Align2::CENTER_BOTTOM,
                text,
                font,
                text_color,
            );
            y = rect.top() - 2.0;
        }
        let level = |label: &str, sign: char, price: f64| {
            let mut text = format!("{label} {sign}{:.2}%", percent(price));
            if display.show_prices {
                text.push_str(&format!("  @ {}", price_text(price)));
            }
            text
        };
        if display.show_target_label {
            text_inside(
                painter,
                Pos2::new(center_x, (entry.y + target.y) / 2.0),
                Align2::CENTER_CENTER,
                level("Target", '+', target_price),
                small.clone(),
                text_color,
            );
        }
        if display.show_stop_label {
            text_inside(
                painter,
                Pos2::new(center_x, (entry.y + stop_y) / 2.0),
                Align2::CENTER_CENTER,
                level("Stop", '\u{2212}', stop_price),
                small.clone(),
                text_color,
            );
        }
        if let Some((exit_price, closed, _)) = exit {
            let change = side * (exit_price - entry_price);
            let pnl = change * size;
            let sign = if pnl >= 0.0 { "+" } else { "\u{2212}" };
            text_inside(
                painter,
                Pos2::new(center_x, bottom),
                Align2::CENTER_TOP,
                format!(
                    "{} P&L {sign}{} ({sign}{:.2}%)",
                    if closed { "Closed" } else { "Open" },
                    money_text(pnl.abs()),
                    percent(exit_price),
                ),
                small,
                if change >= 0.0 { profit } else { loss },
            );
        }
    }

    /// Screen x of the right edge of a position's box: as far right of the entry as the target
    /// anchor is, and at least the minimum width.
    pub(crate) fn position_right(entry: Pos2, target: Pos2) -> f32 {
        entry.x
            + (target.x - entry.x)
                .abs()
                .max(DESIGN_TOKENS.sizing.position_tool.min_width)
    }

    /// Fills the band between the entry line and `level_y`, from the entry to `right`.
    fn draw_zone(
        &self,
        painter: &egui::Painter,
        entry: Pos2,
        level_y: f32,
        right: f32,
        color: Color32,
    ) {
        let zone = Rect::from_two_pos(entry, Pos2::new(right, level_y));
        // Visibility check: skip rendering if rect is outside clip region
        if !painter.clip_rect().intersects(zone) {
            return;
        }
        painter.rect_filled(zone, 0.0, color);
    }
}

/// Price the position exits at, whether it has closed, and the index in `bars` of the bar it
/// exits on (`None` when no bar follows the entry). The first bar that
/// trades through the stop or the target closes it; when one bar reaches both,
/// the stop is taken, since the order within a bar is unknown. Otherwise the
/// position is open at the last close (at the entry when no bar follows it).
/// `None` unless the stop and the target lie on the losing and the winning
/// side of the entry: an order on the wrong side of the market fills at the
/// market, not at its level.
fn position_exit(
    bars: &[Bar],
    entry: f64,
    target: f64,
    stop: f64,
    is_long: bool,
) -> Option<(f64, bool, Option<usize>)> {
    let ordered = if is_long {
        stop < entry && entry < target
    } else {
        target < entry && entry < stop
    };
    if !ordered {
        return None;
    }
    for (i, bar) in bars.iter().enumerate() {
        let (stop_hit, target_hit) = if is_long {
            (bar.low <= stop, bar.high >= target)
        } else {
            (bar.high >= stop, bar.low <= target)
        };
        if stop_hit {
            return Some((stop, true, Some(i)));
        }
        if target_hit {
            return Some((target, true, Some(i)));
        }
    }
    let last = bars.iter().rposition(|bar| !bar.is_whitespace());
    Some((last.map_or(entry, |i| bars[i].close), false, last))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::drawings::rendering::sample_bars;

    /// The header names the direction from the levels with the reward ratio, the account risk
    /// gives the loss and size, the zones carry their percents, and the replayed bars report
    /// the closed result; every label stays inside the plot.
    #[test]
    fn position_labels_follow_levels_account_and_replay() {
        use crate::drawings::{ChartPoint, DrawingToolType, PositionSize};
        use crate::widget::test_frames::chart;
        for (tool, target, stop, low, high, header, pnl) in [
            (
                DrawingToolType::LongPos,
                105.0,
                97.5,
                99.0,
                106.0,
                "LONG  \u{b7}  R:R 2.00",
                "Closed P&L +600",
            ),
            (
                DrawingToolType::ShortPos,
                95.0,
                102.5,
                94.0,
                101.0,
                "SHORT  \u{b7}  R:R 2.00",
                "Closed P&L +600",
            ),
            // A long placed with the short tool reads as long.
            (
                DrawingToolType::ShortPos,
                105.0,
                97.5,
                99.0,
                106.0,
                "LONG  \u{b7}  R:R 2.00",
                "Closed P&L +600",
            ),
        ] {
            let mut drawing = Drawing::new(1, tool);
            drawing.position_size = Some(PositionSize::RiskPercent {
                account_size: 10_000.0,
                percent: 3.0,
            });
            drawing.chart_points = vec![
                ChartPoint::new(30.0, 100.0),
                ChartPoint::new(50.0, target),
                ChartPoint::new(50.0, stop),
            ];
            drawing.completed = true;
            let ctx = egui::Context::default();
            let mut chart = chart(&[low], &[high]);
            let render = |chart: &mut crate::Chart| {
                ctx.run_ui(
                    egui::RawInput {
                        screen_rect: Some(Rect::from_min_size(
                            Pos2::ZERO,
                            egui::vec2(800.0, 600.0),
                        )),
                        ..Default::default()
                    },
                    |ui| {
                        chart.show(ui);
                    },
                )
            };
            render(&mut chart).drop_without_applying_deltas();
            chart.set_visible_logical_range(crate::model::LogicalRange {
                left: 10.0,
                right: 70.0,
            });
            chart.drawings_mut().drawings.push(drawing.clone());
            let output = render(&mut chart);
            let texts: Vec<_> = output
                .shapes
                .iter()
                .filter_map(|s| match &s.shape {
                    egui::Shape::Text(t) => {
                        Some((t.galley.job.text.clone(), t.visual_bounding_rect()))
                    }
                    _ => None,
                })
                .collect();
            output.drop_without_applying_deltas();
            for label in [
                header,
                "Loss 300  \u{b7}  Size 120",
                "Target +5.00%",
                "Stop \u{2212}2.50%",
                pnl,
            ] {
                let (_, rect) = texts
                    .iter()
                    .find(|(text, _)| text.starts_with(label))
                    .unwrap_or_else(|| panic!("{tool:?}: missing {label} in {texts:?}"));
                assert!(
                    chart.get_rendered_price_rect().contains_rect(*rect),
                    "{tool:?}: {label} at {rect:?}"
                );
            }
        }
    }

    /// Expected exits read off the sample bars by hand.
    #[test]
    fn position_exit_replays_bars_after_entry() {
        let bars = sample_bars();
        let after_bar_0 = &bars[1..=5];
        let cases = [
            // Long from 11: bar 4 (high 15) reaches 14.2 before any low reaches 9.5.
            (
                after_bar_0,
                11.0,
                14.2,
                9.5,
                true,
                Some((14.2, true, Some(3))),
            ),
            // Long from 11: bar 1 (low 10) reaches the stop at 10.5 first.
            (
                after_bar_0,
                11.0,
                14.2,
                10.5,
                true,
                Some((10.5, true, Some(0))),
            ),
            // Bar 1 (low 10, high 13) reaches both 10.0 and 12.8: the stop is taken.
            (
                after_bar_0,
                11.0,
                12.8,
                10.0,
                true,
                Some((10.0, true, Some(0))),
            ),
            // Neither level is reached: open at the close of bar 5.
            (
                after_bar_0,
                11.0,
                20.0,
                5.0,
                true,
                Some((12.5, false, Some(4))),
            ),
            // Short from 13.5 at bar 3: bar 5 (low 12) reaches 12.2, no high reaches 15.5.
            (
                &bars[4..=5],
                13.5,
                12.2,
                15.5,
                false,
                Some((12.2, true, Some(1))),
            ),
            // No bar after the entry yet.
            (
                &bars[..0],
                13.0,
                14.0,
                12.0,
                true,
                Some((13.0, false, None)),
            ),
            // Long dragged down: target below the entry, stop mirrored above it.
            (&bars[2..=2], 12.0, 11.5, 12.5, true, None),
            // Short dragged up: target above the entry, stop mirrored below it.
            (&bars[2..=2], 12.0, 12.5, 11.5, false, None),
            // Long whose stop was moved above the entry.
            (after_bar_0, 11.0, 14.2, 11.5, true, None),
        ];
        for (after, entry, target, stop, is_long, want) in cases {
            assert_eq!(position_exit(after, entry, target, stop, is_long), want);
        }
    }
}
