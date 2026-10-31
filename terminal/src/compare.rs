//! Comparisons: compare dialog, comparison series and their synchronisation.

use super::core::Direction;
use super::layout::Region;
use super::{Action, Terminal};
use egui_charts::model::{Instrument, SeriesScale};
use egui_charts::services::UserSettings;
use egui_charts::ui::Control;
use egui_charts::ui::top_toolbar::{
    CompareAction, CompareSymbolsDialog, TopToolbarAction,
};
use egui_charts::widget::SeriesId;

pub(super) struct State {
    pub(super) dialog: CompareSymbolsDialog,
    /// Active comparison series as (symbol, id) pairs.
    comparisons: Vec<(String, SeriesId)>,
}

impl Default for State {
    fn default() -> Self {
        Self {
            dialog: CompareSymbolsDialog::new(),
            comparisons: Vec::new(),
        }
    }
}

impl Terminal {
    pub(super) fn compare_init(&mut self) {
        self.set_control_visible(Control::Compare, true);
        self.compare
            .dialog
            .set_search_service(self.services.symbol_search.clone());
        if let Some(storage) = &self.services.storage {
            self.compare.dialog.set_storage(storage.clone());
        }
    }

    pub(super) fn compare_poll(&mut self, _ctx: &egui::Context) {
        // Remove comparison series that were deleted from the chart outside this module.
        let live_ids: Vec<_> = self.chart().series().map(|(id, _)| id).collect();
        self.compare
            .comparisons
            .retain(|(_, id)| live_ids.contains(id));

        // Keep the dialog's symbol list in sync with the live comparisons.
        self.sync_compare_dialog();
    }

    pub(super) fn compare_show(&mut self, ui: &mut egui::Ui, region: Region) {
        if region != Region::Windows {
            return;
        }
        let instruments: Vec<Instrument> = Vec::new(); // no catalog: search drives discovery
        let action = self.compare.dialog.show(ui.ctx(), &instruments);
        if !matches!(action, CompareAction::None) {
            self.emit(Action::Compare(action));
        }
    }

    pub(super) fn compare_perform(&mut self, action: Action) -> Option<Action> {
        match action {
            Action::TopToolbar(TopToolbarAction::OpenCompare) => {
                self.compare.dialog.open();
                None
            }
            Action::Compare(compare_action) => {
                self.handle_compare_action(compare_action);
                None
            }
            other => Some(other),
        }
    }

    pub(super) fn compare_settings(
        &mut self,
        _settings: &mut UserSettings,
        _direction: Direction,
    ) {
    }

    pub(super) fn compare_modal(&self) -> bool {
        self.compare.dialog.is_open
    }

    // ── Private helpers ──────────────────────────────────────────────────────

    fn handle_compare_action(&mut self, action: CompareAction) {
        match action {
            CompareAction::AddSymbol(symbol, placement) => {
                if self
                    .compare
                    .comparisons
                    .iter()
                    .any(|(s, _)| *s == symbol)
                {
                    return; // already present
                }
                use egui_charts::ui::top_toolbar::ComparePlacement;
                use egui_charts::widget::SeriesOptions;

                let colors = [
                    self.theme.ui.info,
                    self.theme.ui.warning,
                    self.theme.ui.success,
                ];
                let color = colors[self.compare.comparisons.len() % colors.len()];

                let id = match placement {
                    ComparePlacement::NewPane => {
                        let id = self.chart_mut().add_series(SeriesOptions {
                            name: symbol.clone(),
                            color,
                            ..Default::default()
                        });
                        let pane = self.chart_mut().add_pane();
                        self.chart_mut().move_series_to_pane(id, pane);
                        id
                    }
                    ComparePlacement::NewPriceScale => {
                        // Find the first unused separate scale id.
                        let scale = (1..)
                            .map(SeriesScale::Separate)
                            .find(|scale| {
                                self.chart()
                                    .series()
                                    .all(|(_, opts)| opts.scale != *scale)
                            })
                            .unwrap_or(SeriesScale::Separate(1));
                        self.chart_mut().add_series(SeriesOptions {
                            name: symbol.clone(),
                            color,
                            scale,
                            ..Default::default()
                        })
                    }
                    _ => {
                        // Default: overlay on the same price scale as the main series.
                        self.chart_mut().add_comparison(symbol.clone(), color)
                    }
                };

                self.compare.comparisons.push((symbol, id));
                self.sync_compare_dialog();
            }
            CompareAction::RemoveSymbol(symbol) => {
                if let Some(pos) = self
                    .compare
                    .comparisons
                    .iter()
                    .position(|(s, _)| *s == symbol)
                {
                    let (_, id) = self.compare.comparisons.remove(pos);
                    self.chart_mut().remove_series(id);
                    self.sync_compare_dialog();
                }
            }
            CompareAction::ToggleVisibility(symbol) => {
                if let Some((_, id)) = self
                    .compare
                    .comparisons
                    .iter()
                    .find(|(s, _)| *s == symbol)
                {
                    let id = *id;
                    let visible = self.chart().is_series_visible(id);
                    self.chart_mut().set_series_visible(id, !visible);
                    self.sync_compare_dialog();
                }
            }
            CompareAction::None | CompareAction::Close => {}
            _ => {}
        }
    }

    /// Rebuilds the dialog's `compared_symbols` list from the live comparison series so the
    /// dialog always shows the current state (colours, visibility).
    fn sync_compare_dialog(&mut self) {
        use egui_charts::ui::top_toolbar::CompareSymbol;

        self.compare.dialog.compared_symbols = self
            .compare
            .comparisons
            .iter()
            .filter_map(|(symbol, id)| {
                let (_, opts) = self
                    .chart()
                    .series()
                    .find(|(s, _)| *s == *id)?;
                let mut entry = CompareSymbol::new(symbol, "", "", opts.color);
                entry.visible = self.chart().is_series_visible(*id);
                Some(entry)
            })
            .collect();
    }
}
