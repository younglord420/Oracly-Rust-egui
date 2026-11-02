//! Label and text formatting controls for drawings.
//!
//! Tools that accept free-form text show a multiline label editor and a single toolbar row
//! combining text color, size (`Small`, `Normal`, `Large`, `Huge`), `Bold`, `Italic`, and
//! (for line/shape labels) vertical (`Top`, `Inside`, `Middle`, `Bottom`) and horizontal (`Left`,
//! `Center`, `Right`) placement. Tools whose text is generated automatically still expose
//! the formatting row.

use crate::config::FontWeight;
use crate::drawings::{TextHorizontalAlign, TextVerticalAlign};
use crate::localization as l10n;
use crate::styles::typography::FONT_SIZES as VALID_FONT_SIZES;
use crate::theme::ThemeExt;
use crate::ui::dialogs::drawing_properties::actions::DrawingProps;
use crate::ui_kit::buttons::Button;
use crate::ui_kit::color_picker::ColorPickerState;
use egui::{Color32, TextEdit, Ui};

/// Named font size presets offered in the Text / Label section.
const NAMED_FONT_SIZES: &[(f32, &str)] = &[
    (10.0, "Small"),
    (12.0, "Normal"),
    (16.0, "Large"),
    (24.0, "Huge"),
];

fn font_size_label(ctx: &egui::Context, size: f32) -> String {
    for &(preset, name) in NAMED_FONT_SIZES {
        if (size - preset).abs() < f32::EPSILON {
            return l10n::text(ctx, name);
        }
    }
    format!("{}px", size.round() as i32)
}

/// Text / Label section renderer.
pub struct TextTab;

impl TextTab {
    pub fn show(ui: &mut Ui, props: &mut DrawingProps, text_picker: &mut ColorPickerState) {
        let theme = ui.active_theme().clone();
        let control_h = super::style::control_row_height(ui);
        ui.spacing_mut().interact_size.y = control_h;
        let font = egui::FontId::proportional(theme.text.body);
        let inner_width = (ui.available_width() - 16.0).max(64.0);

        let tool = props.tool_settings.as_ref().map(|t| t.settings.tool_type);
        let has_editable_text = tool.is_none_or(|t| t.supports_text());
        let has_line_alignment = tool.is_some_and(|t| t.supports_line_label());

        if has_editable_text {
            ui.scope(|ui| {
                let visuals = ui.visuals_mut();
                visuals.extreme_bg_color = theme.ui.panel_bg;
                visuals.widgets.inactive.bg_stroke = egui::Stroke::new(
                    crate::tokens::DESIGN_TOKENS.stroke.hairline,
                    theme.ui.border,
                );
                visuals.widgets.hovered.bg_stroke = egui::Stroke::new(
                    crate::tokens::DESIGN_TOKENS.stroke.hairline,
                    theme.ui.border_control,
                );
                visuals.widgets.active.bg_stroke =
                    egui::Stroke::new(crate::tokens::DESIGN_TOKENS.stroke.hairline, theme.ui.text);
                ui.add(
                    TextEdit::multiline(&mut props.label)
                        .font(font.clone())
                        .desired_width(inner_width)
                        .desired_rows(2)
                        .margin(egui::Margin::symmetric(8, 6))
                        .hint_text(
                            egui::RichText::new(l10n::text(ui.ctx(), "Add a label"))
                                .size(theme.text.body)
                                .color(theme.ui.text_muted),
                        ),
                )
                .accessible_name(l10n::text(ui.ctx(), "Label"));
            });
        }

        // Single unified formatting row:
        // [Color] [Size (Small/Normal/Large/Huge)] [B] [I] [Top/Middle/Bottom] [Left/Center/Right]
        ui.horizontal(|ui| {
            ui.set_min_height(control_h);
            let gap = if has_line_alignment {
                theme.spacing.xs
            } else {
                6.0
            };
            ui.spacing_mut().item_spacing.x = gap;
            let row_w = ui.available_width();
            let size_w = if has_line_alignment {
                (row_w * 0.24).clamp(58.0, 76.0)
            } else {
                (row_w - 28.0 - 60.0 - gap * 3.0).clamp(84.0, 128.0)
            };
            let align_w = ((row_w - 28.0 - size_w - 58.0 - gap * 5.0) * 0.5).clamp(48.0, 76.0);

            if let Some(template) = &mut props.tool_settings {
                let current_text_color = template
                    .settings
                    .text_color
                    .map(|c| Color32::from_rgba_unmultiplied(c[0], c[1], c[2], c[3]))
                    .unwrap_or(props.color);
                if let Some(picked) = super::style::color_swatch(
                    ui,
                    text_picker,
                    current_text_color,
                    "drawing_props_text_color",
                    true,
                ) {
                    template.settings.text_color = Some(picked.to_srgba_unmultiplied());
                }
            }

            let mut choices: Vec<f32> = NAMED_FONT_SIZES
                .iter()
                .map(|&(size, _)| size)
                .filter(|s| VALID_FONT_SIZES.contains(s))
                .collect();
            if !choices
                .iter()
                .any(|s| (*s - props.font_size).abs() < f32::EPSILON)
            {
                choices.push(props.font_size);
            }
            let ctx = ui.ctx().clone();
            super::style::text_combo_popup(
                ui,
                "font_size",
                "Font size",
                &mut props.font_size,
                &choices,
                |size| font_size_label(&ctx, size),
                size_w,
                control_h,
            );

            if let Some(template) = &mut props.tool_settings {
                let bold = template.settings.font_weight == FontWeight::Bold;
                if Button::new("B")
                    .ghost()
                    .selected(bold)
                    .tooltip("Bold")
                    .show(ui)
                    .clicked()
                {
                    template.settings.font_weight = if bold {
                        FontWeight::Normal
                    } else {
                        FontWeight::Bold
                    };
                }

                let italic = template.settings.font_italic;
                if Button::new("I")
                    .ghost()
                    .selected(italic)
                    .tooltip("Italic")
                    .show(ui)
                    .clicked()
                {
                    template.settings.font_italic = !italic;
                }

                if has_line_alignment {
                    let v_choices = if matches!(
                        tool,
                        Some(
                            crate::drawings::DrawingToolType::FlatTopBottom
                                | crate::drawings::DrawingToolType::DisjointChannel
                        )
                    ) {
                        TextVerticalAlign::channel()
                    } else {
                        TextVerticalAlign::all()
                    };
                    if !v_choices.contains(&template.settings.text_v_align) {
                        template.settings.text_v_align = TextVerticalAlign::Inside;
                    }
                    super::style::text_combo_popup(
                        ui,
                        "text_v_align",
                        "Vertical text alignment",
                        &mut template.settings.text_v_align,
                        v_choices,
                        |align| l10n::text(&ctx, align.label()),
                        align_w,
                        control_h,
                    );
                    super::style::text_combo_popup(
                        ui,
                        "text_h_align",
                        "Horizontal text alignment",
                        &mut template.settings.text_h_align,
                        TextHorizontalAlign::all(),
                        |align| l10n::text(&ctx, align.label()),
                        align_w,
                        control_h,
                    );
                }
            }
        });
    }
}
