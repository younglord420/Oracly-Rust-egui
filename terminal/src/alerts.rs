//! Alerts: the alerts panel, the alert dialog and the alert levels on the chart.

use super::core::Direction;
use super::layout::Region;
use super::{Action, Terminal};
use egui_charts::localization as l10n;
use egui_charts::services::UserSettings;
use egui_charts::ui::Control;
use egui_charts::ui::context_menu::{ChartContextAction, ContextMenuAction};
use egui_charts::ui::dialogs::{AlertDialog, AlertDialogAction, DrawingContextMenuAction};
use egui_charts::ui::dialogs::SeriesContextMenuAction;
use egui_charts::ui::top_toolbar::TopToolbarAction;
use egui_charts::ui::widget_bar::{AlertPanelAction, AlertsPanel};
use egui_charts::widget::SeriesId;

#[derive(Default)]
pub(super) struct State {
    dialog: AlertDialog,
    panel: AlertsPanel,
    /// Whether the floating alerts panel window is open.
    pub(super) panel_open: bool,
}

impl Terminal {
    pub(super) fn alerts_init(&mut self) {
        if let Some(service) = self.services.alerts.clone() {
            // Reveal the toolbar button and menu entries for alerts.
            self.set_control_visible(Control::Alerts, true);
            self.alerts.panel.set_service(service);
        }
    }

    pub(super) fn alerts_poll(&mut self, ctx: &egui::Context) {
        // The panel polls the service and fires triggered alerts.
        self.alerts.panel.poll(ctx, &mut self.toasts);
    }

    pub(super) fn alerts_show(&mut self, ui: &mut egui::Ui, region: Region) {
        if region != Region::Windows {
            return;
        }
        let ctx = ui.ctx().clone();

        // Alert creation / edit modal dialog.
        let dialog_action = self.alerts.dialog.show(&ctx);
        match dialog_action {
            AlertDialogAction::None | AlertDialogAction::Cancel => {}
            action => self.emit(Action::AlertDialog(action)),
        }

        // Alerts panel as a resizable floating window.
        if !self.alerts.panel_open {
            return;
        }
        let mut panel_open = self.alerts.panel_open;
        let mut panel_action = AlertPanelAction::None;
        egui::Window::new(l10n::text(&ctx, "Alerts"))
            .open(&mut panel_open)
            .default_width(340.0)
            .show(&ctx, |ui| {
                panel_action = self.alerts.panel.show(ui);
            });
        self.alerts.panel_open = panel_open;
        if panel_action != AlertPanelAction::None {
            self.emit(Action::Alerts(panel_action));
        }
    }

    pub(super) fn alerts_perform(&mut self, action: Action) -> Option<Action> {
        match action {
            // Top toolbar "Create alert" button.
            Action::TopToolbar(TopToolbarAction::CreateAlert) => {
                self.open_alert(None);
                None
            }
            // Alert dialog results.
            Action::AlertDialog(AlertDialogAction::Create(alert)) => {
                self.alerts.panel.save(*alert);
                None
            }
            Action::AlertDialog(AlertDialogAction::Cancel | AlertDialogAction::None) => None,
            Action::AlertDialog(_) => None,
            // Alerts panel actions forwarded from the floating panel window.
            Action::Alerts(AlertPanelAction::CreateAlert) => {
                self.open_alert(None);
                None
            }
            Action::Alerts(AlertPanelAction::EditAlert(alert)) => {
                self.open_alert(None);
                self.alerts.dialog.edit(*alert);
                None
            }
            Action::Alerts(AlertPanelAction::DuplicateAlert(alert)) => {
                self.open_alert(None);
                self.alerts.dialog.edit(*alert);
                None
            }
            Action::Alerts(AlertPanelAction::None) => None,
            // Remaining panel actions (ExportLog, etc.) go back to the host.
            Action::Alerts(other) => Some(Action::Alerts(other)),
            // Chart background context menu: "Add alert at price".
            Action::ChartMenu(ChartContextAction::Chart(ContextMenuAction::AddAlert {
                price,
                ..
            })) => {
                self.open_alert(Some(price));
                None
            }
            // Series context menu: "Add alert".
            Action::ChartMenu(ChartContextAction::Series(SeriesContextMenuAction::AddAlert)) => {
                self.open_alert(None);
                None
            }
            // Drawing context menu: "Add alert" carries the drawing's price level.
            Action::ChartMenu(ChartContextAction::Drawing(DrawingContextMenuAction::AddAlert(
                alert,
            ))) => {
                self.list_alert_sources();
                self.alerts.dialog.edit(*alert);
                None
            }
            other => Some(other),
        }
    }

    pub(super) fn alerts_settings(
        &mut self,
        _settings: &mut UserSettings,
        _direction: Direction,
    ) {}

    pub(super) fn alerts_modal(&self) -> bool {
        self.alerts.dialog.is_open
    }

    /// Opens the alert creation dialog, pre-filling `price` or the chart's last close.
    fn open_alert(&mut self, price: Option<f64>) {
        self.list_alert_sources();
        let price = price
            .or_else(|| {
                self.chart()
                    .bars(SeriesId::MAIN)
                    .last()
                    .map(|bar| bar.close)
            })
            .unwrap_or(0.0);
        let symbol = self.chart().symbol().to_owned();
        self.alerts.dialog.open(symbol, price);
    }

    /// Shares the chart's current study outputs and drawings with both the dialog and the panel.
    fn list_alert_sources(&mut self) {
        let sources = self.chart().alert_sources();
        self.alerts.dialog.sources = sources.clone();
        self.alerts.panel.set_sources(sources);
    }
}
