//! Tab rendering implementations for SettingsDialog
//!
//! This module contains all the tab-specific UI rendering code,
//! keeping the main dialog module focused on core structure.

mod canvas;
mod scales;
mod symbol;
mod trading;

use super::SettingsDialog;
use crate::localization as l10n;
use crate::theme::Theme;
use crate::theme::ThemeExt;
use egui::{Color32, FontId, RichText, Ui};

impl SettingsDialog {
    pub(super) fn draw_section_header(&self, ui: &mut Ui, title: &str, theme: &Theme) {
        crate::ui_kit::section_header(ui, title, theme);
    }

    /// The width of the label column, which every tab shares: the longest label's width.
    fn label_width(ui: &Ui, theme: &Theme) -> f32 {
        [
            "Panel layout",
            "Extended-session price",
            "Background",
            "Grid lines",
            "Crosshair",
            "Mode",
            "Text",
            "Lines",
            "Top margin (%)",
            "Bottom margin (%)",
            "Right margin (bars)",
            "Zoom smoothing",
            "Visibility",
            "Timezone",
        ]
        .iter()
        .map(|label| {
            ui.painter()
                .layout_no_wrap(
                    l10n::text(ui.ctx(), *label),
                    FontId::proportional(theme.text.title),
                    theme.ui.text,
                )
                .size()
                .x
        })
        .fold(0.0_f32, f32::max)
    }

    pub(super) fn draw_setting_row_with_controls<F>(
        &mut self,
        ui: &mut Ui,
        label: &str,
        theme: &Theme,
        add_content: F,
    ) where
        F: FnOnce(&mut Ui, &mut SettingsDialog),
    {
        let label_width = Self::label_width(ui, theme);
        crate::ui_kit::setting_row(ui, label, label_width, theme, |ui| add_content(ui, self));
    }
}

fn price_line_dropdown(ui: &mut Ui, id: &str, color: Color32, label: &mut bool, line: &mut bool) {
    let mut mode = (*label, *line);
    let name = |mode| match mode {
        (true, true) => "Value and line",
        (true, false) => "Value",
        (false, true) => "Line",
        _ => "Hidden",
    };
    styled_dropdown(ui, id, 180.0, color, name(mode), |ui| {
        for option in [(true, true), (true, false), (false, true), (false, false)] {
            ui.selectable_value(&mut mode, option, l10n::text(ui.ctx(), name(option)));
        }
    });
    (*label, *line) = mode;
}

fn styled_dropdown<R>(
    ui: &mut Ui,
    id: &str,
    width: f32,
    text_color: Color32,
    selected: &str,
    add_contents: impl FnOnce(&mut Ui) -> R,
) {
    crate::ui_kit::styled_dropdown(ui, id, width, text_color, selected, add_contents);
}

/// The width of the Trading tab's labels, where its dropdowns start.
const TRADING_LABEL_WIDTH: f32 = 264.0;
/// How far the Trading tab's sub-options sit right of their group's checkbox.
const TRADING_INDENT: f32 = 32.0;

/// A Trading tab row: a checkbox `indent` from the left and, from
/// [`TRADING_LABEL_WIDTH`], the controls `add_controls` adds.
fn trading_option(
    ui: &mut Ui,
    indent: f32,
    checked: &mut bool,
    label: &str,
    text_color: Color32,
    add_controls: impl FnOnce(&mut Ui),
) {
    ui.horizontal(|ui| {
        ui.set_min_height(42.0);
        ui.add_space(indent);
        ui.checkbox(
            checked,
            RichText::new(l10n::text(ui.ctx(), label))
                .size(ui.active_theme().text.title)
                .color(text_color),
        );
        ui.add_space((TRADING_LABEL_WIDTH - ui.min_rect().width()).max(0.0));
        add_controls(ui);
    });
    ui.add_space(ui.active_theme().spacing.sm);
}

fn styled_checkbox(ui: &mut Ui, checked: &mut bool, label: &str, text_color: Color32) -> bool {
    crate::ui_kit::styled_checkbox(ui, checked, label, text_color)
}
