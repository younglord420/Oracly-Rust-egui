//! Text tab - labels and font settings.

use egui::Ui;

use crate::ui::dialogs::drawing_properties::actions::DrawingProps;
use crate::ui_kit::ext::UiExt;
use crate::ui_kit::{FormGrid, form_control_width_for, form_label};

/// Font sizes
const FONT_SIZES: &[f32] = &[8.0, 10.0, 12.0, 14.0, 16.0, 18.0, 20.0, 24.0, 28.0, 32.0];

/// Text tab renderer
pub struct TextTab;

impl TextTab {
    pub fn show(ui: &mut Ui, props: &mut DrawingProps) {
        let control_width = form_control_width_for(ui);
        ui.space_md();

        FormGrid::new("text_grid").show(ui, |ui| {
            let label = form_label(ui, "Label");
            ui.add(
                egui::TextEdit::multiline(&mut props.label)
                    .desired_width(control_width * 2.0)
                    .desired_rows(3),
            )
            .labelled_by(label.id);
            ui.end_row();

            let label = form_label(ui, "Font size");
            ui.label_inputs_by(label.id, |ui| {
                ui.combo_select_width(
                    "font_size",
                    &mut props.font_size,
                    FONT_SIZES.iter().copied(),
                    |v| format!("{}px", *v as i32),
                    control_width,
                );
            });
            ui.end_row();
        });
    }
}
