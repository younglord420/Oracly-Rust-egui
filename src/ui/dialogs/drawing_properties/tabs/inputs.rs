//! Inputs that correspond to the selected tool's saved drawing configuration.

use crate::drawings::{
    Drawing, DrawingToolType as Tool, FibPreset, FibonacciLevel, GannConfig, GannFanAngle,
    GannFanDirection, PitchforkConfig, PitchforkLevel, PositionSize, RegressionSource,
    TextHorizontalAlign, TextVerticalAlign,
};

use crate::data::{ProfileDisplay, VolumeProfileConfig};
use crate::localization as l10n;
use crate::model::LineStyle;
use crate::theme::ThemeExt;
use crate::ui_kit::ext::UiExt;
use crate::ui_kit::{
    ColorPicker, ColorPickerState, FormGrid, buttons::Button, form_control_width_for, form_label,
};
use egui::Ui;

pub(crate) struct InputsTab;

impl InputsTab {
    pub(crate) fn available(tool: Tool) -> bool {
        matches!(
            tool,
            Tool::FibonacciRetracement
                | Tool::FibonacciChannel
                | Tool::FibonacciExtension
                | Tool::FibonacciWedge
                | Tool::FibonacciTimeZones
                | Tool::FibonacciSpeedFan
                | Tool::TrendBasedFibTime
                | Tool::FibonacciCircles
                | Tool::FibonacciSpeedResistanceArcs
                | Tool::Pitchfork
                | Tool::SchiffPitchfork
                | Tool::ModifiedSchiffPitchfork
                | Tool::InsidePitchfork
                | Tool::Pitchfan
                | Tool::GannFan
                | Tool::GannBox
                | Tool::GannSquare
                | Tool::GannFixed
                | Tool::GhostFeed
                | Tool::LongPos
                | Tool::ShortPos
                | Tool::FixedRangeVolumeProfile
                | Tool::AnchoredVolumeProfile
                | Tool::RegressionTrend
        )
    }

    pub(crate) fn show(ui: &mut Ui, drawing: &mut Drawing, pickers: &mut Vec<ColorPickerState>) {
        match drawing.tool_type {
            Tool::FibonacciRetracement
            | Tool::FibonacciExtension
            | Tool::FibonacciChannel
            | Tool::FibonacciWedge
            | Tool::FibonacciTimeZones
            | Tool::FibonacciSpeedFan
            | Tool::TrendBasedFibTime
            | Tool::FibonacciCircles
            | Tool::FibonacciSpeedResistanceArcs => fibonacci(ui, drawing, pickers),
            Tool::Pitchfork
            | Tool::SchiffPitchfork
            | Tool::ModifiedSchiffPitchfork
            | Tool::InsidePitchfork => pitchfork(ui, drawing, pickers),
            Tool::Pitchfan => pitchfan(ui, drawing, pickers),
            Tool::GannFan => gann_fan(ui, drawing, pickers),
            Tool::GannBox | Tool::GannSquare | Tool::GannFixed => {
                let mut config = drawing.gann.clone().unwrap_or_else(|| {
                    if drawing.tool_type == Tool::GannFixed {
                        GannConfig {
                            price_levels: vec![0.0, 0.25, 0.5, 0.75, 1.0],
                            time_levels: vec![0.0, 0.25, 0.5, 0.75, 1.0],
                            ..GannConfig::default()
                        }
                    } else {
                        GannConfig::default()
                    }
                });
                let original = config.clone();
                let square = drawing.tool_type == Tool::GannSquare;
                let control_h = super::style::control_row_height(ui);
                let col_w = super::style::control_column_width(ui);
                if square {
                    ui.horizontal(|ui| {
                        ui.set_min_height(control_h);
                        ui.checkbox(&mut config.show_grid, l10n::text(ui.ctx(), "Show grid"));
                    });
                }
                if !square || config.show_grid {
                    fractions(
                        ui,
                        "Price divisions",
                        &mut config.price_levels,
                        0.0..=1.0,
                        0.5,
                    );
                    fractions(
                        ui,
                        "Time divisions",
                        &mut config.time_levels,
                        0.0..=1.0,
                        0.5,
                    );
                }
                if square {
                    ui.horizontal(|ui| {
                        ui.set_min_height(control_h);
                        ui.checkbox(&mut config.show_fan, l10n::text(ui.ctx(), "Show fan"));
                        if config.show_fan {
                            ui.with_layout(
                                egui::Layout::right_to_left(egui::Align::Center),
                                |ui| {
                                    let ctx = ui.ctx().clone();
                                    super::style::text_combo_popup(
                                        ui,
                                        "gann_fan_direction",
                                        "Fan direction",
                                        &mut config.fan_direction,
                                        &[
                                            GannFanDirection::Forward,
                                            GannFanDirection::Reverse,
                                            GannFanDirection::Both,
                                        ],
                                        |v| {
                                            l10n::text(
                                                &ctx,
                                                match v {
                                                    GannFanDirection::Forward => "Forward",
                                                    GannFanDirection::Reverse => "Reverse",
                                                    GannFanDirection::Both => "Both",
                                                },
                                            )
                                        },
                                        col_w.min(ui.available_width()).max(96.0),
                                        control_h,
                                    );
                                },
                            );
                        }
                    });
                    if config.show_fan {
                        fractions(ui, "Fan ratios", &mut config.fan_ratios, 0.01..=100.0, 1.0);
                    }
                    ui.horizontal(|ui| {
                        ui.set_min_height(control_h);
                        ui.checkbox(&mut config.show_arcs, l10n::text(ui.ctx(), "Show arcs"));
                    });
                    if config.show_arcs {
                        fractions(ui, "Arc ratios", &mut config.arc_ratios, 0.01..=1.41, 0.5);
                    }
                }
                if config != original {
                    drawing.gann = Some(config);
                }
            }
            Tool::GhostFeed => {
                let mut config = drawing.ghost_feed.unwrap_or_default();
                let original = config;
                FormGrid::new("ghost_feed_inputs").show(ui, |ui| {
                    number(
                        ui,
                        "Average range (ticks)",
                        &mut config.average_range_ticks,
                        f64::MIN_POSITIVE..=f64::MAX,
                    );
                    number(ui, "Noise (ticks)", &mut config.noise_ticks, 0.0..=f64::MAX);
                });
                if config != original {
                    drawing.ghost_feed = Some(config);
                }
            }
            Tool::LongPos | Tool::ShortPos => {
                let mut size = drawing.position_size.unwrap_or_default();
                let original = size;
                let mut mode = match size {
                    PositionSize::Quantity(_) => 0,
                    PositionSize::RiskAmount(_) => 1,
                    PositionSize::RiskPercent { .. } => 2,
                };
                FormGrid::new("position_inputs").show(ui, |ui| {
                    let label = form_label(ui, "Size by");
                    ui.label_inputs_by(label.id, |ui| {
                        ui.combo_select_width(
                            "position_size_mode",
                            &mut mode,
                            [0, 1, 2],
                            |v| ["Quantity", "Risk amount", "Risk percent"][*v].into(),
                            form_control_width_for(ui),
                        );
                    });
                    ui.end_row();
                    if mode
                        != match size {
                            PositionSize::Quantity(_) => 0,
                            PositionSize::RiskAmount(_) => 1,
                            PositionSize::RiskPercent { .. } => 2,
                        }
                    {
                        size = match mode {
                            0 => PositionSize::Quantity(1.0),
                            1 => PositionSize::RiskAmount(100.0),
                            _ => PositionSize::RiskPercent {
                                account_size: 10_000.0,
                                percent: 1.0,
                            },
                        };
                    }
                    match &mut size {
                        PositionSize::Quantity(value) => {
                            number(ui, "Quantity", value, f64::MIN_POSITIVE..=f64::MAX)
                        }
                        PositionSize::RiskAmount(value) => {
                            number(ui, "Risk amount", value, f64::MIN_POSITIVE..=f64::MAX)
                        }
                        PositionSize::RiskPercent {
                            account_size,
                            percent,
                        } => {
                            number(
                                ui,
                                "Account size",
                                account_size,
                                f64::MIN_POSITIVE..=f64::MAX,
                            );
                            number(ui, "Risk (%)", percent, f64::MIN_POSITIVE..=100.0);
                        }
                    }
                    let sizing = &mut drawing.position_sizing;
                    number(
                        ui,
                        "Point value",
                        &mut sizing.point_value,
                        f64::MIN_POSITIVE..=f64::MAX,
                    );
                    number(
                        ui,
                        "Lot size",
                        &mut sizing.lot_size,
                        f64::MIN_POSITIVE..=f64::MAX,
                    );
                    let label = form_label(ui, "Quantity step");
                    ui.add(
                        egui::DragValue::new(&mut sizing.quantity_step)
                            .range(0.0..=f64::MAX)
                            .speed(0.001)
                            .custom_formatter(|value, _| value.to_string()),
                    )
                    .labelled_by(label.id);
                    ui.end_row();
                    number(
                        ui,
                        "Leverage",
                        &mut sizing.leverage,
                        f64::MIN_POSITIVE..=f64::MAX,
                    );
                });
                if size != original {
                    drawing.position_size = Some(size);
                }
            }
            Tool::FixedRangeVolumeProfile | Tool::AnchoredVolumeProfile => {
                let original = drawing.volume_profile.unwrap_or_default();
                let mut config = original;
                let mut rows = config.rows();
                let mut fixed = config.row_size().is_some();
                let mut height = config.row_size().unwrap_or(1.0);
                let mut percent = config.value_area() * 100.0;
                FormGrid::new("profile_inputs").show(ui, |ui| {
                    let control_width = form_control_width_for(ui);
                    let label = form_label(ui, "Row layout");
                    ui.label_inputs_by(label.id, |ui| {
                        ui.combo_select_width(
                            "profile_layout",
                            &mut fixed,
                            [false, true],
                            |fixed| {
                                if *fixed {
                                    "Price height".into()
                                } else {
                                    "Number of rows".into()
                                }
                            },
                            control_width,
                        );
                    });
                    ui.end_row();
                    if fixed {
                        number(
                            ui,
                            "Price height",
                            &mut height,
                            f64::MIN_POSITIVE..=f64::MAX,
                        );
                    } else {
                        let label = form_label(ui, "Rows");
                        ui.add(egui::DragValue::new(&mut rows).range(VolumeProfileConfig::ROWS))
                            .labelled_by(label.id);
                        ui.end_row();
                    }
                    number(
                        ui,
                        "Value area (%)",
                        &mut percent,
                        f64::MIN_POSITIVE..=100.0,
                    );
                    let label = form_label(ui, "Volume");
                    ui.label_inputs_by(label.id, |ui| {
                        ui.combo_select_width(
                            "profile_display",
                            &mut config.display,
                            [
                                ProfileDisplay::Total,
                                ProfileDisplay::UpDown,
                                ProfileDisplay::Delta,
                            ],
                            |mode| match mode {
                                ProfileDisplay::Total => "Total".into(),
                                ProfileDisplay::UpDown => "Up / down".into(),
                                ProfileDisplay::Delta => "Delta".into(),
                            },
                            control_width,
                        );
                    });
                    ui.end_row();
                    if config.display != ProfileDisplay::Total {
                        pickers.resize_with(2, ColorPickerState::default);
                        let label = form_label(ui, "Up colour");
                        ui.label_inputs_by(label.id, |ui| {
                            level_color(ui, &mut config.up_color, &mut pickers[0]);
                        });
                        ui.end_row();
                        let label = form_label(ui, "Down colour");
                        ui.label_inputs_by(label.id, |ui| {
                            level_color(ui, &mut config.down_color, &mut pickers[1]);
                        });
                        ui.end_row();
                    }
                });
                if let Some(mut updated) = VolumeProfileConfig::new(rows, percent / 100.0)
                    .and_then(|c| c.with_row_size(fixed.then_some(height)))
                {
                    updated.display = config.display;
                    updated.up_color = config.up_color;
                    updated.down_color = config.down_color;
                    if updated != original {
                        drawing.volume_profile = Some(updated);
                    }
                }
            }
            Tool::RegressionTrend => {
                let mut config = drawing.regression_trend.unwrap_or_default();
                let original = config;
                let theme = ui.active_theme();
                let control_h = super::style::control_row_height(ui);
                let col_w = super::style::control_column_width(ui);
                ui.spacing_mut().interact_size.y = control_h;

                // Row 1: Source
                ui.horizontal(|ui| {
                    ui.set_min_height(control_h);
                    ui.label(
                        egui::RichText::new(l10n::text(ui.ctx(), "Source"))
                            .size(theme.text.body)
                            .color(theme.ui.text_secondary),
                    );
                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        let ctx = ui.ctx().clone();
                        super::style::text_combo_popup(
                            ui,
                            "regression_source",
                            "Source",
                            &mut config.source,
                            &RegressionSource::all(),
                            |v| l10n::text(&ctx, v.label()),
                            col_w.min(ui.available_width()),
                            control_h,
                        );
                    });
                });

                // Row 2: [✓] Upper deviation  [value]
                ui.horizontal(|ui| {
                    ui.set_min_height(control_h);
                    ui.checkbox(
                        &mut config.use_upper,
                        l10n::text(ui.ctx(), "Upper deviation"),
                    );
                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        ui.add_enabled(
                            config.use_upper,
                            egui::DragValue::new(&mut config.upper_deviation)
                                .range(f64::MIN_POSITIVE..=10.0)
                                .speed(0.1),
                        )
                        .accessible_name(l10n::text(ui.ctx(), "Upper deviation"));
                    });
                });

                // Row 3: [✓] Lower deviation  [value]
                ui.horizontal(|ui| {
                    ui.set_min_height(control_h);
                    ui.checkbox(
                        &mut config.use_lower,
                        l10n::text(ui.ctx(), "Lower deviation"),
                    );
                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        ui.add_enabled(
                            config.use_lower,
                            egui::DragValue::new(&mut config.lower_deviation)
                                .range(f64::MIN_POSITIVE..=10.0)
                                .speed(0.1),
                        )
                        .accessible_name(l10n::text(ui.ctx(), "Lower deviation"));
                    });
                });

                // Row 4: [✓] Pearson's R
                ui.horizontal(|ui| {
                    ui.set_min_height(control_h);
                    ui.checkbox(
                        &mut config.show_pearson,
                        l10n::text(ui.ctx(), "Pearson's R"),
                    );
                });

                if config != original
                    && let Some(valid) =
                        config.with_deviations(config.upper_deviation, config.lower_deviation)
                {
                    drawing.regression_trend = Some(valid);
                }
            }
            _ => {}
        }
    }
}

fn number(ui: &mut Ui, label: &str, value: &mut f64, range: std::ops::RangeInclusive<f64>) {
    let label = form_label(ui, label);
    ui.add(egui::DragValue::new(value).range(range).speed(0.1))
        .labelled_by(label.id);
    ui.end_row();
}

fn background(ui: &mut Ui, show: &mut bool, opacity: &mut u8) {
    let theme = ui.active_theme();
    let control_h = super::style::control_row_height(ui);
    ui.horizontal(|ui| {
        ui.set_min_height(control_h);
        ui.spacing_mut().item_spacing.x = theme.spacing.sm;
        ui.checkbox(show, l10n::text(ui.ctx(), "Background"));
        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
            let mut percent = f32::from(*opacity) * 100.0 / 255.0;
            if super::style::opacity_slider(ui, &mut percent, *show)
                .accessible_name(l10n::text(ui.ctx(), "Background opacity"))
                .changed()
            {
                *opacity = (percent * 255.0 / 100.0).round() as u8;
            }
        });
    });
}

fn use_one_color<T>(
    ui: &mut Ui,
    levels: &mut [T],
    picker: &mut ColorPickerState,
    color_of: impl Fn(&mut T) -> &mut [u8; 4],
) {
    let theme = ui.active_theme();
    let control_h = super::style::control_row_height(ui);
    ui.horizontal(|ui| {
        ui.set_min_height(control_h);
        ui.spacing_mut().item_spacing.x = theme.spacing.sm;
        ui.label(
            egui::RichText::new(l10n::text(ui.ctx(), "Use one color"))
                .size(theme.text.body)
                .color(theme.ui.text_secondary),
        );
        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
            let mut color = levels
                .first_mut()
                .map_or([128, 128, 128, 255], |l| *color_of(l));
            if level_color(ui, &mut color, picker) {
                for level in levels.iter_mut() {
                    *color_of(level) = color;
                }
            }
        });
    });
}

/// Borrowed row fields let each tool keep its own saved level model.
struct LevelRow<'a> {
    visible: &'a mut bool,
    value: &'a mut f32,
    legacy_label: Option<&'a mut String>,
    color: &'a mut [u8; 4],
    line_style: Option<&'a mut LineStyle>,
    width: Option<(&'a mut Option<f32>, f32)>,
    range: std::ops::RangeInclusive<f32>,
}

fn level_grid<T>(
    ui: &mut Ui,
    levels: &mut Vec<T>,
    pickers: &mut Vec<ColorPickerState>,
    new_level: T,
    row: impl Fn(&mut T) -> LevelRow<'_>,
) {
    let theme = ui.active_theme();
    let control_h = super::style::control_row_height(ui);
    ui.spacing_mut().interact_size.y = control_h;
    pickers.resize_with(levels.len() + 1, ColorPickerState::default);

    let has_style = levels
        .first_mut()
        .is_some_and(|l| row(l).line_style.is_some());
    let has_width = levels.first_mut().is_some_and(|l| row(l).width.is_some());
    let compact = ui.available_width() < 260.0;
    let grid_gap = if compact {
        theme.spacing.xs
    } else {
        theme.spacing.sm
    };
    let mut remove = None;

    if !has_style && !has_width {
        let two_col = ui.available_width() >= 310.0;
        if two_col {
            // Two-column side-by-side layout for Fibonacci level lists:
            // [✓] [ratio] [color] [trash]  |  [✓] [ratio] [color] [trash]
            let ratio_w = 50.0;
            egui::Grid::new(ui.scope_id().with("levels_table_2col"))
                .num_columns(8)
                .spacing([grid_gap, theme.spacing.xs])
                .show(ui, |ui| {
                    for (i, level) in levels.iter_mut().enumerate() {
                        ui.push_id(i, |ui| {
                            let fields = row(level);
                            let name = |ui: &Ui, property| {
                                l10n::format_text(ui.ctx(), property, &[("0", (i + 1).to_string())])
                            };
                            ui.checkbox(fields.visible, "")
                                .accessible_name(name(ui, "Level {0} visible"));
                            if ui
                                .add_sized(
                                    [ratio_w, control_h],
                                    egui::DragValue::new(fields.value)
                                        .range(fields.range)
                                        .speed(0.001)
                                        .max_decimals(6),
                                )
                                .accessible_name(name(ui, "Level {0} ratio"))
                                .changed()
                                && let Some(label) = fields.legacy_label
                            {
                                label.clear();
                            }
                            level_color(ui, fields.color, &mut pickers[i + 1]);
                            if Button::icon(&crate::icons::icons::TRASH)
                                .ghost()
                                .small()
                                .tooltip("Remove")
                                .show(ui)
                                .clicked()
                            {
                                remove = Some(i);
                            }
                        });
                        if (i + 1) % 2 == 0 {
                            ui.end_row();
                        }
                    }
                    if !levels.len().is_multiple_of(2) {
                        ui.end_row();
                    }
                });
        } else {
            let ratio_w = 64.0;
            egui::Grid::new(ui.scope_id().with("levels_table_1col"))
                .num_columns(4)
                .spacing([grid_gap, theme.spacing.xs])
                .show(ui, |ui| {
                    for (i, level) in levels.iter_mut().enumerate() {
                        ui.push_id(i, |ui| {
                            let fields = row(level);
                            let name = |ui: &Ui, property| {
                                l10n::format_text(ui.ctx(), property, &[("0", (i + 1).to_string())])
                            };
                            ui.checkbox(fields.visible, "")
                                .accessible_name(name(ui, "Level {0} visible"));
                            if ui
                                .add_sized(
                                    [ratio_w, control_h],
                                    egui::DragValue::new(fields.value)
                                        .range(fields.range)
                                        .speed(0.001)
                                        .max_decimals(6),
                                )
                                .accessible_name(name(ui, "Level {0} ratio"))
                                .changed()
                                && let Some(label) = fields.legacy_label
                            {
                                label.clear();
                            }
                            level_color(ui, fields.color, &mut pickers[i + 1]);
                            if Button::icon(&crate::icons::icons::TRASH)
                                .ghost()
                                .small()
                                .tooltip("Remove")
                                .show(ui)
                                .clicked()
                            {
                                remove = Some(i);
                            }
                        });
                        ui.end_row();
                    }
                });
        }
    } else {
        // Single-column rich row for tools with per-level line style and stroke width (Pitchfork).
        let num_cols = 4 + usize::from(has_style) + usize::from(has_width);
        let ratio_w = if compact { 44.0 } else { 56.0 };
        let style_w = if compact { 54.0 } else { 72.0 };
        let width_w = if compact { 48.0 } else { 62.0 };
        egui::Grid::new(ui.scope_id().with("levels_table"))
            .num_columns(num_cols)
            .spacing([grid_gap, theme.spacing.xs])
            .show(ui, |ui| {
                for (i, level) in levels.iter_mut().enumerate() {
                    ui.push_id(i, |ui| {
                        let fields = row(level);
                        let name = |ui: &Ui, property| {
                            l10n::format_text(ui.ctx(), property, &[("0", (i + 1).to_string())])
                        };
                        ui.checkbox(fields.visible, "")
                            .accessible_name(name(ui, "Level {0} visible"));
                        if ui
                            .add_sized(
                                [ratio_w, control_h],
                                egui::DragValue::new(fields.value)
                                    .range(fields.range)
                                    .speed(0.001)
                                    .max_decimals(6),
                            )
                            .accessible_name(name(ui, "Level {0} ratio"))
                            .changed()
                            && let Some(label) = fields.legacy_label
                        {
                            label.clear();
                        }
                        level_color(ui, fields.color, &mut pickers[i + 1]);
                        if let Some(style) = fields.line_style {
                            super::style::line_style_combo(
                                ui,
                                ("level_style", i),
                                &name(ui, "Level {0} line style"),
                                style,
                                style_w,
                                control_h,
                            );
                        }
                        if let Some((width, inherited)) = fields.width {
                            let mut value = width.unwrap_or(inherited);
                            let before = value;
                            super::style::line_width_combo_with_id(
                                ui,
                                ("level_width", i),
                                &name(ui, "Level {0} width"),
                                &mut value,
                                width_w,
                                control_h,
                            );
                            if (value - before).abs() > f32::EPSILON {
                                *width = Some(value);
                            }
                        }
                        if Button::icon(&crate::icons::icons::TRASH)
                            .ghost()
                            .small()
                            .tooltip("Remove")
                            .show(ui)
                            .clicked()
                        {
                            remove = Some(i);
                        }
                    });
                    ui.end_row();
                }
            });
    }

    if let Some(i) = remove {
        levels.remove(i);
        pickers.remove(i + 1);
    }
    use_one_color(ui, levels, &mut pickers[0], |l| row(l).color);
    if Button::new("Add level")
        .secondary()
        .small()
        .show(ui)
        .clicked()
    {
        levels.push(new_level);
    }
}

/// Font size presets offered on the Fibonacci Labels row.
const FIB_LABEL_FONT_SIZES: &[f32] = &[10.0, 12.0, 14.0, 16.0, 20.0, 24.0];

fn fibonacci(ui: &mut Ui, drawing: &mut Drawing, pickers: &mut Vec<ColorPickerState>) {
    let mut config = drawing.fibonacci_config().into_owned();
    let original = config.clone();
    let theme = ui.active_theme();
    let control_h = super::style::control_row_height(ui);
    ui.spacing_mut().interact_size.y = control_h;

    ui.horizontal(|ui| {
        ui.set_min_height(control_h);
        ui.strong(l10n::text(ui.ctx(), "Fibonacci levels"));
        if drawing.tool_type == Tool::FibonacciRetracement {
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                let ctx = ui.ctx().clone();
                let mut preset = FibPreset::matching(&config);
                let before = preset;
                super::style::text_combo_popup(
                    ui,
                    "fib_level_preset",
                    "Level preset",
                    &mut preset,
                    &FibPreset::ALL.map(Some),
                    |p| l10n::text(&ctx, p.map_or("Custom", FibPreset::label)),
                    super::style::control_column_width(ui)
                        .min(ui.available_width())
                        .clamp(92.0, 140.0),
                    control_h,
                );
                if preset != before
                    && let Some(preset) = preset
                {
                    let (levels, reverse) = preset.levels();
                    config.levels = levels;
                    config.reverse = reverse;
                }
            });
        }
    });
    let next_color = crate::drawings::domain::fib_ratio_color(1.0, config.levels.len());
    level_grid(
        ui,
        &mut config.levels,
        pickers,
        FibonacciLevel::new(1.0, "", next_color),
        |level| LevelRow {
            visible: &mut level.visible,
            value: &mut level.value,
            legacy_label: Some(&mut level.label),
            color: &mut level.color,
            line_style: None,
            width: None,
            range: if matches!(
                drawing.tool_type,
                Tool::FibonacciCircles | Tool::FibonacciSpeedResistanceArcs | Tool::FibonacciWedge
            ) {
                0.0
            } else {
                -f32::MAX
            }..=f32::MAX,
        },
    );

    background(
        ui,
        &mut config.show_background,
        &mut config.background_opacity,
    );

    let has_prices = matches!(
        drawing.tool_type,
        Tool::FibonacciRetracement | Tool::FibonacciExtension | Tool::FibonacciChannel
    );

    // Row: Reverse | Prices
    if drawing.tool_type == Tool::FibonacciRetracement || has_prices {
        ui.horizontal(|ui| {
            ui.set_min_height(control_h);
            ui.spacing_mut().item_spacing.x = theme.spacing.lg;
            if drawing.tool_type == Tool::FibonacciRetracement {
                ui.checkbox(&mut config.reverse, l10n::text(ui.ctx(), "Reverse"));
            }
            if has_prices {
                let mut prices = config
                    .show_prices
                    .unwrap_or(drawing.tool_type != Tool::FibonacciChannel);
                if ui
                    .checkbox(&mut prices, l10n::text(ui.ctx(), "Prices"))
                    .changed()
                {
                    config.show_prices = Some(prices);
                }
            }
        });
    }

    // Row: [✓] Levels   [Values / Percent]
    ui.horizontal(|ui| {
        ui.set_min_height(control_h);
        ui.spacing_mut().item_spacing.x = theme.spacing.sm;
        let mut levels = config
            .show_levels
            .unwrap_or(drawing.tool_type != Tool::FibonacciWedge);
        if ui
            .checkbox(&mut levels, l10n::text(ui.ctx(), "Levels"))
            .changed()
        {
            config.show_levels = Some(levels);
        }
        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
            let ctx = ui.ctx().clone();
            let chip_w = super::style::control_column_width(ui)
                .min(ui.available_width())
                .clamp(92.0, 140.0);
            super::style::text_combo_popup(
                ui,
                "fib_levels_format",
                "Levels format",
                &mut config.levels_as_percent,
                &[false, true],
                |percent| l10n::text(&ctx, if percent { "Percent" } else { "Values" }),
                chip_w,
                control_h,
            );
        });
    });

    // Row: Labels   [Left/Center/Right] [Top/Middle/Bottom] [Font size]
    if matches!(
        drawing.tool_type,
        Tool::FibonacciRetracement | Tool::FibonacciExtension | Tool::FibonacciChannel
    ) {
        ui.horizontal(|ui| {
            ui.set_min_height(control_h);
            let gap = 4.0;
            ui.spacing_mut().item_spacing.x = gap;
            let compact = ui.available_width() < 260.0;
            if !compact {
                ui.label(
                    egui::RichText::new(l10n::text(ui.ctx(), "Labels"))
                        .size(theme.text.body)
                        .color(theme.ui.text_secondary),
                );
            }
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                let avail = ui.available_width();
                let strip_w = if compact {
                    avail
                } else {
                    super::style::control_column_width(ui).max(196.0).min(avail)
                };
                let size_w = 54.0_f32.min(strip_w * 0.28);
                let align_w = ((strip_w - size_w - gap * 2.0) * 0.5).max(52.0);
                ui.allocate_ui_with_layout(
                    egui::Vec2::new(strip_w, control_h),
                    egui::Layout::left_to_right(egui::Align::Center),
                    |ui| {
                        ui.spacing_mut().item_spacing.x = gap;
                        let ctx = ui.ctx().clone();
                        let mut h_align = config.effective_h_align();
                        let before_h = h_align;
                        super::style::text_combo_popup(
                            ui,
                            "fib_label_h_align",
                            "Label horizontal alignment",
                            &mut h_align,
                            TextHorizontalAlign::all(),
                            |a| l10n::text(&ctx, a.label()),
                            align_w,
                            control_h,
                        );
                        if h_align != before_h {
                            config.set_h_align(h_align);
                        }

                        let ctx = ui.ctx().clone();
                        if !TextVerticalAlign::all().contains(&config.label_v_align) {
                            config.label_v_align = TextVerticalAlign::Middle;
                        }
                        super::style::text_combo_popup(
                            ui,
                            "fib_label_v_align",
                            "Label vertical alignment",
                            &mut config.label_v_align,
                            TextVerticalAlign::all(),
                            |a| l10n::text(&ctx, a.label()),
                            align_w,
                            control_h,
                        );

                        let mut sizes = FIB_LABEL_FONT_SIZES.to_vec();
                        if !sizes
                            .iter()
                            .any(|s| (*s - config.label_font_size).abs() < f32::EPSILON)
                        {
                            sizes.push(config.label_font_size);
                        }
                        super::style::text_combo_popup(
                            ui,
                            "fib_label_font_size",
                            "Label font size",
                            &mut config.label_font_size,
                            &sizes,
                            |s| format!("{}", s.round() as i32),
                            size_w,
                            control_h,
                        );
                    },
                );
            });
        });
    } else {
        ui.horizontal(|ui| {
            ui.set_min_height(control_h);
            ui.checkbox(
                &mut config.labels_on_left,
                l10n::text(ui.ctx(), "Labels on left"),
            );
        });
    }

    // Row: Fib levels based on log scale
    if matches!(
        drawing.tool_type,
        Tool::FibonacciRetracement | Tool::FibonacciExtension
    ) {
        ui.horizontal(|ui| {
            ui.set_min_height(control_h);
            ui.checkbox(
                &mut config.logarithmic,
                l10n::text(ui.ctx(), "Fib levels based on log scale"),
            );
        });
    }

    if config != original {
        drawing.fib_config = Some(config);
    }
}

fn pitchfork(ui: &mut Ui, drawing: &mut Drawing, pickers: &mut Vec<ColorPickerState>) {
    let mut config = drawing.pitchfork.clone().unwrap_or_else(|| {
        let mut config = PitchforkConfig::default();
        for level in &mut config.levels {
            // Keep distinct median / half colours; paint the rest with the drawing colour.
            if (level.value - 0.0).abs() > 1e-4 && (level.value - 0.5).abs() > 1e-4 {
                level.color = drawing.color;
            }
            level.width = Some(if (level.value - 1.0).abs() < f32::EPSILON {
                crate::styles::stroke::HAIRLINE
            } else {
                drawing.stroke_width
            });
        }
        config
    });
    config.ensure_standard_levels();
    let original = config.clone();
    ui.strong(l10n::text(ui.ctx(), "Pitchfork levels"));
    ui.label(
        egui::RichText::new(l10n::text(
            ui.ctx(),
            "0 = median · 1 = tines · check to show",
        ))
        .size(ui.active_theme().text.small)
        .color(ui.active_theme().ui.text_secondary),
    );
    level_grid(
        ui,
        &mut config.levels,
        pickers,
        PitchforkLevel::new(2.0, true, drawing.color),
        |level| LevelRow {
            visible: &mut level.visible,
            value: &mut level.value,
            legacy_label: None,
            color: &mut level.color,
            line_style: Some(&mut level.line_style),
            width: Some((&mut level.width, drawing.stroke_width)),
            range: 0.0..=crate::drawings::validation::MAX_PITCHFORK_LEVEL,
        },
    );
    background(
        ui,
        &mut config.show_background,
        &mut config.background_opacity,
    );
    if config != original {
        drawing.pitchfork = Some(config);
    }
}

fn pitchfan(ui: &mut Ui, drawing: &mut Drawing, pickers: &mut Vec<ColorPickerState>) {
    let mut config = drawing
        .pitchfork
        .clone()
        .unwrap_or_else(PitchforkConfig::pitchfan);
    let original = config.clone();
    ui.strong(l10n::text(ui.ctx(), "Fan levels"));
    level_grid(
        ui,
        &mut config.levels,
        pickers,
        PitchforkLevel::new(0.5, true, drawing.color),
        |level| LevelRow {
            visible: &mut level.visible,
            value: &mut level.value,
            legacy_label: None,
            color: &mut level.color,
            line_style: Some(&mut level.line_style),
            width: Some((&mut level.width, drawing.stroke_width)),
            range: 0.0..=crate::drawings::validation::MAX_PITCHFORK_LEVEL,
        },
    );
    background(
        ui,
        &mut config.show_background,
        &mut config.background_opacity,
    );
    if config != original {
        drawing.pitchfork = Some(config);
    }
}

fn gann_fan(ui: &mut Ui, drawing: &mut Drawing, pickers: &mut Vec<ColorPickerState>) {
    let mut config = drawing.gann.clone().unwrap_or_default();
    let original = config.clone();
    let theme = ui.active_theme();
    let control_h = super::style::control_row_height(ui);
    let compact = ui.available_width() < 300.0;
    let ratio_w = if compact { 50.0 } else { 62.0 };
    let style_w = if compact { 54.0 } else { 72.0 };
    let width_w = if compact { 48.0 } else { 62.0 };
    let mut remove = None;

    pickers.resize_with(config.fan_angles.len(), ColorPickerState::default);
    ui.strong(l10n::text(ui.ctx(), "Fan angles"));
    egui::Grid::new(ui.scope_id().with("gann_fan_angles"))
        .num_columns(6)
        .spacing([theme.spacing.xs, theme.spacing.xs])
        .show(ui, |ui| {
            for (index, angle) in config.fan_angles.iter_mut().enumerate() {
                ui.push_id(index, |ui| {
                    let name = |ui: &Ui, property| {
                        l10n::format_text(ui.ctx(), property, &[("0", (index + 1).to_string())])
                    };
                    ui.checkbox(&mut angle.enabled, "")
                        .accessible_name(name(ui, "Angle {0} visible"));
                    ui.add_sized(
                        [ratio_w, control_h],
                        egui::DragValue::new(&mut angle.ratio)
                            .range(f32::MIN_POSITIVE..=f32::MAX)
                            .speed(0.01)
                            .max_decimals(6),
                    )
                    .accessible_name(name(ui, "Angle {0} ratio"));
                    level_color(ui, &mut angle.color, &mut pickers[index]);
                    super::style::line_width_combo_with_id(
                        ui,
                        ("gann_fan_angle_width", index),
                        &name(ui, "Angle {0} width"),
                        &mut angle.line_width,
                        width_w,
                        control_h,
                    );
                    super::style::line_style_combo(
                        ui,
                        ("gann_fan_angle_style", index),
                        &name(ui, "Angle {0} line style"),
                        &mut angle.line_style,
                        style_w,
                        control_h,
                    );
                    if Button::icon(&crate::icons::icons::TRASH)
                        .ghost()
                        .small()
                        .tooltip("Remove")
                        .show(ui)
                        .clicked()
                    {
                        remove = Some(index);
                    }
                });
                ui.end_row();
            }
        });
    if let Some(index) = remove {
        config.fan_angles.remove(index);
        pickers.remove(index);
    }
    if Button::new("Add angle")
        .secondary()
        .small()
        .show(ui)
        .clicked()
    {
        config
            .fan_angles
            .push(GannFanAngle::new(1.0, drawing.color, drawing.stroke_width));
    }
    if config != original {
        drawing.gann = Some(config);
    }
}

fn fractions(
    ui: &mut Ui,
    title: &str,
    levels: &mut Vec<f32>,
    range: std::ops::RangeInclusive<f32>,
    new: f32,
) {
    let theme = ui.active_theme();
    ui.strong(l10n::text(ui.ctx(), title));
    let mut remove = None;
    let columns = ((ui.available_width() / 96.0) as usize).clamp(1, 3);
    egui::Grid::new(title)
        .num_columns(columns * 2)
        .spacing([theme.spacing.xs, theme.spacing.xs])
        .show(ui, |ui| {
            for (i, value) in levels.iter_mut().enumerate() {
                let speed = (range.end() - range.start()) / 1000.0;
                ui.add_sized(
                    [58.0, ui.spacing().interact_size.y],
                    egui::DragValue::new(value)
                        .range(range.clone())
                        .speed(speed),
                )
                .accessible_name(l10n::format_text(
                    ui.ctx(),
                    "{title} {index}",
                    &[
                        ("title", l10n::text(ui.ctx(), title)),
                        ("index", (i + 1).to_string()),
                    ],
                ));
                if Button::icon(&crate::icons::icons::TRASH)
                    .ghost()
                    .small()
                    .tooltip("Remove division")
                    .show(ui)
                    .clicked()
                {
                    remove = Some(i);
                }
                if (i + 1) % columns == 0 {
                    ui.end_row();
                }
            }
        });
    if let Some(i) = remove {
        levels.remove(i);
    }
    if Button::new(format!("Add {}", title.to_lowercase()))
        .secondary()
        .small()
        .show(ui)
        .clicked()
    {
        levels.push(new);
    }
    ui.space_sm();
}

fn level_color(ui: &mut Ui, color: &mut [u8; 4], state: &mut ColorPickerState) -> bool {
    state.sel_color = egui::Color32::from_rgba_unmultiplied(color[0], color[1], color[2], color[3]);
    if let Some(picked) = ColorPicker::new(state, ui.scope_id().with("level_color"))
        .well_size(ui.spacing().interact_size.y.max(24.0))
        .show(ui)
        .inner
    {
        *color = picked.to_srgba_unmultiplied();
        return true;
    }
    false
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn inputs_cover_all_configured_fib_tools_and_gann_fan() {
        for tool in [
            Tool::FibonacciRetracement,
            Tool::FibonacciExtension,
            Tool::FibonacciChannel,
            Tool::FibonacciWedge,
            Tool::FibonacciTimeZones,
            Tool::FibonacciSpeedFan,
            Tool::TrendBasedFibTime,
            Tool::FibonacciCircles,
            Tool::FibonacciSpeedResistanceArcs,
            Tool::GannFan,
            Tool::Pitchfan,
        ] {
            assert!(InputsTab::available(tool), "{tool:?}");
        }
        assert!(!InputsTab::available(Tool::FibonacciSpiral));
    }
}
