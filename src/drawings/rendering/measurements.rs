//! Measurement tool rendering implementations
//!
//! Includes: measure, price range, date range, date and price range, info line, and bars pattern.

use super::{ChartView, bars_between};
use crate::drawings::domain::{Drawing, DrawingToolType};
use crate::model::{Bar, percent_change};
use crate::painting::pixel_snap::PixelSnap;
use crate::scales::price_decimals;
use crate::scales::{PriceFormatter, VolumeFormatter};
use crate::styles::{stroke, typography};
use crate::theme::ThemeExt;
use crate::tokens::DESIGN_TOKENS;
use egui::{Color32, FontId, Pos2, Rect, Stroke, epaint::StrokeKind};

impl Drawing {
    pub(crate) fn render_measure(&self, painter: &egui::Painter, view: &ChartView) {
        // Need at least 1 point (preview adds 2nd point via update_last_point)
        if self.points.is_empty() {
            return;
        }

        // Show semi-transparent rect if we have a second point (during preview or after completion)
        if self.points.len() >= 2 && self.chart_points.len() >= 2 {
            let start = self.points[0];
            let end = self.points[1];
            let cp1 = &self.chart_points[0];
            let cp2 = &self.chart_points[1];

            // Calculate price change to determine color (RED for down, BLUE for up)
            let price_diff = cp2.price - cp1.price;
            let is_measuring_down = price_diff < 0.0;

            // The default line colour reads as direction: red down, blue up.
            let default_color = [
                crate::drawings::manager::DEFAULT_DRAWING_COLOR_DARK,
                crate::drawings::manager::DEFAULT_DRAWING_COLOR_LIGHT,
            ]
            .contains(&self.color);
            let base_color = if !default_color {
                self.color32().to_opaque()
            } else if is_measuring_down {
                DESIGN_TOKENS.semantic.extended.bearish
            } else {
                DESIGN_TOKENS.semantic.extended.accent
            };

            // Draw SEMI-TRANSPARENT RECTANGLE filling the area from start to cursor
            // Color changes based on direction: red down, blue up
            let rect = egui::Rect::from_two_pos(start, end);
            let fill_color = self.fill_color32().unwrap_or_else(|| {
                Color32::from_rgba_unmultiplied(
                    base_color.r(),
                    base_color.g(),
                    base_color.b(),
                    30, // Very transparent
                )
            });
            painter.rect_filled(rect, 0.0, fill_color);

            // Draw thin border lines around the rect
            let border_color = Color32::from_rgba_unmultiplied(
                base_color.r(),
                base_color.g(),
                base_color.b(),
                100,
            );
            painter.rect_stroke(
                rect,
                0.0,
                Stroke::new(stroke::HAIRLINE, border_color),
                egui::epaint::StrokeKind::Outside,
            );

            // Draw cross lines inside rect (+ shape with arrows)
            let snap = crate::painting::pixel_snap::PixelSnap::of(painter);
            let center_x = snap.line_center((rect.min.x + rect.max.x) / 2.0, stroke::HAIRLINE);
            let center_y = snap.line_center((rect.min.y + rect.max.y) / 2.0, stroke::HAIRLINE);
            let line_color = Color32::from_rgba_unmultiplied(
                base_color.r(),
                base_color.g(),
                base_color.b(),
                170,
            );
            let line_stroke = Stroke::new(stroke::HAIRLINE, line_color);

            // Vertical line (from top to bottom)
            painter.line_segment(
                [
                    Pos2::new(center_x, rect.min.y),
                    Pos2::new(center_x, rect.max.y),
                ],
                line_stroke,
            );

            // Horizontal line (from left to right)
            painter.line_segment(
                [
                    Pos2::new(rect.min.x, center_y),
                    Pos2::new(rect.max.x, center_y),
                ],
                line_stroke,
            );

            // Draw arrow at right end of horizontal line (pointing right)
            let arrow_size = 8.0;
            let h_arrow_tip = Pos2::new(rect.max.x, center_y);
            let h_arrow_left = h_arrow_tip + egui::vec2(-arrow_size, -arrow_size * 0.4);
            let h_arrow_right = h_arrow_tip + egui::vec2(-arrow_size, arrow_size * 0.4);
            painter.add(egui::Shape::convex_polygon(
                vec![h_arrow_tip, h_arrow_left, h_arrow_right],
                line_color,
                Stroke::NONE,
            ));

            // Draw arrow at top/bottom of vertical line based on direction
            if price_diff < 0.0 {
                // Measuring down: arrow at bottom pointing down
                let v_arrow_tip = Pos2::new(center_x, rect.max.y);
                let v_arrow_left = v_arrow_tip + egui::vec2(-arrow_size * 0.4, -arrow_size);
                let v_arrow_right = v_arrow_tip + egui::vec2(arrow_size * 0.4, -arrow_size);
                painter.add(egui::Shape::convex_polygon(
                    vec![v_arrow_tip, v_arrow_left, v_arrow_right],
                    line_color,
                    Stroke::NONE,
                ));
            } else {
                // Measuring up: arrow at top pointing up
                let v_arrow_tip = Pos2::new(center_x, rect.min.y);
                let v_arrow_left = v_arrow_tip + egui::vec2(-arrow_size * 0.4, arrow_size);
                let v_arrow_right = v_arrow_tip + egui::vec2(arrow_size * 0.4, arrow_size);
                painter.add(egui::Shape::convex_polygon(
                    vec![v_arrow_tip, v_arrow_left, v_arrow_right],
                    line_color,
                    Stroke::NONE,
                ));
            }

            // Draw + symbol at center where lines cross (same color and thickness as cross lines)
            let plus_size = 6.0;
            painter.line_segment(
                [
                    Pos2::new(center_x - plus_size, center_y),
                    Pos2::new(center_x + plus_size, center_y),
                ],
                line_stroke,
            );
            painter.line_segment(
                [
                    Pos2::new(center_x, center_y - plus_size),
                    Pos2::new(center_x, center_y + plus_size),
                ],
                line_stroke,
            );

            let info = self
                .measurement_label(view.bars, view.instrument, view.time_axis)
                .unwrap_or_default();

            // Draw info box at the top-left corner of rect
            let font = self.annotation_font();
            let ink = self.label_text_on(base_color);
            let galley = painter.layout_no_wrap(info.clone(), font.clone(), ink);
            let text_size = galley.size();

            // Pos box near the top-left corner of rect
            let box_x = rect.min.x.min(rect.max.x) + 5.0;
            let box_y = rect.min.y.min(rect.max.y) - text_size.y - 15.0;

            // Draw background (solid color matching direction: red for down, blue for up)
            let padding = egui::vec2(8.0, 6.0);
            let size = text_size + padding * 2.0;
            let clip = painter.clip_rect();
            let pos = Pos2::new(box_x, box_y).clamp(clip.min, clip.max - size.min(clip.size()));
            let (box_x, box_y) = (snap.edge(pos.x), snap.edge(pos.y));
            let box_rect = egui::Rect::from_min_size(Pos2::new(box_x, box_y), size);
            painter.rect_filled(box_rect, DESIGN_TOKENS.rounding.md, base_color);

            // Draw text with padding
            painter.galley(
                Pos2::new(snap.edge(box_x + padding.x), snap.edge(box_y + padding.y)),
                galley,
                ink,
            );
        }
    }

    /// The line stroke of a range tool: its colour at its own width.
    fn range_stroke(&self) -> Stroke {
        Stroke::new(self.stroke_width.max(stroke::HAIRLINE), self.color32())
    }

    /// The box a range tool shades: its fill, or its line colour faintly.
    fn range_fill(&self) -> Color32 {
        self.fill_color32().unwrap_or_else(|| {
            Color32::from_rgba_unmultiplied(self.color[0], self.color[1], self.color[2], 30)
        })
    }

    /// A line from `from` to `to` with a filled head at `to`.
    fn range_arrow(&self, painter: &egui::Painter, from: Pos2, to: Pos2) {
        let line = self.range_stroke();
        let along = to - from;
        let length = along.length();
        if length < 1.0 {
            return;
        }
        let dir = along / length;
        let head = (6.0 + line.width * 2.0).min(length * 0.5);
        let base = to - dir * head;
        let across = dir.rot90() * head * 0.45;
        painter.line_segment([from, base], line);
        painter.add(egui::Shape::convex_polygon(
            vec![to, base + across, base - across],
            line.color,
            Stroke::NONE,
        ));
    }

    /// Price range, date range or date and price range: the box between the anchors with its
    /// edges, the arrows of what it measures, and the readout beyond the second anchor.
    fn render_range_box(&self, painter: &egui::Painter, view: &ChartView, price: bool, time: bool) {
        let &[p1, p2, ..] = self.points.as_slice() else {
            if let Some(&point) = self.points.first() {
                super::style::placed_anchor(painter, point, self.color32());
            }
            return;
        };
        let rect = Rect::from_two_pos(p1, p2);
        painter.rect_filled(rect, 0.0, self.range_fill());
        let line = self.range_stroke();
        if price {
            for y in [p1.y, p2.y] {
                painter.line_segment(
                    [Pos2::new(rect.left(), y), Pos2::new(rect.right(), y)],
                    line,
                );
            }
        }
        if time {
            for x in [p1.x, p2.x] {
                painter.line_segment(
                    [Pos2::new(x, rect.top()), Pos2::new(x, rect.bottom())],
                    line,
                );
            }
        }
        let center = rect.center();
        if price {
            self.range_arrow(
                painter,
                Pos2::new(center.x, p1.y),
                Pos2::new(center.x, p2.y),
            );
        }
        if time {
            self.range_arrow(
                painter,
                Pos2::new(p1.x, center.y),
                Pos2::new(p2.x, center.y),
            );
        }
        let info = self
            .measurement_label(view.bars, view.instrument, view.time_axis)
            .unwrap_or_default();
        if info.is_empty() {
            return;
        }
        // The readout sits beyond the second anchor's side, inside the pane where it can.
        let below = p2.y >= p1.y;
        let gap = DESIGN_TOKENS.spacing.sm;
        let anchor = if below {
            Pos2::new(center.x, rect.bottom() + gap)
        } else {
            Pos2::new(center.x, rect.top() - gap)
        };
        let card_color = self.color32().to_opaque();
        let ink = self.label_text_on(card_color);
        let galley = painter.layout_no_wrap(info, self.annotation_font(), ink);
        let pad = egui::vec2(8.0, 6.0);
        let size = galley.size() + pad * 2.0;
        let align = if below {
            egui::Align2::CENTER_TOP
        } else {
            egui::Align2::CENTER_BOTTOM
        };
        let clip = painter.clip_rect();
        let mut card = align.anchor_size(anchor, size);
        card = card.translate(egui::vec2(
            (clip.right() - card.right()).min(0.0) + (clip.left() - card.left()).max(0.0),
            (clip.bottom() - card.bottom()).min(0.0) + (clip.top() - card.top()).max(0.0),
        ));
        painter.rect_filled(card, DESIGN_TOKENS.rounding.md, card_color);
        painter.galley(card.min + pad, galley, ink);
    }

    /// Price range: the box between two prices, a vertical arrow and the price change.
    pub(crate) fn render_price_range(
        &self,
        painter: &egui::Painter,
        _chart_rect: Rect,
        view: &ChartView,
    ) {
        self.render_range_box(painter, view, true, false);
    }

    /// Date range: the box between two times, a horizontal arrow and the bars, time and volume.
    pub(crate) fn render_date_range(
        &self,
        painter: &egui::Painter,
        _chart_rect: Rect,
        view: &ChartView,
    ) {
        self.render_range_box(painter, view, false, true);
    }

    /// Date and price range: the box between the anchors, both arrows and the full readout.
    pub(crate) fn render_date_and_price_range(&self, painter: &egui::Painter, view: &ChartView) {
        self.render_range_box(painter, view, true, true);
    }

    pub(crate) fn render_info_line(&self, painter: &egui::Painter, view: &ChartView) {
        let &[start, end, ..] = self.points.as_slice() else {
            return;
        };
        self.styled_segment(
            painter,
            start,
            end,
            Stroke::new(self.stroke_width, self.color32()),
        );

        let Some(stats) = self.info_line_stats(view.bars, view.instrument, view.time_axis) else {
            return;
        };
        let theme = painter.ctx().active_theme();
        let chart = theme.chart;
        let rising = self
            .chart_points
            .get(1)
            .zip(self.chart_points.first())
            .is_some_and(|(b, a)| b.price >= a.price);
        let accent = if rising {
            chart.candle_up
        } else {
            chart.candle_down
        };

        let snap = PixelSnap::of(painter);
        let primary_font = FontId::proportional(typography::SM_MD);
        let secondary_font = FontId::proportional(typography::XS);
        let primary = painter.layout_no_wrap(stats.change, primary_font, accent);
        let bars = painter.layout_no_wrap(
            stats.span,
            secondary_font.clone(),
            chart.axis_text_secondary,
        );
        let angle = painter.layout_no_wrap(stats.angle, secondary_font, chart.axis_text_secondary);

        let pad = egui::vec2(10.0, 8.0);
        let accent_w = 3.0;
        let row_gap = 2.0;
        let content_w = primary.size().x.max(bars.size().x).max(angle.size().x);
        let content_h = primary.size().y + bars.size().y + angle.size().y + row_gap * 2.0;
        let size = egui::vec2(
            (accent_w + pad.x * 2.0 + content_w).ceil(),
            (pad.y * 2.0 + content_h).ceil(),
        );

        let mid = Pos2::new((start.x + end.x) * 0.5, (start.y + end.y) * 0.5);
        // Prefer above a rising line and below a falling one so the card clears the stroke.
        let prefer_above = end.y <= start.y;
        let gap = 14.0;
        let raw = Pos2::new(
            mid.x - size.x * 0.5,
            if prefer_above {
                mid.y - size.y - gap
            } else {
                mid.y + gap
            },
        );
        let clip = painter.clip_rect();
        let raw = raw.clamp(clip.min, clip.max - size.min(clip.size()));
        let box_rect = Rect::from_min_size(Pos2::new(snap.edge(raw.x), snap.edge(raw.y)), size);

        // Soft lift under the card.
        let shadow = box_rect.translate(egui::vec2(0.0, 2.0));
        painter.rect_filled(
            shadow,
            DESIGN_TOKENS.rounding.md,
            Color32::from_black_alpha(90),
        );
        painter.rect_filled(box_rect, DESIGN_TOKENS.rounding.md, chart.bg_tooltip);
        painter.rect_stroke(
            box_rect,
            DESIGN_TOKENS.rounding.md,
            Stroke::new(stroke::HAIRLINE, theme.ui.border.gamma_multiply(0.85)),
            StrokeKind::Inside,
        );

        // Directional accent along the leading edge.
        let radius = DESIGN_TOKENS.rounding.md as u8;
        let accent_rect = Rect::from_min_max(
            Pos2::new(snap.edge(box_rect.min.x), snap.edge(box_rect.min.y)),
            Pos2::new(
                snap.edge(box_rect.min.x + accent_w),
                snap.edge(box_rect.max.y),
            ),
        );
        painter.rect_filled(
            accent_rect,
            egui::CornerRadius {
                nw: radius,
                sw: radius,
                ne: 0,
                se: 0,
            },
            accent,
        );

        let text_x = snap.edge(box_rect.min.x + accent_w + pad.x);
        let mut text_y = snap.edge(box_rect.min.y + pad.y);
        let primary_h = primary.size().y;
        let bars_h = bars.size().y;
        painter.galley(Pos2::new(text_x, text_y), primary, accent);
        text_y = snap.edge(text_y + primary_h + row_gap);
        painter.galley(Pos2::new(text_x, text_y), bars, chart.axis_text_secondary);
        text_y = snap.edge(text_y + bars_h + row_gap);
        painter.galley(Pos2::new(text_x, text_y), angle, chart.axis_text_secondary);

        let connector_start = Pos2::new(
            box_rect.center().x,
            if prefer_above {
                box_rect.max.y
            } else {
                box_rect.min.y
            },
        );
        painter.line_segment(
            [connector_start, mid],
            Stroke::new(
                stroke::HAIRLINE,
                Color32::from_rgba_unmultiplied(accent.r(), accent.g(), accent.b(), 140),
            ),
        );
    }

    /// Structured Info Line rows: price change, bar/time span, and on-screen angle.
    fn info_line_stats(
        &self,
        bars: &[Bar],
        instrument: Option<&crate::model::Instrument>,
        time_axis: bool,
    ) -> Option<InfoLineStats> {
        let [a, b, ..] = &self.chart_points[..] else {
            return None;
        };
        let change = b.price - a.price;
        let pct = percent_change(a.price, b.price);
        let decimals = price_decimals(a.price);
        let mut change_label = format!("{change:+.decimals$} ({pct:+.2}%)");
        if let Some(instrument) = instrument {
            for (size, unit) in [
                (instrument.tick_size, "ticks"),
                (instrument.pip_size, "pips"),
            ] {
                if let Some(size) = size.filter(|s| s.is_finite() && *s > 0.0) {
                    let count = change / size;
                    if count.is_finite() {
                        change_label.push_str(&format!("  {count:+.2} {unit}"));
                    }
                }
            }
        }
        let (from, to) = (a.bar_idx.round(), b.bar_idx.round());
        let mut span = format!("{} bars", (to - from).abs() as i64);
        if let Some(ms) = elapsed_ms(bars, from, to).filter(|_| time_axis) {
            span = format!("{span} · {}", format_duration(ms));
        }
        let [start, end, ..] = self.points[..] else {
            return None;
        };
        // Screen y grows downward, so a rising line has a positive angle.
        let degrees = (start.y - end.y).atan2(end.x - start.x).to_degrees();
        Some(InfoLineStats {
            change: change_label,
            span,
            angle: format!("{degrees:.1}°"),
        })
    }

    // Note: render_bars_pattern is now in trading.rs with full projection features

    /// Label of the measuring tools: the price change from the first anchor to the second and
    /// its percentage, the bars between them and, on a `time_axis`, the time they span, the
    /// volume traded over them, and for the info line the angle of the line on screen. Each
    /// anchor counts at its nearest bar: anchors N bars apart span N bars of time and the volume
    /// of the N + 1 bars from one to the other, both included.
    pub(crate) fn measurement_label(
        &self,
        bars: &[Bar],
        instrument: Option<&crate::model::Instrument>,
        time_axis: bool,
    ) -> Option<String> {
        let [a, b, ..] = &self.chart_points[..] else {
            return None;
        };
        let change = b.price - a.price;
        let pct = percent_change(a.price, b.price);
        let decimals = price_decimals(a.price);
        let mut change_label = format!("{change:+.decimals$} ({pct:+.2}%)");
        if let Some(instrument) = instrument {
            for (size, unit) in [
                (instrument.tick_size, "ticks"),
                (instrument.pip_size, "pips"),
            ] {
                if let Some(size) = size.filter(|s| s.is_finite() && *s > 0.0) {
                    let count = change / size;
                    if count.is_finite() {
                        change_label.push_str(&format!("  {count:+.2} {unit}"));
                    }
                }
            }
        }
        let change = change_label;
        let (from, to) = (a.bar_idx.round(), b.bar_idx.round());
        let mut span = format!("{} bars", (to - from).abs() as i64);
        if let Some(ms) = elapsed_ms(bars, from, to).filter(|_| time_axis) {
            span = format!("{span}, {}", format_duration(ms));
        }
        let volume = bars_between(bars, from, to)
            .map(|(_, bars)| {
                let volume = bars
                    .iter()
                    .map(|bar| bar.volume)
                    .filter(|v| v.is_finite())
                    .sum();
                format!("\nVol {}", VolumeFormatter::default().format(volume))
            })
            .unwrap_or_default();
        let shown = self.measure.unwrap_or_default();
        let lines = |price: bool, time: bool| {
            let mut text = String::new();
            for (show, part) in [
                (price && shown.show_price, change.as_str()),
                (time && shown.show_time, span.as_str()),
                (time && shown.show_volume, volume.trim_start()),
            ] {
                if show && !part.is_empty() {
                    if !text.is_empty() {
                        text.push('\n');
                    }
                    text.push_str(part);
                }
            }
            text
        };
        Some(match self.tool_type {
            DrawingToolType::Measure | DrawingToolType::DateAndPriceRange => lines(true, true),
            DrawingToolType::PriceRange => lines(true, false),
            DrawingToolType::DateRange => lines(false, true),
            DrawingToolType::InfoLine => {
                let [start, end, ..] = self.points[..] else {
                    return None;
                };
                // Screen y grows downward, so a rising line has a positive angle.
                let angle = (start.y - end.y).atan2(end.x - start.x).to_degrees();
                format!("{change}\n{span}\n{angle:.1}°")
            }
            _ => return None,
        })
    }

    /// The rectangle's width in bars and height in price, from its stored anchors.
    pub(crate) fn size_labels(&self) -> Vec<String> {
        let gap = |a: f64, b: f64| {
            let (low, gap) = (a.min(b), (a - b).abs());
            let pct = percent_change(low, low + gap);
            let decimals = price_decimals(low);
            format!("{gap:.decimals$} ({pct:.2}%)")
        };
        match (self.tool_type, &self.chart_points[..]) {
            (DrawingToolType::Rect, [a, b, ..]) => vec![
                format!("{} bars", (b.bar_idx - a.bar_idx).abs() as i64),
                gap(a.price, b.price),
            ],
            _ => Vec::new(),
        }
    }
}

/// Rows painted into the Info Line stats card.
struct InfoLineStats {
    change: String,
    span: String,
    angle: String,
}

impl Drawing {
    /// [`boxed_label`] in the drawing's label font, so its Label size applies.
    pub(super) fn label_box(
        &self,
        painter: &egui::Painter,
        align: egui::Align2,
        pos: Pos2,
        text: String,
        color: Color32,
        bg: Color32,
    ) -> Rect {
        boxed_label(painter, align, pos, text, color, bg, self.annotation_font())
    }

    /// The label text colour over `bg`: the drawing's own text colour when set, else black or
    /// white, whichever reads on it.
    pub(super) fn label_text_on(&self, bg: Color32) -> Color32 {
        self.text_color.map_or_else(
            || crate::chart::rendering::axes::tag_text_color(bg.to_opaque()),
            |c| Color32::from_rgba_unmultiplied(c[0], c[1], c[2], c[3]),
        )
    }

    /// [`Self::label_box`] over `bg` with the text colour that reads on it.
    pub(super) fn label_box_on(
        &self,
        painter: &egui::Painter,
        align: egui::Align2,
        pos: Pos2,
        text: String,
        bg: Color32,
    ) -> Rect {
        self.label_box(painter, align, pos, text, self.label_text_on(bg), bg)
    }
}

/// Paints `text` in `font` over a `bg` box sized to it, placed at `pos` by `align`, and returns
/// its bounds.
pub(super) fn boxed_label(
    painter: &egui::Painter,
    align: egui::Align2,
    pos: Pos2,
    text: String,
    color: Color32,
    bg: Color32,
    font: FontId,
) -> Rect {
    let snap = crate::painting::pixel_snap::PixelSnap::of(painter);
    let galley = painter.layout_no_wrap(text, font, color);
    let raw = align.anchor_size(pos, galley.size() + egui::vec2(8.0, 4.0));
    let rect = Rect::from_min_size(
        Pos2::new(snap.edge(raw.min.x), snap.edge(raw.min.y)),
        raw.size().ceil(),
    );
    painter.rect_filled(rect, DESIGN_TOKENS.rounding.sm, bg);
    let text_pos = Pos2::new(
        snap.edge(rect.center().x - galley.size().x * 0.5),
        snap.edge(rect.center().y - galley.size().y * 0.5),
    );
    painter.galley(text_pos, galley, color);
    rect
}

/// Milliseconds from bar `a` to bar `b`, from the bars' times. An index past either end of the
/// series is placed at the most common spacing of the bars at that end; `None` with fewer than
/// two bars.
pub(super) fn elapsed_ms(bars: &[Bar], a: f32, b: f32) -> Option<i64> {
    let time = |index: f32| {
        let last = bars.len().checked_sub(1)?;
        let ms = |i: usize| bars[i].time.timestamp_millis();
        let index = index.round() as i64;
        let (edge, step) = match usize::try_from(index) {
            Ok(i) if i <= last => return Some(ms(i)),
            _ if last == 0 => return None,
            Ok(_) => (last, spacing(bars.iter().rev())),
            Err(_) => (0, spacing(bars.iter())),
        };
        Some(ms(edge).saturating_add((index - edge as i64).saturating_mul(step)))
    };
    Some(time(b)?.saturating_sub(time(a)?))
}

/// The most common time between consecutive bars among the first eleven
/// `bars` yields, so that a weekend or a gap next to the end of the series does
/// not set the step of the bars placed past it.
fn spacing<'a>(bars: impl Iterator<Item = &'a Bar>) -> i64 {
    let times: Vec<i64> = bars
        .take(11)
        .map(|bar| bar.time.timestamp_millis())
        .collect();
    let mut steps: Vec<i64> = times.windows(2).map(|w| (w[1] - w[0]).abs()).collect();
    steps.sort_unstable();
    steps
        .chunk_by(|a, b| a == b)
        .max_by_key(|run| run.len())
        .map_or(0, |run| run[0])
}

/// A duration in its two largest units: days and hours, hours and minutes, or minutes, and
/// seconds below a minute.
pub(super) fn format_duration(ms: i64) -> String {
    let secs = ms.unsigned_abs() / 1000;
    match (secs / 86_400, secs / 3600 % 24, secs / 60 % 60) {
        (0, 0, 0) => format!("{secs}s"),
        (0, 0, m) => format!("{m}m"),
        (0, h, 0) => format!("{h}h"),
        (0, h, m) => format!("{h}h {m}m"),
        (d, 0, _) => format!("{d}d"),
        (d, h, _) => format!("{d}d {h}h"),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::drawings::ChartPoint;
    use crate::drawings::rendering::sample_bars;

    /// Hourly bars with volumes 100, 200, 150, 300, 250, 400 and 50, the last one 49 h after the
    /// one before it, at 54 h, as across a weekend. Prices change from the first anchor to the
    /// second with their sign, durations come from the bar times (past the last bar at their
    /// common spacing of 1 h, so bar 30 is at 54 + 24 = 78 h, 3d 6h), volume is the sum over the bars from one anchor
    /// to the other, and a line that rises on screen has a positive angle: 10 px up over 300 px
    /// is atan(10 / 300) = 1.9 degrees.
    #[test]
    fn measurement_labels_use_bar_times_volumes_and_signed_values() {
        let mut bars = sample_bars();
        for (i, bar) in bars.iter_mut().enumerate() {
            bar.time = chrono::DateTime::from_timestamp(i as i64 * 3600, 0).unwrap();
        }
        bars[6].time = chrono::DateTime::from_timestamp(54 * 3600, 0).unwrap();
        let cases = [
            (
                DrawingToolType::Measure,
                (1.0, 100.0),
                (4.0, 95.0),
                "-5.00 (-5.00%)\n3 bars, 3h\nVol 900.00",
            ),
            (
                DrawingToolType::InfoLine,
                (0.0, 100.0),
                (30.0, 110.0),
                "+10.00 (+10.00%)\n30 bars, 3d 6h\n1.9°",
            ),
            (
                DrawingToolType::DateRange,
                (2.0, 100.0),
                (5.0, 100.0),
                "3 bars, 3h\nVol 1.10K",
            ),
            // Fractional anchors count at their nearest bars, 1 and 4.
            (
                DrawingToolType::DateRange,
                (1.4, 100.0),
                (3.6, 100.0),
                "3 bars, 3h\nVol 900.00",
            ),
            (
                DrawingToolType::DateAndPriceRange,
                (5.0, 200.0),
                (2.0, 210.0),
                "+10.00 (+5.00%)\n3 bars, 3h\nVol 1.10K",
            ),
        ];
        let mut wrong = Vec::new();
        for (tool, (b1, p1), (b2, p2), want) in cases {
            let mut drawing = Drawing::new(1, tool);
            drawing.chart_points = vec![ChartPoint::new(b1, p1), ChartPoint::new(b2, p2)];
            drawing.update_screen_coords(|bar| bar * 10.0, |price| 500.0 - price as f32);
            let got = drawing.measurement_label(&bars, None, true);
            if got.as_deref() != Some(want) {
                wrong.push(format!("{tool:?}: got {got:?}, want {want:?}"));
            }
        }
        let mut drawing = Drawing::new(0, DrawingToolType::Measure);
        drawing.chart_points = vec![ChartPoint::new(1.0, 1.1000), ChartPoint::new(4.0, 1.0995)];
        // On a horizontal scale other than time the bar times are keys, not times.
        assert_eq!(
            drawing.measurement_label(&bars, None, false).as_deref(),
            Some("-0.0005 (-0.05%)\n3 bars\nVol 900.00")
        );
        for (tick, pip, want) in [
            (Some(0.00001), Some(0.0001), "-50.00 ticks  -5.00 pips"),
            (Some(0.0), Some(f64::NAN), ""),
            (Some(-1.0), Some(f64::INFINITY), ""),
            (None, None, ""),
        ] {
            let mut instrument = crate::model::Instrument::new("PAIR");
            instrument.tick_size = tick;
            instrument.pip_size = pip;
            let label = drawing
                .measurement_label(&bars, Some(&instrument), true)
                .unwrap();
            if want.is_empty() {
                assert!(!label.contains("ticks") && !label.contains("pips"));
            } else {
                assert!(label.contains(want), "{label}");
            }
        }
        assert!(wrong.is_empty(), "{}", wrong.join("\n"));
    }

    /// Info Line paints a neutral tooltip card with a directional accent, not a solid fill.
    #[test]
    fn info_line_stats_card_uses_tooltip_chrome_and_accent() {
        use crate::drawings::ChartPoint;
        use crate::drawings::rendering::ChartView;
        use crate::theme::{Theme, ThemeExt};
        let theme = Theme::dark();
        let ctx = egui::Context::default();
        crate::theme::apply_to_egui(&ctx, &theme);
        let pane = Rect::from_min_max(Pos2::ZERO, Pos2::new(400.0, 300.0));
        let mut drawing = Drawing::new(1, DrawingToolType::InfoLine);
        drawing.points = vec![Pos2::new(80.0, 180.0), Pos2::new(280.0, 80.0)];
        drawing.chart_points = vec![ChartPoint::new(8.0, 100.0), ChartPoint::new(28.0, 120.0)];
        drawing.completed = true;
        let bars = sample_bars();
        let output = ctx.run_ui(Default::default(), |ui| {
            assert_eq!(
                ui.ctx().active_theme().chart.bg_tooltip,
                theme.chart.bg_tooltip
            );
            let view = ChartView {
                instrument: None,
                bars: &bars,
                revision: 0,
                bar_to_x: &|x| x,
                price_to_y: &|price| price as f32,
                y_to_price: &f64::from,
                time_axis: true,
                executions: None,
            };
            drawing.render(ui.painter(), pane, &view);
        });
        let fills: Vec<_> = output
            .shapes
            .iter()
            .filter_map(|shape| match &shape.shape {
                egui::Shape::Rect(rect) if rect.fill.a() > 0 => Some(rect.fill),
                _ => None,
            })
            .collect();
        let texts: Vec<_> = output
            .shapes
            .iter()
            .filter_map(|shape| match &shape.shape {
                egui::Shape::Text(text) => Some(text.galley.job.text.clone()),
                _ => None,
            })
            .collect();
        output.drop_without_applying_deltas();
        assert!(
            fills.iter().any(|fill| *fill == theme.chart.bg_tooltip),
            "tooltip background missing: {fills:?}"
        );
        assert!(
            fills.iter().any(|fill| *fill == theme.chart.candle_up),
            "bullish accent missing: {fills:?}"
        );
        // Card chrome must include the dark tooltip panel; accent alone is not enough.
        assert!(
            fills
                .iter()
                .filter(|fill| **fill == theme.chart.bg_tooltip)
                .count()
                >= 1
                && fills.len() >= 2,
            "expected tooltip card plus accent/shadow: {fills:?}"
        );
        assert!(
            texts.iter().any(|t| t.contains('+') && t.contains('%')),
            "price change row missing: {texts:?}"
        );
        assert!(
            texts.iter().any(|t| t.contains("bars")),
            "bars row missing: {texts:?}"
        );
        assert!(
            texts.iter().any(|t| t.contains('°')),
            "angle row missing: {texts:?}"
        );
    }

    #[test]
    fn measurements_receive_the_chart_instrument() {
        use crate::widget::test_frames::{chart, shapes};
        let mut chart = chart(&[90.0], &[110.0]);
        let ctx = egui::Context::default();
        let mut drawing = Drawing::new(1, DrawingToolType::Measure);
        drawing.chart_points = vec![ChartPoint::new(30.0, 100.0), ChartPoint::new(40.0, 95.0)];
        drawing.completed = true;
        chart.drawings_mut().drawings.push(drawing);
        chart.set_instrument(Some(
            crate::model::Instrument::new("TEST").with_tick_size(0.25),
        ));
        let output = shapes(&ctx, &mut chart, 0.0, vec![]);
        assert!(output.iter().any(
            |s| matches!(s,egui::Shape::Text(t) if t.galley.job.text.contains("-20.00 ticks"))
        ));
    }

    /// 100 to 110 over bars 0 to 20 is 10 (10%) high and 20 bars wide.
    #[test]
    fn size_labels_are_in_price_and_bars() {
        let mut drawing = Drawing::new(1, DrawingToolType::Rect);
        drawing.chart_points = vec![ChartPoint::new(0.0, 100.0), ChartPoint::new(20.0, 110.0)];
        assert_eq!(drawing.size_labels(), ["20 bars", "10.00 (10.00%)"]);
    }
}
