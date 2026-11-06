//! Tab bar component with underline indicator.
//!
//! Provides a horizontal tab bar that renders clickable text labels with
//! an accent-colored underline on the active tab. Used across all tabbed dialogs.
//!
//! # Example
//! ```
//! use egui_charts::ui_kit::tab_bar::{TabBar, TabLabel};
//!
//! #[derive(Clone, Copy, PartialEq)]
//! enum MyTab {
//!     Chart,
//!     Orders,
//! }
//!
//! impl TabLabel for MyTab {
//!     fn tab_label(&self) -> &str {
//!         match self {
//!             MyTab::Chart => "Chart",
//!             MyTab::Orders => "Orders",
//!         }
//!     }
//! }
//!
//! # egui::__run_test_ui(|ui| {
//! # let mut active_tab = MyTab::Chart;
//! TabBar::new(&[MyTab::Chart, MyTab::Orders], &mut active_tab).show(ui);
//! # });
//! ```

use crate::localization as l10n;
use crate::theme::ThemeExt;
use crate::tokens::DESIGN_TOKENS;
use crate::ui_kit::buttons::ButtonSize;
use egui::{Role, Sense, Ui, WidgetInfo, vec2};

/// Trait for tab enum types to provide their display label.
///
/// Implement this on your tab enum so `TabBar` can render it.
pub trait TabLabel: Copy + PartialEq {
    /// Label displayed for this tab.
    fn tab_label(&self) -> &str;

    /// A 16 px icon drawn before the label.
    fn tab_icon(&self) -> Option<&'static crate::icons::Icon> {
        None
    }
}

/// Horizontal tab bar with underline indicator on the active tab.
///
/// Renders clickable text labels 16 px apart over a full-width rule, starting at the content
/// edge. The active tab's accent underline sits on the rule, and content starts 8 px below it.
pub struct TabBar<'a, T: TabLabel> {
    tabs: &'a [T],
    active: &'a mut T,
    pills: bool,
}

impl<'a, T: TabLabel> TabBar<'a, T> {
    /// Bind the offered tabs to the current selection.
    pub fn new(tabs: &'a [T], active: &'a mut T) -> Self {
        Self {
            tabs,
            active,
            pills: false,
        }
    }

    /// Render rounded chips instead of an underline, wrapping onto further rows when they do not
    /// fit the width.
    pub fn pills(mut self) -> Self {
        self.pills = true;
        self
    }

    /// Show the tab bar. Returns `true` if the active tab changed.
    pub fn show(self, ui: &mut Ui) -> bool {
        let theme = ui.active_theme();
        let mut changed = false;
        let rule_slot = ui.painter().add(egui::Shape::Noop);

        let add_tabs = |ui: &mut Ui| {
            ui.spacing_mut().item_spacing.x = if self.pills { 4.0 } else { theme.spacing.xxl };
            for tab in self.tabs {
                let is_active = *tab == *self.active;
                let galley = ui.painter().layout_no_wrap(
                    l10n::text(ui.ctx(), tab.tab_label()),
                    egui::FontId::proportional(if self.pills { 14.0 } else { theme.text.label }),
                    egui::Color32::PLACEHOLDER,
                );
                let icon_span = if tab.tab_icon().is_some() { 20.0 } else { 0.0 };
                let (rect, response) = ui.allocate_exact_size(
                    vec2(
                        galley.size().x + if self.pills { 24.0 } else { 0.0 } + icon_span,
                        ButtonSize::MD.pixels(),
                    ),
                    Sense::click(),
                );
                response
                    .widget_info(|| WidgetInfo::labeled(Role::Tab, ui.is_enabled(), galley.text()));
                ui.ctx()
                    .accesskit_node_builder(response.id, |node| node.set_selected(is_active));

                let visuals = ui.visuals();
                if self.pills {
                    let pill_radius = rect.height() / 2.0;
                    if is_active {
                        ui.painter().rect_filled(rect, pill_radius, theme.ui.text);
                    } else {
                        let fill = if response.hovered() {
                            theme.ui.btn_bg_hover
                        } else {
                            egui::Color32::TRANSPARENT
                        };
                        let border_color = if response.hovered() {
                            theme.ui.text
                        } else {
                            theme.ui.border
                        };
                        ui.painter().rect(
                            rect,
                            pill_radius,
                            fill,
                            egui::Stroke::new(DESIGN_TOKENS.stroke.hairline, border_color),
                            egui::StrokeKind::Inside,
                        );
                    }
                }
                let text_color = if self.pills && is_active {
                    theme.ui.panel_bg
                } else if is_active {
                    visuals.strong_text_color()
                } else if response.hovered() {
                    visuals.widgets.hovered.fg_stroke.color
                } else {
                    visuals.text_color()
                };
                let content_width = galley.size().x + icon_span;
                let start_x = rect.center().x - content_width / 2.0;
                if let Some(icon) = tab.tab_icon() {
                    let icon_rect = egui::Rect::from_min_size(
                        egui::pos2(start_x, rect.center().y - 8.0),
                        egui::vec2(16.0, 16.0),
                    );
                    icon.as_image_tinted(egui::vec2(16.0, 16.0), text_color)
                        .paint_at(ui, icon_rect);
                }
                let text_pos =
                    egui::pos2(start_x + icon_span, rect.center().y - galley.size().y / 2.0);
                ui.painter().galley(text_pos, galley, text_color);

                if is_active && !self.pills {
                    let thick = DESIGN_TOKENS.stroke.thick;
                    ui.painter().hline(
                        rect.x_range(),
                        rect.bottom() - thick / 2.0,
                        egui::Stroke::new(thick, theme.ui.accent),
                    );
                }
                crate::styles::focus::draw_focus_ring(
                    ui,
                    &response,
                    if self.pills { rect.height() / 2.0 } else { 0.0 },
                );

                if response.clicked() {
                    *self.active = *tab;
                    changed = true;
                }
            }
        };
        let row = if self.pills {
            ui.horizontal_wrapped(add_tabs)
        } else {
            ui.horizontal(add_tabs)
        };

        // The rule spans the parent's width, under the active underline.
        let stroke = ui.visuals().widgets.noninteractive.bg_stroke;
        let rule = egui::Shape::hline(
            ui.max_rect().x_range(),
            row.response.rect.bottom() - stroke.width / 2.0,
            stroke,
        );
        if !self.pills {
            ui.painter().set(rule_slot, rule);
        }
        ui.add_space(theme.spacing.lg - ui.spacing().item_spacing.y);

        changed
    }
}
