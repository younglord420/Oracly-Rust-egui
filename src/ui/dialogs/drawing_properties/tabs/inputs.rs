//! Inputs that correspond to the selected tool's saved drawing configuration.

use crate::drawings::{
    Drawing, DrawingToolType as Tool, FibonacciLevel, GannConfig, GannFanDirection,
    PitchforkConfig, PitchforkLevel, PositionSize, RegressionSource, RegressionTrendConfig,
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
                | Tool::GannBox
                | Tool::GannSquare
                | Tool::GannFixed
                | Tool::GhostFeed
                | Tool::LongPos
                | Tool::ShortPos
                | Tool::FixedRangeVolumeProfile
                | Tool::AnchoredVolumeProfile
                | Tool::RegressionTrend
                | Tool::ParallelChannel
                | Tool::FlatTopBottom
                | Tool::DisjointChannel
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
                if square {
                    ui.checkbox(&mut config.show_grid, l10n::text(ui.ctx(), "Show grid"));
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
                    ui.checkbox(&mut config.show_fan, l10n::text(ui.ctx(), "Show fan"));
                    if config.show_fan {
                        let ctx = ui.ctx().clone();
                        ui.combo_select_width(
                            "gann_fan_direction",
                            &mut config.fan_direction,
                            [
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
                            form_control_width_for(ui),
                        );
                        fractions(ui, "Fan ratios", &mut config.fan_ratios, 0.01..=100.0, 1.0);
                    }
                    ui.checkbox(&mut config.show_arcs, l10n::text(ui.ctx(), "Show arcs"));
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
                pickers.resize_with(2, ColorPickerState::default);
                FormGrid::new("regression_inputs").show(ui, |ui| {
                    number(
                        ui,
                        "Upper deviation",
                        &mut config.upper_deviation,
                        f64::MIN_POSITIVE..=10.0,
                    );
                    number(
                        ui,
                        "Lower deviation",
                        &mut config.lower_deviation,
                        f64::MIN_POSITIVE..=10.0,
                    );
                    let label = form_label(ui, "Source");
                    let ctx = ui.ctx().clone();
                    let control_width = form_control_width_for(ui);
                    ui.label_inputs_by(label.id, |ui| {
                        ui.combo_select_width(
                            "regression_source",
                            &mut config.source,
                            RegressionSource::all(),
                            |v| l10n::text(&ctx, v.label()),
                            control_width,
                        );
                    });
                    ui.end_row();
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
                });
                ui.checkbox(
                    &mut config.use_upper,
                    l10n::text(ui.ctx(), "Use upper deviation"),
                );
                ui.checkbox(
                    &mut config.use_lower,
                    l10n::text(ui.ctx(), "Use lower deviation"),
                );
                ui.checkbox(
                    &mut config.show_pearson,
                    l10n::text(ui.ctx(), "Pearson's R"),
                );
                if config != original {
                    if let Some(valid) = RegressionTrendConfig::validated(
                        config.upper_deviation,
                        config.lower_deviation,
                        config.use_upper,
                        config.use_lower,
                        config.source,
                        config.show_pearson,
                        config.up_color,
                        config.down_color,
                    ) {
                        drawing.regression_trend = Some(valid);
                    }
                }
            }
            Tool::ParallelChannel => {
                ui.checkbox(
                    &mut drawing.channel_midline,
                    l10n::text(ui.ctx(), "Show middle line"),
                );
            }
            Tool::FlatTopBottom | Tool::DisjointChannel => {
                ui.checkbox(
                    &mut drawing.show_price_labels,
                    l10n::text(ui.ctx(), "Price labels"),
                );
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
    ui.horizontal(|ui| {
        ui.spacing_mut().item_spacing.x = theme.spacing.sm;
        ui.checkbox(show, l10n::text(ui.ctx(), "Background"));
        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
            let mut percent = f32::from(*opacity) * 100.0 / 255.0;
            if ui
                .add_enabled(
                    *show,
                    egui::DragValue::new(&mut percent)
                        .range(0.0..=100.0)
                        .max_decimals(1)
                        .suffix("%")
                        .prefix(l10n::text(ui.ctx(), "Opacity ")),
                )
                .changed()
            {
                *opacity = (percent * 255.0 / 100.0).round() as u8;
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
    pickers.resize_with(levels.len() + 1, ColorPickerState::default);
    ui.horizontal(|ui| {
        ui.spacing_mut().item_spacing.x = theme.spacing.sm;
        ui.label(
            egui::RichText::new(l10n::text(ui.ctx(), "One colour for all levels"))
                .size(theme.text.body)
                .color(theme.ui.text_secondary),
        );
        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
            let mut color = levels
                .first_mut()
                .map_or([128, 128, 128, 255], |l| *row(l).color);
            if level_color(ui, &mut color, &mut pickers[0]) {
                for level in levels.iter_mut() {
                    *row(level).color = color;
                }
            }
        });
    });

    let has_style = levels.first_mut().is_some_and(|l| row(l).line_style.is_some());
    let has_width = levels.first_mut().is_some_and(|l| row(l).width.is_some());
    let num_cols = 4 + usize::from(has_style) + usize::from(has_width);
    let compact = ui.available_width() < 260.0;
    let ratio_w = if has_style && compact { 44.0 } else { 64.0 };
    let style_w = if compact { 60.0 } else { 86.0 };
    let width_w = if compact { 40.0 } else { 52.0 };
    let grid_gap = if compact {
        theme.spacing.xs
    } else {
        theme.spacing.sm
    };
    let mut remove = None;

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
                            [ratio_w, ui.spacing().interact_size.y],
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
                            ui.spacing().interact_size.y.max(28.0),
                        );
                    }
                    if let Some((width, inherited)) = fields.width {
                        let mut value = width.unwrap_or(inherited);
                        if ui
                            .add_sized(
                                [width_w, ui.spacing().interact_size.y],
                                egui::DragValue::new(&mut value)
                                    .range(
                                        f32::MIN_POSITIVE
                                            ..=crate::drawings::validation::MAX_STROKE_WIDTH,
                                    )
                                    .speed(0.1)
                                    .suffix(" px"),
                            )
                            .accessible_name(name(ui, "Level {0} width"))
                            .changed()
                        {
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

    if let Some(i) = remove {
        levels.remove(i);
        pickers.remove(i + 1);
    }
    if Button::new("Add level").secondary().small().show(ui).clicked() {
        levels.push(new_level);
    }
}

fn fibonacci(ui: &mut Ui, drawing: &mut Drawing, pickers: &mut Vec<ColorPickerState>) {
    let mut config = drawing.fibonacci_config().into_owned();
    let original = config.clone();
    ui.strong(l10n::text(ui.ctx(), "Fibonacci levels"));
    level_grid(
        ui,
        &mut config.levels,
        pickers,
        FibonacciLevel::new(1.0, "", drawing.color),
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
    let theme = ui.active_theme();
    egui::Grid::new("drawing_props_fib_options")
        .num_columns(2)
        .spacing([theme.spacing.lg, theme.spacing.xs])
        .min_col_width((ui.available_width() - theme.spacing.lg) * 0.45)
        .show(ui, |ui| {
            let mut col = 0;
            let push_cell = |ui: &mut Ui, col: &mut usize, add: &mut dyn FnMut(&mut Ui)| {
                add(ui);
                *col += 1;
                if *col % 2 == 0 {
                    ui.end_row();
                }
            };
            if matches!(
                drawing.tool_type,
                Tool::FibonacciRetracement | Tool::FibonacciExtension | Tool::FibonacciChannel
            ) {
                let mut prices = config
                    .show_prices
                    .unwrap_or(drawing.tool_type != Tool::FibonacciChannel);
                push_cell(ui, &mut col, &mut |ui| {
                    if ui
                        .checkbox(&mut prices, l10n::text(ui.ctx(), "Prices"))
                        .changed()
                    {
                        config.show_prices = Some(prices);
                    }
                });
            }
            let mut levels = config
                .show_levels
                .unwrap_or(drawing.tool_type != Tool::FibonacciWedge);
            push_cell(ui, &mut col, &mut |ui| {
                if ui
                    .checkbox(&mut levels, l10n::text(ui.ctx(), "Levels"))
                    .changed()
                {
                    config.show_levels = Some(levels);
                }
            });
            push_cell(ui, &mut col, &mut |ui| {
                ui.checkbox(
                    &mut config.levels_as_percent,
                    l10n::text(ui.ctx(), "Percent"),
                );
            });
            push_cell(ui, &mut col, &mut |ui| {
                ui.checkbox(
                    &mut config.labels_on_left,
                    l10n::text(ui.ctx(), "Labels on left"),
                );
            });
            if matches!(
                drawing.tool_type,
                Tool::FibonacciRetracement | Tool::FibonacciExtension | Tool::FibonacciChannel
            ) {
                if drawing.tool_type == Tool::FibonacciRetracement {
                    push_cell(ui, &mut col, &mut |ui| {
                        ui.checkbox(&mut config.reverse, l10n::text(ui.ctx(), "Reverse"));
                    });
                }
                push_cell(ui, &mut col, &mut |ui| {
                    ui.checkbox(&mut config.extend_left, l10n::text(ui.ctx(), "Extend left"));
                });
                push_cell(ui, &mut col, &mut |ui| {
                    ui.checkbox(
                        &mut config.extend_right,
                        l10n::text(ui.ctx(), "Extend right"),
                    );
                });
            }
            if matches!(
                drawing.tool_type,
                Tool::FibonacciRetracement | Tool::FibonacciExtension
            ) {
                push_cell(ui, &mut col, &mut |ui| {
                    ui.checkbox(
                        &mut config.logarithmic,
                        l10n::text(ui.ctx(), "Logarithmic levels"),
                    );
                });
            }
        });
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
