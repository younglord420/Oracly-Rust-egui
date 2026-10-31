//! Drawing colour button that opens the toolbar colour popup.

use crate::localization as l10n;
use crate::theme::ThemeExt;
use crate::tokens::DESIGN_TOKENS;
use egui::{Color32, Role, Sense, Stroke, StrokeKind, Ui, Vec2, WidgetInfo};

/// Opens the drawing colour popup from a visible swatch control.
pub struct ColorButton;

impl ColorButton {
    /// Draw the current drawing colour as a swatch. Returns true when clicked so the host
    /// toolbar can open its colour popup.
    pub fn show(ui: &mut Ui, color: [u8; 4], open: bool) -> bool {
        let tokens = &DESIGN_TOKENS.sizing.toolbar;
        let (rect, response) = ui.allocate_exact_size(
            Vec2::new(tokens.button_width, tokens.button_height),
            Sense::click(),
        );
        crate::styles::focus::draw_focus_ring(ui, &response, ui.active_theme().radius.sm);

        let bg = if open || response.hovered() {
            ui.visuals().widgets.hovered.bg_fill
        } else {
            Color32::TRANSPARENT
        };
        ui.painter()
            .rect_filled(rect, ui.active_theme().radius.sm, bg);

        let swatch = rect.shrink(tokens.button_width * 0.28);
        let fill = Color32::from_rgba_unmultiplied(color[0], color[1], color[2], color[3]);
        ui.painter()
            .rect_filled(swatch, ui.active_theme().radius.sm, fill);
        ui.painter().rect_stroke(
            swatch,
            ui.active_theme().radius.sm,
            Stroke::new(DESIGN_TOKENS.stroke.hairline, ui.active_theme().ui.border),
            StrokeKind::Outside,
        );

        let label = l10n::text(ui.ctx(), "Drawing colour");
        response
            .widget_info(|| WidgetInfo::selected(Role::Button, response.enabled(), open, &label));
        response
            .on_hover_text(l10n::text(ui.ctx(), "Drawing colour"))
            .clicked()
    }
}
