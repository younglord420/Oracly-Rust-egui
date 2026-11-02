//! Drawing properties dialog - modal popup for editing drawing properties.

use crate::localization as l10n;
use egui::{Context, Event, Key, Modifiers, Ui, Vec2};

use super::actions::{DrawingId, DrawingPropertiesAction, DrawingProps};
use super::config::DrawingPropertiesConfig;
use super::tabs::{
    CoordinatesState, CoordinatesTab, InputsTab, PropertiesTab, StyleTab, TextTab, VisibilityTab,
};
use crate::drawings::{Drawing, DrawingTemplate};
use crate::services::StoreKind;
use crate::theme::ThemeExt;
use crate::ui::dialogs::templates::TemplateStore;
use crate::ui::drawing_toolbar::{DrawingToolbarAction, utilities::TemplatesButton};
use crate::ui_kit::ColorPickerState;
use crate::ui_kit::buttons::Button;
use crate::ui_kit::dialog::{DialogFooter, DialogFrame, dialog_header};

/// Drawing properties dialog
pub struct DrawingPropertiesDialog {
    /// Display and layout options for this component.
    pub config: DrawingPropertiesConfig,
    /// Whether the dialog is currently visible.
    pub is_open: bool,
    /// Identifier of the drawing being edited, when open.
    pub drawing_id: Option<DrawingId>,
    /// Display name of the drawing being edited.
    pub drawing_name: String,
    /// Section to bring into view (hosts may set this to jump, e.g. Text editing).
    pub active_tab: PropertiesTab,
    /// Editable drawing appearance and tool settings.
    pub props: DrawingProps,
    /// Editable drawing anchors.
    pub coordinates: CoordinatesState,
    /// Named templates, kept in memory unless the host attaches its storage service.
    pub templates: TemplateStore<DrawingTemplate>,
    preview: Option<(String, DrawingId, Vec<PreviewDrawing>)>,
    template_name: String,
    saving_template: bool,
    default_template: Option<DrawingTemplate>,
    /// Clear leftover popups (e.g. Template menu) once on the next show after open/close.
    clear_popups: bool,
    /// Incremented on each `open` so section fold states reset cleanly.
    open_generation: u64,
    /// Validation error from the most recent edit.
    pub error: Option<String>,
}

impl Default for DrawingPropertiesDialog {
    fn default() -> Self {
        Self::new()
    }
}

impl DrawingPropertiesDialog {
    /// Create a [`DrawingPropertiesDialog`] with default settings.
    pub fn new() -> Self {
        Self {
            config: DrawingPropertiesConfig::default(),
            is_open: false,
            drawing_id: None,
            drawing_name: String::new(),
            active_tab: PropertiesTab::Style,
            props: DrawingProps::default(),
            coordinates: CoordinatesState::default(),
            error: None,
            templates: TemplateStore::new(StoreKind::DrawingTemplates),
            preview: None,
            template_name: String::new(),
            saving_template: false,
            default_template: None,
            clear_popups: false,
            open_generation: 0,
        }
    }

    /// Open the dialog for a specific drawing
    pub fn open(&mut self, id: DrawingId, name: String, props: DrawingProps) {
        self.is_open = true;
        self.drawing_id = Some(id);
        self.drawing_name = name;
        self.coordinates = props.coordinates.clone();
        self.error = None;
        self.template_name.clear();
        self.saving_template = false;
        self.clear_popups = true;
        self.open_generation = self.open_generation.wrapping_add(1);
        self.default_template = props.tool_settings.as_ref().map(|template| {
            DrawingTemplate::from_drawing(&Drawing::new(0, template.settings.tool_type))
        });
        self.props = props;
        self.active_tab = PropertiesTab::Style;
    }

    /// Open with the selected drawing's saved properties and all of its coordinates.
    pub fn open_for_chart(&mut self, chart: &crate::widget::Chart, id: DrawingId) -> bool {
        let Some(drawing) = chart.drawings().drawings.iter().find(|d| d.id == id.0) else {
            return false;
        };
        self.open(
            id,
            drawing.tool_type.as_str().into(),
            DrawingProps::from_drawing(drawing, chart),
        );
        if let Some(defaults) = chart.drawings().tool_defaults.get(&drawing.tool_type) {
            self.default_template = Some(defaults.clone());
        }
        true
    }

    /// Preview validated edits while open; Ok commits one undoable edit and Cancel restores
    /// the original properties. Apply is a notification the host must not apply a second time.
    /// Call every frame, including after closing externally, to restore an unfinished preview.
    pub fn show_and_apply(
        &mut self,
        ctx: &Context,
        chart: &mut crate::widget::Chart,
    ) -> DrawingPropertiesAction {
        if self
            .preview
            .as_ref()
            .is_some_and(|(_, id, _)| !self.is_open || self.drawing_id != Some(*id))
        {
            self.restore_preview(chart);
        }
        let action = self.show(ctx);
        if !self.is_open || matches!(action, DrawingPropertiesAction::Apply(..)) {
            self.restore_preview(chart);
        }
        if let DrawingPropertiesAction::Apply(id, props) = &action {
            if let Err(error) = props.apply_to_chart(*id, chart) {
                self.error = Some(error.into());
                self.is_open = true;
                return DrawingPropertiesAction::None;
            }
        } else if self.is_open
            && let Some(id) = self.drawing_id
        {
            let mut props = self.props.clone();
            props.coordinates = self.coordinates.clone();
            match props.edited_drawings(id, chart) {
                Ok(edits) => {
                    if self.props.save_as_default
                        && let Some(mut template) = self.current_template()
                    {
                        template.settings.visible = true;
                        chart
                            .drawings_mut()
                            .tool_defaults
                            .insert(template.settings.tool_type, template.clone());
                        self.default_template = Some(template);
                        self.props.save_as_default = false;
                    }
                    let changed = chart
                        .drawings()
                        .drawings
                        .iter()
                        .find(|d| d.id == id.0)
                        .is_some_and(|current| {
                            serde_json::to_value(DrawingTemplate::from_drawing(current)).ok()
                                != self
                                    .current_template()
                                    .and_then(|template| serde_json::to_value(template).ok())
                        })
                        || !props
                            .coordinates
                            .resolve(chart)
                            .unwrap_or_default()
                            .is_empty();
                    if !changed {
                        self.error = None;
                        return action;
                    }
                    let (_, _, originals) = self
                        .preview
                        .get_or_insert_with(|| (chart.symbol().to_owned(), id, Vec::new()));
                    for edit in &edits {
                        if !originals
                            .iter()
                            .any(|original| original.drawing.id == edit.id)
                            && let Some(drawing) =
                                chart.drawings().drawings.iter().find(|d| d.id == edit.id)
                        {
                            originals.push(PreviewDrawing {
                                drawing: drawing.clone(),
                                times: drawing
                                    .chart_points
                                    .iter()
                                    .map(|p| chart.drawing_point_time(p))
                                    .collect(),
                            });
                        }
                    }
                    for edit in edits {
                        if let Some(drawing) = chart
                            .drawings_mut()
                            .drawings
                            .iter_mut()
                            .find(|d| d.id == edit.id)
                        {
                            *drawing = edit;
                        }
                    }
                    self.error = None;
                }
                Err(error) => self.error = Some(error.into()),
            }
        }
        action
    }

    fn restore_preview(&mut self, chart: &mut crate::widget::Chart) {
        let Some((symbol, _, originals)) = self.preview.take() else {
            return;
        };
        if symbol != chart.symbol() {
            return;
        }
        for original in originals {
            let points: Vec<_> = original
                .drawing
                .chart_points
                .iter()
                .zip(&original.times)
                .map(|(point, time)| {
                    time.and_then(|time| chart.drawing_point_at(time, point.price))
                        .filter(|mapped| mapped.bar_idx != point.bar_idx)
                        .unwrap_or(*point)
                })
                .collect();
            if let Some(drawing) = chart.drawings_mut().drawings.iter_mut().find(|d| {
                d.id == original.drawing.id
                    && d.tool_type == original.drawing.tool_type
                    && d.stored_id == original.drawing.stored_id
            }) {
                let old = original.drawing;
                drawing.color = old.color;
                drawing.stroke_width = old.stroke_width;
                drawing.line_style = old.line_style;
                drawing.fill_color = old.fill_color;
                drawing.extend_left = old.extend_left;
                drawing.extend_right = old.extend_right;
                drawing.text = old.text;
                drawing.font_size = old.font_size;
                drawing.timeframe_visibility = old.timeframe_visibility;
                drawing.fib_config = old.fib_config;
                drawing.pitchfork = old.pitchfork;
                drawing.gann = old.gann;
                drawing.ghost_feed = old.ghost_feed;
                drawing.position_size = old.position_size;
                drawing.position_sizing = old.position_sizing;
                drawing.volume_profile = old.volume_profile;
                drawing.regression_trend = old.regression_trend;
                drawing.channel_midline = old.channel_midline;
                drawing.show_price_labels = old.show_price_labels;
                drawing.font_weight = old.font_weight;
                drawing.font_italic = old.font_italic;
                drawing.text_color = old.text_color;
                drawing.text_v_align = old.text_v_align;
                drawing.text_h_align = old.text_h_align;
                drawing.arrow_start_style = old.arrow_start_style;
                drawing.arrow_end_style = old.arrow_end_style;
                drawing.custom_settings = old.custom_settings;
                drawing.chart_points = points;
                drawing.screen_anchor = old.screen_anchor;
            }
        }
    }

    /// Close the dialog
    pub fn close(&mut self) {
        self.is_open = false;
        self.drawing_id = None;
        self.clear_popups = true;
    }

    /// Sync dialog open/close state from an external flag.
    ///
    /// Opening requires drawing context, so only close is synced here.
    pub fn sync_open_state(&mut self, should_be_open: bool) {
        if !should_be_open && self.is_open {
            self.close();
        }
    }

    /// Show the dialog
    pub fn show(&mut self, ctx: &Context) -> DrawingPropertiesAction {
        self.templates.poll(ctx);
        if self.clear_popups {
            egui::Popup::close_all(ctx);
            self.clear_popups = false;
        }
        if !self.is_open {
            return DrawingPropertiesAction::None;
        }

        let mut action = DrawingPropertiesAction::None;

        // Compact tools should not inherit the maximum dialog height. Auto-height keeps the
        // footer directly below the visible sections, while a configured positive height remains
        // available to hosts that deliberately want a fixed-size dialog.
        let frame = if self.config.height > 0.0 {
            DialogFrame::new(
                "Drawing Properties",
                Vec2::new(self.config.width, self.config.height),
            )
        } else {
            DialogFrame::with_width("Drawing Properties", self.config.width)
        };
        frame.show(ctx, |ui| {
            action = self.render_contents(ui);
        });

        action
    }

    fn current_template(&self) -> Option<DrawingTemplate> {
        let mut template = self.props.tool_settings.as_deref()?.clone();
        let d = &mut template.settings;
        d.color = self.props.color.to_srgba_unmultiplied();
        d.stroke_width = self.props.line_width;
        d.line_style = self.props.line_style;
        d.fill_color = self.props.fill_enabled.then(|| {
            self.props
                .fill_color
                .unwrap_or(self.props.color.gamma_multiply(0.2))
                .to_srgba_unmultiplied()
        });
        d.extend_left = self.props.extend_left;
        d.extend_right = self.props.extend_right;
        d.text = (!self.props.label.is_empty()).then(|| self.props.label.clone());
        d.font_size = self.props.font_size;
        d.timeframe_visibility = crate::drawings::TimeframeVisibility {
            show_on_all: self.props.visible_all_timeframes,
            visible_timeframes: self.props.visible_timeframes.iter().cloned().collect(),
        };
        Some(template)
    }

    fn load_template(&mut self, template: DrawingTemplate) {
        if self.props.tool_settings.as_ref().is_none_or(|current| {
            current.settings.tool_type != template.settings.tool_type
                || current.settings.custom_tool != template.settings.custom_tool
        }) {
            return;
        }
        let d = &template.settings;
        self.props.color = d.color32();
        self.props.line_width = d.stroke_width;
        self.props.line_style = d.line_style;
        self.props.fill_color = d.fill_color32();
        self.props.fill_enabled = d.fill_color.is_some();
        self.props.extend_left = d.extend_left;
        self.props.extend_right = d.extend_right;
        self.props.label = d.text.clone().unwrap_or_default();
        self.props.font_size = d.font_size;
        self.props.visible_all_timeframes = d.timeframe_visibility.show_on_all;
        self.props.visible_timeframes = d
            .timeframe_visibility
            .visible_timeframes
            .iter()
            .cloned()
            .collect();
        self.props.tool_settings = Some(Box::new(template));
    }

    fn template_menu(&mut self, ui: &mut Ui) {
        let mut load = None;
        let menu = Button::new("Template").outlined().show(ui);
        let menu_w = 240.0;
        egui::Popup::menu(&menu)
            .frame(crate::ui_kit::glass_menu(ui))
            .width(menu_w)
            .show(|ui| {
                let inner_w = menu_w - 8.0;
                ui.set_width(inner_w);
                ui.set_max_width(inner_w);
                ui.spacing_mut().item_spacing.y = 2.0;

                if Self::template_menu_row(ui, &l10n::text(ui.ctx(), "Save as template…"), inner_w)
                {
                    self.saving_template = true;
                    ui.close();
                }
                if Self::template_menu_row(
                    ui,
                    &l10n::text(ui.ctx(), "Save as default for this tool"),
                    inner_w,
                ) {
                    self.props.save_as_default = true;
                    ui.close();
                }
                if Self::template_menu_row(
                    ui,
                    &l10n::text(ui.ctx(), "Reset to defaults"),
                    inner_w,
                ) {
                    load = self.default_template.clone();
                    ui.close();
                }

                let mut delete: Option<String> = None;
                if let Some(current) = &self.props.tool_settings {
                    let matching: Vec<(String, DrawingTemplate)> = self
                        .templates
                        .entries()
                        .iter()
                        .filter(|(_, template)| {
                            template.settings.tool_type == current.settings.tool_type
                                && template.settings.custom_tool == current.settings.custom_tool
                        })
                        .map(|(n, t)| (n.clone(), t.clone()))
                        .collect();

                    if !matching.is_empty() {
                        ui.add_space(4.0);
                        ui.separator();
                        ui.add_space(4.0);
                    }

                    for (name, template) in matching {
                        ui.horizontal(|ui| {
                            ui.spacing_mut().item_spacing.x = 4.0;
                            let load_w = (inner_w - 76.0).max(80.0);
                            if Self::template_menu_row(ui, &name, load_w) {
                                load = Some(template);
                                ui.close();
                            }
                            let del_label = l10n::format_text(
                                ui.ctx(),
                                "Delete {name}",
                                &[("name", name.clone())],
                            );
                            if Self::template_delete_chip(ui, &del_label, 68.0) {
                                delete = Some(name);
                            }
                        });
                    }
                }
                if let Some(name) = delete
                    && let Err(error) = self.templates.delete(&name)
                {
                    self.templates.error = Some(error);
                }
            });
        if let Some(template) = load {
            self.load_template(template);
        }
        if self.saving_template
            && let Some(action) =
                TemplatesButton::show_name_dialog(ui, menu.rect, &mut self.template_name)
        {
            match action {
                DrawingToolbarAction::SaveTemplate { name } => {
                    if let Some(template) = self.current_template() {
                        let saved = self
                            .props
                            .validate()
                            .map_err(str::to_owned)
                            .and_then(|()| self.templates.save(&name, template));
                        match saved {
                            Ok(()) => {
                                self.saving_template = false;
                                self.template_name.clear();
                            }
                            Err(error) => self.templates.error = Some(error),
                        }
                    }
                }
                _ => self.saving_template = false,
            }
        }
        if let Some(error) = &self.templates.error {
            ui.colored_label(ui.visuals().error_fg_color, l10n::text(ui.ctx(), error));
        }
    }

    fn template_menu_row(ui: &mut Ui, label: &str, width: f32) -> bool {
        let theme = ui.active_theme();
        let (rect, response) = ui.allocate_exact_size(egui::vec2(width, 30.0), egui::Sense::click());
        if response.hovered() {
            ui.painter()
                .rect_filled(rect, egui::CornerRadius::same(4), theme.ui.btn_bg_hover);
        }
        let galley = ui.painter().layout_no_wrap(
            label.to_owned(),
            egui::FontId::proportional(theme.text.body),
            theme.ui.text,
        );
        ui.painter().galley(
            egui::pos2(rect.min.x + 8.0, rect.center().y - galley.size().y * 0.5),
            galley,
            theme.ui.text,
        );
        response.clicked()
    }

    fn template_delete_chip(ui: &mut Ui, a11y_label: &str, width: f32) -> bool {
        let theme = ui.active_theme();
        let (rect, response) = ui.allocate_exact_size(egui::vec2(width, 30.0), egui::Sense::click());
        if response.hovered() {
            ui.painter()
                .rect_filled(rect, egui::CornerRadius::same(4), theme.ui.btn_bg_hover);
        }
        let color = if response.hovered() {
            theme.ui.text
        } else {
            theme.ui.text_muted
        };
        let galley = ui.painter().layout_no_wrap(
            a11y_label.to_owned(),
            egui::FontId::proportional(theme.text.label),
            color,
        );
        ui.painter().galley(
            egui::pos2(
                rect.center().x - galley.size().x * 0.5,
                rect.center().y - galley.size().y * 0.5,
            ),
            galley,
            color,
        );
        response.clicked()
    }

    fn render_contents(&mut self, ui: &mut Ui) -> DrawingPropertiesAction {
        let ctx = ui.ctx().clone();
        // This dialog follows the toolbars' neutral interaction language. Accent remains reserved
        // for semantic selection; pressing or focusing a control uses the theme's grey outline.
        let control_border = ui.active_theme().ui.border_control;
        ui.visuals_mut().widgets.active.bg_stroke =
            egui::Stroke::new(crate::tokens::DESIGN_TOKENS.stroke.hairline, control_border);
        // Read before the body: a combo popup closes itself on the Escape it receives there.
        let popup_open = egui::Popup::is_any_open(&ctx);
        let state_id = ui.scope_id().with("keyboard_and_pickers");
        let mut state: UiState = ui.data(|d| d.get_temp(state_id)).unwrap_or_default();

        let mut cancel = dialog_header(ui, &self.drawing_name);
        // Prevent dense control rows from widening the fixed dialog frame.
        let dialog_width = ui.available_width();
        ui.set_max_width(dialog_width);
        let tool = self
            .props
            .tool_settings
            .as_ref()
            .map(|t| t.settings.tool_type);
        let has_inputs = tool.is_some_and(InputsTab::available);
        let has_text = tool.is_some_and(|t| t.supports_text_style());
        let has_fill = tool.is_some_and(|t| t.supports_fill());
        let section_available = |tab: PropertiesTab| match tab {
            PropertiesTab::Inputs => has_inputs,
            PropertiesTab::Text => has_text,
            _ => true,
        };
        if !section_available(self.active_tab) {
            self.active_tab = PropertiesTab::Style;
        }
        if state.open_generation != self.open_generation {
            state.open_generation = self.open_generation;
            state.open_appearance = true;
            state.open_fill = true;
            state.open_label = false;
            state.open_inputs = false;
            state.open_coordinates = false;
            state.open_visibility = true;
            state.scrolled_tab = None;
        }
        // Scroll and unfold once when a host jumps to a section via `active_tab` (e.g. Open Text).
        let scroll_to = (state.scrolled_tab != Some(self.active_tab)).then_some(self.active_tab);
        if let Some(tab) = scroll_to {
            match tab {
                PropertiesTab::Style => state.open_appearance = true,
                PropertiesTab::Text => state.open_label = true,
                PropertiesTab::Inputs => state.open_inputs = true,
                PropertiesTab::Coordinates => state.open_coordinates = true,
                PropertiesTab::Visibility => state.open_visibility = true,
            }
        }

        let avail_body_h = (ui.available_height()
            - FOOTER_HEIGHT
            - if self.error.is_some() { 40.0 } else { 0.0 })
        .max(0.0);
        let max_scroll_h = if self.config.height > 0.0 {
            avail_body_h
        } else {
            avail_body_h.min(360.0)
        };

        // Vertical auto-shrink stays on: collapsed sections shrink the dialog, while expanded
        // sections scroll smoothly within `max_scroll_h` above the pinned footer.
        egui::ScrollArea::vertical()
            .id_salt("drawing_properties_sections")
            .auto_shrink([false, true])
            .max_height(max_scroll_h)
            .show(ui, |ui| {
                // Keep dense rows wrapping inside the dialog instead of growing it.
                let body_width = ui.available_width();
                ui.set_width(body_width);
                ui.set_max_width(body_width);

                let mut first = true;
                let mut begin_section = |ui: &mut Ui, tab: Option<PropertiesTab>| {
                    if !first {
                        super::section::section_divider(ui);
                    }
                    first = false;
                    tab.is_some_and(|t| scroll_to == Some(t))
                };

                let scroll_appearance = begin_section(ui, Some(PropertiesTab::Style));
                super::section::properties_section(
                    ui,
                    "drawing_props_appearance",
                    "Appearance",
                    &mut state.open_appearance,
                    scroll_appearance,
                    |ui| StyleTab::show(ui, &mut self.props, &mut state.pickers),
                );

                if has_text {
                    let scroll = begin_section(ui, Some(PropertiesTab::Text));
                    super::section::properties_section(
                        ui,
                        "drawing_props_label",
                        "Label",
                        &mut state.open_label,
                        scroll,
                        |ui| TextTab::show(ui, &mut self.props, &mut state.text_picker),
                    );
                }

                if has_fill {
                    let scroll = begin_section(ui, None);
                    super::section::properties_section(
                        ui,
                        "drawing_props_fill",
                        "Fill",
                        &mut state.open_fill,
                        scroll,
                        |ui| StyleTab::show_fill(ui, &mut self.props, &mut state.pickers),
                    );
                }

                if has_inputs {
                    let scroll = begin_section(ui, Some(PropertiesTab::Inputs));
                    super::section::properties_section(
                        ui,
                        "drawing_props_inputs",
                        "Inputs",
                        &mut state.open_inputs,
                        scroll,
                        |ui| {
                            let color = self.props.color.to_srgba_unmultiplied();
                            if let Some(template) = &mut self.props.tool_settings {
                                template.settings.color = color;
                                InputsTab::show(
                                    ui,
                                    &mut template.settings,
                                    &mut state.level_pickers,
                                );
                            }
                        },
                    );
                }

                let scroll_coords = begin_section(ui, Some(PropertiesTab::Coordinates));
                super::section::properties_section(
                    ui,
                    "drawing_props_coordinates",
                    "Coordinates",
                    &mut state.open_coordinates,
                    scroll_coords,
                    |ui| CoordinatesTab::show(ui, &mut self.coordinates),
                );

                let scroll_vis = begin_section(ui, Some(PropertiesTab::Visibility));
                super::section::properties_section(
                    ui,
                    "drawing_props_visibility",
                    "Visibility",
                    &mut state.open_visibility,
                    scroll_vis,
                    |ui| VisibilityTab::show(ui, &mut self.props),
                );
            });
        state.scrolled_tab = Some(self.active_tab);

        if let Some(error) = &self.error {
            ui.colored_label(ui.visuals().error_fg_color, l10n::text(ui.ctx(), error));
        }

        let (mut apply, cancel_clicked) = DialogFooter::new("Ok")
            .secondary("Cancel")
            .show_with_left(ui, |ui| self.template_menu(ui));
        cancel |= cancel_clicked;

        // An open popup, colour picker or focused field takes Escape and Enter first. egui clears
        // the focus on Escape before this frame runs, so the focus is read from the previous frame.
        // Only a fresh press acts: the repeats of a key held after it closed a popup, picker or
        // field must not go on to cancel or apply the dialog.
        let consume = |key| {
            ctx.input_mut(|i| {
                let fresh = i.events.iter().any(|e| {
                    matches!(e, Event::Key { key: k, pressed: true, repeat: false, modifiers, .. }
                        if *k == key && modifiers.is_none())
                });
                fresh && i.consume_key(Modifiers::NONE, key)
            })
        };
        if state
            .pickers
            .iter()
            .chain(std::iter::once(&state.text_picker))
            .chain(&state.level_pickers)
            .any(|p| p.is_open)
        {
            if consume(Key::Escape) {
                state
                    .pickers
                    .iter_mut()
                    .chain(std::iter::once(&mut state.text_picker))
                    .chain(&mut state.level_pickers)
                    .for_each(|p| p.is_open = false);
            }
        } else if !popup_open && !state.had_focus {
            cancel |= consume(Key::Escape);
            apply |= !cancel && !self.saving_template && consume(Key::Enter);
        }
        state.had_focus = ctx.memory(|m| m.focused()).is_some();

        let action = match self.drawing_id {
            Some(id) if apply => {
                self.props.coordinates = self.coordinates.clone();
                DrawingPropertiesAction::Apply(id, self.props.clone())
            }
            _ if cancel => DrawingPropertiesAction::Cancel,
            _ => DrawingPropertiesAction::None,
        };
        // A closed dialog starts with fresh keyboard and picker state next time.
        if matches!(action, DrawingPropertiesAction::None) {
            ui.data_mut(|d| d.insert_temp(state_id, state));
        } else {
            self.is_open = false;
            ui.data_mut(|d| d.remove::<UiState>(state_id));
        }
        action
    }
}

/// Height reserved for the properties footer below the scroll body.
const FOOTER_HEIGHT: f32 = 60.0;

/// Keyboard and colour picker state kept in egui memory between frames.
#[derive(Clone, Default)]
struct UiState {
    /// Colour and fill colour pickers of the Style section.
    pickers: [ColorPickerState; 2],
    /// Text colour picker of the Label section.
    text_picker: ColorPickerState,
    /// Pickers for the current tool's editable levels.
    level_pickers: Vec<ColorPickerState>,
    /// Whether a widget had keyboard focus at the end of the previous frame.
    had_focus: bool,
    /// Last section scrolled into view for `active_tab` deep-links.
    scrolled_tab: Option<PropertiesTab>,
    /// Generation of `open()` this state was initialized for.
    open_generation: u64,
    open_appearance: bool,
    open_label: bool,
    open_fill: bool,
    open_inputs: bool,
    open_coordinates: bool,
    open_visibility: bool,
}

/// Only previewed drawings are restored; incoming host drawings and metadata stay intact.
struct PreviewDrawing {
    drawing: Drawing,
    times: Vec<Option<chrono::DateTime<chrono::Utc>>>,
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::drawings::{ChartPoint, DrawingToolType as Tool};
    use crate::model::BarData;
    use crate::widget::Chart;
    use egui::{Color32, Pos2, Rect, Shape};

    fn frame(
        dialog: &mut DrawingPropertiesDialog,
        chart: &mut Chart,
        ctx: &Context,
        events: Vec<Event>,
    ) -> Vec<(String, Pos2)> {
        let mut output = ctx.run_ui(
            egui::RawInput {
                screen_rect: Some(Rect::from_min_size(Pos2::ZERO, egui::vec2(1440.0, 900.0))),
                events,
                ..Default::default()
            },
            |ui| {
                dialog.show_and_apply(ui.ctx(), chart);
            },
        );
        let text = output
            .shapes
            .iter()
            .filter_map(|s| match &s.shape {
                Shape::Text(t) => Some((t.galley.text().to_owned(), t.pos + t.galley.size() / 2.0)),
                _ => None,
            })
            .collect();
        output.textures_delta.clear();
        output.drop_without_applying_deltas();
        text
    }

    fn click(dialog: &mut DrawingPropertiesDialog, chart: &mut Chart, ctx: &Context, label: &str) {
        let texts = frame(dialog, chart, ctx, vec![]);
        let pos = texts
            .iter()
            .find(|(text, _)| text == label)
            .unwrap_or_else(|| panic!("Missing {label}: {texts:?}"))
            .1;
        frame(dialog, chart, ctx, vec![Event::PointerMoved(pos)]);
        for pressed in [true, false] {
            frame(
                dialog,
                chart,
                ctx,
                vec![Event::PointerButton {
                    pos,
                    button: egui::PointerButton::Primary,
                    pressed,
                    modifiers: Modifiers::NONE,
                }],
            );
        }
    }

    /// Unsupported settings never appear, including when reusing the dialog after a text tool.
    #[test]
    fn properties_rows_follow_the_tool_renderer() {
        for theme in [crate::theme::Theme::light(), crate::theme::Theme::dark()] {
            let ctx = Context::default();
            crate::theme::apply_to_egui(&ctx, &theme);
            let mut chart = Chart::new(BarData::new());
            let mut dialog = DrawingPropertiesDialog::new();
            for (tool, fill, extend, text) in [
                (Tool::HorizontalLine, false, false, true),
                (Tool::Rect, true, true, true),
                (Tool::Brush, false, false, true),
                (Tool::TrendLine, false, true, true),
                (Tool::TextLabel, true, false, true),
                (Tool::Highlighter, false, false, true),
            ] {
                chart.drawings_mut().drawings = vec![Drawing::new(7, tool)];
                dialog.open_for_chart(&chart, DrawingId(7));
                frame(&mut dialog, &mut chart, &ctx, vec![]);
                let labels = frame(&mut dialog, &mut chart, &ctx, vec![]);
                for (label, present) in [
                    ("Fill", fill),
                    ("Extend", extend),
                    ("Appearance", true),
                    ("Coordinates", true),
                    ("Visibility", true),
                    ("Label", text),
                    ("Template", true),
                    ("Ok", true),
                    ("Delete", false),
                    ("Clone", false),
                ] {
                    assert_eq!(
                        labels.iter().any(|(text, _)| text == label),
                        present,
                        "{tool:?}: {label}"
                    );
                }
                dialog.close();
                frame(&mut dialog, &mut chart, &ctx, vec![]);
            }
        }
    }

    /// Preview edits leave both histories alone; accepting a sequence commits one bulk edit.
    #[test]
    fn preview_edits_commit_once_or_restore_without_history() {
        for finish in ["Ok", "Cancel", "Escape", "close", "reopen"] {
            let ctx = Context::default();
            let mut chart = Chart::new(BarData::new());
            for (id, width) in [(7, 2.0), (8, 4.0), (9, 6.0)] {
                let mut drawing = Drawing::new(id, Tool::TrendLine);
                drawing.stroke_width = width;
                drawing.chart_points =
                    vec![ChartPoint::new(2.0, 100.0), ChartPoint::new(5.0, 110.0)];
                chart.drawings_mut().drawings.push(drawing);
            }
            chart.user_action(|chart| {
                chart
                    .drawings_mut()
                    .edit_drawing(9, |d| d.stroke_width = 8.0)
            });
            assert!(chart.undo());
            chart.drawings_mut().select_drawings([7, 8]);
            let mut dialog = DrawingPropertiesDialog::new();
            dialog.open_for_chart(&chart, DrawingId(7));
            frame(&mut dialog, &mut chart, &ctx, vec![]);
            assert_eq!(
                chart.drawings().drawings[1].stroke_width,
                4.0,
                "opening a selection is not an edit"
            );
            assert!(!chart.can_undo());
            for width in [3.0, 5.0] {
                dialog.props.line_width = width;
                dialog.props.color = Color32::GREEN;
                dialog.coordinates.points[1].price = "125".into();
                frame(&mut dialog, &mut chart, &ctx, vec![]);
                assert_eq!(
                    chart.drawings().drawings[0].stroke_width,
                    width,
                    "{finish}: live preview"
                );
                assert_eq!(chart.drawings().drawings[1].stroke_width, width);
                assert_eq!(chart.drawings().drawings[2].stroke_width, 6.0);
                assert!(!chart.can_undo());
                assert!(chart.can_redo(), "preview must retain existing redo");
                assert!(!chart.drawings().can_undo());
            }
            match finish {
                "Ok" | "Cancel" => click(&mut dialog, &mut chart, &ctx, finish),
                "Escape" => {
                    frame(
                        &mut dialog,
                        &mut chart,
                        &ctx,
                        vec![Event::Key {
                            key: Key::Escape,
                            physical_key: None,
                            pressed: true,
                            repeat: false,
                            modifiers: Modifiers::NONE,
                        }],
                    );
                }
                _ => {
                    if finish == "reopen" {
                        dialog.open_for_chart(&chart, DrawingId(9));
                        frame(&mut dialog, &mut chart, &ctx, vec![]);
                        assert_eq!(chart.drawings().drawings[0].stroke_width, 2.0);
                        assert_eq!(chart.drawings().drawings[1].stroke_width, 4.0);
                    }
                    dialog.close();
                    frame(&mut dialog, &mut chart, &ctx, vec![]);
                }
            }
            assert!(!dialog.is_open, "{finish}");
            if finish == "Ok" {
                assert_eq!(chart.drawings().drawings[0].stroke_width, 5.0);
                assert_eq!(chart.drawings().drawings[0].chart_points[1].price, 125.0);
                assert!(chart.undo());
            }
            assert_eq!(chart.drawings().drawings[0].stroke_width, 2.0, "{finish}");
            assert_eq!(chart.drawings().drawings[1].stroke_width, 4.0, "{finish}");
            assert_eq!(
                chart.drawings().drawings[0].chart_points[1].price,
                110.0,
                "{finish}"
            );
            assert!(!chart.can_undo(), "{finish}: exactly one accepted step");
            if finish == "Ok" {
                assert!(chart.redo());
                assert_eq!(chart.drawings().drawings[0].stroke_width, 5.0);
            } else {
                assert!(chart.redo(), "{finish}: cancellation retains existing redo");
                assert_eq!(chart.drawings().drawings[2].stroke_width, 8.0);
                assert_eq!(chart.drawings().drawings[0].stroke_width, 2.0);
            }
        }
    }

    /// Templates act on this tool's style, remain usable after cancelling a preview and can be
    /// deleted from the menu.
    #[test]
    fn footer_templates_apply_save_defaults_and_reset() {
        let ctx = Context::default();
        let mut chart = Chart::new(BarData::new());
        let mut drawing = Drawing::new(7, Tool::TrendLine);
        drawing.arrow_end_style = crate::drawings::ArrowStyle::Circle;
        chart.drawings_mut().drawings.push(drawing);
        let mut dialog = DrawingPropertiesDialog::new();
        dialog.open_for_chart(&chart, DrawingId(7));
        frame(&mut dialog, &mut chart, &ctx, vec![]);
        dialog.props.line_width = 7.0;
        click(&mut dialog, &mut chart, &ctx, "Template");
        click(&mut dialog, &mut chart, &ctx, "Save as template…");
        frame(
            &mut dialog,
            &mut chart,
            &ctx,
            vec![Event::Text("Thick line".into())],
        );
        click(&mut dialog, &mut chart, &ctx, "Save");
        click(&mut dialog, &mut chart, &ctx, "Template");
        click(&mut dialog, &mut chart, &ctx, "Reset to defaults");
        click(&mut dialog, &mut chart, &ctx, "Template");
        click(&mut dialog, &mut chart, &ctx, "Thick line");
        assert_eq!(chart.drawings().drawings[0].stroke_width, 7.0);
        assert_eq!(
            chart.drawings().drawings[0].arrow_end_style,
            crate::drawings::ArrowStyle::Circle
        );
        click(&mut dialog, &mut chart, &ctx, "Template");
        click(&mut dialog, &mut chart, &ctx, "Reset to defaults");
        assert_eq!(chart.drawings().drawings[0].stroke_width, 2.0);
        dialog.props.line_width = 4.0;
        click(&mut dialog, &mut chart, &ctx, "Template");
        click(
            &mut dialog,
            &mut chart,
            &ctx,
            "Save as default for this tool",
        );
        assert_eq!(
            chart.drawings().tool_defaults[&Tool::TrendLine]
                .settings
                .stroke_width,
            4.0
        );
        click(&mut dialog, &mut chart, &ctx, "Ok");
        assert_eq!(
            chart.drawings().tool_defaults[&Tool::TrendLine]
                .settings
                .stroke_width,
            4.0
        );
        assert!(chart.undo());
        assert_eq!(chart.drawings().drawings[0].stroke_width, 2.0);
        chart.drawings_mut().drawings[0].tool_type = Tool::Rect;
        dialog.open_for_chart(&chart, DrawingId(7));
        frame(&mut dialog, &mut chart, &ctx, vec![]);
        click(&mut dialog, &mut chart, &ctx, "Template");
        let labels = frame(&mut dialog, &mut chart, &ctx, vec![]);
        assert!(!labels.iter().any(|(text, _)| text == "Thick line"));
        dialog.close();
        frame(&mut dialog, &mut chart, &ctx, vec![]);
        chart.drawings_mut().drawings[0].tool_type = Tool::TrendLine;
        dialog.open_for_chart(&chart, DrawingId(7));
        // Let the dialog finish its sizing pass before clicking footer controls.
        frame(&mut dialog, &mut chart, &ctx, vec![]);
        click(&mut dialog, &mut chart, &ctx, "Template");
        click(&mut dialog, &mut chart, &ctx, "Delete Thick line");
        assert!(dialog.templates.entries().is_empty());
    }
}
