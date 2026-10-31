//! Color picker popup
//!
//! A color swatch picker with grayscale row, main colors row, and color variations.

use crate::localization as l10n;
use crate::theme::ThemeExt;
use crate::tokens::DESIGN_TOKENS;
use egui::{
    Color32, Id, Pos2, Rect, Role, Sense, Stroke, Ui, Vec2, WidgetInfo, epaint::StrokeKind,
};

// ============================================================================
// COLOR PALETTE - exact colors
// ============================================================================

/// Grayscale colors (row 1)
pub const GRAYSCALE: [Color32; 10] = [
    Color32::from_rgb(255, 255, 255), // white
    Color32::from_rgb(219, 219, 219),
    Color32::from_rgb(184, 184, 184),
    Color32::from_rgb(156, 156, 156),
    Color32::from_rgb(128, 128, 128),
    Color32::from_rgb(99, 99, 99),
    Color32::from_rgb(74, 74, 74),
    Color32::from_rgb(46, 46, 46),
    Color32::from_rgb(15, 15, 15),
    Color32::from_rgb(0, 0, 0), // black
];

/// Main colors (row 2)
pub const MAIN_COLORS: [Color32; 10] = [
    Color32::from_rgb(242, 54, 69),  // red
    Color32::from_rgb(255, 152, 0),  // orange
    Color32::from_rgb(255, 235, 59), // yellow
    Color32::from_rgb(76, 175, 80),  // green
    Color32::from_rgb(8, 153, 129),  // teal
    Color32::from_rgb(0, 188, 212),  // cyan
    Color32::from_rgb(41, 98, 255),  // blue
    Color32::from_rgb(103, 58, 183), // purple
    Color32::from_rgb(156, 39, 176), // magenta
    Color32::from_rgb(233, 30, 99),  // pink
];

/// Color variations - 6 rows from light to dark for each main color
pub const COLOR_VARIATIONS: [[Color32; 10]; 6] = [
    // Row 1 - Lightest
    [
        Color32::from_rgb(252, 203, 205),
        Color32::from_rgb(255, 224, 178),
        Color32::from_rgb(255, 249, 196),
        Color32::from_rgb(200, 230, 201),
        Color32::from_rgb(172, 229, 220),
        Color32::from_rgb(178, 235, 242),
        Color32::from_rgb(187, 217, 251),
        Color32::from_rgb(209, 196, 233),
        Color32::from_rgb(225, 190, 231),
        Color32::from_rgb(248, 187, 208),
    ],
    // Row 2
    [
        Color32::from_rgb(250, 161, 164),
        Color32::from_rgb(255, 204, 128),
        Color32::from_rgb(255, 245, 157),
        Color32::from_rgb(165, 214, 167),
        Color32::from_rgb(112, 204, 189),
        Color32::from_rgb(128, 222, 234),
        Color32::from_rgb(144, 191, 249),
        Color32::from_rgb(179, 157, 219),
        Color32::from_rgb(206, 147, 216),
        Color32::from_rgb(244, 143, 177),
    ],
    // Row 3
    [
        Color32::from_rgb(247, 124, 128),
        Color32::from_rgb(255, 183, 77),
        Color32::from_rgb(255, 241, 118),
        Color32::from_rgb(129, 199, 132),
        Color32::from_rgb(66, 189, 168),
        Color32::from_rgb(77, 208, 225),
        Color32::from_rgb(91, 156, 246),
        Color32::from_rgb(149, 117, 205),
        Color32::from_rgb(186, 104, 200),
        Color32::from_rgb(240, 98, 146),
    ],
    // Row 4
    [
        Color32::from_rgb(247, 82, 95),
        Color32::from_rgb(255, 167, 38),
        Color32::from_rgb(255, 238, 88),
        Color32::from_rgb(102, 187, 106),
        Color32::from_rgb(34, 171, 148),
        Color32::from_rgb(38, 198, 218),
        Color32::from_rgb(49, 121, 245),
        Color32::from_rgb(126, 87, 194),
        Color32::from_rgb(171, 71, 188),
        Color32::from_rgb(236, 64, 122),
    ],
    // Row 5
    [
        Color32::from_rgb(178, 40, 51),
        Color32::from_rgb(245, 124, 0),
        Color32::from_rgb(251, 192, 45),
        Color32::from_rgb(56, 142, 60),
        Color32::from_rgb(5, 102, 86),
        Color32::from_rgb(0, 151, 167),
        Color32::from_rgb(24, 72, 204),
        Color32::from_rgb(81, 45, 168),
        Color32::from_rgb(123, 31, 162),
        Color32::from_rgb(194, 24, 91),
    ],
    // Row 6 - Darkest
    [
        Color32::from_rgb(128, 25, 34),
        Color32::from_rgb(230, 81, 0),
        Color32::from_rgb(245, 127, 23),
        Color32::from_rgb(27, 94, 32),
        Color32::from_rgb(0, 51, 42),
        Color32::from_rgb(0, 96, 100),
        Color32::from_rgb(12, 50, 153),
        Color32::from_rgb(49, 27, 146),
        Color32::from_rgb(74, 20, 140),
        Color32::from_rgb(136, 14, 79),
    ],
];

// ============================================================================
// COLOR PICKER CONFIG
// ============================================================================

/// Configuration for the color picker
#[derive(Clone, Debug)]
pub struct ColorPickerConfig {
    /// Size of each color swatch
    pub swatch_size: f32,
    /// Spacing between swatches
    pub swatch_spacing: f32,
    /// Spacing between rows
    pub row_spacing: f32,
    /// Corner radius for swatches
    pub swatch_rounding: f32,
    /// Popup background color
    pub bg_color: Color32,
    /// Border color for popup
    pub border_color: Color32,
    /// Border color for selected swatch
    pub sel_border_color: Color32,
    /// Popup padding
    pub padding: f32,
}

impl Default for ColorPickerConfig {
    fn default() -> Self {
        // Colors are TRANSPARENT to signal that colors should be fetched from
        // ui.style().visuals at render time for proper theme support.
        Self {
            swatch_size: 20.0,
            swatch_spacing: crate::theme::Spacing::default().sm,
            row_spacing: crate::theme::Spacing::default().sm,
            swatch_rounding: crate::theme::Radii::default().sm,
            bg_color: Color32::TRANSPARENT,
            border_color: Color32::TRANSPARENT,
            sel_border_color: Color32::TRANSPARENT,
            padding: crate::theme::Spacing::default().xl,
        }
    }
}

// ============================================================================
// COLOR PICKER STATE
// ============================================================================

/// State for a color picker popup
#[derive(Clone, Debug, Default)]
pub struct ColorPickerState {
    /// Whether the popup is open
    pub is_open: bool,
    /// Pos of the popup
    pub popup_pos: Option<Pos2>,
    /// Currently selected color
    pub sel_color: Color32,
    /// Whether user is dragging to select custom color
    pub custom_mode: bool,
    /// Custom color being edited
    pub custom_color: Color32,
    /// Whether popup was just opened this frame (skip close detection)
    pub just_opened: bool,
}

impl ColorPickerState {
    /// Create a closed picker with `initial_color` selected.
    pub fn new(initial_color: Color32) -> Self {
        Self {
            is_open: false,
            popup_pos: None,
            sel_color: initial_color,
            custom_mode: false,
            custom_color: initial_color,
            just_opened: false,
        }
    }
}

// ============================================================================
// COLOR PICKER WIDGET
// ============================================================================

/// Color swatch picker
pub struct ColorPicker<'a> {
    config: ColorPickerConfig,
    state: &'a mut ColorPickerState,
    id: Id,
}

impl<'a> ColorPicker<'a> {
    /// Bind a color picker to persistent state and an egui identity.
    pub fn new(
        state: &'a mut ColorPickerState,
        id_src: impl std::hash::Hash + std::fmt::Debug,
    ) -> Self {
        Self {
            config: ColorPickerConfig::default(),
            state,
            id: Id::unique(id_src),
        }
    }

    /// Use the supplied display and layout options.
    pub fn with_config(mut self, config: ColorPickerConfig) -> Self {
        self.config = config;
        self
    }

    /// Show a color swatch button that opens the picker when clicked
    /// Returns the picked colour, if changed, and the colour well's response.
    pub fn show(mut self, ui: &mut Ui) -> egui::InnerResponse<Option<Color32>> {
        let theme = ui.active_theme();
        let defaults = ColorPickerConfig::default();
        for (value, default, themed) in [
            (
                &mut self.config.swatch_spacing,
                defaults.swatch_spacing,
                theme.spacing.sm,
            ),
            (
                &mut self.config.row_spacing,
                defaults.row_spacing,
                theme.spacing.sm,
            ),
            (
                &mut self.config.swatch_rounding,
                defaults.swatch_rounding,
                theme.radius.sm,
            ),
            (&mut self.config.padding, defaults.padding, theme.spacing.xl),
        ] {
            if *value == default {
                *value = themed;
            }
        }

        let mut color_changed = None;

        // Get colors from theme
        let visuals = ui.style().visuals.clone();
        let bg_color = visuals.faint_bg_color;

        let btn_size = Vec2::splat(
            ui.spacing()
                .interact_size
                .y
                .max(DESIGN_TOKENS.sizing.button_md),
        );
        let (btn_rect, mut btn_res) = ui.allocate_exact_size(btn_size, Sense::click());
        let translated_label = l10n::text(ui.ctx(), hex(self.state.sel_color));
        btn_res.widget_info(|| {
            WidgetInfo::labeled(Role::ColorWell, ui.is_enabled(), &translated_label)
        });

        // Draw dark background then blend color on top
        // This avoids the "checkered pattern" look for semi-transparent colors
        ui.painter()
            .rect_filled(btn_rect, ui.active_theme().radius.md, bg_color);

        // Keep the surface border separate from the colour, including white and black.
        ui.painter()
            .rect_filled(btn_rect.shrink(4.0), 2.0, self.state.sel_color);

        // The inset leaves the control border visible beside any swatch colour.
        let border_color = if self.state.is_open {
            ui.active_theme().ui.accent
        } else if btn_res.hovered() {
            visuals.widgets.hovered.fg_stroke.color
        } else {
            ui.active_theme().ui.border_control
        };
        ui.painter().rect_stroke(
            btn_rect,
            ui.active_theme().radius.md,
            Stroke::new(DESIGN_TOKENS.stroke.hairline, border_color),
            StrokeKind::Inside,
        );

        crate::styles::focus::draw_focus_ring(ui, &btn_res, ui.active_theme().radius.md);

        if btn_res.clicked() {
            self.state.is_open = !self.state.is_open;
            self.state.popup_pos = Some(btn_rect.left_bottom() + Vec2::new(0.0, 4.0));
            self.state.just_opened = self.state.is_open;
            self.state.custom_mode = false;
            self.state.custom_color = self.state.sel_color;
        }
        if self.state.is_open
            && ui.input_mut(|i| i.consume_key(egui::Modifiers::NONE, egui::Key::Escape))
        {
            self.state.is_open = false;
        }
        if self.state.is_open {
            color_changed = self.show_popup(ui);
        }
        if !self.state.is_open {
            self.state.custom_mode = false;
            self.state.just_opened = false;
        }
        if color_changed.is_some() {
            btn_res.mark_changed();
        }
        egui::InnerResponse::new(color_changed, btn_res)
    }

    fn show_popup(&mut self, ui: &mut Ui) -> Option<Color32> {
        let mut color_changed = None;
        let popup_pos = self.state.popup_pos.unwrap_or(ui.cursor().min);

        // Get colors from theme (fall back to config if set, otherwise use visuals)
        let visuals = ui.style().visuals.clone();
        let popup_bg = if self.config.bg_color == Color32::TRANSPARENT {
            visuals.window_fill
        } else {
            self.config.bg_color
        };
        // The floating border, which dialogs that restyle their widgets leave alone.
        let popup_border = if self.config.border_color == Color32::TRANSPARENT {
            visuals.window_stroke.color
        } else {
            self.config.border_color
        };
        let separator_color = popup_border;

        // Calculate popup size
        let cols = 10;
        let popup_width = self.config.padding * 2.0
            + (self.config.swatch_size * cols as f32)
            + (self.config.swatch_spacing * (cols - 1) as f32);
        // Draw popup using an egui Area
        let nested_popup_open = egui::Popup::is_any_open(ui.ctx());
        let area = egui::Area::new(self.id.with("popup"))
            .fixed_pos(popup_pos)
            .order(egui::Order::Foreground)
            .show(ui.ctx(), |ui| {
                egui::Frame::popup(ui.style())
                    .fill(popup_bg)
                    .stroke(Stroke::new(DESIGN_TOKENS.stroke.hairline, popup_border))
                    .inner_margin(self.config.padding)
                    .show(ui, |ui| {
                        ui.set_width(popup_width - self.config.padding * 2.0);
                        // Grayscale row
                        if let Some(color) = self.draw_color_row(ui, &GRAYSCALE) {
                            color_changed = Some(color);
                        }

                        // Main colors row
                        if let Some(color) = self.draw_color_row(ui, &MAIN_COLORS) {
                            color_changed = Some(color);
                        }

                        ui.add_space(ui.active_theme().spacing.lg);

                        // Separator
                        let sep_rect = ui.available_rect_before_wrap();
                        ui.painter().hline(
                            sep_rect.x_range(),
                            sep_rect.min.y,
                            Stroke::new(DESIGN_TOKENS.stroke.hairline, separator_color),
                        );
                        ui.add_space(ui.active_theme().spacing.lg);

                        // Color variations
                        for row in &COLOR_VARIATIONS {
                            if let Some(color) = self.draw_color_row(ui, row) {
                                color_changed = Some(color);
                            }
                        }

                        ui.add_space(ui.active_theme().spacing.lg);

                        // Separator
                        let sep_rect = ui.available_rect_before_wrap();
                        ui.painter().hline(
                            sep_rect.x_range(),
                            sep_rect.min.y,
                            Stroke::new(DESIGN_TOKENS.stroke.hairline, separator_color),
                        );
                        ui.add_space(ui.active_theme().spacing.lg);

                        // Custom color button
                        self.draw_custom_color_btn(ui);

                        // If custom mode is active, show color editor
                        if self.state.custom_mode {
                            ui.add_space(ui.active_theme().spacing.lg);
                            let mut custom = self.state.custom_color;
                            if ui.color_edit_button_srgba(&mut custom).changed() {
                                self.state.custom_color = custom;
                                color_changed = Some(custom);
                            }
                            ui.add_space(ui.active_theme().spacing.sm);
                            if ui.small_button(l10n::text(ui.ctx(), "Apply")).clicked() {
                                color_changed = Some(self.state.custom_color);
                                self.state.is_open = false;
                            }
                        }
                    });
            });

        if self.state.just_opened {
            self.state.just_opened = false;
        } else if !nested_popup_open
            && !egui::Popup::is_any_open(ui.ctx())
            && ui.input(|i| i.pointer.any_released())
            && ui
                .input(|i| i.pointer.interact_pos())
                .is_some_and(|pos| !area.response.rect.contains(pos))
        {
            self.state.is_open = false;
        }

        // If color changed, update state and close popup
        if let Some(color) = color_changed {
            self.state.sel_color = color;
            if !self.state.custom_mode {
                self.state.is_open = false;
            }
        }

        color_changed
    }

    fn draw_color_row(&self, ui: &mut Ui, colors: &[Color32]) -> Option<Color32> {
        let mut selected = None;

        // Get colors from theme
        let visuals = ui.style().visuals.clone();
        let sel_border = if self.config.sel_border_color == Color32::TRANSPARENT {
            ui.active_theme().ui.accent
        } else {
            self.config.sel_border_color
        };
        let popup_bg = if self.config.bg_color == Color32::TRANSPARENT {
            visuals.window_fill
        } else {
            self.config.bg_color
        };
        let hover_border = visuals.widgets.hovered.fg_stroke.color;

        ui.horizontal(|ui| {
            ui.spacing_mut().item_spacing.x = self.config.swatch_spacing;

            for color in colors {
                let size = Vec2::splat(self.config.swatch_size);
                let (rect, response) = ui.allocate_exact_size(size, Sense::click());

                // Draw swatch
                let rounding = self.config.swatch_rounding;
                ui.painter().rect_filled(rect, rounding, *color);

                if crate::theme::contrast(*color, popup_bg) < 1.5 {
                    ui.painter().rect_stroke(
                        rect,
                        rounding,
                        visuals.window_stroke,
                        StrokeKind::Inside,
                    );
                }
                let border = if *color == self.state.sel_color {
                    Some(sel_border)
                } else if response.hovered() {
                    Some(hover_border)
                } else {
                    None
                };
                if let Some(border) = border {
                    ui.painter().rect_stroke(
                        rect.expand(1.0),
                        rounding + 1.0,
                        Stroke::new(DESIGN_TOKENS.stroke.thick, border),
                        StrokeKind::Outside,
                    );
                }
                crate::styles::focus::draw_focus_ring(ui, &response, rounding);
                let translated_label = l10n::text(ui.ctx(), hex(*color));
                response.widget_info(|| {
                    WidgetInfo::selected(
                        Role::ColorWell,
                        ui.is_enabled(),
                        *color == self.state.sel_color,
                        &translated_label,
                    )
                });

                if response.clicked() {
                    selected = Some(*color);
                }
            }
        });

        ui.add_space(self.config.row_spacing);

        selected
    }

    /// The hex and opacity row; clicking it toggles the custom colour editor.
    fn draw_custom_color_btn(&mut self, ui: &mut Ui) {
        let visuals = ui.style().visuals.clone();
        let (rect, response) = ui.allocate_exact_size(
            Vec2::new(ui.available_width(), DESIGN_TOKENS.sizing.button_md),
            Sense::click(),
        );
        let translated_label = l10n::text(ui.ctx(), "Custom colour");
        response
            .widget_info(|| WidgetInfo::labeled(Role::Button, ui.is_enabled(), &translated_label));

        let border_color = if response.hovered() || self.state.custom_mode {
            visuals.widgets.hovered.fg_stroke.color
        } else {
            visuals.window_stroke.color
        };
        crate::styles::focus::draw_focus_ring(ui, &response, ui.active_theme().radius.sm);
        let painter = ui.painter();
        painter.rect_stroke(
            rect,
            ui.active_theme().radius.sm,
            Stroke::new(DESIGN_TOKENS.stroke.hairline, border_color),
            StrokeKind::Inside,
        );

        let font = egui::TextStyle::Body.resolve(ui.style());
        let label_color = visuals.widgets.noninteractive.fg_stroke.color;
        let value_color = visuals.widgets.hovered.fg_stroke.color;
        let gap = ui.active_theme().spacing.sm;
        let y = rect.center().y;
        let opacity = format!("{}%", (f32::from(self.state.sel_color.a()) / 2.55).round());

        let label = painter.text(
            egui::pos2(rect.left() + ui.active_theme().spacing.lg, y),
            egui::Align2::LEFT_CENTER,
            l10n::text(ui.ctx(), "Hex"),
            font.clone(),
            label_color,
        );
        painter.text(
            egui::pos2(label.right() + gap, y),
            egui::Align2::LEFT_CENTER,
            l10n::text(ui.ctx(), hex(self.state.sel_color)),
            font.clone(),
            value_color,
        );
        let value = painter.text(
            egui::pos2(rect.right() - ui.active_theme().spacing.lg, y),
            egui::Align2::RIGHT_CENTER,
            l10n::text(ui.ctx(), opacity),
            font.clone(),
            value_color,
        );
        painter.text(
            egui::pos2(value.left() - gap, y),
            egui::Align2::RIGHT_CENTER,
            l10n::text(ui.ctx(), "Opacity"),
            font,
            label_color,
        );

        if response.clicked() {
            if !self.state.custom_mode {
                self.state.custom_color = self.state.sel_color;
            }
            self.state.custom_mode = !self.state.custom_mode;
        }
    }
}

/// `#RRGGBBAA` including alpha so opacity edits stay visible in the field.
fn hex(color: Color32) -> String {
    let [r, g, b, a] = color.to_srgba_unmultiplied();
    format!("#{r:02X}{g:02X}{b:02X}{a:02X}")
}

/// Memory key for recently picked palette colours (floating toolbar, etc.).
fn recent_colors_id() -> Id {
    Id::unique("color_picker_recent")
}

/// Paint the standard charting palette grid (grayscale, hues, shades, recent, opacity).
///
/// For hosts that supply their own trigger and popup chrome (e.g. the floating selection bar).
/// Returns `true` when `color` changed.
pub fn show_palette(ui: &mut Ui, color: &mut Color32) -> bool {
    let theme = ui.active_theme();
    let swatch = 18.0;
    let gap = theme.spacing.xs;
    let rounding = theme.radius.sm;
    // Ten swatches per row — keep opacity / recent on the same width.
    let palette_w = swatch * 10.0 + gap * 9.0;
    ui.set_width(palette_w);
    let mut changed = false;

    let paint_row = |ui: &mut Ui, colors: &[Color32], current: Color32| -> Option<Color32> {
        let mut picked = None;
        ui.horizontal(|ui| {
            ui.spacing_mut().item_spacing.x = gap;
            for &c in colors {
                let (rect, response) = ui.allocate_exact_size(Vec2::splat(swatch), Sense::click());
                ui.painter().rect_filled(rect, rounding, c);
                if crate::theme::contrast(c, theme.ui.panel_bg_floating) < 1.5 {
                    ui.painter().rect_stroke(
                        rect,
                        rounding,
                        Stroke::new(DESIGN_TOKENS.stroke.hairline, theme.ui.border_control),
                        StrokeKind::Inside,
                    );
                }
                let selected = c.to_opaque() == current.to_opaque();
                if selected || response.hovered() {
                    ui.painter().rect_stroke(
                        rect.expand(1.0),
                        rounding + 1.0,
                        Stroke::new(
                            DESIGN_TOKENS.stroke.thick,
                            if selected {
                                theme.ui.text
                            } else {
                                theme.ui.border_control
                            },
                        ),
                        StrokeKind::Outside,
                    );
                }
                let label = hex(c);
                response.widget_info(|| {
                    WidgetInfo::selected(Role::ColorWell, ui.is_enabled(), selected, &label)
                });
                if response.clicked() {
                    picked = Some(c);
                }
            }
        });
        ui.add_space(gap);
        picked
    };

    if let Some(c) = paint_row(ui, &GRAYSCALE, *color) {
        let a = color.a();
        *color = Color32::from_rgba_unmultiplied(c.r(), c.g(), c.b(), a);
        changed = true;
    }
    if let Some(c) = paint_row(ui, &MAIN_COLORS, *color) {
        let a = color.a();
        *color = Color32::from_rgba_unmultiplied(c.r(), c.g(), c.b(), a);
        changed = true;
    }
    for row in &COLOR_VARIATIONS {
        if let Some(c) = paint_row(ui, row, *color) {
            let a = color.a();
            *color = Color32::from_rgba_unmultiplied(c.r(), c.g(), c.b(), a);
            changed = true;
        }
    }

    ui.add_space(theme.spacing.sm);
    let sep = ui.available_rect_before_wrap();
    ui.painter().hline(
        sep.x_range(),
        sep.min.y,
        Stroke::new(DESIGN_TOKENS.stroke.hairline, theme.ui.border_control),
    );
    ui.add_space(theme.spacing.sm);

    // Recent favourites + add + editable hex
    let mut recent = ui.ctx().data(|d| {
        d.get_temp::<Vec<Color32>>(recent_colors_id())
            .unwrap_or_default()
    });
    ui.horizontal(|ui| {
        ui.spacing_mut().item_spacing.x = gap;
        for &c in recent.iter().take(5) {
            let (rect, response) = ui.allocate_exact_size(Vec2::splat(swatch + 2.0), Sense::click());
            ui.painter().rect_filled(rect, rounding, c);
            if response.clicked() {
                let a = color.a();
                *color = Color32::from_rgba_unmultiplied(c.r(), c.g(), c.b(), a);
                changed = true;
            }
        }
        let (add, response) =
            ui.allocate_exact_size(Vec2::splat(swatch + 2.0), Sense::click());
        ui.painter().rect_stroke(
            add,
            rounding,
            Stroke::new(
                DESIGN_TOKENS.stroke.hairline,
                theme.ui.border_control.gamma_multiply(0.85),
            ),
            StrokeKind::Inside,
        );
        let mid = add.center();
        ui.painter().line_segment(
            [Pos2::new(mid.x - 4.0, mid.y), Pos2::new(mid.x + 4.0, mid.y)],
            Stroke::new(1.5, theme.ui.text_muted),
        );
        ui.painter().line_segment(
            [Pos2::new(mid.x, mid.y - 4.0), Pos2::new(mid.x, mid.y + 4.0)],
            Stroke::new(1.5, theme.ui.text_muted),
        );
        if response.clicked() {
            let opaque = color.to_opaque();
            recent.retain(|c| c.to_opaque() != opaque);
            recent.insert(0, opaque);
            recent.truncate(5);
            ui.ctx()
                .data_mut(|d| d.insert_temp(recent_colors_id(), recent.clone()));
        }
        response.on_hover_text(l10n::text(ui.ctx(), "Add to favorites"));

        ui.add_space(theme.spacing.xs);
        if hex_edit(ui, color) {
            changed = true;
        }
    });

    if changed {
        let opaque = color.to_opaque();
        recent.retain(|c| c.to_opaque() != opaque);
        recent.insert(0, opaque);
        recent.truncate(5);
        ui.ctx()
            .data_mut(|d| d.insert_temp(recent_colors_id(), recent));
    }

    ui.add_space(theme.spacing.sm);
    if opacity_bar(ui, color, palette_w) {
        changed = true;
        // Opacity is drawn after the hex field — push the new AA into the draft for next frame.
        ui.ctx()
            .data_mut(|d| d.insert_temp(hex_edit_id(), hex(*color)));
    }

    changed
}

fn hex_edit_id() -> Id {
    Id::unique("color_picker_hex_edit")
}

/// Editable `#RRGGBB` / `#RRGGBBAA` field.
fn hex_edit(ui: &mut Ui, color: &mut Color32) -> bool {
    let theme = ui.active_theme();
    let id = hex_edit_id();
    let current = hex(*color);
    let mut draft = ui.ctx().data_mut(|d| {
        d.get_temp_mut_or_insert_with(id, || current.clone()).clone()
    });
    // Always mirror the live colour when not typing (incl. opacity slider).
    let focused = ui.memory(|m| m.has_focus(id.with("text")));
    if !focused && draft != current {
        draft = current.clone();
        ui.ctx().data_mut(|d| d.insert_temp(id, draft.clone()));
    }

    let mut changed = false;
    let edit_id = id.with("text");
    let output = egui::TextEdit::singleline(&mut draft)
        .id(edit_id)
        .desired_width(96.0)
        .font(egui::FontId::monospace(theme.text.small))
        .margin(egui::Margin::symmetric(6, 4))
        .show(ui);

    ui.ctx().data_mut(|d| d.insert_temp(id, draft.clone()));

    let enter = ui.input(|i| i.key_pressed(egui::Key::Enter));
    if (output.response.lost_focus() || (output.response.has_focus() && enter))
        && let Some(parsed) = parse_hex(&draft)
    {
        if parsed != *color {
            *color = parsed;
            changed = true;
        }
        ui.ctx()
            .data_mut(|d| d.insert_temp(id, hex(*color)));
    }
    changed
}

fn parse_hex(text: &str) -> Option<Color32> {
    let s = text.trim().trim_start_matches('#');
    match s.len() {
        3 => {
            let r = u8::from_str_radix(&s[0..1].repeat(2), 16).ok()?;
            let g = u8::from_str_radix(&s[1..2].repeat(2), 16).ok()?;
            let b = u8::from_str_radix(&s[2..3].repeat(2), 16).ok()?;
            Some(Color32::from_rgb(r, g, b))
        }
        4 => {
            let r = u8::from_str_radix(&s[0..1].repeat(2), 16).ok()?;
            let g = u8::from_str_radix(&s[1..2].repeat(2), 16).ok()?;
            let b = u8::from_str_radix(&s[2..3].repeat(2), 16).ok()?;
            let a = u8::from_str_radix(&s[3..4].repeat(2), 16).ok()?;
            Some(Color32::from_rgba_unmultiplied(r, g, b, a))
        }
        6 => {
            let r = u8::from_str_radix(&s[0..2], 16).ok()?;
            let g = u8::from_str_radix(&s[2..4], 16).ok()?;
            let b = u8::from_str_radix(&s[4..6], 16).ok()?;
            Some(Color32::from_rgb(r, g, b))
        }
        8 => {
            let r = u8::from_str_radix(&s[0..2], 16).ok()?;
            let g = u8::from_str_radix(&s[2..4], 16).ok()?;
            let b = u8::from_str_radix(&s[4..6], 16).ok()?;
            let a = u8::from_str_radix(&s[6..8], 16).ok()?;
            Some(Color32::from_rgba_unmultiplied(r, g, b, a))
        }
        _ => None,
    }
}

/// SS-style opacity control: checkerboard → solid colour track, inset knob, `%` pill.
fn opacity_bar(ui: &mut Ui, color: &mut Color32, width: f32) -> bool {
    let theme = ui.active_theme();
    ui.label(
        egui::RichText::new(l10n::text(ui.ctx(), "Opacity"))
            .size(theme.text.small)
            .color(theme.ui.text_secondary),
    );
    ui.add_space(theme.spacing.xs);

    let track_h = 14.0_f32;
    let knob_r = 7.0_f32;
    let pct_w = 48.0_f32;
    let row_h = (knob_r * 2.0 + 4.0).max(24.0);
    let (row, _) = ui.allocate_exact_size(Vec2::new(width, row_h), Sense::hover());

    let pct_rect = Rect::from_min_size(
        Pos2::new(row.max.x - pct_w, row.center().y - 11.0),
        Vec2::new(pct_w, 22.0),
    );
    // Leave room for the knob so 0%/100% never clip past the track ends.
    let track = Rect::from_min_max(
        Pos2::new(row.min.x + knob_r, row.center().y - track_h * 0.5),
        Pos2::new(pct_rect.min.x - theme.spacing.sm - knob_r, row.center().y + track_h * 0.5),
    );

    let track_resp = ui.interact(
        Rect::from_min_max(
            Pos2::new(row.min.x, track.min.y - 4.0),
            Pos2::new(pct_rect.min.x - theme.spacing.xs, track.max.y + 4.0),
        ),
        ui.scope_id().with("opacity_track"),
        Sense::click_and_drag(),
    );

    let mut pct = f32::from(color.a()) / 255.0;
    let mut changed = false;
    if (track_resp.dragged() || track_resp.clicked())
        && let Some(pos) = track_resp.interact_pointer_pos()
    {
        pct = ((pos.x - track.min.x) / track.width().max(1.0)).clamp(0.0, 1.0);
        let [r, g, b, _] = color.to_srgba_unmultiplied();
        *color = Color32::from_rgba_unmultiplied(r, g, b, (pct * 255.0).round() as u8);
        changed = true;
    }

    let radius = theme.radius.sm;
    let track_painter = ui.painter_at(track);
    let light = Color32::from_rgb(90, 90, 90);
    let dark = Color32::from_rgb(48, 48, 48);
    let cell = 5.0;
    track_painter.rect_filled(track, radius, dark);
    let board_cols = (track.width() / cell).ceil() as i32;
    let board_rows = (track.height() / cell).ceil() as i32;
    for y in 0..board_rows {
        for x in 0..board_cols {
            if (x + y) % 2 == 0 {
                let x0 = track.min.x + x as f32 * cell;
                let y0 = track.min.y + y as f32 * cell;
                let cell_rect = Rect::from_min_max(
                    Pos2::new(x0, y0),
                    Pos2::new((x0 + cell).min(track.max.x), (y0 + cell).min(track.max.y)),
                );
                track_painter.rect_filled(cell_rect, 0.0, light);
            }
        }
    }

    let opaque = color.to_opaque();
    let steps = 32;
    for i in 0..steps {
        let t0 = i as f32 / steps as f32;
        let t1 = (i + 1) as f32 / steps as f32;
        let x0 = track.min.x + t0 * track.width();
        let x1 = track.min.x + t1 * track.width();
        let a = (t1 * 255.0).round() as u8;
        track_painter.rect_filled(
            Rect::from_min_max(
                Pos2::new(x0, track.min.y),
                Pos2::new(x1, track.max.y),
            ),
            0.0,
            Color32::from_rgba_unmultiplied(opaque.r(), opaque.g(), opaque.b(), a),
        );
    }

    track_painter.rect_stroke(
        track,
        radius,
        Stroke::new(
            DESIGN_TOKENS.stroke.hairline,
            theme.ui.border_control.gamma_multiply(0.9),
        ),
        StrokeKind::Inside,
    );

    // Knob + percent use the unclipped painter so they stay fully visible.
    let painter = ui.painter();
    let knob_x = track.min.x + track.width() * pct;
    let knob = Pos2::new(knob_x, track.center().y);
    painter.circle_filled(knob, knob_r, Color32::WHITE);
    painter.circle_stroke(
        knob,
        knob_r,
        Stroke::new(1.0, Color32::from_black_alpha(90)),
    );

    let pct_label = format!("{}%", (pct * 100.0).round() as i32);
    painter.rect_filled(pct_rect, theme.radius.sm, theme.ui.panel_bg_secondary);
    painter.rect_stroke(
        pct_rect,
        theme.radius.sm,
        Stroke::new(DESIGN_TOKENS.stroke.hairline, theme.ui.border_control),
        StrokeKind::Inside,
    );
    painter.text(
        pct_rect.center(),
        egui::Align2::CENTER_CENTER,
        pct_label,
        egui::FontId::proportional(theme.text.small),
        theme.ui.text,
    );

    track_resp.on_hover_cursor(egui::CursorIcon::PointingHand);
    changed
}

#[cfg(test)]
mod tests {
    use super::*;
    use egui::{Context, Event, Key, Modifiers, PointerButton, RawInput, Rect};

    #[test]
    fn hex_row_opens_from_the_current_colour_and_escape_resets_the_popup() {
        for default_state in [false, true] {
            let ctx = Context::default();
            let current = Color32::from_rgb(12, 34, 56);
            let mut state = if default_state {
                ColorPickerState {
                    sel_color: current,
                    ..Default::default()
                }
            } else {
                ColorPickerState::new(current)
            };
            let mut frame = |events: Vec<Event>| {
                let mut hex_row = Rect::NOTHING;
                let mut well = Rect::NOTHING;
                let output = ctx.run_ui(
                    RawInput {
                        screen_rect: Some(Rect::from_min_size(
                            Pos2::ZERO,
                            Vec2::new(1440.0, 900.0),
                        )),
                        events,
                        ..Default::default()
                    },
                    |ui| {
                        well = Rect::from_min_size(ui.cursor().min, Vec2::splat(28.0));
                        ColorPicker::new(&mut state, "picker-test").show(ui);
                    },
                );
                for shape in &output.shapes {
                    if let egui::Shape::Text(text) = &shape.shape
                        && text.galley.job.text == "Hex"
                    {
                        hex_row = text.visual_bounding_rect();
                    }
                }
                output.drop_without_applying_deltas();
                (
                    well,
                    hex_row,
                    state.clone(),
                    ctx.input(|i| i.key_pressed(Key::Escape)),
                )
            };
            let click = |pos| {
                vec![
                    Event::PointerMoved(pos),
                    Event::PointerButton {
                        pos,
                        button: PointerButton::Primary,
                        pressed: true,
                        modifiers: Modifiers::NONE,
                    },
                    Event::PointerButton {
                        pos,
                        button: PointerButton::Primary,
                        pressed: false,
                        modifiers: Modifiers::NONE,
                    },
                ]
            };
            let (well, _, _, _) = frame(vec![]);
            frame(click(well.center()));
            let (_, row, _, _) = frame(vec![]);
            assert!(row.is_finite());
            frame(click(row.center()));
            let (_, _, state, _) = frame(vec![]);
            assert!(state.is_open && state.custom_mode, "Hex closed the popup");
            assert_eq!(state.custom_color, current);
            let (_, _, state, escape) = frame(vec![Event::Key {
                key: Key::Escape,
                physical_key: None,
                pressed: true,
                repeat: false,
                modifiers: Modifiers::NONE,
            }]);
            assert!(!state.is_open && !state.custom_mode);
            assert!(!escape, "Escape must be consumed");
            let (_, _, state, _) = frame(click(well.center()));
            assert!(state.is_open && !state.custom_mode);
        }
    }

    #[test]
    fn swatches_only_ring_selection_and_edge_low_contrast_colours() {
        for (name, theme) in crate::theme::presets() {
            let ctx = Context::default();
            crate::theme::apply_to_egui(&ctx, &theme);
            let colours = [
                Color32::WHITE,
                Color32::from_gray(46),
                Color32::from_gray(15),
                Color32::BLACK,
                COLOR_VARIATIONS[0][0],
            ];
            for selected in [
                Color32::from_rgb(12, 34, 56),
                Color32::BLACK,
                Color32::WHITE,
            ] {
                let mut state = ColorPickerState::new(selected);
                let output = ctx.run_ui(Default::default(), |ui| {
                    ColorPicker::new(&mut state, "row").draw_color_row(ui, &colours);
                });
                let rects: Vec<_> = output
                    .shapes
                    .iter()
                    .filter_map(|s| match &s.shape {
                        egui::Shape::Rect(r) => Some(r.clone()),
                        _ => None,
                    })
                    .collect();
                output.drop_without_applying_deltas();
                let rings: Vec<_> = rects.iter().filter(|r| r.stroke.width == 2.0).collect();
                assert_eq!(
                    rings.len(),
                    usize::from(colours.contains(&selected)),
                    "{name}: {selected:?}"
                );
                if let Some(ring) = rings.first() {
                    assert_eq!(ring.stroke.color, theme.ui.accent);
                }
                for colour in colours {
                    if crate::theme::contrast(colour, theme.ui.panel_bg_floating) < 1.5 {
                        let swatch = rects.iter().find(|r| r.fill == colour).unwrap();
                        assert!(
                            rects.iter().any(|r| r.rect == swatch.rect
                                && r.stroke.width == 1.0
                                && r.stroke_kind == StrokeKind::Inside),
                            "{name}: {colour:?}"
                        );
                    }
                }
            }
        }
    }
}
