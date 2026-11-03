//! Inputs that correspond to the selected tool's saved drawing configuration.

use crate::drawings::{
    Drawing, DrawingToolType as Tool, FibPreset, FibonacciLevel, GannFanDirection, GannLevel,
    PitchforkConfig, PitchforkLevel, RegressionSource, TextHorizontalAlign, TextVerticalAlign,
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
                | Tool::GartleyPattern
                | Tool::BatPattern
                | Tool::ButterflyPattern
                | Tool::CrabPattern
                | Tool::SharkPattern
                | Tool::CypherPattern
                | Tool::HeadAndShoulders
                | Tool::ThreeDrivesPattern
                | Tool::ElliottImpulse
                | Tool::ElliottCorrection
                | Tool::ElliottTriangle
                | Tool::ElliottDoubleCombo
                | Tool::ElliottTripleCombo
                | Tool::CyclicLines
                | Tool::FairValueGap
                | Tool::OrderBlock
                | Tool::Liquidity
                | Tool::DedekindTessellation
                | Tool::Sonic
                | Tool::Supersonic
                | Tool::GoldenSonic
                | Tool::GoldenSupersonic
                | Tool::Measure
                | Tool::DateRange
                | Tool::DateAndPriceRange
                | Tool::AnchoredVWAP
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
            Tool::GannBox | Tool::GannSquare | Tool::GannFixed => gann_grid(ui, drawing, pickers),
            Tool::GartleyPattern
            | Tool::BatPattern
            | Tool::ButterflyPattern
            | Tool::CrabPattern
            | Tool::SharkPattern
            | Tool::CypherPattern => harmonic_rules_table(ui, drawing.tool_type),
            Tool::HeadAndShoulders
            | Tool::ThreeDrivesPattern
            | Tool::ElliottImpulse
            | Tool::ElliottCorrection
            | Tool::ElliottTriangle
            | Tool::ElliottDoubleCombo
            | Tool::ElliottTripleCombo
            | Tool::CyclicLines => pattern_inputs(ui, drawing),
            Tool::AnchoredVWAP => {
                let mut config = drawing.vwap.unwrap_or_default();
                let original = config;

                choice_row(
                    ui,
                    "Band multiplier",
                    &mut config.multiplier,
                    &[0.5, 1.0, 1.5, 2.0, 2.5, 3.0],
                    |m| m.to_string(),
                );
                pattern_toggle(ui, &mut config.show_outer, "Outer bands");
                pickers.resize_with(3, ColorPickerState::default);
                let faint = {
                    let [r, g, b, _] = drawing.color;
                    [r, g, b, 150]
                };
                for (i, (label, slot)) in [
                    ("Upper band", &mut config.upper_color),
                    ("Lower band", &mut config.lower_color),
                ]
                .into_iter()
                .enumerate()
                {
                    let mut color = slot.unwrap_or(faint);
                    color_row(ui, label, &mut color, &mut pickers[i]);
                    if color != slot.unwrap_or(faint) {
                        *slot = Some(color);
                    }
                }
                let mut filled = config.band_fill.is_some();
                pattern_toggle(ui, &mut filled, "Band fill");
                if filled {
                    let [r, g, b, _] = drawing.color;
                    let mut fill = config.band_fill.unwrap_or([r, g, b, 26]);
                    color_row(ui, "Band fill colour", &mut fill, &mut pickers[2]);
                    config.band_fill = Some(fill);
                } else {
                    config.band_fill = None;
                }
                if config != original {
                    drawing.vwap = Some(config);
                }
            }
            Tool::Measure | Tool::DateRange | Tool::DateAndPriceRange => {
                let mut config = drawing.measure.unwrap_or_default();
                let original = config;
                if drawing.tool_type != Tool::DateRange {
                    pattern_toggle(ui, &mut config.show_price, "Show price change");
                }
                pattern_toggle(ui, &mut config.show_time, "Show bars & time");
                pattern_toggle(ui, &mut config.show_volume, "Show volume");
                if config != original {
                    drawing.measure = Some(config);
                }
            }
            Tool::FairValueGap | Tool::OrderBlock | Tool::Liquidity => {
                price_action_inputs(ui, drawing, pickers);
            }
            Tool::DedekindTessellation
            | Tool::Sonic
            | Tool::Supersonic
            | Tool::GoldenSonic
            | Tool::GoldenSupersonic => geometry_inputs(ui, drawing, pickers),
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
            Tool::LongPos | Tool::ShortPos => position_inputs(ui, drawing, pickers),
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
                    let label = form_label(ui, "VAH / VAL lines");
                    ui.label_inputs_by(label.id, |ui| {
                        ui.checkbox(&mut config.show_value_area_lines, "");
                    });
                    ui.end_row();
                    let mut custom = config.value_area_colors.is_some();
                    let label = form_label(ui, "Value area colours");
                    ui.label_inputs_by(label.id, |ui| {
                        ui.checkbox(&mut custom, "");
                    });
                    ui.end_row();
                    if custom {
                        let mut colors = config
                            .value_area_colors
                            .unwrap_or([config.up_color, config.down_color]);
                        pickers.resize_with(4, ColorPickerState::default);
                        for (i, name) in
                            ["Value area up", "Value area down"].into_iter().enumerate()
                        {
                            let label = form_label(ui, name);
                            ui.label_inputs_by(label.id, |ui| {
                                level_color(ui, &mut colors[i], &mut pickers[2 + i]);
                            });
                            ui.end_row();
                        }
                        config.value_area_colors = Some(colors);
                    } else {
                        config.value_area_colors = None;
                    }
                });
                if let Some(mut updated) = VolumeProfileConfig::new(rows, percent / 100.0)
                    .and_then(|c| c.with_row_size(fixed.then_some(height)))
                {
                    updated.display = config.display;
                    updated.up_color = config.up_color;
                    updated.down_color = config.down_color;
                    updated.value_area_colors = config.value_area_colors;
                    updated.show_value_area_lines = config.show_value_area_lines;
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
    text: RatioText,
}

/// How a level's ratio is shown and typed.
#[derive(Clone, Copy)]
enum RatioText {
    Number,
    /// Price over time, as `8/1` or `1/3`.
    PricePerTime,
    /// Time by price, as `3x1` or `1x2`.
    TimeByPrice,
}

fn ratio_value(ui: &mut Ui, width: f32, fields: &mut LevelRow<'_>, name: String) -> egui::Response {
    let height = super::style::control_row_height(ui);
    let mut drag = egui::DragValue::new(&mut *fields.value)
        .range(fields.range.clone())
        .speed(0.001)
        .max_decimals(6);
    let label: Option<fn(f32) -> String> = match fields.text {
        RatioText::Number => None,
        RatioText::PricePerTime => Some(crate::drawings::gann_fan_label),
        RatioText::TimeByPrice => Some(crate::drawings::gann_square_fan_label),
    };
    if let Some(label) = label {
        drag = drag
            .custom_formatter(move |value, _| label(value as f32))
            .custom_parser(|text| crate::drawings::parse_gann_ratio(text).map(f64::from));
    }
    let response = ui.add_sized([width, height], drag).accessible_name(name);
    if response.changed()
        && let Some(label) = fields.legacy_label.as_deref_mut()
    {
        label.clear();
    }
    response
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
                            let mut fields = row(level);
                            let name = |ui: &Ui, property| {
                                l10n::format_text(ui.ctx(), property, &[("0", (i + 1).to_string())])
                            };
                            ui.checkbox(&mut *fields.visible, "")
                                .accessible_name(name(ui, "Level {0} visible"));
                            let ratio_name = name(ui, "Level {0} ratio");
                            ratio_value(ui, ratio_w, &mut fields, ratio_name);
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
                            let mut fields = row(level);
                            let name = |ui: &Ui, property| {
                                l10n::format_text(ui.ctx(), property, &[("0", (i + 1).to_string())])
                            };
                            ui.checkbox(&mut *fields.visible, "")
                                .accessible_name(name(ui, "Level {0} visible"));
                            let ratio_name = name(ui, "Level {0} ratio");
                            ratio_value(ui, ratio_w, &mut fields, ratio_name);
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
                        let mut fields = row(level);
                        let name = |ui: &Ui, property| {
                            l10n::format_text(ui.ctx(), property, &[("0", (i + 1).to_string())])
                        };
                        ui.checkbox(&mut *fields.visible, "")
                            .accessible_name(name(ui, "Level {0} visible"));
                        let ratio_name = name(ui, "Level {0} ratio");
                        ratio_value(ui, ratio_w, &mut fields, ratio_name);
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
            text: RatioText::Number,
        },
    );

    background(
        ui,
        &mut config.show_background,
        &mut config.background_opacity,
    );

    if drawing.tool_type == Tool::FibonacciRetracement {
        ui.horizontal(|ui| {
            ui.set_min_height(control_h);
            ui.checkbox(&mut config.reverse, l10n::text(ui.ctx(), "Reverse"));
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

/// Tools whose level labels the Label section formats in place of free text.
pub(super) fn has_fibonacci_labels(tool: Tool) -> bool {
    matches!(
        tool,
        Tool::FibonacciRetracement
            | Tool::FibonacciExtension
            | Tool::FibonacciChannel
            | Tool::FibonacciWedge
            | Tool::FibonacciTimeZones
            | Tool::FibonacciSpeedFan
            | Tool::TrendBasedFibTime
            | Tool::FibonacciCircles
            | Tool::FibonacciSpeedResistanceArcs
    )
}

/// What the level labels of a Fibonacci tool show, and where and how large they are.
pub(super) fn fibonacci_labels(ui: &mut Ui, drawing: &mut Drawing) {
    let mut config = drawing.fibonacci_config().into_owned();
    let original = config.clone();
    let theme = ui.active_theme();
    let control_h = super::style::control_row_height(ui);
    ui.spacing_mut().interact_size.y = control_h;

    if matches!(
        drawing.tool_type,
        Tool::FibonacciRetracement | Tool::FibonacciExtension | Tool::FibonacciChannel
    ) {
        ui.horizontal(|ui| {
            ui.set_min_height(control_h);
            let mut prices = config
                .show_prices
                .unwrap_or(drawing.tool_type != Tool::FibonacciChannel);
            if ui
                .checkbox(&mut prices, l10n::text(ui.ctx(), "Prices"))
                .changed()
            {
                config.show_prices = Some(prices);
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
            text: RatioText::Number,
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
            text: RatioText::Number,
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

/// A table of Gann levels: visibility, ratio and colour of each, with its own picker states.
fn gann_levels(
    ui: &mut Ui,
    id: &str,
    levels: &mut Vec<GannLevel>,
    pickers: &mut Vec<ColorPickerState>,
    range: std::ops::RangeInclusive<f32>,
    text: RatioText,
    new_ratio: f32,
) {
    let new_level = GannLevel::new(
        new_ratio,
        crate::drawings::domain::fib_ratio_color(new_ratio, levels.len()),
    );
    ui.push_id(id, |ui| {
        level_grid(ui, levels, pickers, new_level, |level| LevelRow {
            visible: &mut level.enabled,
            value: &mut level.ratio,
            legacy_label: None,
            color: &mut level.color,
            line_style: None,
            width: None,
            range: range.clone(),
            text,
        });
    });
}

/// A section's show checkbox in the place of its title.
fn section_toggle(ui: &mut Ui, show: &mut bool, title: &str) {
    ui.horizontal(|ui| {
        ui.set_min_height(super::style::control_row_height(ui));
        ui.checkbox(
            show,
            egui::RichText::new(l10n::text(ui.ctx(), title)).strong(),
        );
    });
}

fn gann_fan(ui: &mut Ui, drawing: &mut Drawing, pickers: &mut Vec<ColorPickerState>) {
    let mut config = drawing.gann_config();
    let original = config.clone();
    ui.strong(l10n::text(ui.ctx(), "Fan angles"));
    gann_levels(
        ui,
        "gann_fan_angles",
        &mut config.fan_angles,
        pickers,
        0.001..=1000.0,
        RatioText::PricePerTime,
        1.0,
    );
    if config != original {
        drawing.gann = Some(config);
    }
}

/// Gann box, square and fixed square: the grid levels, and the fan and arcs of a square.
fn gann_grid(ui: &mut Ui, drawing: &mut Drawing, pickers: &mut Vec<ColorPickerState>) {
    let mut config = drawing.gann_config();
    let original = config.clone();
    if drawing.tool_type != Tool::GannSquare {
        ui.strong(l10n::text(ui.ctx(), "Levels"));
        gann_levels(
            ui,
            "gann_grid_levels",
            &mut config.levels,
            pickers,
            0.0..=1.0,
            RatioText::Number,
            0.5,
        );
    } else {
        // Each table keeps its own picker states, in order after those of the tables above it.
        let grid = config.levels.len() + 1;
        let fan = config.fan_lines.len() + 1;
        pickers.resize_with(
            grid + fan + config.arcs.len() + 1,
            ColorPickerState::default,
        );
        let mut arc_pickers = pickers.split_off(grid + fan);
        let mut fan_pickers = pickers.split_off(grid);

        section_toggle(ui, &mut config.show_grid, "Grid");
        if config.show_grid {
            gann_levels(
                ui,
                "gann_grid_levels",
                &mut config.levels,
                pickers,
                0.0..=1.0,
                RatioText::Number,
                0.5,
            );
        }

        let control_h = super::style::control_row_height(ui);
        let col_w = super::style::control_column_width(ui);
        ui.horizontal(|ui| {
            ui.set_min_height(control_h);
            ui.checkbox(
                &mut config.show_fan,
                egui::RichText::new(l10n::text(ui.ctx(), "Fan")).strong(),
            );
            if config.show_fan {
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
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
                });
            }
        });
        if config.show_fan {
            gann_levels(
                ui,
                "gann_square_fan",
                &mut config.fan_lines,
                &mut fan_pickers,
                0.001..=1000.0,
                RatioText::TimeByPrice,
                1.0,
            );
        }

        section_toggle(ui, &mut config.show_arcs, "Arcs");
        if config.show_arcs {
            gann_levels(
                ui,
                "gann_square_arcs",
                &mut config.arcs,
                &mut arc_pickers,
                0.01..=1.41,
                RatioText::Number,
                0.5,
            );
        }
        pickers.append(&mut fan_pickers);
        pickers.append(&mut arc_pickers);
    }
    if config != original {
        drawing.gann = Some(config);
    }
}

/// The ratios a named harmonic pattern is judged by, for reference.
fn harmonic_rules_table(ui: &mut Ui, tool: Tool) {
    let Some(rules) = crate::drawings::harmonic_rules(tool) else {
        return;
    };
    let theme = ui.active_theme();
    ui.strong(l10n::text(ui.ctx(), "Ideal ratios"));
    egui::Grid::new(ui.scope_id().with("harmonic_rules"))
        .num_columns(2)
        .spacing([theme.spacing.lg, theme.spacing.xs])
        .show(ui, |ui| {
            for rule in rules {
                ui.label(
                    egui::RichText::new(rule.name)
                        .size(theme.text.body)
                        .color(theme.ui.text_secondary),
                );
                ui.label(
                    egui::RichText::new(format!("{} – {}", rule.band.min, rule.band.max))
                        .size(theme.text.body),
                );
                ui.end_row();
            }
        });
}

/// One option of a pattern tool as a checkbox row.
fn pattern_toggle(ui: &mut Ui, value: &mut bool, label: &str) {
    ui.horizontal(|ui| {
        ui.set_min_height(super::style::control_row_height(ui));
        ui.checkbox(value, l10n::text(ui.ctx(), label));
    });
}

fn is_elliott(tool: Tool) -> bool {
    matches!(
        tool,
        Tool::ElliottImpulse
            | Tool::ElliottCorrection
            | Tool::ElliottTriangle
            | Tool::ElliottDoubleCombo
            | Tool::ElliottTripleCombo
    )
}

/// What a pattern, Elliott wave or cycle tool draws beyond its legs.
fn pattern_inputs(ui: &mut Ui, drawing: &mut Drawing) {
    let mut config = drawing.pattern_config();
    let original = config;
    let tool = drawing.tool_type;
    if is_elliott(tool) {
        let control_h = super::style::control_row_height(ui);
        let theme = ui.active_theme();
        ui.horizontal(|ui| {
            ui.set_min_height(control_h);
            ui.label(
                egui::RichText::new(l10n::text(ui.ctx(), "Degree"))
                    .size(theme.text.body)
                    .color(theme.ui.text_secondary),
            );
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                let ctx = ui.ctx().clone();
                super::style::text_combo_popup(
                    ui,
                    "elliott_degree",
                    "Degree",
                    &mut config.degree,
                    &crate::drawings::WaveDegree::ALL,
                    |degree| l10n::text(&ctx, degree.name()),
                    super::style::control_column_width(ui)
                        .min(ui.available_width())
                        .max(120.0),
                    control_h,
                );
            });
        });
    }
    match tool {
        Tool::ElliottImpulse => pattern_toggle(ui, &mut config.show_target, "Wave 3 target"),
        Tool::HeadAndShoulders => {
            pattern_toggle(ui, &mut config.show_neckline, "Neckline");
            pattern_toggle(ui, &mut config.show_target, "Target");
        }
        Tool::ThreeDrivesPattern => pattern_toggle(ui, &mut config.show_zone, "Reversal zone"),
        Tool::CyclicLines => {
            pattern_toggle(ui, &mut config.show_cycle_numbers, "Cycle numbers");
        }
        _ => {}
    }
    if config != original {
        drawing.pattern = Some(config);
    }
}

/// A labelled row choosing one of `choices` from a dropdown.
fn choice_row<T: Copy + PartialEq>(
    ui: &mut Ui,
    label: &str,
    current: &mut T,
    choices: &[T],
    name_of: impl Fn(T) -> String,
) {
    let control_h = super::style::control_row_height(ui);
    let theme = ui.active_theme();
    ui.horizontal(|ui| {
        ui.set_min_height(control_h);
        ui.label(
            egui::RichText::new(l10n::text(ui.ctx(), label))
                .size(theme.text.body)
                .color(theme.ui.text_secondary),
        );
        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
            super::style::text_combo_popup(
                ui,
                label,
                label,
                current,
                choices,
                name_of,
                super::style::control_column_width(ui)
                    .min(ui.available_width())
                    .max(120.0),
                control_h,
            );
        });
    });
}

/// A labelled row with a colour well.
fn color_row(ui: &mut Ui, label: &str, color: &mut [u8; 4], picker: &mut ColorPickerState) {
    let theme = ui.active_theme();
    ui.horizontal(|ui| {
        ui.set_min_height(super::style::control_row_height(ui));
        ui.label(
            egui::RichText::new(l10n::text(ui.ctx(), label))
                .size(theme.text.body)
                .color(theme.ui.text_secondary),
        );
        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
            ui.push_id(label, |ui| level_color(ui, color, picker));
        });
    });
}

/// A labelled number row; returns whether the value changed.
fn value_row(ui: &mut Ui, label: &str, value: &mut f64, speed: f64) -> bool {
    let theme = ui.active_theme();
    let mut changed = false;
    ui.horizontal(|ui| {
        ui.set_min_height(super::style::control_row_height(ui));
        let label = ui.label(
            egui::RichText::new(l10n::text(ui.ctx(), label))
                .size(theme.text.body)
                .color(theme.ui.text_secondary),
        );
        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
            changed = ui
                .add_sized(
                    [
                        super::style::control_column_width(ui).min(ui.available_width()),
                        super::style::control_row_height(ui),
                    ],
                    egui::DragValue::new(value).speed(speed).max_decimals(8),
                )
                .labelled_by(label.id)
                .changed();
        });
    });
    changed
}

/// The direction, sizing, levels, labels and zone colours of a long or short position.
fn position_inputs(ui: &mut Ui, drawing: &mut Drawing, pickers: &mut Vec<ColorPickerState>) {
    let ctx = ui.ctx().clone();
    let mut long = drawing.position_is_long();
    choice_row(ui, "Direction", &mut long, &[true, false], |long| {
        l10n::text(&ctx, if long { "Long" } else { "Short" })
    });
    if long != drawing.position_is_long() {
        drawing.set_position_long(long);
    }

    let (mut balance, mut percent) = drawing.position_account();
    let (_, mut size) = drawing.position_loss_and_size();
    let mut account = value_row(ui, "Account balance", &mut balance, 10.0);
    account |= value_row(ui, "Risk %", &mut percent, 0.05);
    if value_row(ui, "Position size", &mut size, 0.01)
        && let Some((entry, _, stop)) = drawing.position_prices()
        && (entry - stop).abs() > 1e-12
        && balance > 0.0
    {
        percent = size * (entry - stop).abs() / balance * 100.0;
        account = true;
    }
    if account {
        drawing.set_position_account(balance.max(0.0), percent.clamp(0.0, 100.0));
    }

    if drawing.chart_points.len() >= 3 {
        let points_id = ui.make_persistent_id("position_levels_in_points");
        let mut points = ui.data(|d| d.get_temp::<bool>(points_id).unwrap_or(false));
        choice_row(ui, "Levels in", &mut points, &[false, true], |points| {
            l10n::text(&ctx, if points { "Points" } else { "Price" })
        });
        ui.data_mut(|d| d.insert_temp(points_id, points));
        let entry = drawing.chart_points[0].price;
        let speed = (entry.abs() * 0.0005).max(1e-6);
        let mut price = entry;
        if value_row(ui, "Entry price", &mut price, speed) {
            drawing.chart_points[0].price = price;
            drawing.keep_position_sides(0);
        }
        for (level, label) in [(1, "Target"), (2, "Stop")] {
            let entry = drawing.chart_points[0].price;
            let current = drawing.chart_points[level].price;
            let mut shown = if points {
                (current - entry).abs()
            } else {
                current
            };
            let label = format!("{label} {}", if points { "points" } else { "price" });
            if value_row(ui, &label, &mut shown, speed) {
                drawing.chart_points[level].price = if points {
                    let side = (current - entry).signum();
                    entry + if side == 0.0 { 1.0 } else { side } * shown.abs()
                } else {
                    shown
                };
                drawing.keep_position_sides(level);
            }
        }
    }

    let mut display = drawing.position_display();
    let original = display;
    pattern_toggle(ui, &mut display.show_text, "Show text");
    if display.show_text {
        pattern_toggle(ui, &mut display.show_header, "Show direction & ratio");
        pattern_toggle(ui, &mut display.show_loss_size, "Show loss & size");
        pattern_toggle(ui, &mut display.show_target_label, "Show target label");
        pattern_toggle(ui, &mut display.show_stop_label, "Show stop label");
        pattern_toggle(ui, &mut display.show_prices, "Show level prices");
    }
    pickers.resize_with(2, ColorPickerState::default);
    let (profit, loss) = pickers.split_at_mut(1);
    color_row(ui, "Profit zone", &mut display.profit_color, &mut profit[0]);
    color_row(ui, "Loss zone", &mut display.loss_color, &mut loss[0]);
    if display != original {
        drawing.position_display = Some(display);
    }
}

/// How far a fair value gap, order block or liquidity level runs, and how a zone looks.
fn price_action_inputs(ui: &mut Ui, drawing: &mut Drawing, pickers: &mut Vec<ColorPickerState>) {
    let tool = drawing.tool_type;
    let mut config = drawing.price_action_config();
    let original = config;
    let ctx = ui.ctx().clone();
    choice_row(
        ui,
        "Extend right",
        &mut config.extend,
        crate::drawings::ZoneEnd::options(tool),
        |end| l10n::text(&ctx, end.name()),
    );
    if tool == Tool::Liquidity {
        pattern_toggle(ui, &mut config.mark_sweep, "Mark sweep");
    } else {
        if tool == Tool::OrderBlock {
            choice_row(ui, "Zone", &mut config.full_range, &[false, true], |full| {
                l10n::text(&ctx, if full { "Full range" } else { "Body" })
            });
        }
        pickers.resize_with(2, ColorPickerState::default);
        let (up, down) = pickers.split_at_mut(1);
        color_row(ui, "Bullish", &mut config.up_color, &mut up[0]);
        color_row(ui, "Bearish", &mut config.down_color, &mut down[0]);
        pattern_toggle(ui, &mut config.midline, "Midline");
        pattern_toggle(ui, &mut config.border, "Border");
        if config.extend != crate::drawings::ZoneEnd::Always {
            pattern_toggle(ui, &mut config.fade, "Fade at end");
        }
        match (tool, config.extend) {
            (Tool::FairValueGap, crate::drawings::ZoneEnd::Filled) => {
                pattern_toggle(ui, &mut config.flip, "Flip on close");
            }
            (Tool::OrderBlock, crate::drawings::ZoneEnd::Broken) => {
                pattern_toggle(ui, &mut config.flip, "Breaker");
            }
            _ => {}
        }
    }
    if config != original {
        drawing.price_action = Some(config);
    }
}

/// Density of a Dedekind tessellation, or the circles and Mach number of a Mach figure.
fn geometry_inputs(ui: &mut Ui, drawing: &mut Drawing, pickers: &mut Vec<ColorPickerState>) {
    let tool = drawing.tool_type;
    let mut config = drawing.geometry_config();
    let original = config.clone();
    if tool == Tool::DedekindTessellation {
        choice_row(
            ui,
            "Max curvature",
            &mut config.max_curvature,
            &crate::drawings::DEDEKIND_CURVATURES,
            |n| n.to_string(),
        );
    } else {
        if matches!(tool, Tool::Supersonic | Tool::GoldenSupersonic) {
            choice_row(
                ui,
                "Mach number",
                &mut config.mach,
                &[1.5, 2.0, 2.5, 3.0, 4.0, 5.0],
                |m| m.to_string(),
            );
        }
        if matches!(tool, Tool::Sonic | Tool::Supersonic) {
            let mut waves = config.waves();
            let counts: Vec<usize> = (3..=config.levels.len().max(3)).collect();
            choice_row(ui, "Waves", &mut waves, &counts, |n| n.to_string());
            if waves != config.waves() {
                config.set_waves(waves);
            }
        }
        pattern_toggle(ui, &mut config.show_ratios, "Ratios");
        ui.strong(l10n::text(ui.ctx(), "Circles"));
        let next = config.levels.iter().map(|l| l.ratio).fold(0.0, f32::max) + 1.0;
        gann_levels(
            ui,
            "mach_circles",
            &mut config.levels,
            pickers,
            0.01..=100.0,
            RatioText::Number,
            next,
        );
    }
    if config != original {
        drawing.geometry = Some(config);
    }
}

/// Tools whose generated labels the Label section can hide.
pub(super) fn has_pattern_labels(tool: Tool) -> bool {
    is_elliott(tool)
        || matches!(
            tool,
            Tool::XABCDPattern
                | Tool::CypherPattern
                | Tool::GartleyPattern
                | Tool::BatPattern
                | Tool::ButterflyPattern
                | Tool::CrabPattern
                | Tool::SharkPattern
                | Tool::HeadAndShoulders
                | Tool::ABCDPattern
                | Tool::TrianglePattern
                | Tool::ThreeDrivesPattern
        )
}

/// Which generated labels a pattern or Elliott wave shows.
pub(super) fn pattern_labels(ui: &mut Ui, drawing: &mut Drawing) {
    let mut config = drawing.pattern_config();
    let original = config;
    let tool = drawing.tool_type;
    let theme = ui.active_theme();
    ui.horizontal_wrapped(|ui| {
        ui.spacing_mut().item_spacing.x = theme.spacing.lg;
        let anchors = if is_elliott(tool) {
            "Wave labels"
        } else {
            "Point labels"
        };
        ui.checkbox(&mut config.show_labels, l10n::text(ui.ctx(), anchors));
        if matches!(
            tool,
            Tool::XABCDPattern
                | Tool::CypherPattern
                | Tool::GartleyPattern
                | Tool::BatPattern
                | Tool::ButterflyPattern
                | Tool::CrabPattern
                | Tool::SharkPattern
                | Tool::ABCDPattern
                | Tool::ThreeDrivesPattern
        ) {
            ui.checkbox(&mut config.show_ratios, l10n::text(ui.ctx(), "Ratios"));
        }
        if crate::drawings::harmonic_rules(tool).is_some() {
            ui.checkbox(&mut config.show_validity, l10n::text(ui.ctx(), "Validity"));
        }
    });
    if config != original {
        drawing.pattern = Some(config);
    }
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

    /// Pattern tools offer only the options their renderer reads: Elliott waves their degree,
    /// named harmonics their ideal ratios and verdict, and label toggles in the Label section.
    #[test]
    fn pattern_settings_offer_what_each_tool_draws() {
        let texts = |tool: Tool, show: &dyn Fn(&mut Ui, &mut Drawing)| {
            let mut drawing = Drawing::new(0, tool);
            let output = egui::Context::default()
                .run_ui(egui::RawInput::default(), |ui| show(ui, &mut drawing));
            let found: Vec<String> = output
                .shapes
                .iter()
                .filter_map(|s| match &s.shape {
                    egui::Shape::Text(text) => Some(text.galley.job.text.clone()),
                    _ => None,
                })
                .collect();
            output.drop_without_applying_deltas();
            found
        };
        let inputs = |tool| texts(tool, &|ui, d| InputsTab::show(ui, d, &mut Vec::new()));
        let labels = |tool| texts(tool, &|ui, d| pattern_labels(ui, d));
        let has = |found: &[String], text: &str| found.iter().any(|t| t == text);

        let impulse = inputs(Tool::ElliottImpulse);
        assert!(
            has(&impulse, "Degree") && has(&impulse, "Minor") && has(&impulse, "Wave 3 target")
        );
        assert!(!has(&inputs(Tool::ElliottCorrection), "Wave 3 target"));
        let gartley = inputs(Tool::GartleyPattern);
        assert!(
            has(&gartley, "AB/XA") && has(&gartley, "0.55 – 0.68"),
            "{gartley:?}"
        );
        let shoulders = inputs(Tool::HeadAndShoulders);
        assert!(has(&shoulders, "Neckline") && has(&shoulders, "Target"));
        assert!(has(&inputs(Tool::CyclicLines), "Cycle numbers"));
        assert!(!InputsTab::available(Tool::XABCDPattern));

        let harmonic = labels(Tool::BatPattern);
        for label in ["Point labels", "Ratios", "Validity"] {
            assert!(has(&harmonic, label), "{label}: {harmonic:?}");
        }
        let xabcd = labels(Tool::XABCDPattern);
        assert!(has(&xabcd, "Ratios") && !has(&xabcd, "Validity"));
        let wave = labels(Tool::ElliottTriangle);
        assert!(has(&wave, "Wave labels") && !has(&wave, "Ratios"));
        assert!(!has_pattern_labels(Tool::CyclicLines));
    }

    /// Label settings of a Fibonacci tool live in the Label section, not among its Inputs.
    #[test]
    fn fibonacci_label_settings_leave_the_inputs() {
        let texts = |show: &dyn Fn(&mut Ui, &mut Drawing)| {
            let mut drawing = Drawing::new(0, Tool::FibonacciRetracement);
            let output = egui::Context::default()
                .run_ui(egui::RawInput::default(), |ui| show(ui, &mut drawing));
            let found: Vec<String> = output
                .shapes
                .iter()
                .filter_map(|s| match &s.shape {
                    egui::Shape::Text(text) => Some(text.galley.job.text.clone()),
                    _ => None,
                })
                .collect();
            output.drop_without_applying_deltas();
            found
        };
        let inputs = texts(&|ui, d| InputsTab::show(ui, d, &mut Vec::new()));
        let labels = texts(&|ui, d| fibonacci_labels(ui, d));
        for label in ["Prices", "Levels", "Right", "Middle"] {
            assert!(!inputs.contains(&label.to_owned()), "{label} in Inputs");
            assert!(
                labels.contains(&label.to_owned()),
                "{label} missing: {labels:?}"
            );
        }
        assert!(inputs.contains(&"Reverse".to_owned()));
    }
}
