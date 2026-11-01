//! Always-open property sections for the drawing properties dialog.

use crate::localization as l10n;
use crate::theme::ThemeExt;
use crate::tokens::DESIGN_TOKENS;
use egui::{Align, Layout, RichText, Stroke, Ui, Vec2};

/// Thin rule used between property sections.
pub(super) fn section_divider(ui: &mut Ui) {
    let theme = ui.active_theme();
    let y = ui.cursor().top() + theme.spacing.xs;
    let rect = ui.available_rect_before_wrap();
    ui.painter().hline(
        rect.x_range(),
        y,
        Stroke::new(DESIGN_TOKENS.stroke.hairline, theme.ui.border),
    );
    ui.add_space(theme.spacing.xs + DESIGN_TOKENS.stroke.hairline + theme.spacing.xs);
}

/// Always-open calm section: quiet title, then dense controls.
///
/// `scroll_into_view` scrolls the title into view when a host deep-links via `active_tab`.
pub(super) fn properties_section(
    ui: &mut Ui,
    id_salt: impl std::hash::Hash + std::fmt::Debug,
    title: &str,
    scroll_into_view: bool,
    add_body: impl FnOnce(&mut Ui),
) {
    let _ = id_salt;
    let theme = ui.active_theme();

    if scroll_into_view {
        ui.scroll_to_cursor(Some(Align::TOP));
    }

    let title_text = l10n::text(ui.ctx(), title);
    ui.label(
        RichText::new(&title_text)
            .size(theme.text.small)
            .color(theme.ui.text_secondary)
            .strong(),
    );

    ui.add_space(theme.spacing.xs);
    let body_width = ui.available_width();
    ui.allocate_ui_with_layout(
        Vec2::new(body_width, 0.0),
        Layout::top_down(Align::Min),
        |ui| {
            ui.set_width(body_width);
            ui.set_max_width(body_width);
            ui.spacing_mut().item_spacing.y = theme.spacing.sm;
            add_body(ui);
        },
    );
    ui.add_space(theme.spacing.xs);
}
