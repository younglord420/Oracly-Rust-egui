//! Coordinates — staged price and UTC time anchors.

use crate::drawings::{ChartPoint, Drawing, DrawingToolType};
use crate::localization as l10n;
use crate::theme::ThemeExt;
use crate::tokens::DESIGN_TOKENS;
use crate::ui_kit::buttons::ButtonSize;
use crate::widget::Chart;
use chrono::{DateTime, Datelike, NaiveDate, NaiveTime, Timelike, Utc};
use egui::{RichText, TextEdit, Ui};

/// Editable coordinate fields. Times are UTC and retain subsecond precision.
#[derive(Clone, Debug, Default)]
pub struct CoordinatePoint {
    /// Price as entered by the user.
    pub price: String,
    /// UTC calendar date, YYYY-MM-DD; empty for an unmapped anchor.
    pub date: String,
    /// UTC time, HH:MM[:SS[.fraction]].
    pub time: String,
    /// Fractional bar index, used when no timestamp is available.
    pub bar_index: String,
    original: Option<(ChartPoint, Option<DateTime<Utc>>)>,
    /// Initial display text; an untouched rounded field still resolves to the exact anchor price.
    original_price_text: String,
    /// The price is not editable: the drawing's prices follow its bars.
    bar_only: bool,
}

/// Coordinates staged independently of the live drawing until Apply.
#[derive(Clone, Debug, Default)]
pub struct CoordinatesState {
    /// One entry for every anchor of the selected drawing.
    pub points: Vec<CoordinatePoint>,
}

impl CoordinatesState {
    pub(crate) fn from_drawing(drawing: &Drawing, chart: &Chart) -> Self {
        let price_precision = chart
            .instrument()
            .and_then(crate::scales::instrument_price_precision);
        Self {
            points: drawing
                .chart_points
                .iter()
                .map(|point| {
                    let time = chart.drawing_point_time(point);
                    let mut original = *point;
                    original.time_anchor = time.map(|time| (time, point.bar_idx));
                    let price = display_price(point.price, price_precision);
                    CoordinatePoint {
                        original_price_text: price.clone(),
                        price,
                        bar_index: point.bar_idx.to_string(),
                        date: time.map_or_else(String::new, |t| t.format("%Y-%m-%d").to_string()),
                        time: time
                            .map_or_else(String::new, |t| t.format("%H:%M:%S%.f").to_string()),
                        original: Some((original, time)),
                        bar_only: drawing.tool_type == DrawingToolType::RegressionTrend,
                    }
                })
                .collect(),
        }
    }

    pub(crate) fn resolve(&self, chart: &Chart) -> Result<Vec<ChartPoint>, &'static str> {
        if self.points.iter().all(|point| {
            point.original.is_some_and(|(original, time)| {
                (point.bar_only || point.price == point.original_price_text)
                    && point.bar_index == original.bar_idx.to_string()
                    && point.date
                        == time.map_or_else(String::new, |t| t.format("%Y-%m-%d").to_string())
                    && point.time
                        == time.map_or_else(String::new, |t| t.format("%H:%M:%S%.f").to_string())
            })
        }) {
            return Ok(Vec::new());
        }

        self.points
            .iter()
            .map(|point| {
                let price = if let Some((original, _)) = point.original
                    && (point.bar_only || point.price == point.original_price_text)
                {
                    original.price
                } else {
                    point
                        .price
                        .parse::<f64>()
                        .ok()
                        .filter(|v| v.is_finite())
                        .ok_or("Enter a finite price for every point")?
                };
                if point.date.is_empty() && point.time.is_empty() {
                    let index = point
                        .bar_index
                        .parse::<f32>()
                        .ok()
                        .filter(|v| v.is_finite())
                        .ok_or("Enter a finite bar index for every point")?;
                    return Ok(ChartPoint::new(index, price));
                }
                let date = NaiveDate::parse_from_str(&point.date, "%Y-%m-%d")
                    .map_err(|_| "Enter dates as YYYY-MM-DD")?;
                let time = NaiveTime::parse_from_str(&point.time, "%H:%M:%S%.f")
                    .or_else(|_| NaiveTime::parse_from_str(&point.time, "%H:%M"))
                    .map_err(|_| "Enter UTC times as HH:MM or HH:MM:SS")?;
                let time = date.and_time(time).and_utc();
                if let Some((original, Some(old_time))) = point.original
                    && old_time == time
                {
                    let current = ChartPoint::new(original.bar_idx, price);
                    if chart
                        .drawing_point_time(&current)
                        .is_some_and(|current| current != time)
                    {
                        return chart
                            .drawing_point_at(time, price)
                            .ok_or("Load chart data before editing times");
                    }
                    return Ok(ChartPoint { price, ..original });
                }
                chart
                    .drawing_point_at(time, price)
                    .ok_or("Load chart data before editing times")
            })
            .collect()
    }
}

/// Match the chart's visible precision without changing an untouched anchor's exact stored value.
fn display_price(price: f64, precision: Option<u8>) -> String {
    let decimals = precision
        .map(usize::from)
        .unwrap_or_else(|| crate::scales::price_decimals(price));
    format!("{price:.decimals$}")
}

/// Coordinates section renderer.
pub struct CoordinatesTab;

impl CoordinatesTab {
    pub fn show(ui: &mut Ui, state: &mut CoordinatesState) {
        if state.points.is_empty() {
            ui.label(
                RichText::new(l10n::text(
                    ui.ctx(),
                    "Open a drawing to edit its coordinates.",
                ))
                .size(ui.active_theme().text.body)
                .color(ui.active_theme().ui.text_muted),
            );
            return;
        }

        let theme = ui.active_theme();
        let row_h = DESIGN_TOKENS
            .sizing
            .button_md
            .max(ui.spacing().interact_size.y)
            .max(ButtonSize::SM.pixels());
        // `#n` + price + short datetime share one aligned, token-sized row.
        let index_w = DESIGN_TOKENS.sizing.icon_lg + theme.spacing.sm;

        for (i, point) in state.points.iter_mut().enumerate() {
            if i > 0 {
                ui.add_space(theme.spacing.xs);
            }
            let index_label = format!("#{}", i + 1);
            let a11y_prefix = index_label.clone();

            ui.horizontal(|ui| {
                ui.set_min_height(row_h);
                ui.spacing_mut().item_spacing.x = theme.spacing.sm;
                ui.add_sized(
                    [index_w, row_h],
                    egui::Label::new(
                        RichText::new(&index_label)
                            .size(theme.text.body)
                            .color(theme.ui.text_muted)
                            .strong(),
                    ),
                );

                if !point.bar_only {
                    let available_for_fields = (ui.available_width() - theme.spacing.sm).max(0.0);
                    let price_w = (available_for_fields * 0.46)
                        .clamp(
                            DESIGN_TOKENS.sizing.dialog.button_min_width_xs,
                            DESIGN_TOKENS.sizing.settings_dialog.dropdown_width,
                        )
                        .min(available_for_fields);
                    let exact_price = point
                        .original
                        .map(|(original, _)| original.price.to_string());
                    price_field(
                        ui,
                        &a11y_prefix,
                        &mut point.price,
                        exact_price.as_deref(),
                        price_w,
                        row_h,
                    );
                }

                let trailing_width = ui.available_width().max(0.0);
                if point.original.is_some_and(|(_, time)| time.is_none()) {
                    compact_field(
                        ui,
                        if a11y_prefix == "#1" {
                            "Bar index".to_owned()
                        } else {
                            format!("{a11y_prefix} Bar index")
                        },
                        &mut point.bar_index,
                        "",
                        trailing_width,
                        row_h,
                        false,
                    );
                } else {
                    datetime_field(ui, i, point, trailing_width, row_h);
                }
            });
        }
    }
}

fn price_field(
    ui: &mut Ui,
    prefix: &str,
    value: &mut String,
    exact_value: Option<&str>,
    width: f32,
    height: f32,
) {
    let font = egui::FontId::proportional(ui.active_theme().text.body);
    let response = styled_text_field(
        ui,
        TextEdit::singleline(value)
            .font(font)
            .desired_width(width)
            .min_size(egui::vec2(width, height))
            .align(egui::Align2::RIGHT_CENTER),
    );
    if let Some(exact) = exact_value {
        response.clone().on_hover_text(l10n::format_text(
            ui.ctx(),
            "Exact value: {value}",
            &[("value", exact.to_owned())],
        ));
    }
    // First point keeps the short name existing a11y harnesses expect.
    let name = if prefix == "#1" {
        "Price".to_owned()
    } else {
        format!("{prefix} Price")
    };
    response.accessible_name(l10n::text(ui.ctx(), name));
}

fn datetime_field(ui: &mut Ui, index: usize, point: &mut CoordinatePoint, width: f32, height: f32) {
    let draft_id = ui.scope_id().with(("drawing_props_datetime", index));
    let mut text = ui
        .data(|d| d.get_temp::<String>(draft_id))
        .unwrap_or_else(|| short_datetime(&point.date, &point.time));

    let response = compact_field(
        ui,
        if index == 0 {
            "Date and time (UTC)".to_owned()
        } else {
            format!("#{} Date and time (UTC)", index + 1)
        },
        &mut text,
        "25 Oct 22:45",
        width,
        height,
        true,
    );

    let fallback_year = NaiveDate::parse_from_str(&point.date, "%Y-%m-%d")
        .map(|d| d.year())
        .unwrap_or_else(|_| Utc::now().year());

    if response.changed() {
        ui.data_mut(|d| d.insert_temp(draft_id, text.clone()));
        if let Some((date, time)) = parse_datetime_input(&text, fallback_year) {
            apply_parsed_datetime(point, date, time);
        }
    } else if !response.has_focus() {
        // Drop the draft so the next idle frame shows the canonical short form.
        ui.data_mut(|d| d.remove::<String>(draft_id));
    }
}

/// Keep existing seconds when the short field only edited HH:MM.
fn apply_parsed_datetime(point: &mut CoordinatePoint, date: String, time: String) {
    point.date = date;
    if time.len() == 5 && point.time.starts_with(&time) {
        return;
    }
    point.time = time;
}

fn compact_field(
    ui: &mut Ui,
    label: String,
    value: &mut String,
    hint: &str,
    width: f32,
    height: f32,
    muted: bool,
) -> egui::Response {
    let theme = ui.active_theme();
    let font = egui::FontId::proportional(theme.text.body);
    let mut edit = TextEdit::singleline(value)
        .font(font)
        .desired_width(width)
        .min_size(egui::vec2(width, height))
        .hint_text(
            RichText::new(l10n::text(ui.ctx(), hint))
                .size(theme.text.marker)
                .color(theme.ui.text_muted),
        );
    if muted {
        edit = edit.text_color(theme.ui.text_secondary);
    }
    let response = styled_text_field(ui, edit);
    response
        .clone()
        .accessible_name(l10n::text(ui.ctx(), label));
    response
}

/// Neutral text field matching the top and side toolbars rather than the accent selection colour.
fn styled_text_field(ui: &mut Ui, edit: TextEdit<'_>) -> egui::Response {
    let theme = ui.active_theme().clone();
    ui.scope(|ui| {
        let visuals = ui.visuals_mut();
        visuals.extreme_bg_color = theme.ui.btn_bg_hover;
        visuals.widgets.inactive.bg_stroke =
            egui::Stroke::new(DESIGN_TOKENS.stroke.hairline, theme.ui.border_control);
        visuals.widgets.hovered.bg_stroke =
            egui::Stroke::new(DESIGN_TOKENS.stroke.hairline, theme.ui.text_muted);
        visuals.widgets.active.bg_stroke =
            egui::Stroke::new(DESIGN_TOKENS.stroke.hairline, theme.ui.text_secondary);
        visuals.widgets.open.bg_stroke =
            egui::Stroke::new(DESIGN_TOKENS.stroke.hairline, theme.ui.text_secondary);
        // A focused text edit uses the selection stroke as its outline. Keep that outline neutral
        // in both themes instead of inheriting the light theme's accent colour.
        visuals.selection.stroke =
            egui::Stroke::new(DESIGN_TOKENS.stroke.hairline, theme.ui.text_secondary);
        ui.add(edit.margin(egui::Margin::symmetric(8, 4)))
    })
    .inner
}

/// Short muted datetime shown on the right of a coordinate row, e.g. `25 Oct 22:45`.
fn short_datetime(date: &str, time: &str) -> String {
    let Ok(date) = NaiveDate::parse_from_str(date, "%Y-%m-%d") else {
        return [date, time]
            .into_iter()
            .filter(|s| !s.is_empty())
            .collect::<Vec<_>>()
            .join(" ");
    };
    let time = NaiveTime::parse_from_str(time, "%H:%M:%S%.f")
        .or_else(|_| NaiveTime::parse_from_str(time, "%H:%M"))
        .unwrap_or_else(|_| NaiveTime::from_hms_opt(0, 0, 0).unwrap_or_default());
    format!(
        "{} {} {:02}:{:02}",
        date.day(),
        month_abbr(date.month()),
        time.hour(),
        time.minute()
    )
}

fn month_abbr(month: u32) -> &'static str {
    match month {
        1 => "Jan",
        2 => "Feb",
        3 => "Mar",
        4 => "Apr",
        5 => "May",
        6 => "Jun",
        7 => "Jul",
        8 => "Aug",
        9 => "Sep",
        10 => "Oct",
        11 => "Nov",
        12 => "Dec",
        _ => "—",
    }
}

fn parse_month(token: &str) -> Option<u32> {
    match token.to_ascii_lowercase().as_str() {
        "jan" | "january" => Some(1),
        "feb" | "february" => Some(2),
        "mar" | "march" => Some(3),
        "apr" | "april" => Some(4),
        "may" => Some(5),
        "jun" | "june" => Some(6),
        "jul" | "july" => Some(7),
        "aug" | "august" => Some(8),
        "sep" | "sept" | "september" => Some(9),
        "oct" | "october" => Some(10),
        "nov" | "november" => Some(11),
        "dec" | "december" => Some(12),
        _ => None,
    }
}

/// Accepts the short display form or ISO `YYYY-MM-DD HH:MM[:SS]`.
///
/// Short forms without a year (`25 Oct 22:45`) keep `fallback_year` from the existing point.
fn parse_datetime_input(input: &str, fallback_year: i32) -> Option<(String, String)> {
    let input = input.trim();
    if input.is_empty() {
        return Some((String::new(), String::new()));
    }

    if let Some((date, time)) = input.split_once(' ')
        && let Ok(d) = NaiveDate::parse_from_str(date, "%Y-%m-%d")
        && let Ok(t) = NaiveTime::parse_from_str(time, "%H:%M:%S%.f")
            .or_else(|_| NaiveTime::parse_from_str(time, "%H:%M"))
    {
        return Some((
            d.format("%Y-%m-%d").to_string(),
            format_time_storage(t, time.contains('.')),
        ));
    }

    // `25 Oct 22:45` or `25 Oct 2025 22:45`
    let parts: Vec<&str> = input.split_whitespace().collect();
    match parts.as_slice() {
        [day, month, clock] => {
            let day: u32 = day.parse().ok()?;
            let month = parse_month(month)?;
            let time = NaiveTime::parse_from_str(clock, "%H:%M:%S%.f")
                .or_else(|_| NaiveTime::parse_from_str(clock, "%H:%M"))
                .ok()?;
            let date = NaiveDate::from_ymd_opt(fallback_year, month, day)?;
            Some((
                date.format("%Y-%m-%d").to_string(),
                format_time_storage(time, clock.contains('.')),
            ))
        }
        [day, month, year, clock] => {
            let day: u32 = day.parse().ok()?;
            let month = parse_month(month)?;
            let year: i32 = year.parse().ok()?;
            let time = NaiveTime::parse_from_str(clock, "%H:%M:%S%.f")
                .or_else(|_| NaiveTime::parse_from_str(clock, "%H:%M"))
                .ok()?;
            let date = NaiveDate::from_ymd_opt(year, month, day)?;
            Some((
                date.format("%Y-%m-%d").to_string(),
                format_time_storage(time, clock.contains('.')),
            ))
        }
        _ => None,
    }
}

fn format_time_storage(time: NaiveTime, keep_fraction: bool) -> String {
    if keep_fraction {
        time.format("%H:%M:%S%.f").to_string()
    } else if time.second() == 0 && time.nanosecond() == 0 {
        time.format("%H:%M").to_string()
    } else {
        time.format("%H:%M:%S").to_string()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::BarData;

    #[test]
    fn regression_coordinates_edit_only_the_bar_range() {
        let chart = Chart::new(BarData::from_bars(Vec::new()));
        for tool in [DrawingToolType::RegressionTrend, DrawingToolType::TrendLine] {
            let mut drawing = Drawing::new(1, tool);
            drawing.chart_points = vec![ChartPoint::new(2.0, 123.5), ChartPoint::new(8.0, 90.0)];
            let mut state = CoordinatesState::from_drawing(&drawing, &chart);
            let ctx = egui::Context::default();
            let output = ctx.run_ui(egui::RawInput::default(), |ui| {
                CoordinatesTab::show(ui, &mut state)
            });
            output.drop_without_applying_deltas();
            assert_eq!(
                state.points.iter().filter(|p| !p.bar_only).count(),
                if tool == DrawingToolType::RegressionTrend {
                    0
                } else {
                    2
                }
            );
            state.points[0].price = "NaN".into();
            state.points[0].bar_index = "3".into();
            if tool == DrawingToolType::RegressionTrend {
                assert_eq!(
                    state.resolve(&chart).unwrap(),
                    vec![ChartPoint::new(3.0, 123.5), ChartPoint::new(8.0, 90.0)]
                );
            } else {
                assert!(state.resolve(&chart).is_err());
            }
        }
    }

    #[test]
    fn short_datetime_round_trips_for_resolve() {
        let (date, time) = parse_datetime_input("25 Oct 22:45", 2024).unwrap();
        assert_eq!(date, "2024-10-25");
        assert_eq!(time, "22:45");
        assert_eq!(short_datetime(&date, &time), "25 Oct 22:45");

        let (date, time) = parse_datetime_input("2025-10-29 14:44:00", 2024).unwrap();
        assert_eq!(date, "2025-10-29");
        assert_eq!(short_datetime(&date, &time), "29 Oct 14:44");
    }

    #[test]
    fn coordinate_price_is_compact_but_keeps_the_exact_anchor_until_edited() {
        let mut chart = Chart::new(BarData::from_bars(Vec::new()));
        chart.set_instrument(Some(
            crate::model::Instrument::new("TEST").with_price_decimals(2),
        ));
        let exact = 80_368.418_314_22;
        let mut drawing = Drawing::new(1, DrawingToolType::TrendLine);
        drawing.chart_points = vec![ChartPoint::new(2.0, exact)];

        let mut state = CoordinatesState::from_drawing(&drawing, &chart);
        assert_eq!(state.points[0].price, "80368.42");
        assert!(state.resolve(&chart).unwrap().is_empty());

        state.points[0].price = "80368.43".into();
        assert_eq!(state.resolve(&chart).unwrap()[0].price, 80_368.43);
    }

    #[test]
    fn coordinate_price_respects_instrument_precision_above_twelve_places() {
        assert_eq!(
            display_price(1.234_567_890_123_456, Some(14)),
            "1.23456789012346"
        );
    }
}
