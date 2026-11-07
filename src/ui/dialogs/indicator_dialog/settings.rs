//! Settings for an existing study, staged until Apply.

use crate::localization as l10n;
use crate::model::Timeframe;
use crate::studies::{
    Indicator, InputKind, InputValue, PlotType, StudyInput, StudyIntervalRange, StudyScale,
    StudySettings,
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
use egui::{Color32, Ui};

#[derive(Clone, Copy, PartialEq, Eq, Default)]
enum Tab {
    #[default]
    Inputs,
    Style,
    Visibility,
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

/// Inputs, Style and Visibility editor for an applied study. Closing or cancelling discards
/// staged values; no study or output history is cloned while the dialog is open.
#[derive(Default)]
pub struct IndicatorSettingsDialog {
    edit: Option<StudyEdit>,
    inputs: Vec<StudyInput>,
    title: String,
    line_names: Vec<String>,
    tab: Tab,
    current_interval: Option<Timeframe>,
    sources: Vec<(crate::studies::StudySource, String)>,
    overlay: bool,
    price_scale: bool,
    plot_pickers: Vec<ColorPickerState>,
    shade_labels: Option<[&'static str; 2]>,
    error: String,
    defaults: Option<StudySettings>,
}

impl crate::ui_kit::tab_bar::TabLabel for Tab {
    fn tab_label(&self) -> &str {
        match self {
            Self::Inputs => "Inputs",
            Self::Style => "Style",
            Self::Visibility => "Visibility",
        }
    }

    fn tab_icon(&self) -> Option<&'static crate::icons::Icon> {
        Some(match self {
            Self::Inputs => &crate::icons::icons::SETTINGS_NUT,
            Self::Style => &crate::icons::icons::PAINTBRUSH,
            Self::Visibility => &crate::icons::icons::EYE,
        })
    }
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
        self.line_names = study.line_names();
        self.overlay = study.is_overlay();
        self.price_scale = study.scale() == StudyScale::Main;
        self.current_interval = interval;
        self.tab = Tab::Inputs;
        self.sources.clear();
        self.plot_pickers = study
            .settings()
            .plots
            .iter()
            .map(|plot| ColorPickerState::new(plot.color))
            .collect();
        self.shade_labels = study.shading_labels();
        self.error.clear();
        self.defaults = Some(study.default_settings());
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
            .width(620.0)
            .height(560.0)
            .top(120.0)
            .show(ctx, &mut open, |ui| {
                let mut tab = self.tab;
                crate::ui_kit::tab_bar::TabBar::new(
                    &[Tab::Inputs, Tab::Style, Tab::Visibility],
                    &mut tab,
                )
                .pills()
                .show(ui);
                self.tab = tab;
                ui.separator();
                egui::ScrollArea::vertical()
                    .max_height((ui.available_height() - 72.0).max(0.0))
                    .show(ui, |ui| match self.tab {
                        Tab::Inputs => {
                            if self.inputs.is_empty() {
                                ui.label(l10n::text(
                                    ui.ctx(),
                                    "This study has no editable inputs.",
                                ));
                            }
                            crate::ui_kit::section_header(ui, "INPUTS", theme);
                            self.draw_inputs(ui, theme);
                        }
                        Tab::Style => {
                            self.draw_style(ui, &mut edit, theme);
                        }
                        Tab::Visibility => {
                            self.draw_visibility(ui, &mut edit, theme);
                        }
                    });
                let valid = self.inputs.iter().all(|input| input.accepts(&input.value))
                    && edit
                        .settings
                        .shading
                        .as_ref()
                        .is_none_or(|shading| shading.validate().is_ok());
                self.error = edit
                    .settings
                    .shading
                    .as_ref()
                    .and_then(|shading| shading.validate().err())
                    .unwrap_or("")
                    .to_string();
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
                        }
                        if !self.error.is_empty() {
                            status_message(ui, MessageKind::Error, &self.error);
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

    fn calculation_input(id: &str) -> bool {
        matches!(
            id,
            "calculation_symbol" | "calculation_timeframe" | "wait_for_close"
        )
    }

    fn draw_inputs(&mut self, ui: &mut Ui, theme: &Theme) {
        let study_source = self.inputs.iter().any(|input| {
            matches!(
                input.value,
                InputValue::Source(crate::studies::StudySource::Study { .. })
            )
        });
        for input in &mut self.inputs {
            if study_source && Self::calculation_input(&input.id) {
                input.value = input.default.clone();
            }
        }
        let labels: Vec<_> = self
            .inputs
            .iter()
            .filter(|input| !(study_source && Self::calculation_input(&input.id)))
            .map(|input| input.label.clone())
            .collect();
        let width = crate::ui_kit::label_width(ui, theme, &labels, ui.available_width() * 0.4);
        for calculation in [false, true] {
            let present = self.inputs.iter().any(|input| {
                Self::calculation_input(&input.id) == calculation
                    && !(study_source && Self::calculation_input(&input.id))
            });
            if !present {
                continue;
            }
            if calculation {
                crate::ui_kit::section_header(ui, "CALCULATION", theme);
            }
            for index in 0..self.inputs.len() {
                if Self::calculation_input(&self.inputs[index].id) != calculation
                    || (study_source && Self::calculation_input(&self.inputs[index].id))
                {
                    continue;
                }
                let label = self.inputs[index].label.clone();
                crate::ui_kit::setting_row(ui, &label, width, theme, |ui| {
                    draw_input_control(ui, &mut self.inputs[index], &self.sources, theme);
                });
            }
        }
    }

    fn draw_style(&mut self, ui: &mut Ui, edit: &mut StudyEdit, theme: &Theme) {
        crate::ui_kit::section_header(ui, "PLOTS", theme);
        let wide = ui.ctx().content_rect().width() >= 700.0;
        let overlay = self.overlay;
        let price_scale = self.price_scale;
        let names = self.line_names.clone();
        let mut pickers = std::mem::take(&mut self.plot_pickers);
        for (line, plot) in edit.settings.plots.iter_mut().enumerate() {
            let name = names
                .get(line)
                .cloned()
                .unwrap_or_else(|| "Plot".to_owned());
            let mut draw = |ui: &mut Ui, plot: &mut crate::studies::PlotStyle, wide: bool| {
                let shown = l10n::text(ui.ctx(), &name);
                let toggle = |ui: &mut Ui| {
                    ui.checkbox(&mut plot.visible, shown).on_hover_text(&name);
                };
                if wide {
                    ui.allocate_ui_with_layout(
                        egui::vec2(150.0, 28.0),
                        egui::Layout::left_to_right(egui::Align::Center),
                        toggle,
                    );
                } else {
                    toggle(ui);
                }
                if let Some(picker) = pickers.get_mut(line) {
                    picker.sel_color = plot.color;
                    ColorPicker::new(picker, ("plot_color", line))
                        .well_size(18.0)
                        .show(ui);
                    plot.color = picker.sel_color;
                }
                let mut width_value = |ui: &mut Ui| {
                    ui.add(
                        egui::DragValue::new(&mut plot.width)
                            .speed(0.1)
                            .range(0.5..=10.0)
                            .suffix(l10n::text(ui.ctx(), " px")),
                    );
                };
                if wide {
                    ui.allocate_ui(egui::vec2(80.0, 28.0), width_value);
                } else {
                    width_value(ui);
                }
                crate::ui_kit::styled_dropdown(
                    ui,
                    &format!("pattern-{line}"),
                    120.0,
                    theme.ui.text,
                    crate::ui_kit::line_style_preview(plot.pattern),
                    |ui| {
                        for pattern in crate::model::LineStyle::all() {
                            ui.selectable_value(
                                &mut plot.pattern,
                                *pattern,
                                l10n::text(ui.ctx(), crate::ui_kit::line_style_preview(*pattern)),
                            );
                        }
                    },
                );
                crate::ui_kit::styled_dropdown(
                    ui,
                    &format!("plot-type-{line}"),
                    140.0,
                    theme.ui.text,
                    plot.plot_type.name(),
                    |ui| {
                        for kind in PlotType::ALL {
                            let label = l10n::text(ui.ctx(), kind.name());
                            match kind
                                .incompatibility(overlay, price_scale)
                                .filter(|_| kind != plot.plot_type)
                            {
                                Some(reason) => {
                                    ui.add_enabled(false, egui::Button::selectable(false, label))
                                        .on_disabled_hover_text(l10n::text(ui.ctx(), reason));
                                }
                                None => {
                                    ui.selectable_value(&mut plot.plot_type, kind, label);
                                }
                            }
                        }
                    },
                );
            };
            if wide {
                ui.horizontal(|ui| draw(ui, plot, true));
            } else {
                ui.horizontal_wrapped(|ui| draw(ui, plot, false));
            }
        }
        self.plot_pickers = pickers;
        let labels = self.shade_labels;
        shading_rows(ui, edit, labels);
        let width = crate::ui_kit::label_width(ui, theme, ["Decimals"], ui.available_width() * 0.4);
        crate::ui_kit::section_header(ui, "PRECISION", theme);
        crate::ui_kit::setting_row(ui, "Decimals", width, theme, |ui| {
            let mut automatic = edit.settings.precision.is_none();
            if ui
                .checkbox(&mut automatic, l10n::text(ui.ctx(), "Automatic"))
                .changed()
            {
                edit.settings.precision = if automatic { None } else { Some(2) };
            }
            if let Some(precision) = &mut edit.settings.precision {
                ui.add(
                    egui::DragValue::new(precision)
                        .range(0..=12)
                        .suffix(l10n::text(ui.ctx(), " decimals")),
                );
            }
        });
    }

    fn draw_visibility(&mut self, ui: &mut Ui, edit: &mut StudyEdit, theme: &Theme) {
        crate::ui_kit::section_header(ui, "INTERVALS", theme);
        let anchor = self.current_interval.unwrap_or(Timeframe::Min1);
        let mut range = edit.settings.interval_range;
        let width = crate::ui_kit::label_width(ui, theme, ["Range"], ui.available_width() * 0.4);
        crate::ui_kit::setting_row(ui, "Range", width, theme, |ui| {
            crate::ui_kit::styled_dropdown(
                ui,
                "interval_range",
                200.0,
                theme.ui.text,
                range_name(range),
                |ui| {
                    for choice in [
                        StudyIntervalRange::All,
                        StudyIntervalRange::Only(anchor),
                        StudyIntervalRange::AndAbove(anchor),
                        StudyIntervalRange::AndBelow(anchor),
                    ] {
                        ui.selectable_value(
                            &mut range,
                            choice,
                            l10n::text(ui.ctx(), range_name(choice)),
                        );
                    }
                },
            );
        });
        edit.settings.interval_range = range;
        ui.label(l10n::text(ui.ctx(), "Show this study on these intervals:"));
        let mut intervals = Timeframe::all();
        for interval in self
            .current_interval
            .iter()
            .chain(&edit.settings.hidden_intervals)
        {
            if !intervals.contains(interval) {
                intervals.push(*interval);
            }
        }
        egui::Grid::new("intervals")
            .spacing([8.0, 4.0])
            .show(ui, |ui| {
                for (index, interval) in intervals.iter().copied().enumerate() {
                    let allowed = self
                        .current_interval
                        .is_none_or(|current| range_allows(range, current, interval));
                    let mut visible =
                        allowed && !edit.settings.hidden_intervals.contains(&interval);
                    let response = ui
                        .allocate_ui_with_layout(
                            egui::vec2(72.0, 24.0),
                            egui::Layout::left_to_right(egui::Align::Center),
                            |ui| {
                                ui.add_enabled(
                                    allowed,
                                    egui::Checkbox::new(
                                        &mut visible,
                                        l10n::text(ui.ctx(), interval.to_string()),
                                    ),
                                )
                            },
                        )
                        .inner
                        .on_disabled_hover_text(l10n::text(ui.ctx(), "Outside the selected range"));
                    if response.changed() && allowed {
                        edit.settings
                            .hidden_intervals
                            .retain(|timeframe| *timeframe != interval);
                        if !visible {
                            edit.settings.hidden_intervals.push(interval);
                        }
                    }
                    if index % 4 == 3 {
                        ui.end_row();
                    }
                }
            });
    }
}

fn draw_input_control(
    ui: &mut Ui,
    input: &mut crate::studies::StudyInput,
    sources: &[(crate::studies::StudySource, String)],
    theme: &Theme,
) {
    let _ = theme;
    match (&input.kind, &mut input.value) {
        (InputKind::Length(min, max), InputValue::Length(value)) => {
            ui.add(egui::DragValue::new(value).range(*min..=*max))
        }
        (InputKind::Integer(min, max), InputValue::Integer(value)) => {
            ui.add(egui::DragValue::new(value).range(*min..=*max))
        }
        (InputKind::Number(min, max), InputValue::Number(value)) => {
            ui.add(egui::DragValue::new(value).speed(0.1).range(*min..=*max))
        }
        (InputKind::Choice(choices), InputValue::Choice(value)) => {
            egui::ComboBox::from_id_salt(&input.id)
                .selected_text(l10n::text(
                    ui.ctx(),
                    choices.get(*value).map_or("", String::as_str),
                ))
                .show_ui(ui, |ui| {
                    for (index, label) in choices.iter().enumerate() {
                        ui.selectable_value(value, index, l10n::text(ui.ctx(), label));
                    }
                })
                .response
        }
        (InputKind::Toggle, InputValue::Toggle(value)) => ui.checkbox(value, ""),
        (InputKind::Source, InputValue::Source(value)) => {
            egui::ComboBox::from_id_salt(&input.id)
                .selected_text(l10n::text(
                    ui.ctx(),
                    match value {
                        crate::studies::StudySource::Price(source) => source.label().to_owned(),
                        crate::studies::StudySource::Study { .. } => sources
                            .iter()
                            .find(|(source, _)| source == value)
                            .map(|(_, label)| label.clone())
                            .unwrap_or_else(|| "Unavailable study".to_owned()),
                    },
                ))
                .show_ui(ui, |ui| {
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
                })
                .response
        }
        (InputKind::Color, InputValue::Color(value)) => {
            let id = egui::Id::unique(("input-color", &input.id));
            let mut picker = ui
                .ctx()
                .data(|data| data.get_temp::<ColorPickerState>(id))
                .unwrap_or_else(|| ColorPickerState::new(*value));
            picker.sel_color = *value;
            ColorPicker::new(&mut picker, id).well_size(18.0).show(ui);
            *value = picker.sel_color;
            ui.ctx().data_mut(|data| data.insert_temp(id, picker));
            ui.label("")
        }
        (InputKind::Symbol, InputValue::Symbol(value)) => {
            let response = ui.add(
                egui::TextEdit::singleline(value).hint_text(l10n::text(ui.ctx(), "Chart symbol")),
            );
            // A placeholder counts as a name, so the row label would not be attached.
            ui.ctx().accesskit_node_builder(response.id, |node| {
                node.set_placeholder(String::new());
            });
            response
        }
        (InputKind::Text, InputValue::Text(value)) => ui.add(egui::TextEdit::singleline(value)),
        (InputKind::Timeframe, InputValue::Timeframe(value)) => {
            let response = egui::ComboBox::from_id_salt(&input.id)
                .selected_text(l10n::text(
                    ui.ctx(),
                    value.map_or_else(|| "Chart interval".to_owned(), |tf| tf.to_string()),
                ))
                .show_ui(ui, |ui| {
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
                })
                .response;
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
            response
        }
        _ => ui.label(l10n::text(ui.ctx(), "This input is unavailable.")),
    };
}

fn shading_rows(ui: &mut Ui, edit: &mut StudyEdit, labels: Option<[&'static str; 2]>) {
    let Some(mut shading) = edit.settings.shading.clone() else {
        return;
    };
    let theme = ui.ctx().active_theme();
    crate::ui_kit::section_header(ui, "SHADING", &theme);
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
            l10n::text(ui.ctx(), "Gradient fill"),
        );
    }
    let [bull, bear] = labels.unwrap_or(match shading.kind {
        ShadingKind::Bounded { .. } => ["Oversold", "Overbought"],
        _ => ["Bullish", "Bearish"],
    });
    let expansion_only = labels.is_some_and(|labels| labels[0] == "Expansion");
    match shading.kind {
        ShadingKind::MomentumColumns { .. } => {
            color_row(ui, "Grow above", &mut shading.bull);
            color_row(ui, "Fall above", &mut shading.faded_bull);
            color_row(ui, "Grow below", &mut shading.bear);
            color_row(ui, "Fall below", &mut shading.faded_bear);
        }
        ShadingKind::RisingColumns { .. } => {
            color_row(ui, "Rising", &mut shading.bull);
            color_row(ui, "Falling", &mut shading.bear);
        }
        ShadingKind::Intensity { .. } => color_row(ui, "Accent", &mut shading.accent),
        ShadingKind::FlatChannel { .. } => color_row(ui, "Fill", &mut shading.band),
        ShadingKind::AdxInk { .. } => {
            color_row(ui, "+DI", &mut shading.bull);
            color_row(ui, "\u{2212}DI", &mut shading.bear);
        }
        ShadingKind::BandOnly { .. } => {
            color_row(ui, "Band fill", &mut shading.band);
            color_row(ui, "Levels", &mut shading.levels);
        }
        _ if expansion_only => color_row(ui, bull, &mut shading.bull),
        _ => {
            color_row(ui, bull, &mut shading.bull);
            color_row(ui, bear, &mut shading.bear);
            if matches!(
                shading.kind,
                ShadingKind::Bounded {
                    band_fill: true,
                    ..
                }
            ) {
                color_row(ui, "Band fill", &mut shading.band);
            }
            if matches!(
                shading.kind,
                ShadingKind::Bounded { .. } | ShadingKind::Centered { .. }
            ) {
                color_row(ui, "Levels", &mut shading.levels);
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
            if ui
                .add(
                    egui::DragValue::new(&mut zones)
                        .range(0..=100)
                        .prefix(l10n::text(ui.ctx(), "Zones "))
                        .suffix(" %"),
                )
                .changed()
            {
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
            if ui
                .add(
                    egui::DragValue::new(&mut fill)
                        .range(0..=100)
                        .prefix(l10n::text(ui.ctx(), "Fill "))
                        .suffix(" %"),
                )
                .changed()
            {
                *fill_t = fill as u8;
            }
        }
        ShadingKind::Intensity { range, .. } => match range {
            crate::studies::IntensityRange::Threshold(value) => level(ui, "Threshold", value),
            crate::studies::IntensityRange::Rolling(bars) => {
                let mut value = *bars as f64;
                if ui
                    .add(
                        egui::DragValue::new(&mut value)
                            .speed(1.0)
                            .range(1.0..=10_000.0)
                            .prefix(l10n::text(ui.ctx(), "Lookback ")),
                    )
                    .changed()
                {
                    *bars = value.round() as usize;
                }
            }
        },
        _ => {}
    }
    edit.settings.shading = Some(shading);
}

fn level(ui: &mut Ui, label: &str, value: &mut f64) {
    ui.add(
        egui::DragValue::new(value)
            .speed(0.1)
            .prefix(l10n::text(ui.ctx(), label) + " "),
    );
}

fn color_row(ui: &mut Ui, label: &str, color: &mut Color32) {
    let id = egui::Id::unique(("shade-color", label));
    let mut picker = ui
        .ctx()
        .data(|data| data.get_temp::<ColorPickerState>(id))
        .unwrap_or_else(|| ColorPickerState::new(*color));
    picker.sel_color = *color;
    let theme = ui.ctx().active_theme();
    crate::ui_kit::setting_row(ui, label, 160.0, &theme, |ui| {
        ColorPicker::new(&mut picker, id).well_size(18.0).show(ui);
    });
    *color = picker.sel_color;
    ui.ctx().data_mut(|data| data.insert_temp(id, picker));
}

fn range_name(range: StudyIntervalRange) -> &'static str {
    match range {
        StudyIntervalRange::All => "All",
        StudyIntervalRange::Only(_) => "Only current",
        StudyIntervalRange::AndAbove(_) => "Current and above",
        StudyIntervalRange::AndBelow(_) => "Current and below",
    }
}

fn range_allows(range: StudyIntervalRange, _current: Timeframe, interval: Timeframe) -> bool {
    range.contains(interval)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::studies::{ConfiguredStudy, SMA};

    #[test]
    fn dialog_follows_the_window_when_it_shrinks() {
        for theme in [Theme::light(), Theme::dark()] {
            for (name, tab) in [
                ("MA Ribbon", None),
                ("MA Ribbon", Some(Tab::Inputs)),
                ("MA Ribbon", Some(Tab::Style)),
                ("MA Ribbon", Some(Tab::Visibility)),
                ("Vol Profile FR", Some(Tab::Inputs)),
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
                if let Some(tab) = tab {
                    dialog.tab = tab;
                }
                for size in [
                    egui::vec2(1440.0, 900.0),
                    egui::vec2(320.0, 360.0),
                    egui::vec2(600.0, 300.0),
                ] {
                    let size = if tab.is_none() {
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
                                if tab.is_some() {
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
                            m.area_rect(egui::Id::unique(if tab.is_some() {
                                "indicator_settings"
                            } else {
                                "indicator_dialog"
                            }))
                        })
                        .unwrap();
                    assert!(
                        screen.contains_rect(shown),
                        "{name} tab {}: {shown:?} outside {screen:?}",
                        tab.map(|tab| tab as u8).unwrap_or(3)
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
}
