//! Shared label-and-control rows used by the chart and indicator settings dialogs.

use crate::localization as l10n;
use crate::model::LineStyle;
use crate::theme::Theme;
use crate::theme::ThemeExt;
use egui::{Color32, FontId, RichText, Ui};

/// A small uppercase section title.
pub fn section_header(ui: &mut Ui, title: &str, theme: &Theme) {
    ui.label(
        RichText::new(l10n::text(ui.ctx(), title))
            .size(theme.text.small)
            .strong()
            .color(theme.ui.text_secondary),
    );
    ui.add_space(ui.active_theme().spacing.sm);
}

/// The widest translated label, capped at `max`.
pub fn label_width(
    ui: &Ui,
    theme: &Theme,
    labels: impl IntoIterator<Item = impl AsRef<str>>,
    max: f32,
) -> f32 {
    labels
        .into_iter()
        .map(|label| {
            ui.painter()
                .layout_no_wrap(
                    l10n::text(ui.ctx(), label.as_ref()),
                    FontId::proportional(theme.text.title),
                    theme.ui.text,
                )
                .size()
                .x
        })
        .fold(0.0_f32, f32::max)
        .min(max)
}

/// A fixed label column and the controls `add` draws beside it.
pub fn setting_row(
    ui: &mut Ui,
    label: &str,
    label_width: f32,
    theme: &Theme,
    add: impl FnOnce(&mut Ui),
) {
    ui.horizontal(|ui| {
        let label = ui.allocate_ui_with_layout(
            egui::vec2(label_width, 42.0),
            egui::Layout::left_to_right(egui::Align::Center),
            |ui| {
                ui.set_min_width(label_width);
                ui.set_max_width(label_width);
                ui.add(
                    egui::Label::new(
                        RichText::new(l10n::text(ui.ctx(), label))
                            .size(theme.text.title)
                            .color(theme.ui.text),
                    )
                    .wrap(),
                )
            },
        );
        ui.horizontal(|ui| {
            ui.spacing_mut().item_spacing.x = 8.0;
            ui.spacing_mut().interact_size.y = 32.0;
            ui.label_inputs_by(label.inner.id, add);
        });
    });
    ui.add_space(ui.active_theme().spacing.sm);
}

/// A dropdown whose selected text uses the dialog's text size and colour.
pub fn styled_dropdown<R>(
    ui: &mut Ui,
    id: &str,
    width: f32,
    text_color: Color32,
    selected: &str,
    add_contents: impl FnOnce(&mut Ui) -> R,
) {
    egui::ComboBox::from_id_salt(id)
        .icon(crate::ui_kit::buttons::combo_chevron)
        .width(width)
        .selected_text(
            RichText::new(l10n::text(ui.ctx(), selected))
                .size(ui.active_theme().text.title)
                .color(text_color),
        )
        .show_ui(ui, add_contents);
}

/// A full-width checkbox row. Returns whether it changed.
pub fn styled_checkbox(ui: &mut Ui, checked: &mut bool, label: &str, text_color: Color32) -> bool {
    let changed = ui
        .allocate_ui_with_layout(
            egui::vec2(ui.available_width(), 42.0),
            egui::Layout::left_to_right(egui::Align::Center),
            |ui| {
                ui.set_min_height(42.0);
                ui.checkbox(
                    checked,
                    RichText::new(l10n::text(ui.ctx(), label))
                        .size(ui.active_theme().text.title)
                        .color(text_color),
                )
                .changed()
            },
        )
        .inner;
    ui.add_space(ui.active_theme().spacing.sm);
    changed
}

/// The menu label of a line pattern, with a drawn sample for the broken styles.
pub fn line_style_preview(style: LineStyle) -> &'static str {
    match style {
        LineStyle::Solid => "Solid",
        LineStyle::Dashed => "\u{2013} \u{2013} \u{2013}",
        LineStyle::Dotted => "· · · ·",
        LineStyle::LargeDashed => "\u{2014}  \u{2014}  \u{2014}",
        LineStyle::SparseDotted => "·   ·   ·",
    }
}
