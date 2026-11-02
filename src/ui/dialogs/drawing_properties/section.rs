//! Collapsible property sections for the drawing properties dialog.

use crate::icons::icons;
use crate::localization as l10n;
use crate::theme::ThemeExt;
use crate::tokens::DESIGN_TOKENS;
use egui::{Align, Layout, Pos2, Rect, Role, Sense, Stroke, Ui, Vec2, WidgetInfo};

/// Thin rule used between property sections.
pub(super) fn section_divider(ui: &mut Ui) {
    let theme = ui.active_theme();
    ui.add_space(2.0);
    let y = ui.cursor().top() + 1.0;
    let rect = ui.available_rect_before_wrap();
    ui.painter().hline(
        rect.x_range(),
        y,
        Stroke::new(DESIGN_TOKENS.stroke.hairline, theme.ui.border),
    );
    ui.add_space(4.0 + DESIGN_TOKENS.stroke.hairline);
}

/// Collapsible section: clickable title header with a chevron indicator, followed by dense
/// controls when expanded.
///
/// `scroll_into_view` scrolls the header into view when a host deep-links via `active_tab`.
pub(super) fn properties_section(
    ui: &mut Ui,
    id_salt: &'static str,
    title: &str,
    is_open: &mut bool,
    scroll_into_view: bool,
    add_body: impl FnOnce(&mut Ui),
) {
    let _ = id_salt;
    let theme = ui.active_theme();

    if scroll_into_view {
        ui.scroll_to_cursor(Some(Align::TOP));
    }

    let title_text = l10n::text(ui.ctx(), title);
    let header_h = 26.0;
    let full_w = ui.available_width();
    let (header_rect, response) =
        ui.allocate_exact_size(Vec2::new(full_w, header_h), Sense::click());
    response.widget_info(|| {
        WidgetInfo::selected(Role::Button, ui.is_enabled(), *is_open, &title_text)
    });
    if response.clicked() {
        *is_open = !*is_open;
    }
    if response.hovered() {
        ui.painter().rect_filled(
            header_rect.expand2(Vec2::new(4.0, 0.0)),
            theme.radius.sm,
            theme.ui.btn_bg_hover,
        );
    }

    let galley = ui.painter().layout_no_wrap(
        title_text,
        egui::FontId::proportional(theme.text.body),
        theme.ui.text,
    );
    ui.painter().galley(
        Pos2::new(
            header_rect.min.x,
            (header_rect.center().y - galley.size().y * 0.5).round(),
        ),
        galley,
        theme.ui.text,
    );

    let chevron_size = Vec2::splat(DESIGN_TOKENS.sizing.icon_sm);
    let chevron_rect = Rect::from_center_size(
        Pos2::new(
            header_rect.max.x - chevron_size.x * 0.5 - 2.0,
            header_rect.center().y,
        ),
        chevron_size,
    );
    let chevron_icon = if *is_open {
        &icons::CHEVRON_DOWN
    } else {
        &icons::CHEVRON_RIGHT
    };
    let chevron_color = if response.hovered() {
        theme.ui.text
    } else {
        theme.ui.text_muted
    };
    chevron_icon
        .as_image_tinted(chevron_size, chevron_color)
        .paint_at(ui, chevron_rect);

    if *is_open {
        ui.add_space(6.0);
        let body_width = ui.available_width();
        ui.allocate_ui_with_layout(
            Vec2::new(body_width, 0.0),
            Layout::top_down(Align::Min),
            |ui| {
                ui.set_width(body_width);
                ui.set_max_width(body_width);
                ui.spacing_mut().item_spacing.y = 6.0;
                add_body(ui);
            },
        );
        ui.add_space(4.0);
    }
}
