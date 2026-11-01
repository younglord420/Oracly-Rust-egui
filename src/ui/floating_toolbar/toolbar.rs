//! Floating Selection Toolbar Implementation
//!
//! Compact glass property bar for the selected drawing / object.

use crate::icons::icons;
use crate::localization as l10n;
use crate::model::LineStyle;
use crate::painting::dashed_line::draw_dashed_line;
use crate::theme::ThemeExt;
use crate::tokens::DESIGN_TOKENS;
use crate::ui_kit::buttons::{ButtonSize, IconButton};
use crate::ui_kit::show_palette;
use egui::{
    Color32, FontId, PopupCloseBehavior, Pos2, Rect, RectAlign, Role, Sense, Stroke, StrokeKind,
    Ui, Vec2, WidgetInfo,
};

/// Stroke widths offered in the floating width menu.
const LINE_WIDTHS: [f32; 4] = [1.0, 2.0, 3.0, 4.0];

/// Action that can be triggered from the floating toolbar
#[derive(Clone, Debug, PartialEq)]
#[non_exhaustive]
pub enum FloatingToolbarAction {
    /// Change a color slot (index, new color)
    ChangeColor(usize, Color32),
    /// Set the selected object's stroke width
    SetLineWidth(f32),
    /// Set the selected object's line style
    SetLineStyle(LineStyle),
    /// Open settings dialog
    OpenSettings,
    /// Open text / font settings for the selected object
    OpenText,
    /// Add an alert on the selected object
    AddAlert,
    /// Duplicate the selected object
    Duplicate,
    /// Delete the selected object
    Delete,
    /// Toggle lock state
    ToggleLock,
    /// Toggle visibility
    ToggleVisibility,
    /// Close the toolbar
    Close,
}

/// Configuration for color slots shown in toolbar
#[derive(Clone, Debug)]
pub struct ColorSlot {
    /// Text displayed by the component.
    pub label: &'static str,
    /// Color.
    pub color: Color32,
}

/// Floating selection toolbar state
#[derive(Clone, Debug)]
pub struct FloatingSelectionToolbar {
    /// Whether toolbar is visible
    visible: bool,
    /// Current position
    position: Pos2,
    /// Drag offset for dragging
    drag_offset: Option<Vec2>,
    /// Color slots to display (mutable for color picker)
    color_slots: Vec<ColorSlot>,
    /// Stroke width of the selected object
    stroke_width: f32,
    /// Line style of the selected object
    line_style: LineStyle,
    /// Whether object is locked
    is_locked: bool,
    /// Whether object is visible
    is_visible: bool,
    /// Whether to offer an alert on the object
    alert_available: bool,
    /// Last rendered rect (for hit testing)
    last_rect: Rect,
    /// Drawing id the toolbar was last anchored to (reposition when it changes)
    anchored_to: Option<usize>,
}

impl Default for FloatingSelectionToolbar {
    fn default() -> Self {
        Self::new()
    }
}

impl FloatingSelectionToolbar {
    /// Create a new hidden floating selection toolbar at the default position
    pub fn new() -> Self {
        Self {
            visible: false,
            position: Pos2::new(
                DESIGN_TOKENS.sizing.floating_toolbar.default_x,
                DESIGN_TOKENS.sizing.floating_toolbar.default_y,
            ),
            drag_offset: None,
            color_slots: Vec::new(),
            stroke_width: 2.0,
            line_style: LineStyle::Solid,
            is_locked: false,
            is_visible: true,
            alert_available: false,
            last_rect: Rect::NOTHING,
            anchored_to: None,
        }
    }

    /// Show the toolbar at a specific position
    pub fn show_at(&mut self, position: Pos2) {
        self.visible = true;
        self.position = position;
    }

    /// Show near a drawing: repositions when `drawing_id` changes or the bar was hidden.
    pub fn show_near_drawing(&mut self, drawing_id: usize, position: Pos2) {
        if !self.visible || self.anchored_to != Some(drawing_id) {
            self.show_at(position);
            self.anchored_to = Some(drawing_id);
        } else {
            self.visible = true;
        }
    }

    /// Hide the toolbar
    pub fn hide(&mut self) {
        self.visible = false;
        self.anchored_to = None;
    }

    /// Approximate on-screen size used to place the bar near a drawing before the first layout.
    pub fn estimated_size(&self) -> Vec2 {
        let button = DESIGN_TOKENS.sizing.floating_toolbar.button_size;
        let gap = 4.0;
        let colors = self.color_slots.len().max(1) as f32;
        Vec2::new(
            DESIGN_TOKENS.sizing.floating_toolbar.drag_handle_width
                + colors * (button + gap)
                + 56.0
                + 52.0
                + 5.0 * (button + gap)
                + 2.0 * (button + gap)
                + 48.0,
            DESIGN_TOKENS.sizing.floating_toolbar.height,
        )
    }

    /// Place the toolbar above `bounds` when possible, otherwise below; clamp to `clip`.
    pub fn anchor_rect(bounds: Rect, clip: Rect, toolbar: Vec2) -> Pos2 {
        let gap = 8.0;
        let x = (bounds.center().x - toolbar.x * 0.5)
            .min(clip.max.x - toolbar.x - 4.0)
            .max(clip.min.x + 4.0);
        let above = bounds.min.y - toolbar.y - gap;
        let y = if above >= clip.min.y + 4.0 {
            above
        } else {
            (bounds.max.y + gap)
                .min(clip.max.y - toolbar.y - 4.0)
                .max(clip.min.y + 4.0)
        };
        Pos2::new(x, y)
    }

    /// Check if toolbar is visible
    #[allow(clippy::misnamed_getters)]
    pub fn is_visible(&self) -> bool {
        self.visible
    }

    /// Check if a point is inside the toolbar area (for hit testing)
    pub fn contains_point(&self, point: Pos2) -> bool {
        self.visible && self.last_rect.contains(point)
    }

    /// Set the color slots to display
    pub fn set_colors(&mut self, slots: Vec<ColorSlot>) {
        self.color_slots = slots;
    }

    /// Get mutable reference to color slots (for updating from outside)
    pub fn color_slots_mut(&mut self) -> &mut Vec<ColorSlot> {
        &mut self.color_slots
    }

    /// Set the stroke width shown in the width chip
    pub fn set_stroke_width(&mut self, width: f32) {
        self.stroke_width = width;
    }

    /// Set the line style shown in the style chip
    pub fn set_line_style(&mut self, style: LineStyle) {
        self.line_style = style;
    }

    /// Set locked state
    pub fn set_locked(&mut self, locked: bool) {
        self.is_locked = locked;
    }

    /// Set visibility state
    pub fn set_object_visible(&mut self, visible: bool) {
        self.is_visible = visible;
    }

    /// Show the Add alert button, for an object an alert can follow, such as a drawing with
    /// [`alert_levels`](crate::drawings::Drawing::alert_levels). Hidden by default.
    pub fn set_alert_available(&mut self, available: bool) {
        self.alert_available = available;
    }

    /// Render the toolbar
    pub fn show(&mut self, ui: &mut Ui) -> Option<FloatingToolbarAction> {
        if !self.visible {
            return None;
        }

        // Handle dragging from the drag-handle zone.
        if let Some(pointer_pos) = ui.ctx().input(|i| i.pointer.interact_pos()) {
            if ui.ctx().input(|i| i.pointer.any_pressed()) && self.drag_offset.is_none() {
                let toolbar_rect = Rect::from_min_size(self.position, self.toolbar_size(ui));
                let drag_handle_rect = Rect::from_min_size(
                    toolbar_rect.min,
                    Vec2::new(
                        DESIGN_TOKENS.sizing.floating_toolbar.drag_handle_width
                            + ui.active_theme().spacing.md,
                        toolbar_rect.height(),
                    ),
                );
                if drag_handle_rect.contains(pointer_pos) {
                    self.drag_offset = Some(pointer_pos - self.position);
                }
            }

            if ui.ctx().input(|i| i.pointer.any_down())
                && let Some(offset) = self.drag_offset
            {
                self.position = pointer_pos - offset;
            }

            if ui.ctx().input(|i| i.pointer.any_released()) {
                self.drag_offset = None;
            }
        }

        // Clamp: `min` then `max` (not `f32::clamp`) so a too-small viewport pins to the min edge.
        let screen_rect = ui.ctx().input(|i| i.viewport_rect());
        let toolbar_size = self.toolbar_size(ui);
        self.position.x = self
            .position
            .x
            .min(screen_rect.max.x - toolbar_size.x)
            .max(screen_rect.min.x);
        self.position.y = self
            .position
            .y
            .min(screen_rect.max.y - toolbar_size.y)
            .max(screen_rect.min.y);

        let color_slots_len = self.color_slots.len();
        let is_locked = self.is_locked;
        let is_visible = self.is_visible;
        let alert_available = self.alert_available;
        let mut stroke_width = self.stroke_width;
        let mut line_style = self.line_style;
        let mut color_changes: Vec<(usize, Color32)> = Vec::new();
        let mut temp_colors: Vec<Color32> = self.color_slots.iter().map(|s| s.color).collect();

        let area_response = egui::Area::new(egui::Id::unique("floating_selection_toolbar"))
            .fixed_pos(self.position)
            .order(egui::Order::Foreground)
            .interactable(true)
            .show(ui.ctx(), |ui| {
                let theme = ui.active_theme();
                crate::ui_kit::glass_popup(ui)
                    .corner_radius(theme.radius.md)
                    .inner_margin(egui::Margin::symmetric(
                        theme.spacing.sm as i8,
                        theme.spacing.xs as i8,
                    ))
                    .show(ui, |ui| {
                        ui.horizontal(|ui| {
                            ui.spacing_mut().item_spacing.x = theme.spacing.xs;
                            ui.set_height(
                                DESIGN_TOKENS.sizing.floating_toolbar.height
                                    - 2.0 * theme.spacing.xs,
                            );

                            let mut inner_action: Option<FloatingToolbarAction> = None;

                            // Group 1: drag + stroke controls
                            Self::render_drag_handle(ui);
                            Self::render_separator(ui);

                            for idx in 0..color_slots_len {
                                if idx < temp_colors.len() {
                                    let mut color = temp_colors[idx];
                                    if Self::render_pen_swatch(ui, idx, &mut color) {
                                        color_changes.push((idx, color));
                                        temp_colors[idx] = color;
                                    }
                                }
                            }

                            if let Some(width) = Self::render_width_chip(ui, &mut stroke_width) {
                                inner_action = Some(FloatingToolbarAction::SetLineWidth(width));
                            }
                            if let Some(style) = Self::render_style_chip(ui, &mut line_style) {
                                inner_action = Some(FloatingToolbarAction::SetLineStyle(style));
                            }

                            // Group 2: annotate / settings
                            Self::render_separator(ui);

                            if IconButton::new(&icons::TEXT)
                                .size(ButtonSize::MD)
                                .tooltip("Text")
                                .show(ui)
                                .clicked()
                            {
                                inner_action = Some(FloatingToolbarAction::OpenText);
                            }
                            if IconButton::new(&icons::SETTINGS_NUT)
                                .size(ButtonSize::MD)
                                .tooltip("Settings")
                                .show(ui)
                                .clicked()
                            {
                                inner_action = Some(FloatingToolbarAction::OpenSettings);
                            }
                            if let Some(more) =
                                Self::render_more_menu(ui, is_visible, alert_available)
                            {
                                inner_action = Some(more);
                            }

                            // Group 3: lock + delete
                            Self::render_separator(ui);

                            let lock_icon = if is_locked {
                                &icons::LOCK
                            } else {
                                &icons::UNLOCK
                            };
                            let lock_tooltip = if is_locked { "Unlock" } else { "Lock" };
                            if IconButton::new(lock_icon)
                                .size(ButtonSize::MD)
                                .tooltip(lock_tooltip)
                                .show(ui)
                                .clicked()
                            {
                                inner_action = Some(FloatingToolbarAction::ToggleLock);
                            }
                            if IconButton::new(&icons::TRASH)
                                .size(ButtonSize::MD)
                                .tooltip("Delete")
                                .show(ui)
                                .clicked()
                            {
                                inner_action = Some(FloatingToolbarAction::Delete);
                            }

                            inner_action
                        })
                        .inner
                    })
                    .inner
            });

        self.stroke_width = stroke_width;
        self.line_style = line_style;
        self.last_rect = area_response.response.rect;

        for (idx, color) in &color_changes {
            if *idx < self.color_slots.len() {
                self.color_slots[*idx].color = *color;
            }
        }

        if let Some((idx, color)) = color_changes.first() {
            return Some(FloatingToolbarAction::ChangeColor(*idx, *color));
        }

        area_response.inner
    }

    fn toolbar_size(&self, ui: &Ui) -> Vec2 {
        let theme = ui.active_theme();
        let color_count = self.color_slots.len();
        let button = DESIGN_TOKENS.sizing.floating_toolbar.button_size;
        let color = DESIGN_TOKENS.sizing.floating_toolbar.color_size;
        let gap = theme.spacing.xs;
        let sep = DESIGN_TOKENS.sizing.floating_toolbar.separator_width;
        let drag = DESIGN_TOKENS.sizing.floating_toolbar.drag_handle_width;
        // width + style chips are wider than a plain icon.
        let chips = 56.0 + 52.0;
        // text, settings, more, lock, delete
        let icons = 5.0 * (button + gap);
        Vec2::new(
            drag + sep
                + color_count as f32 * (color + gap)
                + chips
                + sep
                + icons
                + sep
                + 2.0 * (button + gap)
                + theme.spacing.md * 2.0,
            DESIGN_TOKENS.sizing.floating_toolbar.height,
        )
    }

    fn render_drag_handle(ui: &mut Ui) {
        let size = Vec2::new(
            DESIGN_TOKENS.sizing.floating_toolbar.drag_handle_width,
            DESIGN_TOKENS.sizing.floating_toolbar.button_size,
        );
        // Hover-only: dragging is handled manually in `show` so this stays out of tab order.
        let (rect, response) = ui.allocate_exact_size(size, Sense::hover());
        response.on_hover_cursor(egui::CursorIcon::Grab);
        let tint = ui.visuals().text_color().gamma_multiply(0.45);
        let icon = Vec2::splat(DESIGN_TOKENS.sizing.icon_sm);
        icons::UI_DRAG_HANDLE
            .as_image_tinted(icon, tint)
            .paint_at(ui, Rect::from_center_size(rect.center(), icon));
    }

    fn render_separator(ui: &mut Ui) {
        let height = DESIGN_TOKENS.sizing.floating_toolbar.separator_height;
        ui.add_space(ui.active_theme().spacing.xs);
        let (rect, _) = ui.allocate_exact_size(
            Vec2::new(DESIGN_TOKENS.stroke.hairline, height),
            Sense::hover(),
        );
        let stroke = Stroke::new(
            DESIGN_TOKENS.stroke.hairline,
            ui.active_theme().ui.border_control.gamma_multiply(0.7),
        );
        ui.painter().line_segment(
            [
                Pos2::new(rect.center().x, rect.min.y),
                Pos2::new(rect.center().x, rect.max.y),
            ],
            stroke,
        );
        ui.add_space(ui.active_theme().spacing.xs);
    }

    /// Circular colour well with a pencil badge; opens a glass colour picker.
    fn render_pen_swatch(ui: &mut Ui, slot: usize, color: &mut Color32) -> bool {
        let theme = ui.active_theme();
        let size = Vec2::splat(DESIGN_TOKENS.sizing.floating_toolbar.button_size);
        let (rect, response) = ui.allocate_exact_size(size, Sense::click());
        let label = l10n::text(ui.ctx(), "Color");
        response.widget_info(|| WidgetInfo::labeled(Role::ColorWell, ui.is_enabled(), &label));

        let open_id = ui.make_persistent_id(("floating_toolbar_color", slot));
        let is_open = egui::Popup::is_id_open(ui.ctx(), open_id);
        let hovered = response.hovered() || is_open;
        let radius = theme.radius.md;
        if hovered {
            let fill = if is_open {
                theme.ui.btn_bg_active
            } else {
                theme.ui.btn_bg_hover
            };
            ui.painter().rect_filled(rect, radius, fill);
        }

        // Colour disc
        let disc = Rect::from_center_size(
            rect.center() - Vec2::new(1.0, 1.0),
            Vec2::splat(DESIGN_TOKENS.sizing.floating_toolbar.color_size),
        );
        ui.painter()
            .circle_filled(disc.center(), disc.width() * 0.5, *color);
        ui.painter().circle_stroke(
            disc.center(),
            disc.width() * 0.5,
            Stroke::new(
                DESIGN_TOKENS.stroke.hairline,
                theme.ui.border_control.gamma_multiply(0.85),
            ),
        );

        // Pencil badge
        let badge = Rect::from_center_size(
            Pos2::new(rect.max.x - 7.0, rect.max.y - 7.0),
            Vec2::splat(12.0),
        );
        ui.painter()
            .circle_filled(badge.center(), 6.0, theme.ui.panel_bg);
        icons::PENCIL
            .as_image_tinted(Vec2::splat(10.0), theme.ui.text_secondary)
            .paint_at(ui, badge);

        response
            .clone()
            .on_hover_text(l10n::text(ui.ctx(), "Click to change color"));

        let mut changed = false;
        egui::Popup::menu(&response)
            .id(open_id)
            .close_behavior(PopupCloseBehavior::CloseOnClickOutside)
            .align(RectAlign::BOTTOM_START)
            .align_alternatives(&[])
            .frame(crate::ui_kit::glass_menu(ui))
            .gap(theme.spacing.xs)
            .show(|ui| {
                // Standard swatch grid + opacity (kit palette).
                ui.set_min_width(200.0);
                if show_palette(ui, color) {
                    changed = true;
                }
            });
        changed
    }

    fn chip_chrome(ui: &Ui, rect: Rect, open: bool, hovered: bool) {
        let theme = ui.active_theme();
        let radius = theme.radius.sm;
        let fill = if open {
            theme.ui.btn_bg_active
        } else if hovered {
            theme.ui.btn_bg_hover
        } else {
            Color32::TRANSPARENT
        };
        if fill != Color32::TRANSPARENT {
            ui.painter().rect_filled(rect, radius, fill);
        }
        ui.painter().rect_stroke(
            rect,
            radius,
            Stroke::new(
                DESIGN_TOKENS.stroke.hairline,
                theme
                    .ui
                    .border_control
                    .gamma_multiply(if open || hovered { 1.0 } else { 0.75 }),
            ),
            StrokeKind::Inside,
        );
    }

    /// Outlined width chip: thick-line preview + label + chevron.
    fn render_width_chip(ui: &mut Ui, width: &mut f32) -> Option<f32> {
        let theme = ui.active_theme();
        let label = format!("{}", (*width).round() as i32);
        let height = DESIGN_TOKENS.sizing.floating_toolbar.button_size;
        let chevron = DESIGN_TOKENS.sizing.icon_sm;
        let preview_w = 18.0;
        let galley = ui.painter().layout_no_wrap(
            label.clone(),
            FontId::proportional(theme.text.small),
            theme.ui.text,
        );
        let size = Vec2::new(
            preview_w + galley.size().x + chevron + theme.spacing.md + theme.spacing.sm,
            height,
        );
        let (rect, response) = ui.allocate_exact_size(size, Sense::click());
        let a11y = format!("{label}px");
        response.widget_info(|| WidgetInfo::labeled(Role::ComboBox, ui.is_enabled(), &a11y));

        let id = ui.make_persistent_id("floating_toolbar_width");
        let is_open = egui::Popup::is_id_open(ui.ctx(), id);
        Self::chip_chrome(ui, rect, is_open, response.hovered());

        let stroke_color = theme.ui.text;
        let line_y = rect.center().y;
        let line_x0 = rect.min.x + theme.spacing.sm;
        let line_x1 = line_x0 + preview_w;
        ui.painter().line_segment(
            [Pos2::new(line_x0, line_y), Pos2::new(line_x1, line_y)],
            Stroke::new((*width).clamp(1.0, 4.0), stroke_color),
        );

        let text_pos = Pos2::new(
            line_x1 + theme.spacing.xs,
            rect.center().y - galley.size().y * 0.5,
        );
        ui.painter().galley(text_pos, galley, theme.ui.text);

        let chevron_rect = Rect::from_center_size(
            Pos2::new(
                rect.max.x - theme.spacing.sm - chevron * 0.5,
                rect.center().y,
            ),
            Vec2::splat(chevron),
        );
        icons::CHEVRON_DOWN
            .as_image_tinted(chevron_rect.size(), theme.ui.text_muted)
            .paint_at(ui, chevron_rect);
        response
            .clone()
            .on_hover_text(l10n::text(ui.ctx(), "Line width"));

        let mut picked = None;
        egui::Popup::menu(&response)
            .id(id)
            .close_behavior(PopupCloseBehavior::CloseOnClickOutside)
            .align(RectAlign::BOTTOM_START)
            .align_alternatives(&[])
            .frame(crate::ui_kit::glass_menu(ui))
            .gap(theme.spacing.xs)
            .show(|ui| {
                ui.set_min_width(108.0);
                ui.spacing_mut().item_spacing.y = theme.spacing.xs;
                for preset in LINE_WIDTHS {
                    let selected = (*width - preset).abs() < f32::EPSILON;
                    let row_h = 28.0;
                    let (row, resp) = ui.allocate_exact_size(
                        Vec2::new(ui.available_width(), row_h),
                        Sense::click(),
                    );
                    if selected || resp.hovered() {
                        ui.painter().rect_filled(
                            row,
                            theme.radius.sm,
                            if selected {
                                theme.ui.btn_bg_active
                            } else {
                                theme.ui.btn_bg_hover
                            },
                        );
                    }
                    let preview = Rect::from_center_size(
                        Pos2::new(row.min.x + 22.0, row.center().y),
                        Vec2::new(28.0, preset),
                    );
                    ui.painter().line_segment(
                        [
                            Pos2::new(preview.min.x, preview.center().y),
                            Pos2::new(preview.max.x, preview.center().y),
                        ],
                        Stroke::new(preset, theme.ui.text),
                    );
                    ui.painter().text(
                        Pos2::new(row.min.x + 48.0, row.center().y),
                        egui::Align2::LEFT_CENTER,
                        format!("{preset}px"),
                        FontId::proportional(theme.text.small),
                        theme.ui.text,
                    );
                    resp.widget_info(|| {
                        WidgetInfo::selected(
                            Role::Button,
                            ui.is_enabled(),
                            selected,
                            format!("{preset}px"),
                        )
                    });
                    if resp.clicked() {
                        *width = preset;
                        picked = Some(preset);
                        ui.close();
                    }
                }
            });
        picked
    }

    /// Outlined style chip with dash preview + chevron.
    fn render_style_chip(ui: &mut Ui, style: &mut LineStyle) -> Option<LineStyle> {
        let theme = ui.active_theme();
        let height = DESIGN_TOKENS.sizing.floating_toolbar.button_size;
        let preview_w = 28.0;
        let chevron = DESIGN_TOKENS.sizing.icon_sm;
        let size = Vec2::new(
            preview_w + chevron + theme.spacing.md + theme.spacing.sm,
            height,
        );
        let (rect, response) = ui.allocate_exact_size(size, Sense::click());
        response.widget_info(|| WidgetInfo::labeled(Role::ComboBox, ui.is_enabled(), style.name()));

        let id = ui.make_persistent_id("floating_toolbar_style");
        let is_open = egui::Popup::is_id_open(ui.ctx(), id);
        Self::chip_chrome(ui, rect, is_open, response.hovered());

        let preview = Rect::from_center_size(
            Pos2::new(
                rect.min.x + theme.spacing.sm + preview_w * 0.5,
                rect.center().y,
            ),
            Vec2::new(preview_w, 2.0),
        );
        Self::paint_style_preview(ui, preview, *style, theme.ui.text);

        let chevron_rect = Rect::from_center_size(
            Pos2::new(
                rect.max.x - theme.spacing.sm - chevron * 0.5,
                rect.center().y,
            ),
            Vec2::splat(chevron),
        );
        icons::CHEVRON_DOWN
            .as_image_tinted(chevron_rect.size(), theme.ui.text_muted)
            .paint_at(ui, chevron_rect);
        response
            .clone()
            .on_hover_text(l10n::text(ui.ctx(), "Line style"));

        let mut picked = None;
        egui::Popup::menu(&response)
            .id(id)
            .close_behavior(PopupCloseBehavior::CloseOnClickOutside)
            .align(RectAlign::BOTTOM_START)
            .align_alternatives(&[])
            .frame(crate::ui_kit::glass_menu(ui))
            .gap(theme.spacing.xs)
            .show(|ui| {
                ui.set_min_width(132.0);
                for choice in LineStyle::all() {
                    let selected = *style == *choice;
                    let (row, resp) = ui
                        .allocate_exact_size(Vec2::new(ui.available_width(), 28.0), Sense::click());
                    if selected || resp.hovered() {
                        ui.painter().rect_filled(
                            row,
                            theme.radius.sm,
                            if selected {
                                theme.ui.btn_bg_active
                            } else {
                                theme.ui.btn_bg_hover
                            },
                        );
                    }
                    let sample = Rect::from_center_size(
                        Pos2::new(row.min.x + 24.0, row.center().y),
                        Vec2::new(36.0, 2.0),
                    );
                    Self::paint_style_preview(ui, sample, *choice, theme.ui.text);
                    ui.painter().text(
                        Pos2::new(row.min.x + 52.0, row.center().y),
                        egui::Align2::LEFT_CENTER,
                        l10n::text(ui.ctx(), choice.name()),
                        FontId::proportional(theme.text.small),
                        theme.ui.text,
                    );
                    resp.widget_info(|| {
                        WidgetInfo::selected(Role::Button, ui.is_enabled(), selected, choice.name())
                    });
                    if resp.clicked() {
                        *style = *choice;
                        picked = Some(*choice);
                        ui.close();
                    }
                }
            });
        picked
    }

    fn paint_style_preview(ui: &Ui, rect: Rect, style: LineStyle, color: Color32) {
        let stroke = Stroke::new(2.0, color);
        let start = Pos2::new(rect.min.x, rect.center().y);
        let end = Pos2::new(rect.max.x, rect.center().y);
        match style.dash_pattern() {
            None => {
                ui.painter().line_segment([start, end], stroke);
            }
            Some((dash, gap)) => {
                draw_dashed_line(ui.painter(), start, end, stroke, dash, gap);
            }
        }
    }

    /// Overflow menu for secondary actions.
    fn render_more_menu(
        ui: &mut Ui,
        is_visible: bool,
        alert_available: bool,
    ) -> Option<FloatingToolbarAction> {
        let id = ui.make_persistent_id("floating_toolbar_more");
        let is_open = egui::Popup::is_id_open(ui.ctx(), id);
        let response = IconButton::new(&icons::UI_MORE_HORIZONTAL)
            .size(ButtonSize::MD)
            .selected(is_open)
            .tooltip("More")
            .show(ui);
        let theme = ui.active_theme();
        let mut action = None;
        egui::Popup::menu(&response)
            .id(id)
            .close_behavior(PopupCloseBehavior::CloseOnClickOutside)
            .align(RectAlign::BOTTOM_START)
            .align_alternatives(&[])
            .frame(crate::ui_kit::glass_menu(ui))
            .gap(theme.spacing.xs)
            .show(|ui| {
                ui.set_min_width(148.0);
                if ui
                    .add(egui::Button::image_and_text(
                        icons::DUPLICATE.as_image_tinted(
                            Vec2::splat(DESIGN_TOKENS.sizing.icon_sm),
                            theme.ui.text_secondary,
                        ),
                        l10n::text(ui.ctx(), "Duplicate"),
                    ))
                    .clicked()
                {
                    action = Some(FloatingToolbarAction::Duplicate);
                    ui.close();
                }
                let (vis_icon, vis_label) = if is_visible {
                    (&icons::EYE_HIDE, "Hide")
                } else {
                    (&icons::EYE, "Show")
                };
                if ui
                    .add(egui::Button::image_and_text(
                        vis_icon.as_image_tinted(
                            Vec2::splat(DESIGN_TOKENS.sizing.icon_sm),
                            theme.ui.text_secondary,
                        ),
                        l10n::text(ui.ctx(), vis_label),
                    ))
                    .clicked()
                {
                    action = Some(FloatingToolbarAction::ToggleVisibility);
                    ui.close();
                }
                if alert_available
                    && ui
                        .add(egui::Button::image_and_text(
                            icons::ALERTS.as_image_tinted(
                                Vec2::splat(DESIGN_TOKENS.sizing.icon_sm),
                                theme.ui.text_secondary,
                            ),
                            l10n::text(ui.ctx(), "Add alert"),
                        ))
                        .clicked()
                {
                    action = Some(FloatingToolbarAction::AddAlert);
                    ui.close();
                }
            });
        action
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Keyboard navigation announces each primary action and chip.
    #[test]
    fn action_buttons_have_accessible_names() {
        for locked in [false, true] {
            let ctx = egui::Context::default();
            let mut toolbar = FloatingSelectionToolbar::new();
            toolbar.show_at(Pos2::new(100.0, 100.0));
            toolbar.set_locked(locked);
            toolbar.set_object_visible(!locked);
            toolbar.set_alert_available(locked);
            let mut frame = |tab| {
                ctx.run_ui(
                    egui::RawInput {
                        screen_rect: Some(Rect::from_min_size(
                            Pos2::ZERO,
                            egui::vec2(800.0, 600.0),
                        )),
                        events: if tab {
                            vec![egui::Event::Key {
                                key: egui::Key::Tab,
                                physical_key: None,
                                pressed: true,
                                repeat: false,
                                modifiers: egui::Modifiers::NONE,
                            }]
                        } else {
                            vec![]
                        },
                        ..Default::default()
                    },
                    |ui| {
                        toolbar.show(ui);
                    },
                )
            };
            frame(false).drop_without_applying_deltas();
            frame(false).drop_without_applying_deltas();
            let expected = [
                "2px",
                "Solid",
                "Text",
                "Settings",
                "More",
                if locked { "Unlock" } else { "Lock" },
                "Delete",
            ];
            let mut found = std::collections::HashSet::<String>::new();
            for _ in 0..expected.len() + 2 {
                let mut output = frame(true);
                let events = std::mem::take(&mut output.platform_output.events);
                output.drop_without_applying_deltas();
                for event in &events {
                    if let Some(label) = event.widget_info().label.clone() {
                        found.insert(label);
                    }
                }
            }
            for label in expected {
                assert!(
                    found.contains(label),
                    "missing accessible name: {label}; saw {found:?}"
                );
            }
        }
    }

    /// Floating surfaces and group rules follow glass popup and separator visuals.
    #[test]
    fn surface_and_separators_follow_the_popup_theme() {
        for (_, theme) in crate::theme::presets() {
            let ctx = egui::Context::default();
            crate::theme::apply_to_egui(&ctx, &theme);
            ctx.global_style_mut(|style| style.animation_time = 0.0);
            let expected_fill = crate::ui_kit::frames::glass_fill(&theme);
            let expected_stroke = crate::ui_kit::frames::glass_stroke(&theme);
            let mut toolbar = FloatingSelectionToolbar::new();
            toolbar.show_at(Pos2::new(100.0, 100.0));
            let raw = egui::RawInput {
                screen_rect: Some(Rect::from_min_size(Pos2::ZERO, egui::vec2(800.0, 600.0))),
                ..Default::default()
            };
            ctx.run_ui(raw.clone(), |ui| {
                toolbar.show(ui);
            })
            .drop_without_applying_deltas();
            let mut output = ctx.run_ui(raw, |ui| {
                toolbar.show(ui);
            });
            let shapes = std::mem::take(&mut output.shapes);
            output.drop_without_applying_deltas();
            let shapes: Vec<_> = shapes
                .into_iter()
                .flat_map(|shape| match shape.shape {
                    egui::Shape::Vec(shapes) => shapes,
                    shape => vec![shape],
                })
                .collect();
            assert!(shapes.iter().any(|shape| matches!(shape,
                egui::Shape::Rect(rect) if rect.fill == expected_fill
                    && rect.stroke == expected_stroke
            )));
            assert!(shapes.iter().any(|shape| matches!(
                shape,
                egui::Shape::Rect(rect) if rect.blur_width >= 16.0
            )));
            let separators = shapes
                .iter()
                .filter(|shape| {
                    matches!(shape,
                        egui::Shape::LineSegment { points, stroke }
                            if points[0].x == points[1].x
                                && (points[1].y - points[0].y).abs()
                                    == DESIGN_TOKENS.sizing.floating_toolbar.separator_height
                                && stroke.width == DESIGN_TOKENS.stroke.hairline
                    )
                })
                .count();
            assert_eq!(separators, 3);
        }
    }

    /// A viewport smaller than the toolbar must not panic and pins to the min edge.
    #[test]
    fn show_survives_viewport_smaller_than_toolbar() {
        for (w, h, expected) in [
            (100.0, 100.0, Pos2::new(0.0, 10.0)),
            (800.0, 20.0, Pos2::new(10.0, 0.0)),
            (100.0, 20.0, Pos2::ZERO),
        ] {
            let ctx = egui::Context::default();
            let mut toolbar = FloatingSelectionToolbar::new();
            toolbar.show_at(Pos2::new(10.0, 10.0));
            let raw = egui::RawInput {
                screen_rect: Some(Rect::from_min_size(Pos2::ZERO, Vec2::new(w, h))),
                ..Default::default()
            };
            ctx.run_ui(raw.clone(), |_| {}).textures_delta.clear();
            ctx.run_ui(raw, |ui| {
                toolbar.show(ui);
            })
            .textures_delta
            .clear();
            assert_eq!(toolbar.position, expected, "viewport {w}x{h}");
        }
    }
}
