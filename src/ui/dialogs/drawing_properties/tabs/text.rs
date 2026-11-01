//! Label controls for drawings that support editable text.

use crate::config::FontWeight;
use crate::localization as l10n;
use crate::styles::typography::FONT_SIZES as VALID_FONT_SIZES;
use crate::theme::ThemeExt;
use crate::ui::dialogs::drawing_properties::actions::DrawingProps;
use crate::ui_kit::buttons::{Button, combo_chevron};
use crate::ui_kit::form_control_width_for;
use egui::{TextEdit, TextStyle, Ui};

/// Font sizes offered in the Label section (subset of valid drawing sizes).
const FONT_SIZES: &[f32] = &[8.0, 10.0, 12.0, 14.0, 16.0, 18.0, 20.0, 24.0, 28.0, 32.0];

/// Text / Label section renderer.
pub struct TextTab;

impl TextTab {
    pub fn show(ui: &mut Ui, props: &mut DrawingProps) {
        let control_width = form_control_width_for(ui).min(112.0);
        let theme = ui.active_theme();
        let body = TextStyle::Body.resolve(ui.style());

        ui.add(
            TextEdit::multiline(&mut props.label)
                .font(body.clone())
                .desired_width(ui.available_width())
                .desired_rows(2)
                .hint_text(
                    egui::RichText::new(l10n::text(ui.ctx(), "Add a label"))
                        .size(theme.text.body)
                        .color(theme.ui.text_muted),
                ),
        )
        .accessible_name(l10n::text(ui.ctx(), "Label"));

        ui.add_space(theme.spacing.sm);
        ui.horizontal(|ui| {
            ui.spacing_mut().item_spacing.x = theme.spacing.sm;

            let selected_text = if (props.font_size - 12.0).abs() < f32::EPSILON {
                l10n::text(ui.ctx(), "Normal")
            } else {
                format!("{}px", props.font_size as i32)
            };
            let font_response = egui::ComboBox::from_id_salt("font_size")
                .icon(combo_chevron)
                .selected_text(egui::RichText::new(selected_text).size(theme.text.body))
                .width(control_width)
                .show_ui(ui, |ui| {
                    for &size in FONT_SIZES {
                        if !VALID_FONT_SIZES.contains(&size) {
                            continue;
                        }
                        let label = if (size - 12.0).abs() < f32::EPSILON {
                            l10n::text(ui.ctx(), "Normal")
                        } else {
                            format!("{}px", size as i32)
                        };
                        ui.selectable_value(&mut props.font_size, size, label);
                    }
                })
                .response;
            font_response.accessible_name(l10n::text(ui.ctx(), "Font size"));

            if let Some(template) = &mut props.tool_settings {
                let bold = template.settings.font_weight == FontWeight::Bold;
                if Button::new("B")
                    .ghost()
                    .small()
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
                // Italic is not persisted on drawings yet; keep the control for layout parity.
                let _ = Button::new("I")
                    .ghost()
                    .small()
                    .enabled(false)
                    .tooltip("Italic is not available")
                    .show(ui);
            }
        });
    }
}
