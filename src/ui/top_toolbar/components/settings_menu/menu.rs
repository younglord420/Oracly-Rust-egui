//! Quick settings menu popup.
//!
//! A compact popup menu for quick chart configuration changes (chart type,
//! grid visibility, scale mode, and label options).

use super::config::ChartSettings;
use crate::config::ChartConfig;
use crate::localization as l10n;
use crate::model::ChartType;
use crate::scales::PriceScaleMode;
use crate::theme::ThemeExt;
use crate::ui_kit::ext::UiExt;
use egui::{Align2, Context, Id, Pos2, Rect, Response, Ui};

// ============================================================================
// Settings Menu (Quick Popup)
// ============================================================================

/// Quick settings popup menu for chart configuration
pub struct SettingsMenu {
    /// Current chart settings
    pub settings: ChartSettings,
    /// Whether the menu popup is open
    pub is_open: bool,
    /// Set when the host should open the full scale settings dialog.
    scale_settings_requested: bool,
    /// Set when the host should return the price scale to auto-fit.
    scale_reset_requested: bool,
    /// Set when the user toggles auto scale. `None` until that toggle.
    auto_scale_requested: Option<bool>,
    /// Rect of the trigger button (for popup positioning)
    btn_rect: Option<Rect>,
    /// Rect of the open popup, used to tell an inside click from an outside one.
    menu_rect: Rect,
}

impl SettingsMenu {
    /// Create a new settings menu with default chart settings
    pub fn new() -> Self {
        Self {
            settings: ChartSettings::default(),
            is_open: false,
            scale_settings_requested: false,
            scale_reset_requested: false,
            auto_scale_requested: None,
            btn_rect: None,
            menu_rect: Rect::NOTHING,
        }
    }

    /// Get a reference to the current chart settings
    pub fn settings(&self) -> &ChartSettings {
        &self.settings
    }

    /// Get a mutable reference to the current chart settings
    pub fn settings_mut(&mut self) -> &mut ChartSettings {
        &mut self.settings
    }

    /// Copy chart configuration into the menu so the controls match the chart.
    pub fn sync_from_config(&mut self, config: &ChartConfig) {
        self.settings.show_horizontal_grid = config.show_horizontal_grid;
        self.settings.show_vertical_grid = config.show_vertical_grid;
        self.settings.show_right_axis = config.show_right_axis;
        self.settings.show_left_axis = config.show_left_axis;
        self.settings.show_symbol_labels = config.show_symbol_labels;
        self.settings.show_symbol_last_val = config.show_symbol_last_val;
        self.settings.show_symbol_prev_close = config.show_symbol_prev_close;
        self.settings.show_indicator_labels = config.show_indicator_labels;
        self.settings.show_indicator_last_val = config.show_indicator_last_val;
        self.settings.show_countdown = config.show_countdown;
        self.settings.lock_scale = config.lock_price_to_bar_ratio;
        self.settings.scale_price_chart_only = config.scale_price_chart_only;
        self.settings.no_overlapping_labels = !config.price_axis.allow_label_overlap;
        self.settings.scale_mode = if config.show_left_axis && !config.show_right_axis {
            config.left_axis_scale_mode
        } else {
            config.right_axis_scale_mode
        };
    }

    /// True once after the user chose Scales Properties in the menu.
    pub fn take_scale_settings_request(&mut self) -> bool {
        std::mem::take(&mut self.scale_settings_requested)
    }

    /// True once after the user chose Reset Scale.
    pub fn take_scale_reset_request(&mut self) -> bool {
        std::mem::take(&mut self.scale_reset_requested)
    }

    /// The auto-scale checkbox, once, after the user changes it.
    pub fn take_auto_scale_request(&mut self) -> Option<bool> {
        self.auto_scale_requested.take()
    }

    /// Apply current settings to a ChartConfig
    pub fn apply_to_config(&self, mut config: ChartConfig) -> ChartConfig {
        config.show_grid = self.settings.show_horizontal_grid || self.settings.show_vertical_grid;
        config.show_horizontal_grid = self.settings.show_horizontal_grid;
        config.show_vertical_grid = self.settings.show_vertical_grid;
        config.show_right_axis = self.settings.show_right_axis;
        config.show_left_axis = self.settings.show_left_axis;
        config.show_symbol_labels = self.settings.show_symbol_labels;
        config.show_symbol_last_val = self.settings.show_symbol_last_val;
        config.show_symbol_prev_close = self.settings.show_symbol_prev_close;
        config.show_indicator_labels = self.settings.show_indicator_labels;
        config.show_indicator_last_val = self.settings.show_indicator_last_val;
        config.show_countdown = self.settings.show_countdown;
        config.lock_price_to_bar_ratio = self.settings.lock_scale;
        config.scale_price_chart_only = self.settings.scale_price_chart_only;
        config.price_axis.allow_label_overlap = !self.settings.no_overlapping_labels;
        // The mode belongs to the axis the main scale is drawn on.
        if config.show_left_axis && !config.show_right_axis {
            config.left_axis_scale_mode = self.settings.scale_mode;
        } else {
            config.right_axis_scale_mode = self.settings.scale_mode;
        }
        config
    }

    /// Chart type selected in the menu. Applying the menu config does not change it;
    /// the host sets it with [`Chart::set_chart_type`](crate::widget::Chart::set_chart_type).
    pub fn chart_type(&self) -> ChartType {
        self.settings.chart_type
    }

    /// Show the settings button (bottom-right corner)
    pub fn show_btn(&mut self, ui: &mut Ui, chart_rect: Rect) -> Response {
        let btn_size = egui::vec2(32.0, 32.0);
        let btn_pos = Pos2::new(
            chart_rect.max.x - btn_size.x - 10.0,
            chart_rect.max.y - btn_size.y - 10.0,
        );

        let btn_rect = Rect::from_min_size(btn_pos, btn_size);
        let btn_id = ui.scope_id().with("settings_btn");

        let response = ui.interact(btn_rect, btn_id, egui::Sense::click());
        let translated_label = l10n::text(ui.ctx(), "Chart settings");
        response.widget_info(|| {
            egui::WidgetInfo::labeled(egui::Role::Button, response.enabled(), &translated_label)
        });
        crate::styles::focus::draw_focus_ring(ui, &response, ui.active_theme().radius.md);

        let visuals = ui.style().interact(&response);
        let bg_color = if response.hovered() {
            visuals.bg_fill
        } else {
            ui.visuals().widgets.inactive.bg_fill
        };

        ui.painter()
            .rect_filled(btn_rect, ui.active_theme().radius.md, bg_color);
        ui.painter().text(
            btn_rect.center(),
            Align2::CENTER_CENTER,
            l10n::text(ui.ctx(), "S"),
            egui::FontId::proportional(ui.active_theme().text.dialog_title),
            visuals.text_color(),
        );

        if response.clicked() {
            self.is_open = !self.is_open;
        }

        self.btn_rect = Some(btn_rect);
        response
    }

    /// Show the settings popup menu
    pub fn show_menu(&mut self, ctx: &Context) {
        if !self.is_open {
            return;
        }
        let Some(btn_rect) = self.btn_rect else {
            return;
        };

        let popup_id = Id::unique("settings_popup");
        let response = egui::Area::new(popup_id)
            .order(egui::Order::Foreground)
            .pivot(Align2::RIGHT_BOTTOM)
            .fixed_pos(btn_rect.left_top())
            .constrain(true)
            .show(ctx, |ui| {
                crate::ui_kit::glass_popup(ui).show(ui, |ui| {
                    ui.set_min_width(180.0);
                    ui.set_max_width(220.0);
                    self.draw_menu_sections(ui);
                });
            });
        self.menu_rect = response.response.rect;
        self.handle_click_outside(ctx, btn_rect);
    }

    fn draw_menu_sections(&mut self, ui: &mut egui::Ui) {
        self.draw_chart_type_section(ui);
        self.draw_appearance_section(ui);
        self.draw_scale_section(ui);
        self.draw_labels_section(ui);

        ui.separator();
        if ui
            .button(l10n::text(ui.ctx(), "Scales Properties..."))
            .clicked()
        {
            self.scale_settings_requested = true;
            self.is_open = false;
        }
    }

    fn draw_chart_type_section(&mut self, ui: &mut egui::Ui) {
        ui.strong_label(l10n::text(ui.ctx(), "Chart Type"));
        ui.separator();

        for chart_type in &[
            ChartType::Candles,
            ChartType::Bars,
            ChartType::Line,
            ChartType::Area,
        ] {
            if ui
                .selectable_label(
                    self.settings.chart_type == *chart_type,
                    l10n::text(ui.ctx(), chart_type.as_str()),
                )
                .clicked()
            {
                self.settings.chart_type = *chart_type;
            }
        }
        ui.separator();
    }

    fn draw_appearance_section(&mut self, ui: &mut egui::Ui) {
        ui.strong_label(l10n::text(ui.ctx(), "Appearance"));
        ui.separator();
        ui.checkbox(
            &mut self.settings.show_horizontal_grid,
            l10n::text(ui.ctx(), "Horizontal Grid"),
        );
        ui.checkbox(
            &mut self.settings.show_vertical_grid,
            l10n::text(ui.ctx(), "Vertical Grid"),
        );
        ui.separator();
    }

    fn draw_scale_section(&mut self, ui: &mut egui::Ui) {
        ui.strong_label(l10n::text(ui.ctx(), "Scale"));
        ui.separator();

        if ui.button(l10n::text(ui.ctx(), "Reset Scale")).clicked() {
            self.scale_reset_requested = true;
            self.is_open = false;
        }

        ui.checkbox(
            &mut self.settings.show_left_axis,
            l10n::text(ui.ctx(), "Left Axis"),
        );
        ui.checkbox(
            &mut self.settings.show_right_axis,
            l10n::text(ui.ctx(), "Right Axis"),
        );

        if ui
            .selectable_label(
                self.settings.scale_mode == PriceScaleMode::Normal,
                l10n::text(ui.ctx(), "Regular"),
            )
            .clicked()
        {
            self.settings.scale_mode = PriceScaleMode::Normal;
        }
        if ui
            .checkbox(
                &mut self.settings.auto_scale,
                l10n::text(ui.ctx(), "Auto scale"),
            )
            .changed()
        {
            self.auto_scale_requested = Some(self.settings.auto_scale);
        }

        ui.checkbox(
            &mut self.settings.lock_scale,
            l10n::text(ui.ctx(), "Lock price to bar ratio"),
        );
        ui.checkbox(
            &mut self.settings.scale_price_chart_only,
            l10n::text(ui.ctx(), "Scale Price Chart Only"),
        );
        ui.separator();

        if ui
            .selectable_label(
                self.settings.scale_mode == PriceScaleMode::Percentage,
                l10n::text(ui.ctx(), "Percentage"),
            )
            .clicked()
        {
            self.settings.scale_mode = PriceScaleMode::Percentage;
        }
        if ui
            .selectable_label(
                self.settings.scale_mode == PriceScaleMode::Logarithmic,
                l10n::text(ui.ctx(), "Log Scale"),
            )
            .clicked()
        {
            self.settings.scale_mode = PriceScaleMode::Logarithmic;
        }
        ui.separator();
    }

    fn draw_labels_section(&mut self, ui: &mut egui::Ui) {
        ui.strong_label(l10n::text(ui.ctx(), "Labels"));
        ui.separator();

        ui.checkbox(
            &mut self.settings.show_symbol_labels,
            l10n::text(ui.ctx(), "Symbol Labels"),
        );
        ui.checkbox(
            &mut self.settings.show_symbol_last_val,
            l10n::text(ui.ctx(), "Symbol Last Value"),
        );
        ui.checkbox(
            &mut self.settings.show_symbol_prev_close,
            l10n::text(ui.ctx(), "Symbol Prev. Close Value"),
        );
        ui.separator();

        ui.checkbox(
            &mut self.settings.show_indicator_labels,
            l10n::text(ui.ctx(), "Indicator Labels"),
        );
        ui.checkbox(
            &mut self.settings.show_indicator_last_val,
            l10n::text(ui.ctx(), "Indicator Last Value"),
        );
        ui.separator();

        ui.checkbox(
            &mut self.settings.show_countdown,
            l10n::text(ui.ctx(), "Countdown"),
        );
        ui.checkbox(
            &mut self.settings.no_overlapping_labels,
            l10n::text(ui.ctx(), "No Overlapping Labels"),
        );
    }

    fn handle_click_outside(&mut self, ctx: &Context, btn_rect: Rect) {
        if ctx.input(|i| i.pointer.any_released())
            && let Some(pos) = ctx.input(|i| i.pointer.interact_pos())
            && !self.menu_rect.contains(pos)
            && !btn_rect.contains(pos)
        {
            self.is_open = false;
        }
    }
}

impl Default for SettingsMenu {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::scales::PriceScaleMode;

    #[test]
    fn reset_scale_is_a_request_and_auto_scale_is_not_regular() {
        let ctx = Context::default();
        let mut menu = SettingsMenu::new();
        menu.settings.scale_mode = PriceScaleMode::Logarithmic;
        let output = ctx.run_ui(Default::default(), |ui| menu.draw_scale_section(ui));
        let text = |label: &str| {
            output.shapes.iter().any(|shape| {
                matches!(&shape.shape, egui::Shape::Text(text) if text.galley.text() == label)
            })
        };
        assert!(text("Regular"));
        assert!(text("Auto scale"));
        assert!(!text("Auto Scale"));
        let reset = output.shapes.iter().find_map(|shape| match &shape.shape {
            egui::Shape::Text(text) if text.galley.text() == "Reset Scale" => {
                Some(text.pos + text.galley.size() / 2.0)
            }
            _ => None,
        });
        output.drop_without_applying_deltas();
        let pos = reset.unwrap();
        let click = |pressed| egui::Event::PointerButton {
            pos,
            button: egui::PointerButton::Primary,
            pressed,
            modifiers: Default::default(),
        };
        for events in [
            vec![egui::Event::PointerMoved(pos)],
            vec![click(true)],
            vec![click(false)],
        ] {
            ctx.run_ui(
                egui::RawInput {
                    events,
                    ..Default::default()
                },
                |ui| menu.draw_scale_section(ui),
            )
            .drop_without_applying_deltas();
        }
        assert!(menu.take_scale_reset_request());
        assert_eq!(menu.settings.scale_mode, PriceScaleMode::Logarithmic);
        assert!(menu.settings.auto_scale);
    }
}
