//! Layouts: chart slots, the layout menu (save/open/rename/favourites) and grid selection.

use super::core::Direction;
use super::layout::Region;
use super::{Action, Terminal};
use egui_charts::ChartBuilder;
use egui_charts::services::UserSettings;
use egui_charts::ui::Control;
use egui_charts::ui::top_toolbar::TopToolbarAction;
use egui_charts::ui::top_toolbar::components::layout_menu::LayoutAction;
use egui_charts::widget::{ChartLayoutMode, SavedLayout};

#[derive(Default)]
pub(super) struct State {
    /// Extra edits not tracked by individual chart histories (grid mode changes).
    grid_edits: u64,
}

impl Terminal {
    pub(super) fn layouts_init(&mut self) {
        // The layout grid selector is always available.
        self.set_control_visible(Control::Layouts, true);
        // Templates and the full layout menu need storage to persist layouts.
        let has_storage = self.services.storage.is_some();
        self.set_control_visible(Control::Templates, has_storage);
        if let Some(storage) = self.services.storage.clone() {
            self.top_toolbar.layout_menu.set_storage(storage);
        }
    }

    pub(super) fn layouts_poll(&mut self, ctx: &egui::Context) {
        // Poll async layout-menu storage operations (list, open, save, rename, delete).
        let opened = self.top_toolbar.layout_menu.poll(ctx);
        if let Some(error) = self.top_toolbar.layout_menu.take_error() {
            self.toasts.error(error);
        }
        if let Some((name, layout)) = opened {
            self.top_toolbar.layout_menu.curr_layout = Some(name);
            self.restore_layout(layout);
        }

        // Autosave: triggered by a quiescent edit count after a short delay.
        if self.top_toolbar.layout_menu.autosave {
            let edits = self
                .charts
                .iter()
                .map(|c| c.edit_count())
                .fold(self.layouts.grid_edits, u64::wrapping_add);
            if self.top_toolbar.layout_menu.autosave_due(ctx, edits) {
                self.save_layout();
            }
        }
    }

    pub(super) fn layouts_show(&mut self, _ui: &mut egui::Ui, _region: Region) {}

    pub(super) fn layouts_perform(&mut self, action: Action) -> Option<Action> {
        match action {
            Action::Layout(layout_action) => {
                self.handle_layout_action(layout_action);
                None
            }
            Action::TopToolbar(TopToolbarAction::Layout(layout_action)) => {
                self.handle_layout_action(layout_action);
                None
            }
            Action::TopToolbar(TopToolbarAction::Save) => {
                self.save_layout();
                None
            }
            other => Some(other),
        }
    }

    pub(super) fn layouts_settings(&mut self, settings: &mut UserSettings, direction: Direction) {
        match direction {
            Direction::Load => {
                self.top_toolbar.layout_menu.autosave = settings.layout_autosave;
                self.top_toolbar.layout_menu.favorites = settings.layout_favorites.clone();
                if let Some(name) = settings.layout.clone() {
                    // Defer opening: the layout_menu will issue a storage read on the next poll.
                    self.top_toolbar.layout_menu.open(&name);
                }
            }
            Direction::Save => {
                settings.layout = self.top_toolbar.layout_menu.curr_layout.clone();
                settings.layout_autosave = self.top_toolbar.layout_menu.autosave;
                settings.layout_favorites = self.top_toolbar.layout_menu.favorites.clone();
            }
        }
    }

    pub(super) fn layouts_modal(&self) -> bool {
        false
    }

    // ── Private helpers ──────────────────────────────────────────────────────

    fn handle_layout_action(&mut self, action: LayoutAction) {
        match action {
            LayoutAction::SelectGrid(mode) => {
                self.layouts.grid_edits = self.layouts.grid_edits.wrapping_add(1);
                self.ensure_chart_slots(mode);
            }
            LayoutAction::Save => {
                self.save_layout();
            }
            LayoutAction::SaveAs(name) => {
                match self.capture_layout() {
                    Ok(layout) => {
                        // Switch to the new name and save under it.
                        self.top_toolbar.layout_menu.curr_layout = Some(name.clone());
                        self.top_toolbar.layout_menu.save(&name, &layout);
                    }
                    Err(error) => self.toasts.error(error),
                }
            }
            LayoutAction::Open(name) => {
                self.top_toolbar.layout_menu.open(&name);
            }
            LayoutAction::Delete(name) => {
                self.top_toolbar.layout_menu.delete(&name);
            }
            LayoutAction::Rename(_, new_name) => {
                if !new_name.trim().is_empty() {
                    match self.capture_layout() {
                        Ok(layout) => {
                            self.top_toolbar.layout_menu.rename(&new_name, &layout);
                        }
                        Err(error) => self.toasts.error(error),
                    }
                }
            }
            LayoutAction::DownloadData => {
                // Export CSV; commands.rs handles the file delivery.
                self.emit(Action::TopToolbar(
                    TopToolbarAction::Layout(LayoutAction::DownloadData),
                ));
            }
            LayoutAction::DownloadImage => {
                // Open the export dialog (handled by commands.rs via TakeScreenshot proxy).
                self.emit(Action::TopToolbar(TopToolbarAction::TakeScreenshot));
            }
            LayoutAction::OpenLayout => {
                // No layout-chooser dialog in the terminal; the layout menu covers this.
            }
            LayoutAction::CreateNew => {
                // Reset to a single chart with the default layout.
                self.ensure_chart_slots(ChartLayoutMode::Single);
                self.layouts.grid_edits = self.layouts.grid_edits.wrapping_add(1);
            }
            LayoutAction::None => {}
            _ => {}
        }
    }

    /// Adds chart slots until there are at least as many as `mode` requires, then sets the mode.
    fn ensure_chart_slots(&mut self, mode: ChartLayoutMode) {
        let needed = if mode == ChartLayoutMode::Tabs {
            self.charts.len().max(2)
        } else {
            mode.chart_count().max(1)
        };
        while self.charts.len() < needed {
            let mut chart = ChartBuilder::new()
                .with_feed(self.feed.clone())
                .with_timeframe(
                    self.chart()
                        .timeframe()
                        .unwrap_or(egui_charts::model::Timeframe::Day1),
                )
                .build();
            chart.config_mut().skip_background = true;
            self.charts.push(chart);
            let idx = self.charts.len() - 1;
            self.wire_slot(idx);
        }
        self.layout.mode = mode;
        self.top_toolbar.chart_grid_selector.set_layout(mode);
        if self.active >= needed {
            self.activate_slot(0);
        }
    }

    /// Captures the current layout as a `SavedLayout`.
    fn capture_layout(&self) -> Result<SavedLayout, String> {
        let mode = self.layout.mode;
        let count = if mode == ChartLayoutMode::Tabs {
            self.charts.len()
        } else {
            mode.chart_count().min(self.charts.len()).max(1)
        };
        let charts = self.charts[..count]
            .iter()
            .map(|c| c.snapshot())
            .collect::<Result<Vec<_>, _>>()
            .map_err(|e| format!("Could not save the layout: {e}"))?;
        Ok(SavedLayout { mode, charts })
    }

    /// Restores chart slots and their state from a `SavedLayout` returned by the layout menu.
    fn restore_layout(&mut self, layout: SavedLayout) {
        if layout.charts.is_empty() {
            self.toasts.error("Could not open layout: no charts");
            return;
        }
        if layout.mode != ChartLayoutMode::Tabs
            && layout.charts.len() != layout.mode.chart_count()
        {
            self.toasts
                .error("Could not open layout: invalid chart count");
            return;
        }
        // Ensure we have enough chart slots; new ones inherit the active chart's feed.
        while self.charts.len() < layout.charts.len() {
            let mut chart = ChartBuilder::new().with_feed(self.feed.clone()).build();
            chart.config_mut().skip_background = true;
            self.charts.push(chart);
            let idx = self.charts.len() - 1;
            self.wire_slot(idx);
        }
        self.layout.mode = layout.mode;
        self.top_toolbar.chart_grid_selector.set_layout(layout.mode);
        if self.active >= layout.charts.len() {
            self.activate_slot(0);
        }
        for (chart, snapshot) in self.charts.iter_mut().zip(&layout.charts) {
            match chart.restore(snapshot) {
                Ok(skipped) if !skipped.is_empty() => {
                    self.toasts.warning(format!(
                        "Skipped {} unreadable drawings",
                        skipped.len()
                    ));
                }
                Err(error) => {
                    self.toasts
                        .error(format!("Could not open chart: {error}"));
                }
                Ok(_) => {}
            }
        }
    }

    /// Saves the current layout under the layout menu's current name.
    fn save_layout(&mut self) {
        if let Some(name) = self.top_toolbar.layout_menu.curr_layout.clone() {
            match self.capture_layout() {
                Ok(layout) => self.top_toolbar.layout_menu.save(&name, &layout),
                Err(error) => self.toasts.error(error),
            }
        }
    }
}
