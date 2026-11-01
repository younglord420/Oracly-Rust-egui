//! Visibility timeframe presets for the drawing properties panel.

use crate::localization as l10n;
use crate::theme::ThemeExt;
use crate::ui::dialogs::drawing_properties::actions::DrawingProps;
use crate::ui_kit::buttons::combo_chevron;
use crate::ui_kit::form_control_width_for;
use egui::{Align, Layout, Ui};

/// Available timeframes, named as `Timeframe::as_str` names them for the chart's interval label.
const TIMEFRAMES: &[&str] = &[
    "1min", "5min", "15min", "30min", "1h", "4h", "1D", "1W", "1M",
];

const INTRADAY: &[&str] = &["1min", "5min", "15min", "30min"];
const HOURLY_AND_UP: &[&str] = &["1h", "4h", "1D", "1W", "1M"];

/// Visibility section renderer.
pub struct VisibilityTab;

/// Preset mapped onto `visible_all_timeframes` / `visible_timeframes`.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum ShowOnPreset {
    All,
    Intraday,
    HourlyAndUp,
    Custom,
}

impl ShowOnPreset {
    fn label(self) -> &'static str {
        match self {
            Self::All => "All timeframes",
            Self::Intraday => "Intraday",
            Self::HourlyAndUp => "1H and up",
            Self::Custom => "Custom",
        }
    }

    fn from_props(props: &DrawingProps) -> Self {
        if props.visible_all_timeframes {
            return Self::All;
        }
        if set_matches(&props.visible_timeframes, INTRADAY) {
            return Self::Intraday;
        }
        if set_matches(&props.visible_timeframes, HOURLY_AND_UP) {
            return Self::HourlyAndUp;
        }
        Self::Custom
    }

    fn apply(self, props: &mut DrawingProps) {
        match self {
            Self::All => {
                props.visible_all_timeframes = true;
                props.visible_timeframes.clear();
            }
            Self::Intraday => {
                props.visible_all_timeframes = false;
                props.visible_timeframes = INTRADAY.iter().map(|s| (*s).to_owned()).collect();
            }
            Self::HourlyAndUp => {
                props.visible_all_timeframes = false;
                props.visible_timeframes = HOURLY_AND_UP.iter().map(|s| (*s).to_owned()).collect();
            }
            Self::Custom => {
                props.visible_all_timeframes = false;
                if props.visible_timeframes.is_empty() {
                    props.visible_timeframes = TIMEFRAMES.iter().map(|s| (*s).to_owned()).collect();
                }
            }
        }
    }
}

impl VisibilityTab {
    pub fn show(ui: &mut Ui, props: &mut DrawingProps) {
        let mut preset = ShowOnPreset::from_props(props);
        let theme = ui.active_theme();
        let combo_w = form_control_width_for(ui).min(160.0);

        ui.horizontal(|ui| {
            ui.spacing_mut().item_spacing.x = theme.spacing.sm;
            ui.label(
                egui::RichText::new(l10n::text(ui.ctx(), "Show on chart"))
                    .size(theme.text.body)
                    .color(theme.ui.text_secondary),
            );
            ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                let selected = l10n::text(ui.ctx(), preset.label());
                let response = egui::ComboBox::from_id_salt("drawing_props_show_on")
                    .icon(combo_chevron)
                    .selected_text(egui::RichText::new(selected).size(theme.text.body))
                    .width(combo_w)
                    .show_ui(ui, |ui| {
                        for choice in [
                            ShowOnPreset::All,
                            ShowOnPreset::Intraday,
                            ShowOnPreset::HourlyAndUp,
                            ShowOnPreset::Custom,
                        ] {
                            let label = l10n::text(ui.ctx(), choice.label());
                            if ui.selectable_label(preset == choice, label).clicked()
                                && preset != choice
                            {
                                choice.apply(props);
                                preset = choice;
                            }
                        }
                    })
                    .response;
                response.accessible_name(l10n::text(ui.ctx(), "Show on chart"));
            });
        });

        if matches!(preset, ShowOnPreset::Custom) {
            ui.add_space(theme.spacing.sm);
            ui.horizontal_wrapped(|ui| {
                ui.spacing_mut().item_spacing.x = theme.spacing.md;
                ui.spacing_mut().item_spacing.y = theme.spacing.xs;
                for tf in TIMEFRAMES {
                    let mut is_visible = props.visible_timeframes.contains(&tf.to_string());
                    if ui
                        .checkbox(&mut is_visible, l10n::text(ui.ctx(), *tf))
                        .changed()
                    {
                        if is_visible {
                            if !props.visible_timeframes.contains(&tf.to_string()) {
                                props.visible_timeframes.push(tf.to_string());
                            }
                        } else {
                            props.visible_timeframes.retain(|t| t != *tf);
                        }
                        props.visible_all_timeframes = false;
                    }
                }
            });
        }
    }
}

fn set_matches(current: &[String], expected: &[&str]) -> bool {
    if current.len() != expected.len() {
        return false;
    }
    expected.iter().all(|tf| current.iter().any(|c| c == *tf))
}
