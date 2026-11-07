//! Filter Widget
//!
//! A text input with filtering capabilities for lists.
//!
//! # Usage
//!
//! ```
//! use egui_charts::ui_kit::filter::FilterWidget;
//!
//! // Create filter widget
//! let mut filter = FilterWidget::new();
//! # let items = ["AAPL", "MSFT", "NVDA"];
//!
//! # egui::__run_test_ui(|ui| {
//! // Show the filter input
//! filter.show(ui);
//!
//! // Filter your items
//! for item in items.iter().filter(|name| filter.matches(name)) {
//!     ui.label(*item);
//! }
//! # });
//! ```

use crate::localization as l10n;
use crate::theme::ThemeExt;
use egui::{Margin, Rect, Response, TextEdit, Ui, Vec2, Widget, pos2};

use crate::icons::icons;
use crate::tokens::DESIGN_TOKENS;

/// Response from showing a FilterWidget
pub struct FilterResponse {
    /// The underlying egui Response
    pub response: Response,
    /// Whether the query changed this frame
    pub changed: bool,
}

/// A text input with filtering capabilities
#[derive(Default, Clone)]
pub struct FilterWidget {
    query: String,
    placeholder: String,
    show_clear_button: bool,
    show_icon: bool,
}

impl FilterWidget {
    /// Create a new filter widget
    pub fn new() -> Self {
        Self {
            query: String::new(),
            placeholder: "Filter...".to_string(),
            show_clear_button: true,
            show_icon: true,
        }
    }

    /// Create a filter widget with initial query
    pub fn with_query(query: impl Into<String>) -> Self {
        Self {
            query: query.into(),
            ..Default::default()
        }
    }

    /// Set the placeholder text
    pub fn placeholder(mut self, text: impl Into<String>) -> Self {
        self.placeholder = text.into();
        self
    }

    /// Set whether to show the clear button
    pub fn show_clear_button(mut self, show: bool) -> Self {
        self.show_clear_button = show;
        self
    }

    /// Set whether to show the search icon
    pub fn show_icon(mut self, show: bool) -> Self {
        self.show_icon = show;
        self
    }

    /// Get the current query
    pub fn query(&self) -> &str {
        &self.query
    }

    /// Get mutable access to the query
    pub fn query_mut(&mut self) -> &mut String {
        &mut self.query
    }

    /// Clear the query
    pub fn clear(&mut self) {
        self.query.clear();
    }

    /// Check if an item matches the current filter (case-insensitive)
    pub fn matches(&self, text: &str) -> bool {
        if self.query.is_empty() {
            return true;
        }
        text.to_lowercase().contains(&self.query.to_lowercase())
    }

    /// Check if the filter is empty
    pub fn is_empty(&self) -> bool {
        self.query.is_empty()
    }

    /// Show the filter widget
    pub fn show(&mut self, ui: &mut Ui) -> FilterResponse {
        let old_query = self.query.clone();
        let response = search_field(
            ui,
            &mut self.query,
            &self.placeholder,
            self.show_icon,
            self.show_clear_button,
        );
        FilterResponse {
            response,
            changed: self.query != old_query,
        }
    }
}

/// Stateless filter input that operates on an external string
pub fn filter_input(ui: &mut Ui, query: &mut String) -> FilterResponse {
    let old_query = query.clone();
    let response = search_field(ui, query, "Filter...", true, true);
    FilterResponse {
        response,
        changed: *query != old_query,
    }
}

/// A full-width text field with the search icon and the clear button inside its frame.
/// Escape clears the query while the field has focus.
fn search_field(
    ui: &mut Ui,
    query: &mut String,
    placeholder: &str,
    show_icon: bool,
    show_clear: bool,
) -> Response {
    let theme = ui.active_theme();
    let icon_size = DESIGN_TOKENS.sizing.icon_sm;
    let inset = theme.spacing.lg;
    let icon_slot = inset + icon_size + theme.spacing.sm;
    let tint = theme.ui.text_muted;
    let font = ui
        .style()
        .override_font_id
        .clone()
        .unwrap_or_else(|| egui::TextStyle::Body.resolve(ui.style()));
    let row = ui
        .painter()
        .layout_no_wrap("Ag".into(), font.clone(), tint)
        .size()
        .y;
    let target = ui.spacing().interact_size.y;
    // Floor the padding so the frame stays within `interact_size` for the shorter filters.
    let pad_y = ((target - row) / 2.0).floor().max(0.0);
    let margin_px = |value: f32| value.round().clamp(0.0, 127.0) as i8;

    let response = ui
        .scope(|ui| {
            // The field keeps its size when hovered or focused.
            let widgets = &mut ui.visuals_mut().widgets;
            widgets.hovered.expansion = 0.0;
            widgets.active.expansion = 0.0;

            let margin = |icon: bool| margin_px(if icon { icon_slot } else { inset });
            TextEdit::singleline(query)
                .font(font)
                .hint_text(l10n::text(ui.ctx(), placeholder))
                .desired_width(f32::INFINITY)
                .min_size(Vec2::new(0.0, ui.spacing().interact_size.y))
                .margin(Margin {
                    left: margin(show_icon),
                    // Keep the gap while the query is empty so text does not jump.
                    right: margin(show_clear),
                    top: margin_px(pad_y),
                    bottom: margin_px(pad_y),
                })
                .ui(ui)
        })
        .inner
        .accessible_name(l10n::text(ui.ctx(), placeholder));

    let icon_rect = |center_x: f32| {
        Rect::from_center_size(
            pos2(center_x, response.rect.center().y),
            Vec2::splat(icon_size),
        )
    };
    if show_icon {
        icons::QUICK_SEARCH
            .as_image_tinted(Vec2::splat(icon_size), tint)
            .paint_at(
                ui,
                icon_rect(response.rect.left() + inset + icon_size / 2.0),
            );
    }
    if show_clear && !query.is_empty() {
        let clear_size = icon_size + 2.0 * theme.spacing.xs;
        let rect = Rect::from_center_size(
            pos2(
                response.rect.right() - inset - icon_size / 2.0,
                response.rect.center().y,
            ),
            Vec2::splat(clear_size),
        );
        let clear = ui.interact(rect, response.id.with("clear"), egui::Sense::click());
        icons::CLOSE
            .as_image_tinted(Vec2::splat(icon_size), tint)
            .paint_at(ui, icon_rect(rect.center().x));
        if clear.clicked() {
            query.clear();
        }
        clear.on_hover_text(l10n::text(ui.ctx(), "Clear filter"));
    }

    if (response.has_focus() || ui.memory(|m| m.had_focus_last_frame(response.id)))
        && !query.is_empty()
        && ui.input_mut(|i| i.consume_key(egui::Modifiers::NONE, egui::Key::Escape))
    {
        query.clear();
    }

    response
}

#[cfg(test)]
mod tests {
    use super::*;
    use egui::{Context, Event, Key, Modifiers, RawInput};

    #[test]
    fn escape_clears_only_the_focused_filter() {
        for (focused, moved) in [(false, false), (true, false), (false, true)] {
            let ctx = Context::default();
            let mut query = "RSI".to_owned();
            let mut id = egui::Id::NULL;
            ctx.run_ui(RawInput::default(), |ui| {
                id = filter_input(ui, &mut query).response.id;
            })
            .drop_without_applying_deltas();
            if focused || moved {
                ctx.memory_mut(|m| m.request_focus(id));
                ctx.run_ui(RawInput::default(), |ui| {
                    filter_input(ui, &mut query);
                })
                .drop_without_applying_deltas();
            }
            if moved {
                ctx.memory_mut(|m| m.request_focus(egui::Id::unique("another field")));
            }
            let input = RawInput {
                events: vec![Event::Key {
                    key: Key::Escape,
                    physical_key: None,
                    pressed: true,
                    repeat: false,
                    modifiers: Modifiers::NONE,
                }],
                ..Default::default()
            };
            ctx.run_ui(input, |ui| {
                filter_input(ui, &mut query);
            })
            .drop_without_applying_deltas();
            assert_eq!(
                query,
                if focused { "" } else { "RSI" },
                "focused: {focused}, moved: {moved}"
            );
        }
    }

    /// The heading-sized field centres its painted placeholder on the magnifier.
    #[test]
    fn text_and_icons_share_a_centre_line() {
        let ctx = Context::default();
        crate::icons::install_loader(&ctx);
        let mut query = String::new();
        let mut field_center = 0.0;
        let input = RawInput {
            screen_rect: Some(egui::Rect::from_min_size(
                egui::Pos2::ZERO,
                egui::vec2(400.0, 80.0),
            )),
            ..Default::default()
        };
        let mut output = ctx.run_ui(input.clone(), |_| {});
        for _ in 0..6 {
            output.textures_delta.clear();
            output = ctx.run_ui(input.clone(), |ui| {
                ui.spacing_mut().interact_size.y = 40.0;
                ui.style_mut().override_font_id = Some(egui::FontId::proportional(18.0));
                field_center = search_field(ui, &mut query, "Search indicators", true, true)
                    .rect
                    .center()
                    .y;
            });
        }
        let mut text_center = None;
        let mut icon_center = None;
        let mut icon_x = f32::MAX;
        fn walk(
            shape: egui::Shape,
            text: &mut Option<f32>,
            icon: &mut Option<f32>,
            icon_x: &mut f32,
        ) {
            match shape {
                egui::Shape::Text(painted)
                    if painted.galley.text().contains("Search indicators") =>
                {
                    *text = Some(painted.pos.y + painted.galley.size().y / 2.0);
                }
                egui::Shape::Callback(callback) if callback.rect.left() < *icon_x => {
                    *icon_x = callback.rect.left();
                    *icon = Some(callback.rect.center().y);
                }
                egui::Shape::Mesh(mesh) => {
                    let (mut min_y, mut max_y, mut min_x) = (f32::MAX, f32::MIN, f32::MAX);
                    for vertex in &mesh.vertices {
                        min_y = min_y.min(vertex.pos.y);
                        max_y = max_y.max(vertex.pos.y);
                        min_x = min_x.min(vertex.pos.x);
                    }
                    if max_y > min_y && max_y - min_y < 48.0 && min_x < *icon_x {
                        *icon_x = min_x;
                        *icon = Some((min_y + max_y) / 2.0);
                    }
                }
                egui::Shape::Vec(shapes) => {
                    for shape in shapes {
                        walk(shape, text, icon, icon_x);
                    }
                }
                _ => {}
            }
        }
        for clipped in std::mem::take(&mut output.shapes) {
            walk(
                clipped.shape,
                &mut text_center,
                &mut icon_center,
                &mut icon_x,
            );
        }
        output.textures_delta.clear();
        let text_center = text_center.expect("placeholder text");
        // The magnifier is painted at the field centre (`icon_rect` uses `response.rect.center()`).
        // Headless frames do not tessellate that SVG into a shape, so the field centre is its centre.
        let icon_center = icon_center.unwrap_or(field_center);
        assert!(
            (text_center - field_center).abs() <= 1.0,
            "text {text_center} field {field_center}"
        );
        assert!(
            (text_center - icon_center).abs() <= 1.0,
            "text {text_center} icon {icon_center}"
        );
    }
}
