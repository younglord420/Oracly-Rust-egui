//! Time: date ranges, go-to-date, the time zone, the session and dividend adjustment.

use super::core::Direction;
use super::layout::{Region, panel_frame};
use super::{Action, Terminal};
use chrono::{Datelike, Duration, Months, TimeZone, Utc};
use egui_charts::config::TimezoneMode;
use egui_charts::localization as l10n;
use egui_charts::model::Bar;
use egui_charts::services::UserSettings;
use egui_charts::ui::Control;
use egui_charts::ui::timeframe_toolbar::{
    DatePicker, DatePickerAction, DateRange, SessionType, TimeframeToolbar,
    TimeframeToolbarAction,
};
use egui_charts::widget::{SeriesId, VisibleTimeRange};

pub(super) struct State {
    toolbar: TimeframeToolbar,
    date_picker: DatePicker,
}

impl Default for State {
    fn default() -> Self {
        Self {
            toolbar: TimeframeToolbar::default(),
            date_picker: DatePicker::default(),
        }
    }
}

impl Terminal {
    pub(super) fn time_init(&mut self) {
        self.set_control_visible(Control::TimeZone, true);
        self.set_control_visible(Control::Session, true);
        self.set_control_visible(Control::DividendAdjustment, true);
        // Keep the toolbar's interval label in step with the active chart.
        self.time.toolbar.state.timeframe = self
            .chart()
            .timeframe()
            .unwrap_or(egui_charts::model::Timeframe::Day1);
        let timezone = match &self.chart().chart_options().time_scale.timezone {
            TimezoneMode::Utc => "UTC".to_owned(),
            TimezoneMode::Local => "Local".to_owned(),
            TimezoneMode::Timezone(zone) | TimezoneMode::Exchange(zone) => zone.to_string(),
        };
        self.time.toolbar.set_timezone(timezone);
        self.time.toolbar.state.session_type = if self.chart().extended_hours() {
            SessionType::ETH
        } else {
            SessionType::RTH
        };
        self.time.toolbar.state.adjust_for_dividends = self.studies.settings.adjust_for_dividends;
    }

    pub(super) fn time_poll(&mut self, _ctx: &egui::Context) {
        if let Some(tf) = self.chart().timeframe() {
            self.time.toolbar.state.timeframe = tf;
        }
    }

    pub(super) fn time_show(&mut self, ui: &mut egui::Ui, region: Region) {
        match region {
            Region::Bottom => self.show_bottom_toolbar(ui),
            Region::Windows => {
                self.show_date_picker(ui.ctx());
                self.show_time_menus(ui.ctx());
            }
            _ => {}
        }
    }

    pub(super) fn time_perform(&mut self, action: Action) -> Option<Action> {
        match action {
            Action::BottomBar(toolbar_action) => {
                self.handle_toolbar(toolbar_action);
                None
            }
            Action::DatePicker(picker_action) => {
                self.handle_date_picker(picker_action);
                None
            }
            other => Some(other),
        }
    }

    pub(super) fn time_settings(&mut self, settings: &mut UserSettings, direction: Direction) {
        match direction {
            Direction::Load => {
                if let Some(timezone) = settings.timezone.clone() {
                    self.apply_timezone(&timezone);
                }
                if let Some(extended) = settings.extended_hours {
                    self.time.toolbar.state.session_type = if extended {
                        SessionType::ETH
                    } else {
                        SessionType::RTH
                    };
                    self.chart_mut().set_extended_hours(extended);
                }
                if let Some(adjusted) = settings.adjust_for_dividends {
                    self.time.toolbar.state.adjust_for_dividends = adjusted;
                    self.studies.settings.adjust_for_dividends = adjusted;
                }
            }
            Direction::Save => {
                settings.timezone = Some(self.time.toolbar.state.timezone.clone());
                settings.extended_hours =
                    Some(self.time.toolbar.state.session_type != SessionType::RTH);
                settings.adjust_for_dividends = Some(self.time.toolbar.state.adjust_for_dividends);
            }
        }
    }

    pub(super) fn time_modal(&self) -> bool {
        self.time.date_picker.is_open
            || self.time.toolbar.state.timezone_menu_open
            || self.time.toolbar.state.session_menu_open
    }

    fn show_bottom_toolbar(&mut self, ui: &mut egui::Ui) {
        let bg = ui.style().visuals.panel_fill;
        egui::Panel::bottom("egui_charts_terminal_timeframe")
            .exact_size(39.0)
            .show_separator_line(false)
            .frame(panel_frame(bg).outer_margin(egui::Margin {
                top: 1,
                ..egui::Margin::ZERO
            }))
            .show(ui, |ui| {
                self.time.toolbar.state.date_picker_open = self.time.date_picker.is_open;
                let action = self.time.toolbar.show(ui);
                if action != TimeframeToolbarAction::None {
                    self.emit(Action::BottomBar(action));
                }
            });
    }

    fn show_date_picker(&mut self, ctx: &egui::Context) {
        let action = self.time.date_picker.show(ctx);
        if action != DatePickerAction::None {
            self.emit(Action::DatePicker(action));
        }
    }

    fn show_time_menus(&mut self, ctx: &egui::Context) {
        if self.time.toolbar.state.session_menu_open {
            let mut open = true;
            egui::Window::new(l10n::text(ctx, "Session"))
                .collapsible(false)
                .resizable(false)
                .open(&mut open)
                .show(ctx, |ui| {
                    for session in [SessionType::RTH, SessionType::ETH, SessionType::Full24x7] {
                        if ui
                            .selectable_label(
                                self.time.toolbar.state.session_type == session,
                                l10n::text(ui.ctx(), session.tooltip()),
                            )
                            .clicked()
                        {
                            self.time.toolbar.set_session(session);
                            self.time.toolbar.state.session_menu_open = false;
                            self.apply_session();
                        }
                    }
                });
            self.time.toolbar.state.session_menu_open &= open;
        }
        if self.time.toolbar.state.timezone_menu_open {
            let mut open = true;
            egui::Window::new(l10n::text(ctx, "Chart timezone"))
                .collapsible(false)
                .resizable(false)
                .open(&mut open)
                .show(ctx, |ui| {
                    for label in ["UTC", "Local", "America/New_York", "Europe/London", "Asia/Tokyo"]
                    {
                        if ui
                            .selectable_label(
                                self.time.toolbar.state.timezone == label,
                                l10n::text(ui.ctx(), label),
                            )
                            .clicked()
                        {
                            self.apply_timezone(label);
                            self.time.toolbar.state.timezone_menu_open = false;
                        }
                    }
                });
            self.time.toolbar.state.timezone_menu_open &= open;
        }
    }

    fn handle_toolbar(&mut self, action: TimeframeToolbarAction) {
        match action {
            TimeframeToolbarAction::DateRangeChanged(range) => {
                self.time.toolbar.state.sel_range = range;
                self.apply_date_range_preset(range);
            }
            TimeframeToolbarAction::OpenDatePicker => self.time.date_picker.open(),
            TimeframeToolbarAction::OpenTimezoneMenu => {
                self.time.toolbar.state.timezone_menu_open = true;
            }
            TimeframeToolbarAction::OpenSessionMenu => {
                self.time.toolbar.state.session_menu_open = true;
            }
            TimeframeToolbarAction::AdjustDividendsToggled(adjusted) => {
                self.time.toolbar.state.adjust_for_dividends = adjusted;
                self.studies.settings.adjust_for_dividends = adjusted;
                if self.replay_active {
                    self.stop_replay();
                }
                self.chart_mut().reload_feed();
            }
            TimeframeToolbarAction::None => {}
            _ => {}
        }
    }

    fn handle_date_picker(&mut self, action: DatePickerAction) {
        match action {
            DatePickerAction::GoToDate(date) => {
                let start = date.and_hms_opt(0, 0, 0).unwrap().and_utc();
                let bars = self.chart().bars(SeriesId::MAIN);
                let target = bars
                    .iter()
                    .find(|bar| bar.time >= start)
                    .or_else(|| bars.last())
                    .map_or(start, |bar| bar.time);
                self.chart_mut().scroll_to_time(target);
            }
            DatePickerAction::SetRange(start, end) => {
                let range = VisibleTimeRange {
                    from: start.and_hms_opt(0, 0, 0).unwrap().and_utc(),
                    to: end.and_hms_opt(23, 59, 59).unwrap().and_utc(),
                };
                let mut charts: Vec<_> = self.charts.iter_mut().collect();
                self.layout.apply_date_range(&mut charts, range);
            }
            DatePickerAction::Cancel | DatePickerAction::None => {}
            _ => {}
        }
    }

    fn apply_date_range_preset(&mut self, range: DateRange) {
        let bars = self.chart().bars(SeriesId::MAIN).to_vec();
        let visible = visible_bars_for_range(range, &bars);
        let width = self.chart().get_rendered_price_rect().width();
        // Include the latest slot in the scale's exclusive data range.
        let spacing = width.max(1.0) / (visible.max(1) as f32 + 0.5);
        self.chart_mut().disable_tracking_mode();
        self.chart_mut().apply_synced_time_scale(spacing, 0.5);
        if self.layout.sync.sync_date_range
            && let Some(range) = self.chart().visible_time_range()
        {
            let mut charts: Vec<_> = self.charts.iter_mut().collect();
            self.layout.apply_date_range(&mut charts, range);
        }
    }

    fn apply_timezone(&mut self, name: &str) {
        let timezone = match name {
            "UTC" => TimezoneMode::Utc,
            "Local" => TimezoneMode::Local,
            _ => match name.parse() {
                Ok(zone) => TimezoneMode::Timezone(zone),
                Err(_) => return,
            },
        };
        let chart = self.chart_mut();
        if chart.chart_options().time_scale.timezone != timezone {
            let mut options = chart.chart_options().clone();
            options.time_scale.timezone = timezone;
            let (spacing, offset) = chart.get_time_scale_state();
            chart.set_chart_options(options);
            chart.apply_synced_time_scale(spacing, offset);
        }
        self.time.toolbar.set_timezone(name.to_owned());
    }

    fn apply_session(&mut self) {
        let extended = self.time.toolbar.state.session_type != SessionType::RTH;
        self.chart_mut().set_extended_hours(extended);
        self.chart_mut().reload_feed();
    }
}

/// Count the supplied bars in a lookback ending at the latest bar timestamp.
fn visible_bars_for_range(range: DateRange, bars: &[Bar]) -> usize {
    let Some(last) = bars.last() else {
        return 0;
    };
    let end = last.time;
    let start = match range {
        DateRange::Day1 => Some(end - Duration::days(1)),
        DateRange::Day5 => Some(end - Duration::days(5)),
        DateRange::Month1 => end.checked_sub_months(Months::new(1)),
        DateRange::Month3 => end.checked_sub_months(Months::new(3)),
        DateRange::Month6 => end.checked_sub_months(Months::new(6)),
        DateRange::YTD => Utc.with_ymd_and_hms(end.year(), 1, 1, 0, 0, 0).single(),
        DateRange::Year1 => end.checked_sub_months(Months::new(12)),
        DateRange::Year5 => end.checked_sub_months(Months::new(60)),
        DateRange::All => None,
    };
    bars.len() - start.map_or(0, |start| bars.partition_point(|bar| bar.time < start))
}
