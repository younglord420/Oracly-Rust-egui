//! Tabbed dialog for editing chart display settings.
//!
//! Open [`SettingsDialog`] with the chart's settings and apply the returned
//! [`SettingsAction`] in the host.

use crate::localization as l10n;
use crate::theme::ThemeExt;
use crate::widget::settings::ChartSettingsState;
mod tabs;

use crate::icons::icons as embedded_icons;
use crate::services::StoreKind;
use crate::theme::Theme;
use crate::tokens::DESIGN_TOKENS;
use crate::ui::Control;
use crate::ui::dialogs::templates::TemplateStore;
use crate::ui::top_toolbar::components::settings_menu::{
    actions::SettingsAction, config::SettingsDialogConfig, types::SettingsTab,
};
use crate::ui_kit::ColorPickerState;
use crate::ui_kit::dialog::DialogFooter;
use crate::ui_kit::modal::Modal;
use crate::ui_kit::tab_bar::TabBar;
use egui::{Color32, Context, Rect, Response, RichText, Ui, Vec2};
use std::collections::HashSet;

// ============================================================================
// Settings Dialog (Modal)
// ============================================================================

/// One outer height for every tab, so switching tabs does not resize the dialog.
const DIALOG_HEIGHT: f32 = 560.0;
/// Width of the appearance preview column.
const PREVIEW_WIDTH: f32 = 280.0;
/// Available width below which the preview column is hidden.
const PREVIEW_MIN_WIDTH: f32 = 720.0;

/// Modal settings dialog with tabbed interface.
///
/// Edits chart display settings, event category visibility and the host trading and
/// alert-overlay preferences.
pub struct SettingsDialog {
    /// Whether the dialog is currently open
    pub is_open: bool,
    active_tab: SettingsTab,
    settings: ChartSettingsState,
    /// Current template name (empty if using default/unsaved settings)
    template_name: String,
    /// Whether the template dropdown menu is open
    template_menu_open: bool,
    /// Host-readable chart templates and their optional storage connection.
    pub templates: TemplateStore<ChartSettingsState>,
    save_name: Option<String>,
    timezones: Vec<String>,
    /// Dialog styling configuration (width, height overrides)
    config: SettingsDialogConfig,
    color_picker_states: ColorPickerStates,
    /// Controls the host hid.
    hidden: HashSet<Control>,
    /// The local sounds the host can play, by name, offered on the Trading and Alerts tabs. The host
    /// plays the chosen one; see
    /// [`Trading::take_execution_sound`](crate::ui::trading::Trading::take_execution_sound).
    /// Without any, the tab offers no choice.
    pub execution_sounds: Vec<String>,
    sound_preview: Option<(String, f32)>,
    /// Pending light/dark switch from the Canvas tab. Emitted with Ok, discarded on Cancel.
    pending_dark_theme: Option<bool>,
    /// Host theme when the dialog opened, so Cancel and an unchanged Ok leave it alone.
    opened_dark: Option<bool>,
    /// Settings committed together with a theme switch, because Ok returns one action.
    committed: Option<ChartSettingsState>,
    /// Colours the user picked in this session, which a theme switch leaves alone.
    colors_touched: TouchedColors,
    /// Read-only chart that shows the edited appearance. Never the host chart.
    preview: Option<crate::widget::Chart>,
    preview_applied: Option<ChartSettingsState>,
    preview_symbol: Option<String>,
}

#[derive(Clone, Default)]
struct TouchedColors {
    background: bool,
    gradient_top: bool,
    gradient_bottom: bool,
    grid_h: bool,
    grid_v: bool,
    crosshair: bool,
    watermark: bool,
    scales_text: bool,
    scales_lines: bool,
}

#[derive(Clone, Default)]
struct ColorPickerStates {
    body_up: ColorPickerState,
    body_down: ColorPickerState,
    border_up: ColorPickerState,
    border_down: ColorPickerState,
    wick_up: ColorPickerState,
    wick_down: ColorPickerState,
    background: ColorPickerState,
    background_gradient_top: ColorPickerState,
    background_gradient_bottom: ColorPickerState,
    grid_h: ColorPickerState,
    grid_v: ColorPickerState,
    crosshair: ColorPickerState,
    watermark: ColorPickerState,
    scales_text: ColorPickerState,
    scales_lines: ColorPickerState,
    alert: ColorPickerState,
    symbol_value: ColorPickerState,
    study_value: ColorPickerState,
    extended_value: ColorPickerState,
}

impl Default for SettingsDialog {
    fn default() -> Self {
        Self::new()
    }
}

impl SettingsDialog {
    /// Create a [`SettingsDialog`] with default settings.
    pub fn new() -> Self {
        let settings = ChartSettingsState::default();
        let timezones: &[&str] = if cfg!(feature = "timezones") {
            &[
                "UTC",
                "America/New_York",
                "America/Chicago",
                "America/Los_Angeles",
                "Europe/London",
                "Europe/Paris",
                "Asia/Tokyo",
                "Asia/Hong_Kong",
                "Asia/Singapore",
                "Australia/Sydney",
            ]
        } else {
            &[
                "UTC", "-08:00", "-06:00", "-05:00", "+01:00", "+08:00", "+09:00", "+10:00",
            ]
        };
        Self {
            is_open: false,
            active_tab: SettingsTab::Symbol,
            color_picker_states: ColorPickerStates {
                body_up: ColorPickerState::new(settings.candle_colors.body_up),
                body_down: ColorPickerState::new(settings.candle_colors.body_down),
                border_up: ColorPickerState::new(settings.candle_colors.border_up),
                border_down: ColorPickerState::new(settings.candle_colors.border_down),
                wick_up: ColorPickerState::new(settings.candle_colors.wick_up),
                wick_down: ColorPickerState::new(settings.candle_colors.wick_down),
                background: ColorPickerState::new(settings.chart_basic_styles.background_color),
                background_gradient_top: ColorPickerState::new(
                    settings.chart_basic_styles.background_gradient_top,
                ),
                background_gradient_bottom: ColorPickerState::new(
                    settings.chart_basic_styles.background_gradient_bottom,
                ),
                grid_h: ColorPickerState::new(settings.grid_lines.horizontal_color),
                grid_v: ColorPickerState::new(settings.grid_lines.vertical_color),
                crosshair: ColorPickerState::new(settings.crosshair.color),
                watermark: ColorPickerState::new(settings.watermark.color),
                scales_text: ColorPickerState::new(settings.scales_appearance.text_color),
                scales_lines: ColorPickerState::new(settings.scales_appearance.lines_color),
                alert: ColorPickerState::new(settings.alerts.alert_color),
                symbol_value: ColorPickerState::new(
                    settings
                        .scales_and_lines
                        .symbol_price_color
                        .unwrap_or(settings.candle_colors.body_up),
                ),
                study_value: ColorPickerState::new(
                    settings
                        .scales_and_lines
                        .study_value_color
                        .unwrap_or(settings.scales_appearance.text_color),
                ),
                extended_value: ColorPickerState::new(
                    settings.scales_and_lines.extended_session_price_color,
                ),
            },
            settings,
            template_name: String::new(),
            template_menu_open: false,
            templates: TemplateStore::new(StoreKind::ChartTemplates),
            save_name: None,
            timezones: timezones.iter().map(|zone| (*zone).to_owned()).collect(),
            config: SettingsDialogConfig::default(),
            hidden: HashSet::new(),
            execution_sounds: Vec::new(),
            sound_preview: None,
            pending_dark_theme: None,
            opened_dark: None,
            committed: None,
            colors_touched: TouchedColors::default(),
            preview: None,
            preview_applied: None,
            preview_symbol: None,
        }
    }

    /// Symbol shown at the right of the preview header. The preview still uses simulated prices.
    pub fn set_preview_symbol(&mut self, symbol: &str) {
        let symbol = symbol.trim();
        self.preview_symbol = (!symbol.is_empty()).then(|| symbol.to_owned());
        if let Some(chart) = self.preview.as_mut() {
            chart.symbol = self.preview_symbol.clone().unwrap_or_default();
        }
    }

    /// Settings saved on Ok when that Ok also switches the theme. `None` after it is taken.
    pub fn take_committed(&mut self) -> Option<ChartSettingsState> {
        self.committed.take()
    }

    /// Shows or hides `control`: Templates (the template button and menu; Reset to defaults
    /// then takes the button's place). The dialog ignores the other controls.
    pub fn set_control_visible(&mut self, control: Control, visible: bool) {
        crate::ui::app_state::set_control_visible(&mut self.hidden, control, visible);
        if control == Control::Templates && !visible {
            self.template_menu_open = false;
        }
    }

    /// Open the dialog with the supplied state.
    pub fn open(&mut self, settings: ChartSettingsState) {
        self.is_open = true;
        self.template_menu_open = false;
        self.save_name = None;
        self.sound_preview = None;
        self.pending_dark_theme = None;
        self.opened_dark = None;
        self.committed = None;
        self.colors_touched = TouchedColors::default();
        self.preview_applied = None;
        self.settings = settings;
        self.active_tab = SettingsTab::Symbol;
        self.update_color_pickers_from_settings();
    }

    /// Sync all color picker states from the current settings
    fn update_color_pickers_from_settings(&mut self) {
        self.color_picker_states.body_up.sel_color = self.settings.candle_colors.body_up;
        self.color_picker_states.body_down.sel_color = self.settings.candle_colors.body_down;
        self.color_picker_states.border_up.sel_color = self.settings.candle_colors.border_up;
        self.color_picker_states.border_down.sel_color = self.settings.candle_colors.border_down;
        self.color_picker_states.wick_up.sel_color = self.settings.candle_colors.wick_up;
        self.color_picker_states.wick_down.sel_color = self.settings.candle_colors.wick_down;
        self.color_picker_states.background.sel_color =
            self.settings.chart_basic_styles.background_color;
        self.color_picker_states.background_gradient_top.sel_color =
            self.settings.chart_basic_styles.background_gradient_top;
        self.color_picker_states
            .background_gradient_bottom
            .sel_color = self.settings.chart_basic_styles.background_gradient_bottom;
        self.color_picker_states.grid_h.sel_color = self.settings.grid_lines.horizontal_color;
        self.color_picker_states.grid_v.sel_color = self.settings.grid_lines.vertical_color;
        self.color_picker_states.crosshair.sel_color = self.settings.crosshair.color;
        self.color_picker_states.watermark.sel_color = self.settings.watermark.color;
        self.color_picker_states.scales_text.sel_color = self.settings.scales_appearance.text_color;
        self.color_picker_states.scales_lines.sel_color =
            self.settings.scales_appearance.lines_color;

        self.color_picker_states.alert.sel_color = self.settings.alerts.alert_color;
        self.color_picker_states.symbol_value.sel_color = self
            .settings
            .scales_and_lines
            .symbol_price_color
            .unwrap_or(self.settings.candle_colors.body_up);
        self.color_picker_states.study_value.sel_color = self
            .settings
            .scales_and_lines
            .study_value_color
            .unwrap_or(self.settings.scales_appearance.text_color);
        self.color_picker_states.extended_value.sel_color =
            self.settings.scales_and_lines.extended_session_price_color;
    }

    /// Close the dialog.
    pub fn close(&mut self) {
        self.is_open = false;
    }

    /// Read the display settings being edited.
    pub fn settings(&self) -> &ChartSettingsState {
        &self.settings
    }

    /// Show the dialog and return the requested action.
    pub fn show(&mut self, ctx: &Context, theme: &Theme) -> SettingsAction {
        self.templates.poll(ctx);
        let mut action = SettingsAction::None;

        if !self.is_open {
            return action;
        }

        let basic = &self.settings.chart_basic_styles;
        for (picker, color, themed) in [
            (
                &mut self.color_picker_states.background,
                basic.background_color,
                theme.chart.bg,
            ),
            (
                &mut self.color_picker_states.background_gradient_top,
                basic.background_gradient_top,
                theme.chart.bg,
            ),
            (
                &mut self.color_picker_states.background_gradient_bottom,
                basic.background_gradient_bottom,
                theme.chart.bg_axis,
            ),
        ] {
            picker.sel_color = if color == Color32::PLACEHOLDER {
                themed
            } else {
                color
            };
        }

        if self.opened_dark.is_none() {
            self.opened_dark = Some(theme.dark);
        }
        let mut is_open = self.is_open;
        let height = if self.config.height > 0.0 {
            self.config.height
        } else {
            DIALOG_HEIGHT
        };
        let width = if self.config.width > 0.0 {
            self.config.width
        } else {
            600.0
        };
        Modal::new("chart_settings", "Chart settings")
            .width(width)
            .height(height)
            .dim_background(false)
            .show(ctx, &mut is_open, |ui| {
                ui.add_space(theme.spacing.sm);
                action = self.draw_content(ui, theme);
            });

        // Only update is_open from egui's Window if btns didn't already close it
        // (btns set self.is_open = false directly)
        if self.is_open && !is_open {
            // Window was closed via egui's mechanism (outside click, etc)
            self.is_open = false;
            action = SettingsAction::Cancel;
        }

        if let SettingsAction::Apply(settings) = &mut action {
            let basic = &mut settings.chart_basic_styles;
            for (color, themed) in [
                (&mut basic.background_color, theme.chart.bg),
                (&mut basic.background_gradient_top, theme.chart.bg),
                (&mut basic.background_gradient_bottom, theme.chart.bg_axis),
            ] {
                if *color == Color32::PLACEHOLDER {
                    *color = themed;
                }
            }
        }

        if let Some((name, volume)) = self.sound_preview.take() {
            return SettingsAction::PreviewSound { name, volume };
        }
        action
    }

    fn draw_content(&mut self, ui: &mut Ui, theme: &Theme) -> SettingsAction {
        let mut action = SettingsAction::None;

        let mut active = self.active_tab;
        TabBar::new(SettingsTab::all(), &mut active)
            .pills()
            .show(ui);
        self.active_tab = active;

        let show_preview = ui.available_width() >= PREVIEW_MIN_WIDTH;
        let content_height = (ui.available_height() - 68.0).max(0.0);
        ui.horizontal(|ui| {
            let preview_width = if show_preview { PREVIEW_WIDTH } else { 0.0 };
            let gap = if show_preview { theme.spacing.lg } else { 0.0 };
            let content_width = (ui.available_width() - preview_width - gap).max(0.0);
            ui.allocate_ui_with_layout(
                Vec2::new(content_width, content_height),
                egui::Layout::top_down(egui::Align::Min),
                |ui| {
                    egui::ScrollArea::both()
                        .id_salt("settings_content")
                        .auto_shrink([false, false])
                        .max_height(ui.available_height())
                        .show(ui, |ui| {
                            ui.spacing_mut().item_spacing = Vec2::splat(theme.spacing.lg);
                            match self.active_tab {
                                SettingsTab::Symbol => self.draw_symbol_tab(ui, theme),
                                SettingsTab::StatusLine => self.draw_status_line_tab(ui, theme),
                                SettingsTab::ScalesAndLines => {
                                    self.draw_scales_and_lines_tab(ui, theme)
                                }
                                SettingsTab::Canvas => self.draw_canvas_tab(ui, theme),
                                SettingsTab::Trading => self.draw_trading_tab(ui, theme),
                                SettingsTab::Alerts => self.draw_alerts_tab(ui, theme),
                                SettingsTab::Events => self.draw_events_tab(ui, theme),
                            }
                        });
                },
            );
            if show_preview {
                ui.add_space(gap);
                ui.allocate_ui_with_layout(
                    Vec2::new(preview_width, content_height),
                    egui::Layout::top_down(egui::Align::Min),
                    |ui| {
                        self.draw_preview(ui);
                    },
                );
            }
        });

        let error = self.settings.validate().err();
        let (ok, cancel) = DialogFooter::new("Ok")
            .secondary("Cancel")
            .primary_enabled(error.is_none())
            .show_with_left(ui, |ui| {
                if let Some(error) = error {
                    crate::ui_kit::status_message::status_message(
                        ui,
                        crate::ui_kit::status_message::MessageKind::Error,
                        error,
                    );
                } else if self.hidden.contains(&Control::Templates) {
                    self.draw_reset_button(ui);
                } else {
                    let button = self.draw_template_button(ui, theme);
                    if button.clicked() {
                        self.template_menu_open = !self.template_menu_open;
                    }
                    if self.template_menu_open {
                        action = self.draw_template_menu(ui, theme, &button, action.clone());
                    }
                }
            });
        if ok {
            self.is_open = false;
            let theme_changed = self
                .pending_dark_theme
                .is_some_and(|dark| self.opened_dark != Some(dark));
            if theme_changed {
                self.committed = Some(self.settings.clone());
                action = SettingsAction::SetDarkTheme(self.pending_dark_theme.unwrap_or(false));
            } else {
                self.settings.dark_theme = None;
                action = SettingsAction::Apply(Box::new(self.settings.clone()));
            }
            self.pending_dark_theme = None;
        } else if cancel {
            self.pending_dark_theme = None;
            action = SettingsAction::Cancel;
            self.is_open = false;
        }
        action
    }

    fn choose_dark_theme(&mut self, host: &Theme, dark: bool) {
        let previous = match self.pending_dark_theme {
            Some(true) => Theme::dark(),
            Some(false) => Theme::light(),
            None => host.clone(),
        };
        let next = if dark { Theme::dark() } else { Theme::light() };
        self.recolor_following(&previous, &next);
        self.pending_dark_theme = Some(dark);
        self.settings.dark_theme = Some(dark);
        self.preview_applied = None;
        self.update_color_pickers_from_settings();
    }

    /// Theme-following colours take the new theme unless the user picked them this session.
    fn recolor_following(&mut self, old: &Theme, new: &Theme) {
        let touched = self.colors_touched.clone();
        let swap = |color: &mut Color32, picked: bool, from: Color32, to: Color32| {
            if !picked && (*color == from || *color == Color32::PLACEHOLDER) {
                *color = to;
            }
        };
        let styles = &mut self.settings.chart_basic_styles;
        swap(
            &mut styles.background_color,
            touched.background,
            old.chart.bg,
            new.chart.bg,
        );
        swap(
            &mut styles.background_gradient_top,
            touched.gradient_top,
            old.chart.bg,
            new.chart.bg,
        );
        swap(
            &mut styles.background_gradient_bottom,
            touched.gradient_bottom,
            old.chart.bg_axis,
            new.chart.bg_axis,
        );
        let grid = &mut self.settings.grid_lines;
        swap(
            &mut grid.horizontal_color,
            touched.grid_h,
            old.chart.grid_line,
            new.chart.grid_line,
        );
        swap(
            &mut grid.vertical_color,
            touched.grid_v,
            old.chart.grid_line,
            new.chart.grid_line,
        );
        swap(
            &mut self.settings.crosshair.color,
            touched.crosshair,
            old.chart.crosshair_line,
            new.chart.crosshair_line,
        );
        swap(
            &mut self.settings.watermark.color,
            touched.watermark,
            old.chart.grid_line,
            new.chart.grid_line,
        );
        swap(
            &mut self.settings.scales_appearance.text_color,
            touched.scales_text,
            old.chart.axis_text,
            new.chart.axis_text,
        );
        swap(
            &mut self.settings.scales_appearance.lines_color,
            touched.scales_lines,
            old.ui.border,
            new.ui.border,
        );
    }

    fn timezone_choices(&self) -> Vec<String> {
        let mut zones = vec!["Local".to_owned()];
        if let Some(zone) = &self.settings.exchange_timezone {
            let exchange = format!("Exchange:{zone}");
            if !zones.contains(&exchange) {
                zones.push(exchange);
            }
        }
        for zone in &self.timezones {
            if !zones.contains(zone) {
                zones.push(zone.clone());
            }
        }
        if !zones.iter().any(|zone| zone == &self.settings.timezone) {
            zones.insert(0, self.settings.timezone.clone());
        }
        zones
    }

    fn draw_preview(&mut self, ui: &mut Ui) {
        ui.label(
            RichText::new(l10n::text(ui.ctx(), "APPEARANCE PREVIEW"))
                .small()
                .strong(),
        );
        ui.horizontal(|ui| {
            ui.label(l10n::text(ui.ctx(), "Sample data"));
            if let Some(symbol) = self.preview_symbol.clone() {
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    ui.label(symbol);
                });
            }
        });
        if self.preview.is_none() {
            self.preview = Some(preview_chart(
                self.preview_symbol.as_deref().unwrap_or("Sample data"),
            ));
        }
        let settings = self.settings.clone();
        if self.preview_applied.as_ref() != Some(&settings) {
            if let Some(chart) = self.preview.as_mut() {
                let _ = settings.apply_to_chart(chart);
                let animations = &mut chart.chart_options.animations;
                animations.autoscale_ms = Some(0);
                animations.scroll_ms = Some(0);
                animations.price_zoom_ms = Some(0);
            }
            self.preview_applied = Some(settings);
        }
        let height = (ui.available_height() - 28.0).max(80.0);
        ui.allocate_ui(Vec2::new(ui.available_width(), height), |ui| {
            if let Some(chart) = self.preview.as_mut() {
                chart.set_size(Some(ui.available_size()));
                chart.suppress_shortcuts(ui.ctx());
                chart.show(ui);
            }
        });
        ui.label(l10n::text(
            ui.ctx(),
            "Simulated prices · scroll to zoom, drag to pan",
        ));
    }

    fn draw_template_button(&self, ui: &mut Ui, theme: &Theme) -> Response {
        let template_label = if self.template_name.is_empty() {
            "Template".to_string()
        } else {
            self.template_name.clone()
        };
        ui.add_sized(
            Vec2::new(
                DESIGN_TOKENS.sizing.settings_dialog.template_button_width,
                DESIGN_TOKENS.sizing.settings_dialog.button_height,
            ),
            egui::Button::new((
                RichText::new(l10n::text(ui.ctx(), &template_label))
                    .size(theme.text.title)
                    .color(theme.ui.text),
                embedded_icons::CHEVRON_DOWN.as_image_tinted(Vec2::splat(12.0), theme.ui.text),
            ))
            .truncate()
            .fill(theme.ui.panel_bg)
            .corner_radius(theme.radius.md as u8)
            .min_size(Vec2::new(
                DESIGN_TOKENS.sizing.settings_dialog.template_button_width,
                DESIGN_TOKENS.sizing.settings_dialog.button_height,
            )),
        )
        .on_hover_text(l10n::text(ui.ctx(), template_label))
    }

    fn draw_template_menu(
        &mut self,
        ui: &mut Ui,
        theme: &Theme,
        template_btn: &Response,
        action: SettingsAction,
    ) -> SettingsAction {
        let menu_id = ui.scope_id().with("template_menu");
        let menu_rect = Rect::from_min_size(
            template_btn.rect.left_bottom() + Vec2::new(0.0, theme.spacing.xs),
            Vec2::new(
                DESIGN_TOKENS.sizing.settings_dialog.template_menu_width,
                DESIGN_TOKENS
                    .sizing
                    .settings_dialog
                    .template_menu_max_height,
            ),
        );

        let result = egui::Area::new(menu_id)
            .fixed_pos(menu_rect.min)
            .order(egui::Order::Foreground)
            .show(ui.ctx(), |ui| {
                egui::Frame::popup(ui.style())
                    .fill(theme.ui.panel_bg)
                    .corner_radius(theme.radius.md as u8)
                    .stroke(egui::Stroke::new(
                        DESIGN_TOKENS.stroke.hairline,
                        theme.ui.border_control,
                    ))
                    .show(ui, |ui| {
                        ui.set_width(DESIGN_TOKENS.sizing.settings_dialog.template_menu_width);
                        egui::ScrollArea::vertical()
                            .max_height(
                                DESIGN_TOKENS
                                    .sizing
                                    .settings_dialog
                                    .template_menu_max_height,
                            )
                            .show(ui, |ui| self.draw_template_menu_items(ui, theme, action))
                            .inner
                    })
                    .inner
            });

        self.handle_template_menu_close(ui, template_btn, result.response.rect);
        result.inner
    }

    fn draw_template_menu_items(
        &mut self,
        ui: &mut Ui,
        theme: &Theme,
        mut action: SettingsAction,
    ) -> SettingsAction {
        action = self.draw_save_as_button(ui, action);
        action = self.draw_update_current_button(ui, action);
        ui.separator();
        action = self.draw_template_list(ui, theme, action);
        ui.separator();
        self.draw_reset_button(ui);
        action
    }

    fn draw_save_as_button(&mut self, ui: &mut Ui, mut action: SettingsAction) -> SettingsAction {
        if let Some(name) = &mut self.save_name {
            let label = ui.label(l10n::text(ui.ctx(), "Template name"));
            ui.add(egui::TextEdit::singleline(name).desired_width(ui.available_width()))
                .labelled_by(label.id);
            let name = name.trim().to_owned();
            let label = if self.templates.entries().contains_key(&name) {
                "Replace"
            } else {
                "Save"
            };
            let save = ui
                .add_enabled(
                    self.templates.ready() && !name.is_empty(),
                    egui::Button::new(l10n::text(ui.ctx(), label)),
                )
                .clicked();
            let cancel = ui.button(l10n::text(ui.ctx(), "Cancel")).clicked();
            if save {
                match self.templates.save(&name, self.settings.clone()) {
                    Ok(()) => {
                        self.template_name = name.clone();
                        action = SettingsAction::SaveTemplate(name);
                        self.save_name = None;
                    }
                    Err(error) => self.templates.error = Some(error),
                }
            } else if cancel {
                self.save_name = None;
            }
        } else if ui
            .button(
                RichText::new(l10n::text(ui.ctx(), "Save as...")).size(ui.active_theme().text.body),
            )
            .clicked()
        {
            self.save_name = Some(String::new());
        }
        if let Some(error) = &self.templates.error {
            ui.colored_label(ui.visuals().error_fg_color, l10n::text(ui.ctx(), error));
            if !self.templates.ready()
                && ui
                    .button(l10n::text(ui.ctx(), "Retry loading templates"))
                    .clicked()
            {
                self.templates.reload();
            }
        }

        action
    }

    fn draw_update_current_button(
        &mut self,
        ui: &mut Ui,
        mut action: SettingsAction,
    ) -> SettingsAction {
        if !self.template_name.is_empty()
            && ui
                .add_enabled(
                    self.templates.ready(),
                    egui::Button::new(
                        RichText::new(l10n::text(ui.ctx(), "Update current"))
                            .size(ui.active_theme().text.body),
                    ),
                )
                .clicked()
        {
            match self
                .templates
                .save(&self.template_name, self.settings.clone())
            {
                Ok(()) => action = SettingsAction::SaveTemplate(self.template_name.clone()),
                Err(error) => self.templates.error = Some(error),
            }
        }
        action
    }

    fn draw_template_list(
        &mut self,
        ui: &mut Ui,
        theme: &Theme,
        mut action: SettingsAction,
    ) -> SettingsAction {
        let templates: Vec<String> = self.templates.entries().keys().cloned().collect();

        if templates.is_empty() {
            ui.label(
                RichText::new(l10n::text(ui.ctx(), "No saved templates"))
                    .size(theme.text.small)
                    .color(theme.ui.text_secondary),
            );
        } else {
            for name in templates {
                action = self.draw_template_item(ui, theme, name, action);
            }
        }
        action
    }

    fn draw_template_item(
        &mut self,
        ui: &mut Ui,
        theme: &Theme,
        name: String,
        mut action: SettingsAction,
    ) -> SettingsAction {
        let is_sel = self.template_name == name;
        let text = if is_sel {
            RichText::new(l10n::text(ui.ctx(), &name))
                .size(theme.text.body)
                .color(theme.ui.text)
        } else {
            RichText::new(l10n::text(ui.ctx(), &name)).size(theme.text.body)
        };

        if ui
            .add_enabled(self.templates.ready(), egui::Button::new(text).truncate())
            .on_hover_text(l10n::text(ui.ctx(), &name))
            .clicked()
        {
            if let Some(template) = self.templates.entries().get(&name) {
                let kind = self.settings.chart_type;
                let params = template
                    .chart_type_params
                    .clone()
                    .or_else(|| self.settings.chart_type_params.clone());
                let source = template.price_source.or(self.settings.price_source);
                self.settings = template.clone();
                self.settings.chart_type = kind;
                self.settings.chart_type_params = params;
                self.settings.price_source = source;
                self.template_name = name.clone();
                self.update_color_pickers_from_settings();
                action = SettingsAction::LoadTemplate(name.clone());
            }
            self.template_menu_open = false;
        }
        if ui
            .add_enabled(
                self.templates.ready(),
                egui::Button::new(l10n::format_text(
                    ui.ctx(),
                    "Delete {name}",
                    &[("name", name.to_string())],
                ))
                .truncate(),
            )
            .clicked()
        {
            match self.templates.delete(&name) {
                Ok(()) => {
                    if self.template_name == name {
                        self.template_name.clear();
                    }
                }
                Err(error) => self.templates.error = Some(error),
            }
        }
        action
    }

    fn draw_reset_button(&mut self, ui: &mut Ui) {
        if ui
            .button(
                RichText::new(l10n::text(ui.ctx(), "Reset to defaults"))
                    .size(ui.active_theme().text.body),
            )
            .clicked()
        {
            self.settings = ChartSettingsState {
                chart_type: self.settings.chart_type,
                chart_type_params: Some(Default::default()),
                price_source: Some(Default::default()),
                ..Default::default()
            };
            self.template_name.clear();
            self.update_color_pickers_from_settings();
            self.template_menu_open = false;
        }
    }

    fn handle_template_menu_close(&mut self, ui: &Ui, template_btn: &Response, menu_rect: Rect) {
        if ui.input(|i| i.pointer.any_click())
            && !template_btn.hovered()
            && let Some(pos) = ui.input(|i| i.pointer.interact_pos())
            && !menu_rect.contains(pos)
        {
            self.template_menu_open = false;
        }
    }
}

/// Sixty deterministic 15-minute bars around 100. The preview never reads the host chart.
fn preview_chart(symbol: &str) -> crate::widget::Chart {
    use crate::model::{Bar, BarData};
    let start = chrono::DateTime::from_timestamp(1_700_000_000, 0).unwrap();
    let mut price = 100.0;
    let mut state = 1u32;
    let mut bars = Vec::with_capacity(60);
    for index in 0..60 {
        state = state.wrapping_mul(1_664_525).wrapping_add(1_013_904_223);
        let delta = ((state >> 16) % 7) as f64 - 3.0;
        let open = price;
        price = (price + delta * 0.35).max(1.0);
        let high = open.max(price) + 0.8;
        let low = (open.min(price) - 0.8).max(0.1);
        bars.push(Bar::new(
            start + chrono::Duration::minutes(15 * index),
            open,
            high,
            low,
            price,
            100.0,
        ));
    }
    let mut chart = crate::widget::Chart::new(BarData::from_bars(bars));
    chart.symbol = symbol.to_owned();
    chart.chart_options.keyboard.enabled = false;
    chart.chart_options.animations.autoscale_ms = Some(0);
    chart.chart_options.animations.scroll_ms = Some(0);
    chart.chart_options.animations.price_zoom_ms = Some(0);
    chart
}

#[cfg(test)]
mod tests {
    use super::*;
    use egui::Pos2;

    #[test]
    fn enabled_sound_controls_offer_a_preview_without_applying_settings() {
        let ctx = Context::default();
        let mut settings = ChartSettingsState::default();
        settings.trading.play_execution_sound = true;
        settings.trading.execution_sound = "Bell".into();
        settings.trading.execution_sound_volume = 0.4;
        let mut dialog = SettingsDialog::new();
        dialog.execution_sounds = vec!["Chime".into(), "Bell".into()];
        dialog.open(settings.clone());
        dialog.active_tab = SettingsTab::Trading;
        let mut preview = None;
        let mut action = SettingsAction::None;
        for step in 0..5 {
            let events = match (step, preview) {
                (3, Some(pos)) | (4, Some(pos)) => vec![
                    egui::Event::PointerMoved(pos),
                    egui::Event::PointerButton {
                        pos,
                        pressed: step == 3,
                        button: egui::PointerButton::Primary,
                        modifiers: Default::default(),
                    },
                ],
                _ => vec![],
            };
            let output = ctx.run_ui(
                egui::RawInput {
                    screen_rect: Some(Rect::from_min_size(Pos2::ZERO, Vec2::new(1440.0, 900.0))),
                    events,
                    ..Default::default()
                },
                |ui| action = dialog.show(ui.ctx(), &Theme::light()),
            );
            preview = output.shapes.iter().find_map(|shape| match &shape.shape {
                egui::Shape::Text(text) if text.galley.text() == "Test sound" => {
                    Some(text.pos + text.galley.size() / 2.0)
                }
                _ => None,
            });
            output.drop_without_applying_deltas();
            if step >= 2 {
                assert!(
                    preview.is_some(),
                    "an enabled sound needs a local preview control"
                );
            }
        }
        assert!(matches!(
            action,
            SettingsAction::PreviewSound { name, volume } if name == "Bell" && volume == 0.4
        ));
        assert!(dialog.is_open);
        assert_eq!(dialog.settings(), &settings);
    }

    #[test]
    fn symbol_controls_follow_the_chart_type_and_retain_its_parameters() {
        use crate::{
            Chart,
            model::{BarData, ChartType, PriceSource},
        };
        for (kind, label) in [
            (ChartType::Heikin, "Body"),
            (ChartType::Bars, "Body"),
            (ChartType::Line, "Line width"),
            (ChartType::Baseline, "Base level"),
            (ChartType::Renko, "Size method"),
            (ChartType::Range, "Size method"),
        ] {
            let mut chart = Chart::new(BarData::new());
            chart.set_chart_type(kind);
            let mut params = chart.chart_type_params().clone();
            params.styles.line.width = 4.5;
            params.renko_brick_size = 7.0;
            chart.set_chart_type_params(params.clone());
            chart.config_mut().price_source = PriceSource::HL2;
            let settings = ChartSettingsState::from_chart(&chart);
            let mut dialog = SettingsDialog::new();
            dialog.open(settings.clone());
            let ctx = Context::default();
            let output = ctx.run_ui(Default::default(), |ui| {
                dialog.draw_symbol_tab(ui, &Theme::light())
            });
            let has_label = output.shapes.iter().any(|shape| {
                matches!(&shape.shape,
                egui::Shape::Text(text) if text.galley.text() == label)
            });
            let candle_colors = output.shapes.iter().any(|shape| {
                matches!(&shape.shape,
                egui::Shape::Text(text) if text.galley.text() == "CANDLES")
            });
            output.drop_without_applying_deltas();
            assert!(has_label, "{kind:?} must offer {label}");
            assert!(!candle_colors);
            let mut restored = Chart::new(BarData::new());
            restored.set_chart_type(kind);
            settings.apply_to_chart(&mut restored).unwrap();
            assert_eq!(restored.chart_type_params(), &params);
            assert_eq!(restored.config().price_source, PriceSource::HL2);
        }
    }

    #[test]
    fn offered_time_zones_and_session_presets_are_available() {
        use crate::config::SessionConfig;
        let mut zones = SettingsDialog::new().timezones;
        zones.extend(
            [
                SessionConfig::default(),
                SessionConfig::nyse(),
                SessionConfig::lse(),
                SessionConfig::tse(),
                SessionConfig::crypto_24_7(),
                SessionConfig::forex(),
            ]
            .map(|session| session.timezone),
        );
        for zone in zones {
            assert!(zone.parse::<crate::model::TimezoneId>().is_ok(), "{zone}");
        }
    }

    #[test]
    fn save_as_prompts_for_a_name_without_saving_automatically() {
        let ctx = Context::default();
        let mut dialog = SettingsDialog::new();
        let mut frame = |events| {
            let mut action = SettingsAction::None;
            let output = ctx.run_ui(
                egui::RawInput {
                    events,
                    ..Default::default()
                },
                |ui| {
                    action = dialog.draw_save_as_button(ui, SettingsAction::None);
                },
            );
            let save_as = output.shapes.iter().find_map(|shape| match &shape.shape {
                egui::Shape::Text(text) if text.galley.text() == "Save as..." => {
                    Some(text.pos + text.galley.size() / 2.0)
                }
                _ => None,
            });
            let prompt = output.shapes.iter().any(|shape| matches!(&shape.shape, egui::Shape::Text(text) if text.galley.text() == "Template name"));
            output.drop_without_applying_deltas();
            assert!(
                matches!(action, SettingsAction::None),
                "opening the name form must not save a template"
            );
            (save_as, prompt)
        };
        let pos = frame(vec![]).0.unwrap();
        frame(vec![egui::Event::PointerMoved(pos)]);
        for pressed in [true, false] {
            frame(vec![egui::Event::PointerButton {
                pos,
                pressed,
                button: egui::PointerButton::Primary,
                modifiers: Default::default(),
            }]);
        }
        assert!(frame(vec![]).1);
    }

    #[test]
    fn grid_colour_pickers_follow_the_visible_directions() {
        use crate::widget::settings::GridLinesMode;
        for theme in [Theme::light(), Theme::dark()] {
            for (mode, horizontal, vertical) in [
                (GridLinesMode::None, false, false),
                (GridLinesMode::Horizontal, true, false),
                (GridLinesMode::Vertical, false, true),
                (GridLinesMode::Both, true, true),
            ] {
                let ctx = Context::default();
                crate::theme::apply_to_egui(&ctx, &theme);
                let mut settings = ChartSettingsState::default();
                settings.grid_lines.mode = mode;
                settings.grid_lines.horizontal_color = Color32::from_rgb(151, 32, 19);
                settings.grid_lines.vertical_color = Color32::from_rgb(29, 43, 157);
                let mut dialog = SettingsDialog::new();
                dialog.open(settings.clone());
                let output = ctx.run_ui(Default::default(), |ui| {
                    dialog.draw_canvas_tab(ui, &theme);
                });
                let shown = [settings.grid_lines.horizontal_color, settings.grid_lines.vertical_color].map(|colour| output.shapes.iter().any(|shape| matches!(&shape.shape, egui::Shape::Rect(rect) if rect.fill == colour)));
                output.drop_without_applying_deltas();
                assert_eq!(shown, [horizontal, vertical], "{mode:?}");
            }
        }
    }

    #[test]
    fn settings_tracks_theme_without_changing_defaults_until_apply() {
        for (width, apply) in [
            (1440.0, false),
            (480.0, false),
            (1440.0, true),
            (480.0, true),
        ] {
            for configured in [Color32::PLACEHOLDER, Color32::from_rgb(80, 90, 100)] {
                let ctx = Context::default();
                let mut dialog = SettingsDialog::new();
                let mut settings = ChartSettingsState::default();
                settings.chart_basic_styles.background_color = configured;
                settings.margins.top_percent = 12.5;
                settings.margins.bottom_percent = 7.5;

                dialog.open(settings.clone());
                assert_eq!(dialog.settings.margins.top_percent, 12.5);
                assert_eq!(dialog.settings.margins.bottom_percent, 7.5);
                let mut frame = |theme: &Theme, events| {
                    crate::theme::apply_to_egui(&ctx, theme);
                    let mut action = SettingsAction::None;
                    let output = ctx.run_ui(
                        egui::RawInput {
                            screen_rect: Some(Rect::from_min_size(
                                Pos2::ZERO,
                                Vec2::new(width, 900.0),
                            )),
                            events,
                            ..Default::default()
                        },
                        |ui| {
                            action = dialog.show(ui.ctx(), theme);
                        },
                    );
                    let symbol = output.shapes.iter().any(|shape| matches!(&shape.shape, egui::Shape::Text(text) if text.galley.text() == "Body"));
                    let ok = output.shapes.iter().find_map(|shape| match &shape.shape {
                        egui::Shape::Text(text) if text.galley.text() == "Ok" => {
                            Some(text.pos + text.galley.size() / 2.0)
                        }
                        _ => None,
                    });
                    output.drop_without_applying_deltas();
                    assert_eq!(dialog.settings, settings);
                    assert_eq!(
                        dialog.color_picker_states.background.sel_color,
                        if configured == Color32::PLACEHOLDER {
                            theme.chart.bg
                        } else {
                            configured
                        }
                    );
                    assert_eq!(
                        dialog.color_picker_states.background_gradient_top.sel_color,
                        theme.chart.bg
                    );
                    assert_eq!(
                        dialog
                            .color_picker_states
                            .background_gradient_bottom
                            .sel_color,
                        theme.chart.bg_axis
                    );
                    (action, symbol, ok)
                };
                for theme in [Theme::light(), Theme::dark(), Theme::light()] {
                    frame(&theme, vec![]);
                    assert!(
                        frame(&theme, vec![]).1,
                        "settings starts with the symbol controls"
                    );
                }
                let action = if apply {
                    let pos = frame(&Theme::light(), vec![]).2.unwrap();
                    let button = |pressed| egui::Event::PointerButton {
                        pos,
                        button: egui::PointerButton::Primary,
                        pressed,
                        modifiers: Default::default(),
                    };
                    frame(&Theme::light(), vec![egui::Event::PointerMoved(pos)]);
                    frame(&Theme::light(), vec![button(true)]);
                    frame(&Theme::light(), vec![button(false)]).0
                } else {
                    frame(
                        &Theme::light(),
                        vec![egui::Event::Key {
                            key: egui::Key::Escape,
                            physical_key: None,
                            pressed: true,
                            repeat: false,
                            modifiers: Default::default(),
                        }],
                    )
                    .0
                };
                if apply {
                    let SettingsAction::Apply(saved) = action else {
                        panic!("expected Apply");
                    };
                    let expected = if configured == Color32::PLACEHOLDER {
                        Theme::light().chart.bg
                    } else {
                        configured
                    };
                    assert_eq!(saved.chart_basic_styles.background_color, expected);
                    assert_eq!(
                        saved.chart_basic_styles.background_gradient_top,
                        Theme::light().chart.bg
                    );
                    assert_eq!(
                        saved.chart_basic_styles.background_gradient_bottom,
                        Theme::light().chart.bg_axis
                    );
                } else {
                    assert!(
                        matches!(action, SettingsAction::Cancel),
                        "escape should cancel, got {action:?}"
                    );
                }
                assert!(!dialog.is_open);
                assert_eq!(dialog.settings, settings);
            }
        }
    }
    #[test]
    fn scale_controls_show_small_ratios_and_keep_modes_exclusive() {
        let ctx = Context::default();
        let mut dialog = SettingsDialog::new();
        let theme = Theme::light();
        for (label, expected, ratio) in [
            ("Log scale", (true, false), 0.005),
            ("Percentage scale", (false, true), 0.00000001),
            ("Log scale", (true, false), 1.0),
        ] {
            dialog.settings.scales_and_lines.price_to_bar_ratio = ratio;
            dialog.settings.scales_and_lines.lock_price_to_bar_ratio = true;
            let mut frame =
                |events| {
                    let output = ctx.run_ui(
                        egui::RawInput {
                            events,
                            ..Default::default()
                        },
                        |ui| {
                            dialog.draw_scales_and_lines_tab(ui, &theme);
                        },
                    );
                    let ratio_shown = output.shapes.iter().any(|shape| matches!(&shape.shape,
                        egui::Shape::Text(text) if text.galley.text().parse::<f64>() == Ok(ratio)
                    ));
                    let pos = output
                        .shapes
                        .iter()
                        .find_map(|shape| match &shape.shape {
                            egui::Shape::Text(text) if text.galley.text() == label => {
                                Some(text.pos + text.galley.size() / 2.0)
                            }
                            _ => None,
                        })
                        .unwrap();
                    output.drop_without_applying_deltas();
                    assert!(ratio_shown, "entered ratio {ratio} must remain legible");
                    pos
                };
            let pos = frame(vec![]);
            frame(vec![egui::Event::PointerMoved(pos)]);
            for pressed in [true, false] {
                frame(vec![egui::Event::PointerButton {
                    pos,
                    pressed,
                    button: egui::PointerButton::Primary,
                    modifiers: Default::default(),
                }]);
            }
            assert_eq!(
                (
                    dialog.settings.scales_and_lines.log_scale,
                    dialog.settings.scales_and_lines.percentage_scale
                ),
                expected
            );
        }
    }

    #[test]
    fn changing_settings_tabs_stays_centered_and_fits_the_window() {
        let mut scaled = Theme::light();
        scaled.text.small = 17.0;
        scaled.spacing.lg = 12.0;
        scaled.spacing.xxxl = 32.0;
        for theme in [Theme::light(), Theme::dark(), scaled] {
            for size in [
                Vec2::new(1440.0, 900.0),
                Vec2::new(600.0, 400.0),
                Vec2::new(320.0, 450.0),
            ] {
                let ctx = Context::default();
                crate::theme::apply_to_egui(&ctx, &theme);
                let mut dialog = SettingsDialog::new();
                dialog.open(ChartSettingsState::default());
                dialog.template_name = "Long template name ".repeat(20);
                let mut rects = Vec::new();
                for tab in SettingsTab::all() {
                    dialog.active_tab = *tab;
                    for pass in 0..2 {
                        let output = ctx.run_ui(
                            egui::RawInput {
                                screen_rect: Some(Rect::from_min_size(Pos2::ZERO, size)),
                                ..Default::default()
                            },
                            |ui| {
                                dialog.show(ui.ctx(), &theme);
                            },
                        );
                        let window = Rect::from_min_size(Pos2::ZERO, size);
                        let labels_fit = SettingsTab::all().iter().all(|tab| {
                            output.shapes.iter().any(|shape| match &shape.shape {
                                egui::Shape::Text(text) if text.galley.text() == tab.name() => {
                                    let label = Rect::from_min_size(text.pos, text.galley.size());
                                    window.contains_rect(label)
                                }
                                _ => false,
                            })
                        });
                        let heading = output.shapes.iter().find_map(|shape| match &shape.shape {
                            egui::Shape::Text(text) if text.galley.text() == "CANDLES" => {
                                Some(text.galley.job.sections[0].format.font_id.size)
                            }
                            _ => None,
                        });
                        output.drop_without_applying_deltas();
                        if pass == 1 {
                            assert!(labels_fit, "tab labels must be fully visible at {size:?}");
                        }
                        if *tab == SettingsTab::Symbol && pass == 1 {
                            assert_eq!(heading, Some(theme.text.small));
                        }
                    }
                    let rect = ctx
                        .memory(|memory| memory.area_rect(egui::Id::unique("chart_settings")))
                        .unwrap();
                    assert!(
                        Rect::from_min_size(Pos2::ZERO, size).contains_rect(rect),
                        "{tab:?}: {rect:?} in {size:?}"
                    );
                    rects.push(rect);
                }
                assert!(
                    rects.iter().all(|rect| {
                        (rect.min - rects[0].min).length() < 1.0
                            && (rect.size() - rects[0].size()).length() < 1.0
                    }),
                    "{rects:?}"
                );
                assert_eq!(dialog.preview.is_some(), size.x >= 1440.0);
            }
        }
    }

    #[test]
    fn clicking_a_tab_shows_that_tab_in_the_same_frame() {
        let ctx = Context::default();
        let theme = Theme::light();
        crate::theme::apply_to_egui(&ctx, &theme);
        let mut dialog = SettingsDialog::new();
        dialog.open(ChartSettingsState::default());
        let screen = Rect::from_min_size(Pos2::ZERO, Vec2::new(1440.0, 900.0));
        let mut frame = |events| {
            let mut action = SettingsAction::None;
            let output = ctx.run_ui(
                egui::RawInput {
                    screen_rect: Some(screen),
                    events,
                    ..Default::default()
                },
                |ui| {
                    action = dialog.show(ui.ctx(), &theme);
                },
            );
            let canvas = output.shapes.iter().find_map(|shape| match &shape.shape {
                egui::Shape::Text(text) if text.galley.text() == "Canvas" => {
                    Some(text.pos + text.galley.size() / 2.0)
                }
                _ => None,
            });
            let canvas_content = output.shapes.iter().any(|shape| {
                matches!(&shape.shape, egui::Shape::Text(text) if text.galley.text() == "CHART BASIC STYLES")
            });
            output.drop_without_applying_deltas();
            (action, canvas, canvas_content)
        };
        frame(vec![]);
        let pos = frame(vec![]).1.expect("Canvas tab");
        let click = |pressed| egui::Event::PointerButton {
            pos,
            button: egui::PointerButton::Primary,
            pressed,
            modifiers: Default::default(),
        };
        frame(vec![egui::Event::PointerMoved(pos)]);
        frame(vec![click(true)]);
        let (_, _, content) = frame(vec![click(false)]);
        assert!(content, "Canvas content is drawn in the click frame");
        assert!(matches!(frame(vec![]).0, SettingsAction::None));
    }

    #[test]
    fn preview_follows_edits_and_hides_on_a_narrow_window() {
        let ctx = Context::default();
        let theme = Theme::light();
        crate::theme::apply_to_egui(&ctx, &theme);
        let mut dialog = SettingsDialog::new();
        dialog.open(ChartSettingsState::default());
        let red = Color32::from_rgb(200, 16, 16);
        dialog.settings.candle_colors.body_up = red;
        let mut action = SettingsAction::None;
        ctx.run_ui(
            egui::RawInput {
                screen_rect: Some(Rect::from_min_size(Pos2::ZERO, Vec2::new(1440.0, 900.0))),
                ..Default::default()
            },
            |ui| {
                action = dialog.show(ui.ctx(), &theme);
            },
        )
        .drop_without_applying_deltas();
        assert!(matches!(action, SettingsAction::None));
        assert_eq!(dialog.preview.as_ref().unwrap().config().bullish_color, red);
        let mut narrow = SettingsDialog::new();
        narrow.open(ChartSettingsState::default());
        ctx.run_ui(
            egui::RawInput {
                screen_rect: Some(Rect::from_min_size(Pos2::ZERO, Vec2::new(320.0, 450.0))),
                ..Default::default()
            },
            |ui| {
                narrow.show(ui.ctx(), &theme);
            },
        )
        .drop_without_applying_deltas();
        assert!(narrow.preview.is_none());
    }

    #[test]
    fn precision_default_follows_the_next_instrument() {
        use crate::model::{BarData, Instrument};
        use crate::widget::settings::PrecisionMode;
        let mut chart = crate::widget::Chart::new(BarData::new());
        let mut instrument = Instrument::new("TEST");
        instrument.price_decimals = Some(2);
        chart.set_instrument(Some(instrument.clone()));
        let state = ChartSettingsState::from_chart(&chart);
        assert_eq!(state.precision, PrecisionMode::Default);
        state.apply_to_chart(&mut chart).unwrap();
        assert_eq!(
            ChartSettingsState::from_chart(&chart).precision,
            PrecisionMode::Default
        );
        instrument.price_decimals = Some(5);
        chart.set_instrument(Some(instrument));
        assert_eq!(
            chart
                .state()
                .price_scale()
                .map(Default::default(), 0.0, 1.0)
                .price_precision(),
            Some(5)
        );
        assert!(chart.state().price_scale().precision_follows_instrument());
    }

    #[test]
    fn dark_theme_waits_for_ok_and_cancel_discards_it() {
        let ctx = Context::default();
        let light = Theme::light();
        crate::theme::apply_to_egui(&ctx, &light);
        let mut settings = ChartSettingsState::default();
        settings.chart_basic_styles.background_color = light.chart.bg;
        settings.grid_lines.horizontal_color = light.chart.grid_line;
        settings.grid_lines.vertical_color = light.chart.grid_line;
        settings.scales_appearance.text_color = light.chart.axis_text;
        settings.crosshair.color = light.chart.crosshair_line;
        settings.watermark.color = light.chart.grid_line;
        let screen = Rect::from_min_size(Pos2::ZERO, Vec2::new(1440.0, 900.0));
        let click = |pos, pressed| egui::Event::PointerButton {
            pos,
            button: egui::PointerButton::Primary,
            pressed,
            modifiers: Default::default(),
        };
        let show = |dialog: &mut SettingsDialog, events| {
            let mut action = SettingsAction::None;
            let output = ctx.run_ui(
                egui::RawInput {
                    screen_rect: Some(screen),
                    events,
                    ..Default::default()
                },
                |ui| {
                    action = dialog.show(ui.ctx(), &light);
                },
            );
            let at = |label: &str| {
                output.shapes.iter().find_map(|shape| match &shape.shape {
                    egui::Shape::Text(text) if text.galley.text() == label => {
                        Some(text.pos + text.galley.size() / 2.0)
                    }
                    _ => None,
                })
            };
            let found = (at("Dark theme"), at("Cancel"), at("Ok"));
            output.drop_without_applying_deltas();
            (action, found.0, found.1, found.2)
        };
        let mut dialog = SettingsDialog::new();
        dialog.open(settings.clone());
        dialog.active_tab = SettingsTab::Canvas;
        show(&mut dialog, vec![]);
        let dark = show(&mut dialog, vec![]).1.unwrap();
        show(&mut dialog, vec![egui::Event::PointerMoved(dark)]);
        show(&mut dialog, vec![click(dark, true)]);
        show(&mut dialog, vec![click(dark, false)]);
        let cancel = show(&mut dialog, vec![]).2.unwrap();
        show(&mut dialog, vec![egui::Event::PointerMoved(cancel)]);
        show(&mut dialog, vec![click(cancel, true)]);
        let (action, _, _, _) = show(&mut dialog, vec![click(cancel, false)]);
        assert!(matches!(action, SettingsAction::Cancel));
        dialog.open(settings);
        dialog.active_tab = SettingsTab::Canvas;
        show(&mut dialog, vec![]);
        let dark = show(&mut dialog, vec![]).1.unwrap();
        show(&mut dialog, vec![click(dark, true)]);
        show(&mut dialog, vec![click(dark, false)]);
        let ok = show(&mut dialog, vec![]).3.unwrap();
        show(&mut dialog, vec![click(ok, true)]);
        let (action, _, _, _) = show(&mut dialog, vec![click(ok, false)]);
        assert!(matches!(action, SettingsAction::SetDarkTheme(true)));
        let saved = dialog.take_committed().unwrap();
        let dark_theme = Theme::dark();
        assert_eq!(
            saved.grid_lines.horizontal_color,
            dark_theme.chart.grid_line
        );
        assert_eq!(
            saved.scales_appearance.text_color,
            dark_theme.chart.axis_text
        );
    }

    #[test]
    fn bid_ask_and_high_low_apply_and_buy_sell_is_only_on_trading() {
        let ctx = Context::default();
        let theme = Theme::light();
        crate::theme::apply_to_egui(&ctx, &theme);
        let mut dialog = SettingsDialog::new();
        dialog.open(ChartSettingsState::default());
        let count = |dialog: &mut SettingsDialog, trading: bool| {
            let output = ctx.run_ui(Default::default(), |ui| {
                if trading {
                    dialog.draw_trading_tab(ui, &theme);
                } else {
                    dialog.draw_status_line_tab(ui, &theme);
                }
            });
            let found = output.shapes.iter().filter(|shape| {
                matches!(&shape.shape, egui::Shape::Text(text) if text.galley.text() == "Buy/sell buttons")
            }).count();
            output.drop_without_applying_deltas();
            found
        };
        assert_eq!(count(&mut dialog, false), 0);
        assert_eq!(count(&mut dialog, true), 1);
        dialog.settings.scales_and_lines.show_bid_ask = true;
        dialog.settings.scales_and_lines.show_high_low = true;
        let mut chart = crate::widget::Chart::new(crate::model::BarData::new());
        dialog
            .settings()
            .clone()
            .apply_to_chart(&mut chart)
            .unwrap();
        assert!(chart.config().show_bid_ask);
        assert!(chart.config().show_high_low);
    }

    #[test]
    fn timezone_keeps_local_exchange_and_the_current_value() {
        let mut dialog = SettingsDialog::new();
        dialog.open(ChartSettingsState::default());
        dialog.settings.timezone = "Local".into();
        assert!(dialog.timezone_choices().iter().any(|zone| zone == "Local"));
        dialog.settings.exchange_timezone = Some("America/New_York".into());
        dialog.settings.timezone = "Exchange:America/New_York".into();
        assert!(
            dialog
                .timezone_choices()
                .iter()
                .any(|zone| zone == "Exchange:America/New_York")
        );
        dialog.settings.timezone = "Pacific/Auckland".into();
        assert!(
            dialog
                .timezone_choices()
                .iter()
                .any(|zone| zone == "Pacific/Auckland")
        );
    }

    #[test]
    fn vertical_grid_keeps_the_horizontal_colour() {
        use crate::widget::settings::GridLinesMode;
        let mut chart = crate::widget::Chart::new(crate::model::BarData::new());
        let original = chart.config().grid_color;
        let mut state = ChartSettingsState::from_chart(&chart);
        state.grid_lines.mode = GridLinesMode::Vertical;
        state.grid_lines.horizontal_color = Color32::from_rgb(200, 16, 16);
        state.apply_to_chart(&mut chart).unwrap();
        assert_eq!(chart.config().grid_color, original);
        state.grid_lines.mode = GridLinesMode::Horizontal;
        state.apply_to_chart(&mut chart).unwrap();
        assert_eq!(chart.config().grid_color, Color32::from_rgb(200, 16, 16));
    }

    #[test]
    fn margins_must_leave_room_for_prices() {
        let mut state = ChartSettingsState::default();
        state.margins.top_percent = 60.0;
        state.margins.bottom_percent = 50.0;
        assert!(state.validate().is_err());
    }
}
