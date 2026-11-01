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

/// Standard control height for Drawing Properties rows (28px).
pub(super) fn control_row_height(ui: &Ui) -> f32 {
    28.0_f32.max(ui.spacing().interact_size.y.min(28.0))
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

/// Style / Appearance section renderer.
pub struct StyleTab;

impl StyleTab {
    /// Dense Appearance rows: color, width, style; optional extend and ends on aligned rows.
    pub fn show(ui: &mut Ui, props: &mut DrawingProps, pickers: &mut [ColorPickerState; 2]) {
        let tool = props.tool_settings.as_ref().map(|t| t.settings.tool_type);
        let [color_picker, _] = pickers;
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
                        if let Some(color) =
                            color_swatch(ui, color_picker, props.color, "drawing_props_color", true)
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
        if show_extend {
            extend_control(ui, &mut props.extend_left, &mut props.extend_right, col_w);
        }
        if show_ends && let Some(template) = &mut props.tool_settings {
            ends_combo(ui, &mut template.settings.arrow_end_style, ends_combo_w);
        }

        if let Some(template) = &mut props.tool_settings
            && template.settings.tool_type == DrawingToolType::Rect
        {
            ui.checkbox(
                &mut template.settings.channel_midline,
                egui::RichText::new(l10n::text(ui.ctx(), "Show middle line"))
                    .size(theme.text.body),
            );
        }
    }

    /// Fill section body when the tool paints a fill.
    pub fn show_fill(ui: &mut Ui, props: &mut DrawingProps, pickers: &mut [ColorPickerState; 2]) {
        let [_, fill_picker] = pickers;
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
                let fill = props
                    .fill_color
                    .unwrap_or(theme.ui.accent.gamma_multiply(0.2));
                props.fill_color = Some(fill);
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    if let Some(picked) =
                        color_swatch(ui, fill_picker, fill, "drawing_props_fill", true)
                    {
                        props.fill_color = Some(picked);
                    }
                });
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
    let theme = ui.active_theme();
    let label = format!("{}px", (*width).round() as i32);
    let (rect, response) = ui.allocate_exact_size(Vec2::new(chip_w, chip_h), Sense::click());
    let a11y_label = l10n::text(ui.ctx(), "Line width");
    response.widget_info(|| WidgetInfo::labeled(Role::ComboBox, ui.is_enabled(), &a11y_label));

    let id = ui.make_persistent_id("drawing_props_line_width");
    let is_open = egui::Popup::is_id_open(ui.ctx(), id);
    paint_control_box(ui, rect, &response, is_open);

    let chevron = DESIGN_TOKENS.sizing.icon_sm;
    let pad_x = theme.spacing.md;
    let galley = ui.painter().layout_no_wrap(
        label,
        FontId::proportional(theme.text.body),
        theme.ui.text,
    );
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
                let (row, resp) =
                    ui.allocate_exact_size(Vec2::new(menu_w, row_h), Sense::click());
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
                let (row, resp) =
                    ui.allocate_exact_size(Vec2::new(menu_w, row_h), Sense::click());
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
    let fill = if is_open {
        theme.ui.btn_bg_hover
    } else if response.hovered() {
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
        Pos2::new(
            rect.min.x + pad_x,
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
                let (row, resp) =
                    ui.allocate_exact_size(Vec2::new(menu_w, row_h), Sense::click());
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
            let (rect, response) =
                ui.allocate_exact_size(Vec2::new(w, control_h), Sense::click());
            let a11y_label = l10n::text(ui.ctx(), "Line end");
            response.widget_info(|| {
                WidgetInfo::labeled(Role::ComboBox, ui.is_enabled(), &a11y_label)
            });

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
