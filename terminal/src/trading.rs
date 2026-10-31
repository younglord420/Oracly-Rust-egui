//! Trading: the broker picker, the account panel, the bottom dock and the strategy report.

use super::core::Direction;
use super::layout::{Region, panel_frame};
use super::services::ExportContents;
use super::{Action, ExportFile, Terminal};
use egui_charts::localization as l10n;
use egui_charts::services::UserSettings;
use egui_charts::ui::Control;
use egui_charts::ui::replay::{ReplayToolbar, ReplayToolbarState};
use egui_charts::widget::SeriesId;
use egui_charts::ui::strategy_report::{StrategyReportAction, StrategyReportPanel};
use egui_charts::ui::study_editor::{StudyEditor, StudyEditorAction};
use egui_charts::ui::top_toolbar::TopToolbarAction;
use egui_charts::ui::trading::{
    AccountPanelAction, BrokerPicker, BrokerPickerAction, BrokerTile, Trading,
};
use egui_charts::ui_kit::tab_bar::{TabBar, TabLabel};

#[derive(Clone, Copy, PartialEq, Eq, Default)]
pub(super) enum DockTab {
    #[default]
    Account,
    Strategy,
    StudyEditor,
    ReplayTrading,
}

impl TabLabel for DockTab {
    fn tab_label(&self) -> &str {
        match self {
            Self::Account => "Account",
            Self::Strategy => "Strategy report",
            Self::StudyEditor => "Study editor",
            Self::ReplayTrading => "Replay trading",
        }
    }
}

#[derive(Clone, Copy, PartialEq, Eq, Default)]
enum DockSize {
    #[default]
    Normal,
    Minimized,
    Maximized,
}

pub(super) struct State {
    pub(super) trading: Trading,
    picker: BrokerPicker,
    strategy: StrategyReportPanel,
    study_editor: StudyEditor,
    pub(super) replay: ReplayToolbarState,
    pub(super) tab: DockTab,
    pub(super) open: bool,
    height: f32,
    size: DockSize,
    report_dirty: bool,
}

impl Default for State {
    fn default() -> Self {
        Self {
            trading: Trading::default(),
            picker: BrokerPicker::new(Vec::new()),
            strategy: StrategyReportPanel::default(),
            study_editor: StudyEditor::new(),
            replay: ReplayToolbarState::default(),
            tab: DockTab::Account,
            open: false,
            height: 280.0,
            size: DockSize::Normal,
            report_dirty: false,
        }
    }
}

impl Terminal {
    pub(super) fn trading_init(&mut self) {
        self.set_control_visible(Control::Trading, true);
        let tiles: Vec<BrokerTile> = self
            .services
            .brokers
            .iter()
            .map(|(tile, _)| tile.clone())
            .collect();
        self.trading.picker = BrokerPicker::new(tiles);
        self.top_toolbar.state.can_trade = true;
        self.trading.report_dirty = true;
    }

    pub(super) fn trading_poll(&mut self, ctx: &egui::Context) {
        self.trading
            .trading
            .poll(ctx, &mut self.toasts);
        if self.trading.report_dirty {
            self.trading.report_dirty = false;
            let bars = self.chart().bars(SeriesId::MAIN).to_vec();
            self.trading
                .strategy
                .set_report(self.strategy_report.clone(), &bars);
        }
    }

    pub(super) fn trading_show(&mut self, ui: &mut egui::Ui, region: Region) {
        match region {
            Region::Bottom => self.show_bottom_dock(ui),
            Region::Windows => {
                if let Some(action) = self.trading.picker.show(ui.ctx()) {
                    self.emit(Action::BrokerPicker(action));
                }
            }
            Region::Chart => {
                let chart = &mut self.charts[self.active];
                self.trading.trading.show(ui, chart);
            }
            _ => {}
        }
    }

    pub(super) fn trading_perform(&mut self, action: Action) -> Option<Action> {
        match action {
            Action::TopToolbar(TopToolbarAction::ToggleTrading) => {
                if self.trading.trading.is_connected() {
                    self.trading.open = !self.trading.open;
                    if self.trading.open {
                        self.trading.tab = DockTab::Account;
                    }
                } else {
                    self.trading.picker.open();
                }
                None
            }
            Action::BrokerPicker(action) => self.handle_broker_picker(action),
            Action::Account(action) => self.handle_account(action),
            Action::StrategyReport(action) => self.handle_strategy_report(action),
            other => Some(other),
        }
    }

    pub(super) fn trading_settings(&mut self, _settings: &mut UserSettings, _direction: Direction) {
    }

    pub(super) fn trading_modal(&self) -> bool {
        self.trading.picker.is_open()
    }

    /// Marks the strategy report panel to reload from [`Self::strategy_report`].
    pub(super) fn mark_strategy_report_dirty(&mut self) {
        self.trading.report_dirty = true;
    }

    fn show_bottom_dock(&mut self, ui: &mut egui::Ui) {
        if !self.trading.open && self.trading.strategy.report().is_none() {
            return;
        }
        if self.trading.strategy.report().is_some() && !self.trading.open {
            // A supplied report opens the dock on the strategy tab once.
            self.trading.open = true;
            self.trading.tab = DockTab::Strategy;
        }
        if !self.trading.open {
            return;
        }
        let height = match self.trading.size {
            DockSize::Minimized => 48.0,
            DockSize::Maximized => (ui.available_height() - 160.0).max(320.0),
            DockSize::Normal => self.trading.height,
        }
        .min((ui.available_height() - 96.0).max(48.0));
        let bg = ui.style().visuals.panel_fill;
        let tabs = [
            DockTab::Account,
            DockTab::Strategy,
            DockTab::StudyEditor,
            DockTab::ReplayTrading,
        ];
        egui::Panel::bottom("egui_charts_terminal_dock")
            .exact_size(height)
            .resizable(self.trading.size == DockSize::Normal)
            .show_separator_line(false)
            .frame(panel_frame(bg).outer_margin(egui::Margin {
                top: 1,
                ..egui::Margin::ZERO
            }))
            .show(ui, |ui| {
                if self.trading.size == DockSize::Normal {
                    self.trading.height = ui.min_rect().height().max(120.0);
                }
                ui.horizontal(|ui| {
                    TabBar::new(&tabs, &mut self.trading.tab).show(ui);
                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        if ui
                            .small_button(l10n::text(ui.ctx(), "Close"))
                            .on_hover_text(l10n::text(ui.ctx(), "Close dock"))
                            .clicked()
                        {
                            self.trading.open = false;
                        }
                    });
                });
                ui.separator();
                if self.trading.size == DockSize::Minimized {
                    return;
                }
                match self.trading.tab {
                    DockTab::Account => {
                        if self.trading.trading.is_connected() {
                            let symbol = self.charts[self.active].symbol().to_owned();
                            if let Some(action) =
                                self.trading.trading.show_account_panel(ui, &symbol)
                            {
                                self.emit(Action::Account(action));
                            }
                        } else {
                            ui.vertical_centered(|ui| {
                                ui.add_space(24.0);
                                ui.label(l10n::text(
                                    ui.ctx(),
                                    "Connect a broker to trade from this terminal",
                                ));
                                if ui
                                    .button(l10n::text(ui.ctx(), "Choose broker"))
                                    .clicked()
                                {
                                    self.trading.picker.open();
                                }
                            });
                        }
                    }
                    DockTab::Strategy => {
                        if self.trading.strategy.report().is_none() {
                            ui.centered_and_justified(|ui| {
                                ui.label(l10n::text(
                                    ui.ctx(),
                                    "No strategy report. The host supplies one through set_strategy_report.",
                                ));
                            });
                        } else {
                            let action = self.trading.strategy.show(ui);
                            if action != StrategyReportAction::None {
                                self.emit(Action::StrategyReport(action));
                            }
                        }
                    }
                    DockTab::StudyEditor => {
                        let action = self.trading.study_editor.show(ui);
                        if action != StudyEditorAction::None {
                            self.emit(Action::StudyEditor(action));
                        }
                    }
                    DockTab::ReplayTrading => {
                        let action = ReplayToolbar::new(&self.trading.replay).show(ui);
                        if let Some(action) = action {
                            self.emit(Action::ReplayToolbar(action));
                        }
                    }
                }
            });
    }

    fn handle_broker_picker(&mut self, action: BrokerPickerAction) -> Option<Action> {
        match action {
            BrokerPickerAction::Connect(id) => {
                if let Some((_, service)) = self
                    .services
                    .brokers
                    .iter()
                    .find(|(tile, _)| tile.id == id)
                {
                    let service = service.clone();
                    let name = self
                        .services
                        .brokers
                        .iter()
                        .find(|(tile, _)| tile.id == id)
                        .map(|(tile, _)| tile.name.clone())
                        .unwrap_or_else(|| id.clone());
                    self.trading.trading.set_service(service);
                    self.trading.trading.account_panel.broker = name;
                    self.trading.open = true;
                    self.trading.tab = DockTab::Account;
                    self.trading.size = DockSize::Normal;
                    None
                } else if id == BrokerTile::PAPER {
                    // No paper service was supplied; the host must attach one.
                    Some(Action::BrokerPicker(BrokerPickerAction::Connect(id)))
                } else {
                    Some(Action::BrokerPicker(BrokerPickerAction::Connect(id)))
                }
            }
            other => Some(Action::BrokerPicker(other)),
        }
    }

    fn handle_account(&mut self, action: AccountPanelAction) -> Option<Action> {
        match action {
            AccountPanelAction::SelectSymbol(symbol) => {
                self.switch_symbol(&symbol);
                None
            }
            AccountPanelAction::Disconnect => {
                self.trading.trading.disconnect();
                None
            }
            AccountPanelAction::Minimize => {
                self.trading.size = if self.trading.size == DockSize::Minimized {
                    DockSize::Normal
                } else {
                    DockSize::Minimized
                };
                None
            }
            AccountPanelAction::Maximize => {
                self.trading.size = if self.trading.size == DockSize::Maximized {
                    DockSize::Normal
                } else {
                    DockSize::Maximized
                };
                None
            }
            AccountPanelAction::Export { name, csv } => {
                let file = format!("{}.csv", name.to_lowercase().replace(' ', "_"));
                if let Some(save) = &mut self.services.save {
                    save(ExportFile {
                        name: file,
                        path: None,
                        contents: ExportContents::Text(csv),
                    });
                    None
                } else {
                    Some(Action::Account(AccountPanelAction::Export { name, csv }))
                }
            }
            other => Some(Action::Account(other)),
        }
    }

    fn handle_strategy_report(&mut self, action: StrategyReportAction) -> Option<Action> {
        match action {
            StrategyReportAction::SelectTrade(trade) => {
                if self.chart().symbol() != trade.symbol {
                    self.switch_symbol(&trade.symbol);
                }
                if self.chart().timeframe() != Some(trade.timeframe) {
                    self.chart_mut().set_timeframe(trade.timeframe);
                }
                self.chart_mut().scroll_to_time(trade.entry_time);
                None
            }
            StrategyReportAction::ExportCsv(csv) => {
                if let Some(save) = &mut self.services.save {
                    save(ExportFile {
                        name: "strategy-trades.csv".into(),
                        path: None,
                        contents: ExportContents::Text(csv),
                    });
                    None
                } else {
                    Some(Action::StrategyReport(StrategyReportAction::ExportCsv(csv)))
                }
            }
            StrategyReportAction::ExportSummaryCsv { filename, content } => {
                if let Some(save) = &mut self.services.save {
                    save(ExportFile {
                        name: filename,
                        path: None,
                        contents: ExportContents::Text(content),
                    });
                    None
                } else {
                    Some(Action::StrategyReport(
                        StrategyReportAction::ExportSummaryCsv { filename, content },
                    ))
                }
            }
            StrategyReportAction::None => None,
            other => Some(Action::StrategyReport(other)),
        }
    }
}
