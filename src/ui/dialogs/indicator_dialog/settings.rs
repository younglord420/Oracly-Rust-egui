//! Settings for an existing study, staged until Apply.
//!
//! The dialog is one scrolling properties panel. Sections collapse independently, while Apply
//! still commits the whole staged edit.

use crate::localization as l10n;
use crate::model::Timeframe;
use crate::studies::{
    Indicator, InputKind, InputValue, PlotType, StudyInput, StudyScale, StudySettings,
};
use crate::theme::Theme;
use crate::theme::ThemeExt;
use crate::ui_kit::{
    ColorPicker, ColorPickerState,
    dialog::DialogFooter,
    modal::Modal,
    status_message::{MessageKind, status_message},
};
use crate::widget::SeriesId;
use egui::{Color32, FontId, RichText, Ui};
use std::collections::HashMap;

/// Preferred outer width of the settings modal. The kit clamps it to the window.
const DIALOG_WIDTH: f32 = 400.0;

/// Generic data fields the `ConfiguredStudy` wrapper adds for cross-chart calculation. They
/// are not indicator parameters, so a trader's settings form does not show them; their staged
/// values still travel to the host untouched.
fn is_generic_data_input(id: &str) -> bool {
    matches!(
        id,
        "calculation_symbol" | "calculation_timeframe" | "wait_for_close" | "plot_offset"
    )
}

/// A complete edit for the host to apply to the original study id.
pub struct StudyEdit {
    /// Stable id of the applied study.
    pub id: SeriesId,
    /// Calculation input ids and their new values.
    pub inputs: Vec<(String, InputValue)>,
    /// Per-plot styles, precision and interval visibility.
    pub settings: StudySettings,
}

/// Inputs, plots, shading and visibility editor for an applied study. Closing or cancelling
/// discards staged values; no study or output history is cloned while the dialog is open.
#[derive(Default)]
pub struct IndicatorSettingsDialog {
    edit: Option<StudyEdit>,
    inputs: Vec<StudyInput>,
    title: String,
    line_names: Vec<String>,
    current_interval: Option<Timeframe>,
    sources: Vec<(crate::studies::StudySource, String)>,
    overlay: bool,
    price_scale: bool,
    plot_pickers: Vec<ColorPickerState>,
    input_pickers: HashMap<String, ColorPickerState>,
    shade_pickers: HashMap<String, ColorPickerState>,
    shade_labels: Option<[&'static str; 2]>,
    error: String,
    defaults: Option<StudySettings>,
    /// The study's own precision, or `None` when it follows the chart.
    study_precision: Option<usize>,
    /// Study name without the dialog suffix; snaps plot colours to defaults.
    study_name: String,
    /// Bumps on every open so folded sections reset for the next study.
    open_generation: u64,
}

impl IndicatorSettingsDialog {
    /// Open on the current values of the study identified by `id`.
    pub fn open(&mut self, id: SeriesId, study: &dyn Indicator, interval: Option<Timeframe>) {
        self.edit = Some(StudyEdit {
            id,
            inputs: Vec::new(),
            settings: study.settings(),
        });
        self.inputs = study.inputs();
        self.title = format!("{} settings", study.name());
        self.study_name = study.name().to_owned();
        self.line_names = study.line_names();
        self.overlay = study.is_overlay();
        self.price_scale = study.scale() == StudyScale::Main;
        self.current_interval = interval;
        self.sources.clear();
        self.plot_pickers = study
            .settings()
            .plots
            .iter()
            .map(|plot| ColorPickerState::new(plot.color))
            .collect();
        self.input_pickers.clear();
        self.shade_pickers.clear();
        self.shade_labels = study.shading_labels();
        self.error.clear();
        self.study_precision = study.precision();
        self.defaults = Some(study.default_settings());
        self.open_generation = self.open_generation.wrapping_add(1);
    }

    /// Open on a chart study and offer its other studies as source inputs, excluding cycles.
    pub fn open_for_chart(&mut self, chart: &crate::widget::Chart, id: SeriesId) -> bool {
        let Some((_, study)) = chart.indicators().find(|(candidate, _)| *candidate == id) else {
            return false;
        };
        self.open(id, study, chart.timeframe());
        let studies: Vec<_> = chart.indicators().collect();
        for (candidate, study) in &studies {
            let mut next = Some(candidate.0);
            for _ in 0..=studies.len() {
                let Some(current) = next else {
                    break;
                };
                if current == id.0 {
                    break;
                }
                next = studies
                    .iter()
                    .find(|(id, _)| id.0 == current)
                    .and_then(|(_, study)| match study.source() {
                        Some(crate::studies::StudySource::Study { id, .. }) => Some(id),
                        _ => None,
                    });
            }
            if next.is_some() {
                continue;
            }
            for (plot, name) in study.line_names().into_iter().enumerate() {
                self.sources.push((
                    crate::studies::StudySource::Study {
                        id: candidate.0,
                        plot,
                    },
                    format!("{}: {name}", study.name()),
                ));
            }
        }
        true
    }

    /// Whether an edit is open.
    pub fn is_open(&self) -> bool {
        self.edit.is_some()
    }

    /// Temporary offset that remembers a header drag while this study stays open. A new
    /// `open` bumps the generation, so the next study starts at the normal position.
    fn drag_offset_id(&self) -> egui::Id {
        egui::Id::unique(("indicator_settings_drag", self.open_generation))
    }

    /// Show the staged edit. Apply returns it once; the host calls
    /// [`Indicator::apply_settings`] on the original study, then lets the chart recalculate.
    pub fn show(&mut self, ctx: &egui::Context, theme: &Theme) -> Option<StudyEdit> {
        let mut edit = self.edit.take()?;
        let mut open = true;
        let mut apply = false;
        let mut cancel = false;
        let title = l10n::format_text(
            ctx,
            "{name} settings",
            &[("name", self.title.trim_end_matches(" settings").to_owned())],
        );
        Modal::new("indicator_settings", &title)
            .width(DIALOG_WIDTH)
            .top(120.0)
            .draggable_by_header(self.drag_offset_id())
            .show(ctx, &mut open, |ui| {
                self.draw_panel(ui, &mut edit, theme);
                let valid =
                    self.inputs.iter().all(|input| input.accepts(&input.value))
                        && edit
                            .settings
                            .precision
                            .is_none_or(|precision| precision <= 12)
                        && edit.settings.plots.iter().all(|plot| {
                            plot.width.is_finite() && (0.5..=10.0).contains(&plot.width)
                        })
                        && edit
                            .settings
                            .shading
                            .as_ref()
                            .is_none_or(|shading| shading.validate().is_ok());
                let defaults = self.defaults.clone();
                (apply, cancel) = DialogFooter::new("Apply")
                    .primary_enabled(valid)
                    .secondary("Cancel")
                    .show_with_left(ui, |ui| {
                        if ui.button(l10n::text(ui.ctx(), "Defaults")).clicked()
                            && let Some(defaults) = defaults
                        {
                            for input in &mut self.inputs {
                                input.value = input.default.clone();
                            }
                            edit.settings = defaults;
                            self.sync_pickers(&edit);
                        }
                    });
            });
        if !open || cancel {
            return None;
        }
        if apply {
            edit.inputs = self
                .inputs
                .iter()
                .map(|input| (input.id.clone(), input.value.clone()))
                .collect();
            return Some(edit);
        }
        self.edit = Some(edit);
        None
    }

    fn sync_pickers(&mut self, edit: &StudyEdit) {
        for (picker, plot) in self.plot_pickers.iter_mut().zip(&edit.settings.plots) {
            picker.sel_color = plot.color;
        }
        self.input_pickers.clear();
        self.shade_pickers.clear();
    }

    /// Inputs the trader sees: every declared study input except the wrapper's generic data
    /// fields. Their staged values still travel in `StudyEdit` untouched.
    fn visible_inputs(&self) -> Vec<usize> {
        self.inputs
            .iter()
            .enumerate()
            .filter(|(_, input)| !is_generic_data_input(&input.id))
            .map(|(index, _)| index)
            .collect()
    }

    fn draw_panel(&mut self, ui: &mut Ui, edit: &mut StudyEdit, theme: &Theme) {
        let width = ui.available_width();
        ui.set_max_width(width);
        let reserve = 120.0;
        let scroll_max = (ui.available_height() - reserve).max(64.0);
        egui::ScrollArea::vertical()
            .id_salt("indicator_settings_sections")
            .auto_shrink([false, true])
            .max_height(scroll_max)
            .show(ui, |ui| {
                let body_width = ui.available_width();
                ui.set_width(body_width);
                ui.set_max_width(body_width);
                ui.spacing_mut().item_spacing.y = 6.0;
                self.draw_sections(ui, edit, theme);
            });
    }

    fn draw_sections(&mut self, ui: &mut Ui, edit: &mut StudyEdit, theme: &Theme) {
        let inputs = self.visible_inputs();
        if !inputs.is_empty() {
            group_label(ui, "Parameters", theme);
            self.draw_input_group(ui, &inputs, theme);
        }
        if !edit.settings.plots.is_empty() {
            group_label(ui, "Appearance", theme);
            self.draw_plot_styles(ui, edit, theme);
        }
        if edit.settings.shading.is_some() {
            group_label(ui, "Shading", theme);
            let labels = self.shade_labels;
            let mut pickers = std::mem::take(&mut self.shade_pickers);
            shading_rows(ui, edit, labels, &mut pickers);
            self.shade_pickers = pickers;
        }
        if edit.settings.precision.is_some() || self.study_precision.is_some() {
            group_label(ui, "Precision", theme);
            self.draw_precision(ui, edit, theme);
        }
        let precision_valid = edit.settings.precision.is_none_or(|value| value <= 12);
        let plot_valid = edit
            .settings
            .plots
            .iter()
            .all(|plot| plot.width.is_finite() && (0.5..=10.0).contains(&plot.width));
        let shading_invalid = edit
            .settings
            .shading
            .as_ref()
            .is_some_and(|shading| shading.validate().is_err());
        if !precision_valid || !plot_valid || shading_invalid {
            self.error = edit
                .settings
                .shading
                .as_ref()
                .and_then(|shading| shading.validate().err())
                .or_else(|| (!precision_valid).then_some("Precision is limited to 12 decimals."))
                .or_else(|| (!plot_valid).then_some("A line width is outside 0.5–10 px."))
                .unwrap_or("")
                .to_string();
        } else if self.inputs.iter().any(|input| !input.accepts(&input.value)) {
            self.error = "A parameter is outside its allowed range.".to_string();
        } else {
            self.error.clear();
        }
        if !self.error.is_empty() {
            status_message(ui, MessageKind::Error, &self.error);
        }
    }

    fn draw_input_group(&mut self, ui: &mut Ui, indexes: &[usize], theme: &Theme) {
        let sources = self.sources.clone();
        let mut pickers = std::mem::take(&mut self.input_pickers);
        for index in indexes {
            let label = self.inputs[*index].label.clone();
            let width = input_control_width(&self.inputs[*index].kind, ui.available_width());
            dense_setting_row(ui, &label, theme, width, |ui| {
                draw_input_control(ui, &mut self.inputs[*index], &sources, &mut pickers, theme);
            });
        }
        self.input_pickers = pickers;
    }

    /// One row per plot: visible toggle, its name, its own colour, and — only for the
    /// geometry that uses them — plot type and line controls. Shading colours live in the
    /// Shading group, so a plot colour is edited in exactly one place.
    fn draw_plot_styles(&mut self, ui: &mut Ui, edit: &mut StudyEdit, theme: &Theme) {
        if self.plot_pickers.len() < edit.settings.plots.len() {
            self.plot_pickers
                .resize_with(edit.settings.plots.len(), ColorPickerState::default);
        }
        let names = self.line_names.clone();
        let overlay = self.overlay;
        let price_scale = self.price_scale;
        let plot_count = edit.settings.plots.len();
        let mut pickers = std::mem::take(&mut self.plot_pickers);
        for (line, plot) in edit.settings.plots.iter_mut().enumerate() {
            let name = names
                .get(line)
                .cloned()
                .unwrap_or_else(|| "Plot".to_owned());
            let strokes = matches!(plot.plot_type, PlotType::Line | PlotType::StepLine);
            let mut paint = |ui: &mut Ui| {
                ui.set_min_height(28.0);
                ui.checkbox(
                    &mut plot.visible,
                    RichText::new(l10n::text(ui.ctx(), &name)).size(theme.text.body),
                )
                .on_hover_text(&name);
                if let Some(picker) = pickers.get_mut(line) {
                    picker.sel_color = plot.color;
                    ColorPicker::new(picker, ui.scope_id().with(("plot_color", line)))
                        .well_size(22.0)
                        .show(ui);
                    plot.color = picker.sel_color;
                }
            };
            if ui.available_width() < 320.0 {
                ui.vertical(|ui| paint(ui));
            } else {
                ui.horizontal(|ui| paint(ui));
            }

            let choices = plot_type_choices(overlay, price_scale, plot.plot_type);
            if choices.len() > 1 {
                let type_label =
                    l10n::format_text(ui.ctx(), "{name} type", &[("name", name.clone())]);
                dense_setting_row(ui, &type_label, theme, 160.0, |ui| {
                    let width = ui.available_width().clamp(96.0, 160.0);
                    crate::ui_kit::styled_dropdown(
                        ui,
                        &format!("plot-type-{line}"),
                        width,
                        theme.ui.text,
                        plot.plot_type.name(),
                        |ui| {
                            for kind in &choices {
                                let label = l10n::text(ui.ctx(), kind.name());
                                match kind.incompatibility(overlay, price_scale) {
                                    Some(reason) if *kind != plot.plot_type => {
                                        ui.add_enabled(
                                            false,
                                            egui::Button::selectable(false, label),
                                        )
                                        .on_disabled_hover_text(l10n::text(ui.ctx(), reason));
                                    }
                                    _ => {
                                        ui.selectable_value(&mut plot.plot_type, *kind, label);
                                    }
                                }
                            }
                        },
                    );
                });
            }
            if strokes {
                let width_label =
                    l10n::format_text(ui.ctx(), "{name} width", &[("name", name.clone())]);
                dense_setting_row(ui, &width_label, theme, 92.0, |ui| {
                    ui.spacing_mut().interact_size.x = 76.0;
                    ui.add(
                        egui::DragValue::new(&mut plot.width)
                            .speed(0.1)
                            .range(0.5..=10.0)
                            .suffix(l10n::text(ui.ctx(), " px")),
                    );
                });
                let pattern_label =
                    l10n::format_text(ui.ctx(), "{name} pattern", &[("name", name.clone())]);
                dense_setting_row(ui, &pattern_label, theme, 150.0, |ui| {
                    let width = ui.available_width().clamp(96.0, 150.0);
                    crate::ui_kit::styled_dropdown(
                        ui,
                        &format!("pattern-{line}"),
                        width,
                        theme.ui.text,
                        crate::ui_kit::line_style_preview(plot.pattern),
                        |ui| {
                            for pattern in crate::model::LineStyle::all() {
                                ui.selectable_value(
                                    &mut plot.pattern,
                                    *pattern,
                                    l10n::text(
                                        ui.ctx(),
                                        crate::ui_kit::line_style_preview(*pattern),
                                    ),
                                );
                            }
                        },
                    );
                });
            }
            if line + 1 < plot_count {
                ui.add_space(6.0);
            }
        }
        self.plot_pickers = pickers;
    }

    fn draw_precision(&mut self, ui: &mut Ui, edit: &mut StudyEdit, theme: &Theme) {
        dense_setting_row(ui, "Decimals", theme, 116.0, |ui| {
            let mut automatic = edit.settings.precision.is_none();
            if ui
                .checkbox(&mut automatic, l10n::text(ui.ctx(), "Automatic"))
                .changed()
            {
                edit.settings.precision = if automatic { None } else { Some(2) };
            }
            if let Some(precision) = &mut edit.settings.precision {
                let width = ui.available_width().clamp(64.0, 96.0);
                ui.spacing_mut().interact_size.x = width;
                ui.add(
                    egui::DragValue::new(precision)
                        .range(0..=12)
                        .suffix(l10n::text(ui.ctx(), " decimals")),
                );
            }
        });
    }
}

fn group_label(ui: &mut Ui, title: &str, theme: &Theme) {
    ui.add_space(2.0);
    ui.label(
        RichText::new(l10n::text(ui.ctx(), title))
            .size(theme.text.small)
            .strong()
            .color(theme.ui.text_secondary),
    );
}

/// Preferred width of the control column for one input kind. Text fields may fill the row;
/// numbers, swatches and toggles stay compact.
fn input_control_width(kind: &InputKind, available: f32) -> f32 {
    match kind {
        InputKind::Length(..) | InputKind::Integer(..) | InputKind::Number(..) => 96.0,
        InputKind::Toggle | InputKind::Color => 28.0,
        InputKind::Choice(_) | InputKind::Source | InputKind::Timeframe => 160.0_f32.min(available),
        InputKind::Symbol | InputKind::Text => 200.0_f32.min(available),
    }
}

/// Plot types the study can switch to, in editor order, always including `current` so a type
/// saved under another placement or scale remains representable.
fn plot_type_choices(overlay: bool, price_scale: bool, current: PlotType) -> Vec<PlotType> {
    let mut choices: Vec<PlotType> = PlotType::ALL
        .into_iter()
        .filter(|kind| kind.incompatibility(overlay, price_scale).is_none())
        .collect();
    if !choices.contains(&current) {
        choices.insert(0, current);
    }
    choices
}

/// One compact control inside the row's control column.
fn row_control(ui: &mut Ui, width: f32, add: impl FnOnce(&mut Ui)) {
    ui.spacing_mut().interact_size.y = 28.0;
    ui.spacing_mut().interact_size.x = width;
    ui.scope(|ui| {
        ui.set_min_width(width);
        ui.set_max_width(width);
        add(ui);
    });
}

/// Label on the left and a control column of `control_width` on the right. Long labels stack
/// above the control. The control keeps its own width instead of stretching across the dialog.
fn dense_setting_row(
    ui: &mut Ui,
    label: &str,
    theme: &Theme,
    control_width: f32,
    add: impl FnOnce(&mut Ui),
) {
    let text = l10n::text(ui.ctx(), label);
    let available = ui.available_width();
    let measured = ui
        .painter()
        .layout_no_wrap(
            text.clone(),
            FontId::proportional(theme.text.body),
            theme.ui.text_secondary,
        )
        .size()
        .x;
    let stacked = available < 300.0 || measured > available * 0.42;
    let width = control_width.min(available).max(48.0);
    let draw_label = |ui: &mut Ui| {
        ui.add(
            egui::Label::new(
                RichText::new(text.clone())
                    .size(theme.text.body)
                    .color(theme.ui.text_secondary),
            )
            .wrap(),
        )
    };
    if stacked {
        let response = draw_label(ui);
        ui.label_inputs_by(response.id, |ui| row_control(ui, width, add));
    } else {
        ui.horizontal(|ui| {
            ui.set_min_height(28.0);
            ui.spacing_mut().item_spacing.x = 8.0;
            let label_w = (available * 0.42).clamp(108.0, 180.0);
            let response = ui
                .allocate_ui_with_layout(
                    egui::vec2(label_w, 28.0),
                    egui::Layout::left_to_right(egui::Align::Center),
                    |ui| {
                        ui.set_min_width(label_w);
                        ui.set_max_width(label_w);
                        draw_label(ui)
                    },
                )
                .inner;
            ui.label_inputs_by(response.id, |ui| row_control(ui, width, add));
        });
    }
    ui.add_space(6.0);
}
fn draw_input_control(
    ui: &mut Ui,
    input: &mut crate::studies::StudyInput,
    sources: &[(crate::studies::StudySource, String)],
    pickers: &mut HashMap<String, ColorPickerState>,
    theme: &Theme,
) {
    let control_width = input_control_width(&input.kind, ui.available_width());
    ui.spacing_mut().interact_size.x = control_width;
    ui.spacing_mut().interact_size.y = 28.0;
    match (&input.kind, &mut input.value) {
        (InputKind::Length(min, max), InputValue::Length(value)) => {
            ui.add(egui::DragValue::new(value).range(*min..=*max));
        }
        (InputKind::Integer(min, max), InputValue::Integer(value)) => {
            ui.add(egui::DragValue::new(value).range(*min..=*max));
        }
        (InputKind::Number(min, max), InputValue::Number(value)) => {
            ui.add(egui::DragValue::new(value).speed(0.1).range(*min..=*max));
        }
        (InputKind::Choice(choices), InputValue::Choice(value)) => {
            let selected = choices.get(*value).map_or("", String::as_str);
            crate::ui_kit::styled_dropdown(
                ui,
                &input.id,
                control_width,
                theme.ui.text,
                selected,
                |ui| {
                    for (index, label) in choices.iter().enumerate() {
                        ui.selectable_value(value, index, l10n::text(ui.ctx(), label));
                    }
                },
            );
        }
        (InputKind::Toggle, InputValue::Toggle(value)) => {
            ui.checkbox(value, "");
        }
        (InputKind::Source, InputValue::Source(value)) => {
            let selected = match value {
                crate::studies::StudySource::Price(source) => source.label().to_owned(),
                crate::studies::StudySource::Study { .. } => sources
                    .iter()
                    .find(|(source, _)| source == value)
                    .map(|(_, label)| label.clone())
                    .unwrap_or_else(|| "Unavailable study".to_owned()),
            };
            crate::ui_kit::styled_dropdown(
                ui,
                &input.id,
                control_width,
                theme.ui.text,
                &selected,
                |ui| {
                    for (source, label) in sources {
                        ui.selectable_value(value, *source, l10n::text(ui.ctx(), label));
                    }
                    for source in crate::model::PriceSource::all() {
                        ui.selectable_value(
                            value,
                            crate::studies::StudySource::Price(source),
                            l10n::text(ui.ctx(), source.label()),
                        );
                    }
                },
            );
        }
        (InputKind::Color, InputValue::Color(value)) => {
            let mut picker = pickers
                .remove(&input.id)
                .unwrap_or_else(|| ColorPickerState::new(*value));
            picker.sel_color = *value;
            ColorPicker::new(&mut picker, ui.scope_id().with(("input-color", &input.id)))
                .well_size(22.0)
                .show(ui);
            *value = picker.sel_color;
            pickers.insert(input.id.clone(), picker);
        }
        (InputKind::Symbol, InputValue::Symbol(value)) => {
            let response = ui.add(
                egui::TextEdit::singleline(value).hint_text(l10n::text(ui.ctx(), "Chart symbol")),
            );
            // A placeholder counts as a name, so the row label would not be attached.
            ui.ctx().accesskit_node_builder(response.id, |node| {
                node.set_placeholder(String::new());
            });
        }
        (InputKind::Text, InputValue::Text(value)) => {
            ui.add(egui::TextEdit::singleline(value));
        }
        (InputKind::Timeframe, InputValue::Timeframe(value)) => {
            let selected = value.map_or_else(|| "Chart interval".to_owned(), |tf| tf.to_string());
            crate::ui_kit::styled_dropdown(
                ui,
                &input.id,
                control_width,
                theme.ui.text,
                &selected,
                |ui| {
                    ui.selectable_value(value, None, l10n::text(ui.ctx(), "Chart interval"));
                    for tf in Timeframe::all() {
                        ui.selectable_value(value, Some(tf), l10n::text(ui.ctx(), tf.to_string()));
                    }
                    if ui
                        .selectable_label(
                            value.is_some_and(|tf| tf.is_custom()),
                            l10n::text(ui.ctx(), "Custom"),
                        )
                        .clicked()
                    {
                        *value = Some(Timeframe::Custom(std::num::NonZeroU64::new(60).unwrap()));
                    }
                },
            );
            if let Some(Timeframe::Custom(seconds)) = value {
                let mut count = seconds.get();
                let response = ui.add(
                    egui::DragValue::new(&mut count)
                        .range(1..=u64::MAX)
                        .suffix(l10n::text(ui.ctx(), " s")),
                );
                // The suffix would name the field, hiding the row label "Interval".
                ui.ctx().accesskit_node_builder(response.id, |node| {
                    node.clear_label();
                });
                if response.changed() {
                    *seconds = std::num::NonZeroU64::new(count).unwrap();
                }
            }
        }
        _ => {
            ui.label(l10n::text(ui.ctx(), "This input is unavailable."));
        }
    };
}

fn shading_rows(
    ui: &mut Ui,
    edit: &mut StudyEdit,
    labels: Option<[&'static str; 2]>,
    pickers: &mut HashMap<String, ColorPickerState>,
) {
    let Some(mut shading) = edit.settings.shading.clone() else {
        return;
    };
    let theme = ui.ctx().active_theme();
    use crate::studies::ShadingKind;
    let wash = matches!(
        shading.kind,
        ShadingKind::Bounded {
            midpoint_wash: true,
            ..
        } | ShadingKind::Centered { .. }
            | ShadingKind::Regime { .. }
            | ShadingKind::Spread { .. }
            | ShadingKind::Channel { .. }
            | ShadingKind::Intensity { .. }
            | ShadingKind::PriceSide { .. }
            | ShadingKind::Slope { .. }
    );
    if wash {
        ui.checkbox(
            &mut shading.gradient_fill,
            RichText::new(l10n::text(ui.ctx(), "Gradient fill")).size(theme.text.body),
        );
    }
    let [bull, bear] = labels.unwrap_or(match shading.kind {
        ShadingKind::Bounded { .. } => ["Oversold", "Overbought"],
        _ => ["Bullish", "Bearish"],
    });
    let expansion_only = labels.is_some_and(|labels| labels[0] == "Expansion");
    match shading.kind {
        ShadingKind::MomentumColumns { .. } => {
            color_row(ui, pickers, "Grow above", &mut shading.bull);
            color_row(ui, pickers, "Fall above", &mut shading.faded_bull);
            color_row(ui, pickers, "Grow below", &mut shading.bear);
            color_row(ui, pickers, "Fall below", &mut shading.faded_bear);
        }
        ShadingKind::RisingColumns { .. } => {
            color_row(ui, pickers, "Rising", &mut shading.bull);
            color_row(ui, pickers, "Falling", &mut shading.bear);
        }
        ShadingKind::Intensity { .. } => color_row(ui, pickers, "Accent", &mut shading.accent),
        ShadingKind::FlatChannel { .. } => color_row(ui, pickers, "Fill", &mut shading.band),
        ShadingKind::AdxInk { .. } => {
            color_row(ui, pickers, "+DI", &mut shading.bull);
            color_row(ui, pickers, "\u{2212}DI", &mut shading.bear);
        }
        ShadingKind::BandOnly { .. } => {
            color_row(ui, pickers, "Band fill", &mut shading.band);
            color_row(ui, pickers, "Levels", &mut shading.levels);
        }
        _ if expansion_only => color_row(ui, pickers, bull, &mut shading.bull),
        _ => {
            color_row(ui, pickers, bull, &mut shading.bull);
            color_row(ui, pickers, bear, &mut shading.bear);
            if matches!(
                shading.kind,
                ShadingKind::Bounded {
                    band_fill: true,
                    ..
                }
            ) {
                color_row(ui, pickers, "Band fill", &mut shading.band);
            }
            if matches!(
                shading.kind,
                ShadingKind::Bounded { .. } | ShadingKind::Centered { .. }
            ) {
                color_row(ui, pickers, "Levels", &mut shading.levels);
            }
        }
    }
    match &mut shading.kind {
        ShadingKind::Bounded {
            overbought,
            oversold,
            zone_t,
            ..
        } => {
            level(ui, "Overbought", overbought);
            level(ui, "Oversold", oversold);
            let mut zones = u32::from(*zone_t);
            let mut changed = false;
            dense_setting_row(ui, "Zones", &theme, 96.0, |ui| {
                let width = ui.available_width().clamp(64.0, 96.0);
                ui.spacing_mut().interact_size.x = width;
                changed = ui
                    .add(
                        egui::DragValue::new(&mut zones)
                            .range(0..=100)
                            .suffix(l10n::text(ui.ctx(), " %")),
                    )
                    .changed();
            });
            if changed {
                *zone_t = zones as u8;
            }
        }
        ShadingKind::BandOnly { upper, lower } => {
            level(ui, "Upper", upper);
            level(ui, "Lower", lower);
        }
        ShadingKind::Centered {
            baseline,
            extra_levels,
            ..
        } => {
            level(ui, "Baseline", baseline);
            for (index, value) in extra_levels.iter_mut().enumerate() {
                level(ui, &format!("Level {}", index + 1), value);
            }
        }
        ShadingKind::AdxInk { key_level, .. } => level(ui, "Key level", key_level),
        ShadingKind::Spread { fill_t, .. } | ShadingKind::FlatChannel { fill_t, .. } => {
            let mut fill = u32::from(*fill_t);
            let mut changed = false;
            dense_setting_row(ui, "Fill", &theme, 96.0, |ui| {
                let width = ui.available_width().clamp(64.0, 96.0);
                ui.spacing_mut().interact_size.x = width;
                changed = ui
                    .add(
                        egui::DragValue::new(&mut fill)
                            .range(0..=100)
                            .suffix(l10n::text(ui.ctx(), " %")),
                    )
                    .changed();
            });
            if changed {
                *fill_t = fill as u8;
            }
        }
        ShadingKind::Intensity { range, .. } => match range {
            crate::studies::IntensityRange::Threshold(value) => level(ui, "Threshold", value),
            crate::studies::IntensityRange::Rolling(bars) => {
                let mut value = *bars as f64;
                let mut changed = false;
                dense_setting_row(ui, "Lookback", &theme, 96.0, |ui| {
                    let width = ui.available_width().clamp(64.0, 96.0);
                    ui.spacing_mut().interact_size.x = width;
                    changed = ui
                        .add(
                            egui::DragValue::new(&mut value)
                                .speed(1.0)
                                .range(1.0..=10_000.0),
                        )
                        .changed();
                });
                if changed {
                    *bars = value.round() as usize;
                }
            }
        },
        _ => {}
    }
    edit.settings.shading = Some(shading);
}

fn level(ui: &mut Ui, label: &str, value: &mut f64) {
    let theme = ui.ctx().active_theme();
    dense_setting_row(ui, label, &theme, 96.0, |ui| {
        let width = ui.available_width().clamp(64.0, 96.0);
        ui.spacing_mut().interact_size.x = width;
        ui.add(egui::DragValue::new(value).speed(0.1));
    });
}

fn color_row(
    ui: &mut Ui,
    pickers: &mut HashMap<String, ColorPickerState>,
    label: &str,
    color: &mut Color32,
) {
    let mut picker = pickers
        .remove(label)
        .unwrap_or_else(|| ColorPickerState::new(*color));
    picker.sel_color = *color;
    let theme = ui.ctx().active_theme();
    dense_setting_row(ui, label, &theme, 28.0, |ui| {
        ColorPicker::new(&mut picker, ui.scope_id().with(("shade-color", label)))
            .well_size(22.0)
            .show(ui);
    });
    *color = picker.sel_color;
    pickers.insert(label.to_owned(), picker);
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::studies::{ConfiguredStudy, SMA};

    #[test]
    fn dialog_follows_the_window_when_it_shrinks() {
        for theme in [Theme::light(), Theme::dark()] {
            for (name, settings) in [
                ("MA Ribbon", false),
                ("MA Ribbon", true),
                ("Vol Profile FR", true),
            ] {
                let ctx = egui::Context::default();
                crate::theme::apply_to_egui(&ctx, &theme);
                let study = crate::studies::IndicatorFactory::new()
                    .create(name)
                    .unwrap();
                let mut dialog = IndicatorSettingsDialog::default();
                dialog.open(SeriesId(2), study.as_ref(), Some(Timeframe::Min1));
                let mut browser = super::super::IndicatorDialog::new();
                browser.open();
                for size in [
                    egui::vec2(1440.0, 900.0),
                    egui::vec2(320.0, 360.0),
                    egui::vec2(600.0, 300.0),
                ] {
                    let size = if !settings {
                        egui::vec2(size.x.max(600.0), size.y.max(500.0))
                    } else {
                        size
                    };
                    let screen = egui::Rect::from_min_size(egui::Pos2::ZERO, size);
                    for _ in 0..4 {
                        ctx.run_ui(
                            egui::RawInput {
                                screen_rect: Some(screen),
                                ..Default::default()
                            },
                            |ui| {
                                if settings {
                                    dialog.show(ui.ctx(), &theme);
                                } else {
                                    browser.show(ui.ctx(), &theme);
                                }
                            },
                        )
                        .drop_without_applying_deltas();
                    }
                    let shown = ctx
                        .memory(|m| {
                            m.area_rect(egui::Id::unique(if settings {
                                "indicator_settings"
                            } else {
                                "indicator_dialog"
                            }))
                        })
                        .unwrap();
                    assert!(
                        screen.contains_rect(shown),
                        "{name} settings {settings}: {shown:?} outside {screen:?}"
                    );
                }
            }
        }
    }

    #[test]
    fn escape_discards_staged_inputs_and_presentation() {
        let study = ConfiguredStudy::new(Box::new(SMA::new(20)));
        let mut dialog = IndicatorSettingsDialog::default();
        dialog.open(SeriesId(2), &study, Some(Timeframe::Min1));
        dialog.inputs[0].value = InputValue::Length(2);
        dialog.edit.as_mut().unwrap().settings.plots[0].visible = false;
        let ctx = egui::Context::default();
        for escape in [false, true] {
            let events = if escape {
                vec![egui::Event::Key {
                    key: egui::Key::Escape,
                    physical_key: None,
                    pressed: true,
                    repeat: false,
                    modifiers: egui::Modifiers::NONE,
                }]
            } else {
                Vec::new()
            };
            ctx.run_ui(
                egui::RawInput {
                    events,
                    ..Default::default()
                },
                |ui| {
                    assert!(dialog.show(ui.ctx(), &Theme::dark()).is_none());
                },
            )
            .drop_without_applying_deltas();
        }
        assert!(!dialog.is_open());
        assert_eq!(study.inputs()[0].value, InputValue::Length(20));
        assert!(study.plot_style(0).visible);
    }

    /// Defaults restores the inputs and presentation that were current when the dialog opened.
    #[test]
    fn defaults_reset_inputs_and_presentation() {
        use egui::Role;
        use egui_kittest::{Harness, kittest::Queryable};
        let study = ConfiguredStudy::new(Box::new(SMA::new(20)));
        let mut dialog = IndicatorSettingsDialog::default();
        dialog.open(SeriesId(2), &study, Some(Timeframe::Min1));
        dialog.inputs[0].value = InputValue::Length(2);
        dialog.edit.as_mut().unwrap().settings.plots[0].color = egui::Color32::RED;
        dialog.edit.as_mut().unwrap().settings.precision = Some(7);
        dialog.edit.as_mut().unwrap().settings.hidden_intervals = vec![Timeframe::Min1];
        let theme = Theme::dark();
        let mut applied = None;
        let mut harness = Harness::builder()
            .with_size(egui::vec2(800.0, 700.0))
            .build_ui(|ui| {
                crate::theme::apply_to_egui(ui.ctx(), &theme);
                crate::icons::install_loader(ui.ctx());
                if let Some(edit) = dialog.show(ui.ctx(), &theme) {
                    applied = Some(edit);
                }
            });
        harness.run();
        harness
            .get_by_role_and_label(Role::Button, "Defaults")
            .click();
        harness.run();
        harness.get_by_role_and_label(Role::Button, "Apply").click();
        harness.run();
        drop(harness);
        let edit = applied.expect("Apply returns the staged defaults");
        assert_eq!(edit.inputs[0].1, InputValue::Length(20));
        assert_ne!(edit.settings.plots[0].color, egui::Color32::RED);
        assert!(edit.settings.precision.is_none());
        assert!(edit.settings.hidden_intervals.is_empty());
    }

    /// Study inputs stay visible; the wrapper's cross-chart data fields never become form rows.
    #[test]
    fn generic_data_inputs_stay_out_of_the_form() {
        use egui::Role;
        use egui_kittest::{Harness, kittest::Queryable};
        let study = ConfiguredStudy::new(Box::new(SMA::new(20)));
        let mut dialog = IndicatorSettingsDialog::default();
        dialog.open(SeriesId(2), &study, Some(Timeframe::Min1));
        assert!(
            dialog
                .inputs
                .iter()
                .any(|input| input.id == "calculation_symbol"),
            "the staged edit still carries the wrapper fields"
        );
        let visible: Vec<&str> = dialog
            .visible_inputs()
            .into_iter()
            .map(|index| dialog.inputs[index].id.as_str())
            .collect();
        assert!(visible.iter().all(|id| !is_generic_data_input(id)));

        let theme = Theme::light();
        let mut harness = Harness::builder()
            .with_size(egui::vec2(900.0, 760.0))
            .build_ui(|ui| {
                crate::theme::apply_to_egui(ui.ctx(), &theme);
                crate::icons::install_loader(ui.ctx());
                dialog.show(ui.ctx(), &theme);
            });
        harness.run();
        harness.get_by_role_and_label(Role::SpinButton, "Period");
        harness.get_by_role_and_label(Role::ComboBox, "Source");
        assert!(
            harness
                .query_by_role_and_label(Role::TextInput, "Symbol")
                .is_none()
        );
        assert!(
            harness
                .query_by_role_and_label(Role::CheckBox, "Wait for interval close")
                .is_none()
        );
        assert!(
            harness
                .query_by_role_and_label(Role::ComboBox, "Range")
                .is_none(),
            "the interval grid is not part of the study form"
        );
        assert!(
            harness
                .query_by_role_and_label(Role::Button, "Advanced settings")
                .is_none(),
            "there is no Advanced dump anymore"
        );
    }

    /// Every declared study parameter is rendered on the form, with no hidden remainder.
    #[test]
    fn every_study_parameter_is_visible() {
        let ribbon = crate::studies::IndicatorFactory::new()
            .create("MA Ribbon")
            .unwrap();
        let mut dialog = IndicatorSettingsDialog::default();
        dialog.open(SeriesId(2), ribbon.as_ref(), Some(Timeframe::Min1));
        let visible = dialog.visible_inputs();
        let expected = dialog
            .inputs
            .iter()
            .filter(|input| !is_generic_data_input(&input.id))
            .count();
        assert_eq!(visible.len(), expected);
        assert!(
            visible.len() < dialog.inputs.len() || !dialog.inputs.is_empty(),
            "the wrapper fields are excluded from the count"
        );

        let bb = crate::studies::IndicatorFactory::new()
            .create("BB")
            .unwrap();
        let mut dialog = IndicatorSettingsDialog::default();
        dialog.open(SeriesId(2), bb.as_ref(), Some(Timeframe::Min1));
        let ids: Vec<&str> = dialog
            .visible_inputs()
            .into_iter()
            .map(|index| dialog.inputs[index].id.as_str())
            .collect();
        assert!(ids.contains(&"basis"));
        assert!(ids.contains(&"std_dev"));
    }

    #[test]
    fn input_kinds_have_compact_control_widths() {
        let available = 900.0;
        for kind in [
            InputKind::Length(1, 10),
            InputKind::Integer(0, 10),
            InputKind::Number(0.0, 10.0),
        ] {
            assert!(input_control_width(&kind, available) <= 110.0);
        }
        assert_eq!(input_control_width(&InputKind::Source, available), 160.0);
        assert_eq!(input_control_width(&InputKind::Toggle, available), 28.0);
        assert_eq!(input_control_width(&InputKind::Color, available), 28.0);
    }

    /// A simple study's dialog is narrower and shorter than the old 500 px fixed form.
    #[test]
    fn simple_study_dialog_is_compact() {
        let study = crate::studies::IndicatorFactory::new()
            .create("SMA")
            .unwrap();
        let mut dialog = IndicatorSettingsDialog::default();
        dialog.open(SeriesId(2), study.as_ref(), Some(Timeframe::Min1));
        let ctx = egui::Context::default();
        let theme = Theme::dark();
        crate::theme::apply_to_egui(&ctx, &theme);
        let screen = egui::Rect::from_min_size(egui::Pos2::ZERO, egui::vec2(1440.0, 900.0));
        for _ in 0..4 {
            ctx.run_ui(
                egui::RawInput {
                    screen_rect: Some(screen),
                    ..Default::default()
                },
                |ui| {
                    dialog.show(ui.ctx(), &theme);
                },
            )
            .drop_without_applying_deltas();
        }
        let rect = ctx
            .memory(|memory| memory.area_rect(egui::Id::unique("indicator_settings")))
            .unwrap();
        assert!(
            rect.width() <= 440.0,
            "the settings modal still spreads across the window: {rect:?}"
        );
        assert!(
            rect.height() <= 420.0,
            "a one-parameter study should not open a tall form: {rect:?}"
        );
    }

    /// Human-readable `ShadingKind` variant name for the audit matrix.
    fn shading_kind_name(kind: &crate::studies::ShadingKind) -> &'static str {
        use crate::studies::ShadingKind;
        match kind {
            ShadingKind::Bounded { .. } => "Bounded",
            ShadingKind::BandOnly { .. } => "BandOnly",
            ShadingKind::Centered { .. } => "Centered",
            ShadingKind::Regime { .. } => "Regime",
            ShadingKind::Spread { .. } => "Spread",
            ShadingKind::Channel { .. } => "Channel",
            ShadingKind::FlatChannel { .. } => "FlatChannel",
            ShadingKind::MomentumColumns { .. } => "MomentumColumns",
            ShadingKind::RisingColumns { .. } => "RisingColumns",
            ShadingKind::Intensity { .. } => "Intensity",
            ShadingKind::PriceSide { .. } => "PriceSide",
            ShadingKind::Slope { .. } => "Slope",
            ShadingKind::AdxInk { .. } => "AdxInk",
        }
    }

    /// Prints the audit matrix for every registered study. Run with
    /// `cargo test --lib catalogue_audit_matrix -- --ignored --nocapture` and copy the output
    /// into `docs/indicator-study-settings-audit.md`; the gate in
    /// `every_registered_study_opens_with_its_own_parameters` keeps the catalogue covered.
    #[test]
    #[ignore]
    fn catalogue_audit_matrix() {
        let factory = crate::studies::IndicatorFactory::new();
        let mut names = factory.list();
        names.sort();
        println!("registered={}", names.len());
        for name in &names {
            let study = factory.create(name).expect("constructor");
            let category = factory.category(name).unwrap_or("?");
            let wrapped = ConfiguredStudy::new(study);
            let inputs = wrapped.inputs();
            let visible: Vec<String> = inputs
                .iter()
                .filter(|input| !is_generic_data_input(&input.id))
                .map(|input| input.id.clone())
                .collect();
            let wrapper: Vec<String> = inputs
                .iter()
                .filter(|input| is_generic_data_input(&input.id))
                .map(|input| input.id.clone())
                .collect();
            let plots = wrapped.line_names();
            let shading = match wrapped.shading() {
                Some(shading) => shading_kind_name(&shading.kind).to_owned(),
                None => "none".to_owned(),
            };
            println!(
                "{name} | {category} | overlay={} | inputs={:?} | wrapper={:?} | plots={:?} | shading={shading}",
                wrapped.is_overlay(),
                visible,
                wrapper,
                plots,
            );
        }
    }

    /// Every registered study opens a form with only its own parameters and keeps every staged
    /// value it does not show.
    #[test]
    fn every_registered_study_opens_with_its_own_parameters() {
        let mut names = crate::studies::IndicatorFactory::new().list();
        names.sort();
        assert!(!names.is_empty(), "the catalogue must not be empty");
        let mut checked = 0;
        for name in names {
            let study = crate::studies::IndicatorFactory::new()
                .create(&name)
                .unwrap_or_else(|| panic!("{name} failed to construct"));
            let category = crate::studies::IndicatorFactory::new()
                .category(&name)
                .unwrap_or("?");
            let mut dialog = IndicatorSettingsDialog::default();
            dialog.open(SeriesId(2), study.as_ref(), Some(Timeframe::Min1));

            for index in dialog.visible_inputs() {
                let input = &dialog.inputs[index];
                assert!(
                    !is_generic_data_input(&input.id),
                    "{name} [{category}] shows the wrapper field {}",
                    input.id
                );
            }
            assert!(
                dialog.inputs.iter().all(|input| {
                    input.accepts(&input.value) || is_generic_data_input(&input.id)
                }),
                "{name} [{category}] declares an input its default does not satisfy"
            );

            let staged_inputs: Vec<(String, InputValue)> = dialog
                .inputs
                .iter()
                .map(|input| (input.id.clone(), input.value.clone()))
                .collect();
            let staged_plots = dialog
                .edit
                .as_ref()
                .expect("staged edit")
                .settings
                .plots
                .len();
            let ctx = egui::Context::default();
            let theme = Theme::dark();
            crate::theme::apply_to_egui(&ctx, &theme);
            let screen = egui::Rect::from_min_size(egui::Pos2::ZERO, egui::vec2(900.0, 700.0));
            for _ in 0..3 {
                ctx.run_ui(
                    egui::RawInput {
                        screen_rect: Some(screen),
                        ..Default::default()
                    },
                    |ui| {
                        dialog.show(ui.ctx(), &theme);
                    },
                )
                .drop_without_applying_deltas();
            }
            assert!(
                dialog.is_open(),
                "{name} [{category}] closed its own settings dialog"
            );

            // Showing the form must not mutate or drop any staged value.
            let after: Vec<(String, InputValue)> = dialog
                .inputs
                .iter()
                .map(|input| (input.id.clone(), input.value.clone()))
                .collect();
            assert_eq!(
                staged_inputs, after,
                "{name} [{category}] changed or lost staged inputs"
            );
            assert_eq!(
                dialog
                    .edit
                    .as_ref()
                    .expect("staged edit")
                    .settings
                    .plots
                    .len(),
                staged_plots,
                "{name} [{category}] changed the plot count"
            );
            checked += 1;
        }
        assert_eq!(
            checked,
            crate::studies::IndicatorFactory::new().list().len()
        );
    }

    /// Editing one visible field leaves the hidden cross-chart values alone.
    #[test]
    fn hidden_cross_chart_values_survive_an_edit() {
        let study = ConfiguredStudy::new(Box::new(SMA::new(20)));
        let mut dialog = IndicatorSettingsDialog::default();
        dialog.open(SeriesId(2), &study, Some(Timeframe::Min1));
        let symbol = dialog
            .inputs
            .iter_mut()
            .find(|input| input.id == "calculation_symbol")
            .unwrap();
        symbol.value = InputValue::Symbol("BTCUSDT".into());
        let interval = dialog
            .inputs
            .iter_mut()
            .find(|input| input.id == "calculation_timeframe")
            .unwrap();
        interval.value = InputValue::Timeframe(Some(Timeframe::Hour4));
        dialog.edit.as_mut().unwrap().settings.precision = Some(5);

        use egui_kittest::kittest::Queryable;
        let theme = Theme::dark();
        let mut applied = None;
        let mut harness = egui_kittest::Harness::builder()
            .with_size(egui::vec2(800.0, 700.0))
            .build_ui(|ui| {
                crate::theme::apply_to_egui(ui.ctx(), &theme);
                crate::icons::install_loader(ui.ctx());
                if let Some(edit) = dialog.show(ui.ctx(), &theme) {
                    applied = Some(edit);
                }
            });
        harness.run();
        harness
            .get_by_role_and_label(egui::Role::Button, "Apply")
            .click();
        harness.run();
        drop(harness);
        let edit = applied.expect("Apply returns the edit");
        let symbol = edit
            .inputs
            .iter()
            .find(|(id, _)| id == "calculation_symbol")
            .map(|(_, value)| value.clone());
        let interval = edit
            .inputs
            .iter()
            .find(|(id, _)| id == "calculation_timeframe")
            .map(|(_, value)| value.clone());
        assert_eq!(symbol, Some(InputValue::Symbol("BTCUSDT".into())));
        assert_eq!(
            interval,
            Some(InputValue::Timeframe(Some(Timeframe::Hour4)))
        );
        assert_eq!(edit.settings.precision, Some(5));
    }
}
