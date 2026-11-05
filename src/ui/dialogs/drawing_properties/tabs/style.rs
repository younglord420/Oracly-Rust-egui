//! Appearance controls for the drawing properties panel.

use crate::drawings::{ArrowStyle, DrawingToolType};
use crate::icons::icons;
use crate::localization as l10n;
use crate::model::LineStyle;
use crate::painting::dashed_line::draw_dashed_line;
use crate::theme::ThemeExt;
use crate::tokens::DESIGN_TOKENS;
use crate::ui::dialogs::drawing_properties::actions::DrawingProps;
use crate::ui_kit::{ColorPicker, ColorPickerState};
use egui::{
    Color32, FontId, PopupCloseBehavior, Pos2, Rect, RectAlign, Role, Sense, Stroke, StrokeKind,
    Ui, Vec2, WidgetInfo,
};

/// Line widths offered in the Appearance width combo.
const LINE_WIDTHS: [f32; 4] = [1.0, 2.0, 3.0, 4.0];

/// Primary line styles shown in the Appearance style combo.
const LINE_STYLES: [LineStyle; 3] = [LineStyle::Solid, LineStyle::Dashed, LineStyle::Dotted];
const LINE_PICKER: usize = 0;
const FILL_PICKER: usize = 1;
const FIB_TREND_PICKER: usize = 2;

/// Standard control height for Drawing Properties rows (28px).
pub(super) fn control_row_height(_ui: &Ui) -> f32 {
    28.0
}

/// Shared width of the right-aligned control column across property rows.
pub(super) fn control_column_width(ui: &Ui) -> f32 {
    let avail = ui.available_width();
    if avail < 260.0 {
        (avail * 0.68).clamp(148.0, 184.0)
    } else {
        (avail * 0.62).clamp(184.0, 200.0)
    }
}

fn has_fib_style_controls(tool: DrawingToolType) -> bool {
    matches!(
        tool,
        DrawingToolType::FibonacciRetracement
            | DrawingToolType::FibonacciExtension
            | DrawingToolType::FibonacciChannel
    )
}

fn shows_generic_line_row(tool: DrawingToolType) -> bool {
    !has_fib_style_controls(tool) && tool != DrawingToolType::Pitchfan
}

/// Fibonacci and Gann tools whose lines take their colours from their levels: the Line row
/// offers width and style only.
fn colours_from_levels(tool: DrawingToolType) -> bool {
    matches!(
        tool,
        DrawingToolType::FibonacciTimeZones
            | DrawingToolType::FibonacciSpeedFan
            | DrawingToolType::TrendBasedFibTime
            | DrawingToolType::FibonacciCircles
            | DrawingToolType::FibonacciSpeedResistanceArcs
            | DrawingToolType::FibonacciWedge
            | DrawingToolType::GannFan
            | DrawingToolType::GannBox
            | DrawingToolType::GannSquare
            | DrawingToolType::GannFixed
    )
}

/// Style / Appearance section renderer.
pub struct StyleTab;

impl StyleTab {
    /// Dense Appearance rows: color, width, style; optional extend and ends on aligned rows.
    pub fn show(ui: &mut Ui, props: &mut DrawingProps, pickers: &mut Vec<ColorPickerState>) {
        let tool = props.tool_settings.as_ref().map(|t| t.settings.tool_type);
        let regression = tool == Some(DrawingToolType::RegressionTrend);
        let fib_tool = tool.is_some_and(has_fib_style_controls);
        let picker_count = if regression {
            7
        } else if fib_tool {
            FIB_TREND_PICKER + 1
        } else {
            FILL_PICKER + 1
        };
        if pickers.len() < picker_count {
            pickers.resize_with(picker_count, ColorPickerState::default);
        }
        let theme = ui.active_theme();
        let gap = 6.0;
        let control_h = control_row_height(ui);
        ui.spacing_mut().interact_size.y = control_h;
        let compact = ui.available_width() < 260.0;
        let col_w = control_column_width(ui);
        let swatch_w = 28.0;
        let pair_w = (col_w - swatch_w - gap * 2.0).max(112.0);
        let width_chip_w = (pair_w * 0.48).round();
        let style_chip_w = (pair_w - width_chip_w).max(56.0);
        let ends_combo_w = col_w;

        if tool.is_none_or(shows_generic_line_row) {
            ui.horizontal(|ui| {
                ui.set_min_height(control_h);
                ui.spacing_mut().item_spacing.x = gap;
                if !compact {
                    ui.label(
                        egui::RichText::new(l10n::text(ui.ctx(), "Line"))
                            .size(theme.text.body)
                            .color(theme.ui.text_secondary),
                    );
                }
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    ui.allocate_ui_with_layout(
                        Vec2::new(col_w.min(ui.available_width()), control_h),
                        egui::Layout::left_to_right(egui::Align::Center),
                        |ui| {
                            ui.spacing_mut().item_spacing.x = gap;
                            if !tool.is_some_and(colours_from_levels)
                                && let Some(color) = color_swatch(
                                    ui,
                                    &mut pickers[LINE_PICKER],
                                    props.color,
                                    "drawing_props_color",
                                    true,
                                )
                            {
                                props.color = color;
                            }
                            line_width_combo(ui, &mut props.line_width, width_chip_w, control_h);
                            line_style_combo(
                                ui,
                                "drawing_props_line_style",
                                "Line style",
                                &mut props.line_style,
                                style_chip_w,
                                control_h,
                            );
                        },
                    );
                });
            });
        }

        let show_extend = tool.is_some_and(|tool| tool.supports_extend());
        let show_ends = props.tool_settings.as_ref().is_some_and(|t| {
            matches!(
                t.settings.tool_type,
                DrawingToolType::TrendLine
                    | DrawingToolType::Ray
                    | DrawingToolType::ExtendedLine
                    | DrawingToolType::InfoLine
                    | DrawingToolType::TrendAngle
                    | DrawingToolType::Arrow
                    | DrawingToolType::Path
                    | DrawingToolType::Polyline
            )
        });
        // Fib tools edit extend only via fib_config in fib_style_controls below.
        if show_extend && !fib_tool {
            extend_control(ui, &mut props.extend_left, &mut props.extend_right, col_w);
        }
        if show_ends && let Some(template) = &mut props.tool_settings {
            ends_combo(ui, &mut template.settings.arrow_end_style, ends_combo_w);
        }

        if let Some(template) = &mut props.tool_settings {
            if matches!(
                template.settings.tool_type,
                DrawingToolType::Rect | DrawingToolType::ParallelChannel
            ) {
                ui.checkbox(
                    &mut template.settings.channel_midline,
                    egui::RichText::new(l10n::text(ui.ctx(), "Show middle line"))
                        .size(theme.text.body),
                );
            } else if matches!(
                template.settings.tool_type,
                DrawingToolType::FlatTopBottom | DrawingToolType::DisjointChannel
            ) {
                ui.checkbox(
                    &mut template.settings.show_price_labels,
                    egui::RichText::new(l10n::text(ui.ctx(), "Price labels")).size(theme.text.body),
                );
            }
        }

        if fib_tool && let Some(template) = &mut props.tool_settings {
            let tool_type = template.settings.tool_type;
            let config = template
                .settings
                .fib_config
                .get_or_insert_with(|| crate::drawings::FibonacciConfig::for_tool(tool_type));
            fib_style_controls(
                ui,
                config,
                &mut pickers[FIB_TREND_PICKER],
                tool_type,
                col_w,
                width_chip_w,
                style_chip_w,
                control_h,
                gap,
            );
        }

        if tool == Some(DrawingToolType::RegressionTrend)
            && let Some(template) = &mut props.tool_settings
        {
            let config = template
                .settings
                .regression_trend
                .get_or_insert_with(crate::drawings::RegressionTrendConfig::default);
            regression_style_controls(
                ui,
                config,
                &mut pickers[2..7],
                col_w,
                style_chip_w,
                control_h,
                gap,
            );
        }
    }

    /// Fill section body when the tool paints a fill.
    pub fn show_fill(ui: &mut Ui, props: &mut DrawingProps, pickers: &mut Vec<ColorPickerState>) {
        if pickers.len() < 2 {
            pickers.resize_with(2, ColorPickerState::default);
        }
        let fill_picker = &mut pickers[FILL_PICKER];
        let theme = ui.active_theme();
        let control_h = control_row_height(ui);
        ui.spacing_mut().interact_size.y = control_h;
        ui.horizontal(|ui| {
            ui.set_min_height(control_h);
            ui.spacing_mut().item_spacing.x = theme.spacing.sm;
            ui.checkbox(
                &mut props.fill_enabled,
                egui::RichText::new(l10n::text(ui.ctx(), "Enable fill")).size(theme.text.body),
            );
            if props.fill_enabled {
                let mut fill = *props
                    .fill_color
                    .get_or_insert(props.color.gamma_multiply(0.2));
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    ui.spacing_mut().item_spacing.x = theme.spacing.sm;
                    let mut pct = f32::from(fill.a()) * 100.0 / 255.0;
                    if opacity_slider(ui, &mut pct, true).changed() {
                        let a = (pct * 255.0 / 100.0).round() as u8;
                        let [r, g, b, _] = fill.to_srgba_unmultiplied();
                        fill = Color32::from_rgba_unmultiplied(r, g, b, a);
                    }
                    if let Some(picked) =
                        color_swatch(ui, fill_picker, fill, "drawing_props_fill", true)
                    {
                        fill = picked;
                    }
                });
                props.fill_color = Some(fill);
            }
        });
    }
}

pub(super) fn color_swatch(
    ui: &mut Ui,
    state: &mut ColorPickerState,
    color: Color32,
    id: &str,
    compact: bool,
) -> Option<Color32> {
    state.sel_color = color;
    let mut picker = ColorPicker::new(state, id);
    if compact {
        picker = picker.well_size(28.0);
    }
    picker.show(ui).inner
}

/// Visual stroke-width selector showing the actual line thickness alongside the pixel value.
pub(super) fn line_width_combo(ui: &mut Ui, width: &mut f32, chip_w: f32, chip_h: f32) {
    line_width_combo_with_id(
        ui,
        "drawing_props_line_width",
        "Line width",
        width,
        chip_w,
        chip_h,
    );
}

pub(super) fn line_width_combo_with_id(
    ui: &mut Ui,
    id_salt: impl std::hash::Hash + std::fmt::Debug,
    a11y_name: &str,
    width: &mut f32,
    chip_w: f32,
    chip_h: f32,
) {
    let theme = ui.active_theme();
    let label = format!("{}px", (*width).round() as i32);
    let (rect, response) = ui.allocate_exact_size(Vec2::new(chip_w, chip_h), Sense::click());
    let a11y_label = l10n::text(ui.ctx(), a11y_name);
    response.widget_info(|| WidgetInfo::labeled(Role::ComboBox, ui.is_enabled(), &a11y_label));

    let id = ui.make_persistent_id(id_salt);
    let is_open = egui::Popup::is_id_open(ui.ctx(), id);
    paint_control_box(ui, rect, &response, is_open);

    let chevron = DESIGN_TOKENS.sizing.icon_sm;
    let pad_x = theme.spacing.md;
    let galley =
        ui.painter()
            .layout_no_wrap(label, FontId::proportional(theme.text.body), theme.ui.text);
    let chevron_x = rect.max.x - pad_x - chevron * 0.5;
    let text_x = chevron_x - chevron * 0.5 - theme.spacing.xs - galley.size().x;
    let line_x0 = rect.min.x + pad_x;
    let line_x1 = (text_x - theme.spacing.sm).max(line_x0 + 12.0);
    let line_y = rect.center().y.round();

    ui.painter().line_segment(
        [Pos2::new(line_x0, line_y), Pos2::new(line_x1, line_y)],
        Stroke::new((*width).clamp(1.0, 4.0), theme.ui.text),
    );
    ui.painter().galley(
        Pos2::new(
            (line_x1 + theme.spacing.sm).min(text_x),
            rect.center().y - galley.size().y * 0.5,
        ),
        galley,
        theme.ui.text,
    );
    icons::CHEVRON_DOWN
        .as_image_tinted(Vec2::splat(chevron), theme.ui.text_muted)
        .paint_at(
            ui,
            Rect::from_center_size(Pos2::new(chevron_x, rect.center().y), Vec2::splat(chevron)),
        );

    let mut presets: Vec<f32> = LINE_WIDTHS.to_vec();
    if !presets.iter().any(|w| (*width - *w).abs() < f32::EPSILON) {
        presets.push(*width);
    }

    let menu_w = chip_w.max(116.0);
    egui::Popup::menu(&response)
        .id(id)
        .width(menu_w)
        .close_behavior(PopupCloseBehavior::CloseOnClickOutside)
        .align(RectAlign::BOTTOM_START)
        .align_alternatives(&[])
        .frame(crate::ui_kit::glass_menu(ui))
        .gap(theme.spacing.xs)
        .show(|ui| {
            ui.set_width(menu_w);
            ui.set_max_width(menu_w);
            ui.spacing_mut().item_spacing.y = 2.0;
            for preset in presets {
                let selected = (*width - preset).abs() < f32::EPSILON;
                let row_h = 30.0;
                let (row, resp) = ui.allocate_exact_size(Vec2::new(menu_w, row_h), Sense::click());
                paint_menu_item_bg(ui, row, selected, resp.hovered());
                let y = row.center().y.round();
                ui.painter().line_segment(
                    [
                        Pos2::new(row.min.x + 12.0, y),
                        Pos2::new(row.min.x + 46.0, y),
                    ],
                    Stroke::new(preset.clamp(1.0, 4.0), theme.ui.text),
                );
                let text = format!("{}px", preset.round() as i32);
                ui.painter().text(
                    Pos2::new(row.min.x + 58.0, row.center().y),
                    egui::Align2::LEFT_CENTER,
                    &text,
                    FontId::proportional(theme.text.body),
                    if selected {
                        theme.ui.accent
                    } else {
                        theme.ui.text
                    },
                );
                resp.widget_info(|| {
                    WidgetInfo::selected(Role::Button, ui.is_enabled(), selected, &text)
                });
                if resp.clicked() {
                    *width = preset;
                    ui.close();
                }
            }
        });
}

/// Visual line-style selector showing painted solid / dashed / dotted line graphics.
pub(super) fn line_style_combo(
    ui: &mut Ui,
    id_salt: impl std::hash::Hash + std::fmt::Debug,
    a11y_name: &str,
    style: &mut LineStyle,
    chip_w: f32,
    chip_h: f32,
) {
    let theme = ui.active_theme();
    let mut choices: Vec<LineStyle> = LINE_STYLES.to_vec();
    if !choices.contains(style) {
        choices.push(*style);
    }

    let (rect, response) = ui.allocate_exact_size(Vec2::new(chip_w, chip_h), Sense::click());
    let a11y_label = l10n::text(ui.ctx(), a11y_name);
    response.widget_info(|| WidgetInfo::labeled(Role::ComboBox, ui.is_enabled(), &a11y_label));

    let id = ui.make_persistent_id(id_salt);
    let is_open = egui::Popup::is_id_open(ui.ctx(), id);
    paint_control_box(ui, rect, &response, is_open);

    let chevron = DESIGN_TOKENS.sizing.icon_sm;
    let pad_x = theme.spacing.md;
    let chevron_x = rect.max.x - pad_x - chevron * 0.5;
    let preview_x0 = rect.min.x + pad_x;
    let preview_x1 = (chevron_x - chevron * 0.5 - theme.spacing.sm).max(preview_x0 + 18.0);
    let sample_rect = Rect::from_min_max(
        Pos2::new(preview_x0, rect.center().y - 1.0),
        Pos2::new(preview_x1, rect.center().y + 1.0),
    );
    paint_line_style_sample(ui, sample_rect, *style, theme.ui.text);

    icons::CHEVRON_DOWN
        .as_image_tinted(Vec2::splat(chevron), theme.ui.text_muted)
        .paint_at(
            ui,
            Rect::from_center_size(Pos2::new(chevron_x, rect.center().y), Vec2::splat(chevron)),
        );

    let menu_w = chip_w.max(140.0);
    egui::Popup::menu(&response)
        .id(id)
        .width(menu_w)
        .close_behavior(PopupCloseBehavior::CloseOnClickOutside)
        .align(RectAlign::BOTTOM_START)
        .align_alternatives(&[])
        .frame(crate::ui_kit::glass_menu(ui))
        .gap(theme.spacing.xs)
        .show(|ui| {
            ui.set_width(menu_w);
            ui.set_max_width(menu_w);
            ui.spacing_mut().item_spacing.y = 2.0;
            for choice in choices {
                let selected = *style == choice;
                let row_h = 30.0;
                let (row, resp) = ui.allocate_exact_size(Vec2::new(menu_w, row_h), Sense::click());
                paint_menu_item_bg(ui, row, selected, resp.hovered());
                let sample = Rect::from_min_max(
                    Pos2::new(row.min.x + 12.0, row.center().y - 1.0),
                    Pos2::new(row.min.x + 48.0, row.center().y + 1.0),
                );
                paint_line_style_sample(ui, sample, choice, theme.ui.text);
                let label = l10n::text(ui.ctx(), choice.name());
                ui.painter().text(
                    Pos2::new(row.min.x + 60.0, row.center().y),
                    egui::Align2::LEFT_CENTER,
                    &label,
                    FontId::proportional(theme.text.body),
                    if selected {
                        theme.ui.accent
                    } else {
                        theme.ui.text
                    },
                );
                resp.widget_info(|| {
                    WidgetInfo::selected(Role::Button, ui.is_enabled(), selected, &label)
                });
                if resp.clicked() {
                    *style = choice;
                    ui.close();
                }
            }
        });
}

pub(super) fn paint_line_style_sample(ui: &Ui, rect: Rect, style: LineStyle, color: Color32) {
    let stroke = Stroke::new(2.0, color);
    let y = rect.center().y.round();
    let start = Pos2::new(rect.min.x, y);
    let end = Pos2::new(rect.max.x, y);
    match style.dash_pattern() {
        None => {
            ui.painter().line_segment([start, end], stroke);
        }
        Some((dash, gap)) => {
            draw_dashed_line(ui.painter(), start, end, stroke, dash, gap);
        }
    }
}

pub(super) fn paint_menu_item_bg(ui: &Ui, row: Rect, selected: bool, hovered: bool) {
    let theme = ui.active_theme();
    if selected || hovered {
        let fill = if hovered {
            theme.ui.btn_bg_active
        } else {
            theme.ui.btn_bg_hover
        };
        ui.painter().rect_filled(row, theme.radius.sm, fill);
    }
}

pub(super) fn paint_control_box(ui: &Ui, rect: Rect, response: &egui::Response, is_open: bool) {
    let theme = ui.active_theme();
    let radius = theme.radius.sm;
    let fill = if is_open || response.hovered() {
        theme.ui.btn_bg_hover
    } else {
        theme.ui.panel_bg
    };
    ui.painter().rect_filled(rect, radius, fill);
    let border = if is_open {
        theme.ui.text
    } else if response.hovered() {
        theme.ui.border_control
    } else {
        theme.ui.border
    };
    ui.painter().rect_stroke(
        rect,
        radius,
        Stroke::new(DESIGN_TOKENS.stroke.hairline, border),
        StrokeKind::Inside,
    );
    crate::styles::focus::draw_focus_ring(ui, response, radius);
}

/// Custom text dropdown matching the visual chips (fixed popup width, no muddy selection box).
pub(super) fn text_combo_popup<T: Copy + PartialEq>(
    ui: &mut Ui,
    id_salt: impl std::hash::Hash + std::fmt::Debug,
    a11y_name: &str,
    current: &mut T,
    choices: &[T],
    label_of: impl Fn(T) -> String,
    chip_w: f32,
    chip_h: f32,
) {
    let theme = ui.active_theme();
    let selected_text = label_of(*current);
    let (rect, response) = ui.allocate_exact_size(Vec2::new(chip_w, chip_h), Sense::click());
    let a11y_label = l10n::text(ui.ctx(), a11y_name);
    response.widget_info(|| WidgetInfo::labeled(Role::ComboBox, ui.is_enabled(), &a11y_label));

    let id = ui.make_persistent_id(id_salt);
    let is_open = egui::Popup::is_id_open(ui.ctx(), id);
    paint_control_box(ui, rect, &response, is_open);

    let chevron = DESIGN_TOKENS.sizing.icon_sm;
    let pad_x = theme.spacing.md;
    let chevron_x = rect.max.x - pad_x - chevron * 0.5;
    let galley = ui.painter().layout_no_wrap(
        selected_text,
        FontId::proportional(theme.text.body),
        theme.ui.text,
    );
    ui.painter().galley(
        Pos2::new(rect.min.x + pad_x, rect.center().y - galley.size().y * 0.5),
        galley,
        theme.ui.text,
    );
    icons::CHEVRON_DOWN
        .as_image_tinted(Vec2::splat(chevron), theme.ui.text_muted)
        .paint_at(
            ui,
            Rect::from_center_size(Pos2::new(chevron_x, rect.center().y), Vec2::splat(chevron)),
        );

    let menu_w = chip_w.max(136.0);
    egui::Popup::menu(&response)
        .id(id)
        .width(menu_w)
        .close_behavior(PopupCloseBehavior::CloseOnClickOutside)
        .align(RectAlign::BOTTOM_START)
        .align_alternatives(&[])
        .frame(crate::ui_kit::glass_menu(ui))
        .gap(theme.spacing.xs)
        .show(|ui| {
            ui.set_width(menu_w);
            ui.set_max_width(menu_w);
            ui.spacing_mut().item_spacing.y = 2.0;
            for &choice in choices {
                let selected = *current == choice;
                let row_h = 30.0;
                let (row, resp) = ui.allocate_exact_size(Vec2::new(menu_w, row_h), Sense::click());
                paint_menu_item_bg(ui, row, selected, resp.hovered());
                let label = label_of(choice);
                ui.painter().text(
                    Pos2::new(row.min.x + 12.0, row.center().y),
                    egui::Align2::LEFT_CENTER,
                    &label,
                    FontId::proportional(theme.text.body),
                    if selected {
                        theme.ui.accent
                    } else {
                        theme.ui.text
                    },
                );
                resp.widget_info(|| {
                    WidgetInfo::selected(Role::Button, ui.is_enabled(), selected, &label)
                });
                if resp.clicked() {
                    *current = choice;
                    ui.close();
                }
            }
        });
}

/// Compact extend icons with soft selection — aligned to the same right-hand control column.
fn regression_style_controls(
    ui: &mut Ui,
    config: &mut crate::drawings::RegressionTrendConfig,
    pickers: &mut [ColorPickerState],
    col_w: f32,
    style_chip_w: f32,
    control_h: f32,
    gap: f32,
) {
    let theme = ui.active_theme();

    regression_line_row(
        ui,
        pickers,
        0,
        "Midline",
        &mut config.midline_color,
        &mut config.midline_style,
        "drawing_props_reg_midline_color",
        "drawing_props_reg_midline_style",
        col_w,
        style_chip_w,
        control_h,
        gap,
        &theme,
    );
    regression_line_row(
        ui,
        pickers,
        1,
        "Upper line",
        &mut config.up_color,
        &mut config.upper_style,
        "drawing_props_reg_upper_color",
        "drawing_props_reg_upper_style",
        col_w,
        style_chip_w,
        control_h,
        gap,
        &theme,
    );
    regression_line_row(
        ui,
        pickers,
        2,
        "Lower line",
        &mut config.down_color,
        &mut config.lower_style,
        "drawing_props_reg_lower_color",
        "drawing_props_reg_lower_style",
        col_w,
        style_chip_w,
        control_h,
        gap,
        &theme,
    );
    regression_fill_row(
        ui,
        pickers,
        3,
        "Upper fill",
        &mut config.show_upper_fill,
        &mut config.upper_fill,
        "drawing_props_reg_upper_fill",
        control_h,
        gap,
        &theme,
    );
    regression_fill_row(
        ui,
        pickers,
        4,
        "Lower fill",
        &mut config.show_lower_fill,
        &mut config.lower_fill,
        "drawing_props_reg_lower_fill",
        control_h,
        gap,
        &theme,
    );
}

fn regression_line_row(
    ui: &mut Ui,
    pickers: &mut [ColorPickerState],
    picker_index: usize,
    label: &str,
    color: &mut [u8; 4],
    style: &mut LineStyle,
    color_id: &str,
    style_id: &str,
    col_w: f32,
    style_chip_w: f32,
    control_h: f32,
    gap: f32,
    theme: &crate::theme::Theme,
) {
    ui.horizontal(|ui| {
        ui.set_min_height(control_h);
        ui.spacing_mut().item_spacing.x = gap;
        ui.label(
            egui::RichText::new(l10n::text(ui.ctx(), label))
                .size(theme.text.body)
                .color(theme.ui.text_secondary),
        );
        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
            ui.allocate_ui_with_layout(
                Vec2::new(col_w.min(ui.available_width()), control_h),
                egui::Layout::left_to_right(egui::Align::Center),
                |ui| {
                    ui.spacing_mut().item_spacing.x = gap;
                    let mut c =
                        Color32::from_rgba_unmultiplied(color[0], color[1], color[2], color[3]);
                    if let Some(picked) =
                        color_swatch(ui, &mut pickers[picker_index], c, color_id, true)
                    {
                        c = picked;
                        *color = c.to_srgba_unmultiplied();
                    }
                    line_style_combo(
                        ui,
                        style_id,
                        &format!("{label} style"),
                        style,
                        style_chip_w,
                        control_h,
                    );
                },
            );
        });
    });
}

fn regression_fill_row(
    ui: &mut Ui,
    pickers: &mut [ColorPickerState],
    picker_index: usize,
    label: &str,
    show: &mut bool,
    fill: &mut [u8; 4],
    color_id: &str,
    control_h: f32,
    gap: f32,
    theme: &crate::theme::Theme,
) {
    ui.horizontal(|ui| {
        ui.set_min_height(control_h);
        ui.spacing_mut().item_spacing.x = gap;
        ui.checkbox(
            show,
            egui::RichText::new(l10n::text(ui.ctx(), label))
                .size(theme.text.body)
                .color(theme.ui.text_secondary),
        );
        if *show {
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                ui.spacing_mut().item_spacing.x = gap;
                let mut c = Color32::from_rgba_unmultiplied(fill[0], fill[1], fill[2], fill[3]);
                let mut pct = f32::from(c.a()) * 100.0 / 255.0;
                if opacity_slider(ui, &mut pct, true).changed() {
                    fill[3] = (pct * 255.0 / 100.0).round() as u8;
                    c = Color32::from_rgba_unmultiplied(fill[0], fill[1], fill[2], fill[3]);
                }
                if let Some(picked) =
                    color_swatch(ui, &mut pickers[picker_index], c, color_id, true)
                {
                    let a = fill[3];
                    let mut rgba = picked.to_srgba_unmultiplied();
                    rgba[3] = a;
                    *fill = rgba;
                }
            });
        }
    });
}

/// Opacity control: a thin track that fades from clear to white, a white knob, and a
/// percent that stays visible beside the track.
pub(super) fn opacity_slider(ui: &mut Ui, percent: &mut f32, enabled: bool) -> egui::Response {
    let theme = ui.active_theme();
    let value = percent.clamp(0.0, 100.0);
    let label = format!("{}%", value.round() as i32);
    let font = FontId::proportional(theme.text.body);
    let label_w = ui
        .painter()
        .layout_no_wrap(label.clone(), font.clone(), theme.ui.text)
        .size()
        .x
        + 8.0;
    let track_w = (ui.available_width() - label_w).clamp(56.0, 96.0);
    let sense = if enabled && ui.is_enabled() {
        Sense::click_and_drag()
    } else {
        Sense::hover()
    };
    let (rect, mut response) = ui.allocate_exact_size(Vec2::new(track_w + label_w, 22.0), sense);
    let track = Rect::from_center_size(
        Pos2::new(rect.min.x + track_w * 0.5, rect.center().y),
        Vec2::new(track_w, 8.0),
    );
    if enabled
        && ui.is_enabled()
        && (response.clicked() || response.dragged())
        && let Some(pos) = response.interact_pointer_pos()
    {
        let t = ((pos.x - track.min.x) / track.width()).clamp(0.0, 1.0);
        let next = (t * 100.0).round().clamp(0.0, 100.0);
        if (next - *percent).abs() > f32::EPSILON {
            *percent = next;
            response.mark_changed();
        }
    }
    let shown = percent.clamp(0.0, 100.0);
    let active = enabled && ui.is_enabled();
    paint_opacity_track(ui.painter(), track, active);
    let knob = Pos2::new(
        track.min.x + track.width() * (shown / 100.0),
        track.center().y,
    );
    ui.painter().circle_filled(
        knob,
        7.0,
        if active {
            Color32::WHITE
        } else {
            Color32::from_rgb(168, 168, 168)
        },
    );
    ui.painter()
        .circle_stroke(knob, 7.0, Stroke::new(1.0, Color32::from_black_alpha(48)));
    ui.painter().text(
        Pos2::new(rect.right(), rect.center().y),
        egui::Align2::RIGHT_CENTER,
        format!("{}%", shown.round() as i32),
        font,
        if active {
            theme.ui.text
        } else {
            theme.ui.text_disabled
        },
    );
    response.widget_info(|| WidgetInfo::slider(active, f64::from(shown), &label));
    response
}

fn paint_opacity_track(painter: &egui::Painter, track: Rect, active: bool) {
    let radius = (track.height() * 0.5).round().clamp(1.0, 8.0) as u8;
    let base = if active {
        Color32::from_rgb(32, 34, 38)
    } else {
        Color32::from_rgb(48, 48, 48)
    };
    painter.rect_filled(track, radius, base);
    let steps = 14;
    for i in 0..steps {
        let t0 = i as f32 / steps as f32;
        let t1 = (i + 1) as f32 / steps as f32;
        let slice = Rect::from_min_max(
            Pos2::new(track.min.x + track.width() * t0, track.min.y),
            Pos2::new(track.min.x + track.width() * t1, track.max.y),
        );
        let alpha = ((t0 + t1) * 0.5 * if active { 255.0 } else { 90.0 }).round() as u8;
        let rounding = egui::CornerRadius {
            nw: if i == 0 { radius } else { 0 },
            sw: if i == 0 { radius } else { 0 },
            ne: if i + 1 == steps { radius } else { 0 },
            se: if i + 1 == steps { radius } else { 0 },
        };
        painter.rect_filled(slice, rounding, Color32::from_white_alpha(alpha));
    }
}

fn fib_style_controls(
    ui: &mut Ui,
    config: &mut crate::drawings::FibonacciConfig,
    color_picker: &mut ColorPickerState,
    tool_type: DrawingToolType,
    col_w: f32,
    width_chip_w: f32,
    style_chip_w: f32,
    control_h: f32,
    gap: f32,
) {
    let theme = ui.active_theme();
    let show_trend = matches!(
        tool_type,
        DrawingToolType::FibonacciRetracement | DrawingToolType::FibonacciExtension
    );

    if show_trend {
        ui.horizontal(|ui| {
            ui.set_min_height(control_h);
            ui.spacing_mut().item_spacing.x = gap;
            ui.checkbox(
                &mut config.show_trend_line,
                egui::RichText::new(l10n::text(ui.ctx(), "Trend line"))
                    .size(theme.text.body)
                    .color(theme.ui.text_secondary),
            );
            if config.show_trend_line {
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    ui.allocate_ui_with_layout(
                        Vec2::new(col_w.min(ui.available_width()), control_h),
                        egui::Layout::left_to_right(egui::Align::Center),
                        |ui| {
                            ui.spacing_mut().item_spacing.x = gap;
                            let mut color = Color32::from_rgba_unmultiplied(
                                config.trend_line_color[0],
                                config.trend_line_color[1],
                                config.trend_line_color[2],
                                config.trend_line_color[3],
                            );
                            if let Some(picked) = color_swatch(
                                ui,
                                color_picker,
                                color,
                                "drawing_props_fib_trend_color",
                                true,
                            ) {
                                color = picked;
                                config.trend_line_color = color.to_srgba_unmultiplied();
                            }
                            line_width_combo_with_id(
                                ui,
                                "drawing_props_fib_trend_width",
                                "Trend line width",
                                &mut config.trend_line_width,
                                width_chip_w,
                                control_h,
                            );
                            line_style_combo(
                                ui,
                                "drawing_props_fib_trend_style",
                                "Trend line style",
                                &mut config.trend_line_style,
                                style_chip_w,
                                control_h,
                            );
                        },
                    );
                });
            }
        });
    }

    ui.horizontal(|ui| {
        ui.set_min_height(control_h);
        ui.spacing_mut().item_spacing.x = gap;
        ui.label(
            egui::RichText::new(l10n::text(ui.ctx(), "Levels"))
                .size(theme.text.body)
                .color(theme.ui.text_secondary),
        );
        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
            ui.allocate_ui_with_layout(
                Vec2::new(col_w.min(ui.available_width()), control_h),
                egui::Layout::right_to_left(egui::Align::Center),
                |ui| {
                    ui.spacing_mut().item_spacing.x = gap;
                    line_style_combo(
                        ui,
                        "drawing_props_fib_levels_style",
                        "Levels line style",
                        &mut config.levels_line_style,
                        style_chip_w,
                        control_h,
                    );
                    line_width_combo_with_id(
                        ui,
                        "drawing_props_fib_levels_width",
                        "Levels line width",
                        &mut config.levels_line_width,
                        width_chip_w,
                        control_h,
                    );
                },
            );
        });
    });

    // Only the retracement's levels run on past their anchors.
    if tool_type == DrawingToolType::FibonacciRetracement {
        extend_control(ui, &mut config.extend_left, &mut config.extend_right, col_w);
    }
}

fn extend_control(ui: &mut Ui, left: &mut bool, right: &mut bool, col_w: f32) {
    let theme = ui.active_theme();
    let options = [
        (false, false, "None"),
        (true, false, "Left"),
        (false, true, "Right"),
        (true, true, "Both"),
    ];
    let control_h = control_row_height(ui);
    let gap = 4.0;
    let count = options.len() as f32;
    let strip_w = col_w.min(ui.available_width());
    let btn_w = ((strip_w - gap * (count - 1.0)) / count).max(24.0);
    let size = Vec2::new(btn_w, control_h);

    ui.horizontal(|ui| {
        ui.set_min_height(control_h);
        ui.spacing_mut().item_spacing.x = theme.spacing.sm;
        ui.label(
            egui::RichText::new(l10n::text(ui.ctx(), "Extend"))
                .size(theme.text.body)
                .color(theme.ui.text_secondary),
        );
        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
            ui.allocate_ui_with_layout(
                Vec2::new(strip_w, control_h),
                egui::Layout::left_to_right(egui::Align::Center),
                |ui| {
                    ui.spacing_mut().item_spacing.x = gap;
                    for (l, r, name) in options {
                        let selected = *left == l && *right == r;
                        let (rect, response) = ui.allocate_exact_size(size, Sense::click());
                        if response.clicked() {
                            *left = l;
                            *right = r;
                        }
                        let label = l10n::format_text(
                            ui.ctx(),
                            "Extend {name}",
                            &[("name", name.to_owned())],
                        );
                        response.widget_info(|| {
                            WidgetInfo::selected(Role::Button, ui.is_enabled(), selected, &label)
                        });
                        paint_soft_selected(ui, rect, &response, selected);
                        let fg = if selected {
                            theme.ui.text
                        } else {
                            theme.ui.text_muted
                        };
                        paint_extend_arrow(ui, rect, l, r, fg);
                    }
                },
            )
            .response
            .accessible_name(l10n::text(ui.ctx(), "Extend"));
        });
    });
}

/// The extend directions as compact, crisp vectors.
///
/// The interface font a lean host installs carries no arrow glyphs, so these must not be text.
fn paint_extend_arrow(ui: &Ui, rect: egui::Rect, left: bool, right: bool, color: Color32) {
    let painter = ui.painter();
    let stroke = Stroke::new(1.25, color);
    let y = rect.center().y.round();
    let half = 6.0_f32.min((rect.width() * 0.25).round());
    let (x0, x1) = (rect.center().x - half, rect.center().x + half);
    painter.line_segment([egui::pos2(x0, y), egui::pos2(x1, y)], stroke);

    let head = 3.0_f32;
    for (tip, direction) in [(x0, 1.0), (x1, -1.0)] {
        if (direction > 0.0 && !left) || (direction < 0.0 && !right) {
            continue;
        }
        for offset in [-head, head] {
            painter.line_segment(
                [
                    egui::pos2(tip, y),
                    egui::pos2(tip + head * direction, y + offset),
                ],
                stroke,
            );
        }
    }
}

fn ends_combo(ui: &mut Ui, end: &mut ArrowStyle, combo_w: f32) {
    let theme = ui.active_theme();
    let control_h = control_row_height(ui);
    let mut choices = vec![ArrowStyle::None, ArrowStyle::Arrow];
    if !choices.contains(end) {
        choices.push(*end);
    }
    ui.horizontal(|ui| {
        ui.set_min_height(control_h);
        ui.spacing_mut().item_spacing.x = theme.spacing.sm;
        ui.label(
            egui::RichText::new(l10n::text(ui.ctx(), "Ends"))
                .size(theme.text.body)
                .color(theme.ui.text_secondary),
        );
        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
            let w = combo_w.min(ui.available_width());
            let (rect, response) = ui.allocate_exact_size(Vec2::new(w, control_h), Sense::click());
            let a11y_label = l10n::text(ui.ctx(), "Line end");
            response
                .widget_info(|| WidgetInfo::labeled(Role::ComboBox, ui.is_enabled(), &a11y_label));

            let id = ui.make_persistent_id("drawing_props_line_ends");
            let is_open = egui::Popup::is_id_open(ui.ctx(), id);
            paint_control_box(ui, rect, &response, is_open);

            let chevron = DESIGN_TOKENS.sizing.icon_sm;
            let pad_x = theme.spacing.sm;
            let chevron_x = rect.max.x - pad_x - chevron * 0.5;
            let selected_text = l10n::text(ui.ctx(), arrow_label(*end));
            let galley = ui.painter().layout_no_wrap(
                selected_text,
                FontId::proportional(theme.text.body),
                theme.ui.text,
            );
            let sample_rect = Rect::from_min_max(
                Pos2::new(rect.min.x + pad_x, rect.center().y - 4.0),
                Pos2::new(rect.min.x + pad_x + 22.0, rect.center().y + 4.0),
            );
            paint_end_sample(ui, sample_rect, *end, theme.ui.text);
            ui.painter().galley(
                Pos2::new(
                    sample_rect.max.x + theme.spacing.sm,
                    rect.center().y - galley.size().y * 0.5,
                ),
                galley,
                theme.ui.text,
            );
            icons::CHEVRON_DOWN
                .as_image_tinted(Vec2::splat(chevron), theme.ui.text_muted)
                .paint_at(
                    ui,
                    Rect::from_center_size(
                        Pos2::new(chevron_x, rect.center().y),
                        Vec2::splat(chevron),
                    ),
                );

            let menu_w = w.max(136.0);
            egui::Popup::menu(&response)
                .id(id)
                .width(menu_w)
                .close_behavior(PopupCloseBehavior::CloseOnClickOutside)
                .align(RectAlign::BOTTOM_START)
                .align_alternatives(&[])
                .frame(crate::ui_kit::glass_menu(ui))
                .gap(theme.spacing.xs)
                .show(|ui| {
                    ui.set_width(menu_w);
                    ui.set_max_width(menu_w);
                    ui.spacing_mut().item_spacing.y = 2.0;
                    for choice in choices {
                        let selected = *end == choice;
                        let row_h = 30.0;
                        let (row, resp) =
                            ui.allocate_exact_size(Vec2::new(menu_w, row_h), Sense::click());
                        paint_menu_item_bg(ui, row, selected, resp.hovered());
                        let sample = Rect::from_min_max(
                            Pos2::new(row.min.x + 12.0, row.center().y - 5.0),
                            Pos2::new(row.min.x + 42.0, row.center().y + 5.0),
                        );
                        paint_end_sample(ui, sample, choice, theme.ui.text);
                        let label = l10n::text(ui.ctx(), arrow_label(choice));
                        ui.painter().text(
                            Pos2::new(row.min.x + 54.0, row.center().y),
                            egui::Align2::LEFT_CENTER,
                            &label,
                            FontId::proportional(theme.text.body),
                            if selected {
                                theme.ui.accent
                            } else {
                                theme.ui.text
                            },
                        );
                        resp.widget_info(|| {
                            WidgetInfo::selected(Role::Button, ui.is_enabled(), selected, &label)
                        });
                        if resp.clicked() {
                            *end = choice;
                            ui.close();
                        }
                    }
                });
        });
    });
}

fn paint_end_sample(ui: &Ui, rect: Rect, end: ArrowStyle, color: Color32) {
    let painter = ui.painter();
    let stroke = Stroke::new(1.75, color);
    let y = rect.center().y.round();
    let x0 = rect.min.x;
    let x1 = rect.max.x;
    painter.line_segment([Pos2::new(x0, y), Pos2::new(x1, y)], stroke);
    match end {
        ArrowStyle::None => {}
        ArrowStyle::Arrow | ArrowStyle::OpenArrow => {
            let head = 4.5;
            painter.line_segment([Pos2::new(x1, y), Pos2::new(x1 - head, y - head)], stroke);
            painter.line_segment([Pos2::new(x1, y), Pos2::new(x1 - head, y + head)], stroke);
        }
        ArrowStyle::Circle => {
            painter.circle_filled(Pos2::new(x1 - 3.0, y), 3.0, color);
        }
        ArrowStyle::Square | ArrowStyle::Diamond => {
            painter.rect_filled(
                Rect::from_center_size(Pos2::new(x1 - 3.0, y), Vec2::splat(5.0)),
                0.0,
                color,
            );
        }
    }
}

fn arrow_label(style: ArrowStyle) -> &'static str {
    match style {
        ArrowStyle::None => "Plain",
        ArrowStyle::Arrow => "Arrow",
        ArrowStyle::OpenArrow => "Open arrow",
        ArrowStyle::Circle => "Circle",
        ArrowStyle::Square => "Square",
        ArrowStyle::Diamond => "Diamond",
    }
}

fn paint_soft_selected(ui: &Ui, rect: egui::Rect, response: &egui::Response, selected: bool) {
    let theme = ui.active_theme();
    let radius = theme.radius.sm;
    if selected {
        // The neutral pressed fill the toolbars use; the accent stays reserved for selection.
        ui.painter()
            .rect_filled(rect, radius, theme.ui.btn_bg_active);
    } else if response.hovered() {
        ui.painter()
            .rect_filled(rect, radius, theme.ui.btn_bg_hover);
    }
    ui.painter().rect_stroke(
        rect,
        radius,
        Stroke::new(DESIGN_TOKENS.stroke.hairline, theme.ui.border_control),
        StrokeKind::Inside,
    );
    crate::styles::focus::draw_focus_ring(ui, response, radius);
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn fib_style_uses_only_functional_rows_and_a_dedicated_picker() {
        for tool in [
            DrawingToolType::FibonacciRetracement,
            DrawingToolType::FibonacciExtension,
            DrawingToolType::FibonacciChannel,
        ] {
            assert!(has_fib_style_controls(tool));
            assert!(!shows_generic_line_row(tool));
        }
        assert_ne!(LINE_PICKER, FIB_TREND_PICKER);
        assert_ne!(FILL_PICKER, FIB_TREND_PICKER);
        assert!(shows_generic_line_row(DrawingToolType::TrendLine));
        assert!(!shows_generic_line_row(DrawingToolType::Pitchfan));
        // Level tools take their colours from the levels; the spiral has none of its own.
        assert!(colours_from_levels(DrawingToolType::FibonacciCircles));
        assert!(colours_from_levels(DrawingToolType::GannFan));
        assert!(colours_from_levels(DrawingToolType::GannSquare));
        assert!(!colours_from_levels(DrawingToolType::FibonacciSpiral));
        assert!(!colours_from_levels(DrawingToolType::TrendLine));
    }
}
