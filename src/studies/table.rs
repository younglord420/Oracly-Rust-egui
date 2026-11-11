//! Labelled study tables, independent of bar-indexed plots.

/// A cell in a study table.
#[derive(Clone, Debug, PartialEq)]
pub enum StudyCell {
    /// A row label or other text.
    Text(String),
    /// A percentage, coloured by sign; `None` displays an unavailable value.
    Percent(Option<f64>),
}

/// Where a study table sits inside its pane.
///
/// Built-in studies leave this at the lower right. A Pine `table.new` names one
/// of the nine `position.*` anchors, plus the three vertical-only names.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum TableAnchor {
    /// Upper left of the pane.
    TopLeft,
    /// Upper middle of the pane.
    TopCenter,
    /// Upper right of the pane.
    TopRight,
    /// Vertical middle, left edge.
    MiddleLeft,
    /// Centre of the pane.
    MiddleCenter,
    /// Vertical middle, right edge.
    MiddleRight,
    /// Lower left of the pane.
    BottomLeft,
    /// Lower middle of the pane.
    BottomCenter,
    /// Lower right of the pane. This is where built-in study tables already sit.
    #[default]
    BottomRight,
}

impl TableAnchor {
    /// Pine `position.*` string. Anything else stays at the lower right.
    pub fn from_pine(position: &str) -> Self {
        match position {
            "top_left" => Self::TopLeft,
            "top_center" | "top" => Self::TopCenter,
            "top_right" => Self::TopRight,
            "middle_left" => Self::MiddleLeft,
            "middle_center" | "middle" => Self::MiddleCenter,
            "middle_right" => Self::MiddleRight,
            "bottom_left" => Self::BottomLeft,
            "bottom_center" | "bottom" => Self::BottomCenter,
            _ => Self::BottomRight,
        }
    }
}

/// Optional colours for one data cell, as `0xRRGGBBAA` bytes.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct StudyCellStyle {
    /// Zero-based row in [`StudyTable::rows`].
    pub row: usize,
    /// Zero-based column.
    pub column: usize,
    /// Text colour. `None` keeps the theme colour.
    pub text_color: Option<[u8; 4]>,
    /// Cell fill. `None` keeps the theme fill.
    pub background: Option<[u8; 4]>,
    /// Pine `size.*`. Empty keeps the normal 11px size.
    pub text_size: String,
    /// `left`/`center`/`right` or Pine's `align_*`. Empty means left.
    pub halign: String,
    /// `top`/`center`/`bottom` or Pine's `align_*`. Empty means center.
    pub valign: String,
}

/// A labelled table drawn in its pane.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct StudyTable {
    /// Caption above the column headings.
    pub title: String,
    /// One heading per column.
    pub columns: Vec<String>,
    /// Cells in each row, in column order.
    pub rows: Vec<Vec<StudyCell>>,
    /// Corner or edge the table hugs. Built-in tables use the lower right.
    pub anchor: TableAnchor,
    /// Table fill. `None` uses the floating panel colour.
    pub background: Option<[u8; 4]>,
    /// Per-cell colours. Empty keeps the theme, including rating tints.
    pub cell_styles: Vec<StudyCellStyle>,
}

#[cfg(feature = "widget")]
fn rating_color(rating: &str) -> egui::Color32 {
    match rating {
        "Strong Buy" => super::palette::BLUE,
        "Buy" => egui::Color32::from_rgb(91, 156, 246),
        "Strong Sell" => super::palette::BRIGHT_RED,
        "Sell" => egui::Color32::from_rgb(239, 154, 154),
        _ => super::palette::DISABLED,
    }
}

#[cfg(feature = "widget")]
fn rgba(bytes: [u8; 4]) -> egui::Color32 {
    egui::Color32::from_rgba_unmultiplied(bytes[0], bytes[1], bytes[2], bytes[3])
}

#[cfg(feature = "widget")]
fn text_px(size: &str) -> f32 {
    match size {
        "tiny" => 8.0,
        "small" => 10.0,
        "large" => 14.0,
        "huge" => 18.0,
        _ => 11.0,
    }
}

#[cfg(feature = "widget")]
fn cell_font(size: &str) -> egui::FontId {
    egui::FontId::proportional(text_px(size))
}

#[cfg(feature = "widget")]
fn style_at(table: &StudyTable, row: usize, column: usize) -> Option<&StudyCellStyle> {
    table
        .cell_styles
        .iter()
        .find(|style| style.row == row && style.column == column)
}

#[cfg(feature = "widget")]
fn text_place(halign: &str, valign: &str, rect: egui::Rect) -> (egui::Pos2, egui::Align2) {
    use egui::{Align, Align2, pos2};
    let (x, horizontal) = match halign {
        "center" | "align_center" => (rect.center().x, Align::Center),
        "right" | "align_right" => (rect.right() - 8.0, Align::RIGHT),
        _ => (rect.left() + 8.0, Align::LEFT),
    };
    let (y, vertical) = match valign {
        "top" | "align_top" => (rect.top() + 2.0, Align::TOP),
        "bottom" | "align_bottom" => (rect.bottom() - 2.0, Align::BOTTOM),
        _ => (rect.center().y, Align::Center),
    };
    (pos2(x, y), Align2([horizontal, vertical]))
}

#[cfg(feature = "widget")]
fn anchored_bounds(anchor: TableAnchor, available: egui::Rect, size: egui::Vec2) -> egui::Rect {
    let left = available.left();
    let right = available.right();
    let top = available.top();
    let bottom = available.bottom();
    let x = match anchor {
        TableAnchor::TopLeft | TableAnchor::MiddleLeft | TableAnchor::BottomLeft => left,
        TableAnchor::TopCenter | TableAnchor::MiddleCenter | TableAnchor::BottomCenter => {
            available.center().x - size.x * 0.5
        }
        TableAnchor::TopRight | TableAnchor::MiddleRight | TableAnchor::BottomRight => {
            right - size.x
        }
    };
    let y = match anchor {
        TableAnchor::TopLeft | TableAnchor::TopCenter | TableAnchor::TopRight => top,
        TableAnchor::MiddleLeft | TableAnchor::MiddleCenter | TableAnchor::MiddleRight => {
            available.center().y - size.y * 0.5
        }
        TableAnchor::BottomLeft | TableAnchor::BottomCenter | TableAnchor::BottomRight => {
            bottom - size.y
        }
    };
    egui::Rect::from_min_size(
        egui::pos2(
            x.clamp(left, (right - size.x).max(left)),
            y.clamp(top, (bottom - size.y).max(top)),
        ),
        size,
    )
}

#[cfg(feature = "widget")]
pub(crate) fn paint(
    ui: &mut egui::Ui,
    rect: egui::Rect,
    table: &StudyTable,
    id: egui::Id,
) -> egui::Rect {
    use crate::theme::ThemeExt;
    use egui::{Align2, FontId, Rect, Stroke, StrokeKind, pos2, vec2};

    let rating_table = table.title == "Technical Ratings";
    let painter = ui.painter().clone();
    let theme = ui.active_theme();
    let font = FontId::proportional(11.0);
    let text = |cell: &StudyCell| match cell {
        StudyCell::Text(text) => text.clone(),
        StudyCell::Percent(Some(value)) if value.is_finite() => format!("{value:+.2}%"),
        StudyCell::Percent(_) => "N/A".into(),
    };
    let mut widths: Vec<_> = table
        .columns
        .iter()
        .map(|column| {
            painter
                .layout_no_wrap(column.clone(), font.clone(), theme.ui.text)
                .size()
                .x
                + 16.0
        })
        .collect();
    for (row_index, row) in table.rows.iter().enumerate() {
        for (column, (width, cell)) in widths.iter_mut().zip(row).enumerate() {
            let size = style_at(table, row_index, column)
                .map(|style| style.text_size.as_str())
                .unwrap_or("");
            *width = width.max(
                painter
                    .layout_no_wrap(text(cell), cell_font(size), theme.ui.text)
                    .size()
                    .x
                    + 16.0,
            );
        }
    }
    let width = widths.iter().sum::<f32>();
    let row_h = table
        .cell_styles
        .iter()
        .map(|style| text_px(&style.text_size) + 8.0)
        .fold(22.0, f32::max);
    let body = row_h * (table.rows.len() + 1) as f32;
    let height = 22.0 + body;
    let mut available = rect.shrink(8.0);
    available.min.y += 24.0;
    if !available.is_positive() || rect.height() < 44.0 {
        return Rect::from_min_size(rect.min, egui::Vec2::ZERO);
    }
    let title_width = painter
        .layout_no_wrap(table.title.clone(), font.clone(), theme.ui.text)
        .size()
        .x
        + 16.0;
    let size = vec2(
        width.max(title_width).min(available.width()),
        height.min(available.height()).min(rect.height() * 0.5),
    );
    let bounds = anchored_bounds(table.anchor, available, size);
    let painter = painter.with_clip_rect(bounds.intersect(ui.clip_rect()));
    painter.rect_filled(
        bounds,
        3.0,
        table
            .background
            .map(rgba)
            .unwrap_or(theme.ui.panel_bg_floating),
    );
    painter.rect_stroke(
        bounds,
        3.0,
        Stroke::new(
            if rating_table { 2.0 } else { 1.0 },
            if rating_table {
                egui::Color32::WHITE
            } else {
                theme.ui.border
            },
        ),
        StrokeKind::Inside,
    );
    painter.text(
        bounds.min + vec2(8.0, 11.0),
        Align2::LEFT_CENTER,
        &table.title,
        font.clone(),
        theme.ui.text,
    );
    let mut child = ui.new_child(
        egui::UiBuilder::new()
            .id_salt(id)
            .max_rect(Rect::from_min_max(bounds.min + vec2(0.0, 22.0), bounds.max)),
    );
    child.set_clip_rect(painter.clip_rect());
    egui::ScrollArea::both()
        .auto_shrink([false, false])
        .show(&mut child, |ui| {
            let (content, _) = ui.allocate_exact_size(vec2(width, body), egui::Sense::hover());
            let painter = ui.painter();
            let mut y = content.top();
            for (row_index, row) in std::iter::once(
                table
                    .columns
                    .iter()
                    .cloned()
                    .map(StudyCell::Text)
                    .collect::<Vec<_>>(),
            )
            .chain(table.rows.iter().cloned())
            .enumerate()
            {
                let mut x = content.left();
                for (column, (cell, width)) in row.iter().zip(&widths).enumerate() {
                    let cell_rect = Rect::from_min_size(pos2(x, y), vec2(*width, row_h));
                    let style = row_index
                        .checked_sub(1)
                        .and_then(|row| style_at(table, row, column));
                    let mut color = match cell {
                        _ if rating_table && row_index == 0 => {
                            painter.rect_filled(
                                cell_rect,
                                0.0,
                                egui::Color32::from_rgb(41, 91, 121),
                            );
                            egui::Color32::WHITE
                        }
                        _ if rating_table => {
                            let rating = match row.get(2) {
                                Some(StudyCell::Text(text)) => text.as_str(),
                                _ => "Neutral",
                            };
                            let fill = rating_color(rating);
                            painter.rect_filled(cell_rect, 0.0, fill);
                            painter.rect_stroke(
                                cell_rect,
                                0.0,
                                Stroke::new(1.0, egui::Color32::WHITE),
                                StrokeKind::Inside,
                            );
                            crate::theme::readable_color(
                                egui::Color32::WHITE,
                                egui::Color32::BLACK,
                                &[fill],
                                4.5,
                            )
                        }
                        StudyCell::Percent(Some(value)) if value.is_finite() => {
                            let color = if *value >= 0.0 {
                                egui::Color32::from_rgb(8, 153, 129)
                            } else {
                                egui::Color32::from_rgb(242, 55, 69)
                            };
                            let tint = color.gamma_multiply(
                                0.1 + if table.title == "Seasonality" {
                                    0.4
                                } else {
                                    0.2
                                } * (value.abs() / 10.0).min(1.0) as f32,
                            );
                            painter.rect_filled(cell_rect, 0.0, tint);
                            crate::theme::readable_color(
                                color,
                                theme.ui.text,
                                &[theme.ui.panel_bg_floating.blend(tint)],
                                4.5,
                            )
                        }
                        _ if matches!(table.title.as_str(), "Performance" | "Seasonality") => {
                            let alpha = if matches!(cell, StudyCell::Percent(None)) {
                                if table.title == "Seasonality" {
                                    128
                                } else {
                                    26
                                }
                            } else {
                                51
                            };
                            painter.rect_filled(
                                cell_rect,
                                0.0,
                                egui::Color32::from_rgba_unmultiplied(120, 123, 134, alpha),
                            );
                            theme.ui.text
                        }
                        _ => theme.ui.text,
                    };
                    if let Some(style) = style {
                        if let Some(background) = style.background {
                            painter.rect_filled(cell_rect, 0.0, rgba(background));
                        }
                        if let Some(text_color) = style.text_color {
                            color = rgba(text_color);
                        }
                    }
                    let (anchor, align) = text_place(
                        style.map(|style| style.halign.as_str()).unwrap_or(""),
                        style.map(|style| style.valign.as_str()).unwrap_or(""),
                        cell_rect,
                    );
                    painter.text(
                        anchor,
                        align,
                        text(cell),
                        cell_font(style.map(|style| style.text_size.as_str()).unwrap_or("")),
                        color,
                    );
                    x += width;
                }
                y += row_h;
            }
        });
    bounds
}

#[cfg(all(test, feature = "widget"))]
mod tests {
    use super::*;

    /// A narrow or short pane keeps overflowing columns and years within a scrollable table.
    #[test]
    fn overflowing_tables_fit_the_pane() {
        use egui::{Event, MouseWheelUnit, RawInput, Rect, Shape, TouchPhase, vec2};

        let table = StudyTable {
            title: "Seasonality".into(),
            columns: (0..13).map(|i| format!("Column {i}")).collect(),
            rows: (0..40)
                .map(|i| {
                    let mut row = vec![StudyCell::Percent(Some(12.34)); 13];
                    row[12] = StudyCell::Text(format!("Last {i}"));
                    row
                })
                .collect(),
            ..Default::default()
        };
        for (_, theme) in crate::theme::presets() {
            for width in [240.0, 800.0] {
                let ctx = egui::Context::default();
                crate::theme::apply_to_egui(&ctx, &theme);
                let rect = Rect::from_min_size(egui::Pos2::ZERO, vec2(width, 300.0));
                for frame in 0..20 {
                    let mut events =
                        vec![Event::PointerMoved(rect.right_bottom() - vec2(100.0, 50.0))];
                    if frame == 1 {
                        events.push(Event::MouseWheel {
                            unit: MouseWheelUnit::Point,
                            delta: vec2(-2000.0, -2000.0),
                            phase: TouchPhase::Move,
                            source: egui::MouseWheelSource::Unknown,
                            modifiers: Default::default(),
                        });
                    }
                    let output = ctx.run_ui(
                        RawInput {
                            screen_rect: Some(rect),
                            time: Some(frame as f64 / 60.0),
                            events,
                            ..Default::default()
                        },
                        |ui| {
                            let bounds = paint(ui, rect, &table, ui.scope_id().with("table"));
                            assert!(rect.contains_rect(bounds), "{bounds:?} outside {rect:?}");
                        },
                    );
                    if frame == 19 {
                        assert!(output.shapes.iter().any(|clipped| matches!(&clipped.shape,
                            Shape::Text(text) if text.galley.text() == "Last 39"
                                && clipped.clip_rect.contains_rect(Rect::from_min_size(text.pos, text.galley.size()))
                        )), "the last column of the last year must be reachable at width {width}");
                    }
                    output.drop_without_applying_deltas();
                }
            }
        }
    }

    /// A Pine anchor moves the table. The default stays in the lower right.
    #[test]
    fn anchors_place_the_table_in_its_corner() {
        use egui::{RawInput, Rect, vec2};

        let mut table = StudyTable {
            title: "Pine".into(),
            columns: vec!["1".into()],
            rows: vec![vec![StudyCell::Text("Hi".into())]],
            ..Default::default()
        };
        let ctx = egui::Context::default();
        let rect = Rect::from_min_size(egui::Pos2::ZERO, vec2(800.0, 300.0));
        let place = |table: &StudyTable| {
            let slot = std::cell::Cell::new(Rect::NOTHING);
            ctx.run_ui(
                RawInput {
                    screen_rect: Some(rect),
                    ..Default::default()
                },
                |ui| slot.set(paint(ui, rect, table, ui.scope_id().with("anchor"))),
            )
            .drop_without_applying_deltas();
            slot.get()
        };
        let lower_right = place(&table);
        table.anchor = TableAnchor::TopLeft;
        let upper_left = place(&table);
        assert!(upper_left.left() < lower_right.left());
        assert!(upper_left.top() < lower_right.top());
        assert!(rect.contains_rect(upper_left));
        assert!(rect.contains_rect(lower_right));
    }

    /// A huge cell is wider than the same word at the normal size, and a
    /// right-aligned word sits further right than a left-aligned one.
    #[test]
    fn cell_size_and_alignment_change_the_glyphs() {
        use egui::{RawInput, Rect, Shape, vec2};

        let ctx = egui::Context::default();
        let rect = Rect::from_min_size(egui::Pos2::ZERO, vec2(800.0, 300.0));
        let draw = |table: &StudyTable| {
            let slot = std::cell::Cell::new(Rect::NOTHING);
            let output = ctx.run_ui(
                RawInput {
                    screen_rect: Some(rect),
                    ..Default::default()
                },
                |ui| slot.set(paint(ui, rect, table, ui.scope_id().with("glyph"))),
            );
            let x = output.shapes.iter().find_map(|clipped| match &clipped.shape {
                Shape::Text(text) if text.galley.text() == "Wider" => Some(text.pos.x),
                _ => None,
            });
            let bounds = slot.get();
            output.drop_without_applying_deltas();
            (bounds, x)
        };
        let table = |heading: &str, size: &str, halign: &str| StudyTable {
            title: "Pine".into(),
            columns: vec![heading.into()],
            rows: vec![vec![StudyCell::Text("Wider".into())]],
            anchor: TableAnchor::TopLeft,
            cell_styles: vec![StudyCellStyle {
                row: 0,
                column: 0,
                text_color: None,
                background: None,
                text_size: size.into(),
                halign: halign.into(),
                valign: String::new(),
            }],
            ..Default::default()
        };
        let (normal_bounds, _) = draw(&table("W", "", ""));
        let (huge_bounds, _) = draw(&table("W", "huge", ""));
        assert!(
            huge_bounds.width() > normal_bounds.width(),
            "huge {} vs normal {}",
            huge_bounds.width(),
            normal_bounds.width()
        );
        let heading = "heading that is much wider than the word";
        let (_, left_x) = draw(&table(heading, "", ""));
        let (_, right_x) = draw(&table(heading, "", "align_right"));
        let left_x = left_x.expect("left glyph");
        let right_x = right_x.expect("right glyph");
        assert!(right_x > left_x, "right {right_x} vs left {left_x}");
    }
}
