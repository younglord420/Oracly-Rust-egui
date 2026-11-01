//! Appearance controls for the drawing properties panel.

use crate::drawings::{ArrowStyle, DrawingToolType};
use crate::localization as l10n;
use crate::model::LineStyle;
use crate::theme::ThemeExt;
use crate::tokens::DESIGN_TOKENS;
use crate::ui::dialogs::drawing_properties::actions::DrawingProps;
use crate::ui_kit::buttons::{ButtonSize, combo_chevron};
use crate::ui_kit::{ColorPicker, ColorPickerState};
use egui::{Color32, Role, Sense, Stroke, StrokeKind, Ui, Vec2, WidgetInfo};

/// Line widths offered in the Appearance width combo.
const LINE_WIDTHS: [f32; 4] = [1.0, 2.0, 3.0, 4.0];

/// Primary line styles shown in the Appearance style combo.
const LINE_STYLES: [LineStyle; 3] = [LineStyle::Solid, LineStyle::Dashed, LineStyle::Dotted];

/// Compact color well for dense Appearance rows.
fn appearance_swatch_size(ui: &Ui) -> f32 {
    DESIGN_TOKENS
        .sizing
        .control_height
        .max(ButtonSize::MD.pixels())
        .max(ui.spacing().interact_size.y)
}

/// Style / Appearance section renderer.
pub struct StyleTab;

impl StyleTab {
    /// Dense Appearance row: color, width, style; optional extend + ends on a second row.
    pub fn show(ui: &mut Ui, props: &mut DrawingProps, pickers: &mut [ColorPickerState; 2]) {
        let tool = props.tool_settings.as_ref().map(|t| t.settings.tool_type);
        let [color_picker, _] = pickers;
        let theme = ui.active_theme();
        let gap = theme.spacing.sm;
        let control_h = appearance_swatch_size(ui);
        ui.spacing_mut().interact_size.y = control_h;
        let combo_w = ((ui.available_width() - control_h - gap * 2.0) / 2.0).clamp(
            DESIGN_TOKENS.sizing.dialog.button_min_width_xs,
            DESIGN_TOKENS.sizing.settings_dialog.small_dropdown_width
                + DESIGN_TOKENS.sizing.button_sm,
        );

        ui.horizontal_wrapped(|ui| {
            ui.spacing_mut().item_spacing.x = gap;
            if let Some(color) =
                color_swatch(ui, color_picker, props.color, "drawing_props_color", true)
            {
                props.color = color;
            }
            line_width_combo(ui, &mut props.line_width, combo_w);
            line_style_combo(ui, &mut props.line_style, combo_w);
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
        if show_extend || show_ends {
            ui.add_space(theme.spacing.xs);
            ui.horizontal_wrapped(|ui| {
                ui.spacing_mut().item_spacing.x = gap;
                if show_extend {
                    extend_control(ui, &mut props.extend_left, &mut props.extend_right);
                }
                if show_ends && let Some(template) = &mut props.tool_settings {
                    ends_combo(ui, &mut template.settings.arrow_end_style);
                }
            });
        }

        if let Some(template) = &mut props.tool_settings
            && template.settings.tool_type == DrawingToolType::Rect
        {
            ui.add_space(theme.spacing.sm);
            ui.checkbox(
                &mut template.settings.channel_midline,
                l10n::text(ui.ctx(), "Show middle line"),
            );
        }
    }

    /// Fill section body when the tool paints a fill.
    pub fn show_fill(ui: &mut Ui, props: &mut DrawingProps, pickers: &mut [ColorPickerState; 2]) {
        let [_, fill_picker] = pickers;
        let theme = ui.active_theme();
        ui.horizontal(|ui| {
            ui.spacing_mut().item_spacing.x = theme.spacing.sm;
            ui.checkbox(&mut props.fill_enabled, l10n::text(ui.ctx(), "Enable fill"));
            if props.fill_enabled {
                let fill = props
                    .fill_color
                    .unwrap_or(theme.ui.accent.gamma_multiply(0.2));
                props.fill_color = Some(fill);
                if let Some(picked) =
                    color_swatch(ui, fill_picker, fill, "drawing_props_fill", true)
                {
                    let [r, g, b, a] = picked.to_srgba_unmultiplied();
                    let a = if fill_picker.custom_mode { a } else { fill.a() };
                    props.fill_color = Some(Color32::from_rgba_unmultiplied(r, g, b, a));
                }
            }
        });
    }
}

fn color_swatch(
    ui: &mut Ui,
    state: &mut ColorPickerState,
    color: Color32,
    id: &str,
    compact: bool,
) -> Option<Color32> {
    state.sel_color = color;
    let mut picker = ColorPicker::new(state, id);
    if compact {
        picker = picker.well_size(appearance_swatch_size(ui));
    }
    picker.show(ui).inner
}

fn line_width_combo(ui: &mut Ui, width: &mut f32, combo_w: f32) {
    let theme = ui.active_theme();
    let selected = format!("{}px", *width as i32);
    let response = egui::ComboBox::from_id_salt("drawing_props_line_width")
        .icon(combo_chevron)
        .selected_text(egui::RichText::new(selected).size(theme.text.body))
        .width(combo_w)
        .show_ui(ui, |ui| {
            for preset in LINE_WIDTHS {
                let label = format!("{}px", preset as i32);
                ui.selectable_value(width, preset, label);
            }
            // Keep a non-preset width selectable so custom strokes stay editable.
            if !LINE_WIDTHS
                .iter()
                .any(|w| (*width - w).abs() < f32::EPSILON)
            {
                let label = format!("{}px", *width as i32);
                ui.selectable_value(width, *width, label);
            }
        })
        .response;
    response.accessible_name(l10n::text(ui.ctx(), "Line width"));
}

fn line_style_combo(ui: &mut Ui, style: &mut LineStyle, combo_w: f32) {
    let theme = ui.active_theme();
    let mut choices: Vec<LineStyle> = LINE_STYLES.to_vec();
    if !choices.contains(style) {
        choices.push(*style);
    }
    let selected = l10n::text(ui.ctx(), style.name());
    let response = egui::ComboBox::from_id_salt("drawing_props_line_style")
        .icon(combo_chevron)
        .selected_text(egui::RichText::new(selected).size(theme.text.body))
        .width(combo_w)
        .show_ui(ui, |ui| {
            for choice in choices {
                ui.selectable_value(style, choice, l10n::text(ui.ctx(), choice.name()));
            }
        })
        .response;
    response.accessible_name(l10n::text(ui.ctx(), "Line style"));
}

/// Compact extend icons with soft selection — not a blue segment strip.
fn extend_control(ui: &mut Ui, left: &mut bool, right: &mut bool) {
    let theme = ui.active_theme();
    let options = [
        (false, false, "None"),
        (true, false, "Left"),
        (false, true, "Right"),
        (true, true, "Both"),
    ];
    let size = Vec2::splat(appearance_swatch_size(ui));
    // Quiet inline caption, matching the Ends control on the same row.
    ui.label(
        egui::RichText::new(l10n::text(ui.ctx(), "Extend"))
            .size(theme.text.small)
            .color(theme.ui.text_muted),
    );
    ui.horizontal(|ui| {
        ui.spacing_mut().item_spacing.x = theme.spacing.xs;
        for (l, r, name) in options {
            let selected = *left == l && *right == r;
            let (rect, response) = ui.allocate_exact_size(size, Sense::click());
            if response.clicked() {
                *left = l;
                *right = r;
            }
            let label = l10n::format_text(ui.ctx(), "Extend {name}", &[("name", name.to_owned())]);
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
    })
    .response
    .accessible_name(l10n::text(ui.ctx(), "Extend"));
}

/// The extend directions as painted vectors.
///
/// The interface font a lean host installs carries no arrow glyphs, so these must not be text.
fn paint_extend_arrow(ui: &Ui, rect: egui::Rect, left: bool, right: bool, color: Color32) {
    let painter = ui.painter();
    let stroke = Stroke::new(DESIGN_TOKENS.stroke.hairline * 1.5, color);
    let y = rect.center().y.round();
    let half = (rect.width() * 0.3).round();
    let (x0, x1) = (rect.center().x - half, rect.center().x + half);
    painter.line_segment([egui::pos2(x0, y), egui::pos2(x1, y)], stroke);

    let head = (rect.width() * 0.17).round();
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

fn ends_combo(ui: &mut Ui, end: &mut ArrowStyle) {
    let theme = ui.active_theme();
    let mut choices = vec![ArrowStyle::None, ArrowStyle::Arrow];
    if !choices.contains(end) {
        choices.push(*end);
    }
    // Quiet inline caption — not a 96px gutter label.
    ui.label(
        egui::RichText::new(l10n::text(ui.ctx(), "Ends"))
            .size(theme.text.small)
            .color(theme.ui.text_muted),
    );
    let selected = arrow_label(*end);
    let response = egui::ComboBox::from_id_salt("drawing_props_line_ends")
        .icon(combo_chevron)
        .selected_text(egui::RichText::new(l10n::text(ui.ctx(), selected)).size(theme.text.body))
        .width(
            DESIGN_TOKENS.sizing.dialog.button_min_width_xs + DESIGN_TOKENS.sizing.button_sm * 1.5,
        )
        .show_ui(ui, |ui| {
            for choice in choices {
                ui.selectable_value(end, choice, l10n::text(ui.ctx(), arrow_label(choice)));
            }
        })
        .response;
    response.accessible_name(l10n::text(ui.ctx(), "Line end"));
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
